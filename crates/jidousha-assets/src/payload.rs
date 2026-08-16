//! What a source hands back, and the ways it can fail.
//!
//! Key types: `Payload`, `TextureData`, `AssetError`.
//! Depends on: `handle` (for `AssetKind`), `jidousha-core` (for the §9 message
//! format). Must never depend on: any I/O.
//! INVARIANT: a payload is finished work. A source returns decoded texels for a
//! texture, not a file it has not looked at — which is what lets the native
//! loader decode on its own thread and keeps PNG decoding off the frame
//! (assets.md §5).

use core::fmt;

use jidousha_core::message;

use crate::handle::AssetKind;

/// The widest texture the engine will accept, on either axis.
///
/// The WebGL2 envelope's floor (renderer.md §8): safe across old mobile GPUs
/// and every browser. Enforced at decode, so an oversized image fails with a
/// message naming the file rather than at upload with a driver error.
pub const MAX_TEXTURE_SIZE: u32 = 2048;

/// Decoded image texels, ready for the GPU.
///
/// RGBA8, sRGB, row-major, top row first (conventions).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureData {
    /// Width in texels.
    pub width: u32,
    /// Height in texels.
    pub height: u32,
    /// `width * height * 4` bytes.
    pub rgba: Vec<u8>,
}

/// What arrived for one request.
///
/// A source returns the shape matching the [`AssetKind`] that was asked for.
/// Nothing downstream re-interprets it: the store files it, and the renderer
/// uploads it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    /// Raw bytes, exactly as they were read.
    Bytes(Vec<u8>),
    /// An image, already decoded.
    Texture(TextureData),
}

impl Payload {
    /// Which kind of load this answers.
    #[must_use]
    pub fn kind(&self) -> AssetKind {
        match self {
            Payload::Bytes(_) => AssetKind::Bytes,
            Payload::Texture(_) => AssetKind::Texture,
        }
    }
}

/// Why an asset did not arrive.
///
/// The failure classes assets.md §6 asks for, kept apart so each can say
/// something specific. A source reports which one happened; the store formats
/// it with the callsite that asked (core.md §9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssetError {
    /// Nothing at that path.
    NotFound,
    /// A file exists whose name differs only in case.
    ///
    /// The single most valuable message in this list: it is the bug that works
    /// on one developer's machine and 404s on the web server, and naming the
    /// near-miss turns a mystery into a rename (assets.md §2).
    CaseMismatch {
        /// The name as it actually appears on disk.
        on_disk: String,
    },
    /// The path exists but could not be read.
    Unreadable {
        /// What the operating system said.
        detail: String,
    },
    /// The bytes are not a picture this engine can read.
    Decode {
        /// What the decoder said.
        detail: String,
    },
    /// The image is larger than the envelope allows.
    TooLarge {
        /// Its width in texels.
        width: u32,
        /// Its height in texels.
        height: u32,
    },
    /// A web server answered, and its answer was not the file.
    ///
    /// Web only. A 404 is reported as [`AssetError::NotFound`] instead, because
    /// that is what it means and its message is the useful one; this carries
    /// everything else — a 403 from a misconfigured host, a 500, a redirect
    /// that went nowhere.
    Http {
        /// The status the server sent.
        status: u16,
    },
    /// The request never reached a server at all.
    ///
    /// Web only: the network is down, the page is offline, or the browser
    /// refused the request before making it.
    Unreachable {
        /// What the browser said.
        detail: String,
    },
}

