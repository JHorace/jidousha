//! The speed-invariance sweep — the substrate's signature test (DESIGN §7).
//!
//! One scenario, one fixed script of orders at fixed world-times, replayed
//! under several speed scripts — all-1x, all-4x, and a mix with a mid-travel
//! pause — and the transcripts must contain **identical event sequences with
//! identical world-time stamps**. A divergence is the exact failure this
//! design exists to prevent.
//!
//! The conductor drives the real game the way a player does: every order and
//! every speed change goes through the `InputSnapshot` (a `SnapshotBuilder`,
//! so the edges are the driver's own), clicks land on the same rectangles a
//! person clicks, and the order script is addressed in **world-minutes** —
//! the conductor watches the clock and acts when the minute arrives, which is
//! what "orders at fixed world-times" means under a speed schedule it does
//! not know in advance.

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FrameRecord, FrameRecorder, InputEvent, SnapshotBuilder,
};

use crate::camera::UiMap;
use crate::checks::Checks;
use crate::clock::Clock;
use crate::constants::Tuning;
use crate::flow::{Flow, SessionSeed};
use crate::grid::{LOCATIONS, Tile};
use crate::modules::ModuleSet;
use crate::sim::{Event, EventClass, Sim};
use crate::{camera, layout, sim, sprites, verify};

/// When a directive falls due.
#[derive(Clone, Copy, Debug)]
pub enum When {
    /// At this engine tick.
    Tick(u64),
    /// The first tick the world clock reads at least this minute.
    Minute(u64),
    /// This many ticks after the clock first read `minute` — how a resume is
    /// addressed while the clock it would otherwise be addressed by is
    /// holding.
    MinuteHeld {
        /// The minute whose first arrival starts the count.
        minute: u64,
        /// Ticks after that arrival.
        after: u64,
    },
}

/// What a directive does.
#[derive(Clone, Copy, Debug)]
pub enum Act {
    /// Tap a key: pressed and released on one tick, like `InputScript::press`.
    Tap(Key),
    /// Move the pointer to a UI point (960x540 space), then click it.
    ClickUi(Vec2),
    /// The same, at a world point — a site marker on the map.
    ClickWorld(Vec2),
    /// Move the pointer to a UI point without clicking — hover.
    PointUi(Vec2),
    /// Move the pointer to a world point without clicking.
    PointWorld(Vec2),
}

/// One scripted action at one moment.
#[derive(Clone, Copy, Debug)]
pub struct Directive {
    /// When.
    pub when: When,
    /// What.
    pub what: Act,
}

/// A dispatch order, as the scripts state one: this party to this site, at
/// this world-minute — two clicks through the real UI.
pub fn order(minute: u64, party: usize, site: usize) -> [Directive; 2] {
    let marker = layout::marker_rect(LOCATIONS[sim::site_location(site)].tile);
    [
        Directive {
            when: When::Minute(minute),
            what: Act::ClickUi(layout::party_chip(party).center()),
        },
        Directive {
            when: When::Minute(minute),
            what: Act::ClickWorld(marker.center()),
        },
    ]
}

/// A frame to keep: its name, and the first moment at or after which it is
/// photographed — both gates must have passed, so a drawer session's photo
/// can be tick-addressed while the clock holds at zero.
#[derive(Clone, Copy, Debug)]
pub struct Photo {
    /// The capture's name.
    pub name: &'static str,
    /// The world-minute to photograph at (0 for "any").
    pub minute: u64,
    /// The tick to photograph at (0 for "any").
    pub tick: u64,
}

/// What one conducted session produced.
pub struct Conducted {
    /// Every event, in firing order — the transcript.
    pub events: Vec<Event>,
    /// The treasury at the end.
    pub treasury: i64,
    /// The clock at the end.
    pub minutes: u64,
    /// The whole sim at the end.
    pub sim: Sim,
    /// Ticks run.
    pub ticks: u64,
    /// Every phase and its systems, in run order.
    pub schedule: String,
    /// The photographs asked for, by name, in the order taken — each with
    /// the sim and clock state of the tick it was drawn on, so a check can
    /// rebuild what the frame ought to show.
    pub photos: Vec<Shot>,
    /// Which backend texture the font landed on, when frames were recorded.
    pub font: BackendTextureId,
    /// Flow/tuning/sim/clock snapshots at the probe ticks asked for.
    pub probes: Vec<(u64, Flow, Tuning, Sim, Clock)>,
}

