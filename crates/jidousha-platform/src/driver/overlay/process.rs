//! What the operating system says this process is costing, sampled once a
//! second.
//!
//! Key types: `Process`, `Counters`.
//! Depends on: the standard library and two per-platform system calls. Must
//! never depend on: a crate. `sysinfo` and its relatives pull a tree of
//! platform back-ends to answer two questions this file answers in twenty
//! lines each, and the dependency budget is a standing rule rather than a
//! preference (agent-practices §5.8).
//! INVARIANT: **once a second, cached in between.** Reading `/proc` sixty times
//! a second would put a file open, a read and a parse inside every frame of
//! every run that had the panel up — an instrument that perturbed exactly the
//! measurement it exists to take. The sample period is counted out of frame
//! deltas the driver already has, so no clock is read here either
//! (frame-pacing.md §7).
//! INVARIANT: every reading is `Option`. A kernel that will not answer, a
//! platform with no implementation here, and a web build all produce `None`,
//! and the panel prints `n/a` rather than a zero that reads like a measurement.

use jidousha_core::Seconds;

/// How often the operating system is asked, in seconds of frame time.
///
/// One second: the rate a person can read a changing percentage at, and slow
/// enough that the cost of asking is invisible beside a frame. It is measured
/// out of the frame deltas the loop already produced rather than off a clock,
/// which is the same trick the panel's repaint period uses (`mod.rs`).
const SAMPLE_PERIOD: Seconds = Seconds(1.0);

/// One reading of the process's own counters.
///
/// Cumulative CPU rather than a rate, because a rate is a difference between
/// two of these and the difference is this module's job rather than the
/// kernel's.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Counters {
    /// Total CPU seconds this process has used, user and system together.
    pub(crate) cpu_seconds: Option<f32>,
    /// Resident set size, in bytes — the memory the process actually occupies.
    pub(crate) rss_bytes: Option<u64>,
}

/// The process counters, sampled at 1Hz and held between samples.
pub(crate) struct Process {
    /// Frame time banked toward the next sample.
    since_sample: Seconds,
    /// How much frame time the current interval has covered, so the CPU share
    /// is a share of the wall time it was actually measured over.
    interval: Seconds,
    /// The previous sample's cumulative CPU, to difference against.
    previous: Option<f32>,
    /// The last computed share of one core, as a percentage.
    share: Option<f32>,
    /// The last resident set size read.
    rss_bytes: Option<u64>,
}

impl Process {
    /// A sampler that has taken no reading at all.
    ///
    /// What almost every run gets, because almost every run has the panel off:
    /// an overlay nobody asked for must not open a file, so the baseline below
    /// is taken by [`start`](Process::start) and only when the level that reads
    /// it is on.
    pub(crate) fn idle() -> Self {
        Self {
            since_sample: Seconds(0.0),
            interval: Seconds(0.0),
            previous: None,
            share: None,
            rss_bytes: None,
        }
    }

    /// A sampler that has taken its baseline.
    ///
    /// The baseline is taken at construction so the first *share* lands one
    /// second in rather than two. Until then there is one reading and no
    /// difference, and the panel says `n/a` — which is the truth: a percentage
    /// needs two samples and a run that has been up for half a second has one.
    pub(crate) fn start() -> Self {
        let counters = read();
        Self {
            since_sample: Seconds(0.0),
            interval: Seconds(0.0),
            previous: counters.cpu_seconds,
            share: None,
            rss_bytes: counters.rss_bytes,
        }
    }

    /// Advance by one frame, sampling if a second of frames has gone by.
    ///
    /// `elapsed` is the frame duration the accumulator was given — clamped, so
    /// a stalled frame contributes `MAX_FRAME` here as it does everywhere else
    /// (core.md §7). The consequence is that a run that stalls hard measures
    /// its CPU share over a slightly short second, which moves the number by
    /// less than the stall itself would move anything else on the panel.
    pub(crate) fn advance(&mut self, elapsed: Seconds) {
        self.since_sample = Seconds(self.since_sample.as_f32() + elapsed.as_f32());
        self.interval = Seconds(self.interval.as_f32() + elapsed.as_f32());
        if self.since_sample < SAMPLE_PERIOD {
            return;
        }
        let counters = read();
        if let (Some(now), Some(before)) = (counters.cpu_seconds, self.previous) {
            self.share = share(now - before, self.interval);
        }
        self.previous = counters.cpu_seconds;
        self.rss_bytes = counters.rss_bytes;
        self.since_sample = Seconds(0.0);
        self.interval = Seconds(0.0);
    }

