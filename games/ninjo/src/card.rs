//! **The petition card, and its two placements** — the ledger drawer and the
//! voicing overlay (GDD §6's card anatomy, wave 1.5; UI.md §3h).
//!
//! # One card
//!
//! [`card`] draws a petition the way the approved mockup did (2026-10-02):
//! who is asking and from which want, the resolved words, a timer bar against
//! the deadline, the reward, the "met when" line, and the declared
//! consequence as a chip whose explanation is `petitions::explain` over the
//! vocabulary row — and the card's gesture. **No need-state is printed on
//! it**: the faces list, the roster and the character panel carry desperation
//! and the source line, and a card that carried them too would be a second
//! place for one number.
//!
//! The ledger draws it beside a list; the overlay draws it over the map while
//! the world is stopped for a voicing. Same function, same anatomy, so the
//! two cannot describe one petition two ways.
//!
//! # ARRANGE navigates; GIVE is the one exception
//!
//! Nothing posts from a card. ARRANGE goes to where the condition is acted on
//! — a person's work list, a site's board, the settlement panel — the way the
//! work list navigates to a board (UI.md §3f). GIVE moves gold, and appears
//! only on a condition money answers.
//!
//! Everything reads the world through the [`Lens`].

use jidousha::prelude::*;

use crate::constants::Tuning;
use crate::flow::{Drawer, Flow};
use crate::lens::Lens;
use crate::panels::clipped;
use crate::petitions::{self, Condition, Reward};
use crate::pleas::{Credit, Petition, Status};
use crate::ui::{IconRun, Panel, TextRun, columns, wrap};
use crate::{layout, theme};

/// **The petition the world is stopped for**, if it is stopped for one — the
/// overlay's whole state, derived from the sim's own pause rather than kept
/// beside it, so the overlay cannot be up over a world that is running or
/// down over one that stopped to show it.
pub fn voicing(lens: &Lens<'_>) -> Option<usize> {
    let pause = lens.pause()?;
    if pause.class != crate::attention::EventClass::PetitionVoiced || !lens.petitions_on() {
        return None;
    }
    lens.events().get(pause.event)?.petition
}

/// **Where the card's gesture goes** — the surface its condition is acted on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrange {
    /// This person's work list.
    WorkOf(usize),
    /// This site's board, with this person selected.
    Board {
        /// Who to select.
        who: usize,
        /// Which board.
        site: usize,
    },
    /// The settlement panel.
    Works,
}

/// **The one answer to "where does ARRANGE go"**, derived from the condition
/// the deadline checks — so the gesture goes where the predicate's subject is
/// acted on and nowhere a card had to be told about.
pub fn arrange(petition: &Petition) -> Arrange {
    let condition = petition.template.condition;
    match condition {
        Condition::SentToFight | Condition::SentSomewhereNew => match petition.site {
            Some(site) => Arrange::Board {
                who: petition.who,
                site,
            },
            None => Arrange::WorkOf(petition.who),
        },
        Condition::Bench => Arrange::Works,
        Condition::PurseAtLeast
        | Condition::OtherSettled
        | Condition::CraftDone
        | Condition::PaidWork => Arrange::WorkOf(condition.subject(petition)),
    }
}

/// What the ARRANGE button says — where it goes, in words.
pub fn arrange_label(lens: &Lens<'_>, petition: &Petition) -> String {
    match arrange(petition) {
        Arrange::WorkOf(who) => format!("ARRANGE - {}'s work", lens.name(who)),
        Arrange::Board { site, .. } => format!(
            "ARRANGE - {}",
            crate::sim::plain(crate::grid::LOCATIONS[crate::sim::site_location(site)].name)
        ),
        Arrange::Works => "ARRANGE - the settlement".to_owned(),
    }
}

/// **How long is left**, in words a card can carry.
pub fn left_words(minutes: u64) -> String {
    let (days, hours) = (minutes / petitions::DAY, (minutes % petitions::DAY) / 60);
    match (days, hours) {
        (0, 0) => format!("{minutes}m left"),
        (0, hours) => format!("{hours}h left"),
        (days, hours) => format!("{days}d {hours}h left"),
    }
}

