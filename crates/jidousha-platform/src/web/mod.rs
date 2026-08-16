//! The web asset source: `fetch`, a shared queue, and decoding at the commit.
//!
//! Key types: `WebSource`; `asset_url`.
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

#[cfg(target_arch = "wasm32")]
mod fetch;

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
}
