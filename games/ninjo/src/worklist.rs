//! **The work list**: the open work of the whole settlement, read for one
//! person (UI.md §3f).
//!
//! **The person-side mirror of the candidate picker.** The board serves one
//! decision — whom to post a job to — approached from the job: here is a job,
//! here is everybody, here is what each of them brings to it. This serves the
//! same decision approached from the person: here is somebody, here is every
//! job standing open anywhere, here is what each of them would cost them and
//! what they would say about it. The owner asked for it after a playtest
//! where the only way to find work for a character was to open four boards in
//! turn and remember.
//!
//! **It navigates; it does not post.** Tapping a row opens that site's board,
//! with the character still selected, and the board's row makes the posting
//! exactly as it has since wave 1.2. There is one way to post, and this is
//! not a second one — it is the second way to *arrive* at the one there is,
//! which is what the picker is too.
//!
//! **Nothing here computes an answer the simulation also computes.** A row's
//! fit is `traits::competence_at` through [`Lens::competence`], its verdict is
//! the `answers::Reading` a job row's own headline comes out of (through
//! [`board::reading_at`], at the job's standing rate), and its journey is
//! `sim::route_out` through [`Lens::travel`] from wherever the person is
//! standing. Three functions, the same three the board and the picker call.

use crate::attention::CHIP;
use crate::board;
use crate::constants::Tuning;
use crate::flow::Flow;
use crate::lens::Lens;
use crate::panels::clipped;
use crate::sim::{JobId, JobState};
use crate::ui::{IconRun, Panel, TextRun};
use crate::{layout, theme};

/// **One open job, read for one person** — a row of the list.
#[derive(Clone, Debug)]
pub struct Opening {
    /// Which job, by site and slot — what a tap navigates to.
    pub job: JobId,
    /// Their fit for this job's kind of work. **The sort key.**
    pub fit: i64,
    /// What the scorer says they would do about it at the standing rate, and
    /// why. `None` with the asks module off, which is the module's whole
    /// degrades-to sentence: there is nobody to ask.
    pub reading: Option<crate::answers::Reading>,
    /// The journey from where they stand to that site, if one reaches.
    pub route: Option<crate::path::Route>,
}

/// **How many jobs stand open anywhere** — the number on the sheet's chip.
///
/// Every open job is open to anybody, so this is a count of the settlement
/// and not of the person; what is theirs is the order the list comes back in
/// and every number on a row.
pub fn open_jobs(lens: &Lens<'_>) -> usize {
    lens.sites().iter().map(crate::sim::Site::open_count).sum()
}

/// **Every job open to this person, best fit first** (UI.md §3f).
///
/// **One function, two readers** — the list draws this and `flow.rs`
/// hit-tests against it, so the row a click lands on is the row the player
/// was looking at. A second ordering computed at the click is the classic
/// version of this bug and the picker refuses it the same way.
///
/// **Sorted by fit, descending; ties in site then authored order.** The walk
/// is sites in registry order and slots in authored order, and `sort_by_key`
/// is stable, so a tie keeps the order it arrived in and two readings of one
/// frame are the same list. **The verdict is shown and never sorted on**, for
/// the reason the picker gives: who is best at the work and who will agree to
/// it are different questions.
pub fn openings(
    lens: &Lens<'_>,
    grid: &crate::grid::Grid,
    tuning: &Tuning,
    now: u64,
    who: usize,
) -> Vec<Opening> {
    let mut out: Vec<Opening> = Vec::new();
    for (site, board) in lens.sites().iter().enumerate() {
        // The travel is the person's journey to the *site*, so it is asked
        // once a site rather than once a job — one route, every row of that
        // site's work.
        let route = lens.travel(grid, tuning, who, site);
        for slot in 0..board.quests.len() {
            if board.state(slot) != Some(JobState::Open) {
                continue;
            }
            let Some(quest) = board.quest(slot) else {
                continue;
            };
            let job = JobId { site, slot };
            out.push(Opening {
                job,
                fit: lens.competence(who, quest.task),
                reading: lens.asks_on().then(|| {
                    board::reading_at(
                        lens,
                        tuning,
                        now,
                        who,
                        job,
                        quest.task,
                        lens.standing_rate(quest.task),
                    )
                }),
                route: route.clone(),
            });
        }
    }
    out.sort_by_key(|opening| std::cmp::Reverse(opening.fit));
    out
}

