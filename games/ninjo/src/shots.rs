//! What each photograph has to show, judged off the frame it was taken on.
//!
//! One function per picture, because a screenshot is only worth taking if
//! something asserts what is in it: the mid-travel map, the feed with the
//! world stopped, the config panel with a class set to pause, a character's
//! own panel, and the settlement before anything is dispatched. Every one of
//! them rebuilds the screen's content from the state of the tick the frame
//! was drawn on and looks for it on the frame (`frames.rs`), so a picture
//! that quietly stopped showing what it is for fails rather than ships.

use crate::attention;
use crate::checks::Checks;
use crate::constants::Tuning;
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
            shot.flow.feed_open,
            "the feed photograph was taken with the drawer shut",
            format!("feed_open is {}", shot.flow.feed_open),
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
            shot.flow.modes_open
                && lens.attention().mode(attention::EventClass::QuestComplete)
                    == attention::Mode::PauseAndFocus,
            "the config photograph does not show the class the session was stopped by",
            format!(
                "the drawer is open={} and the config reads {}",
                shot.flow.modes_open,
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

    // --- the roster, with a chip's explanation open -------------------------
    if let Some(shot) = run.photo("roster") {
        checks.require(
            shot.flow.roster_open && shot.flow.explained.is_some(),
            "the roster photograph does not show the surface it is for",
            format!(
                "the drawer is open={} and the explained chip is {:?}",
                shot.flow.roster_open, shot.flow.explained
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
            shot.flow.feed_open,
            "the feed breakdown photograph was taken with the drawer shut",
            format!("feed_open is {}", shot.flow.feed_open),
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
