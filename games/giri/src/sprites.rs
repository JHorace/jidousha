//! giri's art library: one role, one file, one handle (UI.md §2, §9).
//!
//! **The role is the contract, not the picture.** Every asset is named for
//! what it *means* — `icon_flame` is desperation, `portrait_tim` is Tim — so
//! the owner's curated library replaces the files by name and no code here
//! changes (DESIGN §7's curation model; UI.md §9's asset slots). Nothing is
//! downloaded: `art/make_placeholders.py` writes the committed PNGs from
//! committed grids, and `assets/CREDITS.md` records the provenance of every
//! one.
//!
//! **The bytes are compiled in, and that is a platform fact rather than a
//! preference.** `tools/build-web` stages the repository's root `assets/`
//! directory beside the page, so a game crate that owns its own art has no
//! path a web build would fetch it from; `include_bytes!` and a `MemorySource`
//! give native and web the identical store, and the loading path is the same
//! `Assets` one either way (FINDINGS G-005).
//!
//! **And the decode is the game's**, which is the part that surprised. A
//! `MemorySource` fed raw PNG bytes with `insert` resolves `Ready` and has
//! nothing for a sprite to sample, so every quad draws the engine's magenta
//! placeholder and no failure is reported anywhere. `insert_texture` with an
//! already-decoded `TextureData` is the path that works, and the only decoder
//! the facade offers is `jidousha::testing::decode_png` (FINDINGS G-006).
//!
//! Sizes are native texels. Everything is drawn at an **integer multiple** of
//! them — the engine samples nearest with no filtering, and a pixel-art icon at
//! a fractional scale is a pixel-art icon with a wobble in it (UI.md §1.4).

use jidousha::prelude::*;
use jidousha::testing::decode_png;

use crate::beats::QuestIcon;

/// One slot in the library.
///
/// Flat rather than nested, because the list *is* UI.md §9's asset-slot table
/// and a reader should be able to count it against that table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Art {
    /// Alex.
    PortraitAlex,
    /// Bob.
    PortraitBob,
    /// Steve.
    PortraitSteve,
    /// Tim.
    PortraitTim,
    /// A mouth in a hillside.
    QuestCave,
    /// A stone that is somebody's.
    QuestCrypt,
    /// Something that watches a road.
    QuestTower,
    /// Something with a door and a lock.
    QuestVault,
    /// Desperation.
    Flame,
    /// Infamy.
    Eye,
    /// Gold, and every payout.
    Coin,
    /// Death, and betrayal.
    Skull,
    /// Regard: a bond or a grudge.
    Heart,
}

/// One entry: the role, the file it lives in, the bytes, and its texel size.
struct Slot {
    art: Art,
    file: &'static str,
    bytes: &'static [u8],
    texels: PhysicalSize,
}

/// The library, in the order `Gallery` indexes it.
const LIBRARY: &[Slot] = &[
    slot(
        Art::PortraitAlex,
        "portrait_alex.png",
        include_bytes!("../assets/portrait_alex.png"),
        16,
        16,
    ),
    slot(
        Art::PortraitBob,
        "portrait_bob.png",
        include_bytes!("../assets/portrait_bob.png"),
        16,
        16,
    ),
    slot(
        Art::PortraitSteve,
        "portrait_steve.png",
        include_bytes!("../assets/portrait_steve.png"),
        16,
        16,
    ),
    slot(
        Art::PortraitTim,
        "portrait_tim.png",
        include_bytes!("../assets/portrait_tim.png"),
        16,
        16,
    ),
    slot(
        Art::QuestCave,
        "quest_cave.png",
        include_bytes!("../assets/quest_cave.png"),
        12,
        12,
    ),
    slot(
        Art::QuestCrypt,
        "quest_crypt.png",
        include_bytes!("../assets/quest_crypt.png"),
        12,
        12,
    ),
    slot(
        Art::QuestTower,
        "quest_tower.png",
        include_bytes!("../assets/quest_tower.png"),
        12,
        12,
    ),
    slot(
        Art::QuestVault,
        "quest_vault.png",
        include_bytes!("../assets/quest_vault.png"),
        12,
        12,
    ),
    slot(
        Art::Flame,
        "icon_flame.png",
        include_bytes!("../assets/icon_flame.png"),
        8,
        8,
    ),
    slot(
        Art::Eye,
        "icon_eye.png",
        include_bytes!("../assets/icon_eye.png"),
        8,
        8,
    ),
    slot(
        Art::Coin,
        "icon_coin.png",
        include_bytes!("../assets/icon_coin.png"),
        8,
        8,
    ),
    slot(
        Art::Skull,
        "icon_skull.png",
        include_bytes!("../assets/icon_skull.png"),
        10,
        10,
    ),
    slot(
        Art::Heart,
        "icon_heart.png",
        include_bytes!("../assets/icon_heart.png"),
        8,
        8,
    ),
];

