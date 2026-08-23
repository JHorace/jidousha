//! The naive reference store the real one is checked against, plus the
//! catalogue and operation generator that drive both the model and replay
//! tests.
//!
//! Key types: `Reference`, `Op`, `Handle`, `Rng`.
//! Depends on: `jidousha_assets`' public API only.
//! INVARIANT: the reference implementation is written to be *obviously* right,
//! never efficient — a flat `Vec` of assets in load order, each carrying its
//! own scripted due tick. When the two disagree, the reference is the one that
//! is easy to read (ADR-0006, as core's world model does).
//! INVARIANT: the generator's RNG is this file's own, not `jidousha_core`'s. A
//! test that drew its randomness from the engine would go quiet in exactly the
//! case where the engine's RNG broke.

// Each integration test is its own crate, and each compiles the whole of this
// module while using part of it: the replay test never touches `Reference`, the
// model test never touches it through `Handle::debug` alone. The unused warnings
// are an artifact of that, not a signal about the code.
#![allow(dead_code)]

use jidousha_assets::{
    AssetError, AssetStatus, Assets, BytesHandle, MemorySource, TextureData, TextureHandle,
    encode_png,
};

/// What the world does when asked for a path.
///
/// Pictures are held as the *file's* bytes, exactly as a disk hands them over,
/// because that is where the store's decode lives now: a texture request
/// resolves bytes and the store decodes them (assets.md §3). A catalogue of
/// pre-decoded texels would run past the code this suite exists to check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Content {
    /// A data file's bytes, handed back unchanged.
    Data(&'static [u8]),
    /// A picture: a real PNG of this size, filled with this value.
    Picture { width: u32, height: u32, fill: u8 },
    /// Bytes at a picture's path that are not a picture — the case that used to
    /// resolve `Ready` and draw nothing (FINDINGS G-006).
    Undecodable(&'static [u8]),
    /// The path exists but cannot be read — a permission, a truncated read.
    Unreadable(&'static str),
    /// Nothing is there at all, the commonest real failure.
    Absent,
}

impl Content {
    /// The texels a `Ready` asset of this content holds, if it is a picture.
    pub fn texels(self) -> Option<(u32, u32)> {
        match self {
            Content::Picture { width, height, .. } => Some((width, height)),
            _ => None,
        }
    }
}

/// One path a generated sequence may ask for, and when it answers.
#[derive(Clone, Copy, Debug)]
pub struct Scripted {
    pub path: &'static str,
    pub content: Content,
    /// Whether the game asks for this with `load_texture` rather than
    /// `load_bytes`. A property of the path, not of the operation: a picture
    /// asked for as bytes is a mistake in the store, not a state to explore.
    pub texture: bool,
    /// The tick this path's request completes on. Zero means "at the first
    /// commit after the request", which is what an unscripted source does.
    pub due: u64,
}

/// The paths the generated sequences draw from.
///
/// Deliberately mixed: immediate and late, readable and not, present and
/// absent — so a single sequence walks every edge of the state machine rather
/// than just the happy one.
pub const CATALOG: [Scripted; 9] = [
    Scripted {
        path: "player.png",
        content: Content::Picture {
            width: 2,
            height: 2,
            fill: 11,
        },
        texture: true,
        due: 0,
    },
    Scripted {
        path: "enemy.png",
        content: Content::Picture {
            width: 3,
            height: 1,
            fill: 22,
        },
        texture: true,
        due: 3,
    },
    Scripted {
        path: "boss.png",
        content: Content::Picture {
            width: 4,
            height: 5,
            fill: 33,
        },
        texture: true,
        due: 17,
    },
    Scripted {
        path: "level.bin",
        content: Content::Data(b"level data"),
        texture: false,
        due: 0,
    },
    Scripted {
        path: "music.bin",
        content: Content::Data(b"music data"),
        texture: false,
        due: 8,
    },
    Scripted {
        path: "not-a-picture.png",
        content: Content::Undecodable(b"GIF89a and some bytes that are not a PNG"),
        texture: true,
        due: 0,
    },
    Scripted {
        path: "corrupt.png",
        content: Content::Unreadable("decode failed at byte 12"),
        texture: true,
        due: 0,
    },
    Scripted {
        path: "late-corrupt.bin",
        content: Content::Unreadable("truncated after 4 bytes"),
        texture: false,
        due: 5,
    },
    Scripted {
        path: "missing.png",
        content: Content::Absent,
        texture: true,
        due: 0,
    },
];

/// A flat picture of one value — what a catalogue entry's PNG holds.
pub fn picture(width: u32, height: u32, fill: u8) -> TextureData {
    TextureData {
        width,
        height,
        rgba: vec![fill; (width * height * 4) as usize],
    }
}

/// A source loaded with the catalogue, ready for a store to pull from.
pub fn source() -> MemorySource {
    let mut source = MemorySource::new();
    for entry in CATALOG {
        match entry.content {
            Content::Data(bytes) | Content::Undecodable(bytes) => {
                source.insert(entry.path, bytes.to_vec());
            }
            // The file's bytes, not its texels: the store is what decodes.
            Content::Picture {
                width,
                height,
                fill,
            } => source.insert(entry.path, encode_png(&picture(width, height, fill))),
            Content::Unreadable(reason) => source.fail(
                entry.path,
                AssetError::Unreadable {
                    detail: reason.to_owned(),
                },
            ),
            // Absent means the source has never heard of it: insert nothing.
            Content::Absent => {}
        }
        if entry.due > 0 {
            source.complete_at(entry.path, entry.due);
        }
    }
    source
}

/// One operation in a generated sequence.
#[derive(Clone, Copy, Debug)]
pub enum Op {
    /// Load `CATALOG[index]`, with the load the entry says it is asked for.
    Load { index: usize },
    /// Commit this many ticks after the last one. Zero repeats a tick, which
    /// is legal and must change nothing.
    Commit { advance: u64 },
    /// Unload the live handle at this position.
    Unload { target: usize },
}

/// Either kind of handle, so a sequence can hold a mixed list of them.
///
/// The store's methods are generic over [`AssetHandle`]; a test driving both
/// kinds from one `Vec` needs the runtime version of that choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    Texture(TextureHandle),
    Bytes(BytesHandle),
}

impl Handle {
    pub fn status(self, assets: &Assets) -> AssetStatus {
        match self {
            Handle::Texture(handle) => assets.status(handle),
            Handle::Bytes(handle) => assets.status(handle),
        }
    }

    pub fn bytes_of(self, assets: &Assets) -> Option<&[u8]> {
        match self {
            Handle::Texture(handle) => assets.bytes_of(handle),
            Handle::Bytes(handle) => assets.bytes_of(handle),
        }
    }

    /// The size of the texels behind a texture handle, while the store has them.
    ///
    /// `None` for a bytes handle, which never holds a picture.
    pub fn texels(self, assets: &Assets) -> Option<(u32, u32)> {
        match self {
            Handle::Texture(handle) => assets
                .texture_of(handle)
                .map(|texture| (texture.width, texture.height)),
            Handle::Bytes(_) => None,
        }
    }

    pub fn path_of(self, assets: &Assets) -> &str {
        match self {
            Handle::Texture(handle) => assets.path_of(handle),
            Handle::Bytes(handle) => assets.path_of(handle),
        }
    }

    pub fn unload(self, assets: &mut Assets) {
        match self {
            Handle::Texture(handle) => assets.unload(handle),
            Handle::Bytes(handle) => assets.unload(handle),
        }
    }

    /// The handle as the store prints it — the string the replay test compares,
    /// so that slot reuse is part of what "the same run" means.
    pub fn debug(self) -> String {
        match self {
            Handle::Texture(handle) => format!("{handle:?}"),
            Handle::Bytes(handle) => format!("{handle:?}"),
        }
    }
}

/// One asset as the model sees it.
#[derive(Clone, Debug)]
pub struct ModelAsset {
    pub entry: Scripted,
    pub status: AssetStatus,
    pub live: bool,
}

/// The reference store: every asset ever loaded, in load order, and nothing
/// clever.
///
/// It does not model slots or generations at all — assets are named by their
/// position in this list, which is what makes it obviously right. Slot reuse is
/// checked separately, by comparing the handles the real store hands out.
#[derive(Debug)]
pub struct Reference {
    assets: Vec<ModelAsset>,
}

impl Reference {
    pub fn new() -> Self {
        Self { assets: Vec::new() }
    }

    /// Record a load, returning the key the caller uses to refer to it.
    pub fn load(&mut self, entry: Scripted) -> usize {
        self.assets.push(ModelAsset {
            entry,
            status: AssetStatus::Loading,
            live: true,
        });
        self.assets.len() - 1
    }

    pub fn unload(&mut self, key: usize) {
        self.assets[key].live = false;
    }

    /// Resolve everything due by `tick`, returning the paths that failed, in
    /// load order — which is request order, which is the order the real store
    /// must report them in.
    pub fn commit(&mut self, tick: u64) -> Vec<String> {
        let mut failures = Vec::new();
        for asset in &mut self.assets {
            if !asset.live || asset.status != AssetStatus::Loading || asset.entry.due > tick {
                continue;
            }
            match asset.entry.content {
                Content::Data(_) | Content::Picture { .. } => asset.status = AssetStatus::Ready,
                // Bytes that are not a picture fail exactly as a missing file
                // does: the store decodes at the boundary, so there is no third
                // state where a texture is Ready with nothing in it.
                Content::Undecodable(_) | Content::Unreadable(_) | Content::Absent => {
                    asset.status = AssetStatus::Failed;
                    failures.push(asset.entry.path.to_owned());
                }
            }
        }
        failures
    }

    pub fn status(&self, key: usize) -> AssetStatus {
        self.assets[key].status
    }

    pub fn path(&self, key: usize) -> &str {
        self.assets[key].entry.path
    }

    /// The bytes a `Ready` data asset carries; nothing otherwise.
    ///
    /// A picture carries texels rather than bytes once it is decoded — see
    /// [`texels`](Reference::texels).
    pub fn bytes(&self, key: usize) -> Option<&'static [u8]> {
        let asset = &self.assets[key];
        match (asset.status, asset.entry.content) {
            (AssetStatus::Ready, Content::Data(bytes)) => Some(bytes),
            _ => None,
        }
    }

    /// The size of the texels a `Ready` picture holds; nothing otherwise.
    ///
    /// The model's half of "a store never reports `Ready` for a texture it has
    /// no texels for": every `Ready` picture here has a size, so the real store
    /// answering `None` is a mismatch rather than a shrug (FINDINGS G-006).
    pub fn texels(&self, key: usize) -> Option<(u32, u32)> {
        let asset = &self.assets[key];
        match asset.status {
            AssetStatus::Ready => asset.entry.content.texels(),
            _ => None,
        }
    }

    /// Nothing the game still holds is in flight.
    ///
    /// An unloaded asset is nobody's business, and `Failed` counts as resolved
    /// — a game gating on this must not wait for a file that will never come.
    pub fn all_ready(&self) -> bool {
        self.assets
            .iter()
            .all(|asset| !asset.live || asset.status != AssetStatus::Loading)
    }
}

