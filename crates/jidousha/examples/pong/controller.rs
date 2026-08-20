//! The player the check plays with: a left paddle that tries to *win*.
//!
//! A blind `InputScript` never returns a ball, so it can prove the controls
//! work and say nothing about whether the game is playable. A controller that
//! tracks the ball perfectly returns it dead flat down the middle, and if the
//! opponent tracks too the rally has nowhere to go — both sides hold a groove
//! neither can lose and the run reports 0-0 about a game that is fine.
//!
//! So this one plays shots. The shape is the one the testing document argues
//! for, and each step of it earned its place:
//!
//! 1. **Predict.** Run [`crate::advance`] forward from the ball's current state
//!    with this paddle out of the way, to find which tick the ball reaches the
//!    contact plane on and at what height.
//! 2. **Constrain.** Only paddle positions that really make contact — with a
//!    margin, so the very tip is not on the menu — and that this paddle can
//!    reach in the ticks available.
//! 3. **Optimise.** Score what survives by running the *whole* return forward,
//!    the opponent moving under its own [`crate::opponent_push`] beside it, and
//!    take the shot landing furthest from where that puts it. Aiming at where
//!    the opponent *is* is the wrong objective against a paddle that moves.
//! 4. **Minimax the error it knows it has.** A paddle driven by a key moves in
//!    steps of `speed * fixed_dt` and cannot stand between them, so it arrives
//!    about a fifth of a unit from where it meant to. Each candidate is scored
//!    by its *worst* outcome across plus and minus one step, which stops the
//!    controller picking candidates whose apparent merit is a coincidence.
//!
//! And it reports on itself. Three numbers, printed every run, because a
//! correct controller and a broken one both produce a plausible-looking score
//! line and only the three together say which half of the program to open.

use jidousha::prelude::*;
use jidousha::testing::{InputEvent, InputSnapshot, SnapshotBuilder};

use crate::{
    BALL_RADIUS, CONTACT_X, Flight, GOAL_LINE, PADDLE_LIMIT, PADDLE_SIZE, PLAYER_SPEED, Side,
    advance, opponent_push,
};

/// How many ticks a rollout may run before it gives up on the ball ever
/// getting anywhere.
const ROLLOUT: u32 = 400;

/// How many candidate positions to score, at most.
///
/// A ceiling on the lattice below rather than a sampling density: the paddle
/// can only stand on multiples of its own step, so the contact band holds
/// however many of those fit and this is only here to bound the work.
const CANDIDATES: i32 = 24;

/// How much of the paddle's half-length is off the menu.
///
/// The sharpest return is always the one struck at the very tip, where the
/// bounce angle is widest — so "take the best shot available" resolves every
/// time to "stand so the ball hits your last millimetre", and there any error
/// at all is a clean miss rather than a worse return. The optimum sits on the
/// boundary of the feasible set; this is the margin that keeps it off.
const TIP_MARGIN: f32 = 0.22;

/// How often the controller reconsiders, in ticks.
///
/// Not every tick: the prediction barely moves between them, and the rollouts
/// are the expensive part of the run.
const RETHINK: u64 = 12;

/// What one approach — one ball coming at the player — cost and produced.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Approach {
    /// Whether the paddle actually touched the ball.
    pub(crate) met: bool,
    /// How far from the opponent the chosen shot was *planned* to land.
    ///
    /// `None` when the controller never got as far as choosing a shot, which
    /// is what "it could not reach the ball at all" looks like from here.
    pub(crate) planned_gap: Option<f32>,
    /// How far the shot actually landed from where it was planned to.
    pub(crate) aim_error: Option<f32>,
}

