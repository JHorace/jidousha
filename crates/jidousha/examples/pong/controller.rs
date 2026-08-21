//! The three players the check plays with, and why there are three of them.
//!
//! One controller cannot measure a game's difficulty. The [`Rollout`] one
//! clears the mechanics; the do-nothing player — an `InputSnapshot::new()` per
//! tick, which needs no type — proves the game can be *lost*; and [`Chaser`],
//! the paddle that simply follows the ball, is the one that says whether the
//! game is worth playing, because chasing what you can see is what a person
//! does on their first try. Wins, loses to nobody, and something in between:
//! only the middle line can say the game is playable.
//!
//! Every prediction below is built from the game's own functions —
//! `free_step`, `face_gap`, `face_contact`, `rebound`, `opponent_target`,
//! `paddle_step`. Nothing here re-derives the physics, which is the only reason
//! a plan made here describes the game that actually runs.

use jidousha::prelude::*;
use jidousha::testing::{InputEvent, InputSnapshot, SnapshotBuilder};

use crate::{
    GOAL_X, MAX_SPEED, OPPONENT_SPEED, PADDLE_LIMIT, PADDLE_X, PLAYER_SPEED, SPEED_RAMP, Side,
    contact_span, face_contact, face_gap, free_step, opponent_target, paddle_step, rebound,
};

/// How many ticks a rollout will look ahead before giving up.
///
/// A slow steep ball crosses the court in about three seconds, so this is
/// roughly twice the longest flight the game can produce.
const MAX_ROLLOUT: u32 = 400;

/// How much of the paddle's half-length is off the menu, either side.
///
/// **The optimum sits on the boundary of the feasible set, and there any error
/// at all is a clean miss rather than a worse result.** The sharpest return is
/// always the one struck at the very tip, so "take the best available" resolves
/// every time to "stand so the ball hits your last millimetre" — and then one
/// step of key quantisation is a whiff instead of a slightly flatter shot. This
/// is the margin that keeps the tip off the menu.
const TIP_MARGIN: f32 = 0.22;

/// The most lattice positions to score for one approach.
///
/// A ceiling on the work rather than a sampling density: the paddle can only
/// stand on multiples of its own key step, and an approach two seconds away
/// offers more of those than are worth scoring.
const CANDIDATES: i32 = 64;

/// What a controller learned about itself, printed every run.
///
/// **Three numbers, and the third is the one nobody writes unprompted.**
/// `met 27 of 27` prints happily alongside a 0-0 match, because meeting a ball
/// and threatening with it are different contracts. Read together they say
/// which half of the program to open, which no single number can.
#[derive(Default)]
pub(crate) struct Report {
    /// How many times the ball came at the paddle. The denominator.
    pub approaches: u32,
    /// How many of those it actually returned. Clears it as a *returner*.
    pub met: u32,
    /// Summed planned distance between the return's landing and the opponent's
    /// paddle. Clears its *objective*.
    pub planned_gap: f32,
    /// How many plans that sum is over.
    pub plans: u32,
    /// Summed distance between where a shot was planned to land and where the
    /// shot it actually produced lands. Clears its *aim*.
    pub aim_error: f32,
    /// How many shots that sum is over.
    pub shots: u32,
}

impl Report {
    /// The mean planned distance from the opponent, in world units.
    pub(crate) fn mean_planned_gap(&self) -> f32 {
        if self.plans == 0 {
            0.0
        } else {
            self.planned_gap / self.plans as f32
        }
    }

    /// The mean distance between a planned landing and the achieved one.
    pub(crate) fn mean_aim_error(&self) -> f32 {
        if self.shots == 0 {
            0.0
        } else {
            self.aim_error / self.shots as f32
        }
    }
}

/// Send events, not states — what makes a key held for a hundred ticks press
/// exactly once.
fn press(keyboard: &mut SnapshotBuilder, holding: &mut Option<Key>, want: Option<Key>) {
    if want == *holding {
        return;
    }
    if let Some(key) = *holding {
        keyboard.record(InputEvent::KeyReleased(key));
    }
    if let Some(key) = want {
        keyboard.record(InputEvent::KeyPressed(key));
    }
    *holding = want;
}

/// Which way to lean to get from `at` to `want`. Y is down, so S is towards
/// larger numbers.
///
/// A dead band half a step wide rather than a comparison: narrower and the
/// paddle oscillates across the target for ever, one step each way, which would
/// make its arrival position unpredictable and the lattice below a lie.
fn lean(at: f32, want: f32, step: f32) -> Option<Key> {
    if want - at > step * 0.5 {
        Some(Key::S)
    } else if at - want > step * 0.5 {
        Some(Key::W)
    } else {
        None
    }
}

