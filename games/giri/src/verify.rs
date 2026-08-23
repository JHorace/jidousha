//! `--verify`: the tutorial, played by a script, asserted on, and printed.
//!
//! The beats are the test suite (DESIGN.md §8). Each one is driven end to end
//! through `InputScript` - the same pointer a person uses, at the same
//! rectangles - and judged twice: once on world state (refusals, deaths,
//! infamy, regard, desperation trajectories) and once on the null-backend
//! transcript (every sheet drawn, every report row drawn as its own string).
//!
//! Then three things a played beat cannot reach:
//!
//! - **the contract battery** (`contracts.rs`): the decision function asked
//!   directly, including the roster-order betrayal case no tutorial beat
//!   produces and the two dungeon predicates no tutorial beat uses;
//! - **the mutation round** (`contracts.rs`): every tuning constant perturbed
//!   in turn, demanding that some beat or contract notices. A beat that passes
//!   under a mutated constant is a vacuous assertion;
//! - **the capture**: one recorded frame rendered for real, so somebody can
//!   look at it.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord, FrameRecorder, InputScript};

use crate::beats::{BeatSpec, CHAIN, Expect};
use crate::checks::{Checks, fail, greater};
use crate::constants::Tuning;
use crate::contracts;
use crate::flow::{Flow, Preview, Stage, StartAt};
use crate::judge::{glyph_run, judge_frames, judge_world};
use crate::model::Social;
use crate::ui;
use crate::{config, register};

/// The viewport the headless run draws at - the window's own, so the recorder's
/// override and the game's camera agree and every bounds assertion is about the
/// aspect the game is laid out for.
pub const HEADLESS_VIEWPORT: PhysicalSize = crate::WINDOW;

/// The camera a scripted click is aimed through.
///
/// The same one `open_the_chain` installs. A script converts a world-space
/// rectangle to pixels with `world_to_screen`, and the game converts back with
/// `screen_to_world`; if these two cameras disagreed, every click would land
/// somewhere else and the run would fail with the party empty.
pub fn headless_camera() -> Camera {
    Camera {
        center: Vec2::ZERO,
        height: crate::VIEW_HEIGHT,
        clear_color: ui::BACKDROP,
        viewport: HEADLESS_VIEWPORT,
    }
}

/// A scripted click: put the pointer on a world point, then tap it there.
fn click(script: InputScript, tick: &mut u64, at: Vec2) -> InputScript {
    let screen = headless_camera().world_to_screen(at);
    let next = script
        .pointer_at(*tick, screen)
        .click(PointerButton::Primary, *tick + 1);
    // Three ticks a click: move, press, settle. The settle tick is what makes
    // every mark below a tick on which nothing is happening.
    *tick += 3;
    next
}

/// The scripted session for one beat, and the ticks worth looking at.
struct Plan {
    script: InputScript,
    /// The tick the refusal probe is on screen, if the beat has a refusal.
    probe_at: Option<u64>,
    /// The tick a half-filled party is on screen, if the beat sends more than one.
    partial_at: Option<u64>,
    /// The tick the assembled party is on screen and willing.
    ready_at: u64,
    /// The tick after the dungeon resolved.
    report_at: u64,
    /// The tick after continuing out of the report.
    end_at: u64,
    /// The last tick to run.
    last: u64,
}

/// Build the session: probe the refusal the beat is about, take it back,
/// assemble the intended party, send it, read the report, continue.
fn plan_for(spec: &BeatSpec) -> Plan {
    let probe: Vec<usize> = spec
        .expect
        .iter()
        .find_map(|expect| match expect {
            Expect::Refuses { party, .. } => Some(*party),
            _ => None,
        })
        .map(|party| {
            party
                .iter()
                .filter_map(|name| spec.index_of(name))
                .collect()
        })
        .unwrap_or_default();

    let mut script = InputScript::new();
    let mut tick = 3;
    let mut probe_at = None;
    if !probe.is_empty() {
        for index in &probe {
            script = click(script, &mut tick, ui::card_rect(*index).center());
        }
        probe_at = Some(tick);
        tick += 1;
        // Take it back: a refusal is feedback, not a failure (DESIGN §5).
        for index in &probe {
            script = click(script, &mut tick, ui::card_rect(*index).center());
        }
    }

    let mut partial_at = None;
    for (offered, name) in spec.send.iter().enumerate() {
        if let Some(index) = spec.index_of(name) {
            script = click(script, &mut tick, ui::card_rect(index).center());
        }
        if offered == 0 && spec.send.len() > 1 {
            partial_at = Some(tick);
            tick += 1;
        }
    }
    let ready_at = tick;
    tick += 1;
    script = click(script, &mut tick, ui::send_button().center());
    let report_at = tick;
    tick += 1;
    script = click(script, &mut tick, ui::continue_button().center());
    let end_at = tick;
    Plan {
        script,
        probe_at,
        partial_at,
        ready_at,
        report_at,
        end_at,
        last: end_at + 1,
    }
}

