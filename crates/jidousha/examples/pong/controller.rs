//! The player inside the check: three of them, and the three numbers they
//! report about themselves.
//!
//! One controller cannot measure a game's difficulty, so there are three.
//! [`Brain::Rollout`] clears the mechanics — it can win. [`Brain::Idle`] proves
//! the game can be lost. [`Brain::Chaser`] is the one in between and the one
//! that says whether the game is worth playing: it steers at the ball, which is
//! what a person does on their first try, and it is the run that would catch an
//! opponent nobody can score against or a rally with nowhere to go.
//!
//! Every one of them presses W and S through a [`SnapshotBuilder`], so the
//! edges they produce are the ones a real keyboard produces: events, not
//! states, which is what makes a key held for a hundred ticks press once.
//!
//! # What the rollout controller does, in the order it does it
//!
//! 1. **Aim at where the ball will be.** [`roll_to`] runs the game's own
//!    [`crate::drift`] forward a tick at a time until the ball crosses a
//!    paddle's plane — the same arithmetic the game steps with, so the answer
//!    is the game's answer and not a second model of it. When the ball is
//!    heading the other way it rolls through the opponent's return first,
//!    carrying the opponent's paddle along with
//!    [`crate::opponent_target`], because this opponent *chases* and "take the
//!    return landing furthest from the middle" is close to the worst objective
//!    against one of those.
//! 2. **Constrain, then optimise.** Candidates that would strike with the last
//!    [`CONTACT_MARGIN`] of the paddle are dropped before anything is scored:
//!    the sharpest return is always the one struck at the very tip, and "the
//!    best available" resolves every time to standing so the ball hits your
//!    last millimetre — where half a tick of overshoot is a clean miss rather
//!    than a worse result.
//! 3. **Score only positions the paddle can stand on.** A paddle driven by a
//!    key moves in steps of `speed * fixed_dt` and cannot stop between them, so
//!    the candidates are the lattice `current + k * step` and nothing else. A
//!    candidate off that lattice is a place it cannot be, and the shot computed
//!    about it is a number about a future that will not happen.
//! 4. **Steer, and stop inside half a step**, so the paddle settles on a
//!    lattice point rather than dithering across the one it wanted.

use jidousha::prelude::*;
use jidousha::testing::{InputEvent, InputSnapshot, SnapshotBuilder};

use crate::{
    BALL_RADIUS, Ball, COURT_HALF_Y, Face, OPPONENT_SPEED, PADDLE_SIZE, PADDLE_TRAVEL, PADDLE_X,
    PLAYER_SPEED, Paddle, Side, Velocity, drift, face_crossing, face_of, opponent_target,
    paddle_step, paddle_towards, rebound,
};

/// How far ahead the ball is ever rolled, in ticks.
///
/// A serve crosses the court in about a hundred and forty ticks at its slowest,
/// so this is generous. A roll that runs out returns `None` rather than a
/// guess.
const LOOKAHEAD: u32 = 400;

/// How much of the paddle's half-length is off the menu, at each end.
///
/// The optimum sits on the boundary of what is reachable, and on a boundary any
/// error at all is a miss rather than a worse result. Scoring only the inner
/// 80% is the constraint that keeps the aim off the tip.
const CONTACT_MARGIN: f32 = 0.8;

/// How far past the paddle's plane the prediction still counts as a contact.
///
/// The prediction asks *where* the ball crosses, not whether this paddle
/// catches it, so its face is given a reach that no ball can be outside.
const PREDICTION_REACH: f32 = COURT_HALF_Y * 4.0;

/// Which of the three players is at the keyboard.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Brain {
    /// Plans the shot: rolls the ball forward, scores the positions it can
    /// actually stand on, and takes the return the opponent is furthest from.
    Rollout,
    /// Steers at the ball. What a person does on their first try.
    Chaser,
    /// Does nothing at all, so the game can be seen to be losable.
    Idle,
}

impl Brain {
    /// What this player is called in a verdict line.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Brain::Rollout => "rollout",
            Brain::Chaser => "chaser",
            Brain::Idle => "idle",
        }
    }
}

