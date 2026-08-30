//! What the panel says: every line of it, and the shape each reading takes.
//!
//! Key types: none — this is [`Overlay`]'s text half, split from `mod.rs`
//! because together they are twice the length a file should be
//! (agent-practices §5.7). `mod.rs` owns the switch and what has been measured;
//! this owns what that is written as.
//! Depends on: `jidousha-render-core` (`Presentation`), and the sibling halves
//! for their own blocks. Must never depend on: `winit`, the world, or a clock.
//! INVARIANT: **printable ASCII only.** The built-in font covers ASCII 32 to
//! 126 and draws a visible fallback box for everything else (renderer.md §6),
//! so an em dash in a readout is a box on screen — legible enough to mislead
//! and wrong enough to read as a rendering fault. The last test in this file is
//! what keeps it true as lines are added.
//! INVARIANT: **the levels are cumulative.** Level 2 is level 1 plus sections,
//! never level 1 rearranged, so a reader holding two screenshots side by side
//! finds the same four lines in the same order (frame-pacing.md §7).

use core::fmt::Write as _;

use jidousha_render_core::Presentation;

use super::{BAR, Level, Overlay, SNAPSHOT_KEY, TICK_BUCKETS, memory};

impl Overlay {
    /// Build the panel's text.
    pub(super) fn compose(&self, presentation: Presentation) -> String {
        let mut text = String::new();
        let _ = writeln!(text, "{} {}", self.title(), self.level.switch());
        self.pacing_block(&mut text, presentation);
        if self.level == Level::Perf {
            self.perf_block(&mut text);
        }
        // The panel is drawn as lines and the last one carries no newline, so
        // that an empty final line never opens a gap in the backdrop.
        while text.ends_with('\n') {
            text.pop();
        }
        text
    }

