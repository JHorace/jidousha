//! What each photograph has to show, judged off the frame it was taken on.
//!
//! One function per picture, because a screenshot is only worth taking if
//! something asserts what is in it: the mid-travel map, the feed with the
//! world stopped, the config panel with a class set to pause, a character's
//! own panel, and the settlement before anything is dispatched. Every one of
//! them rebuilds the screen's content from the state of the tick the frame
//! was drawn on and looks for it on the frame (`frames.rs`), so a picture
//! that quietly stopped showing what it is for fails rather than ships.

use jidousha::prelude::Vec2;

use crate::attention;
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::flow::Drawer;
use crate::grid::Tile;
use crate::sim::Activity;
use crate::sweep::{Conducted, Shot};
use crate::{floors, frames, layout, lens, people, screens, verify};

/// Every photograph of the reference run, against what it is for.
pub fn judge(checks: &mut Checks, run: &Conducted, tuning: &Tuning) {
    let grid = crate::grid::grid();
    // **One person, one figure, on every frame this run kept** (UI.md §6).
    // Not two of them: the cast is drawn on the map whatever else is open, so
    // every photograph is a picture of the whole cast and every photograph is
    // owed the count. The double-drawn cast is what fixes that reading — it
    // was on all sixteen and asserted on none (`FINDINGS.md` G-023).
    for shot in &run.photos {
        floors::judge_figures(checks, run, shot, shot.name);
    }
    if let Some(shot) = run.photo("map") {
        verify::judge_terrain(checks, &shot.frame, verify::HEADLESS_VIEWPORT);
        judge_tokens(checks, shot);
        frames::judge_chrome(checks, run, shot, "the mid-travel map");
        floors::judge_frame_floor(checks, run.font, &shot.frame, "the mid-travel map");
    } else {
        checks.require(
            false,
            "the mid-travel photograph was never taken",
            "the conductor's photo schedule names minute 44".to_owned(),
        );
    }
    // --- the feed, photographed with the world stopped ---------------------
    if let Some(shot) = run.photo("feed") {
        checks.require(
            shot.flow.showing(Drawer::Feed),
            "the feed photograph was taken with the drawer shut",
            format!("the open drawer is {:?}", shot.flow.drawer),
        );
        let lens = lens::Lens::on(&shot.sim);
        checks.require(
            shot.clock.paused && lens.pause().is_some(),
            "the feed photograph does not show a world that stopped itself",
            format!(
                "the clock reads paused={} at minute {} and the reason is {:?}; the point of \
                 the picture is the pause and its reason",
                shot.clock.paused,
                shot.clock.minutes,
                lens.pause()
            ),
        );
        let reason = attention::reason_line(&lens).unwrap_or_default();
        checks.require(
            reason.contains("quest-complete") && reason.contains("completed"),
            "the pause reason beside the feed does not name what stopped the world",
            format!("the reason line reads {reason:?}"),
        );
        // The highlighted entry is the one the reason names, and it is on the
        // feed the photograph was taken of.
        let entries = attention::feed(&lens, shot.flow.show_ignored, attention::feed_cap(tuning));
        checks.require(
            lens.pause()
                .is_some_and(|pause| entries.iter().any(|entry| entry.index == pause.event)),
            "the entry that stopped the world is not on the feed that says why",
            format!(
                "the pause names event {:?} and the feed holds {:?}",
                lens.pause().map(|pause| pause.event),
                entries.iter().map(|entry| entry.index).collect::<Vec<_>>()
            ),
        );
        frames::judge_chrome(checks, run, shot, "the feed drawer");
        floors::judge_frame_floor(checks, run.font, &shot.frame, "the feed drawer");
    } else {
        checks.require(
            false,
            "the feed photograph was never taken",
            format!(
                "the conductor's photo schedule names minute {}, the first completion",
                crate::sweep::COMPLETIONS[0]
            ),
        );
    }

    // --- the config panel, photographed with a class set to pause ----------
    if let Some(shot) = run.photo("modes") {
        let lens = lens::Lens::on(&shot.sim);
        checks.require(
            shot.flow.showing(Drawer::Modes)
                && lens.attention().mode(attention::EventClass::QuestComplete)
                    == attention::Mode::PauseAndFocus,
            "the config photograph does not show the class the session was stopped by",
            format!(
                "the open drawer is {:?} and the config reads {}",
                shot.flow.drawer,
                lens.attention().stamp()
            ),
        );
        frames::judge_chrome(checks, run, shot, "the config panel");
        floors::judge_frame_floor(checks, run.font, &shot.frame, "the config panel");
    } else {
        checks.require(
            false,
            "the config photograph was never taken",
            "the conductor's photo schedule names tick 20, before the clock starts".to_owned(),
        );
    }

    // --- the world living on its own ----------------------------------------
    // **Nobody told them to go.** The picture the wave exists to be judged on:
    // a character on the road at a minute when the player has issued no order
    // that reaches it, with the feed carrying the reason they left.
    if let Some(shot) = run.photo("living") {
        let lens = lens::Lens::on(&shot.sim);
        let travelling: Vec<&str> = shot
            .sim
            .parties
            .iter()
            .enumerate()
            .filter(|(_, party)| {
                party.chosen
                    && matches!(
                        party.activity,
                        Activity::Outbound { .. } | Activity::Homebound { .. }
                    )
            })
            .map(|(index, _)| lens.name(index))
            .collect();
        checks.require(
            !travelling.is_empty(),
            "the photograph of a world living on its own has nobody living in it",
            format!(
                "at minute {} the parties on the road are {:?}, and none of them chose to be",
                shot.clock.minutes,
                shot.sim
                    .parties
                    .iter()
                    .map(|party| (party.name, party.chosen))
                    .collect::<Vec<_>>()
            ),
        );
        // And the reason is on the event that started it, not merely on the
        // party: the feed is where a player finds out why.
        let said = shot
            .sim
            .events
            .iter()
            .filter(|event| event.class == attention::EventClass::ActionStarted)
            .count();
        checks.require(
            said > 0,
            "somebody left on their own and the feed was never told why",
            format!(
                "the transcript holds {said} action-started events at minute {}",
                shot.clock.minutes
            ),
        );
        verify::judge_terrain(checks, &shot.frame, verify::HEADLESS_VIEWPORT);
        frames::judge_chrome(checks, run, shot, "the world living on its own");
        floors::judge_frame_floor(checks, run.font, &shot.frame, "the world living on its own");
    } else {
        checks.require(
            false,
            "the photograph of a world living on its own was never taken",
            "the conductor's photo schedule names minute 400".to_owned(),
        );
    }

    // --- the work list, on somebody with a mixed spread of fits ------------
    if let Some(shot) = run.photo("worklist") {
        let lens = lens::Lens::on(&shot.sim);
        let Some(who) = shot.flow.listing else {
            checks.require(
                false,
                "the work list photograph was taken with no list up",
                format!("the list reads {:?}", shot.flow.listing),
            );
            return;
        };
        checks.require(
            shot.flow.selected == Some(who),
            "the work list photograph is of one person's work under another's name",
            format!(
                "the list is {who:?}'s and the selection is {:?}",
                shot.flow.selected
            ),
        );
        // **The picture is of the decision, so it has to show a decision being
        // hard**: fits that separate people and a refusal among the answers.
        // A list of ten yesses at one fit is a picture of a list, not of this
        // one.
        let openings = crate::worklist::openings(&lens, &grid, tuning, shot.clock.minutes, who);
        let shown = &openings[..openings.len().min(layout::WORK_ROWS)];
        let spread = shown.iter().map(|o| o.fit).max().unwrap_or(0)
            - shown.iter().map(|o| o.fit).min().unwrap_or(0);
        let refused = shown
            .iter()
            .filter(|o| o.reading.as_ref().is_some_and(|r| !r.verdict.takes()))
            .count();
        checks.require(
            spread > 0 && refused >= 1 && shown.len() >= 2,
            "the work list photograph does not show the spread it is for",
            format!(
                "{} rows, a fit spread of {spread} and {refused} refusal(s); the picture is of \
                 a player weighing fit against what somebody would say",
                shown.len()
            ),
        );
        frames::judge_chrome(checks, run, shot, "the work list");
        floors::judge_frame_floor(checks, run.font, &shot.frame, "the work list");
    }

    // --- TUNE opened over an open ROSTER: one drawer, the owner's path ------
    if let Some(shot) = run.photo("tuneover") {
        checks.require(
            shot.flow.showing(Drawer::Tune),
            "the TUNE-over-ROSTER photograph was not taken with the tuning drawer up",
            format!("the open drawer is {:?}", shot.flow.drawer),
        );
        let panel = screens::content(
            &shot.flow,
            &lens::Lens::on(&shot.sim),
            &grid,
            &shot.clock,
            tuning,
            screens::reading(&shot.clock, tuning, screens::TICK),
            &shot.camera,
        );
        let drawing: Vec<&str> = Drawer::ALL
            .into_iter()
            .filter(|drawer| panel.runs.iter().any(|run| run.text == drawer.title()))
            .map(Drawer::label)
            .collect();
        checks.require(
            drawing == ["TUNE"],
            "the TUNE-over-ROSTER photograph carries more than the one drawer",
            format!("the frame draws {drawing:?}"),
        );
        frames::judge_chrome(checks, run, shot, "TUNE over an open roster");
        floors::judge_frame_floor(checks, run.font, &shot.frame, "TUNE over an open roster");
    }

    // --- the roster, with a chip's explanation open -------------------------
    if let Some(shot) = run.photo("roster") {
        checks.require(
            shot.flow.showing(Drawer::Roster) && shot.flow.explained.is_some(),
            "the roster photograph does not show the surface it is for",
            format!(
                "the open drawer is {:?} and the explained chip is {:?}",
                shot.flow.drawer, shot.flow.explained
            ),
        );
        // Every row's activity line is the lens's own, reason and all.
        let lens = lens::Lens::on(&shot.sim);
        let panel = screens::content(
            &shot.flow,
            &lens,
            &grid,
            &shot.clock,
            tuning,
            screens::reading(&shot.clock, tuning, screens::TICK),
            &shot.camera,
        );
        let says = |text: &str| panel.runs.iter().any(|run| run.text.contains(text));
        let missing: Vec<&str> = (0..lens.people().len())
            .filter(|who| !says(lens.name(*who)))
            .map(|who| lens.name(who))
            .collect();
        checks.require(
            missing.is_empty(),
            "the roster does not list everybody",
            format!("{missing:?} are not on the roster the photograph was taken of"),
        );
        // **The reason is on the roster**, not only in the feed: a row for
        // somebody who is out says what they are doing and why, in the
        // scorer's own words through the lens.
        let travelling: Vec<usize> = (0..lens.people().len())
            .filter(|who| !lens.at_home(*who) && !lens.reason(*who).is_empty())
            .collect();
        for who in &travelling {
            checks.require(
                says(lens.reason(*who)) || says(&clipped_head(lens.reason(*who))),
                "the roster does not say why somebody is out",
                format!(
                    "{} is out because {:?} and the roster's row does not carry it",
                    lens.name(*who),
                    lens.reason(*who)
                ),
            );
        }
        checks.require(
            !travelling.is_empty(),
            "the roster photograph was taken of a world where nobody is doing anything",
            "the picture is for the column that says what each of them is doing".to_owned(),
        );
        if let Some(id) = shot.flow.explained {
            checks.require(
                says(
                    &crate::traits::explain(id, shot.sim.modules)
                        .chars()
                        .take(24)
                        .collect::<String>(),
                ),
                "the roster's explanation is not the line the trait row derives",
                format!(
                    "{id:?} explains as {:?}",
                    crate::traits::explain(id, shot.sim.modules)
                ),
            );
        }
        frames::judge_chrome(checks, run, shot, "the roster");
        floors::judge_frame_floor(checks, run.font, &shot.frame, "the roster");
    } else {
        checks.require(
            false,
            "the roster photograph was never taken",
            "the conductor's photo schedule names minute 462".to_owned(),
        );
    }

    // --- a character, looked at ---------------------------------------------
    if let Some(shot) = run.photo("person") {
        let lens = lens::Lens::on(&shot.sim);
        let who = shot.flow.selected;
        checks.require(
            who.is_some_and(|who| lens.name(who) == "Steve"),
            "clicking a figure on the map did not select the person standing there",
            format!(
                "the panel is open on {:?} and the script clicked Steve's doorstep",
                who.map(|who| lens.name(who))
            ),
        );
        // The panel says what the lens says — the whole of the one-source rule
        // over a surface that reads a person.
        if let Some(who) = who {
            let panel = screens::content(
                &shot.flow,
                &lens,
                &grid,
                &shot.clock,
                tuning,
                screens::reading(&shot.clock, tuning, screens::TICK),
                &shot.camera,
            );
            let says = |text: &str| panel.runs.iter().any(|run| run.text.contains(text));
            checks.require(
                says(&format!("{}g in hand", lens.wallet(who)))
                    && says(&format!("desperation {}", lens.desperation(who))),
                "the character panel's numbers are not the lens's numbers",
                format!(
                    "the lens reads {}g and desperation {} for {}",
                    lens.wallet(who),
                    lens.desperation(who),
                    lens.name(who)
                ),
            );
        }
        frames::judge_chrome(checks, run, shot, "the character panel");
        floors::judge_frame_floor(checks, run.font, &shot.frame, "the character panel");
    } else {
        checks.require(
            false,
            "the character photograph was never taken",
            "the conductor's photo schedule names minute 590, after the doorstep click".to_owned(),
        );
    }

    // --- the ring on a token: the selection's other state ------------------
    if let Some(shot) = run.photo("roadring") {
        let lens = lens::Lens::on(&shot.sim);
        let now = screens::reading(&shot.clock, tuning, screens::TICK);
        let Some(who) = shot.flow.selected else {
            checks.require(
                false,
                "the road-ring photograph was taken with nobody selected",
                "the picture is the one selection, marked on somebody who is out".to_owned(),
            );
            return;
        };
        checks.require(
            !lens.at_home(who),
            "the road-ring photograph was taken of somebody standing at home",
            format!(
                "{} is at their own door at the photographed minute; the picture is the ring \
                 on a token",
                lens.name(who)
            ),
        );
        let ring = screens::selection_ring(&shot.flow, &lens, now);
        checks.require(
            ring == screens::where_drawn(&lens, who, now)
                && ring.is_some_and(|ring| ring != layout::home_rect(shot.sim.people[who].home)),
            "the ring in the road-ring photograph is not on the token it marks",
            format!(
                "the ring is at {ring:?}, {}'s figure is at {:?} and their empty door is at {:?}",
                lens.name(who),
                screens::where_drawn(&lens, who, now),
                layout::home_rect(shot.sim.people[who].home)
            ),
        );
        frames::judge_chrome(checks, run, shot, "the ring on a token");
        floors::judge_frame_floor(checks, run.font, &shot.frame, "the ring on a token");
    } else {
        checks.require(
            false,
            "the road-ring photograph was never taken",
            "the conductor's photo schedule names minute 52, with Bob out and picked".to_owned(),
        );
    }

    // --- the job board: the three pictures this wave owes ------------------
    judge_board(checks, run, tuning, &grid);
    judge_picker(checks, run, tuning, &grid);

    // --- the settlement: the cast, at home, named --------------------------
    if let Some(shot) = run.photo("settlement") {
        let lens = lens::Lens::on(&shot.sim);
        let away: Vec<&str> = (0..lens.people().len())
            .filter(|index| !lens.at_home(*index))
            .map(|index| lens.name(index))
            .collect();
        checks.require(
            away.is_empty() && lens.people().len() == people::roster().len(),
            "the settlement photograph does not show the whole cast at home",
            format!(
                "{away:?} are away at the photographed tick, and the frame shows {} of {} \
                 people; nothing has been dispatched yet",
                lens.people().len(),
                people::roster().len()
            ),
        );
        // Every figure and every name on the frame, at the position the panel
        // says - the same judge the chrome gets, over map-space content.
        frames::judge_chrome(checks, run, shot, "the settlement");
        floors::judge_frame_floor(checks, run.font, &shot.frame, "the settlement");
        verify::judge_terrain(checks, &shot.frame, verify::HEADLESS_VIEWPORT);
    } else {
        checks.require(
            false,
            "the settlement photograph was never taken",
            "the conductor's photo schedule names tick 10, before the first dispatch".to_owned(),
        );
    }
}

