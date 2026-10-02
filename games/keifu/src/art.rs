//! The cast's sprites: which file plays which of the original's sprite roles, and
//! loading them.
//!
//! `lore.json` and `household.json` name the original's sprites
//! (`character-knight`, `reward-sword`, ...). Those names are a role list, not
//! files: the original's art is not reused (owner's decision, 2026-10-01). Each
//! role is played by a sprite imported from the asset depot into
//! `games/keifu/assets/` (`art/import_sprites.py`, credited in `CREDITS.md`).
//! Presentation only — no rule reads any of this.

use jidousha::prelude::*;

use crate::content::Content;
use crate::hero::Hero;
use crate::ids::Phase;

/// Where this game's art lives, from the top of the repository (ADR-0040).
pub const ASSET_ROOT: &str = "games/keifu/assets";

/// A sprite role this build draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Figure {
    /// `character-knight`.
    Knight,
    /// `character-warrior`.
    Warrior,
    /// `character-guard`, the original's ranger.
    Ranger,
    /// `character-scholar`.
    Scholar,
    /// `character-priest`.
    Priest,
    /// `character-sage`.
    Sage,
    /// `character-grandpa`: an elder knight, warrior or ranger.
    Elder,
    /// `character-hermit`: an elder scholar, priest or sage.
    Hermit,
    /// `character-boy`: every child.
    Child,
    /// `reward-sword`: a blade heirloom.
    Blade,
    /// `reward-guidebook`: a road-book heirloom.
    RoadBook,
    /// `reward-ring`: a cradle-ring heirloom.
    Ring,
}

/// The original's sprite names, and the role each plays here.
pub const ROLES: [(&str, Figure); 12] = [
    ("character-knight", Figure::Knight),
    ("character-warrior", Figure::Warrior),
    ("character-guard", Figure::Ranger),
    ("character-scholar", Figure::Scholar),
    ("character-priest", Figure::Priest),
    ("character-sage", Figure::Sage),
    ("character-grandpa", Figure::Elder),
    ("character-hermit", Figure::Hermit),
    ("character-boy", Figure::Child),
    ("reward-sword", Figure::Blade),
    ("reward-guidebook", Figure::RoadBook),
    ("reward-ring", Figure::Ring),
];

/// The role an original sprite name plays, if this build draws it.
pub fn figure_named(name: &str) -> Option<Figure> {
    ROLES
        .iter()
        .find(|(original, _)| *original == name)
        .map(|(_, figure)| *figure)
}

/// The store a windowed run reads the art from: the game's own asset root.
///
/// Built here, beside the loads, so `tools/check-assets` — which checks the loads
/// of a file that builds a filesystem-backed store — sees every path below.
pub fn store() -> Assets {
    Assets::new(asset_source(ASSET_ROOT))
}

/// The loaded textures, one per role, in `ROLES` order.
pub struct Art {
    textures: Vec<(Figure, TextureHandle)>,
}

impl Resource for Art {}

impl Art {
    /// Ask for every role's file. Each path is a literal, so `tools/check-assets`
    /// can see it; the order is `ROLES`'s, which the capture replays.
    pub fn load(assets: &mut Assets) -> Self {
        Self {
            textures: vec![
                (Figure::Knight, assets.load_texture("hero_knight.png")),
                (Figure::Warrior, assets.load_texture("hero_warrior.png")),
                (Figure::Ranger, assets.load_texture("hero_ranger.png")),
                (Figure::Scholar, assets.load_texture("hero_scholar.png")),
                (Figure::Priest, assets.load_texture("hero_priest.png")),
                (Figure::Sage, assets.load_texture("hero_sage.png")),
                (Figure::Elder, assets.load_texture("hero_elder.png")),
                (Figure::Hermit, assets.load_texture("hero_hermit.png")),
                (Figure::Child, assets.load_texture("hero_child.png")),
                (Figure::Blade, assets.load_texture("heirloom_blade.png")),
                (
                    Figure::RoadBook,
                    assets.load_texture("heirloom_road_book.png"),
                ),
                (Figure::Ring, assets.load_texture("heirloom_ring.png")),
            ],
        }
    }

    /// The texture that plays `figure`.
    pub fn texture(&self, figure: Figure) -> TextureHandle {
        match self.textures.iter().find(|(f, _)| *f == figure) {
            Some((_, handle)) => *handle,
            None => panic!(
                "[keifu] no texture plays {figure:?}\n  likely cause: a role was added to \
                 Figure without a load in Art::load\n  fix: load its file there"
            ),
        }
    }
}

/// The original sprite name a hero is drawn with (SPEC §5.4, "sprite by
/// phase/vocation"): the child sprite for a child, the vocation's elder sprite for
/// an elder, the vocation's sprite otherwise.
pub fn sprite_name<'c>(content: &'c Content, hero: &Hero) -> &'c str {
    let [sprite, elder] = &content.lore.vocation_sprites[hero.vocation.index()];
    match hero.phase() {
        Phase::Child => &content.lore.child_sprite,
        Phase::Elder => elder,
        _ => sprite,
    }
}

/// The role a hero is drawn as.
pub fn hero_figure(content: &Content, hero: &Hero) -> Figure {
    let name = sprite_name(content, hero);
    match figure_named(name) {
        Some(figure) => figure,
        None => panic!(
            "[keifu] {} is drawn with {name:?}, which no imported sprite plays\n  likely \
             cause: lore.json names a sprite art.rs's ROLES does not list\n  fix: import a \
             sprite for it (art/import_sprites.py) and add the role",
            hero.name
        ),
    }
}