impl AssetError {
    /// What an HTTP status means for an asset, or `None` if it means success.
    ///
    /// Lives here rather than in the platform crate because this is the failure
    /// taxonomy and because the web source that calls it compiles for exactly
    /// one target — a mapping table behind a `cfg` is a mapping table no test
    /// on this machine can reach (assets.md §6).
    ///
    /// **404 becomes [`NotFound`](AssetError::NotFound)**, not an `Http`. A
    /// missing file is a missing file whether a filesystem or a web server says
    /// so, and `NotFound`'s message — check the spelling, check the asset root —
    /// is the one that helps. Everything else keeps its status, because "the
    /// server said 403" needs a different fix from "you typed the name wrong".
    #[must_use]
    pub fn from_http_status(status: u16) -> Option<AssetError> {
        match status {
            // 2xx is the file. 3xx never arrives here: the browser follows
            // redirects itself and reports the status it landed on.
            200..=299 => None,
            404 => Some(AssetError::NotFound),
            other => Some(AssetError::Http { status: other }),
        }
    }

    /// The failure as a §9 message, given what was being loaded and from where.
    ///
    /// `requested_at` is the game's own line, recorded at the load callsite, so
    /// the message points at the code that asked rather than at the loader
    /// (assets.md §6).
    #[must_use]
    pub fn message(&self, path: &str, kind: AssetKind, requested_at: &str) -> String {
        let what = format!("asset failed: {path:?}");
        let specifics = format!("requested by: load_{kind} at {requested_at}");
        let (cause, fix) = match self {
            AssetError::NotFound => (
                "no file at that path under the asset root".to_owned(),
                "check the spelling, and check the file is inside the asset root — paths are \
                 relative to it and use forward slashes on every platform"
                    .to_owned(),
            ),
            AssetError::CaseMismatch { on_disk } => (
                format!("the file on disk is named {on_disk:?} — the case differs"),
                "rename the file or the path so they match exactly. Loads are case-strict on \
                 every platform, including Windows, so that art which works locally also works \
                 on a web server (assets.md §2)"
                    .to_owned(),
            ),
            AssetError::Unreadable { detail } => (
                format!("the file exists but could not be read: {detail}"),
                "check the file's permissions, and that it is a file rather than a directory"
                    .to_owned(),
            ),
            AssetError::Decode { detail } => (
                format!("the bytes are not a PNG this engine can read: {detail}"),
                "re-export the image as a PNG. v1 decodes PNG only — no JPEG, no GIF \
                 (assets.md §3)"
                    .to_owned(),
            ),
            AssetError::TooLarge { width, height } => (
                format!("the image is {width}x{height} texels"),
                format!(
                    "resize it to {MAX_TEXTURE_SIZE}x{MAX_TEXTURE_SIZE} or smaller, or split it. \
                     That is the WebGL2 envelope's limit, and it is enforced everywhere so a \
                     texture that works on your machine also works on the web (renderer.md §8)"
                ),
            ),
            AssetError::Http { status } => (
                format!("the server answered {status} for it"),
                "check that the file is actually deployed and that the server is willing to serve \
                 it. Note that a web server is case-sensitive even when your machine is not, so a \
                 name that differs only in case is a 404 here and a working file at home — which \
                 is why loads are case-strict on every platform (assets.md §2)"
                    .to_owned(),
            ),
            AssetError::Unreachable { detail } => (
                format!("the request never reached a server: {detail}"),
                "check the network, and check the page is being served rather than opened from \
                 disk — a `file://` page cannot fetch its own assets in most browsers"
                    .to_owned(),
            ),
        };
        message(&what, &specifics, &cause, &fix)
    }
}

impl fmt::Display for AssetError {
    /// The cause alone, for a source that wants to log without a callsite.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetError::NotFound => formatter.write_str("not found"),
            AssetError::CaseMismatch { on_disk } => {
                write!(formatter, "case mismatch with {on_disk:?}")
            }
            AssetError::Unreadable { detail } => write!(formatter, "unreadable: {detail}"),
            AssetError::Decode { detail } => write!(formatter, "decode failed: {detail}"),
            AssetError::TooLarge { width, height } => {
                write!(formatter, "too large: {width}x{height}")
            }
            AssetError::Http { status } => write!(formatter, "http {status}"),
            AssetError::Unreachable { detail } => write!(formatter, "unreachable: {detail}"),
        }
    }
}

