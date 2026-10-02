//! Old age and the death pages (SPEC §18 steps 3-4, §15.1, §15.4): who the winter's
//! end takes, and the page each of the mourned is given.
//!
//! An old-age death is told on no page of its own but the death page: its fate
//! telling is a SLEEP_DEATHS line, it grieves the house on that page (§12.5), and a
//! carrier's -4 is lost silently [emergent] (OQ-17). Every death page, the summer's
//! dead first, records what the dead leave, passes the Door's promise, and — when
//! there is an heirloom or an undone dream to leave — gathers the heirs and waits for
//! the player's choice (`heirs::choose`).

use jidousha::prelude::Rng;

use crate::chance::chance;
use crate::constants::{
    BED_AGE_FACTOR, CARRIER_LOSS, OLD_AGE_BASE_CHANCE, OLD_AGE_FROM, OLD_AGE_YEARLY_CHANCE,
    OUTLIVING_AGE_FACTOR,
};
use crate::content::Content;
use crate::dream::{progress, told_title};
use crate::grief::grieve;
use crate::heirs::{heirs, undone_dream};
use crate::hero::{DreamFate, Fate, Hero, HeroId};
use crate::house::House;
use crate::ids::{BondKind, Destiny, Pool};
use crate::passage::{Bequest, PageKind, TurnPage};
use crate::text::{capitalized, fmt};
use crate::words::W;

/// The chance old age takes `hero` this turning (SPEC §18 step 3, CONSTANTS §10):
/// `min((0.04 + 0.025 * (age - 55)) * factor, 1)` from 55, 0 below it and for
/// FIRE_WILL_END_YOU; the factor is 0.5 for OUTLIVE_THOSE_YOU_LOVE, 2 for DIE_IN_YOUR_BED.
pub fn old_age_chance(hero: &Hero) -> f64 {
    if hero.age < OLD_AGE_FROM || hero.destiny.kind == Destiny::FireWillEndYou {
        return 0.0;
    }
    let factor = match hero.destiny.kind {
        Destiny::OutliveThoseYouLove => OUTLIVING_AGE_FACTOR,
        Destiny::DieInYourBed => BED_AGE_FACTOR,
        _ => 1.0,
    };
    ((OLD_AGE_BASE_CHANCE + OLD_AGE_YEARLY_CHANCE * f64::from(hero.age - OLD_AGE_FROM)) * factor)
        .min(1.0)
}

/// Old age (SPEC §18 step 3): one roll for each living hero, in creation order, even at
/// chance 0 (§22.2). A death takes a SLEEP_DEATHS line as its fate telling, out of every
/// seat and into the mourned; a carrier's -4 is lost with no line (OQ-17).
pub fn old_age(content: &Content, house: &mut House, rng: &mut Rng) {
    let year = house.calendar.current_year();
    for id in 0..house.heroes.len() {
        if !house.heroes[id].is_living() {
            continue;
        }
        let p = old_age_chance(&house.heroes[id]);
        if !chance(rng, p as f32) {
            continue;
        }
        let telling = house
            .writing
            .pick(content, Pool::SleepDeaths, rng)
            .to_owned();
        let hero = &mut house.heroes[id];
        hero.fate = Fate::Dead;
        hero.fate_year = year;
        hero.fate_age = hero.age;
        hero.fate_telling = telling;
        let carrier = hero.destiny.kind == Destiny::CarryTheHouse;
        house.unseat(id);
        house.mourned.push(id);
        if carrier {
            house.add_renown(-CARRIER_LOSS);
        }
    }
}

