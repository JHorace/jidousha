//! The live tuning drawer: DESIGN §8a's mechanism, as UI.md §12 draws it.
//!
//! **The pending/active split is the whole design.** The drawer edits a
//! *pending* copy that lives in `Flow` and that nothing in the simulation ever
//! reads; the active constants are the `Tuning` resource, which is what the
//! decision function reads and what every recording and verify report is
//! stamped with. Nothing changes mid-beat, ever. `APPLY` is the one moment the
//! two meet, and what it does is DESIGN §8a's determinism resolution: swap the
//! resource **and restart the current beat**, so the run that follows is a pure
//! function of (beat state, assignments, constants) exactly as a fresh run at
//! those constants would be. An apply that did not restart would leave a
//! recording that lies about what produced it.
//!
//! **The drawer displays what the simulation reads.** `drawer` takes the active
//! `Tuning` by reference from the world's resource rather than a copy kept
//! beside it, so the stamp on screen cannot drift from the numbers the beat is
//! being played with. That is the one thing this file exists to make true.
//!
//! Presets are `presets::PRESETS`, walked — data, one place, no code per preset
//! (DESIGN §8b tier 1). Rows are `Field::ALL`, walked — every constant the
//! module has, so a constant added to `constants.rs` grows a row here without
//! this file being edited.

use jidousha::prelude::*;

use crate::constants::{Field, Tuning};
use crate::flow::{Flow, Stage};
use crate::presets::PRESETS;
use crate::ui::{Panel, TextRun, columns, wrap};
use crate::variant::VariantId;
use crate::{layout, presets, theme};

/// The drawer's state. **UI state, all of it** — not one field of this is read
/// by anything that decides an outcome.
#[derive(Clone, Debug)]
pub struct Tuner {
    /// Whether the drawer is open.
    pub open: bool,
    /// The set being edited. Persists until applied or overwritten by a preset;
    /// closing the drawer discards nothing (UI.md §12).
    pub pending: Tuning,
    /// Which row the pointer is on, for its one-line meaning — this game's
    /// hover text, since a pixel-font game has no tooltip.
    pub hover: Option<Field>,
    /// What a rejected `?constants=` said, if one was rejected at startup.
    ///
    /// Kept here because the drawer is where a person can see what *is* in
    /// effect beside what was asked for, and `main` opens the drawer when it is
    /// set: a link that was refused says so on the page (UI.md §12).
    pub fault: Option<String>,
}

impl Default for Tuner {
    fn default() -> Self {
        Self {
            open: false,
            pending: Tuning::SHIPPED,
            hover: None,
            fault: None,
        }
    }
}

/// Whether the pending set differs from the one in effect — what lights `APPLY`
/// and what draws a value in gold.
pub fn dirty(pending: &Tuning, active: &Tuning) -> bool {
    pending != active
}

