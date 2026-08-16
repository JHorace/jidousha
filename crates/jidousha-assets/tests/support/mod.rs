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

use jidousha_assets::{AssetStatus, Assets, BytesHandle, MemorySource, TextureHandle};

/// What the world does when asked for a path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Content {
    /// The bytes are there.
    Present(&'static [u8]),
    /// The path exists but cannot be read — a decode error, a permission.
    Unreadable(&'static str),
    /// Nothing is there at all, the commonest real failure.
    Absent,
}

/// One path a generated sequence may ask for, and when it answers.
#[derive(Clone, Copy, Debug)]
pub struct Scripted {
    pub path: &'static str,
    pub content: Content,
    /// The tick this path's request completes on. Zero means "at the first
    /// commit after the request", which is what an unscripted source does.
    pub due: u64,
}

/// The paths the generated sequences draw from.
///
/// Deliberately mixed: immediate and late, readable and not, present and
/// absent — so a single sequence walks every edge of the state machine rather
/// than just the happy one.
pub const CATALOG: [Scripted; 8] = [
    Scripted {
        path: "player.png",
        content: Content::Present(b"player texels"),
        due: 0,
    },
    Scripted {
        path: "enemy.png",
        content: Content::Present(b"enemy texels"),
        due: 3,
    },
    Scripted {
        path: "boss.png",
        content: Content::Present(b"boss texels"),
        due: 17,
    },
    Scripted {
        path: "level.bin",
        content: Content::Present(b"level data"),
        due: 0,
    },
    Scripted {
        path: "music.bin",
        content: Content::Present(b"music data"),
        due: 8,
    },
    Scripted {
        path: "corrupt.png",
        content: Content::Unreadable("decode failed at byte 12"),
        due: 0,
    },
    Scripted {
        path: "late-corrupt.bin",
        content: Content::Unreadable("truncated after 4 bytes"),
        due: 5,
    },
    Scripted {
        path: "missing.png",
        content: Content::Absent,
        due: 0,
    },
];

/// A source loaded with the catalogue, ready for a store to pull from.
pub fn source() -> MemorySource {
    let mut source = MemorySource::new();
    for entry in CATALOG {
        match entry.content {
            Content::Present(bytes) => source.insert(entry.path, bytes.to_vec()),
            Content::Unreadable(reason) => source.fail(entry.path, reason),
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
    /// Load `CATALOG[index]`, as a texture when `as_texture`, else as bytes.
    Load { index: usize, as_texture: bool },
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
                Content::Present(_) => asset.status = AssetStatus::Ready,
                Content::Unreadable(_) | Content::Absent => {
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

    /// The bytes a `Ready` asset carries; nothing otherwise.
    pub fn bytes(&self, key: usize) -> Option<&'static [u8]> {
        let asset = &self.assets[key];
        match (asset.status, asset.entry.content) {
            (AssetStatus::Ready, Content::Present(bytes)) => Some(bytes),
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
                    as_texture: rng.below(2) == 0,
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
