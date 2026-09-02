//! The art library and every string the game draws — the two contracts about
//! what ninjo *shows* that need no frame to check.
//!
//! Both are walked rather than listed. The library is walked off `Art::ALL`,
//! so a role added without a file is a failure rather than a magenta quad;
//! the strings are walked off `screens::content` for every screen state the
//! floors already build, plus everything only a played run produces — a
//! hand-kept list covers what somebody remembered, and the entry that goes
//! wrong is always the one added after the list.

use jidousha::prelude::*;

use crate::checks::Checks;
use crate::constants::Tuning;
use crate::grid::LOCATIONS;
use crate::lens::Lens;
use crate::sim::Sim;
use crate::sprites::{self, Art};
use crate::sweep::Conducted;
use crate::{clock, floors, screens, theme};

/// **The art library** (giri's curation model, carried whole): every role a
/// distinct file, every file a picture of the size the code says, every
/// marker and token distinct.
pub fn library(checks: &mut Checks) {
    let mut files: Vec<&'static str> = Vec::new();
    for art in Art::ALL.iter().copied() {
        let file = art.file();
        checks.require(
            !files.contains(&file),
            "two roles in the art library share a file",
            format!("{file:?} is named by more than one role, and a role is one picture"),
        );
        files.push(file);
        checks.require(
            file.ends_with(".png")
                && file
                    .trim_end_matches(".png")
                    .chars()
                    .all(|glyph| glyph.is_ascii_lowercase() || glyph == '_'),
            "an art file is not role-named lowercase snake_case",
            format!("{file:?} - the curation model names the shape"),
        );
    }

    // The files themselves, through the store the game loads from.
    let mut assets = sprites::store();
    let gallery = sprites::Gallery::load(&mut assets);
    for failure in sprites::settle(&mut assets) {
        checks.require(
            false,
            "an art file did not load",
            crate::checks::one_line(&failure.message()),
        );
    }
    checks.require(
        gallery.paths(&assets) == sprites::Gallery::library_files(),
        "the loads and the library table are in different orders",
        format!(
            "the store was asked for {:?} and the table reads {:?}; `Gallery::handle` indexes \
             one by the other",
            gallery.paths(&assets),
            sprites::Gallery::library_files()
        ),
    );
    for art in Art::ALL.iter().copied() {
        let file = art.file();
        let Some(texture) = assets.texture_of(gallery.handle(art)) else {
            continue;
        };
        checks.require(
            texture.width == art.texels().width && texture.height == art.texels().height,
            "an art file is not the size the game says it is",
            format!(
                "{file:?} is {}x{} on disk and {}x{} in the library",
                texture.width,
                texture.height,
                art.texels().width,
                art.texels().height
            ),
        );
        checks.require(
            texture.width <= 2048 && texture.height <= 2048,
            "an art file is larger than the curation model allows",
            format!("{file:?} is {}x{}", texture.width, texture.height),
        );
    }

    portraits_are_tellable_apart(checks, &assets, &gallery);

    // Every location draws a marker nobody else draws, and every party a
    // token nobody else carries — a marker is an identity.
    let mut markers: Vec<(&'static str, Art)> = Vec::new();
    for spec in LOCATIONS {
        let art = Art::for_icon(spec.icon);
        if let Some((other, _)) = markers.iter().find(|(_, used)| *used == art) {
            checks.require(
                false,
                "two locations share a marker icon",
                format!("{} and {other} both draw {art:?}", spec.name),
            );
        }
        markers.push((spec.name, art));
    }
    let opening = Sim::opening(&Tuning::SHIPPED, crate::modules::ModuleSet::ALL);
    let parties = &opening.parties;
    for (index, party) in parties.iter().enumerate() {
        for other in parties.iter().skip(index + 1) {
            checks.require(
                party.token != other.token,
                "two parties share a token portrait",
                format!(
                    "{} and {} both draw {:?}",
                    party.name, other.name, party.token
                ),
            );
        }
    }
}

/// How far apart two texels have to be, per channel, to count as different.
///
/// A shipped literal, never derived from the art: a floor computed from the
/// pictures it judges cannot see them move (make-game §A.6). 24 of 255 is
/// wider than the palette's own neighbouring shades and narrower than any two
/// of Tiny Dungeon's colour roles.
const PORTRAIT_TEXEL_DIFFERENCE: i32 = 24;

/// What share of a portrait's texels must differ from every other portrait's.
///
/// Measured over the ten committed on 2026-09-02: the tightest pair is
/// `portrait_tim` and `portrait_odd` — a closed helm and an open one — at 19%,
/// and every other pair is above 20%. The floor sits at 15% so the landed set
/// clears it with room, and a future `chosen` edit that picks a near-duplicate
/// does not: the two busts rejected for being too alike scored 12%
/// (CAST.md §9).
const PORTRAIT_DISTINCT_FLOOR: f32 = 0.15;

/// **No two people wear the same face at map scale** (CAST.md §9's criterion).
///
/// The eye is the gate and this is the floor under it: a portrait is a click
/// target standing at a home tile beside nine others, and two that differ only
/// in a detail are two people the player cannot tell apart. Judged at *native
/// texel size* over the ground colour, because that is where the difference has
/// to exist — an integer upscale cannot add one.
///
/// The comparison is per texel over the alpha-composited picture, so a portrait
/// that differs only by transparency still counts as the same picture, which is
/// what it looks like on the map.
fn portraits_are_tellable_apart(checks: &mut Checks, assets: &Assets, gallery: &sprites::Gallery) {
    let ground = theme::GROUND;
    let over_ground = |art: Art| -> Option<Vec<[i32; 3]>> {
        let texture = assets.texture_of(gallery.handle(art))?;
        Some(
            texture
                .rgba
                .chunks_exact(4)
                .map(|texel| {
                    let alpha = i32::from(texel[3]);
                    let base = [ground.r, ground.g, ground.b];
                    let mut out = [0i32; 3];
                    for (channel, slot) in out.iter_mut().enumerate() {
                        let back = (base[channel] * 255.0).round() as i32;
                        *slot = (i32::from(texel[channel]) * alpha + back * (255 - alpha)) / 255;
                    }
                    out
                })
                .collect(),
        )
    };

    let faces: Vec<(Art, Vec<[i32; 3]>)> = Art::ALL
        .iter()
        .copied()
        .filter(|art| art.file().starts_with("portrait_"))
        .filter_map(|art| over_ground(art).map(|texels| (art, texels)))
        .collect();

    for (index, (art, texels)) in faces.iter().enumerate() {
        for (other, others) in faces.iter().skip(index + 1) {
            if texels.len() != others.len() {
                // Two portraits at two texel sizes are trivially tellable apart,
                // and the size check above already has an opinion about it.
                continue;
            }
            let differing = texels
                .iter()
                .zip(others)
                .filter(|(mine, theirs)| {
                    mine.iter()
                        .zip(theirs.iter())
                        .any(|(a, b)| (a - b).abs() > PORTRAIT_TEXEL_DIFFERENCE)
                })
                .count();
            let share = differing as f32 / texels.len() as f32;
            checks.require(
                share >= PORTRAIT_DISTINCT_FLOOR,
                "two portraits are too alike to tell apart at map scale",
                format!(
                    "{art:?} and {other:?} differ on {differing} of {} texels ({:.0}%), and the \
                     floor is {:.0}% - a `chosen` edit in art/kenney-manifest.json picked two \
                     faces that read as one person on the map",
                    texels.len(),
                    share * 100.0,
                    PORTRAIT_DISTINCT_FLOOR * 100.0,
                ),
            );
        }
    }
}

/// Every string the game draws, in characters the font can draw.
///
/// Walked off the same screen states the floors judge, plus the event lines
/// a whole conducted run produced, the refusal grammar, and the link
/// refusals — every drawn row by construction.
pub fn printable_strings(checks: &mut Checks, baseline: &Conducted) {
    let mut strings: Vec<(String, String)> = Vec::new();
    let mut note = |what: String, text: String| strings.push((what, text));

    for (what, flow, sim, clock) in floors::content_states(baseline) {
        for text in screens::content(&flow, &Lens::on(&sim), &clock, &Tuning::SHIPPED).all_strings()
        {
            note(format!("{what}'s screen"), text.to_owned());
        }
    }
    // The tuning drawer, hovered and pending, and with the longest refusal.
    let mut tuner = crate::flow::Flow::default();
    tuner.tuner.open = true;
    tuner.tuner.hover = crate::constants::Field::ALL.first().copied();
    tuner.tuner.pending = crate::presets::PRESETS
        .last()
        .map_or(crate::constants::Tuning::SHIPPED, |preset| preset.tuning);
    let opening = Sim::opening(&Tuning::SHIPPED, crate::modules::ModuleSet::ALL);
    for text in screens::content(
        &tuner,
        &Lens::on(&opening),
        &crate::clock::Clock::opening(),
        &Tuning::SHIPPED,
    )
    .all_strings()
    {
        note("the tuning drawer".to_owned(), text.to_owned());
    }
    for message in crate::links::refusals() {
        note("a refused link".to_owned(), message);
    }
    {
        let lens = Lens::on(&baseline.sim);
        for event in &baseline.events {
            note("an event line".to_owned(), event.line(&lens));
        }
    }
    for refusal in [
        crate::sim::Refusal::NotIdle,
        crate::sim::Refusal::Dry,
        crate::sim::Refusal::Unreachable,
    ] {
        note(
            "a refusal".to_owned(),
            refusal.message("CRANE", "the Black Vault"),
        );
    }
    note("the opening stamp".to_owned(), clock::stamp(0));
    note("a late stamp".to_owned(), clock::stamp(baseline.minutes));
    for quest in opening.sites.iter().flat_map(|site| &site.quests) {
        note("a quest name".to_owned(), quest.name.to_owned());
    }
    // The people's own strings: names, ids, and the source lines that make two
    // identical desperations two different problems.
    for person in &opening.people {
        note("a character's name".to_owned(), person.name.to_owned());
        note("a character's id".to_owned(), person.id.to_owned());
        note("a desperation source".to_owned(), person.source.to_owned());
    }
    for def in crate::traits::TRAITS {
        note("a trait's name".to_owned(), def.name.to_owned());
        note("a trait's description".to_owned(), def.line.to_owned());
    }
    for mark in crate::traits::MarkId::ALL.iter().copied() {
        note("a mark's name".to_owned(), mark.name().to_owned());
    }

    for (what, text) in &strings {
        let stray = text
            .chars()
            .find(|glyph| *glyph != '\n' && !(' '..='~').contains(glyph));
        checks.require(
            stray.is_none(),
            "a string the game draws has a character the font cannot draw",
            format!(
                "{what} contains {stray:?} in {text:?}; it draws as a box at exactly a \
                 letter's width, and no assertion over what was drawn can tell the difference"
            ),
        );
    }
}