/// Every height the player's paddle can actually stand on `ticks` from now.
///
/// **The step most likely to be skipped, and skipping it makes every number
/// downstream a number about a future that will not happen.** A paddle driven
/// by a key moves a whole `PLAYER_SPEED * fixed_dt` per tick and cannot stand
/// between two of those steps, so its reachable set is not the interval
/// `[y - n*step, y + n*step]` — it is the lattice `y + k*step`, clipped to the
/// court.
fn reachable(from: f32, ticks: u32, dt: f32) -> Vec<f32> {
    let step = PLAYER_SPEED * dt;
    let reach = i32::try_from(ticks).unwrap_or(CANDIDATES).min(CANDIDATES);
    let mut out: Vec<f32> = Vec::new();
    for k in -reach..=reach {
        let y = (from + k as f32 * step).clamp(-PADDLE_LIMIT, PADDLE_LIMIT);
        // The clamp folds several lattice points onto each end; keep one.
        if out.last().is_none_or(|last| (last - y).abs() > 1e-4) {
            out.push(y);
        }
    }
    out
}

/// Roll the ball forward to `side`'s paddle face: the tick it gets there, and
/// the height it gets there at.
///
/// `None` when it never does — which for the player's face means the ball is
/// going the other way, and for the opponent's means the rollout ran out.
fn predict_contact(mut pos: Vec2, mut velocity: Vec2, side: Side, dt: f32) -> Option<(u32, f32)> {
    let plane = side.sign() * PADDLE_X;
    for tick in 1..=MAX_ROLLOUT {
        let (straight, settled, reflected) = free_step(pos, velocity, dt);
        let gap_before = face_gap(pos.x, plane, side);
        let gap_after = face_gap(straight.x, plane, side);
        if let Some(fraction) = face_contact(gap_before, gap_after) {
            return Some((tick, pos.lerp(straight, fraction).y));
        }
        // Past the far goal without ever reaching the face: nothing to aim at.
        if straight.x.abs() > GOAL_X {
            return None;
        }
        pos = settled;
        velocity = reflected;
    }
    None
}

/// Where a return struck at `offset` off the player's paddle centre would land
/// on the opponent's face, and where the opponent would be when it got there.
///
/// The opponent is rolled forward beside the ball rather than assumed static,
/// because it *chases*: aiming at where it is standing now is aiming at the one
/// place it is guaranteed not to be. Its rule and the ball's are stepped in the
/// order a real tick runs them — the paddle first, then the ball — so this
/// disagrees with the game only by its own prediction error.
fn simulate_return(
    contact: Vec2,
    offset: f32,
    speed: f32,
    mut opponent_y: f32,
    dt: f32,
) -> Option<(f32, f32)> {
    let mut velocity = rebound(offset, speed, Side::Player);
    let mut pos = contact;
    for _ in 0..MAX_ROLLOUT {
        opponent_y = paddle_step(
            opponent_y,
            opponent_target(pos, velocity),
            OPPONENT_SPEED,
            dt,
        );
        let (straight, settled, reflected) = free_step(pos, velocity, dt);
        let gap_before = face_gap(pos.x, PADDLE_X, Side::Opponent);
        let gap_after = face_gap(straight.x, PADDLE_X, Side::Opponent);
        if let Some(fraction) = face_contact(gap_before, gap_after) {
            return Some((pos.lerp(straight, fraction).y, opponent_y));
        }
        pos = settled;
        velocity = reflected;
    }
    None
}

/// One approach's plan: where to stand, and what the shot from there is
/// believed to be worth.
#[derive(Clone, Copy)]
struct Aim {
    /// The paddle height to be at when the ball arrives.
    stand_at: f32,
    /// Where the planned return lands on the opponent's face.
    landing: f32,
    /// How far that landing is from where the opponent will be.
    gap: f32,
}

/// The good player: predicts, constrains, optimises, and enumerates.
pub(crate) struct Rollout {
    keyboard: SnapshotBuilder,
    holding: Option<Key>,
    aim: Option<Aim>,
    incoming: bool,
    /// What it has learned about itself.
    pub report: Report,
}

impl Default for Rollout {
    fn default() -> Self {
        Self {
            keyboard: SnapshotBuilder::new(),
            holding: None,
            aim: None,
            incoming: false,
            report: Report::default(),
        }
    }
}

