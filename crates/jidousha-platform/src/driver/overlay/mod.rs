//! The native performance overlay: what it measures, and what it prints.
//!
//! Key types: `Overlay`, `Level`; `SWITCH`, `requested`.
//! Depends on: `jidousha-core` (`Seconds`), `jidousha-render-core`
//! (`Presentation`, `BackendStats`), and this module's five halves — `panel`
//! (what every line of it says), `phases` (where a frame's milliseconds went),
//! `process` (what the operating system charges this program), `memory` (the
//! three memory tiers), `snapshot` (the one file it writes). Must never depend
//! on: `winit`, or anything that can reach the world — this reads numbers the
//! driver already had and produces a string.
//!
//! **This half owns the switch and the state**: what level was asked for, what
//! has been measured, and when the panel is due to be rebuilt. What it is
//! written as is [`panel`], which names none of the sampling and is where the
//! printable-ASCII rule below is enforced. The split is by length
//! (agent-practices §5.7) and it falls here because this is where measuring
//! ends and writing begins.
//! INVARIANT: **off unless asked, and presentation-only when asked.** Nothing
//! measured here reaches a tick, the accumulator, or a recorded transcript: the
//! quads it produces are appended after the Draw phase has finished, so a
//! `--verify` run and a replay see byte-identical submissions at every level
//! (ADR-0005, core.md §7). The sim counters it reports are read at draw time
//! through `World`'s ordinary read paths and nothing is written back.
//! INVARIANT: **the switch has levels and the levels are cumulative.** Level 1
//! is the pacing panel exactly as it was; level 2 adds the performance sections
//! and changes nothing about level 1's readings (frame-pacing.md §6, §7).
//!
//! **The web has its own pacing panel**, page-side, on `?frametime=1`
//! (web-publish.md §2). This is the native counterpart to *that* rather than a
//! port: a page can measure `requestAnimationFrame` from outside the wasm
//! module, and a native run has nothing outside itself to measure it from. The
//! performance sections are the other way round — a page cannot see a sim tick,
//! a texture upload or an entity count without calling into the module, which
//! its own contract forbids — so `?frametime=2` brings *this* panel up on the
//! web too, showing the sections only the module can answer and leaving the
//! pacing readings to the page (frame-pacing.md §7).
//!
//! DELIBERATE: every line composed here is **printable ASCII**, where the web
//! overlay's equivalent lines use an em dash. The built-in font covers ASCII 32
//! to 126 and draws a visible fallback box for everything else (renderer.md §6),
//! so an em dash in a readout is a box on screen — legible enough to mislead and
//! wrong enough to read as a rendering fault. The rule is stated here because
//! it constrains every reading this module takes; the test that keeps it true
//! is in [`panel`], where the lines are actually composed.

use jidousha_core::Seconds;
use jidousha_render_core::Presentation;

mod memory;
mod panel;
mod phases;
mod process;
mod snapshot;

pub(crate) use memory::Engine;
pub(crate) use phases::{Phase, Spans};
pub(crate) use snapshot::KEY as SNAPSHOT_KEY;

use phases::Breakdown;
use process::Process;
use snapshot::Snapshots;

/// The environment variable that turns the overlay on.
///
/// **Off unless this is set to something other than `0` or `false`**, matching
/// the web overlay's `?frametime=` exactly — including that bare
/// `JIDOUSHA_FRAMETIME=` (empty) counts as on, because the two switches
/// answering differently to the same shorthand is the kind of difference nobody
/// discovers until they are debugging something else.
///
/// An environment variable rather than a `GameConfig` field or a command-line
/// flag, and the reason is who turns it on: this is for the person *running*
/// the build, usually one a game author already shipped them. A config field
/// would need a rebuild, and a flag would have to be plumbed through every
/// game's own argument parsing — `--verify` is already each game's, not the
/// engine's (input.md §5).
///
/// Named on every target although only native reads it, the mirror of
/// [`PARAMETER`] below — one switch, written down in one place.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(crate) const SWITCH: &str = "JIDOUSHA_FRAMETIME";

/// The page parameter that is the same switch on the web.
///
/// Named on every target although only the web reads it, so the two halves of
/// one switch are written down beside each other rather than a `cfg` apart.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const PARAMETER: &str = "frametime";

