//! What leaving a quest does, and what next summer holds, as far as the house's state
//! already decides it (keifu-fixes-r2; not in lineage).
//!
//! **The telegraph** (`telegraph`): a quest left unanswered raises its place's trouble
//! (`resolve::risen_trouble`), and the place's next quest is posted from one of its
//! templates other than the one the template memory holds there (SPEC §5.2), through
//! `quest::stakes` at that trouble in the next year. Its danger, seats and renown are
//! shown as ranges over those templates; its demand as the most it can be (the wobble
//! up), because easing (§5.2) can only lower it.
//!
//! **The foresight** (`foresight`): next summer's board is drawn after this summer
//! resolves, from the run's one generator, so it is not shown — drawing it early would
//! move every later draw. What is shown is what the state bounds now: each place's
//! trouble next summer as the heroes are seated (`resolve::eased_trouble` for an answered
//! quest, `risen_trouble` for a left one, unchanged off the board), the first ghost
//! (planning always posts it, §5.2 step 3), and the year's demand step.
//!
//! Both are pure readings: nothing here draws from the generator or writes the house.

use crate::constants::{DEMAND_WOBBLE, TROUBLE_LIMIT, YEARS_PER_DEMAND_STEP};
use crate::content::Content;
use crate::generation::templates_at;
use crate::house::House;
use crate::ids::Place;
use crate::quest::{Making, Source, stakes};
use crate::resolve::{eased_trouble, risen_trouble};
use crate::text::fmt;
use crate::words::W;

/// A closed range of whole numbers, written "3" or "3-4".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    /// The least.
    pub low: i32,
    /// The most.
    pub high: i32,
}

impl Span {
    fn point(value: i32) -> Self {
        Self {
            low: value,
            high: value,
        }
    }

    fn widen(self, value: i32) -> Self {
        Self {
            low: self.low.min(value),
            high: self.high.max(value),
        }
    }

    /// Whether `value` lies in it.
    pub fn holds(self, value: i32) -> bool {
        (self.low..=self.high).contains(&value)
    }

    /// "3", or "3-4".
    pub fn told(self) -> String {
        if self.low == self.high {
            self.low.to_string()
        } else {
            format!("{}-{}", self.low, self.high)
        }
    }
}

/// What leaving one posted quest does to its place's next quest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Telegraph {
    /// Where.
    pub place: Place,
    /// The place's trouble if the quest is left.
    pub trouble: i32,
    /// The next template quest's danger.
    pub danger: Span,
    /// Its seats.
    pub seats: Span,
    /// Its renown.
    pub renown: Span,
    /// The most it can need: the stakes formula's highest, wobble up.
    pub demand_most: i32,
}

/// The place's next template quest if board slot `slot` is left this summer, or `None`
/// when there is no next summer's board (the Door opens next) or the slot is a Door lock.
pub fn telegraph(content: &Content, house: &House, slot: usize) -> Option<Telegraph> {
    let quest = &house.board[slot].quest;
    if quest.is_door_lock() || house.calendar.years_until_door() <= 1 {
        return None;
    }
    let place = quest.place;
    let trouble = risen_trouble(house.places[place.index()].trouble);
    let year = house.calendar.current_year() + 1;
    let remembered = house.templates_last[place.index()];
    let mut out: Option<Telegraph> = None;
    for template in templates_at(content, place) {
        if Some(template) == remembered {
            continue;
        }
        let t = &content.quest_templates[template];
        for calm_seats in t.seats_low..=t.seats_high {
            let making = Making {
                source: Source::Template(template),
                title: String::new(),
                premise: String::new(),
                place,
                aptitude: t.aptitude,
                tags: Vec::new(),
            };
            let next = stakes(making, calm_seats, t.danger, trouble, year);
            let most = next.demand + DEMAND_WOBBLE;
            out = Some(match out {
                None => Telegraph {
                    place,
                    trouble,
                    danger: Span::point(next.danger),
                    seats: Span::point(next.seats),
                    renown: Span::point(next.renown),
                    demand_most: most,
                },
                Some(seen) => Telegraph {
                    danger: seen.danger.widen(next.danger),
                    seats: seen.seats.widen(next.seats),
                    renown: seen.renown.widen(next.renown),
                    demand_most: seen.demand_most.max(most),
                    ..seen
                },
            });
        }
    }
    out
}

