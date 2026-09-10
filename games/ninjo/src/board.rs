//! **The site panel**: one site's job board, and the **posting** made from it
//! (UI.md §3c, GDD's postings section).
//!
//! The surface a site marker opens. It is where the player finds out *what*
//! work stands at a place — before this the marker carried a count and took
//! an order, so with six jobs a site the amount of work was visible and the
//! work itself was not (`FINDINGS.md` G-019).
//!
//! **The gesture is the same and its meaning is not.** Tapping an open row
//! with somebody selected used to order them there; since wave 1.2 it
//! **posts** the job to them at the standing rate, and they answer. The row
//! says which: it carries the wage the tap would offer, and — with somebody
//! selected — the scorer's own read of that offer, in the three words a
//! verdict has and the reason behind them.
//!
//! **Nothing here computes an answer the simulation also computes.** The fit
//! on a row is `traits::competence_at`, which is the scorer's own aptitude
//! term; the travel in the header is `sim::route_out`, which is the route
//! dispatch lays and the party then walks. A preview that could disagree with
//! the journey is the failure this panel is most able to cause, and one
//! decision function per question (GDD §1) is what refuses it.
//!
//! **And since the candidate-picker session the board can name its own
//! person.** A `who?` on each open row opens [`candidate_picker`] — the cast
//! for that job, with fit, verdict and whereabouts — because the board was the
//! one surface whose purpose required a map interaction while it was covering
//! the map (`FINDINGS.md` G-026). The picker writes the one selection and
//! nothing else; the row still makes the posting.
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
    now: u64,
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
                    "{} - {} {}, {} min away",
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

    // **The fit header** — a chip like every other chip, saying what the
    // column under it means and what it does not mean yet (wave 1.2's clarity
    // rider). Only where there is a fit column to explain.
    if flow.selected.is_some() {
        let tone = if flow.fit_explained {
            theme::GOLD
        } else {
            theme::DIM
        };
        let chip = layout::board_fit_chip();
        panel.text(TextRun::new(
            crate::ui::centered(chip, "what is fit?", theme::SMALL, chip.min.y + 10.0),
            "what is fit?",
            theme::SMALL,
            tone,
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
        let mut said = false;
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
        if lens.asks_on() {
            // **The `who?` control** — the board's own way of naming a person
            // for this job (the candidate-picker session). On every open row,
            // whether or not anybody is selected, because a board that could
            // only be aimed by reaching a map sprite is a board that cannot be
            // used while it covers one (`FINDINGS.md` G-026). Not while the
            // board is offering to anyone: a bounty names nobody, so there is
            // no candidate to pick and the click says so instead.
            if open && !flow.post_open {
                let target = layout::board_who(slot);
                panel.text(TextRun::new(
                    crate::ui::centered(target, "who?", theme::SMALL, target.min.y + 12.0),
                    "who?",
                    theme::SMALL,
                    theme::INK,
                ));
            }
            // **What the scorer says they would do about it** — read-only,
            // through the one function (`Lens::would_take`), so the row and
            // the answer are one derivation. It stands where the state would
            // be, because an open row's state is the word this replaces.
            if let Some(who) = flow.selected
                && open
            {
                let job = crate::sim::JobId { site, slot };
                let reading = reading_for(flow, lens, tuning, now, who, job, quest.task);
                let tone = if reading.verdict.takes() {
                    theme::REGARD
                } else {
                    theme::EMBER
                };
                panel.text(TextRun::new(
                    at + layout::job::SAYS,
                    clipped(
                        &format!("{} - {}", reading.verdict.name(), reading.why),
                        layout::job::SAYS_W,
                    ),
                    theme::SMALL,
                    tone,
                ));
                // **The way one tap deeper** (UI.md §3e): a target of its own
                // at the end of the row, because the row is the posting and a
                // control inside a control is what the overlap floor refuses.
                // Gold while this row's sum is the one on the band.
                let why = layout::board_why(slot);
                let lit = flow.breakdown == Some(crate::flow::Breakdown::Job(slot));
                panel.text(TextRun::new(
                    crate::ui::centered(why, "?", theme::BODY, why.min.y + 12.0),
                    "?",
                    theme::BODY,
                    if lit { theme::GOLD } else { theme::DIM },
                ));
                said = true;
            }
        }
        if !said {
            panel.text(TextRun::new(
                at + layout::job::SAYS,
                clipped(&state_text, layout::job::SAYS_W),
                theme::SMALL,
                state_tone,
            ));
        }
    }

    // **The board's own controls**: what the next tap offers, and whom it
    // offers it to. One of each, in the footer band, because the wage is a
    // thing the player is holding rather than a property of a row.
    if lens.asks_on() {
        let wage = board_wage(flow, lens, site);
        for (rect, glyph) in [
            (layout::board_wage_down(), "-"),
            (layout::board_wage_up(), "+"),
        ] {
            panel.text(TextRun::new(
                crate::ui::centered(rect, glyph, theme::BODY, rect.min.y + 10.0),
                glyph,
                theme::BODY,
                theme::INK,
            ));
        }
        let value = layout::board_wage_value();
        let money = format!("{wage}g");
        panel.text(TextRun::new(
            crate::ui::centered(value, &money, theme::SMALL, value.min.y + 10.0),
            money,
            theme::SMALL,
            theme::GOLD,
        ));
        let to = layout::board_to();
        let label = match (flow.post_open, flow.selected) {
            (true, _) => "TO ANY".to_owned(),
            (false, Some(who)) => format!("TO {}", lens.name(who)),
            (false, None) => "NOBODY".to_owned(),
        };
        panel.text(TextRun::new(
            crate::ui::centered(to, &label, theme::SMALL, to.min.y + 10.0),
            clipped(&label, 84.0),
            theme::SMALL,
            if flow.post_open {
                theme::REGARD
            } else {
                theme::GOLD
            },
        ));
    }

    // **The footer says what the tap is.** It is the one place the change of
    // verb from wave 1.1 is stated in words, and the whole risk of keeping the
    // gesture is that nobody notices it changed.
    let hint = if !lens.asks_on() {
        "asks are off - this board is a read of the work, and nobody can be asked".to_owned()
    } else if flow.fit_explained {
        crate::asks::fit_means()
    } else {
        match (flow.post_open, flow.selected) {
            (true, _) => "tap a job to post it to anyone at the wage shown".to_owned(),
            (false, None) => "click somebody, or post to anyone - then tap a job".to_owned(),
            (false, Some(who)) => format!(
                "tap a job to post it to {} - the answer is theirs",
                lens.name(who)
            ),
        }
    };
    panel.block(
        layout::board_hint(),
        &crate::ui::wrap(
            &hint,
            crate::ui::columns(layout::BOARD_HINT_W, theme::SMALL),
        ),
        theme::SMALL,
        theme::FAINT,
    );
    panel
}

/// **What this row says about this offer** — the verdict, its reason and the
/// arithmetic behind both, out of the one read.
///
/// One call, one comparison: the row's headline and the band's breakdown come
/// out of the same `answers::Reading`, so a sum that does not produce the
/// verdict beside it is not a state this surface can reach
/// (`verify::the_breakdown_is_the_judgement`).
pub fn reading_for(
    flow: &Flow,
    lens: &Lens<'_>,
    tuning: &Tuning,
    now: u64,
    who: usize,
    job: crate::sim::JobId,
    task: crate::traits::TaskType,
) -> crate::answers::Reading {
    let wage = wage_offered(flow, lens, job.site, job.slot, task);
    lens.would_take(
        tuning,
        now,
        who,
        &crate::asks::preview_posting(who, job.site, job.slot, wage, lens.standing_rate(task)),
        job,
    )
}

/// **The wage a tap on this row would offer** — the one answer, read by the
/// row that prints it, the preview that weighs it, and the posting that is
/// made from it (`flow::post_from_board`).
///
/// The standing rate for the work, unless the player has stepped this row's
/// wage off it: one row at a time carries an offer, because an offer the
/// player cannot see on the row it belongs to is a number that would surprise
/// somebody at the moment they tap.
pub fn wage_offered(
    flow: &Flow,
    lens: &Lens<'_>,
    site: usize,
    slot: usize,
    task: crate::traits::TaskType,
) -> i64 {
    let _ = (site, slot);
    flow.offer.unwrap_or_else(|| lens.standing_rate(task))
}

/// **What the board's own wage control is showing** — the offer in hand, or
/// the standing rate of the site's first open row when the player has not
/// moved it.
pub fn board_wage(flow: &Flow, lens: &Lens<'_>, site: usize) -> i64 {
    if let Some(wage) = flow.offer {
        return wage;
    }
    lens.site(site)
        .and_then(|board| {
            board
                .open_slots()
                .next()
                .and_then(|slot| board.quest(slot))
                .map(|quest| lens.standing_rate(quest.task))
        })
        .unwrap_or(0)
}

/// **One candidate for one job**, as the picker shows them.
///
/// Every field is the sim's own answer, read once through the one function
/// that answers it: the fit is `traits::competence_at` (via [`Lens::competence`]),
/// the verdict and its reason are the `answers::Reading` a job row's own
/// headline comes out of, and the journey is `sim::route_out` (via
/// [`Lens::travel`]) from wherever they are standing rather than from the door
/// they may not be at. Nothing here is recomputed, and there is nothing on a
/// candidate row that a job row would answer differently.
#[derive(Clone, Debug)]
pub struct Candidate {
    /// Which character, by registry index — the index the one selection is.
    pub who: usize,
    /// Their fit for this job's kind of work. **The sort key.**
    pub fit: i64,
    /// What the scorer says they would do about this offer, and why.
    pub reading: crate::answers::Reading,
    /// The journey from where they stand to this site, if one reaches.
    pub route: Option<crate::path::Route>,
}

/// **Everybody this job could be posted to, best fit first** (UI.md §3c).
///
/// **One function, two readers** — the picker draws this list and `flow.rs`
/// hit-tests against it, so the row a click lands on is the row the player
/// was looking at. A second ordering computed at the click would be the
/// classic version of this bug.
///
/// **Sorted by fit, descending; ties in roster order** (the owner's
/// 2026-09-09 decision). `sort_by_key` is stable, so a tie keeps the registry
/// order it arrived in and two readings of one frame are the same list. **The
/// verdict is shown and never sorted on**: who is best at the work and who
/// will agree to it are different questions, and the gap between them is the
/// player's to solve.
///
/// **Everyone appears**, including the people who are out — posting to
/// somebody who is out is legal and travels (`asks.rs`), so leaving them off
/// would hide a legal move.
pub fn candidates(
    flow: &Flow,
    lens: &Lens<'_>,
    grid: &crate::grid::Grid,
    tuning: &Tuning,
    now: u64,
    site: usize,
    slot: usize,
) -> Vec<Candidate> {
    let Some(task) = lens
        .site(site)
        .and_then(|board| board.quest(slot))
        .map(|quest| quest.task)
    else {
        return Vec::new();
    };
    let job = crate::sim::JobId { site, slot };
    let mut out: Vec<Candidate> = (0..lens.people().len().min(layout::PICKER_ROWS))
        .map(|who| Candidate {
            who,
            fit: lens.competence(who, task),
            reading: reading_for(flow, lens, tuning, now, who, job, task),
            route: lens.travel(grid, tuning, who, site),
        })
        .collect();
    out.sort_by_key(|candidate| std::cmp::Reverse(candidate.fit));
    out
}

/// **The candidate picker**: who this job could go to, what each of them
/// brings to it, what they would say about the wage on offer, and where they
/// are (UI.md §3c).
///
/// **The surface that makes the board self-sufficient.** Every drawer covers
/// the map and that is accepted; the board was the one surface whose *purpose*
/// required a map interaction — a posting had to be aimed by selecting a
/// sprite the board itself might be lying across (`FINDINGS.md` G-026). The
/// defect was the requirement, so this names people instead of moving the
/// panel out of their way.
///
/// **It writes the one selection and nothing else** (UI.md §3b). Choosing a
/// candidate is exactly selecting them, from a fourth door: it sets
/// `Flow::selected`, and the character panel, the map ring, the row verdicts
/// and the footer's `TO <name>` all follow from that field with no new state.
/// It does not post — the row does, as wave 1.2 established, and there is one
/// way to post.
///
/// It draws **instead of** the board rather than over it, because a row nobody
/// can read lying under a panel is what the floors refuse; so its header
/// carries the two things the covered board was saying that still matter — the
/// job by name, and the wage these answers were read at.
pub fn candidate_picker(
    flow: &Flow,
    lens: &Lens<'_>,
    grid: &crate::grid::Grid,
    tuning: &Tuning,
    now: u64,
    site: usize,
    slot: usize,
) -> Panel {
    let mut panel = Panel::default();
    let Some(quest) = lens.site(site).and_then(|board| board.quest(slot)).copied() else {
        return panel;
    };
    let wage = wage_offered(flow, lens, site, slot, quest.task);
    panel.text(TextRun::new(
        layout::picker_title(),
        clipped(&format!("who for {}?", quest.name), layout::PICKER_HEAD_W),
        theme::SMALL,
        theme::INK,
    ));
    // **The wage on its own line**, because it is what the answers below were
    // read at and it appears nowhere else while this panel is covering the
    // board's footer — so it is the one thing here a clip may not take.
    panel.text(TextRun::new(
        layout::picker_wage(),
        clipped(
            &format!("{wage}g offered - best fit first"),
            layout::PICKER_HEAD_W,
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
    // flag, one sentence (`asks::fit_means`). Two surfaces showing fit must
    // not describe it in two voices, and the way to make that structural is
    // for there to be one chip rather than two that agree.
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

    for (row, candidate) in candidates(flow, lens, grid, tuning, now, site, slot)
        .into_iter()
        .enumerate()
        .take(layout::PICKER_ROWS)
    {
        let at = layout::picker_row(row).min;
        let picked = flow.selected == Some(candidate.who);
        if let Some(person) = lens.person(candidate.who) {
            panel.icon(IconRun::new(
                at + layout::cand::PORTRAIT,
                person.icon,
                layout::sheet::PORTRAIT_SCALE,
            ));
        }
        panel.text(TextRun::new(
            at + layout::cand::NAME,
            clipped(lens.name(candidate.who), layout::cand::NAME_W),
            theme::SMALL,
            if picked { theme::GOLD } else { theme::INK },
        ));
        panel.text(TextRun::new(
            at + layout::cand::FIT,
            clipped(&format!("fit {}", candidate.fit), layout::cand::FIT_W),
            theme::SMALL,
            if candidate.fit > 0 {
                theme::GOLD
            } else {
                theme::FAINT
            },
        ));
        // **Where they are** — the whereabouts line, so somebody who is out
        // reads as out, with where and when the work they are on is done.
        panel.text(TextRun::new(
            at + layout::cand::WHERE,
            clipped(&lens.whereabouts(candidate.who), layout::cand::WHERE_W),
            theme::SMALL,
            theme::DIM,
        ));
        panel.text(TextRun::new(
            at + layout::cand::SAYS,
            clipped(
                &format!(
                    "{} - {}",
                    candidate.reading.verdict.name(),
                    candidate.reading.why
                ),
                layout::cand::SAYS_W,
            ),
            theme::SMALL,
            if candidate.reading.verdict.takes() {
                theme::REGARD
            } else {
                theme::EMBER
            },
        ));
        // **The walk from wherever they stand** — `sim::route_out` through the
        // lens, which is the route the dispatch lays and the party then walks.
        panel.text(TextRun::new(
            at + layout::cand::TRAVEL,
            clipped(
                &match &candidate.route {
                    Some(route) => format!("{} min away", route.cost),
                    None => "no way there".to_owned(),
                },
                layout::cand::TRAVEL_W,
            ),
            theme::SMALL,
            theme::GOLD,
        ));
    }

    // **The footer says what a tap does, and never that a row is the right
    // one.** The verdict is shown and not ranked on, so nothing here labels
    // anybody best or recommended: the list is ordered by fit and says so.
    let hint = if flow.fit_explained {
        crate::asks::fit_means()
    } else {
        format!(
            "sorted by fit - tap somebody to name them for {}, then tap its row to post it",
            quest.name
        )
    };
    panel.block(
        layout::picker_hint(),
        &crate::ui::wrap(
            &hint,
            crate::ui::columns(layout::PICKER_HINT_W, theme::SMALL),
        ),
        theme::SMALL,
        theme::FAINT,
    );
    panel
}
