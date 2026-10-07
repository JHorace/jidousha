//! Easing a board nobody could answer (SPEC §5.2 "Easing",
//! `generation/quest-context/quest-context.jai:75-77,150-159`).
//!
//! When the kept board's best pair cannot both reach one half, the pair is re-read
//! and its weaker quest's demand lowered a point at a time — never below one demand
//! per seat, and at most `EASING_LIMIT` points. It draws nothing.

use crate::constants::EASING_LIMIT;
use crate::house::House;
use crate::quest::Quest;
use crate::reading::{Reading, answerable, pair_chances};

/// Why easing stopped (SPEC §5.2 "Easing").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Easing {
    /// The kept board was answerable, or had fewer than two quests: no easing.
    NotNeeded,
    /// Both quests of the remembered pair reached one half.
    BothAnswerable,
    /// The weaker quest's demand was already at or below its seats.
    Floor,
    /// `EASING_LIMIT` reductions were made.
    Limit,
}

/// Easing (SPEC §5.2): while the kept board is not answerable, re-read the
/// remembered pair (parties chosen afresh); stop when both reach one half; else
/// lower the weaker quest's demand by 1 — the first of the pair unless its chance is
/// not lower than the second's — unless it is already at or below its seats.
pub fn ease(house: &House, quests: &mut [Quest], reading: &Reading, eased: &mut [i32]) -> Easing {
    let Some((first, second)) = reading.pair else {
        return Easing::NotNeeded;
    };
    if answerable(reading.ways) || quests.len() < 2 {
        return Easing::NotNeeded;
    }
    for _ in 0..EASING_LIMIT {
        let (a, b) = pair_chances(
            &house.heroes,
            &quests[first],
            &quests[second],
            house.patrons,
        );
        if answerable(a) && answerable(b) {
            return Easing::BothAnswerable;
        }
        let weaker = if a < b { first } else { second };
        if quests[weaker].demand <= quests[weaker].seats {
            return Easing::Floor;
        }
        quests[weaker].demand -= 1;
        eased[weaker] += 1;
    }
    Easing::Limit
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Content;
    use crate::quest::post;
    use crate::testkit::{house, house_without_traits};
    use jidousha::prelude::Rng;

    /// Two quests at fixed demands, by title, posted in year 1 at no trouble.
    fn crafted(content: &Content, quests: [(&str, i32); 2]) -> Vec<Quest> {
        quests
            .iter()
            .map(|(title, demand)| {
                let template = content
                    .quest_templates
                    .iter()
                    .position(|t| t.title == *title)
                    .expect("a template by that title");
                let mut quest = post(content, template, 0, 1, &mut Rng::from_seed(1));
                quest.demand = *demand;
                quest
            })
            .collect()
    }

    fn unanswerable() -> Reading {
        Reading {
            ways: 0,
            pair: Some((0, 1)),
            call: None,
        }
    }

    #[test]
    fn easing_lowers_the_weaker_quest_until_both_reach_one_half() {
        let (content, house) = house_without_traits();
        // Garrick and Brannoc bring 12 to Grave goods: at 14 they have 28 in 100; at
        // 12, 58. The feast at 0 is answerable from the start.
        let mut quests = crafted(&content, [("Grave goods", 14), ("The long feast", 0)]);
        let mut eased = vec![0; 2];
        let stop = ease(&house, &mut quests, &unanswerable(), &mut eased);
        assert_eq!(
            (stop, eased.as_slice()),
            (Easing::BothAnswerable, &[2, 0][..])
        );
        assert_eq!(quests[0].demand, 12);
    }

    #[test]
    fn on_equal_chances_easing_lowers_the_second_of_the_pair_down_to_its_seats() {
        let (content, mut house) = house();
        // Everyone wounded: no likely party, so both chances stay 0 — a tie, always.
        for hero in &mut house.heroes {
            hero.wounded = true;
        }
        let mut quests = crafted(&content, [("Grave goods", 60), ("The rope bridge", 60)]);
        let mut eased = vec![0; 2];
        let stop = ease(&house, &mut quests, &unanswerable(), &mut eased);
        // A tie lowers the second: 58 points would reach its 2 seats, so the limit
        // comes first — 30 points, all off the bridge.
        assert_eq!((stop, eased.as_slice()), (Easing::Limit, &[0, 30][..]));
        let mut quests = crafted(&content, [("Grave goods", 60), ("The rope bridge", 20)]);
        let mut eased = vec![0; 2];
        let stop = ease(&house, &mut quests, &unanswerable(), &mut eased);
        assert_eq!(stop, Easing::Floor);
        assert_eq!(
            quests[1].demand, quests[1].seats,
            "eased exactly to one per seat"
        );
        assert_eq!(eased, [0, 20 - quests[1].seats]);
    }

    #[test]
    fn easing_stops_at_thirty_points() {
        let (content, house) = house_without_traits();
        let mut quests = crafted(&content, [("Grave goods", 50), ("The long feast", 0)]);
        let mut eased = vec![0; 2];
        let stop = ease(&house, &mut quests, &unanswerable(), &mut eased);
        assert_eq!((stop, eased.as_slice()), (Easing::Limit, &[30, 0][..]));
        assert_eq!(quests[0].demand, 20);
    }

    #[test]
    fn an_answerable_board_is_not_eased() {
        let (content, house) = house_without_traits();
        let mut quests = crafted(&content, [("Grave goods", 60), ("The long feast", 0)]);
        let mut eased = vec![0; 2];
        let answered = Reading {
            ways: 18,
            pair: Some((0, 1)),
            call: None,
        };
        assert_eq!(
            ease(&house, &mut quests, &answered, &mut eased),
            Easing::NotNeeded
        );
        assert_eq!(eased, [0, 0]);
        let short = Reading {
            ways: 17,
            pair: Some((0, 1)),
            call: None,
        };
        assert_ne!(
            ease(&house, &mut quests, &short, &mut eased),
            Easing::NotNeeded
        );
    }
}
