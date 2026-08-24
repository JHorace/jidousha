//! giri's art library: one role, one file, one handle (UI.md §2, §9).
//!
//! **The role is the contract, not the picture.** Every asset is named for
//! what it *means* — `icon_flame` is desperation, `portrait_tim` is Tim — so a
//! library replaces the files by name and no code here changes (DESIGN §7's
//! curation model; UI.md §9's asset slots). Nothing is downloaded:
//! the owner supplies a pack and a committed script curates from it, or
//! `art/make_art.py` writes a PNG from committed grids — and `assets/CREDITS.md`
//! records the provenance of every one.
//!
//! That claim was tested on 2026-08-23: the owner's Kenney packs replaced twelve
//! of the thirteen slots and **no code here changed except the texel sizes in
//! `LIBRARY`**. Which pack region fills which role is
//! `art/kenney-manifest.json`; the thirteenth slot, the infamy eye, is still
//! generated from the grids in `art/sprite_defs.py`, because no eye glyph exists
//! in any of the packs. Both paths stay live, and a change to how giri looks is
//! a change to whichever of the two owns the slot.
//!
//! **The art is a directory, and the directory is giri's own.**
//! `games/giri/assets/` is this crate's asset root: the game loads from it
//! through `asset_source` and the paths below are the same strings on native
//! and on the web, because `tools/build-web` stages this directory under the
//! game's page at the path it is named by (ADR-0040). Adding a picture is
//! adding a file — no rebuild of a byte table, no fifty-entry `include_bytes!`,
//! and no art that cannot travel with the game that owns it (FINDINGS G-005).
//!
//! **The store decodes.** `load_texture` resolves the file's bytes and the
//! engine decodes them through its one PNG path, so a file that stops being a
//! picture resolves `Failed` and is reported at the commit — there is no state
//! where the store calls a texture ready and a sprite draws the magenta
//! placeholder (assets.md §3, §6; FINDINGS G-006). This file therefore has no
//! decoder in it and nothing to panic about.
//!
//! Sizes are native texels. Everything is drawn at an **integer multiple** of
//! them — the engine samples nearest with no filtering, and a pixel-art icon at
//! a fractional scale is a pixel-art icon with a wobble in it (UI.md §1.4).

use jidousha::prelude::*;

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

/// giri's asset root: this crate's own `assets/` directory.
///
/// Named from the workspace root, which is where the native loader resolves it
/// from and where `tools/build-web` mirrors it from under the page — one string
/// on both platforms (assets.md §2 CONTRACT, ADR-0040).
pub const ASSET_ROOT: &str = "games/giri/assets";

/// How many polls [`settle`] gives the loader before calling the art absent.
///
/// Generous: thirteen small files off one loader thread finish in the first few
/// polls, and a number this size can only be reached by a fault.
const SETTLE_POLLS: usize = 100_000;

/// One entry: the role, the file it lives in, and its texel size.
struct Slot {
    art: Art,
    file: &'static str,
    texels: PhysicalSize,
}

/// The library, in the order `Gallery` indexes it.
const LIBRARY: &[Slot] = &[
    slot(Art::PortraitAlex, "portrait_alex.png", 16, 16),
    slot(Art::PortraitBob, "portrait_bob.png", 16, 16),
    slot(Art::PortraitSteve, "portrait_steve.png", 16, 16),
    slot(Art::PortraitTim, "portrait_tim.png", 16, 16),
    slot(Art::QuestCave, "quest_cave.png", 8, 8),
    slot(Art::QuestCrypt, "quest_crypt.png", 8, 8),
    slot(Art::QuestTower, "quest_tower.png", 8, 8),
    slot(Art::QuestVault, "quest_vault.png", 16, 16),
    slot(Art::Flame, "icon_flame.png", 8, 8),
    slot(Art::Eye, "icon_eye.png", 8, 8),
    slot(Art::Coin, "icon_coin.png", 8, 8),
    slot(Art::Skull, "icon_skull.png", 8, 8),
    slot(Art::Heart, "icon_heart.png", 8, 8),
];

