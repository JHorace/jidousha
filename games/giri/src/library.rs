//! The art library and every string the game draws — the two contracts about
//! what giri *shows* that need no frame to check.
//!
//! Both are walked rather than listed. The library is walked off `Art::ALL`, so
//! a role added without a file is a failure rather than a magenta quad; the
//! strings are walked off `screens::content` for every beat and every screen,
//! so a row added without a thought about the font is a failure rather than a
//! box on the screen. A hand-kept list of either covers what somebody
//! remembered, and the entry that goes wrong is always the one added after the
//! list.

use crate::beats::CHAIN;
use crate::checks::Checks;
use crate::constants::{Field, Tuning};
use crate::flow::{Flow, Preview};
use crate::model::Social;
use crate::screens;
use crate::sprites::{self, Art};
use crate::verify::play;

/// **The art library** (UI.md §2, §9; DESIGN §7's curation model).
///
/// The library is a contract about *roles*, and a role is only a role if it has
/// a file of its own: two characters sharing a portrait would break "portraits
/// must remain unique per character" silently, and every other check in this
/// game would go on passing. So the files are checked for distinctness, for
/// being pictures at all, and for being the size the code says they are — which
/// is the check that catches a file being replaced by one at a different
/// resolution - by a hand edit to a grid, or by a library arriving one day.
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
            format!("{file:?} - DESIGN §7's curation model names the shape"),
        );
    }

    // The files themselves, through the store the game loads from. Reading them
    // any other way would be a second decoder in a game that no longer has one:
    // a file that is not a picture, or is one the engine will not take, resolves
    // `Failed` here exactly as it would on a player's machine (assets.md §6,
    // FINDINGS G-006).
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
            // Already reported as a failure above; saying it twice per role
            // would bury the message that names the file.
            continue;
        };
        checks.require(
            texture.width == art.texels().width && texture.height == art.texels().height,
            "an art file is not the size the game says it is",
            format!(
                "{file:?} is {}x{} on disk and {}x{} in the library; every icon is placed and \
                 scaled from the library's number",
                texture.width,
                texture.height,
                art.texels().width,
                art.texels().height
            ),
        );
        checks.require(
            texture.width <= 2048 && texture.height <= 2048,
            "an art file is larger than the curation model allows",
            format!(
                "{file:?} is {}x{} and individual PNGs stay at or under 2048",
                texture.width, texture.height
            ),
        );
    }

    // Every signifier UI.md §2's table names has a role, and every character on
    // every beat's roster has a portrait nobody else has.
    let mut faces: Vec<(&'static str, Art)> = Vec::new();
    for spec in CHAIN {
        for (index, character) in spec.roster.iter().enumerate() {
            let face = Art::portrait_for(character.name, index);
            if let Some((other, _)) = faces
                .iter()
                .find(|(name, art)| *art == face && *name != character.name)
            {
                checks.require(
                    false,
                    "two characters share a portrait",
                    format!(
                        "{} and {other} both draw {:?}; UI.md §9 says portraits stay unique \
                         per character",
                        character.name, face
                    ),
                );
            }
            faces.push((character.name, face));
        }
    }
}

/// Every string the game draws, in characters the font can draw.
///
/// **Walked off the screens rather than listed.** `screens::content` is the
/// whole of what a screen says, so playing every beat and asking every screen
/// for its panel covers every drawn row by construction — a hand-kept list
/// covers the rows somebody remembered, and the row that grows an em dash is
/// always the one that was added after the list.
pub fn printable_strings(checks: &mut Checks) {
    let mut strings: Vec<(String, String)> = Vec::new();
    let mut note = |what: String, text: String| strings.push((what, text));
    note(
        "the constants readout".to_owned(),
        Tuning::SHIPPED.readout(),
    );
    for (index, spec) in CHAIN.iter().enumerate() {
        let beat = index + 1;
        note(format!("beat {beat}'s title"), spec.title.to_owned());
        note(format!("beat {beat}'s dilemma"), spec.dilemma.to_owned());
        note(format!("beat {beat}'s lesson"), spec.teaches.to_owned());
        for character in spec.roster {
            note(format!("beat {beat}'s roster"), character.name.to_owned());
        }
        for dungeon in spec.dungeons {
            note(format!("beat {beat}'s job"), crate::job_line(dungeon));
            note(format!("beat {beat}'s blurb"), dungeon.blurb.to_owned());
            note(
                format!("beat {beat}'s requirement"),
                dungeon.requires.describe(),
            );
            note(
                format!("beat {beat}'s shortfall"),
                dungeon.requires.shortfall().to_owned(),
            );
        }
        let played = play(index, Tuning::SHIPPED, false);
        for (mode, flow, social, preview) in [
            (
                "board",
                &played.board_flow,
                &played.at_assembly,
                &played.board_preview,
            ),
            (
                "staged board",
                &played.ready_flow,
                &played.at_assembly,
                &played.ready,
            ),
            (
                "takeover",
                &played.report_flow,
                &played.after,
                &played.report_preview,
            ),
        ] {
            for text in screens::content(flow, social, preview, &played.tuning).strings() {
                note(format!("beat {beat}'s {mode}"), text.to_owned());
            }
        }
        // The drawer and the toast, which no frame above is on: the log is
        // where a bounce is kept, and a toast is the one string a player reads
        // that the board never redraws.
        let mut opened = played.report_flow.clone();
        opened.log_open = true;
        opened.stage = crate::flow::Stage::Board;
        for text in screens::content(
            &opened,
            &played.after,
            &played.report_preview,
            &played.tuning,
        )
        .strings()
        {
            note(format!("beat {beat}'s log drawer"), text.to_owned());
        }
        // The tuning drawer, with a hovered row and a pending set that differs
        // from the active one, so its two conditional rows are on the list too
        // (a constant's meaning, and the sentence an APPLY raises).
        let mut tuner = played.report_flow.clone();
        tuner.stage = crate::flow::Stage::Board;
        tuner.log_open = false;
        tuner.tuner.open = true;
        tuner.tuner.hover = Field::ALL.first().copied();
        tuner.tuner.pending = crate::presets::PRESETS
            .last()
            .map_or(played.tuning, |preset| preset.tuning);
        for text in screens::content(
            &tuner,
            &played.after,
            &played.report_preview,
            &played.tuning,
        )
        .strings()
        {
            note(format!("beat {beat}'s tuning drawer"), text.to_owned());
        }
        for bounce in [&played.refusal, &played.veto].into_iter().flatten() {
            if let Some(toast) = &bounce.toast {
                note(format!("beat {beat}'s bounce"), toast.clone());
            }
            if let Some(line) = &bounce.logged {
                note(format!("beat {beat}'s log line"), line.clone());
            }
        }
        for line in &played.report {
            note(format!("beat {beat}'s narration"), line.clone());
        }
    }
    // And the screen the chain reaches once.
    let done = Flow {
        beat: CHAIN.len(),
        stage: crate::flow::Stage::Complete,
        ..Flow::default()
    };
    for text in screens::content(
        &done,
        &Social::default(),
        &Preview::default(),
        &Tuning::SHIPPED,
    )
    .strings()
    {
        note("the end of the chain".to_owned(), text.to_owned());
    }
    // Every reason a `?constants=` link is refused, since each is a string the
    // page draws and none is reachable by playing (UI.md §12).
    for message in crate::links::refusals() {
        note("a refused ?constants=".to_owned(), message);
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