/// **The job board, photographed three ways** (UI.md §3c): read by somebody,
/// ordered from, and refusing a row that is already taken.
///
/// The wave's own pictures, and each one asserts the thing it is for — a
/// screenshot nobody checks is a screenshot that quietly stops showing what it
/// was taken for.
/// **The candidate picker, photographed twice** (UI.md §3c): open with
/// nobody selected, and the board after choosing from it.
///
/// The pair is the whole of the session's claim as a *picture* — that a job
/// can be aimed at a person without touching the map. So each is asserted for
/// the thing it is for: the first that the list carries a refusal, somebody
/// who is out, and more than one fit (a list where everybody agrees and
/// everybody is home proves nothing), and the second that the footer names
/// the person chosen and their panel is up on them.
fn judge_picker(checks: &mut Checks, run: &Conducted, tuning: &Tuning, grid: &crate::grid::Grid) {
    // --- the list, open with nobody selected --------------------------------
    let mut chosen_name: Option<String> = None;
    if let Some(shot) = run.photo("picker") {
        let lens = lens::Lens::on(&shot.sim);
        let panel = screens::content(
            &shot.flow,
            &lens,
            grid,
            &shot.clock,
            tuning,
            screens::reading(&shot.clock, tuning, screens::TICK),
            &shot.camera,
        );
        let (Some(site), Some(job)) = (shot.flow.board, shot.flow.picking) else {
            checks.require(
                false,
                "the picker photograph was taken with no candidate list open",
                format!(
                    "the flow's board reads {:?} and its picking {:?}",
                    shot.flow.board, shot.flow.picking
                ),
            );
            return;
        };
        checks.require(
            shot.flow.selected.is_none(),
            "the picker photograph was taken with somebody already selected",
            format!(
                "the selection reads {:?}; the picture is of the state the board could not be \
                 used in - a job aimed with nobody picked on the map",
                shot.flow.selected
            ),
        );
        let slot = job.slot;
        let open = lens
            .site(site)
            .and_then(|board| board.state(slot))
            .unwrap_or(crate::sim::JobState::Open);
        checks.require(
            open == crate::sim::JobState::Open && job.site == site,
            "the picker photograph is of a job nobody can be named for",
            format!(
                "row {slot} of site {site} reads {open:?} at minute {}, and the picker holds \
                 {job:?}",
                shot.clock.minutes
            ),
        );
        let listed = crate::board::candidates(
            &shot.flow,
            &lens,
            grid,
            tuning,
            shot.clock.minutes,
            site,
            slot,
        );
        let refusals = listed
            .iter()
            .filter(|candidate| !candidate.reading.verdict.takes())
            .count();
        let away = listed
            .iter()
            .filter(|candidate| !lens.at_home(candidate.who))
            .count();
        let fits = listed
            .iter()
            .map(|candidate| candidate.fit)
            .collect::<std::collections::BTreeSet<_>>();
        checks.require(
            refusals > 0 && away > 0 && fits.len() >= 2,
            "the picker photograph is of a list that shows none of what it is for",
            format!(
                "it carries {refusals} refusal(s), {away} character(s) who are out and \
                 {} distinct fit(s) over {} rows; the picture is for a mixed-fit job with at \
                 least one refusal and at least one person away",
                fits.len(),
                listed.len()
            ),
        );
        // Every row on the photograph is the sim's own three answers — the
        // same identity `verify::the_picker_names_a_person` asserts over every
        // open job, asked again of the frame a person looks at.
        for (row, candidate) in listed.iter().enumerate() {
            let at = crate::layout::picker_row(row).min;
            let cell = |offset: Vec2| {
                panel
                    .runs
                    .iter()
                    .find(|line| {
                        crate::checks::near(line.at.x, (at + offset).x)
                            && crate::checks::near(line.at.y, (at + offset).y)
                    })
                    .map(|line| line.text.clone())
            };
            let fit = crate::traits::competence_at(
                lens.site(site)
                    .and_then(|board| board.quest(slot))
                    .map(|quest| quest.task)
                    .unwrap_or(crate::traits::TaskType::ALL[0]),
                lens.traits(candidate.who),
            );
            checks.require(
                cell(crate::layout::cand::NAME).as_deref() == Some(lens.name(candidate.who))
                    && cell(crate::layout::cand::FIT).as_deref()
                        == Some(format!("fit {fit}").as_str())
                    && cell(crate::layout::cand::WHERE).as_deref()
                        == Some(lens.whereabouts(candidate.who).as_str()),
                "a photographed candidate row is not the person the list puts there",
                format!(
                    "row {row} reads name={:?} fit={:?} where={:?} and the list puts {} there, \
                     whose fit is {fit} and who is {:?}",
                    cell(crate::layout::cand::NAME),
                    cell(crate::layout::cand::FIT),
                    cell(crate::layout::cand::WHERE),
                    lens.name(candidate.who),
                    lens.whereabouts(candidate.who)
                ),
            );
        }
        // And the covered board says nothing at all: a row under the picker
        // would be text nobody can read lying across a control somebody can
        // click, which is the floor this surface would break by drawing both.
        // Asked at the board's own cell positions rather than by looking for a
        // job's name, because a name can also legitimately be in the picker's
        // own header.
        let under: Vec<String> = (0..crate::layout::BOARD_ROWS)
            .flat_map(|slot| {
                let at = crate::layout::board_row(slot).min;
                [crate::layout::job::NAME, crate::layout::job::SAYS]
                    .into_iter()
                    .map(move |offset| at + offset)
            })
            .filter_map(|at| {
                panel
                    .runs
                    .iter()
                    .find(|line| {
                        crate::checks::near(line.at.x, at.x) && crate::checks::near(line.at.y, at.y)
                    })
                    .map(|line| line.text.clone())
            })
            .collect();
        checks.require(
            under.is_empty(),
            "the board is still drawing its rows under the candidate list",
            format!(
                "{under:?} is drawn at the board's own row cells with the picker up; the \
                 picker draws instead of the board, and its header carries what the board \
                 was saying"
            ),
        );
        chosen_name = listed
            .get(crate::verify::PICKED_ROW)
            .map(|candidate| lens.name(candidate.who).to_owned());
        let best = listed.first().map(|candidate| candidate.fit);
        let taken = listed
            .get(crate::verify::PICKED_ROW)
            .map(|candidate| candidate.fit);
        checks.require(
            best.is_some() && taken.is_some() && best != taken,
            "the row the picture chooses from is the best fit on the list",
            format!(
                "the head of the list fits {best:?} and the row chosen fits {taken:?}; the \
                 picture is of a player weighing the verdict against the fit, and a top-row \
                 pick cannot show that"
            ),
        );
        frames::judge_chrome(checks, run, shot, "the candidate picker");
        floors::judge_frame_floor(checks, run.font, &shot.frame, "the candidate picker");
    } else {
        checks.require(
            false,
            "the candidate-picker photograph was never taken",
            "the conductor's photo schedule names minute 612".to_owned(),
        );
    }

    // --- the board after choosing -------------------------------------------
    if let Some(shot) = run.photo("chosen") {
        let lens = lens::Lens::on(&shot.sim);
        let panel = screens::content(
            &shot.flow,
            &lens,
            grid,
            &shot.clock,
            tuning,
            screens::reading(&shot.clock, tuning, screens::TICK),
            &shot.camera,
        );
        let says = |text: &str| panel.runs.iter().any(|row| row.text.contains(text));
        let Some(who) = shot.flow.selected else {
            checks.require(
                false,
                "the after-choosing photograph shows nobody chosen",
                format!("the selection reads {:?}", shot.flow.selected),
            );
            return;
        };
        checks.require(
            shot.flow.picking.is_none() && shot.flow.board.is_some(),
            "choosing did not put the candidate list away, or took the board with it",
            format!(
                "the picker reads {:?} and the board {:?}; the list closes on choosing and \
                 the board it belongs to stays up, because the row is what posts",
                shot.flow.picking, shot.flow.board
            ),
        );
        checks.require(
            chosen_name.is_none() || chosen_name.as_deref() == Some(lens.name(who)),
            "the person chosen is not the person the list's chosen row named",
            format!(
                "the list's row {} named {chosen_name:?} and the selection reads {:?}",
                crate::verify::PICKED_ROW,
                lens.name(who)
            ),
        );
        checks.require(
            says(&format!("TO {}", lens.name(who))),
            "the board's footer does not name the person the picker chose",
            format!(
                "the selection reads {:?} and the footer does not say TO them; the picker \
                 writes the one selection and the footer is one of the things that follows \
                 from it",
                lens.name(who)
            ),
        );
        let name_at = crate::layout::person_panel().min + crate::layout::sheet::NAME;
        checks.require(
            panel.runs.iter().any(|line| {
                crate::checks::near(line.at.x, name_at.x)
                    && crate::checks::near(line.at.y, name_at.y)
                    && line.text == lens.name(who)
            }),
            "the character panel is not up on the person the picker chose",
            format!(
                "the selection reads {:?} and the panel's name row does not say it; the panel \
                 is open exactly while somebody is selected, whichever door named them",
                lens.name(who)
            ),
        );
        // And no posting has been made: the picker names, the row posts.
        checks.require(
            lens.site(shot.flow.board.unwrap_or(0))
                .is_some_and(|board| {
                    board.state(crate::verify::PICKED_ROW_SLOT) == Some(crate::sim::JobState::Open)
                }),
            "choosing a candidate posted the job",
            format!(
                "row {} of the open board reads {:?} after a choice and no row tap",
                crate::verify::PICKED_ROW_SLOT,
                lens.site(shot.flow.board.unwrap_or(0))
                    .and_then(|board| board.state(crate::verify::PICKED_ROW_SLOT))
            ),
        );
        frames::judge_chrome(checks, run, shot, "the board after choosing");
        floors::judge_frame_floor(checks, run.font, &shot.frame, "the board after choosing");
    } else {
        checks.require(
            false,
            "the after-choosing photograph was never taken",
            "the conductor's photo schedule names minute 622".to_owned(),
        );
    }
}