/// What one scripted beat did.
pub struct BeatRun {
    /// Which beat.
    pub index: usize,
    /// The social state the beat was authored with, read after Startup.
    pub at_assembly: Social,
    /// The social state the dungeon left behind.
    pub after: Social,
    /// The preview while the refusing party was selected.
    pub probe: Option<Preview>,
    /// The preview once the intended party was assembled.
    pub ready: Preview,
    /// The resolution's narration.
    pub report: Vec<String>,
    /// The stage the send verb produced.
    pub stage_after_send: Stage,
    /// The stage continuing produced, and which beat it left the game on.
    pub stage_at_end: Stage,
    /// Which beat continuing moved to.
    pub beat_at_end: usize,
    /// How many frames were drawn.
    pub frames: usize,
    /// The frame with the refusal on it.
    pub probe_frame: Option<FrameRecord>,
    /// The frame with a half-filled party on it.
    pub partial_frame: Option<FrameRecord>,
    /// The frame with the assembled party on it.
    pub ready_frame: Option<FrameRecord>,
    /// The frame with the report on it.
    pub report_frame: Option<FrameRecord>,
    /// The frame after continuing - the next beat, or the end of the chain.
    pub end_frame: Option<FrameRecord>,
    /// Everything drawn outside the camera, over every frame of the run.
    pub off_screen: Vec<Rect>,
    /// How close the closest quad came to the camera's edge, over every frame.
    pub clearance: f32,
    /// How many quads the run drew in all.
    pub quads: usize,
    /// Which backend texture the font landed on.
    pub font: BackendTextureId,
    /// The camera the frames were drawn with.
    pub camera: Camera,
    /// Every phase and its systems, in run order.
    pub schedule: String,
}

/// Play one beat through the script, with `tuning` in effect.
///
/// `record` off is the mutation round's shape: the world assertions are the
/// ones a perturbed constant is judged on, and a thousand unread frames are a
/// thousand frames to allocate.
pub fn play(index: usize, tuning: Tuning, record: bool) -> BeatRun {
    let Some(spec) = CHAIN.get(index) else {
        fail(
            "a beat was asked for that the chain does not have",
            &format!("beat {index} of {}", CHAIN.len()),
        );
    };
    let plan = plan_for(spec);
    let mut sim = headless(config(), register);
    // Before Startup, which is what `open_the_chain` reads them with.
    sim.world_mut().insert_resource(tuning);
    sim.world_mut().insert_resource(StartAt(index));

    let mut recorder = record.then(|| FrameRecorder::new(HEADLESS_VIEWPORT));
    let font = recorder
        .as_ref()
        .map_or(BackendTextureId(0), FrameRecorder::font_texture);
    let mut run = BeatRun {
        index,
        at_assembly: Social::default(),
        after: Social::default(),
        probe: None,
        ready: Preview::default(),
        report: Vec::new(),
        stage_after_send: Stage::Assembly,
        stage_at_end: Stage::Assembly,
        beat_at_end: index,
        frames: 0,
        probe_frame: None,
        partial_frame: None,
        ready_frame: None,
        report_frame: None,
        end_frame: None,
        off_screen: Vec::new(),
        clearance: f32::MAX,
        quads: 0,
        font,
        camera: headless_camera(),
        schedule: sim.schedule_debug(),
    };

    for tick in 1..=plan.last {
        sim.world_mut()
            .insert_resource(Input::new(plan.script.snapshot_at(tick)));
        sim.tick();
        if tick == 1 {
            run.at_assembly = Social::read(&sim.world().view());
            run.camera = Camera {
                viewport: HEADLESS_VIEWPORT,
                ..*sim.world().resource::<Camera>()
            };
            run.schedule = sim.schedule_debug();
        }
        let frame = recorder.as_mut().map(|recorder| recorder.draw(&mut sim));
        if let Some(frame) = &frame {
            run.frames += 1;
            let view = run.camera.visible_bounds();
            for quad in frame.quads() {
                let bounds = quad.bounds();
                run.quads += 1;
                if !view.contains_rect(bounds) {
                    run.off_screen.push(bounds);
                }
                let gap = (bounds.min - view.min).min(view.max - bounds.max);
                run.clearance = run.clearance.min(gap.x.min(gap.y));
            }
        }
        if Some(tick) == plan.probe_at {
            run.probe = Some(sim.world().resource::<Preview>().clone());
            run.probe_frame = frame.clone();
        }
        if Some(tick) == plan.partial_at {
            run.partial_frame = frame.clone();
        }
        if tick == plan.ready_at {
            run.ready = sim.world().resource::<Preview>().clone();
            run.ready_frame = frame.clone();
        }
        if tick == plan.report_at {
            let flow = sim.world().resource::<Flow>();
            run.stage_after_send = flow.stage;
            run.report = flow.report.clone();
            run.after = Social::read(&sim.world().view());
            run.report_frame = frame.clone();
        }
        if tick == plan.end_at {
            let flow = sim.world().resource::<Flow>();
            run.stage_at_end = flow.stage;
            run.beat_at_end = flow.beat;
            run.end_frame = frame;
        }
    }
    run
}

