//! The player the check plays with, and the reason this example exists.
//!
//! `docs/api/jidousha-testing.md` argues for a four-step shape — predict,
//! constrain, optimise, enumerate — and every worked instance of it was a paddle
//! returning a ball. This is the same four steps in a game with **no opponent,
//! no bounce and no rally**, so that a reader can tell the lesson from the Pong.
//!
//! Read it against the game and the differences are the interesting part:
//!
//! | the shape | in Pong | here |
//! |---|---|---|
//! | predict | where the ball crosses the contact plane | where the gate will have drifted to |
//! | constrain | contact points that are reachable, tip excluded | positions reachable in the ticks available, gap edges excluded |
//! | optimise | the return landing furthest from the opponent | this gate's clearance, plus the reach it leaves for the next |
//! | enumerate | paddle heights on the key-step lattice | glider positions on the key-step lattice |
//!
//! **The step that has no analogue here is the one worth naming.** Pong's
//! objective is "aim away from where the opponent will be", and a slalom has
//! nobody to aim away from. That is not a hole in this example — it is the
//! boundary between the part of the advice that is about *driving a game* and
//! the part that is about *games with an adversary*. The four rows transfer
//! unchanged; the opponent model does not exist here, and a reader who assumed
//! it was universal would have gone looking for it.

use jidousha::prelude::*;
use jidousha::testing::{InputEvent, InputSnapshot, SnapshotBuilder};

use crate::{
    COURSE_HALF_WIDTH, GATE_HALF_GAP, GATES, GLIDE_SPEED, GLIDER_HALF_WIDTH, clearance,
    gate_center_at, gate_depth, ticks_to_fall,
};

/// How much of the gap is off the menu, either side.
///
/// **The optimum sits on the boundary of the feasible set, and there any error
/// at all is a miss rather than a worse result.** A controller that aims at the
/// widest clearance it can technically achieve ends up planning to fly along the
/// inside face of a post, because that is where the last millimetre of reach
/// buys the most — and then one step of quantisation puts it through the post.
/// This is the margin that keeps the boundary off the menu. Pong's version of
/// this constant is called `TIP_MARGIN`; the failure is identical and the
/// vocabulary is not, which is the point of having two worked instances.
const EDGE_MARGIN: f32 = 0.32;

/// How far ahead the plan looks, in gates.
///
/// One gate is not enough, and the reason generalises past this game: clearing
/// the next gate perfectly can leave the glider on the wrong side of the course
/// for the one after, which drifted the other way. Three is the cheapest number
/// that reliably sees it here, and `verify.rs` reports what it is worth.
const LOOKAHEAD: u32 = 3;

/// How many candidate positions to score, at most.
///
/// A ceiling on the lattice in [`reachable`] rather than a sampling density: the
/// glider can only stand on multiples of its own step, so a gate far enough away
/// offers more of them than are worth scoring and this bounds the work.
const CANDIDATES: i32 = 48;

/// What the controller learned about itself, printed every run.
///
/// **Three numbers, and the third is the one nobody writes unprompted.** A
/// controller is code with a contract like any other, and reading it is not the
/// same as it working. One number cannot say which half of the program to open:
/// `reached 24 of 24` prints happily alongside a course full of clipped posts,
/// because arriving at a gate and fitting through it are different contracts.
#[derive(Default)]
pub(crate) struct Report {
    /// How many gates it made a decision for. Clears it as a *pilot*.
    pub reached: u32,
    /// How many of those it planned to clear. Clears its *objective*: a plan
    /// that concedes gates will not score however well it is flown.
    pub planned: u32,
    /// Total shortfall of achieved clearance against planned. Clears its *aim* —
    /// whether the positions it plans are the positions it reaches — and it is
    /// the number that catches a controller optimising over places its glider
    /// cannot stand.
    pub aim_error: f32,
    /// How many decisions the sum above is over, so it reads as a mean.
    pub decisions: u32,
}

impl Report {
    /// The mean gap between a planned clearance and the achieved one.
    #[must_use]
    pub(crate) fn mean_aim_error(&self) -> f32 {
        if self.decisions == 0 {
            0.0
        } else {
            self.aim_error / self.decisions as f32
        }
    }
}

