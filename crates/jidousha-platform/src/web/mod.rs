//! The web side of the platform: the asset source, and the page's seams.
//!
//! Key types: `WebSource`; `asset_url`, `query_parameter`; `panic`,
//! `render_scale`.
//! Depends on: `web-sys`, `wasm-bindgen-futures`, `jidousha-assets` (web only).
//! Must never be depended on by: `jidousha-assets` — I/O lives on this side of
//! the seam, exactly as it does for the native loader (assets.md §5).
//! INVARIANT: nothing here is observable to simulation except through
//! `Assets::commit`. The browser resolves requests on its own schedule and they
//! land in a queue; the store drains that queue at one point per frame, which
//! is what keeps network timing off the recorded timeline (assets.md §4).
//!
//! **Where this is verified.** The glue is checked by
//! `tools/serve-web sprites --check`, which loads the page in a real Chromium
//! and asserts the art appeared — there is no browser in `cargo test`, and the
//! wasm CI job is a `cargo check`. So everything that *can* be tested on any
//! machine was deliberately put where it can be: the status-to-error mapping is
//! `AssetError::from_http_status`, in the assets crate, and the URL join is
//! below rather than inside the `cfg`. Mutation testing is what made that
//! second point stick — the join's tests were written inside the wasm-only
//! module first, where they never compiled and never ran.

/// The URL for one asset path, relative to the page.
///
/// Relative, always: a game served from `/games/pong/` asks for
/// `assets/hero.png` and the browser resolves it against the page. An absolute
/// path would work in development and break the moment the game is deployed
/// anywhere but the root of a domain — which is the kind of bug that only
/// appears after shipping.
///
/// Compiled on every target although only the web calls it. A function behind a
/// `cfg` is a function no test on this machine can reach, and this one has
/// exactly the failure mode that has to be caught before a deploy rather than
/// after.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn asset_url(root: &str, path: &str) -> String {
    format!("{}/{}", root.trim_end_matches('/'), path)
}

/// The value of one query-string parameter, if the page URL carries it.
///
/// The **one** way this crate reads a page parameter, so `?panic=1` and
/// `?renderscale=0.5` cannot disagree about what a query string means. Matching
/// is on the whole parameter name, never a substring: `?nopanic=1` does not
/// carry `panic`, and `?renderscaled=2` does not carry `renderscale`. A check
/// that can be tripped by accident is a check nobody trusts.
///
/// A parameter written without a value (`?panic`) reads as an empty value
/// rather than as absent — the caller decides whether that is a request or a
/// mistake, and both of this crate's callers call it a mistake.
///
/// Compiled on every target although only the web calls it, for the same reason
/// `asset_url` is: a function behind a `cfg` is a function no test on this
/// machine can reach.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) fn query_parameter<'a>(search: &'a str, name: &str) -> Option<&'a str> {
    search
        .trim_start_matches('?')
        .split('&')
        .find_map(|parameter| {
            let (key, value) = parameter.split_once('=').unwrap_or((parameter, ""));
            (key == name).then_some(value)
        })
}

#[cfg(target_arch = "wasm32")]
mod fetch;
pub(crate) mod panic;
pub(crate) mod render_scale;

#[cfg(target_arch = "wasm32")]
pub use fetch::WebSource;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_asset_url_is_relative_to_the_page() {
        // Absolute would work in development and break the moment the game is
        // served from anywhere but the root of a domain.
        assert_eq!(
            asset_url("assets", "sprites/hero.png"),
            "assets/sprites/hero.png"
        );
        assert!(!asset_url("assets", "hero.png").starts_with('/'));
    }

    #[test]
    fn a_root_with_a_trailing_slash_does_not_double_it() {
        assert_eq!(asset_url("assets/", "hero.png"), "assets/hero.png");
    }

    #[test]
    fn a_nested_root_keeps_its_shape() {
        assert_eq!(
            asset_url("static/art", "ui/panel.png"),
            "static/art/ui/panel.png"
        );
    }

    #[test]
    fn a_query_parameter_is_read_by_its_whole_name() {
        assert_eq!(query_parameter("?panic=1", "panic"), Some("1"));
        assert_eq!(
            query_parameter("?frametime=1&renderscale=0.5", "renderscale"),
            Some("0.5")
        );
        assert_eq!(query_parameter("", "panic"), None);
        assert_eq!(query_parameter("?nopanic=1", "panic"), None);
        assert_eq!(query_parameter("?renderscaled=2", "renderscale"), None);
    }

    #[test]
    fn a_parameter_written_without_a_value_reads_as_empty_rather_than_absent() {
        assert_eq!(query_parameter("?renderscale", "renderscale"), Some(""));
    }

    #[test]
    fn the_first_spelling_of_a_repeated_parameter_wins() {
        // Not a decision worth making twice: one answer, and it is the one a
        // reader of the URL sees first.
        assert_eq!(
            query_parameter("?renderscale=0.5&renderscale=1", "renderscale"),
            Some("0.5")
        );
    }
}
