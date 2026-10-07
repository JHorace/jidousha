//! Family and outsiders (VARIANT.md): the threshold to marry in, what marrying in changes,
//! what an outsider earns the house and who inherits. Expectations are shipped literals
//! from VARIANT.md.

use jidousha::prelude::Rng;

use crate::content::Content;
use crate::hearth::Seat;
use crate::hero::HeroId;
use crate::house::House;
use crate::ids::{BondKind, Place};
use crate::outsiders::may_marry_in;
use crate::plans::{Courtship, Teller, courtship, tellers};
use crate::resolve::resolve_rolled;
use crate::testkit::{house, id, seat};
use crate::winter::{plan, resolve_winter};

/// A wanderer arrived on `house`, aged `age`, with personal renown `renown`.
fn wanderer(content: &Content, house: &mut House, age: i32, renown: i32) -> HeroId {
    crate::wanderer::arrive(content, house, &mut Rng::from_seed(11));
    let hero = house.heroes.len() - 1;
    house.heroes[hero].age = age;
    house.heroes[hero].renown = renown;
    hero
}

#[test]
fn a_wanderer_arrives_an_outsider_and_every_founder_is_family() {
    let (content, mut house) = house();
    assert!(house.heroes.iter().all(|h| h.family));
    let stranger = wanderer(&content, &mut house, 30, 0);
    assert!(!house.heroes[stranger].family);
}

#[test]
fn an_outsider_below_five_renown_is_unproven_and_at_five_will_wed() {
    let (content, mut house) = house();
    let ysolde = id(&house.heroes, "Ysolde");
    let age = house.heroes[ysolde].age;
    let stranger = wanderer(&content, &mut house, age, 4);
    assert_eq!(
        may_marry_in(&house.heroes, ysolde, stranger),
        Some((stranger, 4))
    );
    assert_eq!(
        courtship(&house.heroes, Some(ysolde), Some(stranger)),
        Courtship::Unproven {
            outsider: stranger,
            renown: 4
        }
    );
    let note = courtship(&house.heroes, Some(stranger), Some(ysolde)).note(&content, &house.heroes);
    assert_eq!(
        note,
        format!("{} unproven: renown 4 of 5", house.heroes[stranger].name)
    );
    house.heroes[stranger].renown = 5;
    assert_eq!(may_marry_in(&house.heroes, ysolde, stranger), None);
    assert_eq!(
        courtship(&house.heroes, Some(ysolde), Some(stranger)),
        Courtship::WillWed
    );
}

#[test]
fn two_outsiders_may_wed_each_other_without_a_threshold() {
    let (content, mut house) = house();
    let a = wanderer(&content, &mut house, 30, 0);
    let b = wanderer(&content, &mut house, 30, 0);
    assert_eq!(may_marry_in(&house.heroes, a, b), None);
    assert_eq!(
        courtship(&house.heroes, Some(a), Some(b)),
        Courtship::WillWed
    );
}

#[test]
fn marrying_in_makes_the_outsider_family_takes_the_name_and_pays_half_their_renown_as_dowry() {
    let (content, mut house) = house();
    let ysolde = id(&house.heroes, "Ysolde");
    let age = house.heroes[ysolde].age;
    let stranger = wanderer(&content, &mut house, age, 5);
    house.calendar.begin_winter();
    house.open_hearth();
    for (seat, hero) in [(Seat::Garden(0), ysolde), (Seat::Garden(1), stranger)] {
        house.unseat(hero);
        house.hearth.put(seat, Some(hero));
    }
    let renown = house.renown;
    let lines = {
        let before = plan(&content, &house);
        let (lines, done) = resolve_winter(&content, &mut house, &mut Rng::from_seed(7));
        assert_eq!(done, before);
        lines
    };
    let name = house.heroes[stranger].name.clone();
    assert!(
        lines.iter().any(|l| *l
            == format!("{name} takes the name of Vane, and the house is the richer by 2 renown.")),
        "{lines:?}"
    );
    let hero = &house.heroes[stranger];
    assert!(hero.family);
    assert_eq!(hero.house, "Vane");
    assert_eq!(hero.bond_to(ysolde).map(|b| b.kind), Some(BondKind::Spouse));
    assert_eq!(house.renown, renown + 2);
}

#[test]
fn the_first_family_adult_tells_the_tale_for_the_house_and_an_outsider_only_for_themselves() {
    let (content, mut house) = house();
    let (maren, odo) = (id(&house.heroes, "Maren"), id(&house.heroes, "Odo"));
    let stranger = wanderer(&content, &mut house, 30, 0);
    assert_eq!(
        tellers(&house.heroes, [Some(stranger), Some(odo), Some(maren)]),
        [Some(Teller::Own), Some(Teller::House), Some(Teller::Own)]
    );
    assert_eq!(
        tellers(&house.heroes, [Some(stranger), None, None]),
        [Some(Teller::Own), None, None]
    );
}

