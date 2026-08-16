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
}

impl AssetError {
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
