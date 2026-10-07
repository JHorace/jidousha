//! What the unit tests share: the content and the founding household, and the staging
//! the resolution tests use — a quest put on a slot, a party seated, the demand aimed
//! at a margin, a pool's lines filled.

use jidousha::prelude::Rng;

use crate::board::Posted;
use crate::content::Content;
use crate::hero::{Hero, HeroId};
use crate::house::House;

/// The content and the founding heroes, in creation order.
pub fn founded() -> (Content, Vec<Hero>) {
    let content = crate::content::load().expect("the content loads");
    let heroes = House::found(&content, 1, &mut jidousha::prelude::Rng::from_seed(1))
        .expect("the household founds")
        .heroes;
    (content, heroes)
}

/// A founded house in year 1's summer with every trait taken off: for the mainline rules
/// whose literals count a party's power without the variant's traits (`VARIANT.md`).
pub fn house_without_traits() -> (Content, House) {
    let (content, mut house) = house();
    house.heroes.iter_mut().for_each(|h| h.traits.clear());
    (content, house)
}

/// The id of the founding hero named `name`.
pub fn id(heroes: &[Hero], name: &str) -> HeroId {
    heroes
        .iter()
        .position(|hero| hero.name == name)
        .expect("a founding hero by that name")
}

/// The content and a founded house, in year 1's summer.
pub fn house() -> (Content, House) {
    let content = crate::content::load().expect("the content loads");
    let house = House::found(&content, 1, &mut jidousha::prelude::Rng::from_seed(1))
        .expect("the household founds");
    (content, house)
}

/// Put `title`'s quest on board slot `slot`, posted at its place's trouble.
pub fn stage(content: &Content, house: &mut House, slot: usize, title: &str) {
    let template = content
        .quest_templates
        .iter()
        .position(|t| t.title == title)
        .expect("a template by that title");
    let trouble = house.places[content.quest_templates[template].place.index()].trouble;
    let quest = crate::quest::post(content, template, trouble, 1, &mut Rng::from_seed(3));
    house.board[slot] = Posted {
        seats: vec![None; quest.seats as usize],
        quest,
    };
}

/// Seat `who` on board slot `slot`, adding seats past the quest's if a test needs them.
pub fn seat(house: &mut House, slot: usize, who: &[HeroId]) {
    for (at, &hero) in who.iter().enumerate() {
        house.unseat(hero);
        let seats = &mut house.board[slot].seats;
        if seats.len() <= at {
            seats.push(None);
        }
        seats[at] = Some(hero);
    }
}

/// Set slot `slot`'s demand so the seated party, throwing `dice`, lands on `margin`.
pub fn aim(house: &mut House, slot: usize, dice: [i32; 2], margin: i32) {
    let party = house.party(slot);
    let power = crate::power::party_power(
        &house.heroes,
        &party,
        house.board[slot].quest.facts(),
        house.patrons,
    );
    house.board[slot].quest.demand = power + dice[0] + dice[1] - 7 - margin;
}

/// Every line written that is one of `pool`'s lines filled with `args`.
pub fn pooled(content: &Content, pool: crate::ids::Pool, args: &[&str]) -> Vec<String> {
    content.pools[pool.index()]
        .iter()
        .map(|line| crate::text::fmt(line, args))
        .collect()
}

/// A hero staged dead by hand, given the wording their death page would have rolled
/// (SPEC §15.1 step 1) — every later recomposition reads it (SPEC §20).
pub fn paged(house: &mut House, hero: HeroId) {
    house.heroes[hero].wording = Some(crate::epitaph::roll_wording(
        &mut house.writing,
        &mut Rng::from_seed(hero as u64),
    ));
}
