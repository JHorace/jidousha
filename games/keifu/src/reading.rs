//! Reading a planned board (SPEC §5.2, `generation/quest-context/quest-context.jai:93-132`,
//! `lineage/quest.jai:100-126`): the likely party for a quest, the best pair of
//! quests the house could answer at once, and whether a quest calls a dreamer who
//! could go.
//!
//! Every chance here is P(SUCCESS or TRIUMPH) counted from `forecast` over the
//! party's `party_power` — the numbers the card shows and W6's roll will use — so
//! the board is judged by the same arithmetic the player reads. Chances are kept
//! as whole pairs of 36 and compared against the CONSTANTS §4 thresholds as
//! fractions; equal pair counts are exact ties and keep the first (OQ-33).

use crate::calls::{Telling, dream_call};
use crate::constants::{ANSWERABLE_CHANCE, DREAM_CALL_CHANCE};
use crate::content::Content;
use crate::fear::refuses;
use crate::forecast::{DICE_OUTCOMES, forecast};
use crate::hero::{Hero, HeroId};
use crate::ids::Outcome;
use crate::power::party_power;
use crate::quest::Quest;

/// Every living adult, in creation order: the candidates a board is read against.
pub fn adults(heroes: &[Hero]) -> Vec<HeroId> {
    (0..heroes.len())
        .filter(|&id| heroes[id].is_living() && heroes[id].is_adult())
        .collect()
}

/// The likely party for `quest` (SPEC §5.2): start from `leader`, if any; then
/// repeatedly add the candidate, in candidate order, with the highest **solo**
/// power (patrons 0), skipping the wounded, those who refuse the quest and those
/// already chosen; ties keep the earliest; stop when the seats are full or no
/// candidate is left.
pub fn likely_party(
    heroes: &[Hero],
    candidates: &[HeroId],
    quest: &Quest,
    leader: Option<HeroId>,
) -> Vec<HeroId> {
    let mut party: Vec<HeroId> = leader.into_iter().collect();
    while party.len() < quest.seats as usize {
        let mut best: Option<(HeroId, i32)> = None;
        for &candidate in candidates {
            let hero = &heroes[candidate];
            if hero.wounded || refuses(hero, &quest.tags) || party.contains(&candidate) {
                continue;
            }
            let solo = party_power(heroes, &[candidate], quest.facts(), 0);
            if best.is_none_or(|(_, power)| solo > power) {
                best = Some((candidate, solo));
            }
        }
        let Some((chosen, _)) = best else { break };
        party.push(chosen);
    }
    party
}

/// P(SUCCESS or TRIUMPH) for `party` on `quest` with `patrons`, in pairs of 36.
pub fn success_ways(heroes: &[Hero], party: &[HeroId], quest: &Quest, patrons: i32) -> i32 {
    let power = party_power(heroes, party, quest.facts(), patrons);
    let odds = forecast(power, quest.demand, !party.is_empty());
    odds.ways(Outcome::Success) + odds.ways(Outcome::Triumph)
}

/// Whether `ways` of 36 reach `chance` (a CONSTANTS §4 threshold).
pub fn reaches(ways: i32, chance: f64) -> bool {
    f64::from(ways) / f64::from(DICE_OUTCOMES) >= chance
}

/// Whether `ways` of 36 make a quest answerable (CONSTANTS §4 `ANSWERABLE_CHANCE`).
pub fn answerable(ways: i32) -> bool {
    reaches(ways, ANSWERABLE_CHANCE)
}

/// The two likely parties of an ordered pair (i, j): i's from every adult, j's from
/// the adults not in i's.
pub fn pair_parties(heroes: &[Hero], first: &Quest, second: &Quest) -> (Vec<HeroId>, Vec<HeroId>) {
    let everyone = adults(heroes);
    let a = likely_party(heroes, &everyone, first, None);
    let rest: Vec<HeroId> = everyone.into_iter().filter(|h| !a.contains(h)).collect();
    let b = likely_party(heroes, &rest, second, None);
    (a, b)
}

/// The pair chances of an ordered pair, parties chosen afresh, in pairs of 36.
pub fn pair_chances(heroes: &[Hero], first: &Quest, second: &Quest, patrons: i32) -> (i32, i32) {
    let (a, b) = pair_parties(heroes, first, second);
    (
        success_ways(heroes, &a, first, patrons),
        success_ways(heroes, &b, second, patrons),
    )
}