/// The share of the deadline still to run, `0.0..=1.0` — what the timer bar
/// draws. Zero for anything not running.
pub fn left_share(petition: &Petition, now: u64) -> f32 {
    let Some(voiced) = petition.voiced_at else {
        return 0.0;
    };
    if !petition.active() {
        return 0.0;
    }
    let span = petition.deadline.saturating_sub(voiced).max(1);
    (petition.left(now) as f32 / span as f32).clamp(0.0, 1.0)
}

/// What a card's reward line says — honestly, "pays in regard" where it pays
/// nothing in gold.
pub fn reward_line(petition: &Petition, tuning: &Tuning) -> String {
    match petition.template.reward {
        Reward::InRegard => format!(
            "reward: no gold; pays in regard, +{} if you arrange it",
            tuning.plea_regard
        ),
        Reward::Gold(n) => format!("reward: up to {n}g out of their purse, to whoever meets it"),
    }
}

/// How a resolved card ends, in one line.
pub fn ending(lens: &Lens<'_>, petition: &Petition) -> String {
    match petition.status {
        Status::Met { at, credit } => format!(
            "met {} - {}",
            crate::clock::stamp(at),
            match credit {
                Credit::Player => "your hand was in it".to_owned(),
                Credit::Incidental =>
                    format!("met by {}'s world, not you", lens.name(petition.who)),
            }
        ),
        Status::Failed { at } => format!(
            "failed {} - {} fired, as the card said",
            crate::clock::stamp(at),
            petition.declared().kind.id
        ),
        Status::Active | Status::Waiting => String::new(),
    }
}