/// The generator's own RNG — a SplitMix64, chosen because it is short enough to
/// read and has nothing to do with the engine's (see this file's INVARIANT).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 32) as u32
    }

    pub fn below(&mut self, limit: u32) -> u32 {
        self.next_u32() % limit
    }
}

/// Build one operation sequence.
///
/// Weighted so commits are common — a store that is never committed never
/// leaves `Loading`, and the interesting states are the ones after a commit.
/// Unloads are frequent enough that slot reuse, and bytes arriving for a slot
/// nobody owns any more, both happen well inside a sequence.
pub fn generate(seed: u64, length: usize) -> Vec<Op> {
    let mut rng = Rng::new(seed);
    let mut ops = Vec::with_capacity(length);
    let mut live = 0usize;
    for _ in 0..length {
        let op = match rng.below(100) {
            0..=39 => {
                live += 1;
                Op::Load {
                    index: rng.below(CATALOG.len() as u32) as usize,
                }
            }
            40..=79 => Op::Commit {
                advance: u64::from(rng.below(4)),
            },
            _ if live > 0 => {
                let target = rng.below(live as u32) as usize;
                live -= 1;
                Op::Unload { target }
            }
            // Nothing to unload yet; a commit is always legal.
            _ => Op::Commit { advance: 1 },
        };
        ops.push(op);
    }
    ops
}
