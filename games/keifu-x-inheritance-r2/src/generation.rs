//! Board generation (SPEC §5.2, `generation/quest-context/quest-context.jai:54-217`):
//! plan up to sixteen boards, keep the best by score, stop at the first welcome one,
//! ease the kept board if it is not answerable, remember each place's template, and
//! post the quests in place order.
//!
//! Planning draws from the run's generator (place, template, seats, wobble, per drawn
//! quest; seats and wobble for a forced one); reading, choosing, easing and finishing
//! draw nothing (`reading.rs`). What generation decided is kept on the house as a
//! `BoardReport`, which the W5 checks read — nothing in play reads it.

use jidousha::prelude::Rng;

use crate::chance::{fresh_index, index};
use crate::constants::{
    ANSWERABLE_CHANCE, ANSWERABLE_SCORE, BOARD_ATTEMPTS, CALL_SCORE, QUEST_COUNT,
};
use crate::content::Content;
use crate::easing::{Easing, ease};
use crate::ghost::ghost_quest;
use crate::hero::HeroId;
use crate::house::House;
use crate::ids::Place;
use crate::quest::{Quest, Source, post};
use crate::reading::{Reading, read_board};

/// What generation decided this summer: read by the W5 checks, never by play.
#[derive(Clone, Debug, PartialEq)]
pub struct BoardReport {
    /// Boards planned (1..=16).
    pub attempts: usize,
    /// Which attempt was kept, from 1.
    pub kept: usize,
    /// The kept board's score.
    pub score: f64,
    /// The kept board's reading, in planning order, before easing.
    pub reading: Reading,
    /// Each planned board's score, in attempt order.
    pub scores: Vec<f64>,
    /// Why easing stopped.
    pub easing: Easing,
    /// Demand taken off each board slot by easing.
    pub eased: Vec<i32>,
    /// The remembered pair as board slots, after sorting.
    pub pair_slots: Option<(usize, usize)>,
    /// The ghost whose quest was posted, if any.
    pub ghost: Option<HeroId>,
    /// The per-place template memory planning read (before this summer's).
    pub memory_before: Vec<Option<usize>>,
}

/// The board score: `min(answerability / 0.5, 1) * 2 + (1 if calls a dreamer)`.
pub fn score(reading: &Reading) -> f64 {
    let answered = (reading.answerability() / ANSWERABLE_CHANCE).min(1.0) * ANSWERABLE_SCORE;
    answered
        + if reading.call.is_some() {
            CALL_SCORE
        } else {
            0.0
        }
}

/// A place's four templates, in file order.
pub fn templates_at(content: &Content, place: Place) -> Vec<usize> {
    (0..content.quest_templates.len())
        .filter(|&t| content.quest_templates[t].place == place)
        .collect()
}

/// Plan one board (SPEC §5.2 "Planning one board"), in planning order: the forced
/// opening quests (year 1), then the first ghost's quest if its place is free, then
/// quests at places drawn without replacement, each from a template other than the
/// one `memory` remembers there.
pub fn plan(
    content: &Content,
    house: &House,
    memory: &[Option<usize>],
    rng: &mut Rng,
) -> Vec<Quest> {
    let year = house.calendar.current_year();
    let trouble = |place: Place| house.places[place.index()].trouble;
    // 1. The six questing places; the Door is not one.
    let mut available: Vec<Place> = Place::ALL
        .iter()
        .copied()
        .filter(|&p| p != Place::SealedDoor)
        .collect();
    let mut quests = Vec::new();
    // 2. The forced opening quests, in year 1.
    if year == 1 {
        for &template in &content.opening_quests {
            let place = content.quest_templates[template].place;
            quests.push(post(content, template, trouble(place), year, rng));
            available.retain(|&p| p != place);
        }
    }
    // 3. The first ghost only, if its place is still available.
    if let Some(ghost) = house.ghosts.first()
        && quests.len() < QUEST_COUNT
        && available.contains(&ghost.place)
    {
        quests.push(ghost_quest(
            content,
            &house.heroes,
            ghost,
            trouble(ghost.place),
            year,
        ));
        available.retain(|&p| p != ghost.place);
    }
    // 4. Drawn places, each with a template not remembered there.
    while quests.len() < QUEST_COUNT {
        assert!(
            !available.is_empty(),
            "[keifu_x_inheritance_r2] a board ran out of places at {} quests\n  likely cause: more forced \
             quests than places\n  fix: SPEC §5.2 plans four quests over six places",
            quests.len()
        );
        let place = available.remove(index(rng, available.len()));
        let templates = templates_at(content, place);
        let remembered = memory[place.index()].and_then(|t| templates.iter().position(|&x| x == t));
        // Uniform over the place's templates without the remembered one: the
        // `fresh_index` primitive (SPEC §22.1) is exactly that distribution.
        let pick = match remembered {
            Some(prev) => fresh_index(rng, templates.len(), prev),
            None => index(rng, templates.len()),
        };
        quests.push(post(content, templates[pick], trouble(place), year, rng));
    }
    quests
}

