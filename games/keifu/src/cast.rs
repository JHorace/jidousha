//! The cast's pictures, checked: every sprite role the content names is imported,
//! every card samples its role's texture once it has arrived, and the tints follow
//! SPEC §5.4.
//!
//! INVARIANT: as in `oracles.rs`, every expectation is a shipped literal — who plays
//! whom is read off `lore.json`'s vocations by hand.

use jidousha::prelude::*;
use jidousha::testing::FrameRecorder;

use crate::checks::Checks;
use crate::house::House;
use crate::screen::{Target, ink};
use crate::verify::{content_of, hero_named, session};

fn near(a: Rect, b: Rect) -> bool {
    (a.min - b.min).length() < 0.01 && (a.max - b.max).length() < 0.01
}

/// The cast's sprites: every sprite name the content draws a hero or heirloom with
/// has an imported role; every card on the summer screen samples its role's texture,
/// arrived rather than the placeholder; the tints follow SPEC §5.4; a child stands
/// smaller; and no two roles share a picture.
pub fn check_art(checks: &mut Checks, recorder: &mut FrameRecorder) -> String {
    use crate::art::{Art, Figure, figure_named};
    let mut sim = session(crate::verify::SEEDS[0]);
    let content = content_of(&sim);
    let mut names: Vec<&str> = content
        .lore
        .vocation_sprites
        .iter()
        .flat_map(|pair| pair.iter().map(String::as_str))
        .collect();
    names.push(&content.lore.child_sprite);
    let heirlooms: Vec<String> = sim
        .world()
        .resource::<House>()
        .heroes
        .iter()
        .filter_map(|h| h.heirloom.as_ref().map(|x| x.sprite.clone()))
        .collect();
    let unmapped: Vec<String> = names
        .iter()
        .map(|n| (*n).to_owned())
        .chain(heirlooms)
        .filter(|n| figure_named(n).is_none())
        .collect();
    checks.require(
        unmapped.is_empty(),
        "every sprite the content names has an imported role",
        format!("unmapped: {unmapped:?}"),
    );
    let pictures: Vec<&[u8]> = crate::verify::ART_FILES.iter().map(|(_, b)| *b).collect();
    let distinct = pictures
        .iter()
        .enumerate()
        .all(|(i, a)| pictures[i + 1..].iter().all(|b| a != b));
    checks.require(distinct, "no two roles share a picture", String::new());

    let frame = crate::verify::frame(recorder, &mut sim);
    let page = crate::verify::page_of(&sim);
    let art = sim.world().resource::<Art>();
    let assets = sim.world().resource::<Assets>();
    let mut sampled = 0;
    for mark in &page.figures {
        let handle = art.texture(mark.figure);
        let want = recorder.texture(handle.texture_id());
        let drawn = frame
            .quads()
            .into_iter()
            .any(|q| q.texture == want && near(q.bounds(), mark.rect) && q.tint == mark.tint);
        checks.require(
            drawn && assets.status(handle) == AssetStatus::Ready,
            "a card draws its role's sprite, arrived, where the page put it",
            format!(
                "{:?} at {:?}: drawn {drawn}, {:?}",
                mark.figure,
                mark.rect,
                assets.status(handle)
            ),
        );
        sampled += 1;
    }
    let card_of = |name: &str| {
        let id = hero_named(&sim, name);
        let card = page
            .targets
            .iter()
            .find(|(_, t)| *t == Target::Hero(id))
            .map(|(r, _)| *r);
        card.and_then(|card| {
            page.figures
                .iter()
                .find(|f| card.contains_rect(f.rect))
                .copied()
        })
    };
    // Shipped literals: who plays whom (lore.json vocations), and the tints of §5.4.
    for (name, figure, tint, side) in [
        ("Garrick", Figure::Elder, ink::GONE, 48.0),
        ("Maren", Figure::Ranger, Color::WHITE, 48.0),
        ("Ysolde", Figure::Scholar, Color::WHITE, 48.0),
        ("Brannoc", Figure::Warrior, Color::WHITE, 48.0),
        ("Odo", Figure::Priest, Color::WHITE, 48.0),
        ("Pip", Figure::Child, Color::WHITE, 32.0),
        ("Wren", Figure::Child, Color::WHITE, 32.0),
    ] {
        let mark = card_of(name);
        checks.require(
            mark.is_some_and(|m| m.figure == figure && m.tint == tint && m.rect.size().x == side),
            "a hero is drawn as their calling and phase, tinted as SPEC §5.4 says",
            format!("{name}: {mark:?}, want {figure:?} tinted {tint:?} at {side}"),
        );
    }
    let mut heroes = sim.world().resource::<House>().heroes.clone();
    let maren = hero_named(&sim, "Maren");
    heroes[maren].wounded = true;
    checks.require(
        crate::summer::figure_tint(&heroes[maren]) == ink::WARN,
        "the wounded are tinted red",
        String::new(),
    );
    heroes[maren].fate = crate::hero::Fate::Dead;
    checks.require(
        crate::summer::figure_tint(&heroes[maren]) == ink::GONE,
        "the dead are tinted grey",
        String::new(),
    );
    format!(
        "art: {sampled} sprites on the summer screen, each its role's texture; 10 roles, distinct"
    )
}
