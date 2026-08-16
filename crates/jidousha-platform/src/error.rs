//! What can stop a windowed run before it starts.
//!
//! Key types: `RunError`.
//! Depends on: `jidousha-core`. Must never depend on: `winit` types in its
//! public shape — a `RunError` crossing out of this crate carries strings, not
//! winit errors (ADR-0004).
//! INVARIANT: environmental failures only. A missing display is a fact about
//! the machine; a game that registers a broken system is a contract violation
//! and panics per core §9 instead.

use core::fmt;

use jidousha_core::message;

/// Why a windowed run could not start, or could not continue.
///
/// Returned by [`run`](crate::run) rather than panicking, because every variant
/// here is something about the machine rather than something about the game
/// (core.md §9). The commonest by far is the first one, and it is what a
/// headless CI runner sees.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunError {
    /// There is no display to open a window on.
    NoDisplay {
        /// What the platform said.
        detail: String,
    },
    /// The display exists, but the window could not be created.
    WindowCreation {
        /// What the platform said.
        detail: String,
    },
    /// The event loop stopped with an error.
    EventLoop {
        /// What the platform said.
        detail: String,
    },
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (what, detail, cause, fix) = match self {
            RunError::NoDisplay { detail } => (
                "no display to open a window on",
                detail,
                "the program is running headless — over SSH without X forwarding, in a \
                 container, or on a CI runner",
                "run it on a machine with a desktop session, or use jidousha::headless for a \
                 run that needs no window (core.md §8)",
            ),
            RunError::WindowCreation { detail } => (
                "the window could not be created",
                detail,
                "the display server refused the request — a missing compositor, an exhausted \
                 handle limit, or a driver problem",
                "check that other graphical programs start; tools/doctor reports what it can \
                 see of the display",
            ),
            RunError::EventLoop { detail } => (
                "the event loop stopped with an error",
                detail,
                "the display server went away mid-run, or the window system reported a fault",
                "restart the program; if it repeats, report it with the message above",
            ),
        };
        formatter.write_str(&message(what, detail, cause, fix))
    }
}

impl core::error::Error for RunError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_headless_machine_gets_told_what_to_do_instead() {
        // The message a CI runner and an SSH session both see, and the one most
        // likely to be an agent's first encounter with this crate.
        let error = RunError::NoDisplay {
            detail: "DISPLAY and WAYLAND_DISPLAY are both unset".to_owned(),
        };
        let text = error.to_string();
        assert!(
            text.starts_with("[jidousha] no display to open a window on"),
            "{text}"
        );
        assert!(text.contains("likely cause:"), "{text}");
        assert!(
            text.contains("jidousha::headless"),
            "the fix names the thing to do instead: {text}"
        );
    }

    #[test]
    fn every_variant_reads_like_an_engine_error() {
        for error in [
            RunError::NoDisplay {
                detail: "d".to_owned(),
            },
            RunError::WindowCreation {
                detail: "d".to_owned(),
            },
            RunError::EventLoop {
                detail: "d".to_owned(),
            },
        ] {
            let text = error.to_string();
            assert!(text.starts_with("[jidousha] "), "{text}");
            assert!(text.contains("likely cause:"), "{text}");
            assert!(text.contains("fix:"), "{text}");
        }
    }
}