impl Rollout {
    /// Decide what to hold this tick, and notice what the last one produced.
    pub(crate) fn decide(
        &mut self,
        ball_pos: Vec2,
        ball_velocity: Vec2,
        ball_speed: f32,
        paddle_y: f32,
        opponent_y: f32,
        dt: f32,
    ) -> InputSnapshot {
        let step = PLAYER_SPEED * dt;
        let approaching = ball_velocity.x < 0.0;

        // The shot has just been struck: the ball was coming and is now going.
        // Measure the return it actually produced against the one that was
        // planned, which is the number that catches a controller optimising
        // over places its paddle cannot stand.
        if self.incoming && !approaching && ball_velocity.x > 0.0 {
            self.report.met += 1;
            if let Some(aim) = self.aim
                && let Some((_, landed)) =
                    predict_contact(ball_pos, ball_velocity, Side::Opponent, dt)
            {
                self.report.aim_error += (landed - aim.landing).abs();
                self.report.shots += 1;
            }
            self.aim = None;
        }
        if !approaching {
            self.incoming = false;
            self.aim = None;
            // Waiting: sit on the centre line, which is the shortest mean
            // distance to whatever comes back.
            let want = lean(paddle_y, 0.0, step);
            press(&mut self.keyboard, &mut self.holding, want);
            return self.keyboard.first_tick_snapshot();
        }
        if !self.incoming {
            self.report.approaches += 1;
            self.incoming = true;
        }

        // 1. Predict: where the ball will cross this paddle's face, and when.
        let want = match predict_contact(ball_pos, ball_velocity, Side::Player, dt) {
            None => lean(paddle_y, ball_pos.y, step),
            Some((ticks, contact_y)) => {
                let plan = self.plan(contact_y, ticks, ball_speed, paddle_y, opponent_y, dt);
                match plan {
                    Some(aim) => {
                        if self.aim.is_none() {
                            self.report.planned_gap += aim.gap;
                            self.report.plans += 1;
                        }
                        self.aim = Some(aim);
                        lean(paddle_y, aim.stand_at, step)
                    }
                    // Nothing that makes contact is reachable. Run at the ball
                    // anyway: a controller that concedes reports a game as
                    // unwinnable when it is merely losing this point.
                    None => lean(paddle_y, contact_y, step),
                }
            }
        };
        press(&mut self.keyboard, &mut self.holding, want);
        self.keyboard.first_tick_snapshot()
    }

    /// Constrain, then optimise, over the positions the paddle can stand on.
    ///
    /// The order is not cosmetic: optimising before constraining puts the
    /// answer on the boundary of the feasible set every single time.
    fn plan(
        &self,
        contact_y: f32,
        ticks: u32,
        ball_speed: f32,
        paddle_y: f32,
        opponent_y: f32,
        dt: f32,
    ) -> Option<Aim> {
        // The return's speed is the game's own ramp, so a plan made at 21 units
        // a second is not a plan about a ball travelling at 32.
        let speed = (ball_speed * SPEED_RAMP).min(MAX_SPEED);
        // 2. Constrain: contact points that really connect, less the tip.
        let usable = contact_span() - TIP_MARGIN;
        let contact = Vec2::new(
            -PADDLE_X + crate::PADDLE_SIZE.x / 2.0 + crate::BALL_RADIUS,
            contact_y,
        );
        let mut best: Option<Aim> = None;
        // 4. Enumerate: only heights the paddle can actually be at.
        for stand_at in reachable(paddle_y, ticks, dt) {
            let offset = contact_y - stand_at;
            if offset.abs() > usable {
                continue;
            }
            let Some((landing, opponent_at)) =
                simulate_return(contact, offset, speed, opponent_y, dt)
            else {
                continue;
            };
            // 3. Optimise: the return landing furthest from where the opponent
            // will have got to — which is a different number from "furthest
            // from where it is now", and the difference is the whole game.
            let gap = (landing - opponent_at).abs();
            if best.is_none_or(|seen| gap > seen.gap) {
                best = Some(Aim {
                    stand_at,
                    landing,
                    gap,
                });
            }
        }
        best
    }
}

/// The middle player of three: chases the ball's current height, nothing more.
///
/// **This is the one that measures the game.** The rollout above says whether
/// the game can be won by something that plans; a do-nothing run says whether
/// it can be lost. Neither says whether a *person* has a game worth playing,
/// and a Pong only a rollout controller can win is a Pong nobody will enjoy.
/// The gap between this score and the rollout's is the game's difficulty,
/// measured rather than asserted.
pub(crate) struct Chaser {
    keyboard: SnapshotBuilder,
    holding: Option<Key>,
}

impl Default for Chaser {
    fn default() -> Self {
        Self {
            keyboard: SnapshotBuilder::new(),
            holding: None,
        }
    }
}

impl Chaser {
    /// Steer at where the ball is right now.
    pub(crate) fn decide(&mut self, ball_pos: Vec2, paddle_y: f32, dt: f32) -> InputSnapshot {
        let want = lean(paddle_y, ball_pos.y, PLAYER_SPEED * dt);
        press(&mut self.keyboard, &mut self.holding, want);
        self.keyboard.first_tick_snapshot()
    }
}
