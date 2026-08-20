//! The `--verify` mode: fly the course three ways, assert, and say so.
//!
//! Scripted input, a bounded number of headless ticks, assertions about what the
//! world did and what was drawn, one verdict line beginning `verified `, and
//! then the evidence. `tools/verify slalom` runs exactly this.
//!
//! **Three players, one line of verdict each**, because one controller cannot
//! measure a game's difficulty — all it can say is whether the game is beatable
//! *by that controller*:
//!
//! | player | what it clears | what that clears *for you* |
//! |---|---|---|
//! | the pilot | nearly all of it | the mechanics work |
//! | the chaser | some of it | there is a decision in this course |
//! | nobody | almost none | the game can be lost |
//!
//! The middle one is the one nothing tells you to write, and it is the only one
//! that can say the course is worth flying.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder, Input, InputSnapshot, PhysicalSize};

use crate::checks::{self, Check};
use crate::controller::{Chaser, Pilot};
use crate::{Course, GATES, GLIDER_HALF_WIDTH, Glider, register};

/// The recorder's viewport, and the camera's. One number, so the two agree and
/// the trap in *Testing your game* stops existing.
const VIEWPORT: PhysicalSize = PhysicalSize::new(1280, 720);

/// The seeds this check flies.
///
/// **One course is one course.** The gates' phase comes from the seeded `Rng`,
/// so a single run measures the one arrangement that seed happens to produce —
/// and a game tuned until *that* course separates a planner from a chaser is a
/// game tuned to a coincidence. Three of them cost three seconds and they are
/// what makes the difficulty claim below about the game rather than about seed
/// zero.
const SEEDS: [u64; 3] = [0, 7, 4_242];

/// How many ticks a course gets before the run gives up on it finishing.
///
/// A course is `GATES * GATE_SPACING` deep and the glider falls at a fixed rate,
/// so the real number is known; this is the guard against a game that stops
/// descending, which would otherwise hang rather than fail.
const TICK_LIMIT: u32 = 4_000;

/// Anything that can fly the course.
///
/// A trait rather than a closure because a pilot needs to *observe* as well as
/// decide — what it planned against what it got — and two closures cannot share
/// one `&mut`.
trait Autopilot {
    /// What to hold this tick.
    fn decide(
        &mut self,
        at: Vec2,
        next_gate: u32,
        phase: f32,
        seconds: f32,
        fixed_dt: f32,
    ) -> InputSnapshot;

    /// A gate was judged. Most pilots do not care.
    fn observe(&mut self, _gate: u32, _clearance: f32) {}
}

impl Autopilot for Pilot {
    fn decide(
        &mut self,
        at: Vec2,
        next_gate: u32,
        phase: f32,
        seconds: f32,
        fixed_dt: f32,
    ) -> InputSnapshot {
        Pilot::decide(self, at, next_gate, phase, seconds, fixed_dt)
    }

    fn observe(&mut self, gate: u32, clearance: f32) {
        Pilot::observe(self, gate, clearance);
    }
}

impl Autopilot for Chaser {
    fn decide(
        &mut self,
        at: Vec2,
        next_gate: u32,
        phase: f32,
        seconds: f32,
        fixed_dt: f32,
    ) -> InputSnapshot {
        Chaser::decide(self, at, next_gate, phase, seconds, fixed_dt)
    }
}

/// The third player: present, and doing nothing.
///
/// Not the same as inserting no `Input` at all, and it is what proves the game
/// can be *lost*.
#[derive(Default)]
struct Idle;

impl Autopilot for Idle {
    fn decide(&mut self, _: Vec2, _: u32, _: f32, _: f32, _: f32) -> InputSnapshot {
        InputSnapshot::new()
    }
}