/// The value that asks for the performance sections as well as the pacing ones.
///
/// **One switch with levels rather than two switches.** A second variable would
/// mean two things to remember, two things to document, and a state where one
/// is set and the other is not that nobody would have thought about; a level
/// says "more of the same instrument" and cannot be self-contradictory.
const PERF_LEVEL: &str = "2";

/// How many frame deltas the readout describes.
///
/// Four seconds at sixty a second: long enough that one hitch does not define
/// the median, short enough that the panel describes what is happening now
/// rather than averaging the last minute away. The same window the web
/// overlay's rolling histogram uses, and the same window every level-2 section
/// reports over — one window, so two readings taken from the same panel
/// describe the same stretch of time.
const WINDOW: usize = 240;

/// How often the readout is rebuilt, in seconds of frames.
///
/// Four times a second. The numbers are for a person to read, and a median that
/// changes sixty times a second cannot be read at all; the histogram below it
/// would strobe. Measured out of the frame deltas already collected rather than
/// off a clock, so the panel costs the run no extra timing call.
const REPAINT_PERIOD: Seconds = Seconds(0.25);

/// Buckets for ticks per rendered frame: 0, 1, 2, and 3-or-more.
const TICK_BUCKETS: usize = 4;

/// How wide the histogram's bars are, in characters.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
const BAR: usize = 20;

/// How much of the panel this run asked for.
///
/// Three states rather than a `bool` because the middle one is a promise: a run
/// at [`Pacing`](Level::Pacing) gets the panel frame-pacing.md §6 documents,
/// character for character, and nothing this branch added can appear on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Level {
    /// Nothing is drawn and nothing is measured.
    Off,
    /// The pacing panel: frame deltas, ticks per frame, and the present mode.
    Pacing,
    /// The pacing panel plus the performance sections.
    Perf,
}

impl Level {
    /// What the panel's first line calls the switch that produced it.
    ///
    /// On the panel because a screenshot has to say how it was produced — the
    /// commonest question about any diagnostic image is "how do I get that" —
    /// and it names the *level*, so a reader can tell a level-1 screenshot from
    /// a level-2 one without counting sections.
    #[cfg(not(target_arch = "wasm32"))]
    fn switch(self) -> String {
        match self {
            Level::Off | Level::Pacing => format!("{SWITCH}=1"),
            Level::Perf => format!("{SWITCH}={PERF_LEVEL}"),
        }
    }

    /// The same, as the page URL that asks for it.
    #[cfg(target_arch = "wasm32")]
    fn switch(self) -> String {
        match self {
            Level::Off | Level::Pacing => format!("?{PARAMETER}=1"),
            Level::Perf => format!("?{PARAMETER}={PERF_LEVEL}"),
        }
    }
}

/// What level a switch value asks for.
///
/// Shared by both targets so the environment variable and the page parameter
/// cannot drift apart: `0` and `false` are off, `2` is the performance panel,
/// and anything else — including an empty value — is the pacing panel.
///
/// A value nobody planned for (`?frametime=banana`) is the pacing panel rather
/// than an error. It is a diagnostic switch, and refusing to start a game over
/// a typo in one would be the worse failure; the panel names the level it is
/// actually at, which is where a mistyped `3` is discovered.
fn level_of(value: &str) -> Level {
    match value {
        "0" | "false" => Level::Off,
        PERF_LEVEL => Level::Perf,
        _ => Level::Pacing,
    }
}

/// What level this run was asked for.
///
/// Read once at startup, like `?renderscale=` is (web/render_scale.rs): a
/// switch that changed under a running loop would be a resize and a
/// reconfiguration nobody asked for.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn requested() -> Level {
    match std::env::var(SWITCH) {
        Ok(value) => level_of(&value),
        Err(_) => Level::Off,
    }
}

/// On the web, the page owns the pacing panel and this owns the rest.
///
/// `?frametime=1` is page-side and measures `requestAnimationFrame` from
/// outside the wasm module (web-publish.md §2), so this stays off for it: a
/// second pacing panel inside the module would draw over the one already on the
/// page and measure the same frames twice.
///
/// `?frametime=2` is different in kind. Sim ticks, texture uploads, entity
/// counts and GPU timings are not visible from a page at all, and the page's
/// own contract forbids it calling into the module to ask — so the performance
/// sections have to be drawn from in here. The two panels sit in opposite
/// corners and each shows what only it can see.
#[cfg(target_arch = "wasm32")]
pub(crate) fn requested() -> Level {
    let Some(window) = web_sys::window() else {
        return Level::Off;
    };
    let Ok(search) = window.location().search() else {
        return Level::Off;
    };
    match crate::web::query_parameter(&search, PARAMETER).map(level_of) {
        Some(Level::Perf) => Level::Perf,
        // Including `?frametime=1`: the page is already drawing that panel.
        _ => Level::Off,
    }
}