/// What a controller says about itself, and the only thing that says which half
/// of the program to open when a match comes out wrong.
#[derive(Default)]
pub(crate) struct Report {
    /// How many times the ball started coming at the player's paddle.
    pub(crate) approaches: u32,
    /// How many of those the paddle actually touched.
    pub(crate) met: u32,
    /// For every planned shot, how far from the opponent it was meant to land.
    planned_gaps: Vec<f32>,
    /// For every planned shot, how far from that plan it actually landed.
    aim_errors: Vec<f32>,
}

impl Report {
    /// The mean of a list of readings, or zero when there are none.
    fn mean(values: &[f32]) -> f32 {
        if values.is_empty() {
            return 0.0;
        }
        values.iter().sum::<f32>() / values.len() as f32
    }

    /// How far from the opponent this controller's shots were *meant* to land.
    ///
    /// Clears its objective: shots it hits exactly where it aimed are worth
    /// nothing if the aims were never threats.
    pub(crate) fn planned_gap(&self) -> f32 {
        Self::mean(&self.planned_gaps)
    }

    /// How far from its own plan this controller's shots actually landed.
    ///
    /// Clears its aim, and it is the number nobody writes unprompted: a correct
    /// prediction about a position the paddle cannot stand on is noise.
    pub(crate) fn aim_error(&self) -> f32 {
        Self::mean(&self.aim_errors)
    }

    /// How many shots the two numbers above are the mean of.
    pub(crate) fn shots(&self) -> usize {
        self.aim_errors.len()
    }

    /// The three numbers, as the lines a verdict prints.
    pub(crate) fn lines(&self) -> String {
        format!(
            "met {} of {} approaches; planned returns aimed to land {:.2} from the \
             opponent; shots landed {:.2} from where they were planned to ({} shots)",
            self.met,
            self.approaches,
            self.planned_gap(),
            self.aim_error(),
            self.shots(),
        )
    }
}

/// W and S, pressed and released the way a keyboard does it.
///
/// Events, not states: the controller remembers what it is holding so that a
/// key held for a hundred ticks produces one press edge rather than a hundred.
struct Keyboard {
    /// The driver's own accumulator, so this goes through the same edge rules a
    /// real keyboard does.
    builder: SnapshotBuilder,
    /// What is currently held down, if anything.
    holding: Option<Key>,
}

impl Keyboard {
    /// A keyboard with nothing held.
    fn new() -> Self {
        Keyboard {
            builder: SnapshotBuilder::new(),
            holding: None,
        }
    }

    /// Hold `want` and nothing else, sending only the edges that changed.
    fn press(&mut self, want: Option<Key>) -> InputSnapshot {
        if want != self.holding {
            if let Some(held) = self.holding {
                self.builder.record(InputEvent::KeyReleased(held));
            }
            if let Some(key) = want {
                self.builder.record(InputEvent::KeyPressed(key));
            }
            self.holding = want;
        }
        self.builder.first_tick_snapshot()
    }
}

/// Where the ball is when it reaches a plane, and where the opponent is then.
#[derive(Clone, Copy, Debug)]
struct Rolled {
    /// How many ticks it took to get there.
    ticks: u32,
    /// Where the ball's centre was when it crossed.
    contact_y: f32,
    /// How fast the ball was going, in world units per second.
    speed: f32,
    /// Where the opponent's paddle had got to by then.
    opponent_y: f32,
}

/// Roll the ball forward to `plane`, carrying the opponent's paddle with it.
///
/// The game's own [`crate::drift`] and [`crate::face_crossing`], stepped the
/// same way the same number of times, so this is the game's answer rather than
/// a model of it. `player_y` is where this controller intends to be standing,
/// which is what the opponent's rule reads.
fn roll_to(
    ball: Vec2,
    velocity: Vec2,
    opponent_y: f32,
    player_y: f32,
    plane: Face,
    dt: f32,
) -> Option<Rolled> {
    let mut pos = ball;
    let mut vel = velocity;
    let mut opponent = opponent_y;
    let step = paddle_step(OPPONENT_SPEED, dt);
    for tick in 1..=LOOKAHEAD {
        // The opponent moves before the ball does, which is the order the game
        // registers its systems in.
        opponent = paddle_towards(opponent, opponent_target(pos, vel, player_y), step);
        let to = pos + vel * dt;
        if let Some(at) = face_crossing(pos, to, BALL_RADIUS, plane) {
            return Some(Rolled {
                ticks: tick,
                contact_y: pos.lerp(to, at).y,
                speed: vel.length(),
                opponent_y: opponent,
            });
        }
        let (next, next_velocity) = drift(pos, vel, dt);
        pos = next;
        vel = next_velocity;
    }
    None
}