fn judge_board(checks: &mut Checks, run: &Conducted, tuning: &Tuning, grid: &crate::grid::Grid) {
    // --- the board, read by a selected character ----------------------------
    if let Some(shot) = run.photo("board") {
        let lens = lens::Lens::on(&shot.sim);
        let panel = screens::content(
            &shot.flow,
            &lens,
            grid,
            &shot.clock,
            tuning,
            screens::reading(&shot.clock, tuning, screens::TICK),
            &shot.camera,
        );
        let says = |text: &str| panel.runs.iter().any(|row| row.text.contains(text));
        let Some(site) = shot.flow.board else {
            checks.require(
                false,
                "the board photograph was taken with no board open",
                format!("the flow's board reads {:?}", shot.flow.board),
            );
            return;
        };
        let Some(who) = shot.flow.selected else {
            checks.require(
                false,
                "the board photograph was taken with nobody selected, so it has no fit column",
                "the picture is for the fit a named character has for named work".to_owned(),
            );
            return;
        };
        // Every row's fit is the aptitude the sim reads, and the header's
        // travel is the journey the sim would walk.
        let jobs = lens
            .site(site)
            .map(|board| board.quests.clone())
            .unwrap_or_default();
        let kinds = jobs
            .iter()
            .map(|quest| quest.task.id())
            .collect::<std::collections::BTreeSet<_>>();
        checks.require(
            kinds.len() >= 2,
            "the board photograph was taken of a site whose jobs are all one kind of work",
            format!(
                "the board shows {:?}; the picture is for a fit column that distinguishes \
                 people, and a single-type board cannot",
                jobs.iter().map(|quest| quest.task.id()).collect::<Vec<_>>()
            ),
        );
        for quest in &jobs {
            let fit = crate::traits::competence_at(quest.task, lens.traits(who));
            checks.require(
                says(&format!("fit {fit}")),
                "a photographed job row does not show the fit the sim reads",
                format!(
                    "{:?} is {} work and {} answers {fit}",
                    quest.name,
                    quest.task.id(),
                    lens.name(who)
                ),
            );
        }
        checks.require(
            lens.travel(grid, tuning, who, site)
                .is_some_and(|route| says(&format!("{} min away", route.cost))),
            "the photographed board's travel line is not the journey the sim would walk",
            format!(
                "sim::route_out answers {:?} for {}",
                lens.travel(grid, tuning, who, site)
                    .map(|route| (route.tiles.len(), route.cost)),
                lens.name(who)
            ),
        );
        frames::judge_chrome(checks, run, shot, "the job board");
        floors::judge_frame_floor(checks, run.font, &shot.frame, "the job board");
    } else {
        checks.require(
            false,
            "the job board photograph was never taken",
            "the conductor's photo schedule names minute 500".to_owned(),
        );
    }

    // --- a board an order has just been given from --------------------------
    if let Some(shot) = run.photo("ordered") {
        let lens = lens::Lens::on(&shot.sim);
        let panel = screens::content(
            &shot.flow,
            &lens,
            grid,
            &shot.clock,
            tuning,
            screens::reading(&shot.clock, tuning, screens::TICK),
            &shot.camera,
        );
        let says = |text: &str| panel.runs.iter().any(|row| row.text.contains(text));
        let held: Vec<(usize, usize)> = shot
            .flow
            .board
            .and_then(|site| lens.site(site))
            .map(|board| {
                board
                    .states
                    .iter()
                    .enumerate()
                    .filter_map(|(slot, state)| state.holder().map(|by| (slot, by)))
                    .collect()
            })
            .unwrap_or_default();
        checks.require(
            held.len() == 1 && says("has it"),
            "the ordered photograph does not show a row somebody was just sent to",
            format!(
                "the board open in it is {:?} and its spent rows are {held:?}; the picture is \
                 of the row the player tapped, now reading as the person they sent",
                shot.flow.board
            ),
        );
        // And the person on that row is out, on the journey the row started.
        checks.require(
            held.first().is_some_and(|(_, by)| !lens.at_home(*by)),
            "the person the ordered row names is not on the road",
            format!(
                "the spent rows are {held:?} at minute {}",
                shot.clock.minutes
            ),
        );
        frames::judge_chrome(checks, run, shot, "an order given from a job row");
        floors::judge_frame_floor(
            checks,
            run.font,
            &shot.frame,
            "an order given from a job row",
        );
    } else {
        checks.require(
            false,
            "the ordered-from-a-row photograph was never taken",
            "the conductor's photo schedule names minute 64".to_owned(),
        );
    }

    // --- the bounce on a row somebody already has ---------------------------
    if let Some(shot) = run.photo("bounce") {
        let lens = lens::Lens::on(&shot.sim);
        let toast = shot.flow.toast.as_ref().map(|toast| toast.text.clone());
        let taken = shot
            .flow
            .board
            .and_then(|site| lens.site(site))
            .and_then(|board| board.quest(0))
            .map(|quest| quest.name)
            .unwrap_or_default();
        checks.require(
            toast.as_deref() == Some(crate::sim::Refusal::Taken.message("", "", taken).as_str()),
            "the bounce photograph does not show the refusal it is for",
            format!(
                "the toast reads {toast:?} and the row tapped was {taken:?}; a claimed row \
                 says why it will not take the order, in the established bounce style"
            ),
        );
        checks.require(
            shot.flow.selected.is_some(),
            "the bounce put the selection down",
            format!("the selection reads {:?}", shot.flow.selected),
        );
        frames::judge_chrome(checks, run, shot, "the bounce on a claimed row");
        floors::judge_frame_floor(checks, run.font, &shot.frame, "the bounce on a claimed row");
    } else {
        checks.require(
            false,
            "the claimed-row bounce photograph was never taken",
            "the conductor's photo schedule names minute 520".to_owned(),
        );
    }
}

