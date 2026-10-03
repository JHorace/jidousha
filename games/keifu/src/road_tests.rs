//! The resolution battery's unit half, part three: facing the fear (SPEC §10.1), sharing
//! the road (§12.2), a youth's lessons and a ghost laid (§14.4). Shipped literals, as
//! part one.

use jidousha::prelude::Rng;

use crate::hero::{DeedKind, DreamFate, Fate, HeroId};
use crate::house::House;
use crate::ids::{BondKind, Destiny, Place, Pool};
use crate::resolve::resolve_rolled;
use crate::testkit::{aim, house, id, pooled, seat};

#[test]
fn companions_who_win_twice_become_friends_and_companions_in_a_disaster_become_rivals() {
    let (content, mut house) = house();
    let (ysolde, odo, garrick) = (
        id(&house.heroes, "Ysolde"),
        id(&house.heroes, "Odo"),
        id(&house.heroes, "Garrick"),
    );
    for round in 0..2 {
        seat(&mut house, 1, &[ysolde, odo]);
        aim(&mut house, 1, [3, 3], 0);
        let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(13), 1, [3, 3]);
        let friends = pooled(&content, Pool::Friendships, &["Ysolde", "Odo"]);
        assert_eq!(
            page.lines.iter().any(|l| friends.contains(l)),
            round == 1,
            "{:?}",
            page.lines
        );
    }
    assert_eq!(
        house.heroes[ysolde].bond_to(odo).map(|b| b.kind),
        Some(BondKind::Friend)
    );
    assert!(
        house.heroes[odo]
            .deeds
            .iter()
            .any(|d| d.kind == DeedKind::Befriended && d.other == Some(ysolde))
    );
    // Garrick and Ysolde, strangers, through a disaster: companions, then rivals.
    house.heroes[garrick].destiny.kind = Destiny::DieInYourBed;
    house.heroes[ysolde].destiny.kind = Destiny::DieInYourBed;
    seat(&mut house, 0, &[garrick, ysolde]);
    aim(&mut house, 0, [1, 1], -9);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(14), 0, [1, 1]);
    assert!(page.lines.contains(
        &"Garrick and Ysolde each say the other broke first at the Barrow. They are rivals now, and weaker together.".to_owned()
    ), "{:?}", page.lines);
    assert_eq!(
        house.heroes[garrick].bond_to(ysolde).map(|b| b.kind),
        Some(BondKind::Rival)
    );
}

#[test]
fn rivals_who_triumph_together_come_home_friends() {
    let (content, mut house) = house();
    let (brannoc, ysolde) = (id(&house.heroes, "Brannoc"), id(&house.heroes, "Ysolde"));
    seat(&mut house, 0, &[brannoc, ysolde]);
    aim(&mut house, 0, [6, 6], 5);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(15), 0, [6, 6]);
    assert!(page.lines.contains(
        &"Brannoc and Ysolde went out rivals. Nobody can stay a rival through a day like that. They came home friends.".to_owned()
    ), "{:?}", page.lines);
    assert_eq!(
        house.heroes[ysolde].bond_to(brannoc).map(|b| b.kind),
        Some(BondKind::Friend)
    );
}

#[test]
fn a_parent_and_child_who_fail_together_each_take_dread_parent_first() {
    let (content, mut house) = house();
    let (garrick, maren) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Maren"));
    seat(&mut house, 0, &[maren, garrick]);
    aim(&mut house, 0, [3, 3], -2);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(16), 0, [3, 3]);
    let parent = page
        .lines
        .iter()
        .position(|l| l == "Garrick saw his daughter in the worst of it, and has not stopped seeing it. Dread 3 of 5.")
        .expect("the parent's line");
    assert_eq!(
        page.lines[parent + 1],
        "Maren saw her father go down. Dread 2 of 5."
    );
    assert_eq!(
        (
            house.heroes[garrick].fear.dread,
            house.heroes[maren].fear.dread
        ),
        (3, 2)
    );
}