const fn slot(art: Art, file: &'static str, w: u32, h: u32) -> Slot {
    Slot {
        art,
        file,
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

    /// The picture's own size in texels.
    pub fn texels(self) -> PhysicalSize {
        LIBRARY[self.index()].texels
    }

    /// How big a quad drawing this at `scale` texels-per-texel is.
    pub fn size_at(self, scale: f32) -> Vec2 {
        let texels = self.texels();
        Vec2::new(texels.width as f32, texels.height as f32) * scale
    }

    /// The scale that draws this art `units` reference pixels across.
    ///
    /// **The drawn size is the contract, not the scale.** One slot family is
    /// filled from two packs at two texel sizes — the quest icons are 8x8 from
    /// Micro Roguelike and 16x16 from Tiny Dungeon (`art/kenney-manifest.json`)
    /// — and a single shared scale would draw them at two different sizes in
    /// the same row. Stating the size the row wants and deriving each art's
    /// scale keeps the row even, and keeps every scale a whole number.
    ///
    /// # Panics
    ///
    /// If `units` is not a whole multiple of this art's texel width. That is a
    /// layout constant and an asset disagreeing, and the fix is one of the two
    /// numbers — never a picture drawn at a fractional scale, which the engine
    /// samples nearest and so draws with a wobble in it (UI.md §1.4). The
    /// readability floors assert the same thing over every icon actually drawn;
    /// this catches it at the call site, where the wrong number is.
    pub fn scale_across(self, units: f32) -> f32 {
        let texels = self.texels().width as f32;
        let scale = units / texels;
        if !crate::checks::near(scale, scale.round()) {
            panic!(
                "{}",
                message(
                    "a pixel-art icon would be drawn at a fractional scale",
                    &format!(
                        "{:?} is {texels} texels across and the layout asked for {units} units, \
                         which is {scale} texels per texel",
                        self
                    ),
                    "a layout constant and an asset size that are not whole multiples",
                    "pick a size that is a multiple of every art in the row, or re-import \
                     the odd one at a size that fits",
                )
            )
        }
        scale.round()
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
///
/// One `asset_source` and no `cfg`: the platform crate picks the loader, and
/// [`ASSET_ROOT`] means the same directory on both (ADR-0040). Nothing here can
/// fail at call time — a file that is missing or is no longer a picture
/// resolves `Failed` at the commit that answers it, with the engine's message
/// naming the line that asked (assets.md §6).
pub fn store() -> Assets {
    Assets::new(asset_source(ASSET_ROOT))
}

/// Poll `assets` until every load it was asked for has resolved, at tick 1.
///
/// **For the runs that must not depend on a disk**: `--verify` asserts on draw
/// transcripts and the capture path replays plans through a real GPU, and both
/// would otherwise photograph whatever the loader thread had finished by then.
/// Committing the same tick repeatedly is legal and moves nothing on the
/// timeline (assets.md §4), so every texture becomes ready at tick 1 exactly as
/// a scripted store would make it — the run is reproducible again.
///
/// Not for the game: a window and a browser both draw the placeholder for the
/// frame or two the art takes, which is the engine's whole loading policy, and
/// a browser cannot spin on a `fetch` at all.
///
/// Returns what failed, for a caller to report; the failures are already
/// §6-shaped.
///
/// # Panics
///
/// If the loads have not resolved after [`SETTLE_POLLS`] polls. That is not a
/// slow disk — it is a loader that has stopped — and a capture written from a
/// half-loaded store is a picture nobody can read a verdict off.
pub fn settle(assets: &mut Assets) -> Vec<AssetFailure> {
    let mut failures = Vec::new();
    for _ in 0..SETTLE_POLLS {
        failures.extend(assets.commit(1));
        if assets.all_ready() {
            return failures;
        }
        // The loader is on another thread; there is nothing to do but let it
        // have the processor. No clock is read — the engine forbids one, and a
        // count of polls is what this needs anyway.
        std::thread::yield_now();
    }
    panic!(
        "{}",
        message(
            "giri's art never finished loading",
            &format!("{SETTLE_POLLS} polls of the asset loader and something is still in flight"),
            "the loader thread stopped, or the filesystem is not answering",
            &format!(
                "run `tools/check-assets` to confirm every file under {ASSET_ROOT}/ is there, \
                 then `tools/doctor`"
            ),
        )
    )
}

/// Every handle, by role.
#[derive(Clone, Debug)]
pub struct Gallery {
    handles: Vec<TextureHandle>,
}
impl Resource for Gallery {}

impl Gallery {
    /// Ask the store for every role, in library order.
    ///
    /// **Written out, one literal per role**, because an asset path is a string
    /// literal at the load site (assets.md §2): that is what lets
    /// `tools/check-assets` prove, before the game runs, that all thirteen name
    /// files that exist with exactly this spelling. A fold over `LIBRARY` would
    /// be shorter and would make every path invisible to the check that turns a
    /// typo into a CI failure instead of a magenta quad.
    ///
    /// The order is `LIBRARY`'s, because [`handle`](Gallery::handle) indexes by
    /// it; `library.rs`'s art-library contract asserts the two lists against
    /// each other, so they cannot drift.
    pub fn load(assets: &mut Assets) -> Self {
        Self {
            handles: vec![
                assets.load_texture("portrait_alex.png"),
                assets.load_texture("portrait_bob.png"),
                assets.load_texture("portrait_steve.png"),
                assets.load_texture("portrait_tim.png"),
                assets.load_texture("quest_cave.png"),
                assets.load_texture("quest_crypt.png"),
                assets.load_texture("quest_tower.png"),
                assets.load_texture("quest_vault.png"),
                assets.load_texture("icon_flame.png"),
                assets.load_texture("icon_eye.png"),
                assets.load_texture("icon_coin.png"),
                assets.load_texture("icon_skull.png"),
                assets.load_texture("icon_heart.png"),
            ],
        }
    }

    /// The path each handle was asked for, in library order.
    ///
    /// What `library.rs`'s contract compares against `LIBRARY`: the explicit
    /// load list above and the table are two orders, and only an assertion
    /// keeps them one.
    pub fn paths<'a>(&self, assets: &'a Assets) -> Vec<&'a str> {
        self.handles
            .iter()
            .map(|handle| assets.path_of(*handle))
            .collect()
    }

    /// Every role's file, in library order — the list `paths` must equal.
    pub fn library_files() -> Vec<&'static str> {
        LIBRARY.iter().map(|slot| slot.file).collect()
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
