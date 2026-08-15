//! Which system is running, and saying so when something goes wrong.
//!
//! Key functions: `with_running_system`, `running_system`, `install`.
//! Depends on: nothing. Must never depend on: `world` — this is bookkeeping the
//! schedule keeps, and error text elsewhere reads.
//! INVARIANT: the recorded system is cleared when the call returns, panic
//! included, so a stale name can never be attached to a later failure.
//!
//! core.md §9 requires every engine message to name the running system. The
//! schedule is the only place that knows it, so it records it here and both the
//! panic hook and the world's own messages read it back.

use std::cell::Cell;
use std::sync::Once;

thread_local! {
    /// The (phase, system) currently executing on this thread, if any.
    static RUNNING: Cell<Option<(&'static str, &'static str)>> = const { Cell::new(None) };
}

/// Whether the hook has been installed. Installing twice would chain the
/// engine's line onto itself.
static INSTALLED: Once = Once::new();

/// Restores the previously running system on the way out, panic or not.
struct Restore(Option<(&'static str, &'static str)>);

impl Drop for Restore {
    fn drop(&mut self) {
        RUNNING.with(|running| running.set(self.0));
    }
}

/// Run `body` with `system` recorded as the running system.
pub(crate) fn with_running_system<R>(
    phase: &'static str,
    system: &'static str,
    body: impl FnOnce() -> R,
) -> R {
    let previous = RUNNING.with(|running| running.replace(Some((phase, system))));
    // The guard runs while the panic unwinds, so the hook below still sees the
    // system that failed, and anything after it does not.
    let _restore = Restore(previous);
    body()
}

/// The system currently running, as `"system_name (Phase)"`.
///
/// `None` outside any system — during setup, or in a test driving the world
/// directly.
pub(crate) fn running_system() -> Option<String> {
    RUNNING.with(|running| {
        running
            .get()
            .map(|(phase, system)| format!("{system} ({phase})"))
    })
}

/// The `in system:` line for an engine message, or nothing outside a system.
///
/// Written to slot into the §9 format between the specifics and the likely
/// cause.
pub(crate) fn in_system_line() -> String {
    match running_system() {
        Some(system) => format!("\n  in system: {system}"),
        None => String::new(),
    }
}

/// Install the panic hook that names the running system.
///
/// Called by [`headless`](crate::headless) and, later, by the windowed driver:
/// the app lifecycle owns it, so a library user who only touches `World`
/// directly gets no global side effects. Idempotent, and it chains to whatever
/// hook was already installed rather than replacing it.
pub(crate) fn install() {
    INSTALLED.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if let Some(system) = running_system() {
                eprintln!("[jidousha] the panic below happened inside system: {system}");
            }
            previous(info);
        }));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_system_is_running_by_default() {
        assert_eq!(running_system(), None);
        assert_eq!(in_system_line(), "");
    }

    #[test]
    fn the_running_system_is_reported_while_it_runs() {
        with_running_system("Update", "physics", || {
            assert_eq!(running_system().as_deref(), Some("physics (Update)"));
            assert_eq!(in_system_line(), "\n  in system: physics (Update)");
        });
        assert_eq!(running_system(), None);
    }

    #[test]
    fn nested_systems_restore_the_outer_one() {
        with_running_system("Update", "outer", || {
            with_running_system("Update", "inner", || {
                assert_eq!(running_system().as_deref(), Some("inner (Update)"));
            });
            assert_eq!(running_system().as_deref(), Some("outer (Update)"));
        });
    }

    #[test]
    fn a_panicking_system_does_not_leave_its_name_behind() {
        let panicked = std::panic::catch_unwind(|| {
            with_running_system("Update", "explodes", || panic!("boom"));
        });
        assert!(panicked.is_err());
        assert_eq!(
            running_system(),
            None,
            "the name must be cleared while unwinding, or the next failure inherits it"
        );
    }
}
