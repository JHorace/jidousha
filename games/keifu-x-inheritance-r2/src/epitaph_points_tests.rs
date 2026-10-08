//! W9's composition points, each a test named as a sentence: where SPEC §20 rolls a
//! wording and composes, and where it recomposes with the same wording — the founding, a
//! death page before and after its heir, a ghost laid, a ghost's dream taken up at a
//! coming of age, and a crowning.
//!
//! INVARIANT: every expectation is a shipped literal copied by hand from SPEC.md and
//! `content/epitaph.json` — never computed by the code under test.

use jidousha::prelude::Rng;

use crate::content::Content;
use crate::epitaph::roll_wording;
use crate::heirs::choose;
use crate::hero::{DreamFate, Fate, HeroId};
use crate::house::House;
use crate::ids::Destiny;
use crate::resolve::resolve_rolled;
use crate::testkit::{aim, house, id, seat, stage};
use crate::text::WritingMemory;
use crate::turning::turn_the_year;
use crate::winter::WinterPlan;

/// Into the winter, and the year turned with no winter lines.
fn turn(content: &Content, house: &mut House, seed: u64) {
    if !house.calendar.is_winter() {
        house.calendar.begin_winter();
    }
    turn_the_year(
        content,
        house,
        Vec::new(),
        WinterPlan::default(),
        &mut Rng::from_seed(seed),
    );
}

/// The death page about `hero`.
fn page_of(house: &House, hero: HeroId) -> usize {
    let passage = house.passage.as_ref().expect("a turning");
    passage
        .pages
        .iter()
        .position(|p| p.about == Some(hero))
        .expect("a page about them")
}

fn epitaph(house: &House, hero: HeroId) -> String {
    house.heroes[hero].epitaph.clone().unwrap_or_default()
}

#[test]
fn the_founding_rolls_elsbeths_wording_then_auds_before_the_board_and_composes_both() {
    let content = crate::content::load().expect("the content loads");
    for seed in [1, 7, 0x5eed] {
        let founded = House::found(&content, seed, &mut Rng::from_seed(seed)).expect("founds");
        // The founding's first draws are the two wordings, in this order.
        let mut rng = Rng::from_seed(seed);
        let mut writing = WritingMemory::default();
        let first = roll_wording(&mut writing, &mut rng);
        let second = roll_wording(&mut writing, &mut rng);
        let (elsbeth, aud) = (id(&founded.heroes, "Elsbeth"), id(&founded.heroes, "Aud"));
        assert_eq!(founded.heroes[elsbeth].wording, Some(first));
        assert_eq!(founded.heroes[aud].wording, Some(second));
        assert_eq!(founded.writing.last_frame, Some(second.frame));
        assert!(epitaph(&founded, elsbeth).starts_with("Elsbeth Thorne "));
        assert!(epitaph(&founded, aud).starts_with("Aud Hale "));
        // The living have no epitaph until the Ending (W10).
        for hero in founded.heroes.iter().filter(|h| h.is_living()) {
            assert_eq!(
                (hero.wording, &hero.epitaph),
                (None, &None),
                "{}",
                hero.name
            );
        }
    }
}

#[test]
fn a_death_page_composes_before_its_heir_and_the_choice_recomposes_with_the_same_wording() {
    let (content, mut house) = house();
    let (garrick, maren) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Maren"));
    house.heroes[garrick].age = 93;
    turn(&content, &mut house, 5);
    let wording = house.heroes[garrick]
        .wording
        .expect("the page rolled a wording");
    let before = epitaph(&house, garrick);
    assert!(
        before.contains("wanted to lay the Barrow's dead to rest, and died before it was done."),
        "{before}"
    );
    let page = page_of(&house, garrick);
    choose(&content, &mut house, page, Some(maren));
    let after = epitaph(&house, garrick);
    assert_eq!(
        house.heroes[garrick].wording,
        Some(wording),
        "the same wording"
    );
    let passed = [
        "dream, to lay the Barrow's dead to rest, was left to Maren.",
        "wanted to lay the Barrow's dead to rest, and died before it was done. Maren carries it \
         now.",
    ];
    assert!(passed.iter().any(|p| after.contains(p)), "{after}");
}