/// The card's telegraph line: "Left: danger 3-4, room 1-2".
pub fn card_line(content: &Content, telegraph: &Telegraph) -> String {
    fmt(
        &content.words[W::QuestCardTelegraph],
        &[&telegraph.danger.told(), &telegraph.seats.told()],
    )
}

/// The quest sheet's telegraph line.
pub fn sheet_line(content: &Content, telegraph: &Telegraph) -> String {
    fmt(
        &content.words[W::QuestSheetTelegraph],
        &[
            &telegraph.trouble.to_string(),
            &telegraph.danger.told(),
            &telegraph.seats.told(),
            &telegraph.demand_most.to_string(),
            &telegraph.renown.told(),
        ],
    )
}

/// Each questing place's trouble next summer, as the heroes are seated now, in place order.
pub fn trouble_next(house: &House) -> Vec<(Place, Span)> {
    Place::ALL
        .iter()
        .copied()
        .filter(|&p| p != Place::SealedDoor)
        .map(|place| {
            let now = house.places[place.index()].trouble;
            let slot = (0..house.board.len()).find(|&s| house.board[s].quest.place == place);
            let span = match slot {
                None => Span::point(now),
                Some(slot) if house.party(slot).is_empty() => Span::point(risen_trouble(now)),
                Some(_) => Span::point(eased_trouble(now, true)).widen(eased_trouble(now, false)),
            };
            (place, span)
        })
        .collect()
}

/// What the foresight promises about the ghost: whose, where, and whether it is certain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GhostNext {
    /// Whose ghost.
    pub hero: crate::hero::HeroId,
    /// Where its quest is posted.
    pub place: Place,
    /// Its quest is answered this summer, so it may be laid and not come.
    pub unless_laid: bool,
}

/// The ghost next summer's planning posts first, unless laid this summer (SPEC §5.2 step 3).
pub fn ghost_next(house: &House) -> Option<GhostNext> {
    let ghost = house.ghosts.first()?;
    let answered = (0..house.board.len()).any(|s| {
        house.board[s].quest.source == Source::Ghost(ghost.hero) && !house.party(s).is_empty()
    });
    Some(GhostNext {
        hero: ghost.hero,
        place: ghost.place,
        unless_laid: answered,
    })
}

/// Whether next year's quests ask one more per seat than this year's.
pub fn demand_steps(house: &House) -> bool {
    let year = house.calendar.current_year();
    year / YEARS_PER_DEMAND_STEP > (year - 1) / YEARS_PER_DEMAND_STEP
}

