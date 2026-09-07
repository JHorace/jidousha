//! **The postings ledger**, and the standing rates beside it — the player's
//! own two surfaces for the asks module (GDD's postings section, wave 1.2).
//!
//! One drawer, two bands, for the reason the feed drawer carries the notices
//! band: they are two readings of one subject, and a player deciding what to
//! pay is a player looking at what they have already promised. The ledger
//! band is **a view of `Sim::postings`** — every row derived from the record
//! on every draw, so there is no state here that could disagree with what the
//! scorer weighs; the rates band is four rows and eight steppers over
//! `Sim::rates`, which is the one place a rate is written.
//!
//! Everything reads the world through the [`Lens`], like every other surface.

use crate::asks::{self, Answer, Status, Who};
use crate::flow::Flow;
use crate::lens::Lens;
use crate::panels::clipped;
use crate::traits::TaskType;
use crate::ui::{Panel, TextRun, columns, wrap};
use crate::{layout, theme};

/// The ledger drawer: what the player has asked for, and what the settlement
/// pays for work.
pub fn ledger_drawer(flow: &Flow, lens: &Lens<'_>) -> Panel {
    let mut panel = Panel::default();
    panel.text(TextRun::over(
        layout::ledger_title(),
        "LEDGER - every posting you have made, newest first",
        theme::SMALL,
        theme::DIM,
    ));
    panel.text(TextRun::over(
        layout::ledger_note(),
        clipped(
            "a posting binds nobody until it is heard; withdrawing is instant",
            layout::LEDGER_NOTE_W,
        ),
        theme::SMALL,
        theme::FAINT,
    ));

    let standing: Vec<&asks::Posting> = lens
        .postings()
        .iter()
        .rev()
        .take(layout::LEDGER_ROWS)
        .collect();
    for (row, posting) in standing.iter().enumerate() {
        let at = layout::ledger_row(row).min;
        let tone = match posting.status {
            Status::Open => theme::INK,
            Status::Filled => theme::REGARD,
            Status::Withdrawn => theme::FAINT,
        };
        panel.text(TextRun::over(
            at + layout::ledger::HEAD,
            clipped(
                &format!("{} - {}", posting.status.name(), posting.line(lens)),
                layout::LEDGER_ROW_W,
            ),
            theme::SMALL,
            tone,
        ));
        panel.text(TextRun::over(
            at + layout::ledger::ANSWER,
            clipped(&answers_line(lens, posting), layout::LEDGER_ROW_W),
            theme::SMALL,
            theme::DIM,
        ));
        if posting.status == Status::Open {
            let button = layout::ledger_withdraw(row);
            panel.text(TextRun::over(
                crate::ui::centered(button, "WITHDRAW", theme::SMALL, button.min.y + 10.0),
                "WITHDRAW",
                theme::SMALL,
                theme::EMBER,
            ));
        }
    }
    if lens.postings().is_empty() {
        panel.text(TextRun::over(
            layout::ledger_row(0).min + layout::ledger::HEAD,
            "nothing posted yet - tap a job on a site's board to ask somebody for it",
            theme::SMALL,
            theme::FAINT,
        ));
    }

    // ── the standing rates: four rows, and what they price ────────────────
    panel.text(TextRun::over(
        layout::rates_title(),
        "STANDING RATES",
        theme::SMALL,
        theme::GOLD,
    ));
    for (row, task) in TaskType::ALL.iter().copied().enumerate() {
        panel.text(TextRun::over(
            layout::rates_name(row),
            task.id(),
            theme::SMALL,
            theme::INK,
        ));
        for (rect, glyph) in [(layout::rates_down(row), "-"), (layout::rates_up(row), "+")] {
            panel.text(TextRun::over(
                crate::ui::centered(rect, glyph, theme::BODY, rect.min.y + 10.0),
                glyph,
                theme::BODY,
                theme::INK,
            ));
        }
        let value = layout::rates_value(row);
        let money = format!("{}g", lens.standing_rate(task));
        panel.text(TextRun::over(
            crate::ui::centered(value, &money, theme::SMALL, value.min.y + 10.0),
            money,
            theme::SMALL,
            theme::GOLD,
        ));
        // **The standing posting** — this kind of work, to anyone, until
        // withdrawn, at the rate beside it. Lit while one already stands, so
        // the button says what the ledger holds.
        let standing = lens.postings().iter().any(|posting| {
            posting.status == Status::Open
                && posting.who == Who::Anyone
                && posting.what == asks::What::Task(task)
        });
        let button = layout::rates_post(row);
        let label = if standing { "STANDS" } else { "STAND" };
        panel.text(TextRun::over(
            crate::ui::centered(button, label, theme::SMALL, button.min.y + 10.0),
            label,
            theme::SMALL,
            if standing { theme::GOLD } else { theme::REGARD },
        ));
    }
    panel.block(
        layout::rates_note(),
        &wrap(
            "what a posting offers unless you change it, and what people expect to be paid.",
            columns(layout::RATES_NOTE_W, theme::SMALL),
        ),
        theme::SMALL,
        theme::FAINT,
    );
    panel.text(TextRun::over(
        layout::ledger_footer(),
        clipped(
            "raise a rate and its work fills easier, with nobody named",
            layout::LEDGER_ROW_W,
        ),
        theme::SMALL,
        theme::FAINT,
    ));
    let _ = flow;
    for run in &mut panel.runs {
        run.layer = theme::layers::OVERLAY_TEXT;
    }
    panel
}

/// A posting's second line: who has heard it, and what anybody said.
///
/// Derived from the record's own `heard` and `answered` lists — the ledger is
/// a view, so this is a formatting of the posting and never a second tally.
fn answers_line(lens: &Lens<'_>, posting: &asks::Posting) -> String {
    // Made at, then who has heard it, then what anybody said — the record's
    // own three columns, in the order they happen.
    let made = crate::clock::stamp(posting.made_at);
    let heard = posting
        .heard
        .iter()
        .map(|(who, _)| lens.name(*who))
        .collect::<Vec<_>>()
        .join(", ");
    let said = posting
        .answered
        .iter()
        .map(|(who, answer)| match answer {
            Answer::Agreed { .. } => format!("{} took it", lens.name(*who)),
            Answer::Declined { reason } => format!("{} said no: {reason}", lens.name(*who)),
        })
        .collect::<Vec<_>>()
        .join("; ");
    match (heard.is_empty(), said.is_empty()) {
        (true, _) => match posting.who {
            Who::Anyone => format!("{made} - unread; the camp reads the board as they think"),
            Who::Person(who) => format!(
                "{made} - unheard; a messenger is carrying it to {}",
                lens.name(who)
            ),
        },
        (false, true) => format!("{made} - heard by {heard}"),
        (false, false) => format!("{made} - {said}"),
    }
}