#[test]
fn a_dream_left_to_no_one_is_told_so_and_its_ghost_laid_recomposes_the_epitaph() {
    let (content, mut house) = house();
    let (garrick, ysolde) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Ysolde"));
    house.heroes[garrick].age = 93;
    turn(&content, &mut house, 5);
    let page = page_of(&house, garrick);
    choose(&content, &mut house, page, None);
    let wording = house.heroes[garrick].wording;
    assert!(
        epitaph(&house, garrick).contains(
            "wanted to lay the Barrow's dead to rest. It was left to no one, and it has not left \
             the house."
        ),
        "{}",
        epitaph(&house, garrick)
    );
    // Summer comes; the ghost's quest is won in year 2.
    house.calendar.begin_summer();
    let quest = crate::ghost::ghost_quest(&content, &house.heroes, &house.ghosts[0], 0, 2);
    house.board[0] = crate::board::Posted {
        seats: vec![None; 2],
        quest,
        sworn: None,
    };
    seat(&mut house, 0, &[ysolde]);
    aim(&mut house, 0, [5, 5], 2);
    resolve_rolled(&content, &mut house, &mut Rng::from_seed(30), 0, [5, 5]);
    assert_eq!(house.heroes[garrick].dream_fate, DreamFate::LaidToRest);
    assert_eq!(house.heroes[garrick].wording, wording, "the same wording");
    assert!(
        epitaph(&house, garrick).contains(
            "wanted to lay the Barrow's dead to rest. It was left to no one, and walked until \
             year 2, when the house laid it to rest."
        ),
        "{}",
        epitaph(&house, garrick)
    );
}

#[test]
fn a_ghosts_dream_taken_up_at_a_coming_of_age_recomposes_the_dead_ones_epitaph() {
    let (content, mut house) = house();
    let (garrick, pip) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Pip"));
    house.heroes[garrick].age = 93;
    turn(&content, &mut house, 5);
    let page = page_of(&house, garrick);
    choose(&content, &mut house, page, None);
    let wording = house.heroes[garrick].wording;
    // A year on, Pip — Garrick's grandson, undreamt — comes of age and takes the ghost up.
    house.calendar.begin_summer();
    house.heroes[pip].dream = None;
    house.heroes[pip].age = 11;
    turn(&content, &mut house, 6);
    assert!(house.ghosts.is_empty(), "the ghost was taken up");
    assert_eq!(house.heroes[garrick].wording, wording, "the same wording");
    let after = epitaph(&house, garrick);
    let passed = [
        "dream, to lay the Barrow's dead to rest, was left to Pip.",
        "wanted to lay the Barrow's dead to rest, and died before it was done. Pip carries it now.",
    ];
    assert!(passed.iter().any(|p| after.contains(p)), "{after}");
}

#[test]
fn a_crowning_rolls_a_wording_and_composes_the_epitaph_of_the_departed() {
    let (content, mut house) = house();
    let garrick = id(&house.heroes, "Garrick");
    house.heroes[garrick].destiny.kind = Destiny::WearACrown;
    let frame_before = house.writing.last_frame;
    stage(&content, &mut house, 3, "The tourney");
    seat(&mut house, 3, &[garrick]);
    aim(&mut house, 3, [6, 6], 4);
    resolve_rolled(&content, &mut house, &mut Rng::from_seed(12), 3, [6, 6]);
    assert_eq!(house.heroes[garrick].fate, Fate::Departed);
    let wording = house.heroes[garrick]
        .wording
        .expect("a wording at the crowning");
    assert_ne!(
        Some(wording.frame),
        frame_before,
        "never the last frame rolled"
    );
    let said = epitaph(&house, garrick);
    for part in [
        "wanted to lay the Barrow's dead to rest, and left it undone for a crown.",
        "The Seer said he would wear a crown, and he did.",
        "The house has had a patron at Court since.",
    ] {
        assert!(said.contains(part), "{part:?} in {said:?}");
    }
    assert!(said.matches('.').count() <= 6, "{said}");
    assert!(said.contains("was called to a throne, and went"), "{said}");
}