#[test]
fn a_steadied_win_adds_courage_three_conquers_and_an_unsteadied_loss_adds_two_dread() {
    let (content, mut house) = house();
    let (garrick, maren) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Maren"));
    house.heroes[maren].fear.courage = 2;
    seat(&mut house, 1, &[maren, garrick]);
    aim(&mut house, 1, [3, 3], 0);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(17), 1, [3, 3]);
    let courage = pooled(&content, Pool::Courages, &["Maren", "deep water"]);
    assert!(
        page.lines.iter().any(|l| courage
            .iter()
            .any(|c| *l == format!("{c}, with Garrick beside her. Courage 3 of 3."))),
        "{:?}",
        page.lines
    );
    assert!(
        page.lines
            .iter()
            .any(|l| l.starts_with("Maren has conquered her fear of deep water."))
    );
    assert!(house.heroes[maren].fear.conquered);
    assert_eq!(
        house.heroes[garrick].fear.courage, 1,
        "Garrick, steadied by his daughter"
    );
    assert_eq!(
        house.heroes[maren].fears_faced, 1,
        "household.json: Maren had faced it 0 times; the bell is her first"
    );
    // Garrick alone on the bell, lost: dread +2 and the DREADS line, at 5 he breaks.
    house.heroes[garrick].fear.dread = 3;
    seat(&mut house, 1, &[garrick]);
    aim(&mut house, 1, [3, 3], -2);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(18), 1, [3, 3]);
    let dreads = pooled(&content, Pool::Dreads, &["Garrick", "deep water"]);
    let at = page
        .lines
        .iter()
        .position(|l| dreads.iter().any(|d| *l == format!("{d} Dread 5 of 5.")))
        .expect("the dread line");
    assert!(page.lines[at + 1].starts_with("Something in Garrick gave way."));
    assert!(house.heroes[garrick].fear.broken);
}

#[test]
fn winning_a_ghosts_quest_lays_it_and_swap_removes_it_from_the_list() {
    let (content, mut house) = house();
    let (garrick, maren, odo, ysolde) = (
        id(&house.heroes, "Garrick"),
        id(&house.heroes, "Maren"),
        id(&house.heroes, "Odo"),
        id(&house.heroes, "Ysolde"),
    );
    let ghost = |hero: HeroId, house: &House| crate::ghost::Ghost {
        hero,
        dream: house.heroes[garrick].dream.clone().expect("a dream"),
        place: Place::Barrow,
    };
    for dead in [garrick, maren, odo] {
        house.heroes[dead].fate = Fate::Dead;
        house.unseat(dead);
        crate::testkit::paged(&mut house, dead);
    }
    house.ghosts = vec![
        ghost(garrick, &house),
        ghost(maren, &house),
        ghost(odo, &house),
    ];
    let quest = crate::ghost::ghost_quest(&content, &house.heroes, &house.ghosts[0], 0, 1);
    house.board[0] = crate::board::Posted {
        seats: vec![None; 2],
        quest,
    };
    seat(&mut house, 0, &[ysolde]);
    aim(&mut house, 0, [4, 4], 1);
    let tales = house.tales.len();
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(20), 0, [4, 4]);
    assert!(
        page.story
            .starts_with("Ysolde said aloud what Garrick had meant to do"),
        "{}",
        page.story
    );
    assert_eq!(
        page.lines.last().map(String::as_str),
        Some(
            "Garrick Thorne is at rest. What he wanted is let go, not done. It leaves a tale: The laying of Garrick's ghost. +1 renown every year."
        )
    );
    assert_eq!(house.tales.len(), tales + 1);
    assert_eq!(house.heroes[garrick].dream_fate, DreamFate::LaidToRest);
    assert_eq!(house.heroes[garrick].laid_year, Some(1));
    let order: Vec<HeroId> = house.ghosts.iter().map(|g| g.hero).collect();
    assert_eq!(
        order,
        [odo, maren],
        "the last ghost takes the laid one's place"
    );
    assert!(
        house.heroes[ysolde]
            .deeds
            .iter()
            .any(|d| d.kind == DeedKind::LaidGhost)
    );
}

#[test]
fn a_conquered_fear_is_not_faced() {
    let (content, mut house) = house();
    let maren = id(&house.heroes, "Maren");
    house.heroes[maren].fear.conquered = true;
    seat(&mut house, 1, &[maren]);
    aim(&mut house, 1, [3, 3], -2);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(40), 1, [3, 3]);
    assert_eq!(house.heroes[maren].fears_faced, 0);
    let dreads = pooled(&content, Pool::Dreads, &["Maren", "deep water"]);
    assert!(
        !page
            .lines
            .iter()
            .any(|l| dreads.iter().any(|d| l.starts_with(d.as_str())))
    );
}