/// A plane at a paddle's face that catches any ball crossing it, wherever it
/// is — for asking *where* the ball arrives rather than whether it is caught.
fn prediction_plane(side: Side) -> Face {
    Face {
        reach: PREDICTION_REACH,
        centre_y: 0.0,
        ..face_of(side, Vec2::new(side.sign() * PADDLE_X, 0.0))
    }
}

/// One tick of a controller: what it can see, and what it decided.
pub(crate) struct Controller {
    /// Which of the three this is.
    brain: Brain,
    /// The keyboard it presses.
    keyboard: Keyboard,
    /// Which way the ball was going last tick, so an approach is an edge.
    was_coming: bool,
    /// Where the shot in flight was planned to land, if one is.
    planned_landing: Option<f32>,
    /// What it has to say about itself.
    pub(crate) report: Report,
}

impl Controller {
    /// A controller that has seen nothing yet.
    pub(crate) fn new(brain: Brain) -> Self {
        Controller {
            brain,
            keyboard: Keyboard::new(),
            was_coming: false,
            planned_landing: None,
            report: Report::default(),
        }
    }

    /// Look at the world, decide, and hand back the snapshot for this tick.
    ///
    /// On the way into tick 1 there is nothing to look at: `Startup` runs
    /// inside that tick, so the world is still empty and every read here is a
    /// `find`/`next` that copes with nothing being there.
    pub(crate) fn decide(&mut self, world: &World) -> InputSnapshot {
        let seen = look(world);
        let Some(seen) = seen else {
            return self.keyboard.press(None);
        };
        self.account(&seen);
        let target = match self.brain {
            Brain::Idle => None,
            Brain::Chaser => Some(seen.ball.y),
            Brain::Rollout => self.plan(&seen),
        };
        let want = match target {
            // Stop inside half a step, so the paddle settles on a lattice point
            // rather than dithering across the one it wanted.
            Some(target) if (target - seen.player_y).abs() > seen.step * 0.5 => {
                if target > seen.player_y {
                    Some(Key::S) // Y is down
                } else {
                    Some(Key::W)
                }
            }
            _ => None,
        };
        self.keyboard.press(want)
    }

    /// Count approaches and touches, and close out any shot in flight.
    fn account(&mut self, seen: &Seen) {
        let coming = seen.velocity.x < 0.0;
        if coming && !self.was_coming {
            self.report.approaches += 1;
        }
        // The ball turning round on the player's side of the court is this
        // paddle striking it: nothing else reverses it there.
        if self.was_coming && seen.velocity.x > 0.0 && seen.ball.x < 0.0 {
            self.report.met += 1;
            if let Some(planned) = self.planned_landing.take() {
                // Where the shot it actually produced is going to land, rolled
                // forward from the ball as it now is.
                if let Some(actual) = roll_to(
                    seen.ball,
                    seen.velocity,
                    seen.opponent_y,
                    seen.player_y,
                    prediction_plane(Side::Right),
                    seen.dt,
                ) {
                    self.report
                        .aim_errors
                        .push((actual.contact_y - planned).abs());
                }
            }
        }
        self.was_coming = coming;
    }

