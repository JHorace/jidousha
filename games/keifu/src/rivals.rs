//! Dream rivals (SPEC §12.3): two who want the same thing, with nothing between
//! them, become rivals when one of them gains the dream.
//!
//! The rule compares dream *kinds* only, so two AVENGE_THE_LOST dreams for
//! different people are the same kind and make rivals [emergent] (OQ-27). Its
//! callers are the three ways a hero gains a dream — a wanderer's arrival, a coming
//! of age, a dream passed on (W8) — and they call it after the dream is held.

use crate::bonds::form;
use crate::content::Content;
use crate::dream::{Dream, told_title};
use crate::hero::{Hero, HeroId, kin};
use crate::ids::BondKind;
use crate::text::fmt;
use crate::words::W;

/// `hero` has just gained `gained` (SPEC §12.3). If it is dreamt and unfulfilled,
/// every other living adult whose own dream is unfulfilled and of the same kind,
/// who is not kin and holds no bond to the hero but a companionship, becomes a
/// rival, in creation order. The hero may be a child. Returns the lines.
pub fn dream_rivals(
    content: &Content,
    heroes: &mut [Hero],
    hero: HeroId,
    gained: &Dream,
    year: i32,
) -> Vec<String> {
    let mut lines = Vec::new();
    if gained.is_fulfilled() {
        return lines;
    }
    let pronoun = gained
        .owner
        .map_or(heroes[hero].pronoun, |owner| heroes[owner].pronoun);
    let title = told_title(content, gained, pronoun);
    for other in 0..heroes.len() {
        let them = &heroes[other];
        // SPEC-GAPS KG-20: the other's own dream, not a burden they carry.
        let same = them
            .dream
            .as_ref()
            .is_some_and(|dream| dream.kind == gained.kind && !dream.is_fulfilled());
        let unbound = them
            .bond_to(hero)
            .is_none_or(|bond| bond.kind == BondKind::Companion);
        if other != hero
            && them.is_living()
            && them.is_adult()
            && same
            && unbound
            && !kin(heroes, hero, other)
        {
            form(heroes, hero, other, BondKind::Rival, year);
            lines.push(fmt(
                &content.words[W::BondDreamRivals],
                &[&heroes[hero].name, &heroes[other].name, &title],
            ));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::Setup;
    use crate::ids::{DreamKind, Place, Pronoun, Tag};
    use crate::testkit::{founded, id};

    fn avenge(content: &Content, place: Place, tag: Tag, pronoun: Pronoun) -> Dream {
        Dream::build(
            content,
            DreamKind::AvengeTheLost,
            Some(Setup {
                place,
                tag,
                lost: None,
            }),
            Some(pronoun),
        )
        .expect("builds")
    }

    #[test]
    fn two_avengers_of_different_dead_are_rivals_all_the_same() {
        let (content, mut heroes) = founded();
        let (maren, ysolde) = (id(&heroes, "Maren"), id(&heroes, "Ysolde"));
        let gained = avenge(&content, Place::HighPass, Tag::Cold, Pronoun::He);
        heroes[ysolde].dream = Some(gained.clone());
        let lines = dream_rivals(&content, &mut heroes, ysolde, &gained, 3);
        assert_eq!(
            lines,
            [
                "Ysolde wants what Maren wants: to avenge her father. There is only room for one of them to be first. They are rivals."
            ]
        );
        assert_eq!(
            heroes[ysolde].bond_to(maren).map(|b| (b.kind, b.since)),
            Some((BondKind::Rival, 3))
        );
        assert_eq!(
            heroes[maren].bond_to(ysolde).map(|b| b.kind),
            Some(BondKind::Rival)
        );
    }

    #[test]
    fn kin_the_bonded_the_young_the_dead_and_the_done_are_never_dream_rivals() {
        let (content, mut heroes) = founded();
        let (garrick, maren, pip, odo, brannoc) = (
            id(&heroes, "Garrick"),
            id(&heroes, "Maren"),
            id(&heroes, "Pip"),
            id(&heroes, "Odo"),
            id(&heroes, "Brannoc"),
        );
        let barrow = heroes[garrick].dream.clone().expect("Garrick dreams");
        // Maren is Garrick's daughter (kin); Odo his friend; Pip a child; Brannoc
        // a stranger, who has it fulfilled.
        for hero in [maren, odo, pip, brannoc] {
            heroes[hero].dream = Some(barrow.clone());
        }
        heroes[brannoc]
            .dream
            .as_mut()
            .expect("set")
            .advance_to_stage(3);
        assert!(dream_rivals(&content, &mut heroes, garrick, &barrow, 2).is_empty());
        // Unfulfilled, Brannoc is a rival; but a companionship does not stand in the way.
        heroes[brannoc].dream = Some(barrow.clone());
        crate::bonds::form(&mut heroes, garrick, brannoc, BondKind::Companion, 1);
        assert_eq!(
            dream_rivals(&content, &mut heroes, garrick, &barrow, 2).len(),
            1
        );
        assert_eq!(
            heroes[garrick].bond_to(brannoc).map(|b| b.kind),
            Some(BondKind::Rival)
        );
        // Dead, nobody is a rival; a fulfilled gained dream makes none.
        let mut done = barrow.clone();
        done.advance_to_stage(3);
        heroes[brannoc].bonds.clear();
        heroes[garrick].bonds.retain(|b| b.other != brannoc);
        assert!(dream_rivals(&content, &mut heroes, garrick, &done, 2).is_empty());
        heroes[brannoc].fate = crate::hero::Fate::Dead;
        assert!(dream_rivals(&content, &mut heroes, garrick, &barrow, 2).is_empty());
    }

    #[test]
    fn a_child_gaining_a_dream_can_make_an_adult_rival() {
        let (content, mut heroes) = founded();
        let (wren, odo) = (id(&heroes, "Wren"), id(&heroes, "Odo"));
        let student = heroes[odo].dream.clone().expect("Odo dreams");
        heroes[wren].dream = Some(student.clone());
        let lines = dream_rivals(&content, &mut heroes, wren, &student, 4);
        assert_eq!(lines.len(), 1);
        assert_eq!(
            heroes[odo].bond_to(wren).map(|b| b.kind),
            Some(BondKind::Rival)
        );
    }

    #[test]
    fn siblings_with_no_bond_between_them_are_still_kin_and_never_dream_rivals() {
        let (content, mut heroes) = founded();
        let wren = id(&heroes, "Wren");
        let mut sibling = heroes[wren].clone();
        sibling.name = "Ash".into();
        sibling.bonds.clear();
        heroes.push(sibling);
        let ash = heroes.len() - 1;
        let road = heroes[id(&heroes, "Ysolde")]
            .dream
            .clone()
            .expect("Ysolde dreams");
        heroes[wren].age = 12;
        heroes[ash].age = 12;
        heroes[ash].dream = Some(road.clone());
        heroes[wren].dream = Some(road.clone());
        assert!(heroes[wren].bond_to(ash).is_none());
        let lines = dream_rivals(&content, &mut heroes, wren, &road, 5);
        // Ysolde is a rival; Ash, a sister by the same parents, is not.
        assert_eq!(lines.len(), 1);
        assert!(heroes[wren].bond_to(ash).is_none());
    }

    #[test]
    fn a_carried_dream_is_told_with_its_owners_pronoun_in_the_rivals_line() {
        let (content, mut heroes) = founded();
        let (garrick, maren, ysolde) = (
            id(&heroes, "Garrick"),
            id(&heroes, "Maren"),
            id(&heroes, "Ysolde"),
        );
        heroes[ysolde].dream = Some(avenge(&content, Place::HighPass, Tag::Cold, Pronoun::He));
        let mut gained = heroes[maren].dream.clone().expect("Maren dreams");
        gained.owner = Some(maren);
        heroes[garrick].burden = Some(gained.clone());
        let lines = dream_rivals(&content, &mut heroes, garrick, &gained, 3);
        assert_eq!(
            lines,
            [
                "Garrick wants what Ysolde wants: to avenge her mother. There is only room for one of them to be first. They are rivals."
            ]
        );
    }
}
