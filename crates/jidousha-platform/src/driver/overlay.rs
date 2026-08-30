//! The native frame-pacing overlay: what it measures, and what it prints.
//!
//! Key types: `Overlay`; `SWITCH`, `requested`.
//! Depends on: `jidousha-core` (`Seconds`), `jidousha-render-core`
//! (`Presentation`). Must never depend on: `winit`, or anything that can reach
//! the world — this reads frame durations the driver already had and produces a
//! string.
//! INVARIANT: **off unless asked, and presentation-only when asked.** Nothing
//! measured here reaches a tick, the accumulator, or a recorded transcript: the
//! quads it produces are appended after the Draw phase has finished, so a
//! `--verify` run and a replay see byte-identical submissions whether the
//! overlay is on or not (ADR-0005, core.md §7).
//!
//! **The web has its own**, page-side, on `?frametime=1` (web-publish.md §2),
//! and this is the native counterpart rather than a port: a page can measure
//! `requestAnimationFrame` from outside the wasm module, and a native run has
//! nothing outside itself to measure it from. The readings are therefore the
//! same readings, and two of them are *better* here — ticks per frame is read
//! off the simulation rather than modelled from the deltas, and the present
//! mode is the one the surface was actually configured with rather than a guess
//! from the cadence (frame-pacing.md §6).
//!
//! DELIBERATE: every line composed here is **printable ASCII**, where the web
//! overlay's equivalent lines use an em dash. The built-in font covers ASCII 32
//! to 126 and draws a visible fallback box for everything else (renderer.md §6),
//! so an em dash in a readout is a box on screen — legible enough to mislead and
//! wrong enough to read as a rendering fault. The last test in this file is what
//! keeps it true as lines are added.

use core::fmt::Write as _;

use jidousha_core::Seconds;
use jidousha_render_core::Presentation;

/// The environment variable that turns the overlay on.
///
/// **Off unless this is set to something other than `0` or `false`**, matching
/// the web overlay's `?frametime=1` exactly — including that bare
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
pub(crate) const SWITCH: &str = "JIDOUSHA_FRAMETIME";

/// How many frame deltas the readout describes.
///
/// Four seconds at sixty a second: long enough that one hitch does not define
/// the median, short enough that the panel describes what is happening now
/// rather than averaging the last minute away. The same window the web
/// overlay's rolling histogram uses.
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
const BAR: usize = 20;

/// Whether this run was asked for the overlay.
///
/// Read once at startup, like `?renderscale=` is (web/render_scale.rs): a
/// switch that changed under a running loop would be a resize and a
/// reconfiguration nobody asked for.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn requested() -> bool {
    match std::env::var(SWITCH) {
        Ok(value) => value != "0" && value != "false",
        Err(_) => false,
    }
}

/// Never on the web: the page owns this instrument there.
///
/// `?frametime=1` is page-side and measures `requestAnimationFrame` from
/// outside the wasm module (web-publish.md §2). A second overlay inside the
/// module would draw a panel over the one already on the page and measure the
/// same frames twice.
#[cfg(target_arch = "wasm32")]
pub(crate) fn requested() -> bool {
    false
}

/// What the overlay has seen, and what it says about it.
///
/// Constructed on every run and empty on almost all of them: an overlay nobody
/// asked for records nothing, formats nothing, and allocates nothing, so the
/// off-by-default path costs a branch per frame.
pub(crate) struct Overlay {
    on: bool,
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
}

impl Overlay {
    /// An overlay that is on, or one that is not.
    ///
    /// The flag is an argument rather than read here so that the switch is
    /// consulted in exactly one place — `Driver::new` — and so that every case
    /// below can be tested without setting an environment variable, which is
    /// process-global state two tests running in parallel would fight over.
    pub(crate) fn new(on: bool) -> Self {
        Self {
            on,
            deltas: std::collections::VecDeque::new(),
            ticks: [0; TICK_BUCKETS],
            frames: 0,
            since_repaint: Seconds(0.0),
            readout: String::new(),
        }
    }

    /// Whether anything should be drawn.
    pub(crate) fn is_on(&self) -> bool {
        self.on
    }