pub fn run() -> ExitCode {
    let mut checks = Checks::default();
    let tuning = Tuning::SHIPPED;
    let mut runs = Vec::new();
    for index in 0..CHAIN.len() {
        let played = play(index, tuning, true);
        let Some(spec) = CHAIN.get(index) else {
            continue;
        };
        judge_world(&mut checks, spec, &played, &tuning);
        judge_frames(&mut checks, spec, &played);
        runs.push(played);
    }
    let Some(last) = runs.last() else {
        fail(
            "the chain has no beats",
            "beats.rs::CHAIN is what a beat is added to",
        );
    };

    // --- nothing off screen, over every frame of every beat ------------
    let off_screen: Vec<&Rect> = runs.iter().flat_map(|run| run.off_screen.iter()).collect();
    let view = last.camera.visible_bounds();
    checks.require(
        off_screen.is_empty(),
        "something was drawn outside what the camera shows",
        format!(
            "{} quads of {} fall outside {view:?}; the first is {:?} - a sheet line or a \
             report row wider than its column is the usual culprit",
            off_screen.len(),
            runs.iter().map(|run| run.quads).sum::<usize>(),
            off_screen.first(),
        ),
    );
    let clearance = runs
        .iter()
        .map(|run| run.clearance)
        .fold(f32::MAX, f32::min);

    // --- the layout's own requirements, not its constants ---------------
    //
    // "On screen" is not "in the right place". These say what the layout is
    // *for*, so they survive somebody changing the number that produced it.
    let cards = last.at_assembly.members.len();
    let lowest_card = ui::card_rect(cards.saturating_sub(1)).max.y;
    checks.require(
        greater(ui::send_button().min.y, lowest_card),
        "the send button overlaps the roster it is about",
        format!(
            "the last of {cards} cards ends at y {lowest_card:.2} and the button starts at \
             {:.2}",
            ui::send_button().min.y
        ),
    );
    // The roster sits between the headline and the first job row, and the two
    // columns start level. Stated as a pair rather than against CONTENT_TOP,
    // which is the constant that put both of them there: a check spelled
    // `card_rect(0).min.y == CONTENT_TOP` moves with the cards and cannot see
    // them drift. This pair caught exactly that, injected on purpose.
    checks.require(
        greater(ui::card_rect(0).min.y, ui::header_bar().max.y - 0.11),
        "the roster is drawn up into the headline it sits under",
        format!(
            "the first card starts at y {:.2} and the headline bar ends at {:.2}",
            ui::card_rect(0).min.y,
            ui::header_bar().max.y
        ),
    );
    checks.require(
        !greater(ui::card_rect(0).min.y, ui::dungeon_row_rect(0).min.y),
        "the roster column and the wide column do not start level",
        format!(
            "the first card starts at y {:.2} and the first job row at {:.2}; a column that \
             starts lower than the one beside it is a column that has drifted",
            ui::card_rect(0).min.y,
            ui::dungeon_row_rect(0).min.y
        ),
    );
    checks.require(
        greater(ui::MAIN_X, ui::card_rect(0).max.x),
        "the wide column starts inside the roster column",
        format!(
            "cards end at x {:.2} and the column starts at {:.2}",
            ui::card_rect(0).max.x,
            ui::MAIN_X
        ),
    );
    let report_bottom = ui::report_row_y(last.report.len().saturating_sub(1)) + ui::SMALL;
    checks.require(
        greater(ui::continue_button().min.y, report_bottom),
        "the report runs into the button under it",
        format!(
            "{} report rows end at y {report_bottom:.2} and the button starts at {:.2}",
            last.report.len(),
            ui::continue_button().min.y
        ),
    );

    // --- the schedule order, which nothing else can see -----------------
    let order = &last.schedule;
    let (pointer_at, preview_at) = (order.find("handle_pointer"), order.find("refresh_preview"));
    checks.require(
        pointer_at.is_some() && preview_at.is_some() && pointer_at < preview_at,
        "the preview is computed before the click that changes it",
        format!(
            "handle_pointer is at {pointer_at:?} and refresh_preview at {preview_at:?} in the \
             schedule; reversed, the arithmetic on screen is the previous tick's party"
        ),
    );
    let (headline_at, backdrop_at) = (order.find("draw_headline"), order.find("draw_backdrop"));
    checks.require(
        headline_at.is_some() && backdrop_at.is_some() && headline_at < backdrop_at,
        "the headline is no longer submitted before the bar behind it",
        format!(
            "draw_headline is at {headline_at:?} and draw_backdrop at {backdrop_at:?}; with the \
             bar submitted first, no assertion over a recorded frame can see the band at all"
        ),
    );

    // --- the screen the run reaches exactly once ------------------------
    checks.require(
        last.stage_at_end == Stage::Complete,
        "finishing the last beat did not finish the chain",
        format!("the chain ended in {:?}", last.stage_at_end),
    );
    if let Some(frame) = &last.end_frame {
        for text_run in ui::complete_runs() {
            let drawn = glyph_run(frame, last.font, text_run.at);
            checks.require(
                drawn == text_run.text.chars().count(),
                "a row of the end-of-chain screen is not drawn as the string it is",
                format!(
                    "{:?} is {} characters and {drawn} glyphs were drawn at ({:.2}, {:.2})",
                    text_run.text,
                    text_run.text.chars().count(),
                    text_run.at.x,
                    text_run.at.y
                ),
            );
        }
    }

    // --- the background, which leaves no quad behind --------------------
    if let Some(frame) = &last.report_frame {
        let cleared = frame.plan.clear_color;
        checks.require(
            cleared == ui::BACKDROP,
            "the screen was cleared to a colour the game does not name",
            format!(
                "it cleared to {cleared:?}; the game's constant is {:?}",
                ui::BACKDROP
            ),
        );
        // And the requirement the colour exists to meet, which the constant
        // cannot move: every glyph on it is light, so it has to be dark.
        let brightness = cleared.r.max(cleared.g).max(cleared.b);
        checks.require(
            greater(0.25, brightness) && greater(cleared.a, 0.99),
            "the screen is not dark enough for light text to read against",
            format!(
                "its brightest channel is {brightness:.3} at alpha {:.2}",
                cleared.a
            ),
        );
    }

    // --- every string the game draws, in characters the font has --------
    contracts::printable_strings(&mut checks);
    // --- the contracts a played beat never reaches ----------------------
    contracts::battery(&mut checks, &tuning);
    // --- and the round that says whether any of it is an instrument -----
    let mutations = contracts::mutation_round(&mut checks);

    // The picture is of the screen a player spends the beat on, with a refusal
    // and its arithmetic on it: sheets, the job, the willingness sum, and a
    // name in red saying no. The report is asserted row by row above and the
    // transcript below is its frame; this is the one a person looks at.
    let shown = runs
        .iter()
        .find(|run| run.probe_frame.is_some())
        .unwrap_or(last);
    let captured = shown
        .probe_frame
        .as_ref()
        .or(shown.report_frame.as_ref())
        .map_or_else(
            || "skipped, no frame was recorded".to_owned(),
            |frame| crate::capture::capture_a_frame(&mut checks, frame, shown.font),
        );
    let verdict = checks.verdict();

    println!(
        "verified giri over {} beats, {} ticks of scripted pointer",
        CHAIN.len(),
        runs.iter().map(|run| run.frames).sum::<usize>()
    );
    println!(
        "  constants in effect: {}",
        tuning.readout().replace('\n', "  ")
    );
    for (index, run) in runs.iter().enumerate() {
        let spec = CHAIN.get(index).map_or("?", |spec| spec.title);
        println!(
            "  beat {} {spec:?}: {} frames, {} report rows, {} dead, {} refusals previewed",
            index + 1,
            run.frames,
            run.report.len(),
            run.after.members.iter().filter(|m| !m.alive).count(),
            run.probe.as_ref().map_or(0, |probe| probe
                .entries
                .iter()
                .filter(|e| !e.joins())
                .count()),
        );
    }
    println!("  closest quad to the edge: {clearance:.2} world units");
    println!("  mutation round: {mutations}");
    println!("  capture: {captured}");
    if let Some(frame) = &last.report_frame {
        print!("{}", frame.transcript());
    }
    verdict
}
