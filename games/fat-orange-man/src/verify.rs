//! `--verify`: script three tap cadences, run headless, assert on what the
//! world did and what was drawn, capture a frame.
//!
//! What is checked, by section:
//! - **structure** — one frame per tick, drawn at a full alpha, systems in the
//!   order `main::register` promises out of `schedule_debug()`.
//! - **layout as requirements** — nothing off screen (with the clearance
//!   printed); the button in the bottom third and big enough for a thumb; the
//!   panel clear of the button; the title up top. Requirements, not the
//!   constants that satisfy them (jidousha-testing.md).
//! - **the drawn frame** — an orange quad the size of the square where the
//!   square is; a button-coloured quad the size of the button where the button
//!   is; the panel band lifting the panel over the square though it is
//!   submitted first.
//! - **contracts play does not reach** — `square_size` at zero, its monotonic
//!   climb, its clamp; `is_feed_tap` inside and outside; `project_board` for a
//!   player in the ten and one outside it.
//! - **determinism** — two fresh sims, no input, identical `FeedState` at tick
//!   300, and its global count equal to a checked-in literal.
//! - **the staged screen** — the player in the Top 10, which no 600-tick run
//!   here reaches.
//! - **three cadences** — idle feeds nothing yet the world still moves; human
//!   and masher drop no taps; masher climbs strictly past human.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FrameRecord, FrameRecorder, InputEvent, SnapshotBuilder,
};

use crate::backend::{Backend, FeedState};
use crate::checks::{Checks, fail, greater, near};
use crate::game;
use crate::players::{self, Report};
use crate::{config, register};

/// How long each scripted session runs — ten seconds at sixty ticks.
const TICKS: u64 = 600;

/// The surface the headless run draws at: the game's own window, so the
/// recorder's viewport and the camera's agree and nothing is rebuilt.
const VIEWPORT: PhysicalSize = game::WINDOW;

/// The global feed count a fresh seed-0 sim reaches at tick 300 with nobody
/// tapping — a checked-in literal, the determinism claim reduced to one number
/// (jidousha-testing.md §A.6: a shipped literal, never arithmetic over the
/// thing under test).
const GLOBAL_AT_300: u64 = 1031;

/// What one scripted session did.
struct Run {
    /// The player's own feed count after each tick.
    personal_track: Vec<u64>,
    /// The global feed count after each tick.
    global_track: Vec<u64>,
    /// The player's leaderboard rank at the end.
    rank_end: usize,
    /// How many taps the plan issued.
    taps_issued: u64,
    /// Frames recorded (one per tick).
    frames: usize,
    /// The last frame — every frame is "live", there is no end screen.
    last: FrameRecord,
    /// Which backend texture the font landed on.
    font: BackendTextureId,
    /// The camera the last frame was drawn with, viewport included.
    camera: Camera,
    /// The interpolation fraction of the last frame (1.0 for once-per-tick).
    alpha: f32,
    /// The tap bounce multiplier on the last frame — the drawn square is
    /// `square_size(global) * pop`, clamped, so a check has to carry it.
    pop_end: f32,
}

/// Record a tap — pointer to the button, press and release on this frame.
fn tap(builder: &mut SnapshotBuilder, at: Vec2) {
    builder.record(InputEvent::PointerMoved {
        id: PointerId::PRIMARY,
        screen: at,
    });
    builder.record(InputEvent::ButtonPressed {
        id: PointerId::PRIMARY,
        button: PointerButton::Primary,
    });
    builder.record(InputEvent::ButtonReleased {
        id: PointerId::PRIMARY,
        button: PointerButton::Primary,
    });
}

/// Play one cadence for `TICKS` ticks, recording every frame.
fn play(taps: players::TapPlan) -> Run {
    let mut sim = headless(config(), register);
    let mut recorder = FrameRecorder::new(VIEWPORT);
    let font = recorder.font_texture();

    // The button's centre in screen pixels, through the camera the game uses.
    let button_screen = game::camera().world_to_screen(game::feed_button().center());

    let mut builder = SnapshotBuilder::new();
    let mut personal_track = Vec::new();
    let mut global_track = Vec::new();
    let mut taps_issued = 0u64;
    let mut last = None;

    for tick in 1..=TICKS {
        // Keep the pointer parked on the button every tick; add the edges only
        // when this cadence taps.
        builder.record(InputEvent::PointerMoved {
            id: PointerId::PRIMARY,
            screen: button_screen,
        });
        if taps(tick) {
            tap(&mut builder, button_screen);
            taps_issued += 1;
        }
        sim.world_mut()
            .insert_resource(Input::new(builder.first_tick_snapshot()));
        sim.tick();

        let state = sim.world().resource::<FeedState>();
        personal_track.push(state.personal);
        global_track.push(state.global);
        last = Some(recorder.draw(&mut sim));
    }

    let Some(last) = last else {
        fail("no frame was recorded", "the loop above draws every tick");
    };
    let rank_end = sim.world().resource::<FeedState>().board.your_rank;
    Run {
        personal_track,
        global_track,
        rank_end,
        taps_issued,
        frames: TICKS as usize,
        camera: Camera {
            viewport: VIEWPORT,
            ..*sim.world().resource::<Camera>()
        },
        alpha: sim.world().resource::<Time>().alpha,
        pop_end: sim.world().resource::<crate::sim::Pop>().scale,
        last,
        font,
    }
}