/// Every lateral position the glider can actually occupy `ticks` from now.
///
/// **This is the step a controller is most likely to skip, and skipping it makes
/// every number downstream a number about a future that will not happen.** A
/// glider driven by a held key moves a whole `GLIDE_SPEED * fixed_dt` per tick
/// and cannot stand between two of those steps. So its reachable set after `n`
/// ticks is not the interval `[x - n*step, x + n*step]` — it is the **lattice**
/// `x + k*step` for `-n <= k <= n`, clipped to the course.
///
/// Scoring arbitrary positions inside that interval scores fictions, and the
/// bad part is that it looks like it is working: every plan is achievable to
/// within half a step, the clearances come out plausible, and the run reports a
/// mean aim error with nothing to compare it to.
fn reachable(from: f32, ticks: u32, fixed_dt: f32) -> Vec<f32> {
    let step = GLIDE_SPEED * fixed_dt;
    let limit = COURSE_HALF_WIDTH - GLIDER_HALF_WIDTH;
    let reach = i32::try_from(ticks).unwrap_or(CANDIDATES).min(CANDIDATES);
    let mut out: Vec<f32> = Vec::new();
    for k in -reach..=reach {
        let x = (from + k as f32 * step).clamp(-limit, limit);
        // The clamp folds several lattice points onto each wall; keep one.
        if out.last().is_none_or(|last| (last - x).abs() > 1e-4) {
            out.push(x);
        }
    }
    out
}

/// How good standing at `x` is for gate `index`, counting the gates after it.
///
/// The objective, and what makes it more than "clear the next gate": a position
/// scores its own clearance **plus** how much of the following gates it leaves
/// reachable. Optimising the next gate alone is this game's version of taking
/// the best shot available — locally perfect, and it walks into a course it
/// cannot recover from.
fn score(x: f32, index: u32, phase: f32, seconds: f32, fixed_dt: f32) -> f32 {
    let own = clearance(x, gate_center_at(index, phase, seconds));
    if own < 0.0 {
        // A miss is worth less than any clear, however good the position it
        // leaves. Stated as a floor rather than a large negative constant, so
        // that two misses cannot add up to beat one.
        return own - 10.0;
    }
    let mut total = own;
    let mut weight = 0.5;
    for ahead in 1..=LOOKAHEAD {
        let next = index + ahead;
        if next >= GATES {
            break;
        }
        let ticks = ticks_to_fall(gate_depth(index), gate_depth(next), fixed_dt);
        // Where *that* gate will be when the glider gets there, which is a
        // different place from where it is now and from where this gate is.
        let target = gate_center_at(next, phase, seconds + ticks as f32 * fixed_dt);
        // Not "will I clear it" — "can I still get there", which is the only
        // question a position three gates early can honestly answer.
        let travel = GLIDE_SPEED * fixed_dt * ticks as f32;
        let shortfall = ((x - target).abs() - travel).max(0.0);
        total += weight * (GATE_HALF_GAP - shortfall);
        weight *= 0.5;
    }
    total
}

/// Where to be when gate `index` arrives, and the clearance that plan expects.
///
/// Predict, constrain, optimise, enumerate — and the order is not cosmetic.
/// Enumerating first builds a lattice around the wrong target; optimising before
/// constraining puts the answer on the boundary every single time.
fn plan(
    from: f32,
    at_depth: f32,
    index: u32,
    phase: f32,
    seconds: f32,
    fixed_dt: f32,
) -> (f32, f32) {
    // 4. Enumerate happens below, but the arrival tick is needed first, because
    //    it is what "where the gate will be" is a question about.
    let ticks = ticks_to_fall(at_depth, gate_depth(index), fixed_dt);
    let arrival = seconds + ticks as f32 * fixed_dt;
    // 1. Predict: where the gate *will be* when the glider gets there. Steering
    //    at where it is now arrives about three units late, which is twice the
    //    slack in the gap — see `Chaser`, which does exactly that on purpose.
    let target = gate_center_at(index, phase, arrival);
    // 2. Constrain: inside the gap, less the margin that keeps the optimum off
    //    the boundary.
    let inner = (GATE_HALF_GAP - GLIDER_HALF_WIDTH - EDGE_MARGIN).max(0.0);
    // 4. Enumerate: the positions the glider can actually stand on...
    let mut best: Option<(f32, f32)> = None;
    for x in reachable(from, ticks, fixed_dt) {
        if (x - target).abs() > inner {
            continue;
        }
        let value = score(x, index, phase, arrival, fixed_dt);
        if best.is_none_or(|(seen, _)| value > seen) {
            best = Some((value, x));
        }
    }
    match best {
        Some((_, x)) => (x, clearance(x, target)),
        // Nothing inside the gap is reachable from here. Fly at it anyway: a
        // controller that concedes reports a course as impossible when it is
        // merely hard, and `checks::the_course_is_completable` is what actually
        // answers that question.
        None => (target, clearance(target, target)),
    }
}