    /// What share of one core this process has been using, as a percentage.
    ///
    /// Of *one* core, not of the machine: a number over 100 means more than one
    /// core's worth, which is a reading a game engine's author wants to see
    /// rather than have divided away by however many cores the machine has.
    pub(crate) fn cpu_share(&self) -> Option<f32> {
        self.share
    }

    /// The process's resident set size, in bytes.
    pub(crate) fn rss_bytes(&self) -> Option<u64> {
        self.rss_bytes
    }
}

/// What a CPU delta over a wall-clock interval is, as a percentage of one core.
///
/// Its own function because it is the only arithmetic here that can be wrong in
/// a way no platform would catch: every per-platform reader below is a parse,
/// and a parse that fails answers `None`.
fn share(cpu_seconds: f32, interval: Seconds) -> Option<f32> {
    let wall = interval.as_f32();
    if wall <= 0.0 || cpu_seconds < 0.0 {
        // A counter that went backwards, or no time at all to have used it in.
        // Neither is a number; the panel says `n/a` rather than showing zero
        // percent on a machine that is working hard.
        return None;
    }
    Some(cpu_seconds / wall * 100.0)
}

/// Read the process's counters out of `/proc`.
///
/// Two files, both plain text, both read whole: `/proc/self/stat` for the CPU
/// times and `/proc/self/status` for `VmRSS`. Hand-rolled because that is all
/// it is — and because the alternative is a crate that brings a back-end per
/// platform to do the same two reads.
#[cfg(target_os = "linux")]
fn read() -> Counters {
    Counters {
        cpu_seconds: std::fs::read_to_string("/proc/self/stat")
            .ok()
            .and_then(|text| cpu_seconds_of(&text)),
        rss_bytes: std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|text| rss_bytes_of(&text)),
    }
}

/// The kernel's unit for the CPU times in `/proc`.
///
/// **A hundred, always.** Not `CONFIG_HZ`, which varies by build: the times in
/// `/proc/[pid]/stat` are reported in `USER_HZ`, which Linux fixes at 100 in
/// `include/asm-generic/param.h` for every architecture this engine targets.
/// This is the number `sysconf(_SC_CLK_TCK)` returns, and calling it would mean
/// an `unsafe extern` block for a constant.
#[cfg(target_os = "linux")]
const USER_HZ: f32 = 100.0;

/// `utime + stime` out of a `/proc/self/stat` line, in seconds.
///
/// The parse has one trap in it and this is why it is a named function with
/// tests: the second field is the executable's name **in parentheses**, and it
/// may contain both spaces and parentheses. Splitting the line on whitespace
/// therefore reads the wrong fields for any program whose name has a space in
/// it. Cutting at the *last* `)` is the documented way, and everything after it
/// is fixed-position.
#[cfg(target_os = "linux")]
fn cpu_seconds_of(stat: &str) -> Option<f32> {
    let (_, after_comm) = stat.rsplit_once(')')?;
    let fields: Vec<&str> = after_comm.split_whitespace().collect();
    // The tail starts at field 3 (`state`), so `utime` (14) and `stime` (15)
    // are at offsets 11 and 12 (`man 5 proc`).
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some((utime + stime) as f32 / USER_HZ)
}

/// `VmRSS` out of `/proc/self/status`, in bytes.
///
/// The kernel writes it in kibibytes with a `kB` suffix; both are turned into
/// bytes here so the panel formats one unit everywhere.
#[cfg(target_os = "linux")]
fn rss_bytes_of(status: &str) -> Option<u64> {
    let line = status
        .lines()
        .find(|line| line.starts_with("VmRSS:"))?
        .strip_prefix("VmRSS:")?;
    let kibibytes: u64 = line.split_whitespace().next()?.parse().ok()?;
    Some(kibibytes * 1024)
}

