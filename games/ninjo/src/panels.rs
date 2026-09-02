//! The attention surfaces, as data: the feed, the auto-pause config, the
//! meters and their faces, and one character's panel (GDD §3, wave 0a).
//!
//! Every function here hands back a [`Panel`] — every string and every icon
//! with its position — because that is what makes three readers of one layout
//! possible (`ui.rs`): the draw system turns it into quads, `floors.rs` judges
//! what was meant, and `frames.rs` finds it on the recorded frame.
//!
//! **Everything reads the world through the [`Lens`]**, including the
//! auto-pause config, which is simulation state and not a copy kept beside the
//! screen. A panel here takes no `Sim`, so none of them can reach around it.

use jidousha::prelude::*;

use crate::attention::{self, CHIP, EventClass, FeedEntry, Mode};
use crate::constants::Tuning;
use crate::flow::Flow;
use crate::lens::Lens;
use crate::meters::{self, METERS};
use crate::sprites::Art;
use crate::ui::{IconRun, Panel, TextRun, columns, wrap};
use crate::{layout, theme};

/// An icon on an overlay's band.
fn over_icon(at: Vec2, art: Art, units: f32) -> IconRun {
    IconRun {
        layer: theme::layers::OVERLAY_TEXT,
        ..IconRun::new(at, art, art.scale_across(units))
    }
}

/// As much of `text` as fits across `width`, cut at a word and marked.
///
/// The engine's own measurement, so no advance ratio appears in this game
/// (`jidousha-api.md`: `fits_in` is the tight answer for a string you have).
/// The cut falls back to the last space and leaves three dots, because a row
/// that stops mid-word reads as a rendering fault rather than as a row that
/// ran out of drawer.
fn clipped(text: &str, width: f32) -> String {
    let style = theme::text(theme::SMALL, theme::INK);
    let fits = style.fits_in(text, width);
    if fits >= text.chars().count() {
        return text.to_owned();
    }
    let head: String = text.chars().take(fits.saturating_sub(3)).collect();
    let cut = head.rfind(' ').unwrap_or(head.len());
    format!("{}...", head[..cut].trim_end())
}

/// The feed drawer: the sim's event log, as a view.
///
/// One row per entry, and the row's anatomy is the mockup's: **world
/// timestamp, class chip (colour and icon), the sentence, and the place**.
/// The entry an auto-pause fired on is drawn in gold, which is why the reason
/// line above it and the row below it cannot disagree — both come off
/// `Lens::pause`.
pub fn feed_drawer(flow: &Flow, lens: &Lens<'_>, tuning: &Tuning) -> Panel {
    let mut panel = Panel::default();
    panel.text(TextRun::over(
        layout::feed_title(),
        "FEED - what happened, newest first - click an entry to look at it",
        theme::SMALL,
        theme::DIM,
    ));
    let (reason, tone) = match attention::reason_line(lens) {
        Some(line) => (line, theme::GOLD),
        None => (
            "the world is running - nothing has stopped it".to_owned(),
            theme::FAINT,
        ),
    };
    panel.text(TextRun::over(
        layout::feed_reason(),
        clipped(&reason, layout::FEED_REASON_W),
        theme::SMALL,
        tone,
    ));
    let toggle = layout::feed_ignored_toggle();
    let toggle_label = if flow.show_ignored {
        "IGNORED: SHOWN"
    } else {
        "IGNORED: HIDDEN"
    };
    panel.text(TextRun::over(
        crate::ui::centered(toggle, toggle_label, theme::SMALL, toggle.min.y + 10.0),
        toggle_label,
        theme::SMALL,
        if flow.show_ignored {
            theme::GOLD
        } else {
            theme::DIM
        },
    ));

    let entries = attention::feed(lens, flow.show_ignored, attention::feed_cap(tuning));
    let triggered = lens.pause().map(|pause| pause.event);
    for (row, entry) in entries.iter().take(layout::FEED_ROWS).enumerate() {
        let Some(event) = lens.events().get(entry.index) else {
            continue;
        };
        let at = layout::feed_row(row).min;
        let spec = event.class.spec();
        let highlit = triggered == Some(entry.index);
        let (stamp_tone, class_tone, text_tone, place_tone) = tones(entry, highlit, spec.color);
        panel.text(TextRun::over(
            at + layout::entry::STAMP,
            crate::clock::stamp(event.minute),
            theme::SMALL,
            stamp_tone,
        ));
        let mut icon = over_icon(at + layout::entry::CHIP_ICON, spec.icon, CHIP);
        icon.tint = class_tone;
        panel.icon(icon);
        panel.text(TextRun::over(
            at + layout::entry::CHIP_NAME,
            spec.id,
            theme::SMALL,
            class_tone,
        ));
        panel.text(TextRun::over(
            at + layout::entry::PLACE,
            clipped(&format!("- {}", attention::place_tag(event)), 200.0),
            theme::SMALL,
            place_tone,
        ));
        panel.text(TextRun::over(
            at + layout::entry::TEXT,
            clipped(&event.text(lens), layout::entry::TEXT_W),
            theme::SMALL,
            text_tone,
        ));
    }
    if entries.is_empty() {
        panel.text(TextRun::over(
            layout::feed_row(0).min + layout::entry::TEXT,
            "nothing yet - the world opens paused, and space runs it",
            theme::SMALL,
            theme::FAINT,
        ));
    }

    // The notices band: what the *player* did, and what bounced. Kept apart
    // from the feed on purpose — the feed is the world's, and mixing the two
    // would be the second list this surface exists not to have.
    panel.text(TextRun::over(
        layout::notices_title(),
        "NOTICES - speed, refused orders, restarts",
        theme::SMALL,
        theme::FAINT,
    ));
    for (index, line) in flow.log.iter().take(layout::NOTICE_ROWS).enumerate() {
        panel.text(TextRun::over(
            layout::notice_row(index),
            clipped(line, 900.0),
            theme::SMALL,
            theme::DIM,
        ));
    }
    panel
}

