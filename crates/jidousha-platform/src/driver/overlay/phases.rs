//! Where a frame's milliseconds went, and what a window of them says.
//!
//! Key types: `Phase`, `Spans`, `Breakdown`.
//! Depends on: `jidousha-core` (`Seconds`). Must never depend on: `winit`, the
//! world, or a clock — the driver reads the one clock this engine has and hands
//! the readings here (clock.rs, ADR-0005).
//! INVARIANT: **draw-side and presentation-only.** Nothing here is read by a
//! tick, and a frame is measured whether or not anybody is looking; the switch
//! decides whether the driver takes the marks at all, so an overlay nobody
//! asked for costs the frame one branch (frame-pacing.md §7).
//! INVARIANT: the fifth bucket is **derived, never measured**. `sleep` is what
//! the frame's own duration has left over after the four measured spans, so the
//! five always add up to the frame and a mark this module forgot to take shows
//! up as sleep rather than as time that vanished.

use core::fmt::Write as _;

use jidousha_core::Seconds;

/// The four spans the driver measures inside a frame.
///
/// Deliberately the four the *driver* can see. A frame's time is spent in more
/// places than this — the display's wait is inside [`Present`](Phase::Present)
/// rather than beside it — and where a boundary falls where the driver cannot
/// reach, the bucket says so in its own documentation rather than the panel
/// pretending to a split it did not make (frame-pacing.md §7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    /// The frame's ticks: `Simulation::advance` and nothing else.
    ///
    /// This is the simulation's whole cost for the frame, however many ticks
    /// ran — including none, which is what a machine drawing faster than it
    /// ticks does constantly (core.md §7).
    Sim,
    /// The Draw phase: the game's own submissions, and the camera and face
    /// bookkeeping either side of it.
    Draw,
    /// Turning submissions into a `FramePlan`: the asset commit, the texture
    /// and text-atlas uploads, the overlay's own quads, and `plan_frame`.
    ///
    /// Everything above the backend seam that a frame costs after the game has
    /// said what it wants drawn.
    Encode,
    /// `RenderBackend::render` — and therefore the **present-wait**.
    ///
    /// It is one span rather than two because the seam is where the driver's
    /// reach stops: the backend encodes a command buffer, submits it, and
    /// blocks in the surface acquire, and only the backend is inside that. On
    /// any surface that waits for the display the wait is nearly all of it —
    /// a 16.7ms `present` beside a 0.3ms `encode` is a loop the display is
    /// pacing — which is what makes this the CPU-bound / GPU-bound
    /// discriminator the panel's doc section describes.
    Present,
}

/// How many spans are measured, and the index each one is kept at.
const MEASURED: usize = 4;

/// How many buckets the breakdown reports: the measured four, and `sleep`.
const BUCKETS: usize = MEASURED + 1;

/// Where `sleep` sits in a bucket row.
const SLEEP: usize = MEASURED;

/// What each bucket is called on the panel, in bucket order.
///
/// Padded to a common width here rather than at every call site, so the rows
/// line up without the composer counting characters.
const NAMES: [&str; BUCKETS] = ["sim    ", "draw   ", "encode ", "present", "sleep  "];

/// How wide the breakdown's bars are, in characters.
///
/// The same twenty the frame-delta histogram uses, so the two blocks of the
/// panel read as one instrument rather than two (`mod.rs`'s `BAR`).
const BAR: usize = 20;

impl Phase {
    /// Where this phase's milliseconds are kept.
    fn index(self) -> usize {
        match self {
            Phase::Sim => 0,
            Phase::Draw => 1,
            Phase::Encode => 2,
            Phase::Present => 3,
        }
    }
}

/// One frame's measured spans, in milliseconds, as the driver accumulates them.
///
/// Built up by [`spent`](Spans::spent) as the frame runs and handed to
/// [`Breakdown::record`] when the *next* frame starts — which is the only
/// moment the frame's own total duration is known, because that total is
/// exactly the elapsed time the next frame is given (frame-pacing.md §7).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Spans {
    milliseconds: [f32; MEASURED],
    /// How far into the frame the last mark was taken, in milliseconds.
    ///
    /// Kept so a span is a difference rather than something the caller has to
    /// subtract: the driver reads one running clock (`FrameClock::since_frame`)
    /// and says which phase just ended.
    mark: f32,
}