/// **The card** — one petition, standing at `at`, every row in the `Panel`.
///
/// `in_ledger` is the whole difference between the two placements: the
/// ledger's card carries ARRANGE and, where money answers it, GIVE; the
/// overlay's carries LATER, which the overlay draws itself.
pub fn card(
    lens: &Lens<'_>,
    tuning: &Tuning,
    now: u64,
    petition: &Petition,
    at: Vec2,
    explained: bool,
    in_ledger: bool,
) -> Panel {
    let mut panel = Panel::default();
    let width = layout::CARD_W;
    let wide = columns(width, theme::SMALL);
    if let Some(person) = lens.person(petition.who) {
        let mut icon = IconRun::new(
            at + layout::card::PORTRAIT,
            person.icon,
            person.icon.scale_across(32.0),
        );
        icon.layer = theme::layers::OVERLAY_TEXT;
        panel.icon(icon);
    }
    panel.text(TextRun::over(
        at + layout::card::NAME,
        lens.name(petition.who),
        theme::HEAD,
        theme::GOLD,
    ));
    panel.text(TextRun::over(
        at + layout::card::SOURCE,
        clipped(
            &format!(
                "{} - {} - {}",
                petition.template.source.phrase(),
                petition.template.source.class(),
                petition.template.id
            ),
            width - layout::card::SOURCE.x,
        ),
        theme::SMALL,
        theme::DIM,
    ));
    let words = wrap(&petition.text, columns(width, theme::BODY));
    for (row, line) in words.lines().take(layout::card::TEXT_ROWS).enumerate() {
        panel.text(TextRun::over(
            at + layout::card::TEXT + Vec2::new(0.0, row as f32 * (theme::BODY + 2.0)),
            line,
            theme::BODY,
            theme::INK,
        ));
    }
    let due = match petition.status {
        Status::Active => format!(
            "due {} - {}",
            crate::clock::stamp(petition.deadline),
            left_words(petition.left(now))
        ),
        _ => format!("was due {}", crate::clock::stamp(petition.deadline)),
    };
    panel.text(TextRun::over(
        at + layout::card::DUE,
        clipped(&due, width),
        theme::SMALL,
        if petition.active() && petition.left(now) < petitions::DAY {
            theme::EMBER
        } else {
            theme::INK
        },
    ));
    panel.text(TextRun::over(
        at + layout::card::REWARD,
        clipped(&reward_line(petition, tuning), width),
        theme::SMALL,
        theme::DIM,
    ));
    let met = wrap(
        &format!(
            "met when {}",
            petition.template.condition.met_when(lens, petition)
        ),
        wide,
    );
    for (row, line) in met.lines().take(layout::card::MET_ROWS).enumerate() {
        panel.text(TextRun::over(
            at + layout::card::MET + Vec2::new(0.0, row as f32 * (theme::SMALL + 2.0)),
            line,
            theme::SMALL,
            theme::INK,
        ));
    }
    let chip = layout::card_chip(at);
    let label = format!("on failure: {}", petition.declared().kind.id);
    panel.text(TextRun::over(
        crate::ui::centered(chip, &label, theme::SMALL, chip.min.y + 10.0),
        label,
        theme::SMALL,
        if explained { theme::GOLD } else { theme::EMBER },
    ));
    if explained {
        let other = petition.other.map(|who| lens.name(who));
        let said = wrap(
            &petitions::explain(petition.declared(), tuning, other),
            wide,
        );
        for (row, line) in said.lines().take(layout::card::EXPLAIN_ROWS).enumerate() {
            panel.text(TextRun::over(
                at + layout::card::EXPLAIN + Vec2::new(0.0, row as f32 * (theme::SMALL + 2.0)),
                line,
                theme::SMALL,
                theme::DIM,
            ));
        }
    }
    if in_ledger {
        if petition.active() {
            let act = layout::card_act(at);
            let label = clipped(&arrange_label(lens, petition), act.size().x - 8.0);
            panel.text(TextRun::over(
                crate::ui::centered(act, &label, theme::SMALL, act.min.y + 10.0),
                label,
                theme::SMALL,
                theme::INK,
            ));
            if petition.template.condition.wallet_shaped() {
                let give = layout::card_give(at);
                let (label, tone, after) = match lens.gift(petition.id) {
                    Ok(amount) => (
                        format!("GIVE {amount}g"),
                        theme::GROUND,
                        "money instead of attention: nobody works for it".to_owned(),
                    ),
                    Err(refused) => (
                        format!("GIVE {}g", petition.n),
                        theme::FAINT,
                        format!("no gift: {}", refused.message()),
                    ),
                };
                panel.text(TextRun::over(
                    crate::ui::centered(give, &label, theme::SMALL, give.min.y + 10.0),
                    label,
                    theme::SMALL,
                    tone,
                ));
                panel.text(TextRun::over(
                    at + layout::card::AFTER,
                    clipped(&after, width),
                    theme::SMALL,
                    theme::FAINT,
                ));
            }
        } else {
            panel.text(TextRun::over(
                at + layout::card::ACT + Vec2::new(0.0, 10.0),
                clipped(&ending(lens, petition), width),
                theme::SMALL,
                match petition.status {
                    Status::Met { .. } => theme::REGARD,
                    _ => theme::EMBER,
                },
            ));
        }
    }
    panel
}

/// **The ledger's order**: the running petitions by deadline, soonest first,
/// then the resolved, most recently resolved first. A petition still waiting
/// on its messenger is not on the ledger: it binds nobody yet.
pub fn ledger_order(lens: &Lens<'_>) -> Vec<usize> {
    let mut active: Vec<&Petition> = lens
        .petitions()
        .iter()
        .filter(|petition| petition.active())
        .collect();
    active.sort_by_key(|petition| (petition.deadline, petition.id));
    let mut resolved: Vec<&Petition> = lens
        .petitions()
        .iter()
        .filter(|petition| petition.status.resolved())
        .collect();
    resolved.sort_by_key(|petition| {
        let at = match petition.status {
            Status::Met { at, .. } | Status::Failed { at } => at,
            _ => 0,
        };
        (std::cmp::Reverse(at), std::cmp::Reverse(petition.id))
    });
    active
        .into_iter()
        .chain(resolved)
        .map(|petition| petition.id)
        .collect()
}

