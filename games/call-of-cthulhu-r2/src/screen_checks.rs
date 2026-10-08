//! What every screen owes, judged on every photograph: the floors, the rows
//! found on the frame, nothing off the camera, printable strings, the clear
//! colour — plus the screens a played run never reaches, staged.

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord};
use jidousha::ui::{Panel, TextRun, frame_text_floor, judge_frame, judge_panel};

use crate::checks::Checks;
use crate::conductor::Conductor;
use crate::lore::{BEINGS, Being, FACTS};
use crate::play::{CallState, Ending, Hangup, Phase, ending_lines};
use crate::screens::{self, Art, FLOORS, Flat, palette};

/// The smallest gap any screen left between a quad and the camera's edge.
pub struct Margins {
    pub clearance: f32,
    pub screens: usize,
}

/// Judge one screen: its panel against the floors, its rows on its frame.
pub fn judge(
    checks: &mut Checks,
    margins: &mut Margins,
    name: &str,
    panel: &Panel<Art>,
    frame: &FrameRecord,
    font: BackendTextureId,
) {
    margins.screens += 1;
    let view = crate::camera().visible_bounds();
    let mut breaches = judge_panel(panel, &FLOORS, &[], &[]);
    breaches.extend(judge_frame(panel, frame, font, &Flat, view));
    breaches.extend(frame_text_floor(frame, font, FLOORS.min_text));
    checks.require(
        breaches.is_empty(),
        "a screen breaks a readability floor, or draws something other than what it says",
        format!(
            "{name}: {}",
            breaches
                .iter()
                .map(|breach| format!("{} ({})", breach.what, breach.detail))
                .collect::<Vec<_>>()
                .join("; ")
        ),
    );
    let quads = frame.quads();
    let off: Vec<Rect> = quads
        .iter()
        .map(|quad| quad.bounds())
        .filter(|bounds| !view.contains_rect(*bounds))
        .collect();
    checks.require(
        off.is_empty(),
        "something was drawn outside what the camera shows",
        format!(
            "{name}: {} of {} quads fall outside {view:?}; the first is {:?}",
            off.len(),
            quads.len(),
            off.first()
        ),
    );
    let clearance = quads
        .iter()
        .map(|quad| {
            let bounds = quad.bounds();
            let gap = (bounds.min - view.min).min(view.max - bounds.max);
            gap.x.min(gap.y)
        })
        .fold(f32::MAX, f32::min);
    margins.clearance = margins.clearance.min(clearance);
    let stray: Vec<String> = panel
        .all_strings()
        .filter(|row| row.chars().any(|glyph| !(' '..='~').contains(&glyph)))
        .map(str::to_owned)
        .collect();
    checks.require(
        stray.is_empty(),
        "a row has a character the font cannot draw",
        format!("{name}: {stray:?} — each draws as a box no quad check can see"),
    );
    let clipped: Vec<String> = panel
        .all_strings()
        .filter(|row| row.ends_with("..."))
        .map(str::to_owned)
        .collect();
    checks.require(
        clipped.is_empty(),
        "a row ran out of room and was clipped",
        format!("{name}: {clipped:?}"),
    );
    let cleared = frame.plan.clear_color;
    let brightness = cleared.r.max(cleared.g).max(cleared.b);
    checks.require(
        cleared == palette::NIGHT && brightness < 0.15 && cleared.a > 0.99,
        "the screen is not cleared to the dark the pale text needs",
        format!("{name}: cleared to {cleared:?}, brightest channel {brightness:.3}; the game names {:?}", palette::NIGHT),
    );
}

/// Every string in the content table, printable.
pub fn content_is_printable(checks: &mut Checks) {
    let mut strings: Vec<&str> = Vec::new();
    for being in &BEINGS {
        strings.extend([being.name, being.cult, being.greeting]);
        for fact in &being.facts {
            strings.extend([fact.title, fact.known, fact.question]);
            strings.extend(fact.answers);
        }
    }
    let stray: Vec<&str> = strings
        .iter()
        .copied()
        .filter(|text| text.chars().any(|glyph| !(' '..='~').contains(&glyph)))
        .collect();
    checks.require(
        stray.is_empty(),
        "the lore has a character the font cannot draw",
        format!("{stray:?}"),
    );
}

