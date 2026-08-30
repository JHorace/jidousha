//! The art library and every string the game draws — the two contracts about
//! what ninjo *shows* that need no frame to check.
//!
//! Both are walked rather than listed. The library is walked off `Art::ALL`,
//! so a role added without a file is a failure rather than a magenta quad;
//! the strings are walked off `screens::content` for every screen state the
//! floors already build, plus everything only a played run produces — a
//! hand-kept list covers what somebody remembered, and the entry that goes
//! wrong is always the one added after the list.

use crate::checks::Checks;
use crate::grid::LOCATIONS;
use crate::sim::Sim;
use crate::sprites::{self, Art};
use crate::sweep::Conducted;
use crate::{clock, floors, screens};

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
    let parties = Sim::opening().parties;
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

/// Every string the game draws, in characters the font can draw.
///
/// Walked off the same screen states the floors judge, plus the event lines
/// a whole conducted run produced, the refusal grammar, and the link
/// refusals — every drawn row by construction.
pub fn printable_strings(checks: &mut Checks, baseline: &Conducted) {
    let mut strings: Vec<(String, String)> = Vec::new();
    let mut note = |what: String, text: String| strings.push((what, text));

    for (what, flow, sim, clock) in floors::content_states(baseline) {
        for text in
            screens::content(&flow, &sim, &clock, &crate::constants::Tuning::SHIPPED).all_strings()
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
    for text in screens::content(
        &tuner,
        &Sim::opening(),
        &crate::clock::Clock::opening(),
        &crate::constants::Tuning::SHIPPED,
    )
    .all_strings()
    {
        note("the tuning drawer".to_owned(), text.to_owned());
    }
    for message in crate::links::refusals() {
        note("a refused link".to_owned(), message);
    }
    for event in &baseline.events {
        note("an event line".to_owned(), event.line(&baseline.sim));
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
    for quest in Sim::opening().sites.iter().flat_map(|site| &site.quests) {
        note("a quest name".to_owned(), quest.name.to_owned());
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