impl Spans {
    /// Nothing measured yet — a frame that is just starting.
    pub(crate) fn new() -> Self {
        Self {
            milliseconds: [0.0; MEASURED],
            mark: 0.0,
        }
    }

    /// Close `phase` at `since_frame_start` and open the next one.
    ///
    /// `since_frame_start` is [`FrameClock::since_frame`](crate::FrameClock::since_frame),
    /// which is unclamped and does not move the frame's mark — so calling this
    /// four times in a frame splits that frame rather than spending it.
    ///
    /// A reading that went backwards contributes zero rather than a negative
    /// span: `Instant` is monotonic, so this cannot happen, and a bucket that
    /// could go negative would make the derived `sleep` larger than the frame.
    pub(crate) fn spent(&mut self, phase: Phase, since_frame_start: Seconds) {
        let now = since_frame_start.as_f32() * 1000.0;
        self.milliseconds[phase.index()] += (now - self.mark).max(0.0);
        self.mark = now;
    }
}

/// A rolling window of frame breakdowns.
///
/// The same window the frame-delta histogram uses and for the same reason: long
/// enough that one hitch does not define the median, short enough to describe
/// what is happening now (`mod.rs`'s `WINDOW`).
pub(crate) struct Breakdown {
    rows: std::collections::VecDeque<[f32; BUCKETS]>,
    /// Each row's frame total, so `busy` is a share of the frame it came from
    /// rather than of a median from somewhere else in the window.
    totals: std::collections::VecDeque<f32>,
    window: usize,
}

impl Breakdown {
    /// An empty window `window` frames long.
    pub(crate) fn new(window: usize) -> Self {
        Self {
            rows: std::collections::VecDeque::new(),
            totals: std::collections::VecDeque::new(),
            window,
        }
    }

    /// Whether anything has been recorded.
    ///
    /// Only a test asks: the panel composes a block of zeroes on an empty
    /// window rather than a special case, which is what makes a screenshot
    /// taken on the first frame say something.
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Take one finished frame: how long it was, and where that went.
    ///
    /// `total` is the frame's whole duration — the elapsed time the *next*
    /// frame was given, which is measured from this frame's start and is
    /// therefore the only number the four spans can be subtracted from
    /// honestly.
    ///
    /// A frame whose measured spans overrun its total contributes zero sleep
    /// rather than negative sleep. That happens where the total was clamped:
    /// `FrameClock::frame` caps a frame at `MAX_FRAME`, so a genuine
    /// two-second stall reports a quarter of a second here and the spans
    /// inside it were never capped (core.md §7).
    pub(crate) fn record(&mut self, total: Seconds, spans: Spans) {
        let total = total.as_f32() * 1000.0;
        let mut row = [0.0_f32; BUCKETS];
        row[..MEASURED].copy_from_slice(&spans.milliseconds);
        let measured: f32 = spans.milliseconds.iter().sum();
        row[SLEEP] = (total - measured).max(0.0);
        self.rows.push_back(row);
        self.totals.push_back(total);
        while self.rows.len() > self.window {
            self.rows.pop_front();
            self.totals.pop_front();
        }
    }

    /// The breakdown block, in the panel's histogram style.
    ///
    /// One row per bucket: a bar showing the median's share of the median
    /// frame, then the median, the 95th percentile and the worst frame in the
    /// window. Three numbers because one of them is the question being asked
    /// and the other two are the ones that catch what a median hides — a phase
    /// that is fine every frame but one is a phase that hitches, and a median
    /// alone cannot say so.
    ///
    /// Every column is sorted **once**, in [`sorted`](Breakdown::sorted), and
    /// then read by index. Sorting per number instead would be eighteen sorts
    /// of the window to fill a block of fifteen — which is exactly the shape of
    /// instrument that perturbs what it measures, and it was measurable before
    /// this was one pass (frame-pacing.md §7).
    pub(crate) fn block(&self) -> String {
        let sorted = self.sorted();
        let frame = at(&sorted.totals, 0.5);
        let mut text = String::from("frame breakdown  ms: median  p95  max\n");
        for (bucket, column) in sorted.buckets.iter().enumerate() {
            let median = at(column, 0.5);
            let p95 = at(column, 0.95);
            let worst = at(column, 1.0);
            let filled = if frame > 0.0 {
                ((median / frame) * BAR as f32)
                    .round()
                    .clamp(0.0, BAR as f32) as usize
            } else {
                0
            };
            let _ = writeln!(
                text,
                "  {} {:BAR$} {median:>7.2} {p95:>7.2} {worst:>7.2}",
                NAMES[bucket],
                "#".repeat(filled)
            );
        }
        let _ = write!(text, "  busy    {}", busy_line(&sorted));
        text
    }