/// **Which petition the ledger's card shows** — the one the player tapped, or
/// the first row.
pub fn focused(flow: &Flow, lens: &Lens<'_>) -> Option<usize> {
    let order = ledger_order(lens);
    flow.plea
        .filter(|id| order.iter().take(layout::PLEA_ROWS).any(|row| row == id))
        .or_else(|| order.first().copied())
}

/// **The petition ledger drawer** (UI.md §3h): the list, and the card the
/// focused row opens.
pub fn pleas_drawer(flow: &Flow, lens: &Lens<'_>, tuning: &Tuning, now: u64) -> Panel {
    let mut panel = Panel::default();
    panel.text(TextRun::over(
        layout::pleas_title(),
        Drawer::Pleas.title(),
        theme::SMALL,
        theme::GOLD,
    ));
    let order = ledger_order(lens);
    let asking = lens.petitions().iter().filter(|p| p.active()).count();
    let shown = order.len().min(layout::PLEA_ROWS);
    let count = if order.len() > shown {
        format!(
            "{asking} asking, {} resolved - {shown} of {} shown",
            order.len() - asking,
            order.len()
        )
    } else {
        format!("{asking} asking, {} resolved", order.len() - asking)
    };
    // **A stop that lands while the ledger is open says why here.** An open
    // drawer silences the map's banner, and LATER is what puts the player in
    // this one - so without this line a failure's pause stopped the world
    // and said nothing (found in the wave 1.5 browser playtest). The feed's
    // header says the same sentence from the same function.
    let (note, tone) = match crate::attention::reason_line(lens) {
        Some(reason) => (reason, theme::GOLD),
        None => (
            format!("{count} - no accept, no decline: ARRANGE goes where it is answered"),
            theme::DIM,
        ),
    };
    panel.text(TextRun::over(
        layout::pleas_note(),
        clipped(&note, layout::PLEAS_NOTE_W),
        theme::SMALL,
        tone,
    ));
    if !lens.petitions_on() {
        panel.text(TextRun::over(
            layout::plea_row(0).min + layout::plea::HEAD,
            "petitions are off - nobody asks you for anything",
            theme::SMALL,
            theme::FAINT,
        ));
    } else if order.is_empty() {
        panel.text(TextRun::over(
            layout::plea_row(0).min + layout::plea::HEAD,
            "nobody has asked you for anything yet",
            theme::SMALL,
            theme::FAINT,
        ));
    }
    let focus = focused(flow, lens);
    for (row, id) in order.iter().take(layout::PLEA_ROWS).enumerate() {
        let Some(petition) = lens.petition(*id) else {
            continue;
        };
        let at = layout::plea_row(row).min;
        let head = match petition.status {
            Status::Active => format!(
                "{} - due {}",
                lens.name(petition.who),
                crate::clock::stamp(petition.deadline)
            ),
            Status::Met { at, .. } => {
                format!(
                    "{} - met {}",
                    lens.name(petition.who),
                    crate::clock::stamp(at)
                )
            }
            Status::Failed { at } => format!(
                "{} - failed {}, {}",
                lens.name(petition.who),
                crate::clock::stamp(at),
                petition.declared().kind.id
            ),
            Status::Waiting => lens.name(petition.who).to_owned(),
        };
        panel.text(TextRun::over(
            at + layout::plea::HEAD,
            clipped(&head, layout::plea::HEAD_W),
            theme::SMALL,
            match petition.status {
                Status::Active => {
                    if Some(*id) == focus {
                        theme::GOLD
                    } else {
                        theme::INK
                    }
                }
                Status::Met { .. } => theme::REGARD,
                _ => theme::EMBER,
            },
        ));
        // The row's words without the name the head already says.
        let said = petition
            .text
            .strip_prefix(&format!("{}: ", lens.name(petition.who)))
            .unwrap_or(&petition.text);
        panel.text(TextRun::over(
            at + layout::plea::TEXT,
            clipped(said, layout::plea::TEXT_W),
            theme::SMALL,
            if petition.active() {
                theme::DIM
            } else {
                theme::FAINT
            },
        ));
    }
    if let Some(id) = focus
        && let Some(petition) = lens.petition(id)
    {
        panel.absorb(card(
            lens,
            tuning,
            now,
            petition,
            layout::plea_card(),
            flow.consequence_open,
            true,
        ));
    }
    for run in &mut panel.runs {
        run.layer = theme::layers::OVERLAY_TEXT;
    }
    panel
}

