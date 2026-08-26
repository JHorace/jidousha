//! `--verify`: the chain, played by a script, asserted on, and photographed.
//!
//! The beats are the test suite (DESIGN §14). Each one is driven end to end
//! through `InputScript` - the same pointer a person uses, at the same
//! rectangles - and judged three ways: on world state, on the null-backend
//! transcript, and against UI.md §7's readability floors.
//!
//! The script does more than assemble. It probes the **door rule** in both
//! directions on any beat that has a refusal in it: one order makes the
//! newcomer refuse, the reverse order makes the incumbent block, and DESIGN
//! §3.2 says those are the same numbers seen from two sides. Both are asserted
//! to bounce, to name the right person, and to leave the party untouched.
//!
//! Then the things play cannot reach: the contract battery, the mutation round,
//! and the captures - one PNG per screen mode at reference size and at a narrow
//! one, because a scaling regression is invisible to every assertion here
//! (UI.md §8).

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord, FrameRecorder, InputScript};

use crate::beats::{BeatSpec, CHAIN, Expect};
use crate::checks::{Checks, fail, greater, one_line};
use crate::constants::Tuning;
use crate::flow::{Flow, Preview, Stage, StartAt};
use crate::frames::judge_frames;
use crate::judge::judge_world;
use crate::model::Social;
use crate::{
    capture, contracts, floors, layout, library, links, mutation, onset, restart, scaling, sprites,
    tuning,
};

/// The surface the reference run draws at: UI.md §6's reference resolution,
/// doubled, which is the window the game opens.
pub const HEADLESS_VIEWPORT: PhysicalSize = crate::WINDOW;

/// The narrow surface UI.md §8 asks the second capture set for.
///
/// Narrow rather than short on purpose: horizontal shrink is the axis §6's
/// defect was on, and a capture set that only ever got shorter would have gone
/// on passing through it.
pub const NARROW_VIEWPORT: PhysicalSize = PhysicalSize::new(600, 540);

/// The camera a scripted click is aimed through.
///
/// Built by `scaling::camera_for`, which is also what the game fits its camera
/// with every tick - so a script converts a world rectangle to pixels with the
/// same camera the game converts back with. Nothing stamps `Camera::viewport`
/// under `headless`, and two cameras that disagreed would send every click to
/// the wrong pixel and fail the run with an empty party and no clue why.
pub fn headless_camera(viewport: PhysicalSize) -> Camera {
    scaling::camera_for(viewport)
}

/// A scripted click: put the pointer on a world point, then tap it there.
fn click(script: InputScript, tick: &mut u64, at: Vec2, viewport: PhysicalSize) -> InputScript {
    let screen = headless_camera(viewport).world_to_screen(at);
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
    /// The tick the board is on screen with the quest taken and nobody staged.
    board_at: u64,
    /// The tick a refusal has just bounced, if the beat has one.
    refusal_at: Option<u64>,
    /// The tick an incumbent's veto has just bounced, if the beat has one.
    veto_at: Option<u64>,
    /// Who the veto probe expected to be blocked, and by whom.
    veto_names: Option<(&'static str, &'static str)>,
    /// Who the refusal probe expected to refuse.
    refusal_name: Option<&'static str>,
    /// The tick the assembled party is on screen and sendable.
    ready_at: u64,
    /// The tick the takeover is up.
    report_at: u64,
    /// The tick after dismissing it.
    end_at: u64,
    /// The last tick to run.
    last: u64,
}

/// The pair a door probe is built from: somebody who refuses a party, and the
/// one other member of it.
///
/// Both probes come out of the beat's own `Expect::Refuses`, so a beat that has
/// no refusal in it is probed for neither and says so rather than inventing a
/// case its roster does not hold.
fn refusal_pair(spec: &BeatSpec) -> Option<(&'static str, &'static str)> {
    spec.expect.iter().find_map(|expect| match expect {
        Expect::Refuses { who, party } if party.len() == 2 => party
            .iter()
            .find(|name| *name != who)
            .map(|other| (*who, *other)),
        _ => None,
    })
}

