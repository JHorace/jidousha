//! Destinies (SPEC §13): the predicates the rules of later waves ask — who is
//! shielded on quests, whom a quest's outcome claims, who may still learn — and
//! the Seer speaking an unspoken destiny under the unclaimed rule.
//!
//! The rules that *act* on a claim (burning, crowning, mending, the Door's
//! promise) are W6 and W8; they ask these functions, so the claim is said once.

use jidousha::prelude::*;

use crate::constants::{CROWN_RENOWN, DOOR_DESTINY_POWER};
use crate::content::Content;
use crate::hero::{Hero, HeroId, firstborn};
use crate::ids::{Destiny, Outcome, Place, Tag};

/// Shielded on quests: a second wound does not kill and a disaster's death roll
/// is ignored (FIRE_WILL_END_YOU, DIE_IN_YOUR_BED).
pub fn shields_on_quests(hero: &Hero) -> bool {
    matches!(
        hero.destiny.kind,
        Destiny::FireWillEndYou | Destiny::DieInYourBed
    )
}

/// The fire claims the hero: a SETBACK or DISASTER on a quest carrying Fire.
pub fn fire_claims(hero: &Hero, tags: &[Tag], outcome: Outcome) -> bool {
    hero.destiny.kind == Destiny::FireWillEndYou
        && tags.contains(&Tag::Fire)
        && matches!(outcome, Outcome::Setback | Outcome::Disaster)
}

/// The crown claims the hero: a TRIUMPH at the King's Court with personal renown
/// of at least 8, read after this quest's reward.
pub fn crown_claims(hero: &Hero, place: Place, outcome: Outcome) -> bool {
    hero.destiny.kind == Destiny::WearACrown
        && outcome == Outcome::Triumph
        && place == Place::KingsCourt
        && hero.renown >= CROWN_RENOWN
}

/// The first disaster mends instead of rolling death: BREAK_AND_BE_MENDED, not yet fulfilled.
pub fn mends(hero: &Hero) -> bool {
    hero.destiny.kind == Destiny::BreakAndBeMended && !hero.destiny.fulfilled
}

/// What the Door's promise adds at a lock: +5 for OPEN_THE_SEALED_DOOR (blood of
/// another included), and nothing on any other quest.
pub fn door_power(hero: &Hero, door_lock: bool) -> i32 {
    if door_lock && hero.destiny.kind == Destiny::OpenTheSealedDoor {
        DOOR_DESTINY_POWER
    } else {
        0
    }
}

/// "May still learn" (SPEC §13): not TEACH_A_GREATER, and not CHILD_WILL_SURPASS_YOU
/// once the firstborn (any CHILD bond, living or dead, earliest born) is an adult.
/// It gates the triumph and youth lessons and every winter lesson.
pub fn may_still_learn(heroes: &[Hero], id: HeroId) -> bool {
    match heroes[id].destiny.kind {
        Destiny::TeachAGreater => false,
        Destiny::ChildWillSurpassYou => {
            !firstborn(heroes, id).is_some_and(|child| heroes[child].is_adult())
        }
        _ => true,
    }
}

/// The speakable destinies, and which of them a living hero holds (SPEC §13).
pub fn claimed(content: &Content, heroes: &[Hero]) -> Vec<(Destiny, bool)> {
    Destiny::ALL
        .iter()
        .copied()
        .filter(|kind| content.destinies[kind.index()].speakable)
        .map(|kind| {
            let held = heroes
                .iter()
                .any(|hero| hero.is_living() && hero.destiny.kind == kind);
            (kind, held)
        })
        .collect()
}