/// Post the summer's board (SPEC §5.2): plan, read and choose; ease; remember the
/// templates; sort by place into board slots. Returns what was decided.
pub fn generate(
    content: &Content,
    house: &House,
    rng: &mut Rng,
) -> (Vec<Quest>, Vec<Option<usize>>, BoardReport) {
    assert!(
        !house.calendar.door_stands_open(),
        "[keifu_x_inheritance_r2] a board was generated in the last summer\n  likely cause: the Door's \
         summer reached board generation\n  fix: SPEC §5.1 posts the Door's locks then (W10)"
    );
    let memory = house.templates_last.clone();
    let mut scores = Vec::new();
    let mut kept: Option<(Vec<Quest>, Reading, f64, usize)> = None;
    for attempt in 1..=BOARD_ATTEMPTS {
        // Each attempt plans from the remembered memory: attempts are independent.
        let quests = plan(content, house, &memory, rng);
        let reading = read_board(content, &house.heroes, &quests, house.patrons);
        let this = score(&reading);
        scores.push(this);
        if kept.as_ref().is_none_or(|(_, _, best, _)| this > *best) {
            kept = Some((quests, reading, this, attempt));
        }
        if kept
            .as_ref()
            .is_some_and(|(_, reading, _, _)| reading.welcome())
        {
            break;
        }
    }
    let Some((mut quests, reading, best, kept_at)) = kept else {
        panic!(
            "[keifu_x_inheritance_r2] no board was planned\n  likely cause: BOARD_ATTEMPTS is 0\n  fix: \
             CONSTANTS.md §4 plans up to 16"
        );
    };
    let mut eased = vec![0; quests.len()];
    let easing = ease(house, &mut quests, &reading, &mut eased);
    // Finishing: the remembered memory, overwritten by each non-ghost quest kept.
    let mut memory_after = memory.clone();
    for quest in &quests {
        if let Some(template) = quest.template() {
            memory_after[quest.place.index()] = Some(template);
        }
    }
    let mut order: Vec<usize> = (0..quests.len()).collect();
    order.sort_by_key(|&at| quests[at].place.index());
    let slot_of = |at: usize| order.iter().position(|&o| o == at).unwrap_or(at);
    let report = BoardReport {
        attempts: scores.len(),
        kept: kept_at,
        score: best,
        reading,
        scores,
        easing,
        eased: order.iter().map(|&at| eased[at]).collect(),
        pair_slots: reading.pair.map(|(a, b)| (slot_of(a), slot_of(b))),
        ghost: quests.iter().find_map(|q| match q.source {
            Source::Ghost(hero) => Some(hero),
            Source::Template(_) | Source::Door(_) => None,
        }),
        memory_before: memory,
    };
    let sorted = order.iter().map(|&at| quests[at].clone()).collect();
    (sorted, memory_after, report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::house;

    #[test]
    fn year_one_plans_the_two_opening_quests_first_then_two_drawn_places() {
        let (content, house) = house();
        for seed in 0..32 {
            let quests = plan(
                &content,
                &house,
                &house.templates_last,
                &mut Rng::from_seed(seed),
            );
            let titles: Vec<&str> = quests.iter().map(|q| q.title.as_str()).collect();
            assert_eq!(titles[..2], ["Grave goods", "The bell under the tide"]);
            assert_eq!(quests.len(), 4);
            let places: Vec<Place> = quests.iter().map(|q| q.place).collect();
            for (i, p) in places.iter().enumerate() {
                assert!(!places[i + 1..].contains(p), "seed {seed}: {places:?}");
                assert_ne!(*p, Place::SealedDoor);
            }
        }
    }

    #[test]
    fn a_drawn_place_never_repeats_the_template_it_last_had() {
        let (content, mut house) = house();
        house.calendar.begin_winter();
        house.calendar.begin_summer();
        let mut seen = [[0usize; 4]; 6];
        for seed in 0..400 {
            let memory: Vec<Option<usize>> = (0..Place::ALL.len())
                .map(|p| {
                    Place::ALL
                        .get(p)
                        .filter(|&&pl| pl != Place::SealedDoor)
                        .map(|&pl| templates_at(&content, pl)[1])
                })
                .collect();
            for quest in plan(&content, &house, &memory, &mut Rng::from_seed(seed)) {
                let templates = templates_at(&content, quest.place);
                let at = templates.iter().position(|&t| Some(t) == quest.template());
                let at = at.expect("a drawn quest is one of its place's templates");
                assert_ne!(at, 1, "seed {seed}: the remembered template came back");
                seen[quest.place.index()][at] += 1;
            }
        }
        for (place, counts) in seen.iter().enumerate() {
            assert_eq!(counts[1], 0);
            assert!(
                counts.iter().enumerate().all(|(i, c)| i == 1 || *c > 0),
                "place {place}: {counts:?}"
            );
        }
    }

    #[test]
    fn the_first_ghost_takes_its_place_and_later_ghosts_wait() {
        let (content, mut house) = house();
        let (garrick, odo) = (
            crate::testkit::id(&house.heroes, "Garrick"),
            crate::testkit::id(&house.heroes, "Odo"),
        );
        let dream = house.heroes[garrick].dream.clone().expect("a dream");
        house.calendar.begin_winter();
        house.calendar.begin_summer();
        house.ghosts = vec![
            crate::ghost::Ghost {
                hero: garrick,
                dream: dream.clone(),
                place: Place::Emberfall,
            },
            crate::ghost::Ghost {
                hero: odo,
                dream,
                place: Place::Deepwood,
            },
        ];
        for seed in 0..32 {
            let quests = plan(
                &content,
                &house,
                &house.templates_last,
                &mut Rng::from_seed(seed),
            );
            assert_eq!(quests[0].source, Source::Ghost(garrick), "seed {seed}");
            assert_eq!(quests[0].place, Place::Emberfall);
            assert!(quests.iter().all(|q| q.source != Source::Ghost(odo)));
            assert_eq!(
                quests
                    .iter()
                    .filter(|q| q.place == Place::Emberfall)
                    .count(),
                1
            );
        }
    }

    #[test]
    fn a_ghost_whose_place_is_taken_by_an_opening_quest_waits() {
        let (content, mut house) = house();
        let garrick = crate::testkit::id(&house.heroes, "Garrick");
        let dream = house.heroes[garrick].dream.clone().expect("a dream");
        house.ghosts = vec![crate::ghost::Ghost {
            hero: garrick,
            dream,
            place: Place::Barrow,
        }];
        let quests = plan(
            &content,
            &house,
            &house.templates_last,
            &mut Rng::from_seed(3),
        );
        assert!(quests.iter().all(|q| q.template().is_some()));
        assert_eq!(quests[0].title, "Grave goods");
    }

    #[test]
    fn the_score_is_twice_the_answerability_over_one_half_capped_plus_one_for_a_call() {
        let reading = |ways, call| Reading {
            ways,
            pair: Some((0, 1)),
            call,
        };
        assert_eq!(score(&reading(0, None)), 0.0);
        assert_eq!(score(&reading(9, None)), 1.0);
        assert_eq!(score(&reading(18, None)), 2.0);
        assert_eq!(score(&reading(36, None)), 2.0);
        assert_eq!(score(&reading(9, Some((0, 0)))), 2.0);
        assert_eq!(score(&reading(21, Some((2, 3)))), 3.0);
    }

    #[test]
    fn generation_sorts_the_kept_board_by_place_and_remembers_its_templates() {
        let (content, mut house) = house();
        for seed in 0..12 {
            let (quests, memory, report) = generate(&content, &house, &mut Rng::from_seed(seed));
            let places: Vec<usize> = quests.iter().map(|q| q.place.index()).collect();
            let mut sorted = places.clone();
            sorted.sort_unstable();
            assert_eq!(places, sorted, "seed {seed}");
            for quest in &quests {
                assert_eq!(memory[quest.place.index()], quest.template());
            }
            assert!(report.attempts >= report.kept && report.kept >= 1);
            assert_eq!(report.scores.len(), report.attempts);
            assert_eq!(report.scores[report.kept - 1], report.score);
            assert!(
                report.scores[..report.kept - 1]
                    .iter()
                    .all(|s| *s < report.score)
            );
            assert!(
                report.scores[report.kept..]
                    .iter()
                    .all(|s| *s <= report.score)
            );
            if report.attempts < BOARD_ATTEMPTS {
                assert!(
                    report.reading.welcome(),
                    "seed {seed}: stopped early unwelcome"
                );
            }
            house.templates_last = memory;
        }
    }

    #[test]
    #[should_panic(expected = "a board was generated in the last summer")]
    fn the_last_summer_is_the_doors_and_generates_no_board() {
        let (content, mut house) = house();
        for _ in 0..25 {
            house.calendar.begin_winter();
            house.calendar.begin_summer();
        }
        let _ = generate(&content, &house, &mut Rng::from_seed(1));
    }
}