    /// Every column of the window, sorted, in one pass over the rows.
    ///
    /// Sorted per compose rather than kept sorted: the panel is rebuilt four
    /// times a second (`mod.rs`'s `REPAINT_PERIOD`), and a window kept in sorted
    /// order would pay on every one of the sixty frames a second that push into
    /// it instead.
    fn sorted(&self) -> Sorted {
        let mut sorted = Sorted::default();
        for row in &self.rows {
            let total: f32 = row.iter().sum();
            for (bucket, column) in sorted.buckets.iter_mut().enumerate() {
                column.push(row[bucket]);
            }
            sorted.totals.push(total);
            sorted.waits.push(row[Phase::Present.index()] + row[SLEEP]);
            // Per frame and then taken as a median, rather than as a ratio of
            // two medians: those are not the same number, and the one that
            // answers "what does a typical frame look like" is this one.
            sorted.busy.push(if total > 0.0 {
                (total - row[Phase::Present.index()] - row[SLEEP]) / total * 100.0
            } else {
                0.0
            });
        }
        for column in &mut sorted.buckets {
            column.sort_by(f32::total_cmp);
        }
        sorted.totals.sort_by(f32::total_cmp);
        sorted.waits.sort_by(f32::total_cmp);
        sorted.busy.sort_by(f32::total_cmp);
        sorted
    }
}

/// The window's columns, each sorted, ready to be read by percentile.
#[derive(Default)]
struct Sorted {
    buckets: [Vec<f32>; BUCKETS],
    totals: Vec<f32>,
    waits: Vec<f32>,
    busy: Vec<f32>,
}

/// The busy line: what share of the frame the loop was doing work.
///
/// `busy% = (frame - waits) / frame`, where the waits are `present` and
/// `sleep` — the two buckets in which this thread is not running. A high busy
/// share with a low GPU time is a CPU-bound loop; a low busy share is a loop
/// something else is pacing, and which something is the difference between
/// `present` and `sleep` (frame-pacing.md §7).
///
/// **This thread's work, not the process's.** A backend whose driver rasterizes
/// on worker threads — lavapipe, and every software renderer — waits here while
/// those threads run, so `busy` reads low beside a process CPU share well over
/// 100%. That disagreement is a reading rather than a fault, and the doc
/// section says so.
fn busy_line(sorted: &Sorted) -> String {
    format!(
        "{:.0}% of a {:.2}ms frame - {:.2}ms of it waiting",
        at(&sorted.busy, 0.5),
        at(&sorted.totals, 0.5),
        at(&sorted.waits, 0.5)
    )
}

