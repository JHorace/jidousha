//! Asset handles and poll-based loading — async by design on every platform.
//!
//! Key types: `Assets`, `AssetStatus`, `TextureHandle`, `BytesHandle`,
//! `ByteSource`, `MemorySource`.
//! Depends on: `jidousha-core`. Must never be depended on by: `jidousha-core`.
//! INVARIANT: no async runtime, no `async fn`, no `.await` — anywhere in this
//! crate or its public API (ADR-0011). Loads are asked for, and answers are
//! collected at one deterministic point per frame.
//! INVARIANT: no filesystem, no network. Bytes arrive through the
//! [`ByteSource`] seam, which the platform crates implement (assets.md §5).
//!
//! Built so far (`docs/internal/assets.md` §8): A0 — the store, the states and
//! the commit point; A1 — PNG decoding and, in `jidousha-platform`, the native
//! loader. The web loader is A2's. [`Assets::take_uploads`] is the renderer's
//! side of the store, added at R2 when there was finally a GPU to hand texels
//! to.
//!
//! ```
//! use jidousha_assets::{Assets, AssetStatus, MemorySource};
//!
//! let mut source = MemorySource::new();
//! source.insert("player.png", b"pretend this is a png".to_vec());
//! source.complete_at("player.png", 2);
//!
//! let mut assets = Assets::new(source);
//! let player = assets.load_texture("player.png");
//!
//! // The handle works immediately; the bytes are still on their way.
//! assert_eq!(assets.status(player), AssetStatus::Loading);
//!
//! assets.commit(1);
//! assert_eq!(assets.status(player), AssetStatus::Loading);
//!
//! assets.commit(2);
//! assert_eq!(assets.status(player), AssetStatus::Ready);
//! assert!(assets.all_ready());
//! ```

mod assets;
mod decode;
mod handle;
mod payload;
mod replay;
mod source;

pub use assets::{AssetFailure, AssetStatus, Assets, Resolution, TextureUpload};
pub use decode::decode_png;
pub use handle::{AssetHandle, AssetKind, BytesHandle, TextureHandle};
pub use payload::{AssetError, MAX_TEXTURE_SIZE, Payload, TextureData};
pub use replay::ReplaySource;
pub use source::{ByteSource, Completion, MemorySource, RequestId};
