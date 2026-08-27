//! The live tuning drawer, carried from giri (its DESIGN §8a mechanism).
//!
//! **The pending/active split is the whole design.** The drawer edits a
//! *pending* copy that lives in `Flow` and that nothing in the simulation ever
//! reads; the active constants are the `Tuning` resource, which is what the
//! clock and the pathfinder read and what every recording and verify report is
//! stamped with. Nothing changes mid-scenario, ever. `APPLY` is the one moment
//! the two meet: swap the resource **and restart the scenario**, so the run
//! that follows is a pure function of (orders, constants, seed) exactly as a
//! fresh run at those constants would be — giri applied at a beat boundary,
//! and the fork's boundary is the scenario.
//!
//! **The drawer displays what the simulation reads.** `drawer` takes the
//! active `Tuning` from the world's resource rather than a copy kept beside
//! it, so the stamp on screen cannot drift from the numbers the world is
//! moving with.
//!
//! Presets are `presets::PRESETS`, walked — data, one place, no code per
//! preset. Rows are `Field::ALL`, walked — every constant the module has, so
//! a constant added to `constants.rs` grows a row here without this file
//! being edited.

use jidousha::prelude::*;

use crate::constants::{Field, Tuning};
use crate::flow::Flow;
use crate::presets::PRESETS;
use crate::ui::{Panel, TextRun, columns, wrap};
use crate::{layout, presets, theme};

/// The drawer's state. **UI state, all of it** — not one field of this is read
/// by anything that decides an outcome.
#[derive(Clone, Debug)]
pub struct Tuner {
    /// Whether the drawer is open.
    pub open: bool,
    /// The set being edited. Persists until applied or overwritten by a
    /// preset; closing the drawer discards nothing.
    pub pending: Tuning,
    /// Which row the pointer is on, for its one-line meaning — this game's
    /// hover text, since a pixel-font game has no tooltip.
    pub hover: Option<Field>,
    /// What a rejected `?constants=` said, if one was rejected at startup.
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

/// Whether the pending set differs from the one in effect — what lights
/// `APPLY` and what draws a value in gold.
pub fn dirty(pending: &Tuning, active: &Tuning) -> bool {
    pending != active
}

/// Everything the drawer says, as data (`ui::Panel`, like every other screen).
pub fn drawer(flow: &Flow, active: &Tuning) -> Panel {
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
    if tuner.fault.is_none() {
        panel.block(
            Vec2::new(layout::tuner_hint().x, below + 4.0),
            &wrap(
                "APPLY restarts the scenario with the new values. every recording and verify \
                 report is stamped with the constants in effect.",
                prose,
            ),
            theme::SMALL,
            theme::FAINT,
        );
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
        &format!("{}\nseed {}", active.readout(), flow.seed),
        theme::SMALL,
        theme::INK,
    );
    // Every run of `panel.block` above draws on the base text band; the
    // drawer is an overlay, so they are lifted here rather than at each call.
    for run in &mut panel.runs {
        run.layer = theme::layers::OVERLAY_TEXT;
    }
    panel
}

/// The pointer, every tick: the handle, and the drawer's own rows.
///
/// Returns whether the click was the drawer's. **A click inside the drawer's
/// rectangle is always the drawer's**, hit or miss — it covers the screen,
/// and a click that fell through it would act on a marker the player cannot
/// see.
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
            // Opening it is the acknowledgement a refused link was waiting
            // for.
            flow.tuner.fault = None;
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

    // A preset replaces the pending set outright and touches nothing else:
    // the scenario is still running at the active constants.
    for (index, preset) in PRESETS.iter().enumerate() {
        if layout::tuner_preset(index).contains(at) {
            world.resource_mut::<Flow>().tuner.pending = preset.tuning;
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

/// Commit the pending set: swap the resource, and restart the scenario.
///
/// **The two halves are one action.** Constants are simulation inputs and a
/// run is only reproducible if the constants it ran with were in effect for
/// all of it; a swap without the restart would leave half the journey walked
/// at one cost table and half at another, and the stamp on the recording
/// would be true of neither.
pub fn apply(world: &mut World, tick: u64) {
    let (pending, active) = {
        let flow = world.resource::<Flow>();
        (flow.tuner.pending, *world.resource::<Tuning>())
    };
    if !dirty(&pending, &active) {
        return;
    }
    world.insert_resource(pending);
    crate::flow::load_scenario(world);
    let flow = world.resource_mut::<Flow>();
    // `load_scenario` clears the drawers; this restart is the one the drawer
    // asked for, so it stays up for the next A/B step.
    flow.tuner.open = true;
    flow.note(format!("constants applied - {}", pending.stamp()));
    flow.bounce(
        tick,
        format!(
            "constants applied - the scenario restarts - {}",
            name_of(&pending).unwrap_or("a hand-stepped set")
        ),
    );
}

/// The named preset a set is, if it is one — what a report says instead of
/// eight numbers.
pub fn name_of(tuning: &Tuning) -> Option<&'static str> {
    presets::PRESETS
        .iter()
        .find(|preset| preset.tuning == *tuning)
        .map(|preset| preset.name)
}