/// A feed row's four colours: dimmed throughout when the row is only visible
/// because ignored classes are shown, gold throughout when it is the entry an
/// auto-pause fired on.
fn tones(entry: &FeedEntry, highlit: bool, class: Color) -> (Color, Color, Color, Color) {
    if entry.ignored {
        return (theme::FAINT, theme::FAINT, theme::FAINT, theme::FAINT);
    }
    if highlit {
        return (theme::GOLD, class, theme::GOLD, theme::GOLD);
    }
    (theme::DIM, class, theme::INK, theme::DIM)
}

/// The auto-pause config drawer: every registered class, and what it does.
///
/// The rows are [`attention::CLASSES`] walked, so a wave-1 module's class
/// appears here by existing. Nothing about this panel knows what a class
/// *means*.
pub fn modes_drawer(lens: &Lens<'_>) -> Panel {
    let mut panel = Panel::default();
    panel.text(TextRun::over(
        layout::modes_title(),
        "AUTO-PAUSE - what each kind of event does to the world",
        theme::SMALL,
        theme::GOLD,
    ));
    let prose = columns(layout::modes_prose_width(), theme::SMALL);
    let note = Mode::ALL
        .iter()
        .map(|mode| format!("{}: {}", mode.name(), mode.meaning()))
        .collect::<Vec<_>>()
        .join(".  ");
    panel.block(
        layout::modes_note(),
        &wrap(&note, prose),
        theme::SMALL,
        theme::DIM,
    );
    for (index, class) in EventClass::all().into_iter().enumerate() {
        let spec = class.spec();
        let mut icon = over_icon(layout::modes_icon(index), spec.icon, CHIP);
        icon.tint = spec.color;
        panel.icon(icon);
        panel.text(TextRun::over(
            layout::modes_name(index),
            spec.id,
            theme::SMALL,
            spec.color,
        ));
        let held = lens.attention().mode(class);
        for (slot, mode) in Mode::ALL.iter().copied().enumerate() {
            let button = layout::modes_radio(index, slot);
            panel.text(TextRun::over(
                crate::ui::centered(button, mode.name(), theme::SMALL, button.min.y + 10.0),
                mode.name(),
                theme::SMALL,
                if mode == held {
                    theme::GROUND
                } else {
                    theme::DIM
                },
            ));
        }
    }
    panel.block(
        layout::modes_footer(),
        &wrap(
            "a change here is a recorded input and the config is part of the world: a replay \
             pauses at the same world-minutes, for the same reasons.",
            prose,
        ),
        theme::SMALL,
        theme::FAINT,
    );
    // Everything above is drawn on the overlay's own band.
    for run in &mut panel.runs {
        run.layer = theme::layers::OVERLAY_TEXT;
    }
    panel
}