/// One photograph and the state it was taken in.
pub struct Shot {
    /// The photo's name.
    pub name: &'static str,
    /// The frame.
    pub frame: FrameRecord,
    /// The sim at that tick.
    pub sim: Sim,
    /// The clock at that tick.
    pub clock: Clock,
    /// The flow at that tick.
    pub flow: Flow,
}

impl Conducted {
    /// The photo with this name, if it was taken.
    pub fn photo(&self, name: &str) -> Option<&Shot> {
        self.photos.iter().find(|shot| shot.name == name)
    }

    /// The probe at this tick, if one was taken.
    pub fn probe(&self, tick: u64) -> Option<&(u64, Flow, Tuning, Sim, Clock)> {
        self.probes.iter().find(|(at, ..)| *at == tick)
    }
}

/// Everything a conducted run can be asked for.
pub struct Session<'a> {
    /// The constants to plant.
    pub tuning: Tuning,
    /// Which modules are on. The module-off matrix (GDD §9) is a list of
    /// these; a played run uses `ModuleSet::ALL`.
    pub modules: ModuleSet,
    /// The seed to plant (`None` runs at the authored zero).
    pub seed: Option<u64>,
    /// The script, in due order — the conductor consumes it front-first.
    pub directives: &'a [Directive],
    /// Frames to keep — recording happens only if this is non-empty.
    pub photos: &'a [Photo],
    /// Ticks to snapshot Flow/Tuning/Sim/Clock at.
    pub probe_ticks: &'a [u64],
    /// The surface to draw to.
    pub viewport: PhysicalSize,
    /// Stop after this many ticks even if the world is not at rest.
    pub max_ticks: u64,
    /// Stop early once the world is at rest and the script is spent.
    pub stop_at_rest: bool,
}

impl Session<'_> {
    /// The common shape: a run at the reference viewport, no photos.
    pub fn plain(tuning: Tuning, directives: &[Directive], max_ticks: u64) -> Session<'_> {
        Session {
            tuning,
            modules: ModuleSet::ALL,
            seed: None,
            directives,
            photos: &[],
            probe_ticks: &[],
            viewport: verify::HEADLESS_VIEWPORT,
            max_ticks,
            stop_at_rest: true,
        }
    }
}