/// **The arithmetic, photographed** (UI.md §3e) — both placements, and each
/// one asserted against the reckoning the band was built from.
///
/// The picture the session exists for is the refusal one: a row somebody will
/// not take, with every term of the sum, what produced each, the total, and
/// the job that beat it. A screenshot of a band whose numbers nobody checked
/// is a screenshot that quietly stops being the scorer's.
pub fn judge_breakdowns(checks: &mut Checks, run: &Conducted, tuning: &Tuning) {
    let grid = crate::grid::grid();
    let band = |shot: &crate::sweep::Shot| -> Vec<String> {
        screens::content(
            &shot.flow,
            &lens::Lens::on(&shot.sim),
            &grid,
            &shot.clock,
            tuning,
            screens::reading(&shot.clock, tuning, screens::TICK),
            &shot.camera,
        )
        .runs
        .iter()
        .filter(|run| run.at.y >= layout::breakdown_panel().min.y)
        .map(|run| run.text.clone())
        .collect()
    };
    // --- a refused job row, with its sum open -------------------------------
    if let Some(shot) = run.photo("breakdown") {
        let rows = band(shot);
        checks.require(
            matches!(shot.flow.breakdown, Some(crate::flow::Breakdown::Job(_)))
                && shot.flow.board.is_some()
                && shot.flow.selected.is_some(),
            "the breakdown photograph was taken with no job row's arithmetic open",
            format!(
                "the band reads {:?}, the board is {:?} and the selection {:?}",
                shot.flow.breakdown, shot.flow.board, shot.flow.selected
            ),
        );
        checks.require(
            rows.iter().any(|row| row.contains("would refuse")),
            "the breakdown photograph is not of a refusal",
            format!(
                "the band's heading reads {:?}; the picture is for the thing a player cannot \
                 account for, which is a no",
                rows.first()
            ),
        );
        checks.require(
            rows.iter().any(|row| row.starts_with("= ")),
            "the breakdown photograph shows terms with no total under them",
            format!("the band reads {rows:?}"),
        );
        checks.require(
            rows.iter().any(|row| row.contains("scores")),
            "a refusal's breakdown does not name the candidate that beat it",
            format!("the band reads {rows:?}"),
        );
        frames::judge_chrome(checks, run, shot, "a refused row's arithmetic");
        floors::judge_frame_floor(checks, run.font, &shot.frame, "a refused row's arithmetic");
    } else {
        checks.require(
            false,
            "the job row's breakdown was never photographed",
            "the conductor's photo schedule names minute 526".to_owned(),
        );
    }
    // --- and the same sum for a decision already made -----------------------
    if let Some(shot) = run.photo("feedwhy") {
        let rows = band(shot);
        let Some(crate::flow::Breakdown::Entry(index)) = shot.flow.breakdown else {
            checks.require(
                false,
                "the feed breakdown photograph was taken with no decision's arithmetic open",
                format!("the band reads {:?}", shot.flow.breakdown),
            );
            return;
        };
        checks.require(
            shot.flow.showing(Drawer::Feed),
            "the feed breakdown photograph was taken with the drawer shut",
            format!("the open drawer is {:?}", shot.flow.drawer),
        );
        let recorded = shot
            .sim
            .events
            .get(index)
            .and_then(|event| event.judged.clone());
        let Some(reckoning) = recorded else {
            checks.require(
                false,
                "the feed breakdown was opened on an entry that recorded no decision",
                format!(
                    "entry {index} of {} carries no reckoning",
                    shot.sim.events.len()
                ),
            );
            return;
        };
        // The band on the frame is the record: term for term, and the total.
        for term in &reckoning.terms {
            checks.require(
                rows.iter().any(|row| *row == term.line()),
                "a recorded term is missing from the band that explains its decision",
                format!("{:?} is not among {rows:?}", term.line()),
            );
        }
        checks.require(
            rows.iter()
                .any(|row| *row == format!("= {} in all", reckoning.total())),
            "the feed breakdown's total is not the total of the decision it explains",
            format!(
                "the record totals {} and the band reads {rows:?}",
                reckoning.total()
            ),
        );
        frames::judge_chrome(checks, run, shot, "a decision's arithmetic in the feed");
        floors::judge_frame_floor(
            checks,
            run.font,
            &shot.frame,
            "a decision's arithmetic in the feed",
        );
    } else {
        checks.require(
            false,
            "the feed's breakdown was never photographed",
            "the conductor's photo schedule names minute 542".to_owned(),
        );
    }
}