/// The glance: the meter chips, the pause banner, the faces list a chip opens,
/// and the panel a face opens.
///
/// All of it sits over the map rather than in a drawer, because these are the
/// surfaces the player is meant to read without asking for them.
pub fn glance(flow: &Flow, lens: &Lens<'_>) -> Panel {
    let mut panel = Panel::default();
    for (index, spec) in METERS.iter().enumerate() {
        let chip = layout::meter_chip(index);
        let count = meters::count(lens, index);
        // **A chip colourises only when it has something to say** (the
        // mockup's rule): a zero is a chip you are allowed to not look at.
        let tone = if count == 0 {
            theme::FAINT
        } else {
            theme::GOLD
        };
        let mut icon = IconRun::new(
            chip.min + Vec2::splat(layout::mchip::ICON),
            spec.icon,
            spec.icon.scale_across(CHIP),
        );
        icon.tint = tone;
        panel.icon(icon);
        panel.text(TextRun::new(
            chip.min + Vec2::new(layout::mchip::LABEL_X, layout::mchip::LABEL_TOP),
            format!("{} {count}", spec.label),
            theme::SMALL,
            tone,
        ));
    }
    if let Some(reason) = attention::reason_line(lens) {
        panel.text(TextRun::new(
            layout::banner_at(),
            clipped(&reason, 900.0),
            theme::SMALL,
            theme::GOLD,
        ));
    }
    if let Some(drilled) = flow.drilled {
        panel.absorb(faces_panel(lens, drilled));
    }
    if let Some(who) = flow.selected_person {
        panel.absorb(person_panel(flow, lens, who));
    }
    panel
}

/// The faces behind one chip: who is counted, and the reason each is.
fn faces_panel(lens: &Lens<'_>, index: usize) -> Panel {
    let mut panel = Panel::default();
    let label = METERS.get(index).map_or("", |spec| spec.label);
    panel.text(TextRun::new(
        layout::faces_title(),
        format!("{label} - who, and why"),
        theme::SMALL,
        theme::DIM,
    ));
    for (row, (who, reason)) in meters::faces(lens, index)
        .into_iter()
        .take(layout::FACE_ROWS)
        .enumerate()
    {
        let at = layout::faces_row(row).min;
        if let Some(person) = lens.person(who) {
            panel.icon(IconRun::new(
                at,
                person.icon,
                person.icon.scale_across(32.0),
            ));
        }
        panel.text(TextRun::new(
            at + Vec2::new(36.0, 2.0),
            lens.name(who),
            theme::SMALL,
            theme::INK,
        ));
        panel.text(TextRun::new(
            at + Vec2::new(36.0, 17.0),
            clipped(&reason, 240.0),
            theme::SMALL,
            theme::FAINT,
        ));
    }
    panel
}

/// One character, through the lens: who they are, what they carry, what
/// presses on them, what they are doing — **and why**.
///
/// The reason is the scorer's own string, read through the lens; nothing here
/// recomputes it, because a panel that computed its own answer would be the
/// second decision function GDD §1 forbids.
fn person_panel(flow: &Flow, lens: &Lens<'_>, who: usize) -> Panel {
    use layout::sheet;
    let mut panel = Panel::default();
    let origin = layout::person_panel().min;
    if let Some(person) = lens.person(who) {
        panel.icon(IconRun::new(
            origin + sheet::PORTRAIT,
            person.icon,
            sheet::PORTRAIT_SCALE,
        ));
    }
    panel.text(TextRun::new(
        origin + sheet::NAME,
        lens.name(who),
        theme::HEAD,
        theme::INK,
    ));
    let close = layout::person_close();
    panel.text(TextRun::new(
        crate::ui::centered(close, "X", theme::BODY, close.min.y + 10.0),
        "X",
        theme::BODY,
        theme::INK,
    ));
    panel.text(TextRun::new(
        origin + sheet::TRAITS,
        "traits - tap one for what it does",
        theme::SMALL,
        theme::FAINT,
    ));
    for (slot, id) in lens
        .traits(who)
        .iter()
        .copied()
        .take(layout::SHEET_CHIPS)
        .enumerate()
    {
        panel.absorb(trait_chip(
            layout::sheet_chip(slot),
            id,
            flow.explained == Some(id),
        ));
    }
    panel.icon(IconRun::new(
        origin + sheet::WALLET_ICON,
        Art::Coin,
        Art::Coin.scale_across(CHIP),
    ));
    panel.text(TextRun::new(
        origin + sheet::WALLET_TEXT,
        format!("{}g in hand", lens.wallet(who)),
        theme::SMALL,
        theme::GOLD,
    ));
    panel.icon(IconRun::new(
        origin + sheet::NEED_ICON,
        Art::Flame,
        Art::Flame.scale_across(CHIP),
    ));
    panel.text(TextRun::new(
        origin + sheet::NEED_TEXT,
        format!("desperation {}", lens.desperation(who)),
        theme::SMALL,
        theme::EMBER,
    ));
    let prose = columns(sheet::PROSE_W, theme::SMALL);
    panel.block(
        origin + sheet::SOURCE,
        &wrap(lens.source(who), prose),
        theme::SMALL,
        theme::DIM,
    );
    let doing = match lens.quest(who) {
        Some(quest) => format!("{} ({})", lens.activity_line(who), quest.name),
        None => lens.activity_line(who),
    };
    panel.block(
        origin + sheet::DOING,
        &wrap(&doing, prose),
        theme::SMALL,
        theme::INK,
    );
    let home = lens.home(who).map_or("nowhere".to_owned(), |tile| {
        format!("({}, {})", tile.x, tile.y)
    });
    panel.text(TextRun::new(
        origin + sheet::HOME,
        format!("home {home}"),
        theme::SMALL,
        theme::FAINT,
    ));
    if let Some(id) = flow.explained {
        panel.block(
            origin + sheet::EXPLAIN,
            &wrap(&crate::traits::explain(id), prose),
            theme::SMALL,
            theme::GOLD,
        );
    }
    panel
}

