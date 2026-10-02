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
use crate::flow::{Drawer, Flow};
use crate::presets::PRESETS;
use crate::ui::{Panel, TextRun, columns, wrap};
use crate::{layout, presets, theme};

/// The drawer's state. **UI state, all of it** — not one field of this is read
/// by anything that decides an outcome.
///
/// **Its openness is not here.** It moved to `Flow::drawer`, the one value
/// every drawer's openness lives in, when a tuning drawer opened over the
/// roster and both drew (`FINDINGS.md` G-027). What is left is the state that
/// is not open-or-shut, and none of it is a claim about what is on screen.
#[derive(Clone, Debug)]
pub struct Tuner {
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

/// **What APPLY is**, in the drawer's own words — the note after the resting
/// line in the prose band.
///
/// A constant, because `floors::tuner_right_column` measures the very string
/// the drawer prints: a note the floor guessed at would be a floor about a
/// different screen. Shorter since wave 1.4, when the band moved up into the
/// header and was given three rows.
pub const APPLY_NOTE: &str = "APPLY restarts the scenario; reports carry the set.";

/// What the prose band says when the player is pointing at nothing.
pub const RESTING_HINT: &str = "point at a constant to read it.";

/// **The stamp** — what is actually in effect, as the drawer and the floor
/// both measure it.
///
/// **What differs from the shipped set** (`Tuning::moved`, since wave 1.5):
/// the shipped set is in the build, so naming the difference loses nothing,
/// and the full set rides every verify report and every `?constants=` link.
/// **Packed since wave 1.6**, when the stamp moved up into the header band's
/// three rows ([`layout::TUNER_STAMP_ROWS`]): the first row is the seed and
/// the scenario, and `in effect:` leads the moved constants along the rest —
/// `name value`, a pair never split across a row — with a count of whatever
/// did not fit.
pub fn stamp_text(active: &Tuning, seed: u64, scenario: &str) -> String {
    const LEAD: &str = "in effect:";
    let width = columns(layout::TUNER_STAMP_W, theme::SMALL);
    let head = format!("seed {seed} {scenario}");
    let moved = active.moved();
    if moved.is_empty() {
        return format!("{head}\n{LEAD} the shipped set");
    }
    let budget = layout::TUNER_STAMP_ROWS.saturating_sub(1);
    let mut rows: Vec<String> = vec![LEAD.to_owned()];
    let mut shown = 0;
    for pair in &moved {
        let last = rows.len() - 1;
        let joined = if rows[last] == LEAD {
            format!("{LEAD} {pair}")
        } else {
            format!("{}, {pair}", rows[last])
        };
        if joined.len() <= width {
            rows[last] = joined;
        } else if rows.len() < budget {
            rows.push(pair.clone());
        } else {
            break;
        }
        shown += 1;
    }
    // **The count of the rest**, on the last row — taking pairs back off it
    // until the count fits, so it is never wrapped onto a row of its own.
    while shown < moved.len() {
        let rest = format!(" +{} more", moved.len() - shown);
        let last = rows.len() - 1;
        if rows[last].len() + rest.len() <= width {
            rows[last].push_str(&rest);
            break;
        }
        let cut = rows[last]
            .rfind(", ")
            .or_else(|| rows[last].starts_with(LEAD).then_some(LEAD.len()));
        match cut {
            Some(cut) if cut < rows[last].len() => {
                rows[last].truncate(cut);
                shown -= 1;
            }
            _ => {
                rows[last] = rest.trim_start().to_owned();
                break;
            }
        }
    }
    format!("{head}\n{}", rows.join("\n"))
}

/// **What the prose band says right now**, its tone, and whether the APPLY
/// note follows it — one answer, read by the drawer and measured by the floor.
pub fn prose(flow: &Flow) -> (String, Color) {
    let tuner = &flow.tuner;
    if let Some(fault) = &tuner.fault {
        (fault.clone(), theme::EMBER)
    } else if let Some(toast) = &flow.toast {
        (toast.text.clone(), theme::GOLD)
    } else if let Some(field) = tuner.hover {
        (
            format!("{} - {}", field.name(), field.meaning()),
            theme::DIM,
        )
    } else {
        (format!("{RESTING_HINT} {APPLY_NOTE}"), theme::FAINT)
    }
}

/// Everything the drawer says, as data (`ui::Panel`, like every other screen).
pub fn drawer(flow: &Flow, active: &Tuning) -> Panel {
    let tuner = &flow.tuner;
    let mut panel = Panel::default();
    panel.text(TextRun::over(
        layout::tuner_title(),
        Drawer::Tune.title(),
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

    // --- the stamp, in the header band (wave 1.6) ---------------------------
    //
    // The stamp is the one thing in the drawer that has to stay legible while
    // every other row is being moved; it packs into the header's three rows
    // (`stamp_text`), which no constant added can push into.
    let stamp = stamp_text(active, flow.seed, flow.scenario);
    panel.block(layout::tuner_stamp(), &stamp, theme::SMALL, theme::INK);

    // **The prose band, at the fourth column's foot** (since wave 1.6; the
    // header from 1.4): the hint and the note are one band and only one state
    // of it is up at a time — a hovered constant's meaning, a refused link,
    // an applied set, or the resting line with the APPLY note after it.
    // `floors::tuner_right_column` measures every one of those states under
    // one more constant than the drawer has.
    let (hint, tone) = prose(flow);
    panel.block(
        layout::tuner_hint(),
        &wrap(&hint, columns(layout::TUNER_HINT_W, theme::SMALL)),
        theme::SMALL,
        tone,
    );
    // Every run of `panel.block` above draws on the base text band; the
    // drawer is an overlay, so they are lifted here rather than at each call.
    for run in &mut panel.runs {
        run.layer = theme::layers::OVERLAY_TEXT;
    }
    panel
}

/// The pointer's hover, every tick — the drawer's own rows, so a row's
/// meaning appears by pointing at it.
///
/// **Hover only.** The handle is `flow.rs`'s, with the other four, and the
/// click is [`click`]: one place decides which drawer is open, and one match
/// decides which drawer a click goes to.
pub fn hover(world: &mut World, at: Vec2) {
    let hover = world
        .resource::<Flow>()
        .showing(Drawer::Tune)
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
}

/// A click inside the open tuning drawer.
///
/// **Every click inside the drawer is the drawer's**, hit or miss — it covers
/// the screen, and one that fell through would act on a marker the player
/// cannot see. The handle in the top bar is the way out, exactly as it is for
/// every other drawer.
pub fn click(world: &mut World, at: Vec2, tick: u64) {
    if !layout::tuner_panel().contains(at) {
        return;
    }

    // A preset replaces the pending set outright and touches nothing else:
    // the scenario is still running at the active constants.
    for (index, preset) in PRESETS.iter().enumerate() {
        if layout::tuner_preset(index).contains(at) {
            world.resource_mut::<Flow>().tuner.pending = preset.tuning;
            return;
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
        return;
    }
    if layout::tuner_apply().contains(at) {
        apply(world, tick);
    }
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
    // `load_scenario` clears the drawer; this restart is the one the drawer
    // asked for, so it stays up for the next A/B step.
    flow.drawer = Some(Drawer::Tune);
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