/// **The settlement one notch of the wheel out** (UI.md §4).
///
/// The picture the label rule exists to make possible, and the one the owner
/// judges the default camera against. What it asserts is the rule: the map's
/// words are gone, the pictures are not, and the selected character is still
/// named — because that name is chrome and no zoom reaches it.
pub fn judge_zoomed(checks: &mut Checks, run: &Conducted, tuning: &Tuning) {
    let Some(shot) = run.photo("zoomed") else {
        checks.require(
            false,
            "the zoomed-out settlement was never photographed",
            "the conductor's photo schedule names tick 20 of the zoomed session".to_owned(),
        );
        return;
    };
    let lens = lens::Lens::on(&shot.sim);
    let panel = screens::content(
        &shot.flow,
        &lens,
        &crate::grid::grid(),
        &shot.clock,
        tuning,
        screens::reading(&shot.clock, tuning, screens::TICK),
        &shot.camera,
    );
    checks.require(
        shot.camera.height > crate::camera::DEFAULT_H,
        "the zoomed photograph was taken at the default camera",
        format!(
            "it was taken at height {:.1} and the default is {:.0}; the wheel notch did not \
             reach the camera",
            shot.camera.height,
            crate::camera::DEFAULT_H
        ),
    );
    checks.require(
        panel.world_runs.is_empty() && !panel.world_icons.is_empty(),
        "the zoomed photograph does not show the label rule it was taken for",
        format!(
            "{} map words and {} map pictures at height {:.1}",
            panel.world_runs.len(),
            panel.world_icons.len(),
            shot.camera.height
        ),
    );
    let Some(who) = shot.flow.selected else {
        checks.require(
            false,
            "the zoomed photograph was taken with nobody selected",
            "the one word that survives the zoom is the selected character's name".to_owned(),
        );
        return;
    };
    checks.require(
        panel.runs.iter().any(|row| row.text == lens.name(who)),
        "the selected character is not named on the zoomed-out map",
        format!(
            "{} is selected and no chrome row carries their name",
            lens.name(who)
        ),
    );
    frames::judge_chrome(checks, run, shot, "the settlement one notch out");
    floors::judge_frame_floor(
        checks,
        run.font,
        &shot.frame,
        "the settlement one notch out",
    );
}