/// The report a cadence prints about itself.
fn report(run: &Run) -> Report {
    Report {
        taps_issued: run.taps_issued,
        personal_end: *run.personal_track.last().unwrap_or(&0),
        rank_end: run.rank_end,
    }
}

pub fn run() -> ExitCode {
    let mut checks = Checks::default();

    let human = play(players::HUMAN.taps);
    let idle = play(players::IDLE.taps);
    let masher = play(players::MASHER.taps);

    let view = human.camera.visible_bounds();
    let quads = human.last.quads();

    // --- structure --------------------------------------------------
    checks.require(
        near(human.alpha, 1.0),
        "a run that draws once per tick did not draw at a full alpha",
        format!("Time::alpha is {}, not 1.0", human.alpha),
    );
    checks.require(
        human.frames == TICKS as usize,
        "one frame per tick was expected",
        format!("{} frames for {TICKS} ticks", human.frames),
    );
    let schedule = headless(config(), register).schedule_debug();
    let order = [
        schedule.find("handle_tap"),
        schedule.find("advance_backend"),
        schedule.find("ease_pop"),
    ];
    checks.require(
        order.iter().all(Option::is_some) && order[0] < order[1] && order[1] < order[2],
        "the update systems are not in the order register promises",
        format!("handle_tap / advance_backend / ease_pop found at {order:?} in:\n{schedule}"),
    );

    // --- layout as requirements ----------------------------------
    let off: Vec<Rect> = quads
        .iter()
        .map(|quad| quad.bounds())
        .filter(|bounds| !view.contains_rect(*bounds))
        .collect();
    checks.require(
        off.is_empty(),
        "something was drawn outside what the camera shows",
        format!(
            "{} of {} quads fall outside {view:?}; first {:?} — text centred by width_of is the \
             usual culprit",
            off.len(),
            quads.len(),
            off.first(),
        ),
    );
    let clearance = quads
        .iter()
        .map(|quad| {
            let b = quad.bounds();
            let gap = (b.min - view.min).min(view.max - b.max);
            gap.x.min(gap.y)
        })
        .fold(f32::MAX, f32::min);

    let button = game::feed_button();
    let bottom_third = view.min.y + view.size().y * 2.0 / 3.0;
    checks.require(
        greater(button.min.y, bottom_third),
        "the feed button is not in the bottom third of the screen (GDD §7: single-thumb)",
        format!(
            "its top is at y {:.2}; the bottom third starts at {bottom_third:.2}",
            button.min.y
        ),
    );
    checks.require(
        greater(button.size().x, 6.0) && greater(button.size().y, 1.5),
        "the feed button is too small to be a thumb target",
        format!(
            "it is {:.2} x {:.2} world units",
            button.size().x,
            button.size().y
        ),
    );
    let panel = game::leaderboard_panel();
    checks.require(
        !panel.overlaps(button),
        "the leaderboard panel overlaps the feed button (GDD §7: must not compete)",
        format!("panel {panel:?} overlaps button {button:?}"),
    );
    let title_band = view.min.y + view.size().y * 0.2;
    let title_top = quads
        .iter()
        .filter(|quad| quad.texture == human.font && quad.tint == game::ORANGE)
        .map(|quad| quad.bounds().min.y)
        .fold(f32::MAX, f32::min);
    checks.require(
        greater(title_band, title_top),
        "the title is not in the top fifth of the screen",
        format!("its highest glyph starts at y {title_top:.2}; the band ends at {title_band:.2}"),
    );

    // --- the drawn frame ---------------------------------------------
    let global_end = *human.global_track.last().unwrap_or(&0);
    // The drawn side is the curve times the tap bounce, clamped — the run ends
    // on a tap tick, so `pop_end` is above 1.0 and has to be carried in.
    let side = (game::square_size(global_end) * human.pop_end).min(game::SQUARE_MAX);
    let square_shaped = human
        .last
        .covering(game::SQUARE_CENTER)
        .into_iter()
        .any(|quad| {
            let b = quad.bounds();
            quad.tint == game::ORANGE
                && near(b.size().x, side)
                && near(b.size().y, side)
                && near(b.center().x, game::SQUARE_CENTER.x)
                && near(b.center().y, game::SQUARE_CENTER.y)
        });
    checks.require(
        square_shaped,
        "no square-shaped orange quad where the man is",
        format!(
            "global {global_end} makes a side of {side:.3} at {:?}; covering it: {}",
            game::SQUARE_CENTER,
            crate::checks::sizes_covering(&human.last, game::SQUARE_CENTER),
        ),
    );
    let button_shaped = human
        .last
        .covering(button.center())
        .into_iter()
        .any(|quad| {
            let b = quad.bounds();
            quad.tint == game::BUTTON
                && near(b.size().x, button.size().x)
                && near(b.size().y, button.size().y)
        });
    checks.require(
        button_shaped,
        "no button-shaped quad where the feed button is",
        format!(
            "the button is {:.2}x{:.2} at {:?}; covering it: {}",
            button.size().x,
            button.size().y,
            button.center(),
            crate::checks::sizes_covering(&human.last, button.center()),
        ),
    );

    // The panel is submitted before the square (main::register) but sits in a
    // higher band, so in draw order it must come *after* the square — the one
    // place a check can see the PANEL band act.
    let square_at = quads
        .iter()
        .position(|quad| quad.tint == game::ORANGE && greater(quad.bounds().size().x, 2.0));
    let panel_at = quads.iter().position(|quad| quad.tint == game::PANEL);
    checks.require(
        square_at.is_some() && panel_at.is_some(),
        "the square or the panel drew nothing in the last frame",
        format!("square at {square_at:?}, panel at {panel_at:?} in the draw order"),
    );
    if let (Some(sq), Some(pn)) = (square_at, panel_at) {
        checks.require(
            pn > sq,
            "the panel is drawn under the square instead of over it",
            format!(
                "panel is at index {pn} in the draw order and the square at {sq}; register \
                 submits the panel first, so only the PANEL band can put it last"
            ),
        );
    }

    // --- the background --------------------------------------------
    let cleared = human.last.plan.clear_color;
    checks.require(
        cleared == game::BACKGROUND,
        "the screen was cleared to a colour the game does not name",
        format!(
            "cleared to {cleared:?}; the constant is {:?}",
            game::BACKGROUND
        ),
    );
    let brightness = cleared.r.max(cleared.g).max(cleared.b);
    checks.require(
        greater(0.25, brightness) && greater(cleared.a, 0.99),
        "the background is not dark enough for the orange to read against it",
        format!(
            "brightest channel {brightness:.3} at alpha {:.2}",
            cleared.a
        ),
    );

    // --- contracts play does not reach ----------------------------
    checks.require(
        near(game::square_size(0), game::SQUARE_MIN),
        "square_size(0) is not the minimum side",
        format!(
            "it is {:.4}, not {:.4}",
            game::square_size(0),
            game::SQUARE_MIN
        ),
    );
    let samples = [0u64, 1, 4, 25, 100, 400, 1600, 6400, 25_600];
    let monotone = samples
        .windows(2)
        .all(|pair| game::square_size(pair[1]) >= game::square_size(pair[0]));
    checks.require(
        monotone,
        "square_size is not monotonically non-decreasing",
        format!(
            "sizes for {samples:?} are {:?}",
            samples.map(game::square_size)
        ),
    );
    checks.require(
        near(game::square_size(1_000_000_000), game::SQUARE_MAX),
        "square_size does not clamp to SQUARE_MAX",
        format!(
            "a billion feeds gives {:.4}",
            game::square_size(1_000_000_000)
        ),
    );
    let big = game::square_bounds(1_000_000);
    checks.require(
        game::is_feed_tap(button.center(), big)
            && game::is_feed_tap(game::SQUARE_CENTER, big)
            && !game::is_feed_tap(view.min + Vec2::splat(0.2), big),
        "is_feed_tap does not agree on the button, the man, and empty space",
        "expected true on the button centre and the square, false in the top-left corner"
            .to_owned(),
    );
    // project_board: fifteen bots on 100, "you" on 0 -> rank 16, pinned row.
    let mut field: Vec<game::Entry> = (0..15)
        .map(|i| game::Entry {
            name: format!("BOT{i:02}"),
            count: 100,
            you: false,
        })
        .collect();
    field.push(game::Entry {
        name: "YOU".to_owned(),
        count: 0,
        you: true,
    });
    let board = game::project_board(field, 10);
    checks.require(
        board.your_rank == 16 && board.top.len() == 10 && board.your_row.is_some(),
        "project_board mislocates a player outside the top ten",
        format!(
            "rank {}, top {} rows, pinned row {:?}",
            board.your_rank,
            board.top.len(),
            board.your_row.is_some()
        ),
    );
    let top_board = game::project_board(
        vec![
            game::Entry {
                name: "A".into(),
                count: 5,
                you: false,
            },
            game::Entry {
                name: "B".into(),
                count: 9,
                you: true,
            },
            game::Entry {
                name: "C".into(),
                count: 1,
                you: false,
            },
        ],
        10,
    );
    checks.require(
        top_board.your_rank == 1 && top_board.your_row.is_none() && top_board.top[0].you,
        "project_board does not show a leading player in the ten itself",
        format!(
            "rank {}, pinned row {:?}, top[0].you {}",
            top_board.your_rank,
            top_board.your_row.is_some(),
            top_board.top[0].you
        ),
    );

    // --- determinism ---------------------------------------------
    let at_300 = || {
        let mut sim = headless(config(), register);
        for _ in 0..300 {
            sim.tick();
        }
        sim.world().resource::<FeedState>().clone()
    };
    let (a, b) = (at_300(), at_300());
    checks.require(
        a == b,
        "two fresh sims disagree at tick 300",
        format!("global {} vs {}", a.global, b.global),
    );
    checks.require(
        a.global == GLOBAL_AT_300,
        "the seed-0 global count at tick 300 moved",
        format!(
            "it is {}; the checked-in literal is {GLOBAL_AT_300}",
            a.global
        ),
    );

    // --- the staged screen: the player in the Top 10 --------------
    {
        let mut sim = headless(config(), register);
        sim.tick(); // Startup
        for _ in 0..50_000 {
            sim.world_mut().resource_mut::<Backend>().feed();
        }
        sim.tick(); // advance_backend rebuilds FeedState
        let mut staged = FrameRecorder::new(VIEWPORT);
        let frame = staged.draw(&mut sim);
        let state = sim.world().resource::<FeedState>();
        let in_ten = state.board.top.iter().any(|entry| entry.you);
        let you_glyph = frame.quads().iter().any(|quad| {
            quad.texture == human.font
                && quad.tint == game::TEXT_YOU
                && panel.contains(quad.bounds().center())
        });
        checks.require(
            in_ten && state.board.your_row.is_none() && you_glyph,
            "a runaway player is not shown inside the leaderboard",
            format!(
                "in_ten {in_ten}, pinned row {:?}, a TEXT_YOU glyph in the panel {you_glyph}",
                state.board.your_row.is_some()
            ),
        );
    }

    // --- the three cadences -------------------------------------
    let (r_idle, r_human, r_masher) = (report(&idle), report(&human), report(&masher));
    checks.require(
        r_idle.personal_end == 0
            && greater(idle.global_track[299] as f32, idle.global_track[0] as f32),
        "idle either fed the man or the world stopped moving without it",
        format!(
            "idle personal {}, global {} -> {}",
            r_idle.personal_end, idle.global_track[0], idle.global_track[299]
        ),
    );
    let human_monotone = human
        .personal_track
        .windows(2)
        .all(|pair| pair[1] >= pair[0]);
    checks.require(
        human_monotone && r_human.personal_end == r_human.taps_issued,
        "the human run dropped a tap or its count went backwards",
        format!(
            "issued {}, ended {}, monotone {human_monotone}",
            r_human.taps_issued, r_human.personal_end
        ),
    );
    checks.require(
        r_masher.personal_end == r_masher.taps_issued
            && r_masher.personal_end > r_human.personal_end,
        "the masher dropped a tap or did not out-feed the human",
        format!(
            "masher issued {} ended {}; human ended {}",
            r_masher.taps_issued, r_masher.personal_end, r_human.personal_end
        ),
    );
    checks.require(
        r_masher.rank_end < r_idle.rank_end,
        "mashing did not climb past a single bot",
        format!(
            "masher rank {}, idle rank {}",
            r_masher.rank_end, r_idle.rank_end
        ),
    );

    // --- capture --------------------------------------------------
    let captured = crate::capture::capture_a_frame(&mut checks, &human.last, human.font);
    let verdict = checks.verdict();

    println!("verified fat-orange-man over {TICKS} ticks, three cadences");
    for (name, r) in [
        (players::IDLE.name, r_idle),
        (players::HUMAN.name, r_human),
        (players::MASHER.name, r_masher),
    ] {
        println!(
            "  {name:<6}: {:>4} taps issued, {:>4} registered, ended rank {}",
            r.taps_issued, r.personal_end, r.rank_end,
        );
    }
    println!(
        "  square side at end: {side:.2} of {:.2} max",
        game::SQUARE_MAX
    );
    println!("  closest quad to the edge: {clearance:.2} world units");
    println!(
        "  seed-0 global at tick 300: {} (literal {GLOBAL_AT_300})",
        a.global
    );
    println!("  capture: {captured}");
    print!("{}", human.last.transcript());
    verdict
}