/// Read the process's counters out of Windows' own accounting.
///
/// `GetProcessTimes` for the CPU, `K32GetProcessMemoryInfo` for the working
/// set — the two calls that answer what `/proc` answers on Linux. Declared here
/// rather than taken from a bindings crate, for the same reason the Linux half
/// parses two files by hand: this is two function signatures and one struct,
/// and a crate for it would be a tree of them.
#[cfg(windows)]
#[allow(unsafe_code)]
fn read() -> Counters {
    // SAFETY: every call below takes a pseudo-handle that needs no closing and
    // writes into a local this thread owns. The out-parameters are all `#[repr(C)]`
    // types laid out as the Windows headers declare them, and each call's return
    // value is checked before anything it wrote is read — a failed call leaves
    // the locals at the zeroes they were initialized with, and those are
    // discarded rather than reported.
    unsafe {
        let process = GetCurrentProcess();
        let mut created = FileTime::default();
        let mut exited = FileTime::default();
        let mut kernel = FileTime::default();
        let mut user = FileTime::default();
        let cpu_seconds = if GetProcessTimes(
            process,
            &raw mut created,
            &raw mut exited,
            &raw mut kernel,
            &raw mut user,
        ) == 0
        {
            None
        } else {
            Some(hundred_nanoseconds(kernel, user))
        };

        let mut counters = ProcessMemoryCounters::default();
        let size = u32::try_from(core::mem::size_of::<ProcessMemoryCounters>()).unwrap_or(0);
        counters.cb = size;
        let rss_bytes = if K32GetProcessMemoryInfo(process, &raw mut counters, size) == 0 {
            None
        } else {
            u64::try_from(counters.working_set_size).ok()
        };
        Counters {
            cpu_seconds,
            rss_bytes,
        }
    }
}

/// A Windows `FILETIME`: one 64-bit count of 100-nanosecond units, in halves.
#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct FileTime {
    low: u32,
    high: u32,
}

/// Windows' `PROCESS_MEMORY_COUNTERS`, field for field.
///
/// Only `working_set_size` is read; the rest are here because the struct is
/// passed by size and a short one would be rejected — or worse, filled in
/// against the wrong offsets.
#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ProcessMemoryCounters {
    cb: u32,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
}

#[cfg(windows)]
#[allow(unsafe_code)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut core::ffi::c_void;
    fn GetProcessTimes(
        process: *mut core::ffi::c_void,
        creation: *mut FileTime,
        exit: *mut FileTime,
        kernel: *mut FileTime,
        user: *mut FileTime,
    ) -> i32;
    fn K32GetProcessMemoryInfo(
        process: *mut core::ffi::c_void,
        counters: *mut ProcessMemoryCounters,
        size: u32,
    ) -> i32;
}

/// Kernel plus user time, as seconds.
///
/// A `FILETIME` counts 100-nanosecond units in two 32-bit halves, so the two
/// have to be rejoined before anything is added — adding the halves separately
/// is the classic way to get a number that is right for the first seven minutes
/// of a run and wrong afterwards.
#[cfg(windows)]
fn hundred_nanoseconds(kernel: FileTime, user: FileTime) -> f32 {
    let units = |time: FileTime| (u64::from(time.high) << 32) | u64::from(time.low);
    (units(kernel) + units(user)) as f32 / 1e7
}