/// Build the session: take the quest, probe the door from both sides, assemble
/// the intended party, send it, read the takeover, continue.
fn plan_for(spec: &BeatSpec, viewport: PhysicalSize) -> Plan {
    let mut script = InputScript::new();
    let mut tick = 3;
    let click_at =
        |script: InputScript, tick: &mut u64, at: Vec2| click(script, tick, at, viewport);

    // The quest first: the send verb does not exist until one is taken.
    script = click_at(script, &mut tick, layout::quest_card(0).center());
    let board_at = tick;
    tick += 1;

    let pair = refusal_pair(spec);
    let card = |name: &str| {
        spec.index_of(name)
            .map(|index| layout::party_card(index).center())
    };

    // Rule 1, the newcomer's own refusal: the other one goes in first.
    let mut refusal_at = None;
    let mut refusal_name = None;
    if let Some((refuser, other)) = pair
        && let (Some(refuser_card), Some(other_card)) = (card(refuser), card(other))
    {
        script = click_at(script, &mut tick, other_card);
        script = click_at(script, &mut tick, refuser_card);
        refusal_at = Some(tick);
        refusal_name = Some(refuser);
        tick += 1;
        // Take it back: a refusal is feedback, not a failure (DESIGN §7).
        script = click_at(script, &mut tick, other_card);
    }

    // Rule 2, the incumbent's veto: the *same two people, the other way round*.
    let mut veto_at = None;
    let mut veto_names = None;
    if let Some((refuser, other)) = pair
        && let (Some(refuser_card), Some(other_card)) = (card(refuser), card(other))
    {
        script = click_at(script, &mut tick, refuser_card);
        script = click_at(script, &mut tick, other_card);
        veto_at = Some(tick);
        veto_names = Some((other, refuser));
        tick += 1;
        script = click_at(script, &mut tick, refuser_card);
    }

    for name in spec.send {
        if let Some(at) = card(name) {
            script = click_at(script, &mut tick, at);
        }
    }
    let ready_at = tick;
    tick += 1;
    script = click_at(script, &mut tick, layout::send_button().center());
    let report_at = tick;
    tick += 1;
    script = click_at(script, &mut tick, layout::takeover().center());
    let end_at = tick;
    Plan {
        script,
        board_at,
        refusal_at,
        veto_at,
        veto_names,
        refusal_name,
        ready_at,
        report_at,
        end_at,
        last: end_at + 1,
    }
}

/// What a door probe produced: the message, and the party it left behind.
#[derive(Clone, Debug, Default)]
pub struct Bounce {
    /// The toast, if one was raised.
    pub toast: Option<String>,
    /// The most recent log line.
    pub logged: Option<String>,
    /// The party at that moment, by name and in order.
    pub party: Vec<&'static str>,
}

/// What one scripted beat did.
pub struct BeatRun {
    /// Which beat.
    pub index: usize,
    /// The constants the run was played with — what every screen it recorded is
    /// stamped with, and what a floor or a judge has to rebuild a screen from.
    pub tuning: Tuning,
    /// The social state the beat was authored with, read after Startup.
    pub at_assembly: Social,
    /// The social state the dungeon left behind.
    pub after: Social,
    /// The rule-1 probe.
    pub refusal: Option<Bounce>,
    /// Who it expected to refuse.
    pub refusal_name: Option<&'static str>,
    /// The rule-2 probe.
    pub veto: Option<Bounce>,
    /// Who it expected to be blocked, and by whom.
    pub veto_names: Option<(&'static str, &'static str)>,
    /// The preview once the intended party was assembled.
    pub ready: Preview,
    /// The flow once the intended party was assembled.
    pub ready_flow: Flow,
    /// The flow with the quest taken and nobody staged.
    pub board_flow: Flow,
    /// The preview at that moment.
    pub board_preview: Preview,
    /// The flow while the takeover was up.
    pub report_flow: Flow,
    /// The preview at that moment.
    pub report_preview: Preview,
    /// The resolution's narration.
    pub report: Vec<String>,
    /// The stage the send verb produced.
    pub stage_after_send: Stage,
    /// The stage dismissing the takeover produced.
    pub stage_at_end: Stage,
    /// Which beat that left the game on.
    pub beat_at_end: usize,
    /// How many frames were drawn.
    pub frames: usize,
    /// The board with the quest taken and the party empty.
    pub board_frame: Option<FrameRecord>,
    /// The frame the refusal bounced on.
    pub refusal_frame: Option<FrameRecord>,
    /// The frame the veto bounced on.
    pub veto_frame: Option<FrameRecord>,
    /// The board with the party staged and the send verb live.
    pub ready_frame: Option<FrameRecord>,
    /// The resolution takeover.
    pub report_frame: Option<FrameRecord>,
    /// The frame after dismissing it.
    pub end_frame: Option<FrameRecord>,
    /// Everything drawn outside the design rect, over every frame.
    pub outside: Vec<Rect>,
    /// How close the closest quad came to the design rect's edge.
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

fn names(social: &Social, party: &[Entity]) -> Vec<&'static str> {
    party.iter().map(|entity| social.name(*entity)).collect()
}