    /// Take one frame's reading: how long it was, and how many ticks it ran.
    ///
    /// `elapsed` is the clamped frame duration the accumulator was given, which
    /// is the honest number to report — it is what the loop actually spent on
    /// this frame's worth of simulation, and a frame past `MAX_FRAME` shows up
    /// as the ceiling here exactly as it does in the tick counts beside it
    /// (core.md §7).
    pub(crate) fn record(&mut self, elapsed: Seconds, ticks: u32, presentation: Presentation) {
        if !self.on {
            return;
        }
        self.deltas.push_back(elapsed.as_f32() * 1000.0);
        while self.deltas.len() > WINDOW {
            self.deltas.pop_front();
        }
        self.ticks[(ticks as usize).min(TICK_BUCKETS - 1)] += 1;
        self.frames += 1;

        self.since_repaint = Seconds(self.since_repaint.as_f32() + elapsed.as_f32());
        // The first frame repaints too, so a screenshot taken immediately says
        // what it knows rather than "measuring…".
        if self.since_repaint >= REPAINT_PERIOD || self.frames == 1 {
            self.since_repaint = Seconds(0.0);
            self.readout = self.compose(presentation);
        }
    }

    /// What to draw, as lines of text.
    pub(crate) fn readout(&self) -> &str {
        &self.readout
    }

    /// Build the panel's text.
    fn compose(&self, presentation: Presentation) -> String {
        let mut sorted: Vec<f32> = self.deltas.iter().copied().collect();
        sorted.sort_by(f32::total_cmp);
        let median = percentile(&sorted, 0.5);
        let mean = if sorted.is_empty() {
            0.0
        } else {
            sorted.iter().sum::<f32>() / sorted.len() as f32
        };
        let presented = if median > 0.0 { 1000.0 / median } else { 0.0 };
        let lowest = sorted.first().copied().unwrap_or(0.0);
        let highest = sorted.last().copied().unwrap_or(0.0);

        let mut text = String::new();
        // The switch is on the panel because a screenshot has to say how it was
        // produced — the commonest question about any diagnostic image is "how
        // do I get that".
        let _ = writeln!(text, "jidousha frame pacing: {SWITCH}=1");
        let _ = writeln!(
            text,
            "present   ~{presented:.1} fps - median {median:.2}ms, mean {mean:.2}ms"
        );
        let _ = writeln!(
            text,
            "spread    {lowest:.2}ms .. {highest:.2}ms over {} frames",
            sorted.len()
        );
        let _ = writeln!(text, "pacing    {}", pacing_line(presentation));
        let _ = writeln!(text, "ticks/fr  {}", self.tick_shares());
        let _ = write!(text, "frame deltas\n{}", histogram(&sorted));
        text
    }

    /// The ticks-per-frame line: the symptom, straight from the accumulator.
    ///
    /// Not modelled. The web overlay has to re-run the engine's accumulator over
    /// its own deltas because a page cannot see inside the wasm module; here the
    /// number comes back from `Simulation::advance`, so a disagreement between
    /// this line and the frame times is a real disagreement rather than a
    /// modelling artefact (frame-pacing.md §4).
    fn tick_shares(&self) -> String {
        let mut line = String::new();
        for (bucket, count) in self.ticks.iter().enumerate() {
            let share = (*count as f64 / self.frames.max(1) as f64 * 100.0).round();
            let name = if bucket == TICK_BUCKETS - 1 {
                format!("{bucket}+")
            } else {
                bucket.to_string()
            };
            let _ = write!(line, "{name}:{count} ({share:.0}%)  ");
        }
        line.trim_end().to_owned()
    }
}

/// What the pacing line says, which is the whole reason this overlay is on
/// native at all.
///
/// The reading a bug report needs: *is anything bounding this frame rate, and
/// what*. "vsync" means the display is, "capped" means the loop is because
/// nothing else would, and the number beside it is `FALLBACK_CAP_HZ` so the
/// panel is self-explaining rather than sending a reader to the source.
fn pacing_line(presentation: Presentation) -> String {
    if presentation.needs_a_cap() {
        format!(
            "{presentation} - no vsync on this surface, so the loop is capped at {:.0} fps",
            super::pacing::FALLBACK_CAP_HZ
        )
    } else if presentation == Presentation::Vsync {
        format!("{presentation} - the display sets the rate")
    } else {
        format!("{presentation} - nothing is being presented yet")
    }
}