/// One trait chip, wherever it appears: the icon, the name, and gold when it
/// is the chip whose explanation is showing.
///
/// **One function, every surface.** A chip on a sheet and a chip on a roster
/// row are the same picture and the same target, so "tap a chip for what it
/// does" is true everywhere without anybody remembering to make it true.
fn trait_chip(rect: Rect, id: crate::traits::TraitId, lit: bool) -> Panel {
    use layout::rrow;
    let mut panel = Panel::default();
    let tone = if lit { theme::GOLD } else { theme::INK };
    let mut icon = IconRun::new(
        rect.min + rrow::CHIP_ICON,
        id.icon(),
        id.icon().scale_across(CHIP),
    );
    icon.tint = tone;
    panel.icon(icon);
    panel.text(TextRun::new(
        rect.min + rrow::CHIP_NAME,
        id.name(),
        theme::SMALL,
        tone,
    ));
    panel
}

/// **The roster**: every character in one list — name, chips, wallet,
/// desperation, and what they are doing with the reason they are doing it
/// (wave 1.1's clarity slice).
///
/// Every number and every sentence comes off the [`Lens`], including the
/// reason, which is the scorer's own words. A row's name opens that
/// character's panel; a chip opens its explanation.
pub fn roster_drawer(flow: &Flow, lens: &Lens<'_>) -> Panel {
    use layout::rrow;
    let mut panel = Panel::default();
    panel.text(TextRun::over(
        layout::roster_title(),
        "ROSTER - everyone, what they carry, and what they are doing about it",
        theme::SMALL,
        theme::DIM,
    ));
    let (explanation, tone) = match flow.explained {
        Some(id) => (crate::traits::explain(id), theme::GOLD),
        None => ("tap a trait chip for what it does".to_owned(), theme::FAINT),
    };
    panel.text(TextRun::over(
        layout::roster_explain(),
        clipped(&explanation, layout::ROSTER_EXPLAIN_W),
        theme::SMALL,
        tone,
    ));
    for who in 0..lens.people().len().min(layout::ROSTER_ROWS) {
        let open = layout::roster_open(who);
        if let Some(person) = lens.person(who) {
            let mut face = IconRun::new(
                open.min + rrow::PORTRAIT,
                person.icon,
                layout::sheet::PORTRAIT_SCALE,
            );
            face.layer = theme::layers::OVERLAY_TEXT;
            panel.icon(face);
        }
        panel.text(TextRun::over(
            open.min + rrow::NAME,
            lens.name(who),
            theme::SMALL,
            theme::INK,
        ));
        for (slot, id) in lens
            .traits(who)
            .iter()
            .copied()
            .take(layout::SHEET_CHIPS)
            .enumerate()
        {
            let mut chip = trait_chip(
                layout::roster_chip(who, slot),
                id,
                flow.explained == Some(id),
            );
            for run in &mut chip.runs {
                run.layer = theme::layers::OVERLAY_TEXT;
            }
            for icon in &mut chip.icons {
                icon.layer = theme::layers::OVERLAY_TEXT;
            }
            panel.absorb(chip);
        }
        panel.text(TextRun::over(
            open.min + rrow::WALLET,
            format!("{}g", lens.wallet(who)),
            theme::SMALL,
            theme::GOLD,
        ));
        panel.text(TextRun::over(
            open.min + rrow::NEED,
            format!("desp {}", lens.desperation(who)),
            theme::SMALL,
            theme::EMBER,
        ));
        panel.text(TextRun::over(
            open.min + rrow::DOING,
            clipped(&lens.activity_line(who), rrow::DOING_W),
            theme::SMALL,
            theme::DIM,
        ));
    }
    panel
}