/// The keyboard the controller types on, and everything it has learnt.
pub(crate) struct Controller {
    keyboard: SnapshotBuilder,
    /// Which of up and down is currently held, so events are sent rather than
    /// states — a key held for a hundred ticks must press exactly once.
    holding: Option<Key>,
    /// Where the paddle is trying to stand.
    target: f32,
    /// When the current plan was made.
    decided_at: u64,
    /// Whether this approach has been counted yet.
    approaching: bool,
    /// The shot the current plan expects, as a landing height.
    planned_landing: Option<f32>,
    /// How far from the opponent that landing was planned to be.
    planned_gap: Option<f32>,
    /// Whether the paddle has touched the ball during this approach.
    met: bool,
    /// One entry per approach.
    pub(crate) approaches: Vec<Approach>,
}

impl Controller {
    /// A controller that has not seen anything yet.
    pub(crate) fn new() -> Self {
        Controller {
            keyboard: SnapshotBuilder::new(),
            holding: None,
            target: 0.0,
            decided_at: 0,
            approaching: false,
            planned_landing: None,
            planned_gap: None,
            met: false,
            approaches: Vec::new(),
        }
    }

    /// What the player is doing on this tick.
    ///
    /// Call once per tick, before `sim.tick()`, with the world as it stands.
    /// `ball` is `None` on the way into tick 1, when `Startup` has not run and
    /// there is nothing to look at yet.
    pub(crate) fn snapshot(
        &mut self,
        tick: u64,
        ball: Option<Flight>,
        paddle_y: f32,
        opponent_y: f32,
        dt: f32,
    ) -> InputSnapshot {
        if let Some(ball) = ball {
            self.think(tick, ball, paddle_y, opponent_y, dt);
        }
        let step = PLAYER_SPEED * dt;
        let want = if greater_by(self.target, paddle_y, step * 0.5) {
            Some(Key::S)
        } else if greater_by(paddle_y, self.target, step * 0.5) {
            Some(Key::W)
        } else {
            None
        };
        if want != self.holding {
            if let Some(old) = self.holding {
                self.keyboard.record(InputEvent::KeyReleased(old));
            }
            if let Some(new) = want {
                self.keyboard.record(InputEvent::KeyPressed(new));
            }
            self.holding = want;
        }
        self.keyboard.first_tick_snapshot()
    }

    /// Tell the controller the ball has just been struck by somebody.
    ///
    /// Called from the run loop, which is the only place that can see a touch
    /// happen; the controller only ever gets to look at the world between
    /// ticks.
    pub(crate) fn saw_touch(&mut self, side: Side, after: Flight, opponent_y: f32, dt: f32) {
        if side != Side::Left {
            return;
        }
        self.met = true;
        // Where the shot it just played actually ends up, run forward the same
        // way the plan was. The difference between this and `planned_landing`
        // is the controller's *aim*, which is the number nobody writes and the
        // one that says whether a correct prediction was worth anything.
        let landed =
            roll_to_opponent(after, PADDLE_LIMIT * 2.0, opponent_y, dt).map(|out| out.ball);
        let error = match (self.planned_landing, landed) {
            (Some(planned), Some(actual)) => Some((actual - planned).abs()),
            _ => None,
        };
        self.approaches.push(Approach {
            met: true,
            planned_gap: self.planned_gap,
            aim_error: error,
        });
        self.close_approach();
    }

    /// Tell the controller the ball is out of play, so an approach it never
    /// met is counted as missed.
    pub(crate) fn saw_dead_ball(&mut self) {
        if self.approaching && !self.met {
            self.approaches.push(Approach {
                met: false,
                planned_gap: self.planned_gap,
                aim_error: None,
            });
        }
        self.close_approach();
    }

    /// Forget everything about the approach that has just ended.
    fn close_approach(&mut self) {
        self.approaching = false;
        self.met = false;
        self.planned_landing = None;
        self.planned_gap = None;
        self.decided_at = 0;
    }