/// Everywhere else, including the web: nothing to read.
///
/// The web deliberately: a page has no process counters, `performance.memory`
/// is a Chrome-only estimate of the whole tab, and the honest reading a wasm
/// build *can* take is its own linear memory, which `memory.rs` answers. The
/// panel prints `n/a` on the process line and says which platform it is on
/// (frame-pacing.md §7).
#[cfg(not(any(target_os = "linux", windows)))]
fn read() -> Counters {
    Counters {
        cpu_seconds: None,
        rss_bytes: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_of_cpu_over_a_second_of_wall_time_is_one_whole_core() {
        let Some(percent) = share(1.0, Seconds(1.0)) else {
            panic!("a well-formed pair has an answer");
        };
        assert!((percent - 100.0).abs() < 1e-3, "{percent}");
    }

    #[test]
    fn more_than_one_core_reads_as_more_than_a_hundred_percent() {
        // The reading frame-pacing.md §6.5 records — 150% of one core for a
        // static scene — is only sayable if this is not clamped. A number
        // divided by the machine's core count would have read as 19% on a
        // sixteen-core box and told nobody anything.
        let Some(percent) = share(1.5, Seconds(1.0)) else {
            panic!("a well-formed pair has an answer");
        };
        assert!((percent - 150.0).abs() < 1e-3, "{percent}");
    }

    #[test]
    fn an_interval_of_no_time_is_not_a_percentage() {
        assert_eq!(share(0.5, Seconds(0.0)), None);
        assert_eq!(share(0.5, Seconds(-1.0)), None);
    }

    #[test]
    fn a_counter_that_went_backwards_is_not_a_percentage() {
        // Cumulative CPU cannot decrease, so this is a reading that did not
        // mean what it looked like — and a negative percentage on the panel
        // would be worse than an honest `n/a`.
        assert_eq!(share(-0.2, Seconds(1.0)), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_stat_parse_survives_a_program_whose_name_has_spaces_and_brackets_in_it() {
        // The whole reason this is parsed rather than split: the comm field is
        // the executable's name in parentheses and may contain anything. A
        // whitespace split would read `flags` as `utime` here and report a
        // preposterous CPU share for any binary with a space in its name.
        let stat = "42 (my game (beta)) S 1 42 42 0 -1 4194304 900 0 0 0 \
                    250 130 0 0 20 0 9 0 8675309 123456 789";
        let Some(seconds) = cpu_seconds_of(stat) else {
            panic!("the fields after the last bracket are fixed-position");
        };
        // 250 + 130 ticks at 100Hz is 3.8 seconds.
        assert!((seconds - 3.8).abs() < 1e-3, "{seconds}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_truncated_stat_line_is_no_reading_rather_than_a_panic() {
        assert_eq!(cpu_seconds_of(""), None);
        assert_eq!(cpu_seconds_of("42 (game) S 1 2 3"), None);
        assert_eq!(cpu_seconds_of("no brackets here at all"), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn vm_rss_is_read_in_kibibytes_and_reported_in_bytes() {
        let status = "Name:\tgame\nVmPeak:\t  900000 kB\nVmRSS:\t  184320 kB\nThreads:\t4\n";
        assert_eq!(rss_bytes_of(status), Some(184_320 * 1024));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_status_file_without_vm_rss_is_no_reading() {
        // A kernel built without it, or a `/proc` that is not Linux's. Answering
        // zero would put "rss 0.0MB" on the panel of a process using a gigabyte.
        assert_eq!(rss_bytes_of("Name:\tgame\nThreads:\t4\n"), None);
        assert_eq!(rss_bytes_of("VmRSS:\tnot a number kB\n"), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn this_process_can_read_its_own_counters() {
        // The end-to-end reading, on the one platform the test suite is
        // guaranteed to be running on: whatever the numbers are, they exist and
        // they are not absurd. A parse that silently answered `None` on a real
        // `/proc` would leave the panel saying `n/a` on the very platform it
        // was written for, and every test above it would still pass.
        let counters = read();
        let Some(cpu) = counters.cpu_seconds else {
            panic!("/proc/self/stat is readable on Linux");
        };
        assert!((0.0..86_400.0).contains(&cpu), "{cpu} seconds of CPU");
        let Some(rss) = counters.rss_bytes else {
            panic!("/proc/self/status carries VmRSS on Linux");
        };
        assert!(
            rss > 1024,
            "a running process occupies more than a kibibyte"
        );
    }

    #[test]
    fn a_sampler_says_nothing_about_a_run_too_young_to_have_two_samples() {
        // A percentage needs a difference, and a run half a second old has one
        // reading. `n/a` is the honest answer and the panel prints it.
        let mut process = Process::start();
        process.advance(Seconds(0.016));
        assert_eq!(process.cpu_share(), None);
    }

    #[test]
    fn the_operating_system_is_asked_once_a_second_and_not_once_a_frame() {
        // The instrument-perturbation rule, as behaviour: sixty frames of
        // sixteen milliseconds is under a second, so nothing has been sampled
        // and the share is still absent. One more frame crosses it.
        let mut process = Process::start();
        // Forty frames of twenty milliseconds is 0.8s — under the period.
        for _ in 0..40 {
            process.advance(Seconds(0.020));
        }
        assert_eq!(process.cpu_share(), None, "under a second, no sample yet");
        for _ in 0..20 {
            process.advance(Seconds(0.020));
        }
        #[cfg(any(target_os = "linux", windows))]
        assert!(
            process.cpu_share().is_some(),
            "a second of frames buys a sample"
        );
    }
}