/// **The work list**: every open job in the settlement, read for the selected
/// character (UI.md §3f).
///
/// It draws **instead of** the board, in the board's own rectangle, for the
/// reason the candidate picker does: the left of the screen is one column and
/// a surface drawn under another is a row nobody can read lying across a
/// control somebody can click.
pub fn work_list(
    flow: &Flow,
    lens: &Lens<'_>,
    grid: &crate::grid::Grid,
    tuning: &Tuning,
    now: u64,
    who: usize,
) -> Panel {
    let mut panel = Panel::default();
    let openings = openings(lens, grid, tuning, now, who);
    panel.text(TextRun::new(
        layout::worklist_title(),
        clipped(
            &format!("the work open to {}", lens.name(who)),
            layout::WORKLIST_HEAD_W,
        ),
        theme::SMALL,
        theme::INK,
    ));
    // **What the answers below were read at, and how many of them there
    // are**, on a line of its own: the wage because a verdict is about an
    // offer and this list's offer is the standing rate, and the count because
    // the column holds ten and the list may be longer (the declared cap).
    // What order they are in is the footer's, where the picker says it too —
    // this line would clip at twenty-four of twenty-four otherwise, and what a
    // clip takes is the tail.
    let shown = openings.len().min(layout::WORK_ROWS);
    panel.text(TextRun::new(
        layout::worklist_note(),
        clipped(
            &format!("{shown} of {} open - at the standing rate", openings.len()),
            layout::WORKLIST_HEAD_W,
        ),
        theme::SMALL,
        theme::GOLD,
    ));
    let close = layout::board_close();
    panel.text(TextRun::new(
        crate::ui::centered(close, "X", theme::BODY, close.min.y + 10.0),
        "X",
        theme::BODY,
        theme::INK,
    ));
    // **The board's own fit chip, in the board's own place** — one chip, one
    // flag, one sentence (`asks::fit_means`), because fit described in a
    // third voice is three surfaces somebody has to keep in step.
    let chip = layout::board_fit_chip();
    panel.text(TextRun::new(
        crate::ui::centered(chip, "what is fit?", theme::SMALL, chip.min.y + 10.0),
        "what is fit?",
        theme::SMALL,
        if flow.fit_explained {
            theme::GOLD
        } else {
            theme::DIM
        },
    ));

    for (row, opening) in openings.iter().take(layout::WORK_ROWS).enumerate() {
        let at = layout::worklist_row(row).min;
        let Some(quest) = lens
            .site(opening.job.site)
            .and_then(|board| board.quest(opening.job.slot))
        else {
            continue;
        };
        let art = quest.task.aptitude().icon();
        panel.icon(IconRun::new(
            at + layout::work::TASK_ICON,
            art,
            art.scale_across(CHIP),
        ));
        panel.text(TextRun::new(
            at + layout::work::NAME,
            clipped(quest.name, layout::work::NAME_W),
            theme::SMALL,
            theme::INK,
        ));
        panel.text(TextRun::new(
            at + layout::work::POT,
            clipped(&format!("{}g", quest.pot), layout::work::POT_W),
            theme::SMALL,
            theme::GOLD,
        ));
        panel.text(TextRun::new(
            at + layout::work::DURATION,
            clipped(&format!("{} min", quest.duration), layout::work::DURATION_W),
            theme::SMALL,
            theme::DIM,
        ));
        panel.text(TextRun::new(
            at + layout::work::FIT,
            clipped(&format!("fit {}", opening.fit), layout::work::FIT_W),
            theme::SMALL,
            if opening.fit > 0 {
                theme::GOLD
            } else {
                theme::FAINT
            },
        ));
        panel.text(TextRun::new(
            at + layout::work::WHERE,
            clipped(
                crate::grid::LOCATIONS[crate::sim::site_location(opening.job.site)].name,
                layout::work::WHERE_W,
            ),
            theme::SMALL,
            theme::DIM,
        ));
        panel.text(TextRun::new(
            at + layout::work::TRAVEL,
            clipped(
                &match &opening.route {
                    Some(route) => format!("{} min away", route.cost),
                    None => "no way there".to_owned(),
                },
                layout::work::TRAVEL_W,
            ),
            theme::SMALL,
            theme::GOLD,
        ));
        // **What the scorer says they would do about it**, read-only and at
        // the standing rate — the same sentence a board row carries, from the
        // same function, so a player who walks this row into its board reads
        // one answer twice rather than two answers once each.
        let (says, tone) = match &opening.reading {
            Some(reading) => (
                format!("{} - {}", reading.verdict.name(), reading.why),
                if reading.verdict.takes() {
                    theme::REGARD
                } else {
                    theme::EMBER
                },
            ),
            None => (
                "asks are off - nobody can be asked for this".to_owned(),
                theme::FAINT,
            ),
        };
        panel.text(TextRun::new(
            at + layout::work::SAYS,
            clipped(&says, layout::work::SAYS_W),
            theme::SMALL,
            tone,
        ));
    }

    // **The footer says what a tap does, and it says what it does not do.**
    // The gesture that posts is a board row's, and a list of work that posted
    // would be a second way to do the one thing this game has one way to do.
    let hint = if flow.fit_explained {
        crate::asks::fit_means()
    } else if openings.is_empty() {
        "nothing stands open anywhere - the settlement's work is all claimed or done".to_owned()
    } else {
        format!(
            "sorted by fit - tap a job to open its board with {} still selected, and nothing \
             is posted from here",
            lens.name(who)
        )
    };
    panel.block(
        layout::worklist_hint(),
        &crate::ui::wrap(
            &hint,
            crate::ui::columns(layout::WORKLIST_HINT_W, theme::SMALL),
        ),
        theme::SMALL,
        theme::FAINT,
    );
    panel
}