const fn slot(art: Art, file: &'static str, bytes: &'static [u8], w: u32, h: u32) -> Slot {
    Slot {
        art,
        file,
        bytes,
        texels: PhysicalSize::new(w, h),
    }
}

impl Art {
    /// Every slot, in library order.
    pub const ALL: &'static [Art] = &[
        Art::PortraitAlex,
        Art::PortraitBob,
        Art::PortraitSteve,
        Art::PortraitTim,
        Art::QuestCave,
        Art::QuestCrypt,
        Art::QuestTower,
        Art::QuestVault,
        Art::Flame,
        Art::Eye,
        Art::Coin,
        Art::Skull,
        Art::Heart,
    ];

    fn index(self) -> usize {
        // A linear walk over thirteen entries, so the enum and the table cannot
        // drift the way parallel `as usize` indices do.
        LIBRARY
            .iter()
            .position(|slot| slot.art == self)
            .unwrap_or(0)
    }

    /// The file this role loads from — the name the owner's library replaces.
    pub fn file(self) -> &'static str {
        LIBRARY[self.index()].file
    }

    /// The bytes, compiled in.
    pub fn bytes(self) -> &'static [u8] {
        LIBRARY[self.index()].bytes
    }

    /// The picture's own size in texels.
    pub fn texels(self) -> PhysicalSize {
        LIBRARY[self.index()].texels
    }

    /// How big a quad drawing this at `scale` texels-per-texel is.
    pub fn size_at(self, scale: f32) -> Vec2 {
        let texels = self.texels();
        Vec2::new(texels.width as f32, texels.height as f32) * scale
    }

    /// The portrait for a character.
    ///
    /// Portraits must stay unique per character (UI.md §9), so the mapping is
    /// by name. A roster name this library has no face for takes one by roster
    /// position, which keeps the four distinct rather than collapsing them onto
    /// a default — the game says loudly which faces it has by having exactly
    /// these four, and a fifth character is a new file, not a shared one.
    pub fn portrait_for(name: &str, roster_index: usize) -> Art {
        const FACES: [Art; 4] = [
            Art::PortraitBob,
            Art::PortraitAlex,
            Art::PortraitTim,
            Art::PortraitSteve,
        ];
        match name {
            "Bob" => Art::PortraitBob,
            "Alex" => Art::PortraitAlex,
            "Tim" => Art::PortraitTim,
            "Steve" => Art::PortraitSteve,
            _ => FACES[roster_index % FACES.len()],
        }
    }

    /// The icon for a quest type.
    pub fn for_quest(icon: QuestIcon) -> Art {
        match icon {
            QuestIcon::Cave => Art::QuestCave,
            QuestIcon::Crypt => Art::QuestCrypt,
            QuestIcon::Tower => Art::QuestTower,
            QuestIcon::Vault => Art::QuestVault,
        }
    }
}

/// The asset store giri loads from — the same one the game, the recorder and
/// the capture path build, so all three sample the same textures.
pub fn store() -> Assets {
    let mut source = MemorySource::new();
    for slot in LIBRARY {
        match decode_png(slot.bytes) {
            Ok(texture) => source.insert_texture(slot.file, texture),
            // No silent failure: art that stopped decoding is art every sprite
            // draws as a magenta placeholder, which is a fault no assertion
            // over quads can see and which the store itself calls `Ready`.
            // This is the only place giri can notice, so it says so loudly.
            Err(error) => panic!(
                "{}",
                message(
                    "giri's own art no longer decodes",
                    &format!("{}: {error}", slot.file),
                    "a file under games/giri/assets/ is not a PNG this engine reads",
                    "run games/giri/art/make_placeholders.py to rewrite the placeholders, or \
                     re-import the library file",
                )
            ),
        }
    }
    Assets::new(source)
}

/// Every handle, by role.
#[derive(Clone, Debug)]
pub struct Gallery {
    handles: Vec<TextureHandle>,
}
impl Resource for Gallery {}

impl Gallery {
    /// Ask the store for every role, in library order.
    pub fn load(assets: &mut Assets) -> Self {
        Self {
            handles: LIBRARY
                .iter()
                .map(|slot| assets.load_texture(slot.file))
                .collect(),
        }
    }

    /// The handle for a role.
    pub fn handle(&self, art: Art) -> TextureHandle {
        self.handles[art.index()]
    }

    /// The sprite that draws `art` at `scale`, on `layer`, tinted `tint`.
    ///
    /// Anchored top-left, because every rectangle in `layout.rs` is stated as
    /// a top-left corner and a size, and an icon placed from its centre is an
    /// icon whose position moves when its scale does.
    pub fn sprite(&self, art: Art, scale: f32, layer: i16, tint: Color) -> Sprite {
        Sprite {
            size: art.size_at(scale),
            anchor: Vec2::new(-0.5, -0.5),
            tint,
            layer,
            ..Sprite::new(self.handle(art))
        }
    }
}