#[test]
fn a_steadied_loss_adds_one_dread_and_no_courage() {
    let (content, mut house) = house();
    let (garrick, maren) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Maren"));
    seat(&mut house, 1, &[maren, garrick]);
    aim(&mut house, 1, [3, 3], -2);
    resolve_rolled(&content, &mut house, &mut Rng::from_seed(41), 1, [3, 3]);
    // Maren: dread 1, +1 for the loss, +1 for facing it, -1 for her father beside her;
    // then +1 more for seeing her father fail (§12.2).
    assert_eq!(
        (
            house.heroes[maren].fear.dread,
            house.heroes[maren].fear.courage
        ),
        (1 + 1 + 1, 0)
    );
}

#[test]
fn a_settled_hero_who_loses_is_told_no_dread() {
    let (content, mut house) = house();
    let garrick = id(&house.heroes, "Garrick");
    house.heroes[garrick].settled = true;
    seat(&mut house, 1, &[garrick]);
    aim(&mut house, 1, [3, 3], -2);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(42), 1, [3, 3]);
    assert!(
        !page.lines.iter().any(|l| l.contains("Dread ")),
        "{:?}",
        page.lines
    );
    assert_eq!(
        house.heroes[garrick].fears_faced, 1,
        "faced, though settled"
    );
}

#[test]
fn rivals_who_only_succeed_stay_rivals() {
    let (content, mut house) = house();
    let (brannoc, ysolde) = (id(&house.heroes, "Brannoc"), id(&house.heroes, "Ysolde"));
    seat(&mut house, 0, &[brannoc, ysolde]);
    aim(&mut house, 0, [3, 3], 1);
    resolve_rolled(&content, &mut house, &mut Rng::from_seed(43), 0, [3, 3]);
    assert_eq!(
        house.heroes[ysolde].bond_to(brannoc).map(|b| b.kind),
        Some(BondKind::Rival)
    );
}

/// A ghost of Garrick at `place` on slot 0, his dead self off every seat.
fn ghost_on_the_board(content: &crate::content::Content, house: &mut House, place: Place) {
    let garrick = id(&house.heroes, "Garrick");
    house.heroes[garrick].fate = Fate::Dead;
    house.unseat(garrick);
    crate::testkit::paged(house, garrick);
    house.ghosts = vec![crate::ghost::Ghost {
        hero: garrick,
        dream: house.heroes[garrick].dream.clone().expect("Garrick dreams"),
        place,
    }];
    let quest = crate::ghost::ghost_quest(content, &house.heroes, &house.ghosts[0], 0, 1);
    house.board[0] = crate::board::Posted {
        seats: vec![None; 2],
        quest,
    };
}

#[test]
fn a_lost_ghosts_quest_leaves_the_ghost_walking() {
    let (content, mut house) = house();
    let ysolde = id(&house.heroes, "Ysolde");
    ghost_on_the_board(&content, &mut house, Place::Barrow);
    seat(&mut house, 0, &[ysolde]);
    aim(&mut house, 0, [3, 3], -1);
    resolve_rolled(&content, &mut house, &mut Rng::from_seed(44), 0, [3, 3]);
    assert_eq!(house.ghosts.len(), 1);
    assert_eq!(
        house.heroes[id(&house.heroes, "Garrick")].dream_fate,
        DreamFate::Undecided
    );
}

#[test]
fn a_member_crowned_on_a_ghosts_quest_records_no_laying() {
    let (content, mut house) = house();
    let ysolde = id(&house.heroes, "Ysolde");
    ghost_on_the_board(&content, &mut house, Place::KingsCourt);
    house.heroes[ysolde].destiny.kind = Destiny::WearACrown;
    house.heroes[ysolde].renown = 8;
    seat(&mut house, 0, &[ysolde]);
    aim(&mut house, 0, [6, 6], 5);
    resolve_rolled(&content, &mut house, &mut Rng::from_seed(45), 0, [6, 6]);
    assert_eq!(house.heroes[ysolde].fate, Fate::Departed);
    assert!(house.ghosts.is_empty(), "the ghost is laid");
    assert!(
        !house.heroes[ysolde]
            .deeds
            .iter()
            .any(|d| d.kind == DeedKind::LaidGhost)
    );
}
