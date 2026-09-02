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
    BackendTextureId, FingerId, FrameRecord, FrameRecorder, InputEvent, SnapshotBuilder,
};

use crate::attention::EventClass;
use crate::camera::UiMap;
use crate::checks::Checks;
use crate::clock::Clock;
use crate::constants::Tuning;
use crate::flow::{Flow, SessionSeed};
use crate::grid::{LOCATIONS, Tile};
use crate::modules::ModuleSet;
use crate::sim::{Event, Sim};
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
    /// **`lead` ticks before the clock reaches `minute`** — so a multi-tick
    /// input sequence *lands* at that world-minute rather than starting at it.
    ///
    /// This is how an order is addressed in world-time under a clock that
    /// carries more than a minute per tick. At the shipped speeds 4x carries
    /// 1.6 world-minutes a tick, so a click begun when the minute arrives
    /// takes effect several minutes later — and by a different several at
    /// every speed, which would make the invariance sweep a test of the
    /// conductor rather than of the world. The conductor simulates the clock
    /// forward `lead` ticks at the rate it is running and starts the sequence
    /// when that lands on or past the target.
    Approaching {
        /// The world-minute the action is to take effect at.
        minute: u64,
        /// How many ticks the sequence takes before its effect lands.
        lead: u64,
    },
    /// As soon as the directive before it has finished — the tail of a
    /// sequence whose head was addressed in world-time.
    Now,
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
    /// **Tap a world point with a finger** — a touch down and a touch up, and
    /// no pointer event at all.
    ///
    /// The engine mirrors the first finger onto the primary pointer
    /// (`jidousha-api.md`: "a game written for a mouse is already playable by
    /// touch"), so this exists to *verify* that claim over this game's own
    /// hit-tests rather than to build anything: the game has no touch code.
    TouchWorld(Vec2),
    /// The down half of a [`Act::TouchWorld`], as the conductor microsteps it.
    TouchDown(Vec2),
    /// And the up half.
    TouchUp(Vec2),
}

/// One scripted action at one moment.
#[derive(Clone, Copy, Debug)]
pub struct Directive {
    /// When.
    pub when: When,
    /// What.
    pub what: Act,
}

/// How many ticks an order takes from its first microstep to the tick its
/// dispatch lands on: point, click, point, click.
pub const ORDER_LEAD: u64 = 3;

/// A dispatch order, as the scripts state one: this party to this site, **at
/// this world-minute** — two clicks through the real UI, begun early enough
/// that the second one lands on the minute it names.
pub fn order(minute: u64, party: usize, site: usize) -> [Directive; 2] {
    let marker = layout::marker_rect(LOCATIONS[sim::site_location(site)].tile);
    [
        Directive {
            when: When::Approaching {
                minute,
                lead: ORDER_LEAD,
            },
            what: Act::ClickUi(layout::party_chip(party).center()),
        },
        Directive {
            when: When::Now,
            what: Act::ClickWorld(marker.center()),
        },
    ]
}

