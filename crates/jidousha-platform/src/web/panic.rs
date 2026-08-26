//! The wasm panic path: the §9 message, delivered to the page.
//!
//! Key functions: `panic_message`, `install`, `forced_panic_requested`.
//! Depends on: `web-sys` (web only). Must never be depended on by:
//! `jidousha-core` — the hook chains onto core's, it does not replace it.
//! INVARIANT: a panic on the web is never console-only. The playtest page
//! renders it as an overlay (web-publish.md §2 CONTRACT), and this module is
//! the engine's half of that: the full §9 text, on `console.error`, behind a
//! marker the page recognizes.
//!
//! `eprintln!` writes into a void on `wasm32-unknown-unknown` (see `report`),
//! so without this the §9 panic message — the whole point of panicking with a
//! great message — is lost on exactly the target where the person watching is
//! a remote playtester with no terminal. The pure formatting lives outside the
//! `cfg` for the same reason `asset_url` does: a function behind a `cfg` is a
//! function no test on this machine can reach.

/// The first line of a panic report on the console.
///
/// The playtest page watches `console.error` for this marker and renders
/// everything after it as the panic overlay. The engine's *handled* §9 reports
/// (`report::problem`) do not carry it — a missing asset is worth showing and
/// is not a panic.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) const PANIC_MARKER: &str = "[jidousha panic]";

/// The full §9 text for one panic.
///
/// Engine panics already carry the complete §9 message as their payload
/// (core.md §9 CONTRACT) — those pass through verbatim, with the source
/// location appended. A game-code panic with an arbitrary payload is wrapped
/// in the §9 shape instead, so the overlay always shows what happened, the
/// likely cause, and the fix, whoever panicked.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn panic_message(payload: &str, location: Option<&str>) -> String {
    let at = match location {
        Some(location) => format!("\n  at {location}"),
        None => String::new(),
    };
    if payload.starts_with("[jidousha] ") {
        format!("{payload}{at}")
    } else {
        format!(
            "[jidousha] the game panicked: {payload}{at}\n  \
             likely cause: a bug in a game system — the first line is the panic payload\n  \
             fix: reproduce natively with `cargo run -p jidousha --example <name>` \
             for a full backtrace"
        )
    }
}

/// The panic payload as text, however it was thrown.
///
/// `panic!("...")` payloads are `&str`; `panic!("{}", x)` payloads are
/// `String`; anything else (a typed payload from `panic_any`) has no text to
/// show, and saying so beats showing nothing.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn payload_text(payload: &dyn std::any::Any) -> String {
    if let Some(text) = payload.downcast_ref::<&str>() {
        (*text).to_string()
    } else if let Some(text) = payload.downcast_ref::<String>() {
        text.clone()
    } else {
        "(the panic payload was not text)".to_string()
    }
}

/// Whether a page query string asks for the forced test panic.
///
/// Exactly `panic=1` as a parameter — a substring match would fire on
/// `?nopanic=1` or `?panic=10`, and a check that can be tripped by accident
/// is a check nobody trusts. `super::query_parameter` is what makes the name
/// half of that exact, and it is the same reader `?renderscale=` uses.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn query_asks_for_panic(search: &str) -> bool {
    super::query_parameter(search, "panic") == Some("1")
}

/// Install the hook that mirrors panics onto the console for the page.
///
/// Chains onto whatever hook is already installed (core's, which names the
/// running system) rather than replacing it. Idempotent. Installed by `run`
/// before anything that can panic, so the overlay contract holds from the
/// first instruction (web-publish.md §2).
#[cfg(target_arch = "wasm32")]
pub(crate) fn install() {
    use std::sync::Once;
    static INSTALLED: Once = Once::new();
    INSTALLED.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let payload = payload_text(info.payload());
            let location = info.location().map(|location| location.to_string());
            let text = format!(
                "{PANIC_MARKER}\n{}",
                panic_message(&payload, location.as_deref())
            );
            web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(&text));
            previous(info);
        }));
    });
}

/// Whether the page URL carries `?panic=1` — the forced test panic.
///
/// This is how the panic overlay stays verifiable: `tools/serve-web --check`
/// loads the page with `?panic=1` and asserts the overlay rendered the §9
/// text (web-publish.md §2). Real games keep it too — it costs one query-string
/// read at startup and makes "does the bug reporting work" checkable on any
/// deployed build.
#[cfg(target_arch = "wasm32")]
pub(crate) fn forced_panic_requested() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    match window.location().search() {
        Ok(search) => query_asks_for_panic(&search),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_engine_panic_passes_through_with_its_location_appended() {
        let payload = "[jidousha] component access failed: Position not present\n  \
                       likely cause: x\n  fix: y";
        let message = panic_message(payload, Some("src/game.rs:10:5"));
        assert!(message.starts_with(payload));
        assert!(message.ends_with("\n  at src/game.rs:10:5"));
    }

    #[test]
    fn a_game_panic_is_wrapped_in_the_engines_message_shape() {
        let message = panic_message("index out of bounds", Some("examples/pong.rs:42:9"));
        assert!(message.starts_with("[jidousha] the game panicked: index out of bounds"));
        assert!(message.contains("\n  at examples/pong.rs:42:9"));
        assert!(message.contains("likely cause:"));
        assert!(message.contains("fix:"));
    }

    #[test]
    fn a_panic_with_no_location_gets_no_dangling_at_line() {
        let message = panic_message("boom", None);
        assert!(!message.contains("\n  at "));
    }

    #[test]
    fn str_and_string_payloads_both_read_back_as_their_text() {
        assert_eq!(payload_text(&"literal"), "literal");
        assert_eq!(payload_text(&String::from("formatted")), "formatted");
    }

    #[test]
    fn a_payload_that_is_not_text_says_so_instead_of_showing_nothing() {
        assert_eq!(payload_text(&17_u32), "(the panic payload was not text)");
    }

    #[test]
    fn only_the_exact_panic_parameter_forces_a_panic() {
        assert!(query_asks_for_panic("?panic=1"));
        assert!(query_asks_for_panic("?seed=3&panic=1"));
        assert!(!query_asks_for_panic(""));
        assert!(!query_asks_for_panic("?panic=0"));
        assert!(!query_asks_for_panic("?panic=10"));
        assert!(!query_asks_for_panic("?nopanic=1"));
    }
}