/// Send events, not states — what makes a key held for a hundred ticks press
/// exactly once. Shared by both pilots below, because getting it wrong is the
/// same bug twice.
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

/// Which way to lean to get from `at` to `want`.
///
/// A dead band half a step wide, rather than a comparison. Narrower and the
/// glider oscillates across the target for ever, one step each way — which is
/// what would make its arrival position unpredictable and the lattice above a
/// lie. The controller's own quantisation is something it has to *model*, and
/// this is where the model is enforced.
fn lean(at: f32, want: f32, step: f32) -> Option<Key> {
    if want - at > step * 0.5 {
        Some(Key::D)
    } else if at - want > step * 0.5 {
        Some(Key::A)
    } else {
        None
    }
}

/// The good pilot: predicts, constrains, optimises and enumerates.
pub(crate) struct Pilot {
    keyboard: SnapshotBuilder,
    holding: Option<Key>,
    /// The gate the current plan is for, where it aims, and what it expects.
    aim: Option<(u32, f32, f32)>,
    /// What it has learned about itself.
    pub report: Report,
}

impl Default for Pilot {
    fn default() -> Self {
        Self {
            keyboard: SnapshotBuilder::new(),
            holding: None,
            aim: None,
            report: Report::default(),
        }
    }
}

impl Pilot {
    /// Decide what to hold this tick.
    pub(crate) fn decide(
        &mut self,
        at: Vec2,
        next_gate: u32,
        phase: f32,
        seconds: f32,
        fixed_dt: f32,
    ) -> InputSnapshot {
        let mut want = None;
        if next_gate < GATES {
            let (target, expected) = plan(at.x, at.y, next_gate, phase, seconds, fixed_dt);
            if self.aim.is_none_or(|(gate, _, _)| gate != next_gate) {
                self.report.reached += 1;
                if expected >= 0.0 {
                    self.report.planned += 1;
                }
            }
            self.aim = Some((next_gate, target, expected));
            want = lean(at.x, target, GLIDE_SPEED * fixed_dt);
        }
        press(&mut self.keyboard, &mut self.holding, want);
        self.keyboard.first_tick_snapshot()
    }

    /// Record what a gate actually cost, against what the plan expected.
    pub(crate) fn observe(&mut self, gate: u32, achieved: f32) {
        if let Some((planned_gate, _, expected)) = self.aim
            && planned_gate == gate
        {
            self.report.aim_error += (expected - achieved).abs();
            self.report.decisions += 1;
        }
    }
}

/// The middle player of three: steers at the gate's *current* centre, nothing
/// more.
///
/// **This is the one that measures the game.** The pilot above says whether the
/// course is clearable by something that plans; a do-nothing run says whether it
/// can be failed. Neither says whether a *person* has a course worth flying, and
/// a game only a rollout controller can clear is a game nobody will enjoy. This
/// is what somebody does on their first try — chase what you can see — and the
/// gap between its score and the pilot's is the game's difficulty, measured
/// rather than asserted.
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
    /// Steer at where the next gate is right now.
    pub(crate) fn decide(
        &mut self,
        at: Vec2,
        next_gate: u32,
        phase: f32,
        seconds: f32,
        fixed_dt: f32,
    ) -> InputSnapshot {
        let want = if next_gate < GATES {
            // `seconds`, not the arrival time. That one substitution is the
            // whole difference between this pilot and the one above, and it is
            // worth two thirds of the course.
            lean(
                at.x,
                gate_center_at(next_gate, phase, seconds),
                GLIDE_SPEED * fixed_dt,
            )
        } else {
            None
        };
        press(&mut self.keyboard, &mut self.holding, want);
        self.keyboard.first_tick_snapshot()
    }
}