/// What a board reading finds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reading {
    /// Answerability: the best ordered pair's weaker chance, in pairs of 36.
    pub ways: i32,
    /// The ordered pair that first attains it, in planning order (SPEC-GAPS KG-30:
    /// the first pair even when every pair reads 0).
    pub pair: Option<(usize, usize)>,
    /// The first quest, in planning order, that calls a dreamer who could go, and
    /// the dreamer.
    pub call: Option<(usize, HeroId)>,
}

impl Reading {
    /// Answerability as the chance it is.
    pub fn answerability(&self) -> f64 {
        f64::from(self.ways) / f64::from(DICE_OUTCOMES)
    }

    /// Welcome: answerable, and calling a dreamer who could go (SPEC §5.2).
    pub fn welcome(&self) -> bool {
        answerable(self.ways) && self.call.is_some()
    }
}

/// Read a planned board (SPEC §5.2 "Reading a board"): every ordered pair of
/// distinct quests, the max of their weaker chances, and the first pair at it.
pub fn read_board(content: &Content, heroes: &[Hero], quests: &[Quest], patrons: i32) -> Reading {
    let mut best: Option<(i32, (usize, usize))> = None;
    for i in 0..quests.len() {
        for j in 0..quests.len() {
            if i == j {
                continue;
            }
            let (a, b) = pair_chances(heroes, &quests[i], &quests[j], patrons);
            // SPEC-GAPS KG-30: planning order, i outer; the running best starts
            // empty, so the first pair is remembered even when every pair reads 0.
            let weaker = a.min(b);
            if best.is_none_or(|(ways, _)| weaker > ways) {
                best = Some((weaker, (i, j)));
            }
        }
    }
    let call = quests.iter().enumerate().find_map(|(at, quest)| {
        could_go_dreamer(content, heroes, quest, patrons).map(|dreamer| (at, dreamer))
    });
    Reading {
        ways: best.map_or(0, |(ways, _)| ways),
        pair: best.map(|(_, pair)| pair),
        call,
    }
}

/// The first living adult, in creation order, that `quest` calls as a dreamer who
/// could go (SPEC §9.6, the board reader's form): a call with an empty party that is
/// not "stay behind", not wounded, not refusing it, and whose likely party with them
/// as leader has success or better at `DREAM_CALL_CHANCE` or more (with the house's
/// patrons — SPEC-GAPS KG-32).
pub fn could_go_dreamer(
    content: &Content,
    heroes: &[Hero],
    quest: &Quest,
    patrons: i32,
) -> Option<HeroId> {
    adults(heroes)
        .into_iter()
        .find(|&dreamer| could_go(content, heroes, quest, patrons, dreamer))
}