impl core::error::Error for AssetError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_payload_knows_which_kind_it_answers() {
        assert_eq!(Payload::Bytes(vec![1]).kind(), AssetKind::Bytes);
        assert_eq!(
            Payload::Texture(TextureData {
                width: 1,
                height: 1,
                rgba: vec![0; 4],
            })
            .kind(),
            AssetKind::Texture
        );
    }

    #[test]
    fn every_failure_reads_like_an_engine_error() {
        for error in [
            AssetError::NotFound,
            AssetError::CaseMismatch {
                on_disk: "Player.png".to_owned(),
            },
            AssetError::Unreadable {
                detail: "permission denied".to_owned(),
            },
            AssetError::Decode {
                detail: "bad chunk".to_owned(),
            },
            AssetError::TooLarge {
                width: 4096,
                height: 4096,
            },
            AssetError::Http { status: 403 },
            AssetError::Unreachable {
                detail: "network error".to_owned(),
            },
        ] {
            let text = error.message("art/player.png", AssetKind::Texture, "src/game.rs:12");
            assert!(text.starts_with("[jidousha] asset failed:"), "{text}");
            assert!(text.contains("art/player.png"), "{text}");
            assert!(text.contains("src/game.rs:12"), "{text}");
            assert!(text.contains("likely cause:"), "{text}");
            assert!(text.contains("fix:"), "{text}");
        }
    }

    #[test]
    fn the_case_mismatch_message_names_the_file_that_is_actually_there() {
        // The whole value of separating this from NotFound: the fix is a
        // rename, and the message can say which name.
        let error = AssetError::CaseMismatch {
            on_disk: "Player.png".to_owned(),
        };
        let text = error.message("player.png", AssetKind::Texture, "src/game.rs:12");
        assert!(text.contains("\"Player.png\""), "{text}");
        assert!(text.contains("case-strict"), "{text}");
    }

    #[test]
    fn a_missing_file_reads_the_same_whether_a_disk_or_a_server_said_so() {
        // 404 is not an "http error" to a game — it is a missing file, and the
        // message that helps is the one about spelling and the asset root.
        assert_eq!(
            AssetError::from_http_status(404),
            Some(AssetError::NotFound)
        );
    }

    #[test]
    fn a_successful_status_is_not_a_failure() {
        for status in [200, 201, 206, 299] {
            assert_eq!(AssetError::from_http_status(status), None, "{status}");
        }
    }

    #[test]
    fn every_other_status_keeps_its_number() {
        // "The server said 403" needs a different fix from "you typed the name
        // wrong", so the number survives to the message.
        for status in [400, 403, 500, 503] {
            assert_eq!(
                AssetError::from_http_status(status),
                Some(AssetError::Http { status }),
                "{status}"
            );
        }
    }

    #[test]
    fn the_http_message_warns_about_the_case_trap() {
        // The single most valuable thing this message can say: a web server is
        // case-sensitive even when the machine you built on was not.
        let text = AssetError::Http { status: 403 }.message(
            "sprites/Hero.png",
            AssetKind::Texture,
            "src/game.rs:1",
        );
        // Both halves, separately: the status is what distinguishes this from
        // every other failure, and the case warning is the reason this message
        // is worth more than "it did not work". An `||` between them, which is
        // what this test said first, passes with either one missing.
        assert!(text.contains("403"), "the status is named: {text}");
        assert!(text.contains("case"), "the case trap is named: {text}");
    }

    #[test]
    fn the_size_message_names_the_size_and_the_limit() {
        let error = AssetError::TooLarge {
            width: 4096,
            height: 3000,
        };
        let text = error.message("big.png", AssetKind::Texture, "src/game.rs:1");
        assert!(text.contains("4096x3000"), "{text}");
        assert!(text.contains("2048x2048"), "{text}");
    }
}