/// The world-minute the clock will read `ticks` ticks from now, at the rate it
/// is running — the look-ahead [`When::Approaching`] is addressed by.
///
/// Integer, and exactly the arithmetic `clock::advance` performs, so the
/// prediction and the clock cannot disagree. A held clock predicts itself,
/// which is what makes an order issued while paused land at the held minute.
fn minutes_after(clock: &Clock, tuning: &Tuning, ticks: u64) -> u64 {
    let step = clock.accumulation(tuning);
    let per_minute = tuning.minute_ticks.max(1);
    let mut accum = clock.accum;
    let mut minutes = clock.minutes;
    for _ in 0..ticks {
        accum += step;
        while accum >= per_minute {
            accum -= per_minute;
            minutes += 1;
        }
    }
    minutes
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
    /// Whether to wait for the world to have stopped itself.
    ///
    /// A picture *of* an auto-pause cannot be addressed by the minute the
    /// event fires at: the clock is coarser than a minute at speed, and the
    /// resume is a few ticks behind. The pause is a sim fact, so the gate is
    /// the fact.
    pub paused: bool,
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
    /// **Tap this key `n` ticks after the world stops itself.**
    ///
    /// The player, resuming. A run under a config that pauses on a class the
    /// scenario fires a dozen times needs a dozen resumes, and addressing each
    /// of them by world-minute would make the script a list of literals that
    /// the next content change invalidates. The world stopping itself is a
    /// *sim fact* the conductor can read, so this reads it and presses the key
    /// the script is already running at — which is what a player does, and
    /// leaves the world-times on the far side unchanged.
    pub resume_after: Option<(Key, u64)>,
    /// **Stop once the world clock reads this minute.** The stopping condition
    /// a living world needs: with autonomy on, characters keep choosing, so
    /// "at rest" is a lull rather than an ending — and two speed scripts can
    /// only be compared over the same span of *world*-time.
    pub stop_at_minute: Option<u64>,
}