/// The Seer speaks (SPEC §13): only for an UNSPOKEN destiny, uniformly among the
/// speakable destinies no living hero holds, or among all eight if every one is
/// held. Returns the destiny spoken, or `None` (and draws nothing) if it was
/// already spoken.
pub fn speak(content: &Content, heroes: &mut [Hero], id: HeroId, rng: &mut Rng) -> Option<Destiny> {
    if heroes[id].destiny.kind != Destiny::Unspoken {
        return None;
    }
    let speakable = claimed(content, heroes);
    let held: Vec<bool> = speakable.iter().map(|(_, held)| *held).collect();
    let (kind, _) = speakable[crate::chance::unclaimed_index(rng, &held)];
    heroes[id].destiny.kind = kind;
    Some(kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hero::Fate;
    use crate::testkit::{founded, id};

    fn with(heroes: &[Hero], name: &str, kind: Destiny) -> Hero {
        let mut hero = heroes[id(heroes, name)].clone();
        hero.destiny.kind = kind;
        hero
    }

    #[test]
    fn the_fire_and_the_bed_shield_on_quests_and_no_other_destiny_does() {
        let (_, heroes) = founded();
        let shielded: Vec<Destiny> = Destiny::ALL
            .iter()
            .copied()
            .filter(|kind| shields_on_quests(&with(&heroes, "Odo", *kind)))
            .collect();
        assert_eq!(shielded, [Destiny::FireWillEndYou, Destiny::DieInYourBed]);
    }

    #[test]
    fn the_fire_claims_on_a_failed_fire_quest_only() {
        let (_, heroes) = founded();
        let brannoc = &heroes[id(&heroes, "Brannoc")];
        let fire = [Tag::Dark, Tag::Fire];
        let claimed: Vec<bool> = Outcome::ALL
            .iter()
            .map(|o| fire_claims(brannoc, &fire, *o))
            .collect();
        assert_eq!(claimed, [true, true, false, false]);
        assert!(!fire_claims(brannoc, &[Tag::Dark], Outcome::Disaster));
        let odo = &heroes[id(&heroes, "Odo")];
        assert!(!fire_claims(odo, &fire, Outcome::Disaster));
    }

    #[test]
    fn the_crown_claims_a_triumph_at_court_at_eight_renown() {
        let (_, heroes) = founded();
        let mut crowned = with(&heroes, "Ysolde", Destiny::WearACrown);
        crowned.renown = 8;
        assert!(crown_claims(&crowned, Place::KingsCourt, Outcome::Triumph));
        assert!(!crown_claims(&crowned, Place::KingsCourt, Outcome::Success));
        assert!(!crown_claims(&crowned, Place::Barrow, Outcome::Triumph));
        crowned.renown = 7;
        assert!(!crown_claims(&crowned, Place::KingsCourt, Outcome::Triumph));
        let mut other = with(&heroes, "Ysolde", Destiny::CarryTheHouse);
        other.renown = 20;
        assert!(!crown_claims(&other, Place::KingsCourt, Outcome::Triumph));
    }

    #[test]
    fn the_first_disaster_mends_only_until_the_destiny_has_come() {
        let (_, heroes) = founded();
        let mut maren = heroes[id(&heroes, "Maren")].clone();
        assert!(mends(&maren));
        maren.destiny.fulfilled = true;
        assert!(!mends(&maren));
        assert!(!mends(&heroes[id(&heroes, "Garrick")]));
    }

    #[test]
    fn the_door_promise_adds_five_at_a_lock_and_nothing_elsewhere() {
        let (_, heroes) = founded();
        let ysolde = &heroes[id(&heroes, "Ysolde")];
        assert_eq!(door_power(ysolde, true), 5);
        assert_eq!(door_power(ysolde, false), 0);
        assert_eq!(door_power(&heroes[id(&heroes, "Odo")], true), 0);
    }

    #[test]
    fn garrick_may_not_learn_because_his_firstborn_is_grown_and_a_greater_teacher_never_may() {
        let (_, mut heroes) = founded();
        let (garrick, maren, odo) = (
            id(&heroes, "Garrick"),
            id(&heroes, "Maren"),
            id(&heroes, "Odo"),
        );
        assert!(!may_still_learn(&heroes, garrick));
        assert!(may_still_learn(&heroes, maren));
        assert!(may_still_learn(&heroes, odo));
        heroes[odo].destiny.kind = Destiny::TeachAGreater;
        assert!(!may_still_learn(&heroes, odo));
        // A surpass-destiny whose firstborn is still a child may learn; so may one with no child.
        heroes[maren].destiny.kind = Destiny::ChildWillSurpassYou;
        assert!(may_still_learn(&heroes, maren), "Pip is ten");
        let pip = id(&heroes, "Pip");
        heroes[pip].age = 12;
        assert!(!may_still_learn(&heroes, maren), "Pip is of age");
        heroes[odo].destiny.kind = Destiny::ChildWillSurpassYou;
        assert!(may_still_learn(&heroes, odo));
    }

    #[test]
    fn the_seer_speaks_only_destinies_no_living_hero_holds() {
        let (content, mut heroes) = founded();
        let pip = id(&heroes, "Pip");
        // Living holders at founding: Garrick (surpass), Maren (mended), Brannoc (fire), Odo (outlive).
        let held: Vec<Destiny> = claimed(&content, &heroes)
            .into_iter()
            .filter(|(_, h)| *h)
            .map(|(k, _)| k)
            .collect();
        assert_eq!(
            held,
            [
                Destiny::FireWillEndYou,
                Destiny::OutliveThoseYouLove,
                Destiny::ChildWillSurpassYou,
                Destiny::BreakAndBeMended
            ]
        );
        let unheld = [
            Destiny::WearACrown,
            Destiny::DieInYourBed,
            Destiny::CarryTheHouse,
            Destiny::TeachAGreater,
        ];
        let mut seen = Vec::new();
        for seed in 0..200 {
            let mut trial = heroes.clone();
            let spoken = speak(&content, &mut trial, pip, &mut Rng::from_seed(seed));
            let Some(spoken) = spoken else {
                panic!("an unspoken destiny was not spoken")
            };
            assert!(
                unheld.contains(&spoken),
                "seed {seed} spoke {spoken:?}, which a living hero holds"
            );
            assert_eq!(trial[pip].destiny.kind, spoken);
            if !seen.contains(&spoken) {
                seen.push(spoken);
            }
        }
        assert_eq!(
            seen.len(),
            4,
            "every unheld destiny can be spoken: {seen:?}"
        );
        // A dead holder does not claim.
        let brannoc = id(&heroes, "Brannoc");
        heroes[brannoc].fate = Fate::Dead;
        assert!(!claimed(&content, &heroes)[0].1);
    }

    #[test]
    fn when_every_destiny_is_held_the_seer_speaks_any_of_the_eight_and_never_the_door() {
        let (content, mut heroes) = founded();
        let speakable: Vec<Destiny> = claimed(&content, &heroes)
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(speakable.len(), 8);
        assert!(
            !speakable.contains(&Destiny::OpenTheSealedDoor)
                && !speakable.contains(&Destiny::Unspoken)
        );
        // Everyone but Wren holds one of the eight, alive (the two dead founders revived).
        let wren = id(&heroes, "Wren");
        let holders: Vec<HeroId> = (0..heroes.len()).filter(|&h| h != wren).collect();
        for (holder, kind) in holders.iter().zip(&speakable) {
            heroes[*holder].fate = Fate::Living;
            heroes[*holder].destiny.kind = *kind;
        }
        assert!(claimed(&content, &heroes).iter().all(|(_, held)| *held));
        let mut seen = Vec::new();
        for seed in 0..400 {
            let mut trial = heroes.clone();
            trial[wren].destiny.kind = Destiny::Unspoken;
            let spoken = speak(&content, &mut trial, wren, &mut Rng::from_seed(seed));
            if let Some(kind) = spoken
                && !seen.contains(&kind)
            {
                seen.push(kind);
            }
        }
        assert_eq!(seen.len(), 8, "{seen:?}");
    }

    #[test]
    fn a_spoken_destiny_is_never_spoken_again_and_draws_nothing() {
        let (content, mut heroes) = founded();
        let odo = id(&heroes, "Odo");
        let mut rng = Rng::from_seed(7);
        let before = rng.clone().next_u32();
        assert_eq!(speak(&content, &mut heroes, odo, &mut rng), None);
        assert_eq!(rng.next_u32(), before);
        assert_eq!(heroes[odo].destiny.kind, Destiny::OutliveThoseYouLove);
    }
}