/// The first few words of a reason - what survives the roster's own clip.
fn clipped_head(reason: &str) -> String {
    reason.split(' ').take(3).collect::<Vec<_>>().join(" ")
}

/// **The figures sit where the derivation says** (ADR-0041, DESIGN §3): the
/// mid-travel frame carries a figure-sized quad at each person's derived
/// position — presentation read from discrete state, never written back.
///
/// The expectation is `screens::where_drawn`, the one answer to where
/// somebody is drawn, so a person on the road is looked for on their token
/// and a person at home on their doorstep, by the same call the draw made.
fn judge_tokens(checks: &mut Checks, shot: &Shot) {
    let tuning = Tuning::SHIPPED;
    let reading = screens::reading(&shot.clock, &tuning, screens::TICK);
    let lens = lens::Lens::on(&shot.sim);
    let mut travelling = 0;
    for (index, party) in shot.sim.parties.iter().enumerate() {
        if matches!(
            party.activity,
            Activity::Outbound { .. } | Activity::Homebound { .. }
        ) {
            travelling += 1;
        }
        let Some(expected) = screens::where_drawn(&lens, index, reading) else {
            checks.require(
                false,
                "a party is drawn nowhere at all",
                format!("{} has no figure at clock reading {reading:.2}", party.name),
            );
            continue;
        };
        let drawn = shot.frame.quads().iter().any(|quad| {
            let bounds = quad.bounds();
            crate::checks::near(bounds.min.x, expected.min.x)
                && crate::checks::near(bounds.min.y, expected.min.y)
                && crate::checks::near(bounds.size().x, layout::HOME)
        });
        checks.require(
            drawn,
            "a party token is not drawn at its derived position",
            format!(
                "{}'s figure should sit at ({:.1}, {:.1}) at clock reading {reading:.2} and no \
                 figure-sized quad does",
                party.name, expected.min.x, expected.min.y
            ),
        );
    }
    checks.require(
        travelling >= 2,
        "the mid-travel photograph does not show two parties on the road",
        format!(
            "{travelling} of {} parties are travelling at the photographed minute; \
             simultaneity is the point",
            shot.sim.parties.len()
        ),
    );
    let routes: Vec<Option<Tile>> = shot
        .sim
        .parties
        .iter()
        .map(|party| match &party.activity {
            Activity::Outbound { route, .. } => route.tiles.last().copied(),
            _ => None,
        })
        .collect();
    let distinct = routes
        .iter()
        .flatten()
        .collect::<std::collections::BTreeSet<_>>();
    checks.require(
        distinct.len() >= 2,
        "the two travelling parties are not on visibly different routes",
        format!("outbound goals: {routes:?}"),
    );
}