/// The screens a played run may never reach, staged whole and judged.
pub fn staged(checks: &mut Checks, margins: &mut Margins) {
    let mut stage = Conductor::new(crate::decisions::SEED);
    let font = stage.font();
    // A call where every fact is known: the longest lore list, under the
    // longest question, with the longest answers.
    for being in Being::ALL {
        for fact in 0..FACTS {
            stage.game_mut().run.known[being.index()][fact] = true;
        }
        let questions: Vec<usize> = (0..FACTS).collect();
        let game = stage.game_mut();
        game.night.calls = vec![crate::rules::PlannedCall { being, questions }];
        game.phase = Phase::Call(CallState {
            slot: 0,
            being,
            interest: 5,
            asked: 2,
            quiet: 0,
            spent: 0,
            exchanges: 0,
            last: Some("It is ANGERED. (-10 sanity)".to_owned()),
        });
        game.run.temper[being.index()] = 5;
        game.run.sanity = 9;
        let photo = stage.photo();
        judge(
            checks,
            margins,
            &format!("staged call, {} all known", being.lore().name),
            &photo.panel,
            &photo.frame,
            font,
        );
    }
    // The hang-up between calls, the last of a night, and of the run.
    for (slot, day) in [(0, 2), (0, crate::rules::DAYS)] {
        let game = stage.game_mut();
        game.run.day = day;
        game.night.calls = vec![crate::rules::PlannedCall {
            being: Being::YogSothoth,
            questions: vec![0],
        }];
        game.phase = Phase::Hangup(Hangup {
            slot,
            being: Being::YogSothoth,
            spent: 44,
            exchanges: 11,
            last: "You said nothing. It listened. (-5 sanity)".to_owned(),
        });
        let photo = stage.photo();
        judge(
            checks,
            margins,
            &format!("staged hang-up, day {day}"),
            &photo.panel,
            &photo.frame,
            font,
        );
    }
    // The three endings, and that they are three different screens.
    let endings = [
        Ending::Lost {
            day: 3,
            being: Being::Nyarlathotep,
        },
        Ending::Won { sanity: 50 },
        Ending::Won { sanity: 49 },
    ];
    let mut verdicts = Vec::new();
    for ending in endings {
        let game = stage.game_mut();
        game.phase = Phase::Ended(ending);
        game.run.sanity = match ending {
            Ending::Won { sanity } => sanity,
            Ending::Lost { .. } => 0,
        };
        let photo = stage.photo();
        judge(
            checks,
            margins,
            &format!("staged ending {ending:?}"),
            &photo.panel,
            &photo.frame,
            font,
        );
        verdicts.push(ending_lines(ending));
    }
    checks.require(
        verdicts[0].0 != verdicts[1].0
            && verdicts[1].1 != verdicts[2].1
            && verdicts[1].0 == verdicts[2].0,
        "the endings are not the screens they claim: a loss, a sound sleep, a shaken survival",
        format!("{verdicts:?}"),
    );
    checks.require(
        verdicts[0].0.contains("GONE")
            && verdicts[1].1.contains("sleep through")
            && verdicts[2].1.contains("dreams"),
        "an ending says the wrong thing for its outcome (lost at 0, won at 50, won at 49)",
        format!("{verdicts:?}"),
    );
}

/// A floor you have not seen bite is a floor you have not got: two rows
/// stacked on one band must be refused by name.
pub fn floor_bites(checks: &mut Checks) {
    let mut staged: Panel<Art> = Panel::default();
    staged.text(TextRun::new(
        Vec2::new(40.0, 300.0),
        "1  A promise, paid in gold.",
        screens::body(),
    ));
    staged.text(TextRun::new(
        Vec2::new(40.0, 306.0),
        "2  Nothing. They are fishermen.",
        screens::body(),
    ));
    let bites = judge_panel(&staged, &FLOORS, &[], &[])
        .iter()
        .any(|breach| breach.what == "two rows of chrome text overlap");
    checks.require(
        bites,
        "the overlap floor does not refuse two answers drawn on top of each other",
        String::new(),
    );
}