#[test]
fn an_outsider_alone_on_a_won_quest_earns_only_themselves_and_with_family_the_house_earns() {
    let (content, mut house) = house();
    let maren = id(&house.heroes, "Maren");
    let stranger = wanderer(&content, &mut house, 30, 0);
    house.heroes[stranger].aptitudes = [9, 9, 9];
    seat(&mut house, 1, &[stranger]);
    house.board[1].quest.demand = 1;
    let renown = house.renown;
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(4), 1, [6, 6]);
    let quest = page.quest.renown + 1;
    assert_eq!(
        page.lines[0],
        format!(
            "{quest} renown to {}. No one of the name went, so the house has none.",
            house.heroes[stranger].name
        )
    );
    assert_eq!(house.renown, renown, "the house gained nothing");
    assert_eq!(house.heroes[stranger].renown, quest);
    // With family beside them the house earns.
    let (content, mut house) = (content, {
        let (_, h) = crate::testkit::house();
        h
    });
    let stranger = wanderer(&content, &mut house, 30, 0);
    house.heroes[stranger].aptitudes = [9, 9, 9];
    seat(&mut house, 1, &[maren, stranger]);
    house.board[1].quest.demand = 1;
    let renown = house.renown;
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(4), 1, [6, 6]);
    assert!(page.lines[0].ends_with("renown to the house, and to each who went."));
    assert_eq!(house.renown, renown + page.quest.renown + 1);
}

#[test]
fn an_outsider_is_never_on_an_heir_list_and_never_inherits_a_crowned_heirloom() {
    let (content, mut house) = house();
    let garrick = id(&house.heroes, "Garrick");
    let stranger = wanderer(&content, &mut house, 30, 0);
    let list = crate::heirs::heirs(&house.heroes, garrick);
    assert!(!list.contains(&stranger));
    house.heroes[stranger].heirloom = house.heroes[garrick].heirloom.clone();
    assert_eq!(crate::heirs::nearest_kin(&house.heroes, stranger), None);
    let _ = content;
}

#[test]
fn an_outsiders_death_page_never_waits_buries_the_heirloom_and_says_nothing_passes_down() {
    let (content, mut house) = house();
    let stranger = wanderer(&content, &mut house, 94, 0);
    house.heroes[stranger].heirloom = house.heroes[id(&house.heroes, "Garrick")].heirloom.clone();
    house.heroes[stranger].fate = crate::hero::Fate::Dead;
    let page =
        crate::death_page::death_page(&content, &mut house, stranger, &mut Rng::from_seed(5));
    let name = house.heroes[stranger].name.clone();
    assert_eq!(
        page.lines[0],
        format!(
            "{name} was never of the name, and the house keeps nothing of {}.",
            content.lore.pronouns[house.heroes[stranger].pronoun.index()].object
        )
    );
    assert!(
        page.lines[1].starts_with("Thornfall was laid in the ground with"),
        "{:?}",
        page.lines
    );
    let bequest = page.bequest.expect("a bequest record");
    assert!(!bequest.leaves && !bequest.undecided() && bequest.heirs.is_empty());
    assert!(house.heroes[stranger].bequest_decided);
    let _ = Place::Barrow;
}

#[test]
fn an_outsiders_carrier_destiny_adds_nothing_to_the_house() {
    let (content, mut house) = house();
    let maren = id(&house.heroes, "Maren");
    let stranger = wanderer(&content, &mut house, 30, 0);
    house.heroes[stranger].aptitudes = [9, 9, 9];
    house.heroes[stranger].destiny.kind = crate::ids::Destiny::CarryTheHouse;
    seat(&mut house, 1, &[maren, stranger]);
    house.board[1].quest.demand = 1;
    let renown = house.renown;
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(4), 1, [6, 6]);
    assert_eq!(house.renown, renown + page.quest.renown + 1);
}

#[test]
fn a_wed_family_hero_is_wed_already_before_an_outsider_is_unproven() {
    let (content, mut house) = house();
    let (maren, brannoc) = (id(&house.heroes, "Maren"), id(&house.heroes, "Brannoc"));
    crate::bonds::form(&mut house.heroes, maren, brannoc, BondKind::Spouse, 1);
    let age = house.heroes[maren].age;
    let stranger = wanderer(&content, &mut house, age, 0);
    assert_eq!(
        courtship(&house.heroes, Some(maren), Some(stranger)),
        Courtship::WedAlready
    );
}