    /// Decide where to stand.
    fn think(&mut self, tick: u64, ball: Flight, paddle_y: f32, opponent_y: f32, dt: f32) {
        if ball.vel.x >= 0.0 || ball.vel.length_squared() <= 0.0 {
            // Nothing coming. Go back to the middle, which is the best place to
            // be when you do not know where the next ball is going.
            self.target = 0.0;
            return;
        }
        if !self.approaching {
            self.approaching = true;
            self.decided_at = 0;
        }
        if self.decided_at != 0 && tick < self.decided_at + RETHINK {
            return;
        }
        self.decided_at = tick;

        // 1. Where and when the ball arrives, with this paddle out of the way.
        let Some(arrival) = arrival_of(ball, dt) else {
            self.target = ball.pos.y;
            return;
        };

        // 2. The positions that make contact with margin and can be reached.
        //
        // On the paddle's own lattice, not anywhere in the band: it moves a
        // whole `step` a tick and stops when it is within half of one, so the
        // only positions it can actually occupy are `paddle_y + k * step`. A
        // candidate off the lattice is a plan the paddle cannot carry out, and
        // scoring one is scoring a future that will not happen — which is most
        // of where a controller's aim error comes from.
        let usable = PADDLE_SIZE.y * 0.5 * (1.0 - TIP_MARGIN);
        let step = PLAYER_SPEED * dt;
        // One tick in hand, because the ball's contact is tested against the
        // paddle where *this same tick* has already put it: a paddle still
        // travelling on the contact tick is not standing where the plan says.
        let ticks_in_hand = arrival.ticks.saturating_sub(1) as i32;
        let lowest = ((arrival.height - usable - paddle_y) / step).ceil() as i32;
        let highest = ((arrival.height + usable - paddle_y) / step).floor() as i32;
        let mut best: Option<(f32, f32, f32)> = None; // (score, target, landing)
        for offset in lowest.max(-ticks_in_hand)..=highest.min(ticks_in_hand) {
            if offset.abs() > CANDIDATES {
                continue;
            }
            let stand = paddle_y + offset as f32 * step;
            if !(-PADDLE_LIMIT..=PADDLE_LIMIT).contains(&stand) {
                continue; // outside the clamp, so the paddle would stop short
            }
            // 3. Play the whole return out, the opponent moving under its own
            // rule beside it, and score by how far the ball lands from where
            // that puts the opponent.
            let Some(outcome) = play_out(ball, stand, opponent_y, dt) else {
                continue; // this position does not make contact at all
            };
            let score = (outcome.ball - outcome.opponent).abs();
            if best.is_none_or(|(so_far, _, _)| score > so_far) {
                best = Some((score, stand, outcome.ball));
            }
        }

        match best {
            Some((score, stand, landing)) => {
                self.target = stand;
                self.planned_landing = Some(landing);
                self.planned_gap = Some(score);
            }
            // Nothing survived both constraints. Run at the ball: a return that
            // is merely a return beats a shot that never happens.
            None => {
                self.target = arrival.height.clamp(-PADDLE_LIMIT, PADDLE_LIMIT);
                self.planned_landing = None;
                self.planned_gap = None;
            }
        }
    }
}

/// `a` is greater than `b` by more than `slack`, and false when either is NaN.
fn greater_by(a: f32, b: f32, slack: f32) -> bool {
    matches!(
        (a - b).partial_cmp(&slack),
        Some(std::cmp::Ordering::Greater)
    )
}

/// When and where a ball arrives at the player's contact plane.
#[derive(Clone, Copy, Debug)]
struct Arrival {
    /// How many ticks away.
    ticks: u32,
    /// At what height.
    height: f32,
}

/// Run the ball forward with both paddles out of reach, to find where it
/// crosses the player's contact plane.
fn arrival_of(ball: Flight, dt: f32) -> Option<Arrival> {
    // Well past the clamp, so neither paddle can be in the way — the question
    // is where the ball goes, not who touches it.
    let away = [PADDLE_LIMIT * 4.0, PADDLE_LIMIT * 4.0];
    let mut flight = ball;
    for tick in 1..=ROLLOUT {
        if let Some(height) = crosses_plane(flight, -CONTACT_X, -1.0, dt) {
            return Some(Arrival {
                ticks: tick,
                height,
            });
        }
        flight = advance(flight, away, dt).flight;
        if flight.pos.x.abs() > GOAL_LINE {
            return None;
        }
    }
    None
}