/// Play one beat through the script, with `tuning` in effect.
///
/// `record` off is the mutation round's shape: the world assertions are the
/// ones a perturbed constant is judged on, and a thousand unread frames are a
/// thousand frames to allocate (FINDINGS G-004).
pub fn play(index: usize, tuning: Tuning, record: bool) -> BeatRun {
    play_at(index, tuning, record, HEADLESS_VIEWPORT)
}

/// The same, on a surface of a stated size — what the narrow capture set runs.
pub fn play_at(index: usize, tuning: Tuning, record: bool, viewport: PhysicalSize) -> BeatRun {
    let Some(spec) = CHAIN.get(index) else {
        fail(
            "a beat was asked for that the chain does not have",
            &format!("beat {index} of {}", CHAIN.len()),
        );
    };
    let plan = plan_for(spec, viewport);
    let mut sim = headless(crate::config(), crate::register);
    // Before Startup, which is what `open_the_chain` reads them with.
    sim.world_mut().insert_resource(tuning);
    sim.world_mut().insert_resource(StartAt(index));
    sim.world_mut().insert_resource(scaling::Surface(viewport));

    let mut recorder = record.then(|| FrameRecorder::new(viewport));
    let font = recorder
        .as_ref()
        .map_or(BackendTextureId(0), FrameRecorder::font_texture);
    let mut run = BeatRun {
        index,
        tuning,
        at_assembly: Social::default(),
        after: Social::default(),
        refusal: None,
        refusal_name: plan.refusal_name,
        veto: None,
        veto_names: plan.veto_names,
        ready: Preview::default(),
        ready_flow: Flow::default(),
        board_flow: Flow::default(),
        board_preview: Preview::default(),
        report_flow: Flow::default(),
        report_preview: Preview::default(),
        report: Vec::new(),
        stage_after_send: Stage::Board,
        stage_at_end: Stage::Board,
        beat_at_end: index,
        frames: 0,
        board_frame: None,
        refusal_frame: None,
        veto_frame: None,
        ready_frame: None,
        report_frame: None,
        end_frame: None,
        outside: Vec::new(),
        clearance: f32::MAX,
        quads: 0,
        font,
        camera: headless_camera(viewport),
        schedule: sim.schedule_debug(),
    };

    for tick in 1..=plan.last {
        sim.world_mut()
            .insert_resource(Input::new(plan.script.snapshot_at(tick)));
        sim.tick();
        if tick == 1 {
            // The art, before the first frame is photographed. `sprites::settle`
            // is what keeps a recorded run reproducible now that the pictures
            // come off a disk: without it the transcript would say whichever of
            // the thirteen files the loader thread had finished, which is a
            // property of the machine rather than of the game.
            if recorder.is_some() {
                let assets = sim.world_mut().resource_mut::<Assets>();
                if let Some(failure) = sprites::settle(assets).first() {
                    // Not one fault among several: every frame after this one is
                    // a picture of placeholders, so every reading taken off it
                    // would be a reading of the wrong screen.
                    fail(
                        "giri's art did not load for a recorded run",
                        &one_line(&failure.message()),
                    );
                }
            }
            run.at_assembly = Social::read(&sim.world().view());
            run.camera = Camera {
                viewport,
                ..*sim.world().resource::<Camera>()
            };
            run.schedule = sim.schedule_debug();
        }
        if let Some(recorder) = recorder.as_mut() {
            recorder.settle_assets(&mut sim, tick);
        }
        let frame = recorder.as_mut().map(|recorder| recorder.draw(&mut sim));
        if let Some(frame) = &frame {
            run.frames += 1;
            let design = layout::design();
            for quad in frame.quads() {
                let bounds = quad.bounds();
                run.quads += 1;
                if !inside(design, bounds) {
                    run.outside.push(bounds);
                }
                let gap = (bounds.min - design.min).min(design.max - bounds.max);
                run.clearance = run.clearance.min(gap.x.min(gap.y));
            }
        }
        let social = Social::read(&sim.world().view());
        let flow = sim.world().resource::<Flow>().clone();
        if tick == plan.board_at {
            run.board_flow = flow.clone();
            run.board_preview = sim.world().resource::<Preview>().clone();
            run.board_frame = frame.clone();
        }
        if Some(tick) == plan.refusal_at {
            run.refusal = Some(bounce_of(&flow, &social));
            run.refusal_frame = frame.clone();
        }
        if Some(tick) == plan.veto_at {
            run.veto = Some(bounce_of(&flow, &social));
            run.veto_frame = frame.clone();
        }
        if tick == plan.ready_at {
            run.ready = sim.world().resource::<Preview>().clone();
            run.ready_flow = flow.clone();
            run.ready_frame = frame.clone();
        }
        if tick == plan.report_at {
            run.stage_after_send = flow.stage;
            run.report = flow.report.clone();
            run.report_flow = flow.clone();
            run.report_preview = sim.world().resource::<Preview>().clone();
            run.after = social.clone();
            run.report_frame = frame.clone();
        }
        if tick == plan.end_at {
            run.stage_at_end = flow.stage;
            run.beat_at_end = flow.beat;
            run.end_frame = frame;
        }
    }
    run
}