/// The death page for `dead` (SPEC §15.1, `lineage/passage.jai:146-184`): the bequest
/// lines — the heirloom left, the dream left undone — then the Door's promise and, for
/// an old-age death, grief. The dream's fate is settled where there is nothing to
/// choose; with an heirloom or an undone dream the heirs are gathered and the page
/// waits. The epitaph wording the original rolls here, and the epitaph it composes,
/// are W9's: until then the page shows the dead's condition line.
pub fn death_page(content: &Content, house: &mut House, dead: HeroId) -> TurnPage {
    let words = &content.words;
    let year = house.calendar.current_year();
    let mut lines = Vec::new();
    let hero = &house.heroes[dead];
    let he = capitalized(&content.lore.pronouns[hero.pronoun.index()].subject);
    if let Some(heirloom) = &hero.heirloom {
        let effect = fmt(
            &content.legacies.heirloom_effect,
            &[
                &heirloom.bonus.to_string(),
                &content.lore.aptitudes[heirloom.aptitude.index()],
            ],
        );
        lines.push(fmt(
            &words[W::DeathLeavesHeirloom],
            &[&he, &heirloom.name, &effect],
        ));
    }
    let undone = undone_dream(hero).map(|(_, dream)| dream.clone());
    if let Some(dream) = &undone {
        let owner = dream
            .owner
            .map_or(hero.pronoun, |o| house.heroes[o].pronoun);
        lines.push(fmt(
            &words[W::DeathLeavesDream],
            &[
                &he,
                &told_title(content, dream, owner),
                // SPEC-GAPS KG-14: told about the hero the line is about.
                &progress(content, dream, hero.pronoun),
            ],
        ));
    }
    let bequest_end = lines.len();
    let heirloom = hero.heirloom.as_ref().map(|h| h.name.clone());
    house.heroes[dead].bequest_heirloom = heirloom.clone();
    lines.extend(door_promise(content, &mut house.heroes, dead));
    lines.extend(grieve(content, &mut house.heroes, dead, year));
    let hero = &mut house.heroes[dead];
    match &hero.dream {
        None => hero.dream_fate = DreamFate::NeverDreamt,
        Some(dream) if dream.is_fulfilled() => hero.dream_fate = DreamFate::Fulfilled,
        Some(_) => {}
    }
    let leaves = heirloom.is_some() || undone.is_some();
    let heirs = if leaves {
        heirs(&house.heroes, dead)
    } else {
        house.heroes[dead].bequest_decided = true;
        Vec::new()
    };
    TurnPage {
        kind: PageKind::Death,
        title: fmt(&words[W::DeathTitle], &[&house.heroes[dead].full_name()]),
        lines,
        bequest: Some(Bequest {
            dead,
            heirs,
            chosen: None,
            bequest_end,
        }),
        about: Some(dead),
    }
}