/// What the overlay has seen, and what it says about it.
///
/// Constructed on every run and empty on almost all of them: an overlay nobody
/// asked for records nothing, formats nothing, and allocates nothing, so the
/// off-by-default path costs a branch per frame.
pub(crate) struct Overlay {
    level: Level,
    /// The rolling window, in milliseconds, oldest first.
    deltas: std::collections::VecDeque<f32>,
    /// How many frames ran 0, 1, 2, and 3-or-more ticks — over the whole run,
    /// not the window, because the *share* is what the reading is and a share
    /// over four seconds jumps around too much to compare.
    ticks: [u64; TICK_BUCKETS],
    frames: u64,
    /// Frame time banked toward the next repaint.
    since_repaint: Seconds,
    readout: String,
    /// Where each frame's milliseconds went. Level 2 only, and empty otherwise.
    breakdown: Breakdown,
    /// GPU milliseconds per frame, where the device will say. Level 2 only.
    gpu: std::collections::VecDeque<f32>,
    /// The operating system's own counters, sampled at 1Hz. Level 2 only.
    process: Process,
    /// What the engine is holding, as of the last frame. Level 2 only.
    engine: Engine,
    /// The snapshot key's writer, and what its last press produced.
    snapshots: Snapshots,
    wrote: Option<String>,
}

impl Overlay {
    /// An overlay at the level this run asked for.
    ///
    /// The level is an argument rather than read here so that the switch is
    /// consulted in exactly one place — `Driver::new` — and so that every case
    /// below can be tested without setting an environment variable, which is
    /// process-global state two tests running in parallel would fight over.
    pub(crate) fn new(level: Level) -> Self {
        Self {
            level,
            deltas: std::collections::VecDeque::new(),
            ticks: [0; TICK_BUCKETS],
            frames: 0,
            since_repaint: Seconds(0.0),
            readout: String::new(),
            breakdown: Breakdown::new(WINDOW),
            gpu: std::collections::VecDeque::new(),
            // The baseline is taken only at the level that reads it: an
            // overlay nobody asked for opens no file (process.rs).
            process: if level == Level::Perf {
                Process::start()
            } else {
                Process::idle()
            },
            engine: Engine::default(),
            snapshots: Snapshots::new(),
            wrote: None,
        }
    }

    /// Whether anything should be drawn.
    pub(crate) fn is_on(&self) -> bool {
        self.level != Level::Off
    }

    /// Whether the driver should time this frame's phases.
    ///
    /// The **one** thing that costs a frame anything when the panel is up: four
    /// extra clock reads. A run at level 1 or off does not take them, which is
    /// what keeps the pacing panel exactly as expensive as it was
    /// (frame-pacing.md §7).
    pub(crate) fn wants_phases(&self) -> bool {
        self.level == Level::Perf
    }

    /// Take one frame's reading: how long it was, and how many ticks it ran.
    ///
    /// `elapsed` is the clamped frame duration the accumulator was given, which
    /// is the honest number to report — it is what the loop actually spent on
    /// this frame's worth of simulation, and a frame past `MAX_FRAME` shows up
    /// as the ceiling here exactly as it does in the tick counts beside it
    /// (core.md §7).
    pub(crate) fn record(&mut self, elapsed: Seconds, ticks: u32, presentation: Presentation) {
        if !self.is_on() {
            return;
        }
        self.deltas.push_back(elapsed.as_f32() * 1000.0);
        while self.deltas.len() > WINDOW {
            self.deltas.pop_front();
        }
        self.ticks[(ticks as usize).min(TICK_BUCKETS - 1)] += 1;
        self.frames += 1;
        if self.level == Level::Perf {
            self.process.advance(elapsed);
        }

        self.since_repaint = Seconds(self.since_repaint.as_f32() + elapsed.as_f32());
        // The first frame repaints too, so a screenshot taken immediately says
        // what it knows rather than "measuring…".
        if self.since_repaint >= REPAINT_PERIOD || self.frames == 1 {
            self.since_repaint = Seconds(0.0);
            self.readout = self.compose(presentation);
        }
    }

