//! What a place's next quest can be: the one function the telegraph on a quest card
//! (SPEC §7.2, "left alone, next time") and the foresight in the dock ("next summer")
//! both read, so neither can promise a quest the sim then does not deal.
//!
//! Next summer's board is drawn from the run's one generator when that summer begins,
//! so nothing here draws: it reads what already exists — the place's trouble, the
//! template remembered there, the year — and evaluates `quest::stakes`, the formula
//! `quest::post` and `ghost::ghost_quest` use, over every roll that could be made.
//!
//! INVARIANT: every quest `generation::plan` can post at `place` lies inside
//! `next_quest`'s ranges for that place's trouble, except a board eased past the demand
//! floor (`generation::generate` eases demand down, never below the seats).

use crate::constants::{DEMAND_WOBBLE, TROUBLE_LIMIT};
use crate::content::Content;
use crate::generation::templates_at;
use crate::ghost::{Ghost, ghost_quest};
use crate::hero::Hero;
use crate::house::House;
use crate::ids::Place;
use crate::quest::{Quest, Source, making_of, stakes};

/// A closed range of whole numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    /// The least.
    pub lo: i32,
    /// The most.
    pub hi: i32,
}

impl Span {
    /// The span of one value.
    pub fn of(value: i32) -> Self {
        Self {
            lo: value,
            hi: value,
        }
    }

    /// This span widened to take `value` in.
    pub fn take(self, value: i32) -> Self {
        Self {
            lo: self.lo.min(value),
            hi: self.hi.max(value),
        }
    }

    /// Whether `value` lies in it.
    pub fn holds(self, value: i32) -> bool {
        (self.lo..=self.hi).contains(&value)
    }

    /// "3-4", or "3" when it is one value.
    pub fn text(self) -> String {
        if self.lo == self.hi {
            self.lo.to_string()
        } else {
            format!("{}-{}", self.lo, self.hi)
        }
    }
}

/// What a place's next quest could be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Next {
    /// The templates it can be made from: the place's four, less the remembered one.
    /// Empty for a ghost's quest.
    pub pool: Vec<usize>,
    /// Its seats.
    pub seats: Span,
    /// Its danger.
    pub danger: Span,
    /// Its renown on success.
    pub renown: Span,
    /// What it asks, before the board is eased: easing lowers a demand, never below
    /// its seats, so the floor is the least of what it may be eased to.
    pub demand: Span,
}

/// The place's trouble once its quest is left unanswered (SPEC §7.2): one more, at most
/// `TROUBLE_LIMIT`. `resolve::unanswered` raises it by this.
pub fn trouble_if_unanswered(trouble: i32) -> i32 {
    (trouble + 1).min(TROUBLE_LIMIT)
}

/// The place's trouble once its quest is answered and ends in `win` or not (SPEC §7.1
/// step 6): gone on a win, one less on a loss.
pub fn trouble_if_answered(trouble: i32, win: bool) -> i32 {
    if win { 0 } else { (trouble - 1).max(0) }
}

/// The templates `generation::plan` can draw at `place` when `remembered` is the one it
/// remembers there (SPEC §5.2 step 4): the place's four, less that one.
pub fn pool(content: &Content, place: Place, remembered: Option<usize>) -> Vec<usize> {
    templates_at(content, place)
        .into_iter()
        .filter(|&t| Some(t) != remembered)
        .collect()
}

impl Next {
    /// What one quest could be, its demand between `least` and `most`.
    fn of(quest: &Quest, least: i32, most: i32) -> Self {
        Self {
            pool: Vec::new(),
            seats: Span::of(quest.seats),
            danger: Span::of(quest.danger),
            renown: Span::of(quest.renown),
            demand: Span {
                lo: least,
                hi: most,
            },
        }
    }

    /// This, widened to take `other` in.
    fn widen(self, other: &Next) -> Self {
        Self {
            pool: Vec::new(),
            seats: self.seats.take(other.seats.lo).take(other.seats.hi),
            danger: self.danger.take(other.danger.lo).take(other.danger.hi),
            renown: self.renown.take(other.renown.lo).take(other.renown.hi),
            demand: self.demand.take(other.demand.lo).take(other.demand.hi),
        }
    }
}

/// Fold what each quest could be into one `Next`, naming `what` if there were none.
fn fold(all: impl Iterator<Item = Next>, what: &str) -> Next {
    let mut folded: Option<Next> = None;
    for next in all {
        folded = Some(match folded {
            None => next,
            Some(so_far) => so_far.widen(&next),
        });
    }
    let Some(folded) = folded else {
        panic!(
            "[keifu] an outlook was read over no quests\n  likely cause: {what}\n  fix: \
             SPEC §5.2 gives every place four templates and a trouble span at least one value"
        );
    };
    folded
}