/// What one flight of the course came to.
struct Flight {
    /// How many gates were cleared.
    cleared: usize,
    /// How many ticks it took.
    ticks: u32,
    /// The course's drift phase, so a check can ask about the same course.
    phase: f32,
    /// The last frame drawn **while the course was still being flown**.
    ///
    /// Not the last frame drawn: that one is the finished screen, and every
    /// geometric assertion here wants a picture of the game being *played*.
    live: Option<FrameRecord>,
    /// Where the glider was on that frame, carried out of the loop with it.
    live_at: Vec2,
    /// The camera on that frame, likewise — a check comparing bounds from a
    /// different tick's camera is comparing against the wrong rectangle.
    live_camera: Camera,
    /// The timestep the engine actually handed this run, so a check can state a
    /// bound in the game's own quantum rather than against an assumed 1/60.
    fixed_dt: f32,
}

/// Fly the whole course with `pilot`, recording frames if asked.
fn fly(seed: u64, pilot: &mut dyn Autopilot, record: bool) -> Flight {
    let config = GameConfig {
        seed,
        ..GameConfig::default()
    };
    let mut sim = headless(config, register);
    let mut recorder = FrameRecorder::new(VIEWPORT);
    // Read the timestep from the engine rather than assuming 1/60: a game that
    // hard-codes it is a game that breaks when `GameConfig` changes.
    sim.tick();
    let fixed_dt = sim.world().resource::<Time>().fixed_dt.as_f32();
    let mut flight = Flight {
        cleared: 0,
        ticks: 0,
        phase: sim.world().resource::<Course>().phase,
        live: None,
        live_at: Vec2::ZERO,
        live_camera: Camera::default(),
        fixed_dt,
    };
    let mut reported = 0_usize;
    for tick in 1..=TICK_LIMIT {
        flight.ticks = tick;
        let (at, next_gate, seconds, finished) = {
            let world = sim.world();
            let course = world.resource::<Course>();
            let seconds = world.resource::<Time>().elapsed.as_f32();
            let at = world
                .query::<(&Transform, With<Glider>)>()
                .next()
                .map_or(Vec2::ZERO, |(_, transform, _)| transform.pos);
            (at, course.next_gate, seconds, course.finished())
        };
        if finished {
            break;
        }
        let snapshot = pilot.decide(at, next_gate, flight.phase, seconds, fixed_dt);
        sim.world_mut().insert_resource(Input::new(snapshot));
        sim.tick();

        // Anything judged this tick goes back to the pilot, so it can compare
        // what it planned against what it got. That comparison is the third of
        // the three numbers and the one nobody writes unprompted.
        let judged: Vec<(u32, f32)> = {
            let course = sim.world().resource::<Course>();
            course.clearances[reported..].to_vec()
        };
        reported += judged.len();
        for (gate, room) in judged {
            pilot.observe(gate, room);
        }

        if record {
            let frame = recorder.draw(&mut sim);
            let world = sim.world();
            flight.live_at = world
                .query::<(&Transform, With<Glider>)>()
                .next()
                .map_or(Vec2::ZERO, |(_, transform, _)| transform.pos);
            flight.live_camera = *world.resource::<Camera>();
            flight.live = Some(frame);
        }
    }
    flight.cleared = sim.world().resource::<Course>().cleared();
    flight
}