    /// Close the previous frame's breakdown, now that its duration is known.
    ///
    /// Called at the top of a frame with the elapsed time that frame was given:
    /// that span runs from the *previous* frame's start to now, so it is the
    /// previous frame's whole duration and the only total its four measured
    /// spans can honestly be subtracted from. The breakdown is therefore one
    /// frame behind the pacing readings above it, which at four repaints a
    /// second nobody can see and which is the price of never inventing a number
    /// (frame-pacing.md §7).
    pub(crate) fn close_frame(&mut self, elapsed: Seconds, spans: Spans) {
        if self.level != Level::Perf {
            return;
        }
        self.breakdown.record(elapsed, spans);
    }

    /// Take this frame's engine-side counters.
    ///
    /// Every number in here is a running total something already keeps — the
    /// backend's byte counts, the world's entity count, the frame's own
    /// submission count — so this is a read rather than a walk, and it is taken
    /// at draw time through the ordinary read paths (frame-pacing.md §7).
    pub(crate) fn observe(&mut self, engine: Engine) {
        if self.level != Level::Perf {
            return;
        }
        if let Some(gpu) = engine.backend.gpu_frame {
            self.gpu.push_back(gpu.as_f32() * 1000.0);
            while self.gpu.len() > WINDOW {
                self.gpu.pop_front();
            }
        }
        self.engine = engine;
    }

    /// Write the panel to a file, because somebody pressed the snapshot key.
    ///
    /// Only at level 2, and only on a press: this is the only file the overlay
    /// ever writes. The path lands on the panel so the person who pressed the
    /// key can see that it worked and where it went.
    pub(crate) fn snapshot(&mut self) {
        if self.level != Level::Perf {
            return;
        }
        self.wrote = match self.snapshots.write(&self.readout) {
            Some(path) => Some(format!("wrote {}", path.display())),
            None => Some("could not write under target/".to_owned()),
        };
    }