    /// What the panel calls itself.
    fn title(&self) -> &'static str {
        match self.level {
            Level::Perf => "jidousha performance:",
            Level::Off | Level::Pacing => "jidousha frame pacing:",
        }
    }

    /// The pacing readings — the panel exactly as level 1 has always drawn it.
    ///
    /// Absent on the web, where the page's own `?frametime=1` panel measures
    /// these from outside the module and measures them better: it sees the
    /// frames the browser actually presented, and this side sees only the ones
    /// the module was called for (web-publish.md §2).
    #[cfg(not(target_arch = "wasm32"))]
    fn pacing_block(&self, text: &mut String, presentation: Presentation) {
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
        let _ = writeln!(text, "frame deltas\n{}", histogram(&sorted));
    }

    /// On the web the page owns these readings, and says so in one line.
    #[cfg(target_arch = "wasm32")]
    fn pacing_block(&self, text: &mut String, _presentation: Presentation) {
        let _ = writeln!(
            text,
            "pacing    on the page's own panel, which measures the frames the \
             browser presented"
        );
    }

    /// The level-2 sections: where the frame went, and what is being held.
    fn perf_block(&self, text: &mut String) {
        let _ = writeln!(text, "{}", self.breakdown.block());
        let _ = writeln!(text, "cpu       {}", self.cpu_line());
        let _ = writeln!(text, "gpu       {}", self.gpu_line());
        let _ = writeln!(text, "memory    {}", self.memory_line());
        let _ = writeln!(
            text,
            "  renderer {} textures, {} buffers",
            memory::megabytes(self.engine.backend.texture_bytes),
            memory::megabytes(self.engine.backend.buffer_bytes)
        );
        let _ = writeln!(
            text,
            "  world    {} entities, {} components, {} quads drawn",
            self.engine.entities, self.engine.components, self.engine.quads
        );
        let _ = writeln!(text, "snapshot  {}", self.snapshot_line());
    }

    /// What the process is costing the machine.
    ///
    /// A share of **one core**, so a number over 100 means more than a core's
    /// worth — which is the reading frame-pacing.md §6.5 turned on, and which
    /// dividing by the machine's core count would have hidden.
    fn cpu_line(&self) -> String {
        match self.process.cpu_share() {
            Some(share) => format!("process {share:.0}% of one core"),
            #[cfg(target_arch = "wasm32")]
            None => {
                "process n/a - a page has no process counters; read busy above instead".to_owned()
            }
            #[cfg(not(target_arch = "wasm32"))]
            None => "process n/a - no reading yet, or none this platform offers".to_owned(),
        }
    }

    /// How long the GPU spent on the frame, where the device will say.
    ///
    /// Milliseconds, never a percentage: a GPU *utilization* figure needs
    /// vendor libraries this engine does not have and will not take on, and a
    /// percentage invented from a frame time would be a number that looked like
    /// an answer without being one (renderer.md §12a).
    fn gpu_line(&self) -> String {
        if self.gpu.is_empty() {
            return "n/a - this device offers no timestamp queries".to_owned();
        }
        let mut sorted: Vec<f32> = self.gpu.iter().copied().collect();
        sorted.sort_by(f32::total_cmp);
        format!(
            "median {:.2}ms, p95 {:.2}ms over {} frames",
            percentile(&sorted, 0.5),
            percentile(&sorted, 0.95),
            sorted.len()
        )
    }

    /// The memory tier the operating system or the runtime can answer.
    fn memory_line(&self) -> String {
        if let Some(rss) = self.process.rss_bytes() {
            return format!("rss {}", memory::megabytes(rss));
        }
        if let Some(linear) = memory::linear_bytes() {
            return format!("wasm linear {}", memory::megabytes(linear));
        }
        "n/a - no process reading on this platform".to_owned()
    }

    /// What the snapshot key does, and what the last press did.
    fn snapshot_line(&self) -> String {
        if let Some(wrote) = &self.wrote {
            return wrote.clone();
        }
        if cfg!(target_arch = "wasm32") {
            return "not on the web - a page has nowhere to write; screenshot this".to_owned();
        }
        format!("press {SNAPSHOT_KEY:?} to write this panel under target/")
    }

    /// The ticks-per-frame line: the symptom, straight from the accumulator.
    ///
    /// Not modelled. The web overlay has to re-run the engine's accumulator over
    /// its own deltas because a page cannot see inside the wasm module; here the
    /// number comes back from `Simulation::advance`, so a disagreement between
    /// this line and the frame times is a real disagreement rather than a
    /// modelling artefact (frame-pacing.md §4).
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
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
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn pacing_line(presentation: Presentation) -> String {
    if presentation.needs_a_cap() {
        format!(
            "{presentation} - no vsync on this surface, so the loop is capped at {:.0} fps",
            crate::driver::pacing::FALLBACK_CAP_HZ
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
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
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
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
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
    use crate::driver::overlay::{Engine, Phase, Spans};
    use jidousha_core::Seconds;
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
    fn the_pacing_panel_carries_nothing_the_performance_panel_added() {
        // The level-1 promise, and the one this branch could most easily break:
        // a run at level 1 gets the panel frame-pacing.md §6 documents and not
        // one line more.
        let mut overlay = on();
        for _ in 0..120 {
            overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        }
        let readout = overlay.readout();
        for added in ["frame breakdown", "cpu ", "gpu ", "memory", "snapshot"] {
            assert!(
                !readout.contains(added),
                "level 1 shows {added:?}: {readout}"
            );
        }
        assert!(readout.starts_with("jidousha frame pacing: JIDOUSHA_FRAMETIME=1"));
    }

    #[test]
    fn the_performance_panel_names_its_own_level_on_its_first_line() {
        // A screenshot has to say how it was produced, and a level-2 image that
        // said `=1` would send the next reader looking for sections that switch
        // does not draw.
        let mut overlay = perf();
        overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        assert!(
            overlay.readout().starts_with("jidousha performance: "),
            "{}",
            overlay.readout()
        );
        assert!(
            overlay.readout().contains("JIDOUSHA_FRAMETIME=2"),
            "{}",
            overlay.readout()
        );
    }

    #[test]
    fn the_performance_panel_keeps_every_pacing_reading_and_adds_to_them() {
        // Cumulative levels: level 2 is level 1 plus sections, never level 1
        // rearranged. A reader comparing a level-1 screenshot with a level-2
        // one has to find the same four lines in the same order.
        let mut overlay = perf();
        for _ in 0..120 {
            overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        }
        let readout = overlay.readout();
        for kept in [
            "present   ",
            "spread    ",
            "pacing    ",
            "ticks/fr  ",
            "frame deltas",
        ] {
            assert!(
                readout.contains(kept),
                "level 2 dropped {kept:?}: {readout}"
            );
        }
        for added in [
            "frame breakdown",
            "cpu ",
            "gpu ",
            "memory    ",
            "snapshot  ",
        ] {
            assert!(
                readout.contains(added),
                "level 2 lacks {added:?}: {readout}"
            );
        }
    }

    #[test]
    fn a_device_with_no_timestamps_says_so_rather_than_reporting_zero() {
        // The `gpu n/a` path, which is every WebGL2 build and plenty of native
        // drivers. A zero would read as a GPU doing the frame in no time at
        // all, on exactly the machines where there is no reading.
        let mut overlay = perf();
        overlay.observe(Engine {
            backend: BackendStats::default(),
            ..Engine::default()
        });
        overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        assert!(
            overlay.readout().contains("gpu       n/a"),
            "{}",
            overlay.readout()
        );
    }

    #[test]
    fn a_device_that_offers_timestamps_reports_milliseconds_rather_than_a_percentage() {
        // Milliseconds, deliberately: a utilization percentage needs vendor
        // libraries this engine does not have, and one invented from a frame
        // time would look like an answer without being one (renderer.md §12a).
        let mut overlay = perf();
        for _ in 0..10 {
            overlay.observe(Engine {
                backend: BackendStats {
                    gpu_frame: Some(Seconds(0.0021)),
                    ..BackendStats::default()
                },
                ..Engine::default()
            });
            overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        }
        let readout = overlay.readout();
        assert!(readout.contains("gpu       median 2.10ms"), "{readout}");
        assert!(!readout.contains("gpu       n/a"), "{readout}");
    }

    #[test]
    fn the_accounting_reports_what_the_engine_is_holding_rather_than_what_the_os_is() {
        // The actionable tier. A resident set size that climbs is a fact with
        // no address in it; these counters have addresses, which is why they
        // are on the panel beside it rather than instead of it.
        let mut overlay = perf();
        overlay.observe(Engine {
            backend: BackendStats {
                texture_bytes: 12 * 1024 * 1024,
                buffer_bytes: 512 * 1024,
                gpu_frame: None,
            },
            entities: 412,
            components: 1236,
            quads: 318,
        });
        overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        let readout = overlay.readout();
        assert!(readout.contains("12.0MB textures"), "{readout}");
        assert!(readout.contains("0.5MB buffers"), "{readout}");
        assert!(readout.contains("412 entities"), "{readout}");
        assert!(readout.contains("1236 components"), "{readout}");
        assert!(readout.contains("318 quads"), "{readout}");
    }

    #[test]
    fn the_frame_breakdown_says_what_a_vsynced_loop_looks_like() {
        // The anchor section, end to end through the overlay: mostly waiting in
        // present, with the work that is left named phase by phase.
        let mut overlay = perf();
        for _ in 0..120 {
            overlay.close_frame(Seconds(1.0 / 60.0), spans(0.3, 0.1, 0.1, 16.17));
            overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        }
        let readout = overlay.readout();
        assert!(readout.contains("frame breakdown"), "{readout}");
        assert!(readout.contains("busy    3% of a"), "{readout}");
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
        // anything else (renderer.md §6). An em dash — the web panel uses
        // several — comes out as a box, which reads as a rendering fault rather
        // than as a punctuation choice. Every level, because a section added at
        // level 2 is exactly where a stray character would go unnoticed.
        for level in [Level::Pacing, Level::Perf] {
            let mut overlay = Overlay::new(level);
            for (index, ticks) in [1, 1, 2, 0, 1, 6].into_iter().enumerate() {
                overlay.close_frame(Seconds(0.05), spans(1.0, 0.5, 0.2, 40.0));
                overlay.observe(Engine {
                    backend: BackendStats {
                        texture_bytes: 1024 * 1024,
                        buffer_bytes: 4096,
                        gpu_frame: Some(Seconds(0.002)),
                    },
                    entities: 12,
                    components: 30,
                    quads: 44,
                });
                overlay.record(
                    Seconds(0.05 * (index as f32 + 1.0)),
                    ticks,
                    Presentation::Vsync,
                );
            }
            overlay.wrote = Some("wrote target/jidousha-perf-1756000000-000.txt".to_owned());
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
                    "{level:?} {presentation}: {stray:?} would each draw a fallback box"
                );
            }
        }
    }

    #[test]
    fn the_panel_never_ends_in_a_blank_line() {
        // The backdrop is sized from the text's extents, so a trailing newline
        // is a strip of dark ground under nothing — which reads as a section
        // that failed to render.
        let mut overlay = perf();
        overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        assert!(!overlay.readout().ends_with('\n'), "{}", overlay.readout());
    }

    #[test]
    fn nothing_measured_yet_says_so_rather_than_dividing_by_zero() {
        assert!(histogram(&[]).contains("nothing measured"));
        assert_eq!(percentile(&[], 0.5), 0.0);
        // And the whole level-2 panel composes on a run that has seen nothing.
        assert!(perf().compose(Presentation::Offscreen).contains("busy"));
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
    fn the_snapshot_key_is_named_on_the_panel_before_anybody_presses_it() {
        // A key nobody is told about is a key nobody presses. The panel is the
        // only place it is documented at run time, and a screenshot carries it.
        let mut overlay = perf();
        overlay.record(Seconds(1.0 / 60.0), 1, Presentation::Vsync);
        assert!(overlay.readout().contains("F9"), "{}", overlay.readout());
        assert!(
            overlay.readout().contains("target/"),
            "{}",
            overlay.readout()
        );
    }
}
