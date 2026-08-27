//! `?constants=` — a tuning configuration as a URL (UI.md §12).
//!
//! A playtest link that carries its weights, and a repro link when a playtester
//! reports a feel. The whole of the web half is below: read the page's query
//! string, hand the `constants` value to `Tuning::parse`, and let `main` decide
//! what to do with either answer.
//!
//! **Native has no equivalent and is not getting one.** The drawer is the native
//! path (UI.md §12: the drawer is reachable on every platform — no query params,
//! no flags), and a `--constants` flag would be a second way to do the same
//! thing (conventions §1). On native this module answers `None` and the game
//! starts at the shipped set.
//!
//! **Why this file names `web_sys`.** The facade has no launch parameters: the
//! engine reads `?frametime=` and `?renderscale=` off the same page and exposes
//! neither the values nor the mechanism, and `std::env::args` is empty on
//! `wasm32-unknown-unknown`. So a game that wants a shareable URL reads the
//! location itself. That is FINDINGS G-008, and this is its workaround.

use crate::constants::{ConstantsError, Tuning};

/// The set the page asked for, if it asked for one.
///
/// `None` means no `?constants=` (or not a browser); `Some(Err(..))` means the
/// page asked for something that is not a constants set, which is refused
/// rather than clamped.
pub fn constants() -> Option<Result<Tuning, ConstantsError>> {
    query_value("constants").map(|value| Tuning::SHIPPED.parse(&value))
}

/// The session seed the page asked for with `?seed=`: the scenario stamps it
/// everywhere a recording looks — a repro link is `?constants=...&seed=...`.
/// S1 never reads the `Rng`; the plumbing and the stamp remain (DESIGN §2).
/// Refused loudly, never guessed at.
pub fn seed() -> Option<Result<u64, String>> {
    query_value("seed").map(|value| parse_seed(&value))
}

/// The `?seed=` grammar, split out so `links.rs` can hold it to its refusals
/// without a browser.
pub fn parse_seed(value: &str) -> Result<u64, String> {
    let value = value.trim();
    value.parse::<u64>().map_err(|_| {
        format!("?seed= was given {value:?}, which is not a whole number - write ?seed=7")
    })
}

/// The value of one query parameter of the page this is running on.
#[cfg(target_arch = "wasm32")]
fn query_value(key: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    // `search` is the raw `?a=1&b=2`. Percent-decoding is not needed for the
    // vocabulary `Tuning::parse` accepts - names, colons, commas and digits -
    // except for the comma, which a browser may write as `%2C`; nothing else
    // in the grammar has an encoded form, so one substitution covers it and an
    // unexpected escape falls through to `parse`, which refuses it by name.
    let search = search.trim_start_matches('?').replace("%2C", ",");
    search
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| *name == key)
        .map(|(_, value)| value.to_owned())
}

/// Native: there is no page, so there is no parameter (see the module header).
#[cfg(not(target_arch = "wasm32"))]
fn query_value(_key: &str) -> Option<String> {
    None
}