/// What a template quest at `place` could be at any trouble in `troubles` in `year`,
/// the pool being `pool(place, remembered)`.
pub fn next_quest(
    content: &Content,
    place: Place,
    remembered: Option<usize>,
    troubles: Span,
    year: i32,
) -> Next {
    let pool = pool(content, place, remembered);
    let mut all = Vec::new();
    for &template in &pool {
        let t = &content.quest_templates[template];
        for trouble in troubles.lo..=troubles.hi {
            for calm in t.seats_low..=t.seats_high {
                let quest = stakes(making_of(content, template), calm, t.danger, trouble, year);
                // Easing takes a demand down a point at a time and stops at its seats
                // (SPEC §5.2), and the wobbled demand is never under its seats: so the
                // floor is the seats.
                let least = quest.seats;
                all.push(Next::of(&quest, least, quest.demand + DEMAND_WOBBLE));
            }
        }
    }
    let mut next = fold(all.into_iter(), "the pool was empty");
    next.pool = pool;
    next
}

/// What the ghost's quest would be at any trouble in `troubles` in `year`: the ghost
/// quest itself, which has no wobble and no pool.
pub fn next_ghost_quest(
    content: &Content,
    heroes: &[Hero],
    ghost: &Ghost,
    troubles: Span,
    year: i32,
) -> Next {
    fold(
        (troubles.lo..=troubles.hi).map(|trouble| {
            let quest = ghost_quest(content, heroes, ghost, trouble, year);
            Next::of(&quest, quest.demand, quest.demand)
        }),
        "the trouble span was empty",
    )
}

/// What leaving a posted quest unanswered does: where its place's trouble goes, and what
/// the place's next quest could then be (SPEC §7.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Telegraph {
    /// The place's trouble once the summer is resolved with nobody going.
    pub trouble_after: i32,
    /// Whether it was already at the limit, so nothing rises.
    pub already_worst: bool,
    /// What the place's next quest could be at that trouble.
    pub next: Next,
}

/// The telegraph for `quest`, or nothing where there is no next quest to tell: the last
/// summer's Door, and the summer before it.
pub fn telegraph(content: &Content, house: &House, quest: &Quest) -> Option<Telegraph> {
    if quest.is_door_lock() || house.calendar.years_until_door() <= 1 {
        return None;
    }
    let trouble = house.places[quest.place.index()].trouble;
    let trouble_after = trouble_if_unanswered(trouble);
    let year = house.calendar.current_year() + 1;
    let at = Span::of(trouble_after);
    let next = match quest.source {
        Source::Ghost(dead) => {
            let ghost = house.ghosts.iter().find(|g| g.hero == dead)?;
            next_ghost_quest(content, &house.heroes, ghost, at, year)
        }
        // A quest's own template is the one its place remembers next summer.
        Source::Template(template) => next_quest(content, quest.place, Some(template), at, year),
        Source::Door(_) => return None,
    };
    Some(Telegraph {
        trouble_after,
        already_worst: trouble_after == trouble,
        next,
    })
}

/// What one place could post next summer, given the seating now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceOutlook {
    /// The place.
    pub place: Place,
    /// Its trouble when next summer comes: what this summer's resolution can leave it at.
    pub trouble: Span,
    /// The ghost whose quest is certain to be posted here, unless it is laid first.
    pub ghost: Option<crate::hero::HeroId>,
    /// What it could post.
    pub next: Next,
}