/// The Door's promise (SPEC §15.4, `lineage/passage.jai:313-334`): a dead
/// OPEN_THE_SEALED_DOOR's firstborn living child — the earliest born of the living
/// children, first found on ties — takes the promise, blood of the first promisee,
/// whatever the Seer said to them before; with no living child it ends. The fulfilled
/// flag it tests is never set (OQ-32). Returns the line, if any.
pub fn door_promise(content: &Content, heroes: &mut [Hero], dead: HeroId) -> Vec<String> {
    let words = &content.words;
    let hero = &heroes[dead];
    if hero.destiny.kind != Destiny::OpenTheSealedDoor || hero.destiny.fulfilled {
        return Vec::new();
    }
    let mut firstborn: Option<HeroId> = None;
    for bond in hero.bonds.iter().filter(|b| b.kind == BondKind::Child) {
        let child = &heroes[bond.other];
        if child.is_living() && firstborn.is_none_or(|f| child.born_year < heroes[f].born_year) {
            firstborn = Some(bond.other);
        }
    }
    let Some(child) = firstborn else {
        return vec![fmt(&words[W::PromiseNoChild], &[&hero.name])];
    };
    let blood = hero
        .destiny
        .blood_of
        .clone()
        .unwrap_or_else(|| hero.name.clone());
    let line = fmt(
        &words[W::PromisePasses],
        &[
            &hero.name,
            &content.lore.pronouns[hero.pronoun.index()].possessive,
            &heroes[child].name,
            &blood,
            &heroes[child].name,
        ],
    );
    heroes[child].destiny = crate::hero::DestinyState {
        kind: Destiny::OpenTheSealedDoor,
        fulfilled: false,
        blood_of: Some(blood),
    };
    vec![line]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{founded, id};

    #[test]
    fn old_age_comes_at_constants_tens_rates() {
        let (_, heroes) = founded();
        let mut hero = heroes[id(&heroes, "Garrick")].clone();
        let at = |hero: &mut Hero, age: i32, kind: Destiny| {
            hero.age = age;
            hero.destiny.kind = kind;
            (old_age_chance(hero) * 10_000.0).round() as i32
        };
        // CONSTANTS §10's table, in hundredths of a percent.
        let ordinary = Destiny::Unspoken;
        assert_eq!(at(&mut hero, 54, ordinary), 0);
        assert_eq!(at(&mut hero, 55, ordinary), 400);
        assert_eq!(at(&mut hero, 56, ordinary), 650);
        assert_eq!(at(&mut hero, 63, ordinary), 2400);
        assert_eq!(at(&mut hero, 70, ordinary), 4150);
        assert_eq!(at(&mut hero, 93, ordinary), 9900);
        assert_eq!(at(&mut hero, 94, ordinary), 10_000);
        assert_eq!(at(&mut hero, 99, ordinary), 10_000);
        assert_eq!(at(&mut hero, 58, Destiny::OutliveThoseYouLove), 575);
        assert_eq!(at(&mut hero, 94, Destiny::OutliveThoseYouLove), 5075);
        assert_eq!(at(&mut hero, 60, Destiny::DieInYourBed), 3300);
        assert_eq!(at(&mut hero, 74, Destiny::DieInYourBed), 10_000);
        assert_eq!(at(&mut hero, 90, Destiny::FireWillEndYou), 0);
    }

    #[test]
    fn the_door_falls_to_the_earliest_born_living_child_and_ends_with_none() {
        let (content, mut heroes) = founded();
        let (ysolde, pip, wren) = (
            id(&heroes, "Ysolde"),
            id(&heroes, "Pip"),
            id(&heroes, "Wren"),
        );
        assert_eq!(
            door_promise(&content, &mut heroes, ysolde),
            [
                "Ysolde was promised to the Sealed Door, and has no child living. The Door will \
              open for no one by name."
            ]
        );
        crate::bonds::form(&mut heroes, wren, ysolde, BondKind::Parent, 1);
        crate::bonds::form(&mut heroes, pip, ysolde, BondKind::Parent, 1);
        // Pip (born -9) is older than Wren (born -7): Pip is the firstborn living child.
        assert_eq!(
            door_promise(&content, &mut heroes, ysolde),
            [
                "The Door was promised to Ysolde. It falls to her firstborn, Pip: blood of \
              Ysolde. Whatever the Seer said to Pip before is set aside."
            ]
        );
        assert_eq!(heroes[pip].destiny.kind, Destiny::OpenTheSealedDoor);
        assert_eq!(heroes[pip].destiny.blood_of.as_deref(), Some("Ysolde"));
        // From Pip, dead, it goes on to Wren, still blood of Ysolde.
        heroes[pip].fate = Fate::Dead;
        crate::bonds::form(&mut heroes, wren, pip, BondKind::Parent, 1);
        door_promise(&content, &mut heroes, pip);
        assert_eq!(heroes[wren].destiny.blood_of.as_deref(), Some("Ysolde"));
        // A dead child is passed over.
        let mut again = heroes.clone();
        again[wren].fate = Fate::Dead;
        again[pip].fate = Fate::Living;
        again[pip].destiny.kind = Destiny::Unspoken;
        again[pip].destiny.blood_of = None;
        door_promise(&content, &mut again, ysolde);
        assert_eq!(again[pip].destiny.kind, Destiny::OpenTheSealedDoor);
        let odo = id(&heroes, "Odo");
        assert!(door_promise(&content, &mut heroes, odo).is_empty());
    }
}
