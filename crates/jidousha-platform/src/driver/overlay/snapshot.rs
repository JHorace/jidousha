//! The one file the overlay ever writes, and only when a key asks for it.
//!
//! Key types: `Snapshots`; `KEY`.
//! Depends on: `std::fs` and `clock::stamp`. Must never depend on: anything a
//! tick can reach.
//! INVARIANT: **native only, one file per press, under `target/` and nowhere
//! else.** A diagnostic that could write anywhere would be a diagnostic nobody
//! could leave switched on. The web build has no equivalent and deliberately
//! so — a page cannot write to a path, and the browser's own "save the panel"
//! is a screenshot (frame-pacing.md §7).
//! INVARIANT: the key is **observed, never consumed.** The press reaches the
//! game exactly as it would with the overlay off, so a recorded transcript, a
//! replay and a `--verify` run are byte-identical whether or not a snapshot was
//! taken — which is the same promise the drawn panel makes.

/// Which key writes a snapshot while the panel is up.
///
/// **F9.** A function key, because the alphanumerics belong to the game and a
/// diagnostic that stole one would fire in the middle of somebody's playtest;
/// F9 rather than F1 because F1 is help on every platform and F5 and F12 are
/// spoken for by browsers, which matters for a switch documented in one place
/// for both targets even though only one of them acts on it.
pub(crate) const KEY: jidousha_input::Key = jidousha_input::Key::F9;

/// Where snapshots go, relative to wherever the program was started.
///
/// `target/` because that is this project's one disposable directory: it is in
/// `.gitignore`, `tools/verify` already writes its artifacts there
/// (tooling.md §3), and a reader who has been told "the file is under target/"
/// needs no further instructions to find it.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
const DIRECTORY: &str = "target";

/// How many snapshots one run will write before it stops.
///
/// A held-down key repeats, and a repeat that wrote a file would fill a disk
/// while somebody leaned on the keyboard. The driver already filters auto-repeat
/// out of `KeyPressed` (input.md §2), so this is the second belt: a run that
/// somehow produced a thousand snapshots has a bug, and the thousand-and-first
/// would not be the file that revealed it.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
const LIMIT: u32 = 1_000;

/// The overlay's snapshot writer.
pub(crate) struct Snapshots {
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    written: u32,
}

impl Snapshots {
    /// A writer that has written nothing.
    pub(crate) fn new() -> Self {
        Self { written: 0 }
    }

    /// Write `readout` to a fresh file, and say where it went.
    ///
    /// Returns the path, so the panel can name the file it just produced — the
    /// commonest thing to get wrong about a key that writes a file is not
    /// knowing whether it did anything.
    ///
    /// `None` when the file could not be written or the run has hit
    /// [`LIMIT`]. A snapshot that fails is not a reason to stop a game, and it
    /// is not reported through `report::problem` either: this is a key a person
    /// pressed, and the panel telling them is the whole of the feedback that is
    /// wanted (no stream is ever printed to — frame-pacing.md §6).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn write(&mut self, readout: &str) -> Option<std::path::PathBuf> {
        if self.written >= LIMIT {
            return None;
        }
        let path = std::path::Path::new(DIRECTORY).join(name(crate::clock::stamp(), self.written));
        std::fs::create_dir_all(DIRECTORY).ok()?;
        std::fs::write(&path, body(readout)).ok()?;
        self.written += 1;
        Some(path)
    }

    /// The web has no path to write to, so this never writes one.
    ///
    /// Present rather than absent so the driver has one shape on both targets:
    /// the asymmetry is stated here and in the documentation, instead of being
    /// a `cfg` around the call site that a reader has to notice.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn write(&mut self, _readout: &str) -> Option<std::path::PathBuf> {
        None
    }
}

/// What one snapshot is called.
///
/// Timestamped, so a sequence of presses sorts into the order they happened and
/// a file pasted into a pull request says when it was taken. The counter breaks
/// the tie between two presses inside the same second, which is easy to do by
/// accident and would otherwise silently overwrite the first one.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn name(stamp: u64, index: u32) -> String {
    format!("jidousha-perf-{stamp}-{index:03}.txt")
}

/// What one snapshot contains.
///
/// The panel's own text and one trailing newline, and nothing else. Not JSON,
/// not a table: the point of the file is that it is exactly what was on screen,
/// so the numbers in a pull request and the numbers in the screenshot beside it
/// cannot disagree.
///
/// Printable ASCII throughout, because the readout is (`mod.rs`) and because a
/// file that is pasted into a review has the same constraint a bitmap font has
/// for a different reason.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn body(readout: &str) -> String {
    let mut text = String::from(readout);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_snapshots_in_the_same_second_are_two_files() {
        // The collision this rules out: a timestamp in whole seconds, two
        // presses a moment apart, and the first reading silently replaced by
        // the second — which is worst precisely when somebody is capturing a
        // before and an after.
        assert_ne!(name(1_756_000_000, 0), name(1_756_000_000, 1));
    }

    #[test]
    fn a_snapshots_name_carries_the_time_it_was_taken() {
        assert_eq!(
            name(1_756_000_000, 0),
            "jidousha-perf-1756000000-000.txt",
            "sorts by time, and says which run it came from"
        );
    }

    #[test]
    fn the_file_is_the_panel_and_nothing_else() {
        // Not a format of its own: the file and the screenshot beside it in a
        // pull request have to be the same numbers, and a second formatter is
        // a second thing that can disagree.
        let readout = "jidousha frame pacing: JIDOUSHA_FRAMETIME=2\npresent   ~60.0 fps";
        assert_eq!(body(readout), format!("{readout}\n"));
    }

    #[test]
    fn a_readout_that_already_ends_in_a_newline_does_not_gain_a_second_one() {
        assert_eq!(body("one line\n"), "one line\n");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_snapshot_is_written_where_it_says_and_holds_what_the_panel_said() {
        // End to end, on the platform that has the key: the file exists, it is
        // under `target/`, and its bytes are the readout's.
        let mut snapshots = Snapshots::new();
        let readout = "jidousha perf snapshot test\nbusy 3%";
        let Some(path) = snapshots.write(readout) else {
            panic!("target/ is writable from a test run");
        };
        assert!(
            path.starts_with(DIRECTORY),
            "{} is not under target/",
            path.display()
        );
        let Ok(written) = std::fs::read_to_string(&path) else {
            panic!("the snapshot was reported as written");
        };
        assert_eq!(written, format!("{readout}\n"));
        assert!(
            written
                .chars()
                .all(|c| c == '\n' || (' '..='~').contains(&c)),
            "a snapshot is printable ASCII: {written:?}"
        );
        let _ = std::fs::remove_file(&path);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_run_that_never_presses_the_key_writes_nothing() {
        // The whole of "the only file the overlay ever writes, and only on that
        // press": constructing the writer must not touch the filesystem.
        let snapshots = Snapshots::new();
        assert_eq!(snapshots.written, 0);
    }
}
