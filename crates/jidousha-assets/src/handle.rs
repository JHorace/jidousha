//! Asset handles: copyable opaque IDs, the same shape as `Entity`.
//!
//! Key types: `TextureHandle`, `BytesHandle`, `AssetHandle`, `AssetKind`.
//! Depends on: nothing. Must never depend on: `store` — a handle is a name, not
//! a lookup.
//! INVARIANT: handles are generational. `unload` bumps the slot's generation, so
//! a handle used after unload is detectably stale rather than silently pointing
//! at whatever took its place (assets.md §1).

use core::fmt;
use core::num::NonZeroU32;

/// The first generation for a slot; generations start at 1 so the niche keeps
/// an id the size of two `u32`s.
const FIRST_GENERATION: NonZeroU32 = NonZeroU32::MIN;

/// Which kind of asset a handle names.
///
/// Handles of different kinds never collide: each kind has its own slots, and
/// the kind is part of the handle's type, so mixing them is a compile error
/// rather than a lookup that quietly finds the wrong thing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetKind {
    /// An image, decoded to texels once the decoder lands (A1).
    Texture,
    /// Bytes, handed back exactly as they arrived.
    Bytes,
}

impl fmt::Display for AssetKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetKind::Texture => formatter.write_str("texture"),
            AssetKind::Bytes => formatter.write_str("bytes"),
        }
    }
}

/// A slot in one kind's table, with the generation that owns it.
///
/// DELIBERATE: declared `pub` while living in a private module and never
/// re-exported, so it is nameable only inside this crate. `pub(crate)` would say
/// the same thing to a reader but not to the compiler: `AssetId` appears in the
/// return type of [`sealed::Lookup::id`], and a supertrait of a public trait is
/// reachable, so `pub(crate)` there is a `private_interfaces` warning — and
/// warnings are errors here. Widening the *declared* visibility is what settles
/// the lint; the private module is what actually keeps this out of the API.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetId {
    index: u32,
    generation: NonZeroU32,
}

impl AssetId {
    pub(crate) fn index(self) -> usize {
        self.index as usize
    }

    pub(crate) fn generation(self) -> NonZeroU32 {
        self.generation
    }
}

impl fmt::Debug for AssetId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} v{}", self.index, self.generation)
    }
}

/// A loaded — or loading — image.
///
/// Returned immediately by `load_texture` and usable at once: the renderer
/// draws a placeholder until the real texels arrive (ADR-0011).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextureHandle(pub(crate) AssetId);

/// A loaded — or loading — blob of bytes, for anything a game invents.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BytesHandle(pub(crate) AssetId);

impl fmt::Debug for TextureHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "TextureHandle({:?})", self.0)
    }
}

impl fmt::Debug for BytesHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "BytesHandle({:?})", self.0)
    }
}

/// What every asset handle can do, so `status` and `unload` take either kind.
///
/// Sealed: [`TextureHandle`] and [`BytesHandle`] are the whole set, and a game
/// cannot add a third. The slot a handle names is the store's business, so the
/// lookup half of this trait lives in a private supertrait — a game can call
/// `kind()`, and cannot reach a slot index at all.
pub trait AssetHandle: sealed::Lookup + Copy + fmt::Debug {
    /// Which table this handle indexes.
    fn kind(self) -> AssetKind;
}

/// The half of [`AssetHandle`](super::AssetHandle) that only the store may use.
///
/// Private module, so `Lookup` is unnameable outside this crate: that is what
/// makes `AssetHandle` sealed, and what keeps [`AssetId`] out of the public API.
pub(crate) mod sealed {
    use super::AssetId;

    /// Turns a handle into the slot it names.
    pub trait Lookup {
        /// The slot, with the generation that must still own it.
        fn id(self) -> AssetId;
    }
}

impl sealed::Lookup for TextureHandle {
    fn id(self) -> AssetId {
        self.0
    }
}

impl sealed::Lookup for BytesHandle {
    fn id(self) -> AssetId {
        self.0
    }
}