/// The delta `fraction` of the way up a sorted window.
fn percentile(sorted: &[f32], fraction: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() as f32 * fraction) as usize).min(sorted.len() - 1);
    sorted[index]
}

/// A rolling histogram of frame deltas, one millisecond to a bucket.
///
/// One millisecond is the resolution that separates 8.33 from 16.67 and that
/// shows a coarse clock piling its deltas onto whole numbers — the same buckets
/// and the same coarse tail as the web overlay, so the two pictures can be
/// compared side by side (web-publish.md §2).
fn histogram(sorted: &[f32]) -> String {
    if sorted.is_empty() {
        return "  (nothing measured yet)".to_owned();
    }
    // Sorted input, so a bucket is a run: walk once and cut where the label
    // changes, which needs no map and keeps the rows in ascending order for
    // free.
    let mut rows: Vec<(String, usize)> = Vec::new();
    for delta in sorted {
        let key = bucket(*delta);
        match rows.last_mut() {
            Some((last, count)) if *last == key => *count += 1,
            _ => rows.push((key, 1)),
        }
    }
    let most = rows.iter().map(|(_, count)| *count).max().unwrap_or(1);
    let mut text = String::new();
    for (index, (key, count)) in rows.iter().enumerate() {
        let filled = ((count * BAR).div_ceil(most.max(1))).clamp(1, BAR);
        let share = (*count as f64 / sorted.len() as f64 * 100.0).round();
        if index > 0 {
            text.push('\n');
        }
        let _ = write!(
            text,
            "  {key:>8} {:BAR$} {count:>4} ({share:.0}%)",
            "#".repeat(filled)
        );
    }
    text
}