/// The value `fraction` of the way up a sorted column.
fn at(sorted: &[f32], fraction: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() as f32 * fraction) as usize).min(sorted.len() - 1);
    sorted[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame whose four spans are the given milliseconds.
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
    fn a_span_is_the_gap_since_the_last_mark_rather_than_the_reading_itself() {
        // The mistake this rules out: recording the clock reading as the span,
        // which makes every phase look like the sum of everything before it and
        // the frame look four times as long as it was.
        let measured = spans(2.0, 1.0, 0.5, 12.0).milliseconds;
        assert!((measured[0] - 2.0).abs() < 1e-3, "{measured:?}");
        assert!((measured[1] - 1.0).abs() < 1e-3, "{measured:?}");
        assert!((measured[2] - 0.5).abs() < 1e-3, "{measured:?}");
        assert!((measured[3] - 12.0).abs() < 1e-3, "{measured:?}");
    }

    #[test]
    fn sleep_is_whatever_the_frame_had_left_over() {
        // The derived bucket, and the property that makes the five add up: a
        // 16.67ms frame with 4ms of measured work slept the rest, whether the
        // pacer asked it to or winit simply had nothing to do.
        let mut breakdown = Breakdown::new(240);
        breakdown.record(Seconds(0.016_67), spans(2.0, 1.0, 0.5, 0.5));
        let block = breakdown.block();
        assert!(block.contains("sleep"), "{block}");
        // 16.67 - 4.0 = 12.67
        assert!(block.contains("12.67"), "{block}");
    }

    /// The block's bucket rows, without its header or its busy line.
    fn block_rows(breakdown: &Breakdown) -> Vec<String> {
        breakdown
            .block()
            .lines()
            .filter(|line| {
                NAMES
                    .iter()
                    .any(|name| line.trim_start().starts_with(name.trim_end()))
            })
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn a_frame_whose_spans_overran_its_total_reports_no_sleep_rather_than_negative_sleep() {
        // `FrameClock::frame` clamps at MAX_FRAME, so a genuine long stall
        // reports a quarter second here while the spans inside it were never
        // clamped. A negative bucket would make the bars meaningless and the
        // busy share exceed 100%.
        let mut breakdown = Breakdown::new(240);
        breakdown.record(Seconds(0.010), spans(20.0, 0.0, 0.0, 0.0));
        // Every number in the block is unsigned: a bucket that could go
        // negative would make the bars meaningless and the busy share exceed
        // 100%. Checked column by column rather than by hunting for a `-`,
        // because the busy line below carries one as punctuation.
        for line in block_rows(&breakdown) {
            for column in line.split_whitespace().skip(1) {
                if let Ok(value) = column.parse::<f32>() {
                    assert!(value >= 0.0, "a negative bucket: {line}");
                }
            }
        }
    }

    #[test]
    fn a_loop_the_display_is_pacing_reads_as_mostly_waiting() {
        // The reading the whole section exists for. Half a millisecond of work
        // against sixteen of present is a vsynced loop with headroom, and the
        // busy line has to say so in one number a screenshot carries.
        let mut breakdown = Breakdown::new(240);
        for _ in 0..120 {
            breakdown.record(Seconds(0.016_67), spans(0.3, 0.1, 0.1, 16.17));
        }
        let line = busy_line(&breakdown.sorted());
        assert!(line.starts_with("3% of a 16.67ms frame"), "{line}");
        assert!(line.contains("16.17ms of it waiting"), "{line}");
    }

    #[test]
    fn a_cpu_bound_loop_reads_as_mostly_busy() {
        // The other end of the same discriminator: the work fills the frame and
        // nothing is waiting, which is what a CPU-bound run looks like before
        // anyone has looked at a GPU number.
        let mut breakdown = Breakdown::new(240);
        for _ in 0..120 {
            breakdown.record(Seconds(0.020), spans(14.0, 3.0, 2.0, 1.0));
        }
        let line = busy_line(&breakdown.sorted());
        assert!(line.starts_with("95% of a 20.00ms frame"), "{line}");
    }

    #[test]
    fn one_hitch_moves_the_worst_column_and_leaves_the_median_alone() {
        // Why three numbers rather than one: a phase that is fine on every
        // frame but one is a phase that hitches, and it is invisible in a
        // median. The p95 and max columns are what make it visible.
        let mut breakdown = Breakdown::new(240);
        for _ in 0..100 {
            breakdown.record(Seconds(0.016_67), spans(1.0, 0.5, 0.2, 14.0));
        }
        breakdown.record(Seconds(0.100), spans(40.0, 0.5, 0.2, 14.0));
        let block = breakdown.block();
        let sim = block
            .lines()
            .find(|line| line.trim_start().starts_with("sim"))
            .unwrap_or_default();
        assert!(sim.contains("1.00"), "the median is unmoved: {sim}");
        assert!(sim.contains("40.00"), "and the hitch is in max: {sim}");
    }

    #[test]
    fn the_window_stops_growing() {
        let mut breakdown = Breakdown::new(8);
        for _ in 0..50 {
            breakdown.record(Seconds(0.016_67), spans(1.0, 0.5, 0.2, 14.0));
        }
        assert_eq!(breakdown.rows.len(), 8);
        assert_eq!(breakdown.totals.len(), 8);
    }

    #[test]
    fn nothing_measured_yet_composes_a_block_of_zeroes_rather_than_dividing_by_zero() {
        let breakdown = Breakdown::new(240);
        assert!(breakdown.is_empty());
        let block = breakdown.block();
        assert!(block.contains("0.00"), "{block}");
        assert!(block.contains("busy"), "{block}");
    }

    #[test]
    fn every_bucket_has_a_name_and_they_are_all_the_same_width() {
        // The rows are read as a column of numbers, and a label a character
        // short shifts a whole row out of alignment.
        for name in NAMES {
            assert_eq!(name.len(), NAMES[0].len(), "{name} is a different width");
        }
    }
}