/// Run every check, print the verdict, and return whether it held.
pub(crate) fn run() -> ExitCode {
    let mut failures: Vec<String> = Vec::new();

    // --- the three players, over three courses --------------------------
    let mut pilot = Pilot::default();
    let flown = fly(SEEDS[0], &mut pilot, true);
    let mut table: Vec<(u64, usize, usize, usize)> = Vec::new();
    for seed in SEEDS {
        let planned = fly(seed, &mut Pilot::default(), false);
        let chased = fly(seed, &mut Chaser::default(), false);
        let idled = fly(seed, &mut Idle, false);
        // The game, before any controller is blamed for it. Asked per course,
        // because it is a question about the course.
        failures.extend(checks::the_course_is_completable(planned.phase).err());
        failures.extend(checks::the_gates_stay_inside(planned.phase).err());
        failures.extend(
            checks::the_gap_between_pilots_is_a_game(planned.cleared, chased.cleared).err(),
        );
        if idled.cleared >= chased.cleared {
            failures.push(format!(
                "on seed {seed}, doing nothing cleared {} gates and chasing cleared {} — \
                 this course does not ask the player for anything",
                idled.cleared, chased.cleared
            ));
        }
        table.push((seed, planned.cleared, chased.cleared, idled.cleared));
    }

    // --- the controller's contract on itself ----------------------------
    if pilot.report.reached < GATES {
        failures.push(format!(
            "the pilot decided for only {} of {GATES} gates, so it left the course early \
             and every number after this is about a flight that did not happen",
            pilot.report.reached
        ));
    }
    if pilot.report.planned < GATES {
        failures.push(format!(
            "the pilot planned to clear {} of {GATES}: its objective concedes gates, \
             so no amount of flying accuracy will score them",
            pilot.report.planned
        ));
    }
    // **The bound is the game's own quantum, not a number that looked right.**
    // The glider moves a whole `GLIDE_SPEED * fixed_dt` a tick and cannot stand
    // between two of those, so no controller can do better than half a step;
    // the arrival tick is an integer too, so the gate's own position is known
    // only to within one tick of its travel. One glide step is the honest
    // ceiling, and writing it this way means the check survives someone
    // changing the speeds — an `aim > 0.05` would not.
    let step = flown.fixed_dt * crate::GLIDE_SPEED;
    let aim = pilot.report.mean_aim_error();
    if aim > step {
        failures.push(format!(
            "the pilot's flying lands {aim:.3} units from where it planned, over {} \
             decisions, against a quantisation step of {step:.3} — it is optimising over \
             positions the glider cannot occupy. Enumerate the reachable lattice \
             rather than the interval",
            pilot.report.decisions
        ));
    }

    // --- the schedule this game chose -----------------------------------
    {
        let sim = headless(GameConfig::default(), register);
        failures.extend(checks::the_schedule_is_the_one_we_chose(&sim.schedule_debug()).err());
    }

    // --- what was drawn, on a frame from while it was being played ------
    match flown.live.as_ref() {
        None => failures.push("no frame was ever recorded".to_string()),
        Some(frame) => {
            let mut note = |check: Check| failures.extend(check.err());
            note(checks::the_sky_is_dark_enough(frame));
            note(checks::everything_is_on_screen(frame, &flown.live_camera));
            note(checks::the_course_fits_the_view(&flown.live_camera));
            note(checks::the_glider_is_drawn_at(
                frame,
                flown.live_at,
                GLIDER_HALF_WIDTH,
            ));
            let recorder = FrameRecorder::new(VIEWPORT);
            note(checks::the_glider_is_in_front(
                frame,
                flown.live_at,
                checks::font_of(&recorder),
            ));
        }
    }
    failures.extend(checks::every_glyph_is_printable("A/D steer  the gates drift").err());
    failures.extend(checks::every_glyph_is_printable("course complete").err());

    if !failures.is_empty() {
        eprintln!("slalom --verify FAILED, {} check(s):", failures.len());
        for (index, failure) in failures.iter().enumerate() {
            eprintln!("  {}. {failure}", index + 1);
        }
        return ExitCode::FAILURE;
    }

    println!(
        "verified slalom: {} of {GATES} gates in {} ticks, over {} courses",
        flown.cleared,
        flown.ticks,
        SEEDS.len()
    );
    println!("    seed   pilot  chaser  nobody   (gates of {GATES})");
    for (seed, planned, chased, idled) in &table {
        println!("    {seed:>5}   {planned:>5}  {chased:>6}  {idled:>6}");
    }
    println!(
        "    the pilot reached {} of {GATES}, planned {}, and its flying missed its \
         plans by {:.3} units on average over {} decisions",
        pilot.report.reached,
        pilot.report.planned,
        pilot.report.mean_aim_error(),
        pilot.report.decisions
    );
    if let Some(frame) = flown.live.as_ref() {
        println!("{}", frame.transcript());
    }
    ExitCode::SUCCESS
}