/// Whether `quest` calls `dreamer` as one who could go (SPEC §9.6, the board
/// reader's form; see `could_go_dreamer`).
pub fn could_go(
    content: &Content,
    heroes: &[Hero],
    quest: &Quest,
    patrons: i32,
    dreamer: HeroId,
) -> bool {
    let hero = &heroes[dreamer];
    let goes = dream_call(content, heroes, dreamer, quest.facts(), &[])
        .is_some_and(|call| call.telling != Telling::StayBehind);
    if !goes || hero.wounded || refuses(hero, &quest.tags) {
        return false;
    }
    let party = likely_party(heroes, &adults(heroes), quest, Some(dreamer));
    reaches(
        success_ways(heroes, &party, quest, patrons),
        DREAM_CALL_CHANCE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{house, id};

    fn quest_titled(content: &Content, title: &str, demand: i32) -> Quest {
        let template = content
            .quest_templates
            .iter()
            .position(|t| t.title == title)
            .expect("a template by that title");
        let mut quest = crate::quest::post(
            content,
            template,
            0,
            1,
            &mut jidousha::prelude::Rng::from_seed(1),
        );
        quest.demand = demand;
        quest
    }

    #[test]
    fn a_likely_party_takes_the_strongest_solo_first_and_skips_the_wounded_and_refusers() {
        let (content, mut house) = house();
        let grave = quest_titled(&content, "Grave goods", 10);
        let heroes = &house.heroes;
        let everyone = adults(heroes);
        // Might, effective: Garrick 5 + Thornfall 1 = 6, Brannoc 6, Maren 4 ...;
        // Garrick comes first in creation order, so the tie keeps him.
        let (garrick, brannoc) = (id(heroes, "Garrick"), id(heroes, "Brannoc"));
        assert_eq!(
            likely_party(heroes, &everyone, &grave, None),
            [garrick, brannoc]
        );
        house.heroes[garrick].wounded = true;
        let party = likely_party(&house.heroes, &everyone, &grave, None);
        assert!(!party.contains(&garrick), "{party:?}");
        assert_eq!(party[0], brannoc);
        house.heroes[garrick].wounded = false;
        house.heroes[brannoc].fear.tag = crate::ids::Tag::Dark;
        house.heroes[brannoc].fear.broken = true;
        let party = likely_party(&house.heroes, &everyone, &grave, None);
        assert!(!party.contains(&brannoc), "{party:?}");
    }

    #[test]
    fn a_leader_is_seated_first_and_the_rest_fill_around_them() {
        let (content, house) = house();
        let grave = quest_titled(&content, "Grave goods", 10);
        let heroes = &house.heroes;
        let odo = id(heroes, "Odo");
        let party = likely_party(heroes, &adults(heroes), &grave, Some(odo));
        assert_eq!(party.len(), 2);
        assert_eq!(party[0], odo);
        assert_eq!(party[1], id(heroes, "Garrick"));
    }

    #[test]
    fn the_thresholds_fall_between_fifteen_and_twenty_one_and_ten_and_fifteen() {
        assert!(!answerable(15));
        assert!(answerable(21));
        assert!(answerable(18), "exactly one half is answerable");
        assert!(!reaches(10, DREAM_CALL_CHANCE));
        assert!(reaches(15, DREAM_CALL_CHANCE));
        assert!(!reaches(12, DREAM_CALL_CHANCE));
        assert!(reaches(13, DREAM_CALL_CHANCE), "13/36 is 0.361");
    }

    #[test]
    fn the_board_reading_keeps_the_first_ordered_pair_at_the_best_weaker_chance() {
        let (content, house) = house();
        let heroes = &house.heroes;
        // Two quests nobody can answer: every pair reads 0, and (0, 1) is kept.
        let hard = [
            quest_titled(&content, "Grave goods", 60),
            quest_titled(&content, "The bell under the tide", 60),
        ];
        let reading = read_board(&content, heroes, &hard, 0);
        assert_eq!((reading.ways, reading.pair), (0, Some((0, 1))));
        // Grave goods easy, the bell hard: the pair (1, 0) gives the bell first pick,
        // and still the bell's chance is the weaker; both orders read 0 for the bell.
        let mixed = [
            quest_titled(&content, "Grave goods", 2),
            quest_titled(&content, "The bell under the tide", 4),
        ];
        let reading = read_board(&content, heroes, &mixed, 0);
        let (a, b) = pair_chances(heroes, &mixed[0], &mixed[1], 0);
        let (c, d) = pair_chances(heroes, &mixed[1], &mixed[0], 0);
        let best = a.min(b).max(c.min(d));
        assert_eq!(reading.ways, best);
        let first = if a.min(b) == best { (0, 1) } else { (1, 0) };
        assert_eq!(reading.pair, Some(first));
    }

    #[test]
    fn grave_goods_calls_garrick_as_a_dreamer_who_could_go_at_a_fair_demand() {
        let (content, mut house) = house();
        let garrick = id(&house.heroes, "Garrick");
        // Garrick and Brannoc bring 12: at demand 13 they have 42 in 100 (>= 35).
        let grave = quest_titled(&content, "Grave goods", 13);
        assert_eq!(
            could_go_dreamer(&content, &house.heroes, &grave, 0),
            Some(garrick)
        );
        // At 14 they have 28 in 100: nobody who could go.
        let grave = quest_titled(&content, "Grave goods", 14);
        let caller = could_go_dreamer(&content, &house.heroes, &grave, 0);
        assert_ne!(caller, Some(garrick), "{caller:?}");
        // A patron is +1, and lifts him back over.
        assert_eq!(
            could_go_dreamer(&content, &house.heroes, &grave, 1),
            Some(garrick)
        );
        // Wounded, he cannot go.
        house.heroes[garrick].wounded = true;
        let grave = quest_titled(&content, "Grave goods", 2);
        assert_ne!(
            could_go_dreamer(&content, &house.heroes, &grave, 0),
            Some(garrick)
        );
    }

    #[test]
    fn the_houses_patrons_count_in_the_board_reading() {
        let (content, house) = house();
        // Grave goods at 13 against Garrick and Brannoc's 12: 42 in 100, 15 of 36.
        // With a patron they bring 13: 58 in 100, 21 of 36.
        let quests = [
            quest_titled(&content, "Grave goods", 13),
            quest_titled(&content, "The long feast", 0),
        ];
        assert_eq!(read_board(&content, &house.heroes, &quests, 0).ways, 15);
        assert_eq!(read_board(&content, &house.heroes, &quests, 1).ways, 21);
    }
}