/// Drive one session and report what happened.
pub fn conduct(session: &Session<'_>) -> Conducted {
    let config = GameConfig {
        seed: session.seed.unwrap_or(0),
        ..crate::config()
    };
    let mut sim = headless(config, crate::register);
    sim.world_mut().insert_resource(session.tuning);
    sim.world_mut().insert_resource(session.modules);
    sim.world_mut().insert_resource(SessionSeed(session.seed));
    sim.world_mut()
        .insert_resource(camera::Surface(session.viewport));

    let mut recorder = (!session.photos.is_empty()).then(|| FrameRecorder::new(session.viewport));
    let font = recorder
        .as_ref()
        .map_or(BackendTextureId(0), FrameRecorder::font_texture);

    // The minutes any MinuteHeld directive counts from, with the tick each
    // was first reached — filled as the clock passes them.
    let mut held_minutes: Vec<(u64, Option<u64>)> = session
        .directives
        .iter()
        .filter_map(|directive| match directive.when {
            When::MinuteHeld { minute, .. } => Some((minute, None)),
            _ => None,
        })
        .collect();

    let mut keyboard = SnapshotBuilder::new();
    // The microstep queue: the directive being executed, expanded so a click
    // is a move on one tick and the press on the next.
    let mut steps: Vec<Act> = Vec::new();
    let mut next_directive = 0usize;
    let mut photos: Vec<Shot> = Vec::new();
    let mut probes = Vec::new();
    let mut ticks = 0u64;

    for tick in 1..=session.max_ticks {
        ticks = tick;
        // What the clock read after the last tick — the conductor's view of
        // the world it is about to act on.
        let minutes = if tick == 1 {
            0
        } else {
            sim.world().resource::<Clock>().minutes
        };
        for (minute, first) in held_minutes.iter_mut() {
            if first.is_none() && minutes >= *minute {
                *first = Some(tick);
            }
        }

        // Start the next due directive, if nothing is mid-execution.
        if steps.is_empty()
            && let Some(directive) = session.directives.get(next_directive)
        {
            let due = match directive.when {
                When::Tick(at) => tick >= at,
                When::Minute(minute) => minutes >= minute,
                When::MinuteHeld { minute, after } => held_minutes
                    .iter()
                    .find(|(m, _)| *m == minute)
                    .and_then(|(_, first)| *first)
                    .is_some_and(|first| tick >= first + after),
            };
            if due {
                match directive.what {
                    Act::Tap(key) => steps.push(Act::Tap(key)),
                    Act::PointUi(at) => steps.push(Act::PointUi(at)),
                    Act::PointWorld(at) => steps.push(Act::PointWorld(at)),
                    Act::ClickUi(at) => {
                        steps.push(Act::PointUi(at));
                        steps.push(Act::ClickUi(at));
                    }
                    Act::ClickWorld(at) => {
                        steps.push(Act::PointWorld(at));
                        steps.push(Act::ClickWorld(at));
                    }
                }
                next_directive += 1;
            }
        }
        // Feed one microstep per tick.
        if !steps.is_empty() {
            let step = steps.remove(0);
            let world_camera = *sim.world().resource::<Camera>();
            let ui_map = UiMap::for_camera(&world_camera);
            match step {
                Act::Tap(key) => {
                    keyboard.record(InputEvent::KeyPressed(key));
                    keyboard.record(InputEvent::KeyReleased(key));
                }
                Act::PointUi(at) => keyboard.record(InputEvent::PointerMoved {
                    id: PointerId::PRIMARY,
                    screen: world_camera.world_to_screen(ui_map.to_world(at)),
                }),
                Act::PointWorld(at) => keyboard.record(InputEvent::PointerMoved {
                    id: PointerId::PRIMARY,
                    screen: world_camera.world_to_screen(at),
                }),
                Act::ClickUi(_) | Act::ClickWorld(_) => {
                    keyboard.record(InputEvent::ButtonPressed {
                        id: PointerId::PRIMARY,
                        button: PointerButton::Primary,
                    });
                    keyboard.record(InputEvent::ButtonReleased {
                        id: PointerId::PRIMARY,
                        button: PointerButton::Primary,
                    });
                }
            }
        }

        sim.world_mut()
            .insert_resource(Input::new(keyboard.first_tick_snapshot()));
        sim.tick();
        if tick == 1 && recorder.is_some() {
            // The art, before the first frame is photographed — what keeps a
            // recorded run reproducible now that the pictures come off a
            // disk.
            let assets = sim.world_mut().resource_mut::<Assets>();
            if let Some(failure) = sprites::settle(assets).first() {
                crate::checks::fail(
                    "ninjo's art did not load for a recorded run",
                    &crate::checks::one_line(&failure.message()),
                );
            }
        }
        if let Some(recorder) = recorder.as_mut() {
            recorder.settle_assets(&mut sim, tick);
            let now = sim.world().resource::<Clock>().minutes;
            let due: Vec<Photo> = session
                .photos
                .iter()
                .filter(|photo| {
                    now >= photo.minute
                        && tick >= photo.tick
                        && !photos.iter().any(|shot| shot.name == photo.name)
                })
                .copied()
                .collect();
            if !due.is_empty() {
                let frame = recorder.draw(&mut sim);
                for photo in due {
                    photos.push(Shot {
                        name: photo.name,
                        frame: frame.clone(),
                        sim: sim.world().resource::<Sim>().clone(),
                        clock: *sim.world().resource::<Clock>(),
                        flow: sim.world().resource::<Flow>().clone(),
                    });
                }
            }
        }
        if session.probe_ticks.contains(&tick) {
            probes.push((
                tick,
                sim.world().resource::<Flow>().clone(),
                *sim.world().resource::<Tuning>(),
                sim.world().resource::<Sim>().clone(),
                *sim.world().resource::<Clock>(),
            ));
        }
        if session.stop_at_rest
            && next_directive >= session.directives.len()
            && steps.is_empty()
            && photos.len() == session.photos.len()
            && sim.world().resource::<Sim>().at_rest()
        {
            break;
        }
    }

    let world = sim.world();
    Conducted {
        events: world.resource::<Sim>().events.clone(),
        treasury: world.resource::<Sim>().treasury,
        minutes: world.resource::<Clock>().minutes,
        sim: world.resource::<Sim>().clone(),
        ticks,
        schedule: sim.schedule_debug(),
        photos,
        font,
        probes,
    }
}