    /// Choose where to stand, out of the positions the paddle can be in.
    fn plan(&mut self, seen: &Seen) -> Option<f32> {
        let arrival = self.arrival(seen)?;
        let half = PADDLE_SIZE.y * 0.5;
        let usable = half * CONTACT_MARGIN;

        let mut best: Option<(f32, f32, f32)> = None; // score, position, landing
        let reach = i32::try_from(arrival.ticks).unwrap_or(i32::MAX);
        // Only the lattice points that could touch the ball are worth scoring;
        // the rest are constrained out before anything is computed about them.
        let span = (usable / seen.step).ceil() as i32 + 2;
        for k in -span..=span {
            if k.abs() > reach {
                continue;
            }
            let stand = (seen.player_y + k as f32 * seen.step).clamp(-PADDLE_TRAVEL, PADDLE_TRAVEL);
            if (arrival.contact_y - stand).abs() > usable {
                continue;
            }
            let leaving = rebound(Side::Left, arrival.contact_y, stand, arrival.speed);
            let contact = Vec2::new(
                prediction_plane(Side::Left).plane_x - BALL_RADIUS,
                arrival.contact_y,
            );
            let Some(landing) = roll_to(
                contact,
                leaving,
                arrival.opponent_y,
                stand,
                prediction_plane(Side::Right),
                seen.dt,
            ) else {
                continue;
            };
            let score = (landing.contact_y - landing.opponent_y).abs();
            if best.is_none_or(|(so_far, _, _)| score > so_far) {
                best = Some((score, stand, landing.contact_y));
            }
        }

        match best {
            Some((score, stand, landing)) => {
                // Recorded once per shot: the plan is set while the ball is
                // still coming and read back when the paddle strikes.
                if self.planned_landing.is_none() {
                    self.report.planned_gaps.push(score);
                }
                self.planned_landing = Some(landing);
                Some(stand)
            }
            // Nothing survived both constraints: run at the ball, which is at
            // least a chance of touching it.
            None => Some(arrival.contact_y),
        }
    }

    /// Where the ball will next reach this paddle's plane.
    ///
    /// When it is heading the other way that means rolling through the
    /// opponent's return first — which is only answerable because the
    /// opponent's rule is a function the game hands out rather than a branch
    /// buried in a system.
    fn arrival(&self, seen: &Seen) -> Option<Rolled> {
        let mine = prediction_plane(Side::Left);
        if seen.velocity.x < 0.0 {
            return roll_to(
                seen.ball,
                seen.velocity,
                seen.opponent_y,
                seen.player_y,
                mine,
                seen.dt,
            );
        }
        if seen.velocity.x <= 0.0 {
            return None; // parked at the centre, waiting for a serve
        }
        let theirs = roll_to(
            seen.ball,
            seen.velocity,
            seen.opponent_y,
            seen.player_y,
            prediction_plane(Side::Right),
            seen.dt,
        )?;
        let leaving = rebound(
            Side::Right,
            theirs.contact_y,
            theirs.opponent_y,
            theirs.speed,
        );
        let contact = Vec2::new(
            prediction_plane(Side::Right).plane_x + BALL_RADIUS,
            theirs.contact_y,
        );
        let back = roll_to(
            contact,
            leaving,
            theirs.opponent_y,
            seen.player_y,
            mine,
            seen.dt,
        )?;
        Some(Rolled {
            ticks: theirs.ticks + back.ticks,
            ..back
        })
    }
}

/// Everything a controller reads out of the world in one tick.
struct Seen {
    /// Where the ball is.
    ball: Vec2,
    /// How fast it is going, in world units per second.
    velocity: Vec2,
    /// Where the player's paddle is.
    player_y: f32,
    /// Where the opponent's paddle is.
    opponent_y: f32,
    /// How far the player's paddle moves in one tick.
    step: f32,
    /// How long one tick is, in seconds.
    dt: f32,
}

/// Read the world, or `None` on the way into tick 1 when there is nothing yet.
fn look(world: &World) -> Option<Seen> {
    let (ball, velocity) = world
        .query::<(&Transform, &Velocity, With<Ball>)>()
        .map(|(_, transform, velocity, _)| (transform.pos, velocity.0))
        .next()?;
    let dt = world.find_resource::<Time>()?.fixed_dt.as_f32();
    let mut player_y = None;
    let mut opponent_y = None;
    for (_, transform, paddle) in world.query::<(&Transform, &Paddle)>() {
        match paddle.side {
            Side::Left => player_y = Some(transform.pos.y),
            Side::Right => opponent_y = Some(transform.pos.y),
        }
    }
    Some(Seen {
        ball,
        velocity,
        player_y: player_y?,
        opponent_y: opponent_y?,
        step: paddle_step(PLAYER_SPEED, dt),
        dt,
    })
}
