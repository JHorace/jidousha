//! Blessings (SPEC §14.1, §6 line 7): wording one for a dream, laying it on its
//! recipients, telling its effect, and what it adds to a member's power.
//!
//! `blessing_power` is the one place a blessing's reach is judged; the power sum
//! (`power::member_power`) asks it, and so will W4's breakdown line.

use crate::content::Content;
use crate::dream::Dream;
use crate::hero::{Blessing, Hero, HeroId, Scope, descends_from};
use crate::ids::{DreamKind, Place, Tag};
use crate::legacy_lore::ScopeLore;
use crate::text::fmt;

/// The blessing `dream` leaves, named for `dreamer` (SPEC §14.1): "<dreamer>'s
/// rest" +2 against Undead, "<dreamer>'s debt, paid" +2 at the avenged place, or
/// "<dreamer>'s patience" +1 on every quest.
pub fn word_blessing(content: &Content, dream: &Dream, dreamer: &Hero) -> Blessing {
    let lore = &content.legacies.blessings[match dream.kind {
        DreamKind::QuietTheBarrow => 0,
        DreamKind::AvengeTheLost => 1,
        _ => 2,
    }];
    let scope = match lore.scope {
        ScopeLore::AgainstTag(tag) => Scope::AgainstTag(tag),
        ScopeLore::AtSetupPlace => match dream.setup {
            Some(setup) => Scope::AtPlace(setup.place),
            None => panic!(
                "[keifu_x_inheritance_r3v2] {}'s blessing is laid at its setup place, and the dream holds no \
                 setup\n  likely cause: a dream built without Dream::build\n  fix: build \
                 every dream with Dream::build",
                dream.kind.id()
            ),
        },
        ScopeLore::Everywhere => Scope::Everywhere,
    };
    Blessing {
        title: fmt(&lore.title, &[&dreamer.name]),
        scope,
        power: lore.power,
    }
}

/// "+2 against Undead", "+2 at the Drowned Coast", "+1 on every quest".
pub fn blessing_effect(content: &Content, blessing: &Blessing) -> String {
    let effects = &content.legacies.blessing_effects;
    let power = blessing.power.to_string();
    match blessing.scope {
        Scope::AgainstTag(tag) => fmt(
            &effects[0],
            &[&power, &content.lore.tags[tag.index()].title],
        ),
        // SPEC-GAPS KG-17: the place mid-sentence, by its `name`.
        Scope::AtPlace(place) => fmt(
            &effects[1],
            &[&power, &content.lore.places[place.index()].name],
        ),
        Scope::Everywhere => fmt(&effects[2], &[&power]),
    }
}

/// Lay `blessing` on its recipients (SPEC §14.1): the fulfiller, every descendant
/// of the dreamer, every descendant of the fulfiller, and for WORTHY_STUDENT
/// everyone the fulfiller taught. Each living one receives it unless they already
/// hold a blessing of that title. Returns the newly blessed, in the order blessed.
pub fn bless(
    heroes: &mut [Hero],
    blessing: &Blessing,
    kind: DreamKind,
    dreamer: HeroId,
    fulfiller: HeroId,
) -> Vec<HeroId> {
    // SPEC-GAPS KG-16: descendants in creation order, the taught in bond order.
    let mut candidates = vec![fulfiller];
    for ancestor in [dreamer, fulfiller] {
        candidates.extend((0..heroes.len()).filter(|&id| descends_from(heroes, id, ancestor)));
    }
    if kind == DreamKind::WorthyStudent {
        candidates.extend(
            heroes[fulfiller]
                .bonds
                .iter()
                .filter(|bond| bond.taught)
                .map(|bond| bond.other),
        );
    }
    let mut blessed = Vec::new();
    for id in candidates {
        let hero = &mut heroes[id];
        if hero.is_living() && !hero.blessings.iter().any(|b| b.title == blessing.title) {
            hero.blessings.push(blessing.clone());
            blessed.push(id);
        }
    }
    blessed
}

/// Whether a blessing applies on a quest at `place` carrying `tags` (SPEC §6 line 7).
pub fn applies(blessing: &Blessing, place: Place, tags: &[Tag]) -> bool {
    match blessing.scope {
        Scope::AgainstTag(tag) => tags.contains(&tag),
        Scope::AtPlace(at) => at == place,
        Scope::Everywhere => true,
    }
}

