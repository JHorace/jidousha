//! The quest model and its stakes (SPEC §5.2, the quest part; CONSTANTS §4).
//!
//! A quest is a template posted with stakes: seats, danger, renown and demand,
//! raised by its place's trouble and by the year. `post` is the formula and the
//! order of its two rolls (seats first, then the wobble). Which templates are
//! posted, and in what order, is board generation — W5's.

use jidousha::prelude::Rng;

use crate::chance::between;
use crate::constants::{
    DANGER_LIMIT, DEATH_PER_DANGER, DEMAND_PER_SEAT, DEMAND_WOBBLE, RENOWN_PER_EXPECTATION,
    TROUBLE_DEMAND, TROUBLE_LIMIT, TROUBLE_SEATS, TROUBLED_RENOWN, UNANSWERED_RENOWN,
    YEARS_PER_DEMAND_STEP,
};
use crate::content::{Content, QuestTemplate};
use crate::ids::{Aptitude, Place, Tag};
use crate::power::QuestFacts;

/// One posted quest (SPEC §5.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quest {
    /// The template it was made from, an index into `Content::quest_templates`.
    pub template: usize,
    /// Where.
    pub place: Place,
    /// What it needs.
    pub aptitude: Aptitude,
    /// What it carries: the template's tags.
    pub tags: Vec<Tag>,
    /// The seats it would have had in calm.
    pub calm_seats: i32,
    /// Its seats: calm seats less one per trouble, never below one.
    pub seats: i32,
    /// The template's danger.
    pub calm_danger: i32,
    /// Danger with trouble, at most 4.
    pub danger: i32,
    /// Renown on success: calm danger plus trouble, uncapped.
    pub renown: i32,
    /// What the party must bring.
    pub demand: i32,
    /// The place's trouble when it was posted.
    pub trouble: i32,
}

/// The stakes formula: `demand = seats * (3 + calm_danger + trouble + (year - 1) / 6)`.
pub fn base_demand(seats: i32, calm_danger: i32, trouble: i32, year: i32) -> i32 {
    seats
        * (DEMAND_PER_SEAT
            + calm_danger
            + trouble * TROUBLE_DEMAND
            + (year - 1) / YEARS_PER_DEMAND_STEP)
}

/// Post `template` at its place's `trouble` in `year`: roll the calm seats, then
/// the wobble (SPEC §5.2, "Generating a quest from a template").
pub fn post(content: &Content, template: usize, trouble: i32, year: i32, rng: &mut Rng) -> Quest {
    assert!(
        (0..=TROUBLE_LIMIT).contains(&trouble),
        "[keifu] a quest posted at trouble {trouble}\n  likely cause: trouble was raised \
         past TROUBLE_LIMIT\n  fix: CONSTANTS.md §4 caps it at {TROUBLE_LIMIT}"
    );
    let t: &QuestTemplate = &content.quest_templates[template];
    let calm_seats = between(rng, t.seats_low, t.seats_high);
    let seats = (calm_seats - trouble * TROUBLE_SEATS).max(1);
    let demand =
        base_demand(seats, t.danger, trouble, year) + between(rng, -DEMAND_WOBBLE, DEMAND_WOBBLE);
    Quest {
        template,
        place: t.place,
        aptitude: t.aptitude,
        tags: t.tags.clone(),
        calm_seats,
        seats,
        calm_danger: t.danger,
        danger: (t.danger + trouble).min(DANGER_LIMIT),
        renown: t.danger + trouble,
        demand,
        trouble,
    }
}

impl Quest {
    /// What a power sum and a dream call need to know about it.
    pub fn facts(&self) -> QuestFacts<'_> {
        QuestFacts {
            aptitude: self.aptitude,
            place: self.place,
            tags: &self.tags,
            door_lock: false,
        }
    }

    /// The unanswered cost at house renown `renown` (SPEC §7.2):
    /// `1 + (1 if posted with trouble) + renown / 20`.
    pub fn unanswered_cost(&self, renown: i32) -> i32 {
        UNANSWERED_RENOWN
            + if self.trouble > 0 { TROUBLED_RENOWN } else { 0 }
            + renown / RENOWN_PER_EXPECTATION
    }

    /// Each member's death chance in a disaster, as the sheet prints it: `min(danger
    /// * 0.15, 1)` through CONSTANTS §3's `percent`.
    pub fn death_percent(&self) -> i32 {
        let chance = (f64::from(self.danger) * DEATH_PER_DANGER).min(1.0);
        (chance * 100.0 + 0.5) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template(content: &Content, title: &str) -> usize {
        content
            .quest_templates
            .iter()
            .position(|t| t.title == title)
            .expect("a template by that title")
    }

    #[test]
    fn grave_goods_in_year_one_asks_nine_to_eleven_might_of_two() {
        let content = crate::content::load().expect("the content loads");
        let grave = template(&content, "Grave goods");
        let mut seen = [false; 3];
        for seed in 0..64 {
            let quest = post(&content, grave, 0, 1, &mut Rng::from_seed(seed));
            assert_eq!(
                (quest.seats, quest.danger, quest.renown, quest.aptitude),
                (2, 2, 2, Aptitude::Might)
            );
            assert!((9..=11).contains(&quest.demand), "demand {}", quest.demand);
            seen[(quest.demand - 9) as usize] = true;
            assert_eq!(quest.unanswered_cost(15), 1);
        }
        assert_eq!(seen, [true; 3], "the wobble reaches -1, 0 and +1");
    }

    #[test]
    fn trouble_takes_a_seat_raises_danger_and_renown_and_adds_demand_per_seat() {
        let content = crate::content::load().expect("the content loads");
        // The lamps in the Barrow: seats 2..3, danger 2.
        let lamps = template(&content, "The lamps in the Barrow");
        for seed in 0..32 {
            let quest = post(&content, lamps, 2, 13, &mut Rng::from_seed(seed));
            let want_seats = (quest.calm_seats - 2).max(1);
            assert_eq!(quest.seats, want_seats);
            assert_eq!((quest.danger, quest.renown), (4, 4));
            // seats * (3 + 2 + 2 + 12 / 6) = seats * 9, then the wobble.
            assert!((quest.demand - want_seats * 9).abs() <= 1);
            assert_eq!(quest.unanswered_cost(40), 1 + 1 + 2);
        }
        // Danger caps at 4; renown does not.
        let king = template(&content, "The Barrow-king wakes");
        let quest = post(&content, king, 2, 1, &mut Rng::from_seed(3));
        assert_eq!(quest.danger, 4.min(quest.calm_danger + 2));
        assert_eq!(quest.renown, quest.calm_danger + 2);
    }

    #[test]
    fn the_year_adds_one_demand_per_seat_every_six_years() {
        assert_eq!(base_demand(2, 2, 0, 1), 10);
        assert_eq!(base_demand(2, 2, 0, 6), 10);
        assert_eq!(base_demand(2, 2, 0, 7), 12);
        assert_eq!(base_demand(2, 2, 0, 13), 14);
        assert_eq!(base_demand(3, 1, 1, 25), 3 * (3 + 1 + 1 + 4));
    }

    #[test]
    fn the_death_chance_is_fifteen_in_a_hundred_per_danger_up_to_all() {
        let content = crate::content::load().expect("the content loads");
        let mut quest = post(&content, 0, 0, 1, &mut Rng::from_seed(1));
        for (danger, want) in [(1, 15), (2, 30), (3, 45), (4, 60), (7, 100)] {
            quest.danger = danger;
            assert_eq!(quest.death_percent(), want);
        }
    }
}
