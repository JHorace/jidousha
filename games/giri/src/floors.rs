//! UI.md §7's readability floors, as assertions rather than as advice.
//!
//! "Enforcement over exhortation" is the section's own phrase: a floor written
//! in a comment is a floor nobody keeps, and every rule below is stated here as
//! a check that fails the run. They are stated **at reference scale**, which is
//! the scale giri's world units are (one world unit is one reference pixel), so
//! a floor and the number UI.md writes are the same number.
//!
//! Six floors, in the section's order:
//!
//! 1. no text below 12 reference pixels;
//! 2. every clickable target at least 32x32;
//! 3. interactive cards never overlap each other or the info panel;
//! 4. nothing drawn outside the design rect (and the design rect always inside
//!    what the camera shows, which is §6's contract seen from here);
//! 5. every stat drawn as a number has its icon quad adjacent;
//! 6. every drawn line ASCII — which `contracts::printable_strings` owns,
//!    because it can walk every string without a frame.

use jidousha::prelude::*;

use crate::checks::{Checks, greater};
use crate::flow::Flow;
use crate::model::Member;
use crate::verify::{BeatRun, inside};
use crate::{layout, party, scaling, screens, theme, verify};

/// Every rectangle a click does something in, with the name a message uses.
///
/// The release control and the send verb are conditional on screen and
/// unconditional here: a target that is only sometimes drawn still has to be
/// big enough on the frames it is drawn on, and a floor that skipped it would
/// be a floor with a hole exactly where the state machine is.
pub fn targets() -> Vec<(&'static str, Rect)> {
    let mut out: Vec<(&'static str, Rect)> = (0..layout::QUEST_SLOTS)
        .map(|index| ("a quest card", layout::quest_card(index)))
        .collect();
    for index in 0..4 {
        out.push(("a party card", layout::party_card(index)));
    }
    out.push(("the send verb", layout::send_button()));
    out.push(("the release control", layout::release_button()));
    out.push(("the log drawer's handle", layout::log_button()));
    out.push(("the tuning drawer's handle", layout::tune_button()));
    out
}

/// Every rectangle the tuning drawer answers a click in.
///
/// **A set of its own, and not part of `targets`**, because the drawer covers
/// the board: a stepper sitting over a quest card is the drawer working, and
/// folding the two sets together would make the overlap floor report the
/// covering as a collision. The 32x32 floor applies to both sets identically,
/// and these are the smallest targets in the game — twenty steppers at exactly
/// the floor (UI.md §12).
pub fn tuner_targets() -> Vec<(String, Rect)> {
    let mut out: Vec<(String, Rect)> = Vec::new();
    for (index, preset) in crate::presets::PRESETS.iter().enumerate() {
        out.push((
            format!("the {} preset", preset.name),
            layout::tuner_preset(index),
        ));
    }
    for (index, field) in crate::constants::Field::ALL.iter().copied().enumerate() {
        out.push((format!("{}'s -", field.name()), layout::tuner_minus(index)));
        out.push((format!("{}'s +", field.name()), layout::tuner_plus(index)));
    }
    for (index, id) in crate::variant::VariantId::ALL.iter().enumerate() {
        out.push((
            format!("the {} variant", id.key()),
            layout::variant_button(index),
        ));
    }
    out.push(("the APPLY verb".to_owned(), layout::tuner_apply()));
    out
}

/// UI.md §7's floors over the tuning drawer, which no played beat opens.
pub fn tuner_floors(checks: &mut Checks) {
    let drawer = layout::tuner_panel();
    for (what, rect) in tuner_targets() {
        let size = rect.size();
        checks.require(
            !greater(theme::MIN_TARGET, size.x) && !greater(theme::MIN_TARGET, size.y),
            "a clickable target is smaller than the readability floor allows",
            format!(
                "{what} is {:.0}x{:.0} reference pixels and UI.md §7's floor is {}x{}",
                size.x,
                size.y,
                theme::MIN_TARGET,
                theme::MIN_TARGET
            ),
        );
        checks.require(
            inside(drawer, rect),
            "a tuning control is outside the drawer that holds it",
            format!("{what} is {rect:?} and the drawer is {drawer:?}"),
        );
    }
    let controls = tuner_targets();
    for (index, (what, rect)) in controls.iter().enumerate() {
        for (other_what, other) in controls.iter().skip(index + 1) {
            checks.require(
                !rect.overlaps(*other),
                "two tuning controls overlap",
                format!("{what} at {rect:?} overlaps {other_what} at {other:?}"),
            );
        }
    }
    // The drawer's handle is on the board and the drawer is not: a handle under
    // its own drawer is a drawer that cannot be closed.
    checks.require(
        !layout::tune_button().overlaps(drawer),
        "the tuning drawer covers its own handle",
        format!(
            "the handle is {:?} and the drawer is {drawer:?}",
            layout::tune_button()
        ),
    );
}

/// The floors that are questions about the layout alone.
pub fn layout_floors(checks: &mut Checks) {
    // --- floor 2: every clickable target is at least 32x32 ----------------
    for (what, rect) in targets() {
        let size = rect.size();
        checks.require(
            !greater(theme::MIN_TARGET, size.x) && !greater(theme::MIN_TARGET, size.y),
            "a clickable target is smaller than the readability floor allows",
            format!(
                "{what} is {:.0}x{:.0} reference pixels and UI.md §7's floor is {}x{}",
                size.x,
                size.y,
                theme::MIN_TARGET,
                theme::MIN_TARGET
            ),
        );
        checks.require(
            inside(layout::design(), rect),
            "a clickable target is partly off the design rect",
            format!(
                "{what} is {rect:?} and the design rect is {:?}",
                layout::design()
            ),
        );
    }

    // --- floor 3: interactive cards never overlap ---------------------------
    let mut interactive: Vec<(&'static str, Rect)> = targets();
    interactive.push(("the info panel", layout::info_panel()));
    for (index, (what, rect)) in interactive.iter().enumerate() {
        for (other_what, other) in interactive.iter().skip(index + 1) {
            // The release control lives *inside* the panel by design; a nested
            // target is not a collision, and the floor is about siblings.
            if layout::info_panel().contains_rect(*rect)
                || layout::info_panel().contains_rect(*other)
            {
                continue;
            }
            checks.require(
                !rect.overlaps(*other),
                "two interactive rectangles overlap",
                format!("{what} at {rect:?} overlaps {other_what} at {other:?}"),
            );
        }
    }

    // The party strip's cards have to finish inside the strip, and the strip's
    // label has to finish before they start: the floor above catches cards
    // colliding with each other and not a card running off its own band.
    let strip = layout::party_strip();
    let last = layout::party_card(3);
    checks.require(
        inside(strip, last),
        "a party card runs off the strip it belongs to",
        format!("the last card is {last:?} and the strip is {strip:?}"),
    );
    checks.require(
        greater(
            layout::party_card(0).min.y,
            layout::party_label().y + theme::SMALL,
        ),
        "the party strip's cards are drawn up into its own label",
        format!(
            "the label sits at y {:.0} and the first card starts at {:.0}",
            layout::party_label().y,
            layout::party_card(0).min.y
        ),
    );
    // And the quest row has to clear the top bar it hangs under.
    checks.require(
        greater(layout::quest_card(0).min.y, layout::topbar().max.y),
        "the quest row is drawn up into the status bar",
        format!(
            "the bar ends at y {:.0} and the first card starts at {:.0}",
            layout::topbar().max.y,
            layout::quest_card(0).min.y
        ),
    );
}

/// **UI.md §6's contract**, asserted at four surfaces rather than described.
///
/// A uniform fit is four claims at once — the whole design rect is on screen,
/// the scale is the same on both axes, the spare span is split evenly, and the
/// scale stops falling at the floor — and none of them is visible to an
/// assertion about one viewport. So the arithmetic is asked at the reference
/// size, at the narrow one the captures use, at a short one, and at a window
/// far below the minimum scale.
pub fn scaling_contract(checks: &mut Checks) -> String {
    let mut notes = Vec::new();
    for (what, viewport) in [
        ("reference", verify::HEADLESS_VIEWPORT),
        ("narrow", verify::NARROW_VIEWPORT),
        ("short", PhysicalSize::new(1280, 300)),
        ("tiny", PhysicalSize::new(200, 160)),
    ] {
        let camera = scaling::camera_for(viewport);
        let view = camera.visible_bounds();
        let scale = scaling::scale_for(viewport);
        // Uniform: a world unit is the same number of pixels on both axes. The
        // camera's own contract already guarantees it (width follows height by
        // the viewport's aspect), so what this catches is a fit that stopped
        // deriving one from the other.
        let per_pixel_x = view.size().x / viewport.width as f32;
        let per_pixel_y = view.size().y / viewport.height as f32;
        checks.require(
            crate::checks::near(per_pixel_x, per_pixel_y),
            "the view is not scaled uniformly - one axis is stretched",
            format!(
                "at {what} ({}x{}) a world unit is {per_pixel_x:.4} pixels across and \
                 {per_pixel_y:.4} down; aspect-preserving means one number",
                viewport.width, viewport.height
            ),
        );
        // Symmetric: the spare span is split evenly, which is what makes it a
        // letterbox rather than a design pinned to a corner.
        let design = layout::design();
        checks.require(
            crate::checks::near(design.min.x - view.min.x, view.max.x - design.max.x)
                && crate::checks::near(design.min.y - view.min.y, view.max.y - design.max.y),
            "the letterbox is not symmetric",
            format!(
                "at {what} the spare span is {:.1} left / {:.1} right and {:.1} above / \
                 {:.1} below",
                design.min.x - view.min.x,
                view.max.x - design.max.x,
                design.min.y - view.min.y,
                view.max.y - design.max.y
            ),
        );
        if scale > scaling::MIN_SCALE {
            // Above the floor, the whole design rect is on screen.
            checks.require(
                inside(view, design),
                "the whole design rect is not on screen above the minimum scale",
                format!(
                    "at {what} the camera shows {view:?} and the design rect is {design:?} at \
                     a scale of {scale:.3}"
                ),
            );
        }
        notes.push(format!(
            "{what} {}x{} = {scale:.3}x",
            viewport.width, viewport.height
        ));
    }
    // The floor itself: a window far below it clamps rather than shrinking on.
    let tiny = scaling::scale_for(PhysicalSize::new(80, 60));
    checks.require(
        crate::checks::near(tiny, scaling::MIN_SCALE),
        "the view keeps shrinking below the minimum scale",
        format!(
            "an 80x60 surface scales to {tiny:.3} and the floor is {:.3}",
            scaling::MIN_SCALE
        ),
    );
    // And the reference surface is exactly reference scale, which is the claim
    // every number in UI.md §7 is stated against.
    let reference = scaling::scale_for(layout::REFERENCE);
    checks.require(
        crate::checks::near(reference, 1.0),
        "the reference resolution is no longer reference scale",
        format!(
            "{}x{} scales to {reference:.3}, and every floor in UI.md §7 is stated at 1.0",
            layout::REFERENCE.width,
            layout::REFERENCE.height
        ),
    );
    notes.join(", ")
}

/// The floors over the drawer as it is actually drawn — the same three the
/// board gets, on the one screen no played beat reaches.
pub fn judge_tuner(checks: &mut Checks, run: &crate::restart::DrawerRun) {
    // Four screens, and three of them are on no frame the session photographed:
    //
    // - the drawer as the session photographed it;
    // - the drawer just after an APPLY, whose hint row carries the whole
    //   constants stamp - a hundred and forty-five characters with no space in
    //   it, and the longest single row this game ever draws;
    // - the drawer as a refused `?constants=` leaves it, carrying the list of
    //   every key;
    // - the *log* drawer with that same stamp in it, because the applied line
    //   goes to the log as well and the log drawer has a width of its own.
    //
    // The second and the fourth are here because a hand playtest found the
    // stamp running a third of the way off the screen, on exactly the two
    // screens nothing was checking.
    let mut refused = run.pending.clone();
    refused.tuner.fault = crate::links::refusals().into_iter().max_by_key(String::len);
    let mut logged = run.applied_flow.clone();
    logged.tuner.open = false;
    logged.log_open = true;
    // And the board with both drawers closed and the APPLY toast still up: the
    // toast lands in the dilemma band, which is the shortest band on the board.
    let mut toasted = run.applied_flow.clone();
    toasted.tuner.open = false;
    for flow in [&run.pending, &run.applied_flow, &refused, &logged, &toasted] {
        judge_drawer_screen(checks, run, flow);
    }
    judge_drawer_frame(checks, run);
}

fn judge_drawer_screen(checks: &mut Checks, run: &crate::restart::DrawerRun, flow: &Flow) {
    let panel = screens::content(
        flow,
        &run.pending_social,
        &run.pending_preview,
        &run.pending_active,
        crate::variant::VariantId::default(),
    );
    // Which drawer is up decides what an overlay row owes: the one it is in,
    // and - for the tuning drawer, whose controls only exist while it is open -
    // the steppers it must not lie across.
    let (what, drawer) = if flow.tuner.open {
        ("the tuning drawer", Some(layout::tuner_panel()))
    } else if flow.log_open {
        ("the log drawer", Some(layout::log_panel()))
    } else {
        ("the board after an APPLY", None)
    };
    for text in &panel.runs {
        checks.require(
            !greater(theme::MIN_TEXT, text.size),
            "a row of text is smaller than the readability floor allows",
            format!(
                "{what}: {:?} is set at {:.1} reference pixels and UI.md §7's floor is {:.0}",
                text.text,
                text.size,
                theme::MIN_TEXT
            ),
        );
        checks.require(
            inside(layout::design(), text.bounds()),
            "a row of text runs off the design rect",
            format!("{what}: {:?} occupies {:?}", text.text, text.bounds()),
        );
        // The board's own rows are behind the open drawer, and they are judged
        // against the board's controls in `judge`. What is left is the drawer's
        // own text, and the two things it owes: staying inside the drawer, and
        // not lying across a control.
        let Some(drawer) = drawer else { continue };
        if text.layer != theme::layers::OVERLAY_TEXT {
            continue;
        }
        checks.require(
            inside(drawer, text.bounds()),
            "a row of a drawer is drawn outside it",
            format!(
                "{what}: {:?} occupies {:?} and the drawer is {drawer:?}",
                text.text,
                text.bounds()
            ),
        );
        if !flow.tuner.open {
            continue;
        }
        for (control, target) in tuner_targets() {
            if !text.bounds().overlaps(target) {
                continue;
            }
            checks.require(
                inside(target, text.bounds()),
                "a row of text lies across a control it is not the label of",
                format!(
                    "{what}: {:?} at {:?} crosses {control} at {target:?}",
                    text.text,
                    text.bounds()
                ),
            );
        }
    }
}

/// The smallest glyph actually on the frame, which the layout cannot give: the
/// drawer is where the smallest type in the game is.
fn judge_drawer_frame(checks: &mut Checks, run: &crate::restart::DrawerRun) {
    if let Some(frame) = &run.pending_frame {
        let smallest = frame
            .quads()
            .iter()
            .filter(|quad| quad.texture == run.font)
            .map(|quad| quad.bounds().size().y)
            .fold(f32::MAX, f32::min);
        checks.require(
            smallest == f32::MAX || !greater(theme::MIN_TEXT - 0.01, smallest),
            "a glyph was drawn below the readability floor",
            format!(
                "the shortest glyph quad with the tuning drawer open is {smallest:.2} \
                 reference pixels and the floor is {:.0}",
                theme::MIN_TEXT
            ),
        );
    }
}

/// The floors that need a played beat and its frames.
pub fn judge(checks: &mut Checks, run: &BeatRun) {
    let beat = run.index + 1;

    // --- floor 1: no text below the minimum size --------------------------
    for (mode, flow, social, preview) in [
        (
            "the board",
            &run.board_flow,
            &run.at_assembly,
            &run.board_preview,
        ),
        (
            "the staged board",
            &run.ready_flow,
            &run.at_assembly,
            &run.ready,
        ),
        (
            "the takeover",
            &run.report_flow,
            &run.after,
            &run.report_preview,
        ),
    ] {
        let panel = screens::content(flow, social, preview, &run.tuning, run.variant);
        for text in &panel.runs {
            checks.require(
                !greater(theme::MIN_TEXT, text.size),
                "a row of text is smaller than the readability floor allows",
                format!(
                    "beat {beat}, {mode}: {:?} is set at {:.1} reference pixels and UI.md §7's \
                     floor is {:.0}",
                    text.text,
                    text.size,
                    theme::MIN_TEXT
                ),
            );
            checks.require(
                inside(layout::design(), text.bounds()),
                "a row of text runs off the design rect",
                format!(
                    "beat {beat}, {mode}: {:?} occupies {:?}",
                    text.text,
                    text.bounds()
                ),
            );
        }
        // Every icon inside the rect too, which is the same floor for art.
        for icon in &panel.icons {
            checks.require(
                inside(layout::design(), icon.bounds()),
                "an icon runs off the design rect",
                format!(
                    "beat {beat}, {mode}: {:?} occupies {:?}",
                    icon.art,
                    icon.bounds()
                ),
            );
            checks.require(
                crate::checks::near(icon.scale, icon.scale.round()),
                "a pixel-art icon is drawn at a fractional scale",
                format!(
                    "beat {beat}, {mode}: {:?} is drawn at {:.2}x, and the engine samples \
                     nearest - a fraction puts a wobble in it (UI.md §1.4)",
                    icon.art, icon.scale
                ),
            );
        }
    }

    // --- floor 3, for text: nothing lies across a control it is not the
    // label of. A row that half-covers a button is a button a player cannot
    // read, and the overlap check on rectangles alone cannot see it.
    for (mode, flow, social, preview) in [
        (
            "the board",
            &run.board_flow,
            &run.at_assembly,
            &run.board_preview,
        ),
        (
            "the staged board",
            &run.ready_flow,
            &run.at_assembly,
            &run.ready,
        ),
    ] {
        let panel = screens::content(flow, social, preview, &run.tuning, run.variant);
        for text in &panel.runs {
            for (what, target) in targets() {
                if !text.bounds().overlaps(target) {
                    continue;
                }
                checks.require(
                    inside(target, text.bounds()),
                    "a row of text lies across a control it is not the label of",
                    format!(
                        "beat {beat}, {mode}: {:?} at {:?} crosses {what} at {target:?}",
                        text.text,
                        text.bounds()
                    ),
                );
            }
        }
    }

    // --- floor 1, on the frame: no glyph quad shorter than the floor ------
    if let Some(frame) = &run.ready_frame {
        let smallest = frame
            .quads()
            .iter()
            .filter(|quad| quad.texture == run.font)
            .map(|quad| quad.bounds().size().y)
            .fold(f32::MAX, f32::min);
        checks.require(
            smallest == f32::MAX || !greater(theme::MIN_TEXT - 0.01, smallest),
            "a glyph was drawn below the readability floor",
            format!(
                "beat {beat}: the shortest glyph quad on the staged board is {smallest:.2} \
                 reference pixels and the floor is {:.0}",
                theme::MIN_TEXT
            ),
        );
    }

    // --- floor 5: every stat number has its icon beside it -----------------
    for member in &run.at_assembly.members {
        stat_redundancy(checks, run, member, "the staged board", &run.ready_frame);
    }
}

/// **The redundancy floor** (UI.md §1, §7): a stat never appears as a bare
/// number.
///
/// Asked two ways, because either alone passes for the wrong reason: the icon
/// has to be *adjacent* to the number in the layout — within one glyph's
/// advance, on the same row — and a quad that is not the font has to actually
/// be on the frame where the layout put it. Adjacency alone would pass for an
/// icon nobody drew; a quad alone would pass for an icon on the other side of
/// the screen.
fn stat_redundancy(
    checks: &mut Checks,
    run: &BeatRun,
    member: &Member,
    mode: &str,
    frame: &Option<jidousha::testing::FrameRecord>,
) {
    let beat = run.index + 1;
    let card = layout::party_card(member.roster_index);
    for stat in party::stats_of(card, member, member.alive) {
        let icon = stat.icon.bounds();
        let value = stat.value.bounds();
        let gap = value.min.x - icon.max.x;
        checks.require(
            !greater(gap, theme::SMALL) && !greater(-1.0, gap),
            "a stat's icon is not adjacent to the number it names",
            format!(
                "beat {beat}, {mode}: {:?} ends at x {:.1} and {:?}'s number starts at {:.1}, \
                 a gap of {gap:.1}",
                stat.icon.art, icon.max.x, member.name, value.min.x
            ),
        );
        checks.require(
            !greater(icon.min.y, value.max.y) && !greater(value.min.y, icon.max.y),
            "a stat's icon is not on the same row as the number it names",
            format!(
                "beat {beat}, {mode}: {:?} spans y {:.1}-{:.1} and the number {:.1}-{:.1}",
                stat.icon.art, icon.min.y, icon.max.y, value.min.y, value.max.y
            ),
        );
        let Some(frame) = frame else { continue };
        let drawn = frame
            .quads()
            .iter()
            .any(|quad| quad.texture != run.font && quad.bounds().overlaps(icon));
        checks.require(
            drawn,
            "a stat's icon is in the layout and not on the frame",
            format!(
                "beat {beat}, {mode}: nothing but glyphs covers {:?} at {icon:?}",
                stat.icon.art
            ),
        );
    }
}