/// Everything the drawer says, as data (`ui::Panel`, like every other screen).
///
/// `variant` and the seed on `flow` join the stamp: a recording is only
/// reproducible if it says everything it ran with, and since P2 that is
/// constants, variant and seed together (DESIGN §8b, §12).
pub fn drawer(flow: &Flow, active: &Tuning, variant: VariantId) -> Panel {
    let tuner = &flow.tuner;
    let mut panel = Panel::default();
    panel.text(TextRun::over(
        layout::tuner_title(),
        "TUNING - the constants the simulation reads",
        theme::HEAD,
        theme::GOLD,
    ));

    // --- presets: one button per row of the committed table ----------------
    panel.text(TextRun::over(
        layout::tuner_presets_label(),
        "presets",
        theme::SMALL,
        theme::DIM,
    ));
    for (index, preset) in PRESETS.iter().enumerate() {
        let button = layout::tuner_preset(index);
        panel.text(TextRun::over(
            crate::ui::centered(button, preset.name, theme::SMALL, button.min.y + 10.0),
            preset.name,
            theme::SMALL,
            theme::DIM,
        ));
    }

    // --- one stepper row per constant in the module ------------------------
    for (index, field) in Field::ALL.iter().copied().enumerate() {
        let moved = tuner.pending.field(field) != active.field(field);
        panel.text(TextRun::over(
            layout::tuner_name(index),
            field.name(),
            theme::SMALL,
            if moved { theme::GOLD } else { theme::DIM },
        ));
        let value = format!("{}", tuner.pending.field(field));
        let cell = layout::tuner_value(index);
        panel.text(TextRun::over(
            crate::ui::centered(cell, &value, theme::SMALL, cell.min.y + 10.0),
            value,
            theme::SMALL,
            if moved { theme::GOLD } else { theme::INK },
        ));
        for (rect, glyph) in [
            (layout::tuner_minus(index), "-"),
            (layout::tuner_plus(index), "+"),
        ] {
            panel.text(TextRun::over(
                crate::ui::centered(rect, glyph, theme::BODY, rect.min.y + 10.0),
                glyph,
                theme::BODY,
                theme::INK,
            ));
        }
    }

    // --- the commit verb ---------------------------------------------------
    let apply = layout::tuner_apply();
    panel.text(TextRun::over(
        crate::ui::centered(apply, "APPLY", theme::SMALL, apply.min.y + 11.0),
        "APPLY",
        theme::SMALL,
        if dirty(&tuner.pending, active) {
            theme::GROUND
        } else {
            theme::DIM
        },
    ));

    // --- the prose band: the hint row, then the note under it --------------
    //
    // The hint has four possible tenants, in priority order, because each is
    // more urgent than the one under it: a link that was refused, then the
    // sentence the last APPLY raised (the board's own toast is behind this
    // drawer and unreadable while it is open), then the meaning of whatever the
    // pointer is on, then how to get one.
    let (hint, tone) = if let Some(fault) = &tuner.fault {
        (fault.clone(), theme::EMBER)
    } else if let Some(toast) = &flow.toast {
        (toast.text.clone(), theme::GOLD)
    } else if let Some(field) = tuner.hover {
        (
            format!("{} - {}", field.name(), field.meaning()),
            theme::DIM,
        )
    } else {
        (
            "point at a constant for what it does".to_owned(),
            theme::FAINT,
        )
    };
    let prose = columns(layout::tuner_prose_width(), theme::SMALL);
    let below = panel.block(
        layout::tuner_hint(),
        &wrap(&hint, prose),
        theme::SMALL,
        tone,
    );
    // **The note gives its room to a refused link and to nothing else.** The
    // rejection is three lines of key names and is the only thing on the screen
    // a player has to act on; the note is the one row of this drawer they will
    // have read already.
    if tuner.fault.is_none() {
        panel.block(
            Vec2::new(layout::tuner_hint().x, below + 4.0),
            &wrap(
                "APPLY restarts this beat with the new values. every recording and verify \
                 report is stamped with the constants in effect.",
                prose,
            ),
            theme::SMALL,
            theme::FAINT,
        );
    }

    // --- the variant picker (DESIGN §8b): rule-set assembly is chain-start,
    // so the picker lives with the other simulation inputs and switching
    // restarts the chain from the top.
    panel.text(TextRun::over(
        layout::variant_label(),
        "variant",
        theme::SMALL,
        theme::DIM,
    ));
    for (index, id) in VariantId::ALL.iter().copied().enumerate() {
        let button = layout::variant_button(index);
        panel.text(TextRun::over(
            crate::ui::centered(button, id.key(), theme::SMALL, button.min.y + 10.0),
            id.key(),
            theme::SMALL,
            if id == variant {
                theme::GOLD
            } else {
                theme::DIM
            },
        ));
    }

    // --- the stamp: what is actually in effect, always visible -------------
    panel.text(TextRun::over(
        layout::tuner_stamp(),
        "in effect:",
        theme::SMALL,
        theme::DIM,
    ));
    panel.block(
        layout::tuner_stamp() + Vec2::new(0.0, 14.0),
        &format!(
            "{}\nvariant {} - seed {}",
            active.readout(),
            variant.key(),
            flow.seed
        ),
        theme::SMALL,
        theme::INK,
    );
    // Every run of `panel.block` above draws on the board's text band; the
    // drawer is an overlay, so they are lifted here rather than at each call.
    for run in &mut panel.runs {
        run.layer = theme::layers::OVERLAY_TEXT;
    }
    panel
}