/// SPEC §6 line 7: `+power` for each of the hero's blessings that applies.
pub fn blessing_power(hero: &Hero, place: Place, tags: &[Tag]) -> i32 {
    hero.blessings
        .iter()
        .filter(|blessing| applies(blessing, place, tags))
        .map(|blessing| blessing.power)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::Setup;
    use crate::testkit::{founded, id};

    #[test]
    fn garricks_rest_is_two_against_the_undead_and_reaches_his_daughter_and_grandson() {
        let (content, mut heroes) = founded();
        let garrick = id(&heroes, "Garrick");
        let dream = heroes[garrick].dream.clone().expect("Garrick dreams");
        let blessing = word_blessing(&content, &dream, &heroes[garrick]);
        assert_eq!(blessing.title, "Garrick's rest");
        assert_eq!(blessing_effect(&content, &blessing), "+2 against Undead");
        let blessed = bless(&mut heroes, &blessing, dream.kind, garrick, garrick);
        let names: Vec<&str> = blessed.iter().map(|&b| heroes[b].name.as_str()).collect();
        assert_eq!(names, ["Garrick", "Maren", "Pip"]);
        assert!(
            bless(&mut heroes, &blessing, dream.kind, garrick, garrick).is_empty(),
            "a title already held is not laid twice"
        );
    }

    #[test]
    fn a_debt_paid_is_laid_at_the_avenged_place_and_patience_everywhere() {
        let (content, heroes) = founded();
        let maren = id(&heroes, "Maren");
        let dream = heroes[maren].dream.clone().expect("Maren dreams");
        let debt = word_blessing(&content, &dream, &heroes[maren]);
        assert_eq!(debt.title, "Maren's debt, paid");
        assert_eq!(blessing_effect(&content, &debt), "+2 at the Drowned Coast");
        assert!(applies(&debt, Place::DrownedCoast, &[]));
        assert!(!applies(&debt, Place::Barrow, &[Tag::Water]));
        let odo = id(&heroes, "Odo");
        let patience = word_blessing(
            &content,
            &heroes[odo].dream.clone().expect("Odo dreams"),
            &heroes[odo],
        );
        assert_eq!(patience.title, "Odo's patience");
        assert_eq!(blessing_effect(&content, &patience), "+1 on every quest");
        assert!(applies(&patience, Place::SealedDoor, &[]));
    }

    #[test]
    fn a_worthy_students_patience_reaches_the_taught_and_skips_the_dead() {
        let (content, mut heroes) = founded();
        let (odo, wren, ysolde) = (
            id(&heroes, "Odo"),
            id(&heroes, "Wren"),
            id(&heroes, "Ysolde"),
        );
        crate::bonds::form(&mut heroes, wren, odo, crate::ids::BondKind::Mentor, 1);
        crate::bonds::form(&mut heroes, ysolde, odo, crate::ids::BondKind::Mentor, 1);
        for student in [wren, ysolde] {
            if let Some(bond) = heroes[odo].bonds.iter_mut().find(|b| b.other == student) {
                bond.taught = true;
            }
        }
        heroes[ysolde].fate = crate::hero::Fate::Dead;
        let dream = heroes[odo].dream.clone().expect("Odo dreams");
        let blessing = word_blessing(&content, &dream, &heroes[odo]);
        let blessed = bless(&mut heroes, &blessing, dream.kind, odo, odo);
        assert_eq!(blessed, [odo, wren]);
        // Without the dream kind, the taught are not recipients.
        let other = Blessing {
            title: "x".into(),
            ..blessing
        };
        assert_eq!(
            bless(&mut heroes, &other, DreamKind::QuietTheBarrow, odo, odo),
            [odo]
        );
    }

    #[test]
    fn a_carried_dream_is_named_for_its_dreamer_and_reaches_both_lines() {
        let (content, mut heroes) = founded();
        let (elsbeth, odo, brannoc, wren) = (
            id(&heroes, "Elsbeth"),
            id(&heroes, "Odo"),
            id(&heroes, "Brannoc"),
            id(&heroes, "Wren"),
        );
        let mut dream = Dream::build(
            &content,
            DreamKind::AvengeTheLost,
            Some(Setup {
                place: Place::HighPass,
                tag: Tag::Cold,
                lost: None,
            }),
            None,
        )
        .expect("builds");
        dream.owner = Some(elsbeth);
        let blessing = word_blessing(&content, &dream, &heroes[elsbeth]);
        assert_eq!(blessing.title, "Elsbeth's debt, paid");
        assert_eq!(blessing.scope, Scope::AtPlace(Place::HighPass));
        let blessed = bless(&mut heroes, &blessing, dream.kind, elsbeth, brannoc);
        let names: Vec<&str> = blessed.iter().map(|&b| heroes[b].name.as_str()).collect();
        assert_eq!(names, ["Brannoc", "Maren", "Pip", "Wren"]);
        assert!(!blessed.contains(&odo));
        assert_eq!(heroes[wren].blessings.len(), 1);
    }

    #[test]
    fn each_applying_blessing_adds_its_power_once() {
        let (_, mut heroes) = founded();
        let garrick = id(&heroes, "Garrick");
        heroes[garrick].blessings = vec![
            Blessing {
                title: "a".into(),
                scope: Scope::AgainstTag(Tag::Undead),
                power: 2,
            },
            Blessing {
                title: "b".into(),
                scope: Scope::AtPlace(Place::Barrow),
                power: 2,
            },
            Blessing {
                title: "c".into(),
                scope: Scope::Everywhere,
                power: 1,
            },
        ];
        let hero = &heroes[garrick];
        assert_eq!(
            blessing_power(hero, Place::Barrow, &[Tag::Dark, Tag::Undead]),
            5
        );
        assert_eq!(blessing_power(hero, Place::Barrow, &[Tag::Dark]), 3);
        assert_eq!(blessing_power(hero, Place::Deepwood, &[Tag::Undead]), 3);
        assert_eq!(blessing_power(hero, Place::Deepwood, &[Tag::Beasts]), 1);
    }
}