    /// What to draw, as lines of text.
    pub(crate) fn readout(&self) -> &str {
        &self.readout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jidousha_render_core::BackendStats;

    /// An overlay showing the pacing panel, whatever this machine's environment
    /// says.
    fn on() -> Overlay {
        Overlay::new(Level::Pacing)
    }

    /// An overlay showing everything.
    fn perf() -> Overlay {
        Overlay::new(Level::Perf)
    }

    /// One frame's spans, in milliseconds.
    fn spans(sim: f32, draw: f32, encode: f32, present: f32) -> Spans {
        let mut spans = Spans::new();
        let mut at = 0.0;
        for (phase, milliseconds) in [
            (Phase::Sim, sim),
            (Phase::Draw, draw),
            (Phase::Encode, encode),
            (Phase::Present, present),
        ] {
            at += milliseconds;
            spans.spent(phase, Seconds(at / 1000.0));
        }
        spans
    }

    #[test]
    fn an_overlay_nobody_asked_for_records_nothing_and_says_nothing() {
        // The off-by-default promise, from the inside: not merely "is not
        // drawn" but "has nothing to draw", so a switch flipped by accident
        // half way through a run cannot produce a panel full of history.
        let mut overlay = Overlay::new(Level::Off);
        for _ in 0..10 {
            overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        }
        assert!(overlay.readout().is_empty());
        assert!(!overlay.is_on());
        assert!(!overlay.wants_phases());
        assert_eq!(overlay.frames, 0);
    }

    #[test]
    fn the_switch_reads_the_same_shorthand_on_both_targets() {
        // The two switches answering differently to the same value is the kind
        // of difference nobody discovers until they are debugging something
        // else, so one function decides it for both (`level_of`).
        assert_eq!(level_of("0"), Level::Off);
        assert_eq!(level_of("false"), Level::Off);
        assert_eq!(level_of(""), Level::Pacing, "a bare switch is on");
        assert_eq!(level_of("1"), Level::Pacing);
        assert_eq!(level_of("true"), Level::Pacing);
        assert_eq!(level_of("2"), Level::Perf);
    }

    #[test]
    fn a_level_nobody_planned_for_is_the_pacing_panel_rather_than_a_refusal() {
        // A diagnostic switch must never be the reason a game will not start,
        // and the panel names the level it actually reached — which is where a
        // mistyped 3 is noticed.
        assert_eq!(level_of("3"), Level::Pacing);
        assert_eq!(level_of("banana"), Level::Pacing);
    }

    #[test]
    fn a_run_at_level_one_measures_no_phases_and_reports_none() {
        // The cost promise: the driver only reads the clock four extra times a
        // frame when the sections that use those readings are on.
        let mut overlay = on();
        assert!(!overlay.wants_phases());
        overlay.close_frame(Seconds(1.0 / 60.0), spans(0.3, 0.1, 0.1, 16.17));
        assert!(overlay.breakdown.is_empty());
        assert!(perf().wants_phases());
    }

    #[test]
    fn the_window_stops_growing_and_describes_the_recent_frames() {
        let mut overlay = on();
        for _ in 0..(WINDOW * 3) {
            overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        }
        assert_eq!(overlay.deltas.len(), WINDOW);
        assert!(
            overlay.readout().contains(&format!("over {WINDOW} frames")),
            "{}",
            overlay.readout()
        );
    }

    #[test]
    fn the_instruments_own_bookkeeping_costs_a_frame_almost_nothing() {
        // The other half of "an instrument that perturbs is the failure mode",
        // and the half this module owns: what the *measuring* costs, with the
        // drawing left out. Every per-frame path at level 2 — the rolling
        // windows, the 1Hz process sampler, the engine counters, and the panel
        // rebuilt four times a second — against level 1's, over frames long
        // enough to cross several repaints.
        //
        // The end-to-end figure, which is dominated by drawing a quad per
        // character of the panel rather than by any of this, is measured in
        // `driver/frame.rs`.
        let cost_of = |level| {
            let mut overlay = Overlay::new(level);
            let mut run = |frames: u32| {
                for _ in 0..frames {
                    overlay.close_frame(Seconds(1.0 / 60.0), spans(0.3, 0.1, 0.1, 16.17));
                    overlay.observe(Engine {
                        backend: BackendStats {
                            texture_bytes: 12 * 1024 * 1024,
                            buffer_bytes: 512 * 1024,
                            gpu_frame: Some(Seconds(0.0021)),
                        },
                        entities: 412,
                        components: 1236,
                        quads: 318,
                    });
                    overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
                }
            };
            // Fill the windows first, so the measurement is of steady state
            // rather than of a panel describing forty frames.
            run(WINDOW as u32);
            let started = web_time::Instant::now();
            run(BOOKKEEPING_FRAMES);
            started.elapsed().as_secs_f64() / f64::from(BOOKKEEPING_FRAMES)
        };
        let pacing = cost_of(Level::Pacing);
        let perf = cost_of(Level::Perf);
        let added = (perf - pacing) * 1e6;
        println!(
            "instrument bookkeeping: level 1 {:.2}us, level 2 {:.2}us (+{added:.2}) a frame",
            pacing * 1e6,
            perf * 1e6
        );
        // A hundredth of the tick period the engine's picture changes at.
        // Stated against `fixed_dt` so the bound moves with the thing it is a
        // share of, and loose enough not to flake on a loaded runner while
        // still catching the structural mistakes: sampling the operating
        // system every frame, or composing the panel every frame, would each
        // land well past it.
        let hundredth =
            f64::from(jidousha_core::GameConfig::default().fixed_dt.as_f32()) * 1e6 / 100.0;
        assert!(
            added < hundredth,
            "the level-2 sections added {added:.2}us a frame of measuring against a              {hundredth:.2}us bound"
        );
    }

    /// How many frames the bookkeeping measurement above times.
    const BOOKKEEPING_FRAMES: u32 = 20_000;

    #[test]
    fn a_snapshot_press_at_level_one_writes_nothing_at_all() {
        // The key belongs to the performance panel. At level 1 it is a key the
        // game gets and the overlay does not act on — which is also what keeps
        // the level-1 panel exactly what it was.
        let mut overlay = on();
        overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        overlay.snapshot();
        assert_eq!(overlay.wrote, None);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_snapshot_press_writes_one_file_and_the_panel_names_it() {
        let mut overlay = perf();
        overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        overlay.snapshot();
        let Some(wrote) = overlay.wrote.clone() else {
            panic!("the press produced no report at all");
        };
        let Some(path) = wrote.strip_prefix("wrote ") else {
            panic!("target/ was not writable from this test: {wrote}");
        };
        let Ok(written) = std::fs::read_to_string(path) else {
            panic!("the panel named a file that is not there: {path}");
        };
        assert!(written.contains("jidousha performance:"), "{written}");
        let _ = std::fs::remove_file(path);
    }
}