/// The shared order script under one speed prologue: four dispatches at
/// fixed world-minutes — three parties out at once, then a re-dispatch to
/// the barrier-detour site once OX is home.
fn script_with(speed_prologue: &[Directive]) -> Vec<Directive> {
    let mut script = speed_prologue.to_vec();
    script.extend(order(8, 0, 0)); // OX to the Watchtower
    script.extend(order(12, 1, 1)); // OWL to the Deep Cave
    script.extend(order(20, 2, 2)); // CRANE to the Old Crypt
    script.extend(order(330, 0, 3)); // OX again, to the Black Vault
    script
}

/// The three speed scripts (DESIGN §7). The mixed script changes speed
/// mid-travel, pauses exactly when CRANE's order falls due — so one dispatch
/// happens against a held clock, the orders-while-paused property run for
/// real — and resumes 300 ticks later.
pub fn speed_scripts() -> Vec<(&'static str, Vec<Directive>)> {
    let tap = |when: When, key: Key| Directive {
        when,
        what: Act::Tap(key),
    };
    let mut mixed = vec![tap(When::Tick(5), Key::Digit2)];
    mixed.extend(order(8, 0, 0));
    mixed.extend(order(12, 1, 1));
    mixed.push(tap(When::Minute(20), Key::Space));
    mixed.extend(order(20, 2, 2));
    mixed.push(tap(
        When::MinuteHeld {
            minute: 20,
            after: 300,
        },
        Key::Space,
    ));
    mixed.push(tap(When::Minute(40), Key::Digit3));
    mixed.extend(order(330, 0, 3));
    vec![
        ("all-1x", script_with(&[tap(When::Tick(5), Key::Digit1)])),
        ("all-4x", script_with(&[tap(When::Tick(5), Key::Digit3)])),
        ("mixed-pause", mixed),
    ]
}

/// The expected transcript at the shipped constants — every world-time a sum
/// of authored terrain costs along the asserted routes (the map's comment in
/// `grid.rs` walks the arithmetic; `verify::path_contracts` asserts the
/// routes themselves).
pub fn expected_events() -> Vec<(u64, EventClass, usize, usize)> {
    // (minute, class, party, location index)
    vec![
        (8, EventClass::Departed, 0, 0),
        (12, EventClass::Departed, 1, 0),
        (20, EventClass::Departed, 2, 0),
        (71, EventClass::Arrived, 1, 2),
        (71, EventClass::WorkBegan, 1, 2),
        (76, EventClass::Arrived, 2, 3),
        (76, EventClass::WorkBegan, 2, 3),
        (102, EventClass::Arrived, 0, 1),
        (102, EventClass::WorkBegan, 0, 1),
        (161, EventClass::QuestComplete, 1, 2),
        (176, EventClass::QuestComplete, 2, 3),
        (215, EventClass::Returned, 1, 0),
        (222, EventClass::QuestComplete, 0, 1),
        (230, EventClass::Returned, 2, 0),
        (316, EventClass::Returned, 0, 0),
        (330, EventClass::Departed, 0, 0),
        (426, EventClass::Arrived, 0, 4),
        (426, EventClass::WorkBegan, 0, 4),
        (606, EventClass::QuestComplete, 0, 4),
        (702, EventClass::Returned, 0, 0),
    ]
}

/// The treasury the script ends on: every pot, paid once.
pub const EXPECTED_TREASURY: i64 = 60 + 40 + 45 + 80;

/// A transcript reduced to what invariance is about: address, class, party,
/// place.
pub fn transcript(events: &[Event]) -> Vec<(u64, &'static str, usize, Tile, Option<usize>)> {
    events
        .iter()
        .map(|event| {
            (
                event.minute,
                event.class.name(),
                event.party,
                event.tile,
                event.location,
            )
        })
        .collect()
}

