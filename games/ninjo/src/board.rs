//! **The site panel**: one site's job board, and the order given from it
//! (UI.md §3c, DESIGN §5).
//!
//! The surface a site marker opens. It is where the player finds out *what*
//! work stands at a place — before this the marker carried a count and took
//! an order, so with six jobs a site the amount of work was visible and the
//! work itself was not (`FINDINGS.md` G-019).
//!
//! **Nothing here computes an answer the simulation also computes.** The fit
//! on a row is `traits::competence_at`, which is the scorer's own aptitude
//! term; the travel in the header is `sim::route_out`, which is the route
//! dispatch lays and the party then walks. A preview that could disagree with
//! the journey is the failure this panel is most able to cause, and one
//! decision function per question (GDD §1) is what refuses it.
//!
//! Its own file rather than another two hundred lines of `panels.rs`, and
//! `panels::clipped` is shared because every row on every surface is cut the
//! same way — by the engine's own measurement, never by an advance ratio.

use crate::attention::CHIP;
use crate::constants::Tuning;
use crate::flow::Flow;
use crate::lens::Lens;
use crate::panels::clipped;
use crate::sim::JobState;
use crate::ui::{IconRun, Panel, TextRun};
use crate::{layout, theme};

/// **The site panel**: one site's job board, which is where an order is given
/// (UI.md §3c, DESIGN §5).
///
/// One row per authored job — name, task-type chip, pot, duration, and what
/// has become of it — plus, when somebody is selected, their **fit** for each
/// row and the **travel** from their own door in the header. Both of those are
/// the sim's own answers read through the lens (`Lens::competence` is
/// `traits::competence_at`, `Lens::travel` is `sim::route_out`), never a
/// second computation: a preview that could disagree with the journey is the
/// failure this panel is most able to cause.
///
/// With nobody selected the fit column and the travel line are **absent**
/// rather than guessed at — a board read without a candidate in mind is still
/// a board, and a fit for nobody is a number about nothing.
pub fn site_board(
    flow: &Flow,
    lens: &Lens<'_>,
    grid: &crate::grid::Grid,
    tuning: &Tuning,
    site: usize,
) -> Panel {
    let mut panel = Panel::default();
    let Some(board) = lens.site(site) else {
        return panel;
    };
    let where_ = crate::grid::LOCATIONS[crate::sim::site_location(site)].name;
    panel.text(TextRun::new(
        layout::board_title(),
        clipped(
            &format!(
                "{where_} - {} of {} jobs open",
                board.open_count(),
                board.quests.len()
            ),
            layout::BOARD_HEAD_W,
        ),
        theme::SMALL,
        theme::INK,
    ));
    let close = layout::board_close();
    panel.text(TextRun::new(
        crate::ui::centered(close, "X", theme::BODY, close.min.y + 10.0),
        "X",
        theme::BODY,
        theme::INK,
    ));
    // The travel estimate, for the one selection and nobody else.
    if let Some(who) = flow.selected
        && let Some(route) = lens.travel(grid, tuning, who, site)
    {
        panel.text(TextRun::new(
            layout::board_travel(),
            clipped(
                &format!(
                    "{} - {} {}, {} min from where they stand",
                    lens.name(who),
                    route.tiles.len(),
                    if route.tiles.len() == 1 {
                        "tile"
                    } else {
                        "tiles"
                    },
                    route.cost
                ),
                layout::BOARD_HEAD_W,
            ),
            theme::SMALL,
            theme::GOLD,
        ));
    }

    for (slot, quest) in board.quests.iter().enumerate().take(layout::BOARD_ROWS) {
        let at = layout::board_row(slot).min;
        let state = board.state(slot).unwrap_or(JobState::Open);
        let open = state == JobState::Open;
        let (name_tone, state_tone, state_text) = match state {
            JobState::Open => (theme::INK, theme::REGARD, "open".to_owned()),
            JobState::Claimed { by } => {
                (theme::DIM, theme::DIM, format!("{} has it", lens.name(by)))
            }
            JobState::Done { by } => (
                theme::FAINT,
                theme::FAINT,
                format!("{} did it", lens.name(by)),
            ),
        };
        panel.text(TextRun::new(
            at + layout::job::NAME,
            clipped(quest.name, layout::job::NAME_W),
            theme::SMALL,
            name_tone,
        ));
        panel.text(TextRun::new(
            at + layout::job::POT,
            clipped(&format!("{}g", quest.pot), layout::job::POT_W),
            theme::SMALL,
            if open { theme::GOLD } else { theme::FAINT },
        ));
        panel.text(TextRun::new(
            at + layout::job::DURATION,
            clipped(&format!("{} min", quest.duration), layout::job::DURATION_W),
            theme::SMALL,
            theme::DIM,
        ));
        let art = quest.task.aptitude().icon();
        let mut icon = IconRun::new(at + layout::job::TASK_ICON, art, art.scale_across(CHIP));
        icon.tint = if open { theme::INK } else { theme::FAINT };
        panel.icon(icon);
        panel.text(TextRun::new(
            at + layout::job::TASK_NAME,
            clipped(quest.task.id(), layout::job::TASK_W),
            theme::SMALL,
            if open { theme::INK } else { theme::FAINT },
        ));
        panel.text(TextRun::new(
            at + layout::job::STATE,
            clipped(&state_text, layout::job::STATE_W),
            theme::SMALL,
            state_tone,
        ));
        // **The fit**, for the selected character and this row's own task —
        // the aptitude row the scorer multiplies in, printed rather than
        // recomputed.
        if let Some(who) = flow.selected {
            let fit = lens.competence(who, quest.task);
            panel.text(TextRun::new(
                at + layout::job::FIT,
                clipped(&format!("fit {fit}"), layout::job::FIT_W),
                theme::SMALL,
                if fit > 0 { theme::GOLD } else { theme::FAINT },
            ));
        }
    }

    let hint = match flow.selected {
        None => "click somebody, then an open job here".to_owned(),
        Some(who) if lens.at_home(who) => {
            format!("tap an open job to send {}", lens.name(who))
        }
        Some(who) => format!(
            "{} is out - only an idle character can be sent",
            lens.name(who)
        ),
    };
    panel.text(TextRun::new(
        layout::board_hint(),
        clipped(&hint, layout::BOARD_HINT_W),
        theme::SMALL,
        theme::FAINT,
    ));
    panel
}