impl AssetHandle for TextureHandle {
    fn kind(self) -> AssetKind {
        AssetKind::Texture
    }
}

impl AssetHandle for BytesHandle {
    fn kind(self) -> AssetKind {
        AssetKind::Bytes
    }
}

/// Hands out ids for one kind of asset and recycles unloaded slots.
///
/// INVARIANT: allocation is a pure function of the operation history — free
/// slots are reused LIFO, exactly as entity slots are (core.md §2), so the same
/// sequence of loads and unloads yields the same handles on every run.
#[derive(Debug)]
pub(crate) struct IdAllocator {
    slots: Vec<Slot>,
    free: Vec<u32>,
}

#[derive(Clone, Copy, Debug)]
struct Slot {
    generation: NonZeroU32,
    live: bool,
}

impl IdAllocator {
    pub(crate) fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    pub(crate) fn create(&mut self) -> AssetId {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.live = true;
            return AssetId {
                index,
                generation: slot.generation,
            };
        }
        let Ok(index) = u32::try_from(self.slots.len()) else {
            panic!(
                "[jidousha] asset allocation failed: {} slots already exist\n  \
                 the limit is u32::MAX slots per asset kind\n  \
                 likely cause: assets are loaded every frame and never unloaded\n  \
                 fix: load once and keep the handle, or unload what is no longer needed",
                self.slots.len()
            );
        };
        self.slots.push(Slot {
            generation: FIRST_GENERATION,
            live: true,
        });
        AssetId {
            index,
            generation: FIRST_GENERATION,
        }
    }

    /// Free `id`'s slot, making every outstanding handle to it detectably stale.
    pub(crate) fn destroy(&mut self, id: AssetId) {
        let slot = &mut self.slots[id.index()];
        slot.live = false;
        let Some(next) = slot.generation.checked_add(1) else {
            panic!(
                "[jidousha] asset slot {} has exhausted its generations\n  \
                 the slot has been reused u32::MAX times\n  \
                 likely cause: one asset is loaded and unloaded every frame for years\n  \
                 fix: this is an engine limit — report it with the reproduction",
                id.index
            );
        };
        slot.generation = next;
        self.free.push(id.index);
    }

    pub(crate) fn is_live(&self, id: AssetId) -> bool {
        matches!(
            self.slots.get(id.index()),
            Some(slot) if slot.live && slot.generation == id.generation()
        )
    }

    /// The generation now occupying `id`'s slot, for error messages that
    /// explain *how* a handle went stale.
    pub(crate) fn slot_generation(&self, id: AssetId) -> Option<NonZeroU32> {
        self.slots.get(id.index()).map(|slot| slot.generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_handle_prints_its_slot_and_generation() {
        let mut allocator = IdAllocator::new();
        let handle = TextureHandle(allocator.create());
        assert_eq!(format!("{handle:?}"), "TextureHandle(0 v1)");
    }

    #[test]
    fn an_unloaded_slot_is_reused_with_a_new_generation() {
        let mut allocator = IdAllocator::new();
        let first = allocator.create();
        allocator.destroy(first);
        let reused = allocator.create();
        assert!(allocator.is_live(reused));
        assert!(!allocator.is_live(first), "the old handle is stale");
    }

    #[test]
    fn freed_slots_come_back_most_recently_freed_first() {
        let mut allocator = IdAllocator::new();
        let first = allocator.create();
        let second = allocator.create();
        allocator.destroy(first);
        allocator.destroy(second);
        assert_eq!(format!("{:?}", allocator.create()), "1 v2");
        assert_eq!(format!("{:?}", allocator.create()), "0 v2");
    }

    #[test]
    fn the_two_handle_kinds_report_themselves() {
        let mut allocator = IdAllocator::new();
        let texture = TextureHandle(allocator.create());
        let bytes = BytesHandle(allocator.create());
        assert_eq!(texture.kind(), AssetKind::Texture);
        assert_eq!(bytes.kind(), AssetKind::Bytes);
    }
}