/// The pointer, while the board is up: the handle, and the drawer's own rows.
///
/// Returns whether the click was the drawer's. **A click inside the drawer's
/// rectangle is always the drawer's**, hit or miss — it covers the board, and a
/// click that fell through it would act on a card the player cannot see (the
/// log drawer's rule, and the same reason).
pub fn handle_pointer(world: &mut World, at: Vec2, tick: u64, clicked: bool) -> bool {
    // Hover every tick, click or no click, so a row's meaning appears by
    // pointing at it.
    let open = world.resource::<Flow>().tuner.open;
    let hover = open
        .then(|| {
            Field::ALL
                .iter()
                .copied()
                .enumerate()
                .find(|(index, _)| layout::tuner_row(*index).contains(at))
                .map(|(_, field)| field)
        })
        .flatten();
    world.resource_mut::<Flow>().tuner.hover = hover;

    if clicked && layout::tune_button().contains(at) {
        let flow = world.resource_mut::<Flow>();
        flow.tuner.open = !flow.tuner.open;
        if flow.tuner.open {
            // Opening it is the acknowledgement a refused link was waiting for.
            flow.tuner.fault = None;
            // Two drawers over one board would be two drawers a click has to
            // choose between; the second one to open wins.
            flow.log_open = false;
        }
        return true;
    }
    if !open {
        return false;
    }
    if !clicked {
        return layout::tuner_panel().contains(at);
    }
    if !layout::tuner_panel().contains(at) {
        return false;
    }

    // A preset replaces the pending set outright (UI.md §12) and touches
    // nothing else: the beat is still being played at the active constants.
    for (index, preset) in PRESETS.iter().enumerate() {
        if layout::tuner_preset(index).contains(at) {
            world.resource_mut::<Flow>().tuner.pending = preset.tuning;
            return true;
        }
    }
    // The variant picker: rule-set assembly happens at chain start (DESIGN
    // §8b), so picking a different rule set restarts the chain from the top —
    // immediately, not pending, because a variant is not a number a beat can
    // absorb at its own boundary.
    for (index, id) in VariantId::ALL.iter().copied().enumerate() {
        if layout::variant_button(index).contains(at) {
            let current = world
                .find_resource::<VariantId>()
                .copied()
                .unwrap_or_default();
            if id != current {
                world.insert_resource(id);
                crate::flow::load_beat(world, 0);
                let flow = world.resource_mut::<Flow>();
                flow.tuner.open = true;
                flow.stage = Stage::Board;
                flow.bounce(
                    tick,
                    format!("variant {} - the chain restarts from the top", id.key()),
                );
            }
            return true;
        }
    }
    for (index, field) in Field::ALL.iter().copied().enumerate() {
        let step = if layout::tuner_minus(index).contains(at) {
            -1
        } else if layout::tuner_plus(index).contains(at) {
            1
        } else {
            continue;
        };
        let pending = &mut world.resource_mut::<Flow>().tuner.pending;
        let slot = pending.field_mut(field);
        *slot = (*slot + step).clamp(Tuning::MIN, Tuning::MAX);
        return true;
    }
    if layout::tuner_apply().contains(at) {
        apply(world, tick);
    }
    true
}

/// Commit the pending set: swap the resource, and restart the current beat.
///
/// **The two halves are one action.** DESIGN §8a's resolution is that constants
/// are simulation inputs and a run is only reproducible if the constants it ran
/// with were in effect for all of it; a swap without the restart would leave the
/// first half of a beat played at one set and the second at another, and the
/// stamp on the recording would be true of neither.
pub fn apply(world: &mut World, tick: u64) {
    let (pending, active) = {
        let flow = world.resource::<Flow>();
        (flow.tuner.pending, *world.resource::<Tuning>())
    };
    if !dirty(&pending, &active) {
        return;
    }
    world.insert_resource(pending);
    let beat = world.resource::<Flow>().beat;
    crate::flow::load_beat(world, beat);
    let flow = world.resource_mut::<Flow>();
    // `load_beat` closes the drawers, and this restart is the one the drawer
    // asked for: a tuning session that had to reopen it after every apply
    // would be a session that stops A/B-ing after the second try.
    flow.tuner.open = true;
    flow.stage = Stage::Board;
    // **Two lines, and which one carries the stamp is the point.** UI.md §12
    // asks for "a toast saying so and a log line recording the applied stamp":
    // the toast is read in the two and a half seconds it is up and says what
    // happened, and the stamp - a hundred and forty-five characters of machine
    // text - is the record, so it goes to the log, where it can be read at
    // leisure and wraps into a drawer built for rows. Both are logged, because
    // nothing that matters appears only in a toast (UI.md §3).
    flow.note(format!("constants applied - {}", pending.stamp()));
    flow.bounce(
        tick,
        format!(
            "constants applied - this beat restarts - {}",
            name_of(&pending).unwrap_or("a hand-stepped set")
        ),
    );
}

/// The named preset a set is, if it is one — what a report says instead of ten
/// numbers.
pub fn name_of(tuning: &Tuning) -> Option<&'static str> {
    presets::PRESETS
        .iter()
        .find(|preset| preset.tuning == *tuning)
        .map(|preset| preset.name)
}