/// **The voicing overlay** (UI.md §3h): the card, front and centre, while the
/// world is stopped for it — and LATER, which dismisses it to the ledger.
pub fn voicing_overlay(flow: &Flow, lens: &Lens<'_>, tuning: &Tuning, now: u64) -> Panel {
    let mut panel = Panel::default();
    let Some(id) = voicing(lens) else {
        return panel;
    };
    let Some(petition) = lens.petition(id) else {
        return panel;
    };
    panel.text(TextRun::over(
        layout::voicing_stamp(),
        clipped("PETITION VOICED - the world stopped for it", layout::CARD_W),
        theme::SMALL,
        theme::GOLD,
    ));
    panel.absorb(card(
        lens,
        tuning,
        now,
        petition,
        layout::voicing_card(),
        flow.consequence_open,
        false,
    ));
    let later = layout::card_act(layout::voicing_card());
    panel.text(TextRun::over(
        crate::ui::centered(later, LATER, theme::SMALL, later.min.y + 10.0),
        LATER,
        theme::SMALL,
        theme::INK,
    ));
    panel.text(TextRun::over(
        layout::voicing_hint(),
        "or space resumes",
        theme::SMALL,
        theme::FAINT,
    ));
    for run in &mut panel.runs {
        run.layer = theme::layers::OVERLAY_TEXT;
    }
    panel
}

/// The overlay's one gesture.
pub const LATER: &str = "LATER - to the ledger";

/// **The fills under a card** — its timer bar, its chip and its buttons —
/// drawn on the overlay band like every drawer's ghosts.
pub fn draw_card_ground(
    ctx: &mut DrawCtx,
    map: &crate::camera::UiMap,
    lens: &Lens<'_>,
    petition: &Petition,
    at: Vec2,
    now: u64,
    in_ledger: bool,
) {
    let fill = |ctx: &mut DrawCtx, rect: Rect, color: Color, layer: i16| {
        crate::ui::fill(ctx, map.to_world_rect(rect), color, layer);
    };
    let track = layout::card_bar(at);
    let layer = theme::layers::OVERLAY + 1;
    fill(ctx, track, theme::GHOST, layer);
    let share = left_share(petition, now);
    if share > 0.0 {
        let tone = if petition.left(now) < petitions::DAY {
            theme::EMBER
        } else {
            theme::GOLD
        };
        fill(
            ctx,
            Rect::from_min_size(track.min, Vec2::new(track.size().x * share, track.size().y)),
            tone,
            layer,
        );
    }
    let ghost = |ctx: &mut DrawCtx, rect: Rect| {
        crate::ui::fill(ctx, map.to_world_rect(rect), theme::GHOST, layer);
        crate::ui::border(
            ctx,
            map.to_world_rect(rect),
            theme::BORDER,
            2.0 * map.scale,
            layer,
        );
    };
    ghost(ctx, layout::card_chip(at));
    if !in_ledger {
        ghost(ctx, layout::card_act(at));
    } else if petition.active() {
        ghost(ctx, layout::card_act(at));
        if petition.template.condition.wallet_shaped() {
            if lens.gift(petition.id).is_ok() {
                crate::ui::button(ctx, map.to_world_rect(layout::card_give(at)), true, layer);
            } else {
                ghost(ctx, layout::card_give(at));
            }
        }
    }
}