/// Where each questing place stands for next summer, in place order, or nothing where
/// there is no next summer's board: the Door's summer and the one before it.
///
/// A place whose quest has no one on it is left alone, and its trouble rises; one with
/// someone going ends at nothing on a win and one less on a loss; one with no quest this
/// summer keeps its trouble. Which of the six places post is drawn when summer comes and
/// is not known now — except the first ghost's, which is posted first.
pub fn foresight(content: &Content, house: &House) -> Option<Vec<PlaceOutlook>> {
    if house.calendar.years_until_door() <= 1 {
        return None;
    }
    let year = house.calendar.current_year() + 1;
    let first_ghost = house.ghosts.first();
    let mut out = Vec::new();
    for place in Place::ALL
        .iter()
        .copied()
        .filter(|&p| p != Place::SealedDoor)
    {
        let trouble = house.places[place.index()].trouble;
        let posted = house
            .board
            .iter()
            .enumerate()
            .find(|(_, p)| p.quest.place == place);
        let (after, remembered) = match posted {
            Some((slot, p)) if house.party(slot).is_empty() => (
                Span::of(trouble_if_unanswered(trouble)),
                p.quest.template().or(house.templates_last[place.index()]),
            ),
            Some((_, p)) => (
                Span::of(trouble_if_answered(trouble, true))
                    .take(trouble_if_answered(trouble, false)),
                p.quest.template().or(house.templates_last[place.index()]),
            ),
            None => (Span::of(trouble), house.templates_last[place.index()]),
        };
        let ghost = first_ghost.filter(|g| g.place == place);
        let next = match ghost {
            Some(g) => next_ghost_quest(content, &house.heroes, g, after, year),
            None => next_quest(content, place, remembered, after, year),
        };
        out.push(PlaceOutlook {
            place,
            trouble: after,
            ghost: ghost.map(|g| g.hero),
            next,
        });
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quest::post;
    use jidousha::prelude::Rng;

    #[test]
    fn trouble_rises_by_one_and_stops_at_the_limit() {
        assert_eq!([0, 1, 2].map(trouble_if_unanswered), [1, 2, TROUBLE_LIMIT]);
    }

    #[test]
    fn a_win_clears_trouble_and_a_loss_eases_it_by_one() {
        assert_eq!([0, 1, 2].map(|t| trouble_if_answered(t, true)), [0, 0, 0]);
        assert_eq!([0, 1, 2].map(|t| trouble_if_answered(t, false)), [0, 0, 1]);
    }

    #[test]
    fn the_pool_is_the_places_templates_less_the_remembered_one() {
        let (content, _) = crate::testkit::house();
        let all = templates_at(&content, Place::Barrow);
        assert_eq!(all.len(), 4);
        assert_eq!(pool(&content, Place::Barrow, None), all);
        let left = pool(&content, Place::Barrow, Some(all[1]));
        assert_eq!(left, [all[0], all[2], all[3]]);
        // A template remembered at another place removes nothing here.
        let other = templates_at(&content, Place::DrownedCoast)[0];
        assert_eq!(pool(&content, Place::Barrow, Some(other)), all);
    }

    #[test]
    fn every_quest_post_can_make_lies_inside_the_range_it_was_read_for() {
        let (content, _) = crate::testkit::house();
        for place in Place::ALL
            .iter()
            .copied()
            .filter(|&p| p != Place::SealedDoor)
        {
            for remembered in [None, Some(templates_at(&content, place)[2])] {
                for trouble in 0..=TROUBLE_LIMIT {
                    for year in [1, 7, 25] {
                        let next = next_quest(&content, place, remembered, Span::of(trouble), year);
                        for seed in 0..40 {
                            let mut rng = Rng::from_seed(seed);
                            for &template in &next.pool {
                                let q = post(&content, template, trouble, year, &mut rng);
                                assert!(next.seats.holds(q.seats), "seats {q:?} {next:?}");
                                assert!(next.danger.holds(q.danger), "danger {q:?} {next:?}");
                                assert!(next.renown.holds(q.renown), "renown {q:?} {next:?}");
                                assert!(next.demand.holds(q.demand), "demand {q:?} {next:?}");
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_lamps_at_trouble_one_in_year_one_are_one_to_two_seats_of_danger_three() {
        let (content, _) = crate::testkit::house();
        let lamps = templates_at(&content, Place::Barrow)
            .into_iter()
            .find(|&t| content.quest_templates[t].title == "The lamps in the Barrow")
            .expect("the lamps");
        let others = templates_at(&content, Place::Barrow)
            .into_iter()
            .filter(|&t| t != lamps)
            .collect::<Vec<_>>();
        // Remembering one of the others leaves the lamps and two more in the pool.
        let next = next_quest(&content, Place::Barrow, Some(others[0]), Span::of(1), 1);
        assert!(next.pool.contains(&lamps));
        assert_eq!(next.pool.len(), 3);
        // The lamps: seats 2..3 less one, danger 2 + 1.
        assert!(next.seats.holds(1) && next.seats.holds(2));
        assert!(next.danger.holds(3));
    }

    #[test]
    fn a_wider_trouble_span_is_the_union_of_its_troubles() {
        let (content, _) = crate::testkit::house();
        let (low, high) = (
            next_quest(&content, Place::HighPass, None, Span::of(0), 4),
            next_quest(&content, Place::HighPass, None, Span::of(2), 4),
        );
        let both = next_quest(&content, Place::HighPass, None, Span { lo: 0, hi: 2 }, 4);
        assert_eq!(both.danger.lo, low.danger.lo);
        assert_eq!(both.danger.hi, high.danger.hi);
        assert!(both.demand.hi >= high.demand.hi);
    }

    fn with_ghost(house: &mut House) {
        let garrick = crate::testkit::id(&house.heroes, "Garrick");
        let dream = house.heroes[garrick].dream.clone().expect("Garrick dreams");
        house.ghosts.push(Ghost {
            hero: garrick,
            dream,
            place: Place::Barrow,
        });
    }

    #[test]
    fn the_year_a_quest_comes_in_is_next_summers_not_this_ones() {
        let (content, mut house) = crate::testkit::house();
        // Year 6's summer: demand creeps one per seat in year 7.
        house.calendar.year_index = 5;
        let told = telegraph(&content, &house, &house.board[0].quest).expect("a telegraph");
        let in_year_seven = next_quest(
            &content,
            Place::Barrow,
            house.board[0].quest.template(),
            Span::of(1),
            7,
        );
        let in_year_six = next_quest(
            &content,
            Place::Barrow,
            house.board[0].quest.template(),
            Span::of(1),
            6,
        );
        assert_eq!(told.next, in_year_seven);
        assert!(in_year_seven.demand.hi > in_year_six.demand.hi);
    }

    #[test]
    fn nothing_is_told_of_a_summer_that_is_the_doors() {
        let (content, mut house) = crate::testkit::house();
        house.calendar.year_index = 24;
        assert_eq!(telegraph(&content, &house, &house.board[0].quest), None);
        assert_eq!(foresight(&content, &house), None);
        house.calendar.year_index = 23;
        assert!(foresight(&content, &house).is_some());
    }

    #[test]
    fn a_ghosts_quest_returns_as_the_one_quest_at_a_higher_trouble() {
        let (content, mut house) = crate::testkit::house();
        with_ghost(&mut house);
        let ghost = house.ghosts[0].clone();
        house.board[0].quest = crate::ghost::ghost_quest(&content, &house.heroes, &ghost, 0, 1);
        let told = telegraph(&content, &house, &house.board[0].quest).expect("a telegraph");
        // Trouble 1 in year 2: seats 1, danger 3, demand 1 x (3 + 2 + 1), no wobble.
        assert_eq!(told.trouble_after, 1);
        assert!(told.next.pool.is_empty());
        assert_eq!(
            (told.next.seats, told.next.danger, told.next.demand),
            (Span::of(1), Span::of(3), Span::of(6))
        );
    }

    #[test]
    fn the_first_ghosts_place_is_foretold_as_its_ghost_and_the_rest_by_pool() {
        let (content, mut house) = crate::testkit::house();
        with_ghost(&mut house);
        let places = foresight(&content, &house).expect("a foresight");
        assert_eq!(places.len(), 6);
        assert_eq!(places[0].place, Place::Barrow);
        assert_eq!(places[0].ghost, Some(house.ghosts[0].hero));
        assert!(places[0].next.pool.is_empty());
        assert!(
            places[1..]
                .iter()
                .all(|p| p.ghost.is_none() && p.next.pool.len() >= 3)
        );
    }

    #[test]
    fn the_foresight_follows_the_seating_left_alone_rises_answered_clears() {
        let (content, mut house) = crate::testkit::house();
        house.places[Place::Barrow.index()].trouble = 1;
        house.board[0].quest.trouble = 1;
        let troubles = |house: &House| {
            foresight(&content, house)
                .expect("a foresight")
                .iter()
                .map(|p| p.trouble)
                .collect::<Vec<_>>()
        };
        // Nobody on the Barrow's quest: it is left alone, one more.
        crate::testkit::seat(&mut house, 0, &[]);
        assert_eq!(troubles(&house)[0], Span::of(2));
        // Someone going: a win clears it, a loss eases it by one.
        let garrick = crate::testkit::id(&house.heroes, "Garrick");
        crate::testkit::seat(&mut house, 0, &[garrick]);
        assert_eq!(troubles(&house)[0], Span { lo: 0, hi: 0 });
        // A place with no quest on the board keeps its trouble.
        house.places[Place::Deepwood.index()].trouble = 2;
        assert_eq!(troubles(&house)[5], Span::of(2));
    }

    #[test]
    fn the_demand_floor_is_the_seats_where_easing_stops() {
        let (content, _) = crate::testkit::house();
        for trouble in 0..=TROUBLE_LIMIT {
            let next = next_quest(&content, Place::Barrow, None, Span::of(trouble), 1);
            assert_eq!(next.demand.lo, next.seats.lo, "trouble {trouble}");
        }
    }

    #[test]
    fn the_foresight_reads_the_year_after_this_one() {
        let (content, mut house) = crate::testkit::house();
        let first = |house: &House| {
            foresight(&content, house).expect("a foresight")[0]
                .next
                .demand
                .hi
        };
        // Year 5's summer looks at year 6, year 6's at year 7, where demand creeps up.
        house.calendar.year_index = 4;
        let year_six = first(&house);
        house.calendar.year_index = 5;
        let year_seven = first(&house);
        assert!(year_seven > year_six, "{year_six} then {year_seven}");
    }

    #[test]
    fn a_span_prints_one_value_or_a_range() {
        assert_eq!(Span::of(3).text(), "3");
        assert_eq!(Span { lo: 3, hi: 4 }.text(), "3-4");
    }
}