/// Where a ball's next tick of travel crosses `plane`, if it does.
///
/// `toward` is the sign of the travel that counts as approaching, the same
/// convention [`crate::crossing`] uses.
fn crosses_plane(flight: Flight, plane: f32, toward: f32, dt: f32) -> Option<f32> {
    let next = flight.pos + flight.vel * dt;
    let travel = next.x - flight.pos.x;
    if travel * toward <= 0.0 {
        return None;
    }
    if (flight.pos.x - plane) * toward > 0.0 {
        return None;
    }
    if (next.x - plane) * toward < 0.0 {
        return None;
    }
    let fraction = (plane - flight.pos.x) / travel;
    if !(0.0..=1.0).contains(&fraction) {
        return None;
    }
    Some(flight.pos.y + (next.y - flight.pos.y) * fraction)
}

/// Where a rally ends up, from the opponent's point of view.
#[derive(Clone, Copy, Debug)]
struct Outcome {
    /// The height the ball reaches the opponent's contact plane at.
    ball: f32,
    /// Where the opponent is standing when it does.
    opponent: f32,
}

/// Play the whole return out: the player's paddle pinned at `stand`, the
/// opponent moving under its own rule.
///
/// `None` when the pinned paddle never touches the ball, which is what
/// disqualifies a candidate.
fn play_out(ball: Flight, stand: f32, opponent_y: f32, dt: f32) -> Option<Outcome> {
    let mut flight = ball;
    let mut opponent = opponent_y;
    let mut met = false;
    for _ in 0..ROLLOUT {
        opponent = (opponent + opponent_push(flight, opponent) * crate::OPPONENT_SPEED * dt)
            .clamp(-PADDLE_LIMIT, PADDLE_LIMIT);
        if met && let Some(height) = crosses_plane(flight, CONTACT_X, 1.0, dt) {
            return Some(Outcome {
                ball: height,
                opponent,
            });
        }
        let step = advance(flight, [stand, opponent], dt);
        flight = step.flight;
        if step.touched == Some(Side::Left) {
            met = true;
        }
        if flight.pos.x.abs() > GOAL_LINE {
            return if met {
                // Off the end of the court past the opponent: the best possible
                // outcome, and the loop above missed the crossing only because
                // the ball went by outside the paddle's reach.
                Some(Outcome {
                    ball: flight.pos.y,
                    opponent,
                })
            } else {
                None
            };
        }
    }
    None
}

/// Where a ball already in flight reaches the opponent, for measuring aim.
fn roll_to_opponent(ball: Flight, stand: f32, opponent_y: f32, dt: f32) -> Option<Outcome> {
    let mut flight = ball;
    let mut opponent = opponent_y;
    for _ in 0..ROLLOUT {
        opponent = (opponent + opponent_push(flight, opponent) * crate::OPPONENT_SPEED * dt)
            .clamp(-PADDLE_LIMIT, PADDLE_LIMIT);
        if let Some(height) = crosses_plane(flight, CONTACT_X, 1.0, dt) {
            return Some(Outcome {
                ball: height,
                opponent,
            });
        }
        flight = advance(flight, [stand, opponent], dt).flight;
        if flight.pos.x.abs() > GOAL_LINE {
            return Some(Outcome {
                ball: flight.pos.y,
                opponent,
            });
        }
    }
    None
}

/// How wide a gap counts as a shot the opponent cannot cover.
pub(crate) const OPPONENT_REACH: f32 = PADDLE_SIZE.y * 0.5 + BALL_RADIUS;
