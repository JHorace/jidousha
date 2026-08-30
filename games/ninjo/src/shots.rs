//! What each photograph has to show, judged off the frame it was taken on.
//!
//! One function per picture, because a screenshot is only worth taking if
//! something asserts what is in it: the mid-travel map, the feed with the
//! world stopped, the config panel with a class set to pause, a character's
//! own panel, and the settlement before anything is dispatched. Every one of
//! them rebuilds the screen's content from the state of the tick the frame
//! was drawn on and looks for it on the frame (`frames.rs`), so a picture
//! that quietly stopped showing what it is for fails rather than ships.

use jidousha::prelude::*;

use crate::attention;
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::grid::Tile;
use crate::sim::Activity;
use crate::sweep::{Conducted, Shot};
use crate::{floors, frames, layout, lens, people, screens, verify};

/// Every photograph of the reference run, against what it is for.
pub fn judge(checks: &mut Checks, run: &Conducted, tuning: &Tuning) {
    if let Some(shot) = run.photo("map") {
        verify::judge_terrain(checks, &shot.frame, verify::HEADLESS_VIEWPORT);
        judge_tokens(checks, shot);
        frames::judge_chrome(checks, run, shot, "the mid-travel map");
        floors::judge_frame_floor(checks, run.font, &shot.frame, "the mid-travel map");
    } else {
        checks.require(
            false,
            "the mid-travel photograph was never taken",
            "the conductor's photo schedule names minute 40".to_owned(),
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
                "the clock reads paused={} at minute {} and the reason is {:?}; the point of                  the picture is the pause and its reason",
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
            "the conductor's photo schedule names minute 161, the first completion".to_owned(),
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

    // --- a character, looked at ---------------------------------------------
    if let Some(shot) = run.photo("person") {
        let lens = lens::Lens::on(&shot.sim);
        let who = shot.flow.selected_person;
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
            let panel = screens::content(&shot.flow, &lens, &shot.clock, tuning);
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
            "the conductor's photo schedule names minute 645, after the doorstep click".to_owned(),
        );
    }

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

/// **The tokens sit where the derivation says** (ADR-0041, DESIGN §3): the
/// mid-travel frame carries a token-sized quad at each party's derived
/// position — presentation read from discrete state, never written back.
fn judge_tokens(checks: &mut Checks, shot: &Shot) {
    let tuning = Tuning::SHIPPED;
    let reading = shot.clock.reading(&tuning);
    let mut travelling = 0;
    for (index, party) in shot.sim.parties.iter().enumerate() {
        if matches!(
            party.activity,
            Activity::Outbound { .. } | Activity::Homebound { .. }
        ) {
            travelling += 1;
        }
        let expected = screens::token_position(party, reading) - Vec2::splat(layout::TOKEN * 0.5)
            + Vec2::new(index as f32 * 4.0, index as f32 * -4.0);
        let drawn = shot.frame.quads().iter().any(|quad| {
            let bounds = quad.bounds();
            crate::checks::near(bounds.min.x, expected.x)
                && crate::checks::near(bounds.min.y, expected.y)
                && crate::checks::near(bounds.size().x, layout::TOKEN)
        });
        checks.require(
            drawn,
            "a party token is not drawn at its derived position",
            format!(
                "{}'s token should sit at ({:.1}, {:.1}) at clock reading {reading:.2} and no \
                 token-sized quad does",
                party.name, expected.x, expected.y
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