impl Session<'_> {
    /// The common shape: a run at the reference viewport, no photos, stopped
    /// at the world-minute the sweep compares over.
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
            stop_at_rest: false,
            stop_at_minute: Some(RUN_UNTIL),
            resume_after: None,
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
    // The tick the world last stopped itself on, for `resume_after`.
    let mut paused_since: Option<u64> = None;
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
                When::Now => true,
                When::Approaching { minute, lead } => {
                    minutes >= minute
                        || sim.world().find_resource::<Clock>().is_some_and(|clock| {
                            minutes_after(clock, &session.tuning, lead) >= minute
                        })
                }
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
                    Act::TouchWorld(at) => {
                        steps.push(Act::TouchDown(at));
                        steps.push(Act::TouchUp(at));
                    }
                    Act::TouchDown(at) => steps.push(Act::TouchDown(at)),
                    Act::TouchUp(at) => steps.push(Act::TouchUp(at)),
                }
                next_directive += 1;
            }
        }
        // The player, resuming a world that stopped itself.
        if let Some((key, after)) = session.resume_after {
            let held = sim
                .world()
                .find_resource::<Sim>()
                .is_some_and(|sim| sim.paused_by.is_some());
            match (held, paused_since) {
                (true, None) => paused_since = Some(tick),
                (true, Some(since)) if tick >= since + after && steps.is_empty() => {
                    steps.push(Act::Tap(key));
                    paused_since = None;
                }
                (false, _) => paused_since = None,
                _ => {}
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
                // One finger, down and up. No `PointerMoved` and no
                // `ButtonPressed`: whether those arrive is the engine's
                // mirror, which is exactly what the check is asking about.
                Act::TouchWorld(at) | Act::TouchDown(at) => {
                    keyboard.record(InputEvent::Touched {
                        finger: FingerId::from_platform(1),
                        phase: TouchPhase::Began,
                        screen: world_camera.world_to_screen(at),
                    });
                }
                Act::TouchUp(at) => {
                    keyboard.record(InputEvent::Touched {
                        finger: FingerId::from_platform(1),
                        phase: TouchPhase::Ended,
                        screen: world_camera.world_to_screen(at),
                    });
                }
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
            let held = sim.world().resource::<Sim>().paused_by.is_some();
            let due: Vec<Photo> = session
                .photos
                .iter()
                .filter(|photo| {
                    now >= photo.minute
                        && tick >= photo.tick
                        && (!photo.paused || held)
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
        if let Some(until) = session.stop_at_minute
            && photos.len() == session.photos.len()
            && sim.world().resource::<Clock>().minutes >= until
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

/// **The window the sweep compares over**, in world-minutes: the scenario's
/// first day.
///
/// A world with autonomy in it never comes to rest — somebody is always about
/// to decide something — so "run until nothing is scheduled" is not a stopping
/// condition any more. Two speed scripts are comparable over the same span of
/// *world*-time, and this is that span.
pub const WINDOW: u64 = 720;

/// The minute a conducted session stops at: past [`WINDOW`], so everything
/// addressed inside the window has fired before the run ends.
pub const RUN_UNTIL: u64 = 800;

/// The world-minutes every quest in this scenario's first day resolves at —
/// the four the player orders and the three the scorer takes.
///
/// Stated as literals because they are where an auto-pause on
/// `quest-complete` stops the world, and a resume has to be addressed at one:
/// the clock is holding, so a resume cannot be addressed by the clock
/// (`When::MinuteHeld` counts ticks from the minute's first arrival). They are
/// the same minutes [`expected_events`] pins.
pub const COMPLETIONS: [u64; 12] = [165, 170, 240, 500, 506, 540, 589, 590, 603, 654, 664, 688];

/// The shared order script under one speed prologue: four dispatches at
/// fixed world-times — three parties out at once, then a re-dispatch to
/// the barrier-detour site once OX is home.
///
/// `resume_key` is `Some` for the auto-pause variant of the sweep: a tap of
/// the script's own speed key a few ticks after each completion, which is the
/// player pressing the key they were already at. It resumes a world that
/// stopped itself and does nothing at all to a world that did not — which is
/// what makes the two variants comparable.
fn script_with(speed_prologue: &[Directive]) -> Vec<Directive> {
    let mut script = speed_prologue.to_vec();
    // **Eight world-minutes apart**, because that is what the retuned clock's
    // coarsest speed can address: 4x carries 1.6 minutes a tick, an order is
    // four ticks of clicking, and two orders closer together than that cannot
    // both land on the minute they name under every speed script.
    script.extend(order(8, 0, 0)); // Bob to the Watchtower
    script.extend(order(16, 1, 1)); // Steve to the Deep Cave
    script.extend(order(24, 2, 2)); // Alex to the Old Crypt
    script.extend(order(360, 0, 3)); // Bob again, to the Black Vault
    script
}

/// The key a script is running at — what the auto-pause variant of the sweep
/// resumes with, so it does not quietly become a different speed schedule.
pub fn resume_key(name: &str) -> Key {
    match name {
        "all-1x" => Key::Digit1,
        _ => Key::Digit3,
    }
}

/// The three speed scripts (DESIGN §7). The mixed script changes speed
/// mid-travel, pauses exactly when Alex's order falls due — so one dispatch
/// happens against a held clock, the orders-while-paused property run for
/// real — and resumes 300 ticks later.
pub fn speed_scripts() -> Vec<(&'static str, Vec<Directive>)> {
    let tap = |when: When, key: Key| Directive {
        when,
        what: Act::Tap(key),
    };
    let mut mixed = vec![tap(When::Tick(5), Key::Digit2)];
    mixed.extend(order(8, 0, 0));
    mixed.extend(order(16, 1, 1));
    mixed.push(tap(When::Minute(24), Key::Space));
    mixed.extend(order(24, 2, 2));
    mixed.push(tap(
        When::MinuteHeld {
            minute: 24,
            after: 300,
        },
        Key::Space,
    ));
    mixed.push(tap(When::Minute(40), Key::Digit3));
    mixed.extend(order(360, 0, 3));
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
pub fn expected_events() -> Vec<(u64, EventClass, usize, Option<usize>)> {
    // (minute, class, party, location index - None on an unnamed tile)
    vec![
        (8, EventClass::Departed, 0, None),
        (16, EventClass::Departed, 1, None),
        (24, EventClass::Departed, 2, None),
        (56, EventClass::Arrived, 2, Some(3)),
        (56, EventClass::WorkBegan, 2, Some(3)),
        (75, EventClass::Arrived, 1, Some(2)),
        (75, EventClass::WorkBegan, 1, Some(2)),
        (94, EventClass::Arrived, 0, Some(1)),
        (94, EventClass::WorkBegan, 0, Some(1)),
        (156, EventClass::QuestComplete, 2, Some(3)),
        (165, EventClass::QuestComplete, 1, Some(2)),
        (188, EventClass::Returned, 2, None),
        (214, EventClass::QuestComplete, 0, Some(1)),
        (221, EventClass::Returned, 1, None),
        (302, EventClass::Returned, 0, None),
        (312, EventClass::ActionStarted, 3, None),
        (312, EventClass::Departed, 3, None),
        (336, EventClass::ActionStarted, 4, None),
        (336, EventClass::Departed, 4, None),
        (360, EventClass::ActionStarted, 5, None),
        (360, EventClass::Departed, 5, None),
        (360, EventClass::Departed, 0, None),
        (374, EventClass::Arrived, 3, Some(1)),
        (374, EventClass::WorkBegan, 3, Some(1)),
        (384, EventClass::ActionStarted, 6, None),
        (384, EventClass::Departed, 6, None),
        (390, EventClass::Arrived, 4, Some(1)),
        (390, EventClass::WorkBegan, 4, Some(1)),
        (408, EventClass::ActionStarted, 7, None),
        (408, EventClass::Departed, 7, None),
        (432, EventClass::ActionStarted, 8, None),
        (432, EventClass::Departed, 8, None),
        (436, EventClass::Arrived, 5, Some(3)),
        (436, EventClass::WorkBegan, 5, Some(3)),
        (448, EventClass::Arrived, 0, Some(4)),
        (448, EventClass::WorkBegan, 0, Some(4)),
        (453, EventClass::Arrived, 6, Some(2)),
        (453, EventClass::WorkBegan, 6, Some(2)),
        (456, EventClass::ActionStarted, 9, None),
        (456, EventClass::Departed, 9, None),
        (477, EventClass::Arrived, 7, Some(2)),
        (477, EventClass::WorkBegan, 7, Some(2)),
        (490, EventClass::QuestComplete, 4, Some(1)),
        (494, EventClass::QuestComplete, 3, Some(1)),
        (504, EventClass::ActionStarted, 1, None),
        (504, EventClass::Departed, 1, None),
        (514, EventClass::Arrived, 8, Some(1)),
        (514, EventClass::WorkBegan, 8, Some(1)),
        (530, EventClass::Arrived, 9, Some(1)),
        (530, EventClass::WorkBegan, 9, Some(1)),
        (536, EventClass::QuestComplete, 5, Some(3)),
        (546, EventClass::Returned, 4, None),
        (546, EventClass::ActionDone, 4, None),
        (558, EventClass::Returned, 3, None),
        (558, EventClass::ActionDone, 3, None),
        (582, EventClass::Arrived, 1, Some(1)),
        (582, EventClass::WorkBegan, 1, Some(1)),
        (587, EventClass::QuestComplete, 7, Some(2)),
        (594, EventClass::QuestComplete, 8, Some(1)),
        (603, EventClass::QuestComplete, 6, Some(2)),
        (618, EventClass::Returned, 5, None),
        (618, EventClass::ActionDone, 5, None),
        (628, EventClass::QuestComplete, 0, Some(4)),
        (659, EventClass::Returned, 7, None),
        (659, EventClass::ActionDone, 7, None),
        (670, EventClass::QuestComplete, 9, Some(1)),
        (672, EventClass::QuestComplete, 1, Some(1)),
        (675, EventClass::Returned, 6, None),
        (675, EventClass::ActionDone, 6, None),
        (678, EventClass::Returned, 8, None),
        (678, EventClass::ActionDone, 8, None),
        (718, EventClass::Returned, 0, None),
        (720, EventClass::ActionStarted, 0, None),
        (720, EventClass::Departed, 0, None),
    ]
}

/// What the quests resolving inside the window pay, in gold.
///
/// The player orders four of them and the scorer takes the rest, which is
/// itself the module's loudest claim: a world where people go looking for work
/// is a world where the board empties without anybody being told to empty it.
pub const EXPECTED_TREASURY: i64 = 605;

/// One row of a reduced transcript: address, class, party, place, sentence.
pub type Entry = (u64, &'static str, usize, Tile, Option<usize>, String);

/// A transcript reduced to what invariance is about: address, class, party,
/// place — **and the sentence**, because since wave 1.1 a sentence carries the
/// reason somebody did something, and a replay that reproduced the choices and
/// not the reasons would be reproducing half a decision.
pub fn transcript(events: &[Event]) -> Vec<Entry> {
    events
        .iter()
        .filter(|event| event.minute <= WINDOW)
        .map(|event| {
            (
                event.minute,
                event.class.name(),
                event.party,
                event.tile,
                event.location,
                event.note.clone(),
            )
        })
        .collect()
}

/// The same, without the sentences — what a check that is only about
/// addresses prints when a full transcript would be unreadable.
pub fn addresses(events: &[Event]) -> Vec<(u64, &'static str, usize)> {
    events
        .iter()
        .filter(|event| event.minute <= WINDOW)
        .map(|event| (event.minute, event.class.name(), event.party))
        .collect()
}

/// The exact-time judge: the fixed script's whole event list, to the minute,
/// and the pot arithmetic (DESIGN §7).
pub fn judge_orders(checks: &mut Checks, run: &Conducted, label: &str) {
    let got: Vec<(u64, EventClass, usize, Option<usize>)> = run
        .events
        .iter()
        .filter(|event| event.minute <= WINDOW)
        .map(|event| (event.minute, event.class, event.party, event.location))
        .collect();
    let wanted = expected_events();
    checks.require(
        got == wanted,
        "the fixed order script did not produce its asserted timeline",
        format!(
            "{label}: the transcript is {got:?} and the authored arithmetic says {wanted:?}; \
             every expected minute is a sum of terrain costs along the stored route"
        ),
    );
    // **The pots, summed over the window rather than at the end of the run.**
    // A conducted session stops at the first tick past [`RUN_UNTIL`], and at
    // 4x that tick carries the clock a minute or two further than at 1x — so
    // the treasury *at the end* is a fact about tick granularity, and the
    // treasury *inside the window* is a fact about the world.
    let paid: i64 = run
        .events
        .iter()
        .filter(|event| event.minute <= WINDOW)
        .map(|event| event.gold)
        .sum();
    checks.require(
        paid == EXPECTED_TREASURY,
        "the pots did not pay what the quests promise",
        format!(
            "{label}: {paid}g was paid inside the first {WINDOW} world-minutes and the quests \
             that resolve in them promise {EXPECTED_TREASURY}g"
        ),
    );
    checks.require(
        run.minutes >= WINDOW,
        "the fixed script did not run out its world-time window",
        format!(
            "{label}: the clock ended at minute {} after {} ticks and the window is {WINDOW}",
            run.minutes, run.ticks
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
        session.stop_at_minute = None;
        let conducted = conduct(&session);
        checks.require(
            conducted.minutes == want,
            "the clock does not pace at its named constants",
            format!(
                "300 ticks at {label} carried {} world-minutes and the shipped arithmetic \
                 (minute_ticks 30; accumulations 12/24/48) says {want}",
                conducted.minutes
            ),
        );
        notes.push(format!("{label}={}m", conducted.minutes));
    }
    notes
}

/// The shipped pacing: 300 accumulating ticks at each speed.
///
/// **1x is 24 world-minutes a real second** at the engine's fixed sixty — a
/// world-day every wall minute — which is where the wave-0a playtest put it
/// after DESIGN §4's first guess came out an order of magnitude slower.
pub const SHIPPED_PACING: [u64; 3] = [120, 240, 480];

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
        "speed-invariance sweep: {} scripts over {WINDOW} world-minutes, {} events each, \
         treasury {}g, pacing {}",
        runs.len(),
        transcript(&runs[0].1.events).len(),
        runs[0].1.treasury,
        pacing.join(" "),
    );
    let (_, baseline_run) = runs.remove(0);
    (summary, baseline_run)
}