/// The foresight, as the dock reads it: heading first, one logical line each.
pub fn foresight(content: &Content, house: &House) -> Vec<String> {
    let words = &content.words;
    let mut out = vec![words[W::OutlookHeading].to_owned()];
    if house.calendar.years_until_door() <= 1 {
        out.push(words[W::OutlookDoor].to_owned());
        return out;
    }
    out.push(words[W::OutlookIntro].to_owned());
    for (place, span) in trouble_next(house) {
        let title = &content.lore.places[place.index()].title;
        out.push(if span.high == 0 {
            fmt(&words[W::OutlookCalm], &[title])
        } else {
            fmt(&words[W::OutlookTroubled], &[title, &span.told()])
        });
    }
    if let Some(ghost) = ghost_next(house) {
        let word = if ghost.unless_laid {
            W::OutlookGhostUnless
        } else {
            W::OutlookGhost
        };
        out.push(fmt(
            &words[word],
            &[
                &house.heroes[ghost.hero].name,
                &content.lore.places[ghost.place.index()].name,
            ],
        ));
    }
    if demand_steps(house) {
        out.push(fmt(&words[W::OutlookStep], &["1"]));
    }
    out.push(fmt(&words[W::OutlookHow], &[&TROUBLE_LIMIT.to_string()]));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{house, id};

    #[test]
    fn a_span_of_one_value_is_told_as_that_value_and_a_wider_one_as_a_range() {
        assert_eq!(Span::point(3).told(), "3");
        assert_eq!(Span::point(3).widen(1).told(), "1-3");
        assert!(Span::point(3).widen(1).holds(2));
        assert!(!Span::point(3).widen(1).holds(4));
    }

    #[test]
    fn leaving_grave_goods_in_year_one_telegraphs_the_barrows_other_templates_at_trouble_one() {
        let (content, house) = house();
        let telegraph = telegraph(&content, &house, 0).expect("year 1 has a next summer");
        assert_eq!(telegraph.place, Place::Barrow);
        assert_eq!(telegraph.trouble, 1);
        // Every template but the remembered one, at trouble 1 in year 2.
        for template in templates_at(&content, Place::Barrow) {
            if Some(template) == house.templates_last[Place::Barrow.index()] {
                continue;
            }
            let t = &content.quest_templates[template];
            assert!(telegraph.danger.holds((t.danger + 1).min(4)));
            assert!(telegraph.renown.holds(t.danger + 1));
        }
        assert_eq!(
            card_line(&content, &telegraph),
            format!(
                "Left: danger {}, room {}",
                telegraph.danger.told(),
                telegraph.seats.told()
            )
        );
    }

    #[test]
    fn a_remembered_barrow_king_is_left_out_of_the_barrows_telegraph() {
        // The other three at trouble 1 in year 2: lamps (2-3 seats, danger 2) -> room
        // 1-2, danger 3, need 6 * 2 + 1; a name (1-2, danger 1) -> room 1, danger 2;
        // Grave goods (2, danger 2) -> room 1, danger 3, need 6 + 1.
        let (content, mut house) = house();
        let king = templates_at(&content, Place::Barrow)
            .into_iter()
            .find(|&t| content.quest_templates[t].title == "The Barrow-king wakes")
            .expect("the Barrow-king is a Barrow template");
        house.templates_last[Place::Barrow.index()] = Some(king);
        let telegraph = telegraph(&content, &house, 0).expect("year 1 has a next summer");
        assert_eq!(
            sheet_line(&content, &telegraph),
            "Left: trouble 1 here. Next time: danger 2-3, room 1-2, needs up to 13, pays 2-3."
        );
    }

    #[test]
    fn leaving_a_quest_in_year_six_telegraphs_year_sevens_need() {
        // Year 7 asks one more per seat: the Barrow-king at trouble 1 has room 2 and
        // needs 2 * (3 + 3 + 1 + 1) + 1 = 17.
        let (content, mut house) = house();
        while house.calendar.current_year() < 6 {
            house.calendar.begin_winter();
            house.calendar.begin_summer();
        }
        let grave = house.board[0].quest.template();
        house.templates_last[Place::Barrow.index()] = grave;
        let telegraph = telegraph(&content, &house, 0).expect("year 6 has a next summer");
        assert_eq!(telegraph.demand_most, 17);
        assert_eq!(card_line(&content, &telegraph), "Left: danger 2-4, room 1-2");
    }

    #[test]
    fn seating_a_party_turns_a_rising_place_into_one_that_may_ease() {
        let (_, mut house) = house();
        let barrow = Place::Barrow.index();
        house.places[barrow].trouble = 1;
        let span = |house: &House| {
            trouble_next(house)
                .into_iter()
                .find(|(p, _)| *p == Place::Barrow)
                .map(|(_, s)| s)
        };
        assert_eq!(span(&house), Some(Span::point(2)));
        let garrick = id(&house.heroes, "Garrick");
        house.board[0].seats[0] = Some(garrick);
        assert_eq!(span(&house), Some(Span { low: 0, high: 0 }));
        house.places[barrow].trouble = 2;
        assert_eq!(span(&house), Some(Span { low: 0, high: 1 }));
    }

    #[test]
    fn the_year_before_the_door_foresees_no_board_and_telegraphs_nothing() {
        let (content, mut house) = house();
        while house.calendar.years_until_door() > 1 {
            house.calendar.begin_winter();
            house.calendar.begin_summer();
        }
        assert_eq!(telegraph(&content, &house, 0), None);
        assert_eq!(
            foresight(&content, &house),
            [
                "NEXT SUMMER",
                "Next summer is the last. The Sealed Door opens, and no quests are posted."
            ]
        );
    }

    #[test]
    fn the_demand_steps_up_after_years_six_twelve_and_eighteen() {
        let (_, mut house) = house();
        let mut stepped = Vec::new();
        while house.calendar.years_until_door() > 1 {
            if demand_steps(&house) {
                stepped.push(house.calendar.current_year());
            }
            house.calendar.begin_winter();
            house.calendar.begin_summer();
        }
        assert_eq!(stepped, [6, 12, 18, 24]);
    }
}