/// Which bucket a delta falls in, coarsening past the point where the exact
/// millisecond stops mattering.
fn bucket(ms: f32) -> String {
    if ms >= 50.0 {
        return "50ms+".to_owned();
    }
    if ms >= 34.0 {
        return "34-50ms".to_owned();
    }
    if ms >= 25.0 {
        return "25-34ms".to_owned();
    }
    let low = ms.floor() as i32;
    format!("{low}-{}ms", low + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An overlay that is on, whatever this machine's environment says.
    fn on() -> Overlay {
        Overlay::new(true)
    }

    #[test]
    fn an_overlay_nobody_asked_for_records_nothing_and_says_nothing() {
        // The off-by-default promise, from the inside: not merely "is not
        // drawn" but "has nothing to draw", so a switch flipped by accident
        // half way through a run cannot produce a panel full of history.
        let mut overlay = Overlay::new(false);
        for _ in 0..10 {
            overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        }
        assert!(overlay.readout().is_empty());
        assert!(!overlay.is_on());
        assert_eq!(overlay.frames, 0);
    }

    #[test]
    fn the_first_frame_already_says_something() {
        // A screenshot taken a moment after launch is the commonest artifact a
        // bug report carries. It must not read "measuring…".
        let mut overlay = on();
        overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        assert!(
            overlay.readout().contains("present"),
            "{}",
            overlay.readout()
        );
    }

    #[test]
    fn the_readout_states_the_frame_time_and_the_rate_it_implies() {
        let mut overlay = on();
        for _ in 0..120 {
            overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        }
        let readout = overlay.readout();
        assert!(readout.contains("16.67ms"), "the frame time: {readout}");
        assert!(readout.contains("~60.0 fps"), "and the rate: {readout}");
    }

    #[test]
    fn the_pacing_line_names_the_cap_when_the_surface_will_not_vsync() {
        // The one line this overlay exists on native to print. A reader has to
        // be able to tell "the display is pacing this" from "nothing was, so we
        // capped it" without reading any source.
        let capped = pacing_line(Presentation::Immediate);
        assert!(capped.contains("immediate"), "{capped}");
        assert!(capped.contains("capped at 60 fps"), "{capped}");

        let vsynced = pacing_line(Presentation::Vsync);
        assert!(vsynced.contains("vsync"), "{vsynced}");
        assert!(
            !vsynced.contains("capped"),
            "a vsynced run is not capped by the loop: {vsynced}"
        );
    }

    #[test]
    fn ticks_per_frame_come_from_the_simulation_rather_than_from_the_deltas() {
        // The reading the web overlay has to model and this one does not: a
        // frame that ran two ticks is reported as two whatever its duration
        // says, because the accumulator is what was asked.
        let mut overlay = on();
        overlay.record(Seconds(0.001), 0, Presentation::Vsync);
        overlay.record(Seconds(0.001), 2, Presentation::Vsync);
        overlay.record(Seconds(0.001), 9, Presentation::Vsync);
        // Composed here rather than read off `readout`, because three
        // millisecond-long frames are nowhere near a repaint and the panel is
        // still showing the first of them — which is the repaint rule working,
        // not a fault. What is under test is the line, not when it is rebuilt.
        let readout = overlay.compose(Presentation::Vsync);
        assert!(readout.contains("0:1"), "{readout}");
        assert!(readout.contains("2:1"), "{readout}");
        assert!(
            readout.contains("3+:1"),
            "nine ticks lands in 3+: {readout}"
        );
        assert!(readout.contains("1:0"), "{readout}");
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
    fn a_hitch_shows_up_in_the_spread_and_in_the_histogram_without_moving_the_median() {
        // What the instrument is for: one long frame among many short ones is
        // exactly the shape of the defect frame-pacing.md investigates, and a
        // panel that averaged it away would hide it.
        let mut overlay = on();
        for _ in 0..100 {
            overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        }
        overlay.record(Seconds(0.1), 6, Presentation::Vsync);
        // Past the repaint period, so the hitch is in the composed panel.
        for _ in 0..30 {
            overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        }
        let readout = overlay.readout();
        assert!(readout.contains(".. 100.00ms"), "the spread: {readout}");
        assert!(readout.contains("median 16.67ms"), "unmoved: {readout}");
        assert!(readout.contains("50ms+"), "and a bucket for it: {readout}");
    }

    #[test]
    fn every_histogram_bucket_is_drawn_with_at_least_one_character() {
        // A row with an empty bar reads as a row with no frames in it, which is
        // the opposite of what a rare long frame — the interesting one — means.
        let sorted = vec![16.7; 200]
            .into_iter()
            .chain([60.0])
            .collect::<Vec<f32>>();
        for line in histogram(&sorted).lines() {
            assert!(line.contains('#'), "an empty bar: {line}");
        }
    }

    #[test]
    fn the_histogram_is_ordered_from_the_quickest_frames_to_the_slowest() {
        let sorted = vec![8.4, 8.9, 16.7, 16.9, 30.0, 80.0];
        let drawn = histogram(&sorted);
        let labels: Vec<&str> = drawn
            .lines()
            .map(|line| line.trim().split(' ').next().unwrap_or(""))
            .collect();
        assert_eq!(labels, vec!["8-9ms", "16-17ms", "25-34ms", "50ms+"]);
    }

    #[test]
    fn every_line_the_panel_composes_is_drawable_in_the_built_in_font() {
        // The built-in font is printable ASCII and draws a fallback box for
        // anything else (renderer.md §6). An em dash — the web overlay uses
        // several — comes out as a box, which reads as a rendering fault rather
        // than as a punctuation choice.
        let mut overlay = on();
        for (index, ticks) in [1, 1, 2, 0, 1, 6].into_iter().enumerate() {
            overlay.record(
                Seconds(0.05 * (index as f32 + 1.0)),
                ticks,
                Presentation::Vsync,
            );
        }
        for presentation in [
            Presentation::Offscreen,
            Presentation::Vsync,
            Presentation::Mailbox,
            Presentation::Immediate,
        ] {
            let readout = overlay.compose(presentation);
            let stray: Vec<char> = readout
                .chars()
                .filter(|character| *character != '\n' && !(' '..='~').contains(character))
                .collect();
            assert!(
                stray.is_empty(),
                "{presentation}: {stray:?} would each draw a fallback box"
            );
        }
    }

    #[test]
    fn nothing_measured_yet_says_so_rather_than_dividing_by_zero() {
        assert!(histogram(&[]).contains("nothing measured"));
        assert_eq!(percentile(&[], 0.5), 0.0);
    }
}