fn bounce_of(flow: &Flow, social: &Social) -> Bounce {
    Bounce {
        toast: flow.toast.as_ref().map(|toast| toast.text.clone()),
        logged: flow.log.first().cloned(),
        party: names(social, &flow.party),
    }
}

/// Whether `bounds` sits inside `area`, to within a hundredth of a world unit.
///
/// A tolerance rather than `contains_rect`, because the design rect's own
/// background quad is exactly the design rect: the camera's width is
/// `height * viewport.aspect()` in f32, and whether that lands a hair over or
/// under 960 is a rounding question no requirement should depend on.
pub fn inside(area: Rect, bounds: Rect) -> bool {
    const SLACK: f32 = 0.01;
    bounds.min.x >= area.min.x - SLACK
        && bounds.min.y >= area.min.y - SLACK
        && bounds.max.x <= area.max.x + SLACK
        && bounds.max.y <= area.max.y + SLACK
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
        judge_frames(&mut checks, &played);
        floors::judge(&mut checks, &played);
        onset::judge(&mut checks, &played);
        runs.push(played);
    }
    let Some(last) = runs.last() else {
        fail(
            "the chain has no beats",
            "beats.rs::CHAIN is what a beat is added to",
        );
    };

    // --- nothing outside the design rect, over every frame of every beat ---
    let outside: Vec<&Rect> = runs.iter().flat_map(|run| run.outside.iter()).collect();
    checks.require(
        outside.is_empty(),
        "something was drawn outside the 960x540 the layout is stated in",
        format!(
            "{} quads of {} fall outside {:?}; the first is {:?} - a wrapped line or a \
             status line wider than its card is the usual culprit",
            outside.len(),
            runs.iter().map(|run| run.quads).sum::<usize>(),
            layout::design(),
            outside.first(),
        ),
    );
    let clearance = runs
        .iter()
        .map(|run| run.clearance)
        .fold(f32::MAX, f32::min);

    floors::layout_floors(&mut checks);
    floors::tuner_floors(&mut checks);
    let scaling_report = floors::scaling_contract(&mut checks);

    // --- the tuning drawer: one scripted session, read four ways ----------
    let drawer = restart::drawer_run(true);
    restart::judge(&mut checks, &drawer);
    floors::judge_tuner(&mut checks, &drawer);
    links::link_contracts(&mut checks);

    // --- the schedule order, which nothing else can see -------------------
    let order = &last.schedule;
    let marks = |name: &str| order.find(name);
    for (first, second, why) in [
        (
            "fit",
            "handle_pointer",
            "the camera is fitted after the click that is converted through it, so a resized \
             window sends one frame's clicks to the previous frame's rectangles",
        ),
        (
            "handle_pointer",
            "refresh_preview",
            "the preview is computed before the click that changes it, so the arithmetic on \
             screen is the previous tick's party",
        ),
        (
            "draw_overlay",
            "draw_content",
            "the log drawer's scrim is submitted after the text it is meant to sit behind",
        ),
    ] {
        let (a, b) = (marks(first), marks(second));
        checks.require(
            a.is_some() && b.is_some() && a < b,
            "a system order the game depends on has been reversed",
            format!("{first} is at {a:?} and {second} at {b:?} in the schedule; {why}"),
        );
    }

    // --- the screen the run reaches exactly once --------------------------
    checks.require(
        last.stage_at_end == Stage::Complete,
        "finishing the last beat did not finish the chain",
        format!("the chain ended in {:?}", last.stage_at_end),
    );

    // --- the background, which leaves no quad behind ----------------------
    if let Some(frame) = &last.report_frame {
        let cleared = frame.plan.clear_color;
        checks.require(
            cleared == crate::theme::VOID,
            "the screen was cleared to a colour the game does not name",
            format!(
                "it cleared to {cleared:?}; the letterbox's constant is {:?}",
                crate::theme::VOID
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

    // --- the art library, and every string the game draws -----------------
    library::library(&mut checks);
    library::printable_strings(&mut checks);
    // --- the people vocabulary's own shape: caps, names, table cells ------
    crate::traits::vocabulary(&mut checks);
    // --- the contracts a played beat never reaches ------------------------
    contracts::battery(&mut checks, &tuning);
    // --- and the round that says whether any of it is an instrument -------
    let mutations = mutation::mutation_round(&mut checks);
    // --- the pictures a person looks at -----------------------------------
    let captured = capture::capture_screens(&mut checks, &runs, tuning, &drawer);

    let verdict = checks.verdict();
    println!(
        "verified giri over {} beats, {} frames of scripted pointer",
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
            "  beat {} {spec:?}: {} frames, {} report rows, {} event cards, {} dead, \
             door probes {}",
            index + 1,
            run.frames,
            run.report.len(),
            run.report_flow.events.len(),
            run.after.members.iter().filter(|m| !m.alive).count(),
            match (&run.refusal, &run.veto) {
                (Some(_), Some(_)) => "refusal and veto",
                (Some(_), None) => "refusal only",
                (None, Some(_)) => "veto only",
                (None, None) => "none - no refusal in this beat",
            },
        );
    }
    println!(
        "  tuning drawer: applied {} at beat {} - {} steppers, {} presets, stamp {}",
        tuning::name_of(&drawer.applied).unwrap_or("a hand-stepped set"),
        restart::BEAT + 1,
        floors::tuner_targets().len() - crate::presets::PRESETS.len() - 1,
        crate::presets::PRESETS.len(),
        drawer.applied_active.stamp(),
    );
    println!("  closest quad to the design edge: {clearance:.2} world units");
    println!("  scaling: {scaling_report}");
    println!("  mutation round: {mutations}");
    println!("{captured}");
    if let Some(frame) = &last.report_frame {
        print!("{}", frame.transcript());
    }
    verdict
}

/// Every screen mode a capture set covers, and the frame each comes from.
pub fn screen_modes(run: &BeatRun) -> Vec<(&'static str, Option<&FrameRecord>)> {
    vec![
        ("board", run.board_frame.as_ref()),
        ("staged", run.ready_frame.as_ref()),
        ("resolution", run.report_frame.as_ref()),
    ]
}