/// The exact-time judge: the fixed script's whole event list, to the minute,
/// and the pot arithmetic (DESIGN §7).
pub fn judge_orders(checks: &mut Checks, run: &Conducted, label: &str) {
    let got: Vec<(u64, EventClass, usize, Option<usize>)> = run
        .events
        .iter()
        .map(|event| (event.minute, event.class, event.party, event.location))
        .collect();
    let wanted: Vec<(u64, EventClass, usize, Option<usize>)> = expected_events()
        .into_iter()
        .map(|(minute, class, party, location)| (minute, class, party, Some(location)))
        .collect();
    checks.require(
        got == wanted,
        "the fixed order script did not produce its asserted timeline",
        format!(
            "{label}: the transcript is {got:?} and the authored arithmetic says {wanted:?}; \
             every expected minute is a sum of terrain costs along the stored route"
        ),
    );
    checks.require(
        run.treasury == EXPECTED_TREASURY,
        "the pots did not pay what the quests promise",
        format!(
            "{label}: the treasury ended at {}g and the four pots sum to {}g",
            run.treasury, EXPECTED_TREASURY
        ),
    );
    checks.require(
        run.sim.at_rest(),
        "the fixed script ended with the world still moving",
        format!(
            "{label}: after {} ticks something is still scheduled or abroad",
            run.ticks
        ),
    );
}

/// The pacing probes: the clock's own constants, tick for minute. 304 ticks
/// with the speed set on tick 5 leaves exactly 300 accumulating ticks.
///
/// `expected` is the **shipped** arithmetic as literals — 10, 20 and 40
/// minutes — stated by the caller rather than recomputed from `tuning`,
/// because a check that derives its expectation from the constant under test
/// cannot see that constant move (the mutation round leans on this).
pub fn judge_pacing(checks: &mut Checks, tuning: Tuning, expected: [u64; 3]) -> Vec<String> {
    let mut notes = Vec::new();
    for ((key, label), want) in [
        (Key::Digit1, "1x"),
        (Key::Digit2, "2x"),
        (Key::Digit3, "4x"),
    ]
    .into_iter()
    .zip(expected)
    {
        let script = [Directive {
            when: When::Tick(5),
            what: Act::Tap(key),
        }];
        let mut session = Session::plain(tuning, &script, 304);
        session.stop_at_rest = false;
        let conducted = conduct(&session);
        checks.require(
            conducted.minutes == want,
            "the clock does not pace at its named constants",
            format!(
                "300 ticks at {label} carried {} world-minutes and the shipped arithmetic \
                 (minute_ticks 30; accumulations 1/2/4) says {want}",
                conducted.minutes
            ),
        );
        notes.push(format!("{label}={}m", conducted.minutes));
    }
    notes
}

/// The shipped pacing: 300 accumulating ticks at each speed.
pub const SHIPPED_PACING: [u64; 3] = [10, 20, 40];

/// The sweep itself: the same orders under three speed scripts, transcripts
/// identical to the stamp; the exact-time judge on each; the pacing probes.
/// Returns the summary and the all-1x run, which later checks read.
pub fn run(checks: &mut Checks) -> (String, Conducted) {
    let tuning = Tuning::SHIPPED;
    let mut runs: Vec<(&'static str, Conducted)> = Vec::new();
    for (name, script) in speed_scripts() {
        let conducted = conduct(&Session::plain(tuning, &script, 60_000));
        runs.push((name, conducted));
    }

    // The invariance claim: identical event sequences with identical
    // world-time stamps, under any speed script.
    let baseline = transcript(&runs[0].1.events);
    for (name, conducted) in runs.iter().skip(1) {
        let this = transcript(&conducted.events);
        checks.require(
            this == baseline,
            "the event transcript depends on the speed schedule",
            format!(
                "under {} the transcript is {this:?}; under {} it is {baseline:?} - every \
                 occurrence's world-time must be independent of the speed schedule (DESIGN §4)",
                name, runs[0].0
            ),
        );
    }
    // And the exact authored timeline, judged on every script, so a drift
    // that moved all three together is still caught.
    for (name, conducted) in &runs {
        judge_orders(checks, conducted, name);
    }
    let pacing = judge_pacing(checks, tuning, SHIPPED_PACING);

    let summary = format!(
        "speed-invariance sweep: {} scripts, {} events each, treasury {}g, pacing {}",
        runs.len(),
        runs[0].1.events.len(),
        runs[0].1.treasury,
        pacing.join(" "),
    );
    let (_, baseline_run) = runs.remove(0);
    (summary, baseline_run)
}
