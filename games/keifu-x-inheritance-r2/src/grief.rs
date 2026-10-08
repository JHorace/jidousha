//! Grief (SPEC §12.5): when a hero dies, everyone bonded to them takes dread by
//! their own bond's kind, through the one dread rule.
//!
//! W6 calls `grieve` on the quest page for a quest death and W8 on the death page
//! for an old-age one; this wave owns the rule and the lines it writes.

use crate::bonds::kinship_telling;
use crate::constants::{DREAD_LIMIT, OUTLIVING_DREAD, bond_grief};
use crate::content::Content;
use crate::fear::{Occasion, add_dread, can_dread};
use crate::hero::{Hero, HeroId};
use crate::ids::{BondKind, Destiny};
use crate::text::{fmt, name_list};
use crate::words::W;

/// The dread a mourner takes for a bond of `kind` to the dead: the kind's grief,
/// +2 for OUTLIVE_THOSE_YOU_LOVE when there is any grief at all (CONSTANTS §6, §8).
pub fn grief_for(mourner: &Hero, kind: BondKind) -> i32 {
    let base = bond_grief(kind);
    if base > 0 && mourner.destiny.kind == Destiny::OutliveThoseYouLove {
        base + OUTLIVING_DREAD
    } else {
        base
    }
}

/// Grieve `dead` (SPEC §12.5), once: for each of the dead's bonds in order, to a
/// living mourner, by the mourner's bond kind to the dead. Rivals have no one left
/// to be better than; companions grieve nothing; whoever cannot dread bears it,
/// and the bearers are told together at the end. Returns the lines, in order.
pub fn grieve(content: &Content, heroes: &mut [Hero], dead: HeroId, year: i32) -> Vec<String> {
    let words = &content.words;
    let mut lines = Vec::new();
    // SPEC §15.1 step 5: "grieve if not already grieved".
    if heroes[dead].grieved {
        return lines;
    }
    heroes[dead].grieved = true;
    let mut bearers: Vec<(HeroId, String)> = Vec::new();
    let mourners: Vec<HeroId> = heroes[dead].bonds.iter().map(|bond| bond.other).collect();
    for mourner in mourners {
        if !heroes[mourner].is_living() {
            continue;
        }
        let Some(kind) = heroes[mourner].bond_to(dead).map(|bond| bond.kind) else {
            panic!(
                "[keifu_x_inheritance_r2] {} holds a bond to {} that {} does not hold back\n  likely cause: a \
                 bond written on one side only\n  fix: write bonds with bonds::form or change",
                heroes[dead].name, heroes[mourner].name, heroes[mourner].name
            );
        };
        if kind == BondKind::Rival {
            lines.push(fmt(&words[W::GriefRival], &[&heroes[mourner].name]));
            continue;
        }
        let grief = grief_for(&heroes[mourner], kind);
        if grief <= 0 {
            continue;
        }
        let kinship = kinship_telling(content, kind, &heroes[dead]).to_owned();
        if !can_dread(&heroes[mourner]) {
            bearers.push((mourner, kinship));
            continue;
        }
        // SPEC-GAPS KG-10: the grief line first, then whatever breaking writes.
        let hero = &heroes[mourner];
        let after = (hero.fear.dread + grief).min(DREAD_LIMIT);
        lines.push(fmt(
            &words[W::GriefDread],
            &[
                &hero.name,
                &content.lore.pronouns[hero.pronoun.index()].possessive,
                &kinship,
                &after.to_string(),
                &DREAD_LIMIT.to_string(),
            ],
        ));
        lines.extend(add_dread(
            content,
            heroes,
            mourner,
            grief,
            Occasion::Grief(dead),
            year,
        ));
    }
    match bearers.as_slice() {
        [] => {}
        [(one, kinship)] => {
            let hero = &heroes[*one];
            lines.push(fmt(
                &words[W::GriefBorneOne],
                &[
                    &hero.name,
                    &content.lore.pronouns[hero.pronoun.index()].possessive,
                    kinship,
                ],
            ));
        }
        many => {
            let names: Vec<&str> = many
                .iter()
                .map(|(id, _)| heroes[*id].name.as_str())
                .collect();
            let shared = many.iter().all(|(_, kinship)| *kinship == many[0].1);
            let whom = if shared {
                fmt(&words[W::GriefBorneSharedKinship], &[&many[0].1])
            } else {
                heroes[dead].name.clone()
            };
            lines.push(fmt(
                &words[W::GriefBorneMany],
                &[&name_list(content, &names), &whom],
            ));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bonds::form;
    use crate::hero::{DeedKind, Fate};
    use crate::testkit::{founded, id};

    #[test]
    fn grief_is_two_for_kin_and_spouses_one_for_friends_and_teachers_and_none_for_the_rest() {
        let (_, heroes) = founded();
        let wren = &heroes[id(&heroes, "Wren")];
        let by_kind: Vec<i32> = BondKind::ALL.iter().map(|k| grief_for(wren, *k)).collect();
        // COMPANION, FRIEND, RIVAL, SPOUSE, MENTOR, STUDENT, PARENT, CHILD.
        assert_eq!(by_kind, [0, 1, 0, 2, 1, 1, 2, 2]);
    }

    #[test]
    fn those_who_will_outlive_their_loves_grieve_two_deeper_but_only_where_grief_applies() {
        let (_, heroes) = founded();
        let odo = &heroes[id(&heroes, "Odo")];
        let by_kind: Vec<i32> = BondKind::ALL.iter().map(|k| grief_for(odo, *k)).collect();
        assert_eq!(by_kind, [0, 3, 0, 4, 3, 3, 4, 4]);
    }

    #[test]
    fn when_garrick_dies_maren_grieves_her_father_and_odo_his_friend_in_bond_order() {
        let (content, mut heroes) = founded();
        let (garrick, maren, odo) = (
            id(&heroes, "Garrick"),
            id(&heroes, "Maren"),
            id(&heroes, "Odo"),
        );
        heroes[garrick].fate = Fate::Dead;
        let lines = grieve(&content, &mut heroes, garrick, 1);
        // Garrick's bonds: Elsbeth (dead), Maren, Odo. Maren 1 + 2; Odo 0 + 1 + 2 (outlive).
        assert_eq!(
            lines,
            [
                "Maren grieves for her father. Dread 3 of 5.",
                "Odo grieves for his friend. Dread 3 of 5."
            ]
        );
        assert_eq!(heroes[maren].fear.dread, 3);
        assert_eq!(heroes[odo].fear.dread, 3);
        assert!(heroes[garrick].grieved);
        assert!(
            grieve(&content, &mut heroes, garrick, 1).is_empty(),
            "grief comes once"
        );
        assert_eq!(heroes[maren].fear.dread, 3);
    }

    #[test]
    fn grief_that_reaches_five_breaks_the_mourner_with_a_scar_naming_the_dead() {
        let (content, mut heroes) = founded();
        let (garrick, maren) = (id(&heroes, "Garrick"), id(&heroes, "Maren"));
        heroes[maren].fear.dread = 3;
        heroes[maren].fate = Fate::Dead;
        let lines = grieve(&content, &mut heroes, maren, 4);
        // Maren's bonds: Garrick, Elsbeth (dead), Pip (a child, who can dread).
        assert_eq!(lines[0], "Garrick grieves for his daughter. Dread 4 of 5.");
        assert_eq!(lines[1], "Pip grieves for his mother. Dread 2 of 5.");
        assert_eq!(lines.len(), 2);
        heroes[garrick].fear.dread = 3;
        let mut again = heroes.clone();
        again[maren].grieved = false;
        let lines = grieve(&content, &mut again, maren, 4);
        assert_eq!(lines[0], "Garrick grieves for his daughter. Dread 5 of 5.");
        assert!(lines[1].starts_with("Something in Garrick gave way."));
        assert!(again[garrick].fear.broken);
        assert_eq!(
            again[garrick].scars.last().map(String::as_str),
            Some("Broken by grief for Maren")
        );
        assert_eq!(
            again[garrick].deeds.last().map(|d| (d.kind, d.other)),
            Some((DeedKind::Broken, Some(maren)))
        );
    }

    #[test]
    fn a_rival_has_no_one_left_to_be_better_than_and_takes_no_dread() {
        let (content, mut heroes) = founded();
        let (brannoc, ysolde) = (id(&heroes, "Brannoc"), id(&heroes, "Ysolde"));
        heroes[brannoc].fate = Fate::Dead;
        let lines = grieve(&content, &mut heroes, brannoc, 2);
        // Brannoc's bonds: Aud (dead), Wren (child), Ysolde (rival).
        assert_eq!(
            lines,
            [
                "Wren grieves for her father. Dread 2 of 5.",
                "Ysolde has no one left to be better than."
            ]
        );
        assert_eq!(heroes[ysolde].fear.dread, 0);
    }

    #[test]
    fn companions_grieve_nothing() {
        let (content, mut heroes) = founded();
        let (odo, ysolde) = (id(&heroes, "Odo"), id(&heroes, "Ysolde"));
        form(&mut heroes, odo, ysolde, BondKind::Companion, 1);
        heroes[ysolde].fate = Fate::Dead;
        let lines = grieve(&content, &mut heroes, ysolde, 2);
        assert!(lines.iter().all(|line| !line.starts_with("Odo")));
        assert_eq!(heroes[odo].fear.dread, 0);
    }

    #[test]
    fn one_who_cannot_dread_bears_it_alone_and_several_bear_it_together() {
        let (content, mut heroes) = founded();
        let (garrick, maren, odo) = (
            id(&heroes, "Garrick"),
            id(&heroes, "Maren"),
            id(&heroes, "Odo"),
        );
        heroes[garrick].fate = Fate::Dead;
        heroes[maren].settled = true;
        let mut one = heroes.clone();
        let lines = grieve(&content, &mut one, garrick, 1);
        assert_eq!(
            lines,
            [
                "Odo grieves for his friend. Dread 3 of 5.",
                "Maren grieves for her father, and bears it."
            ]
        );
        assert_eq!(one[maren].fear.dread, 1);
        heroes[odo].fear.conquered = true;
        let lines = grieve(&content, &mut heroes, garrick, 1);
        // Different kinships (father, friend): the dead's name.
        assert_eq!(lines, ["Maren and Odo grieve for Garrick, and bear it."]);
    }

    #[test]
    fn bearers_who_share_a_kinship_are_told_it_together() {
        let (content, mut heroes) = founded();
        let (brannoc, wren, pip) = (
            id(&heroes, "Brannoc"),
            id(&heroes, "Wren"),
            id(&heroes, "Pip"),
        );
        // Give Pip Brannoc as a father too, mirrored, so Wren and Pip share "father".
        form(&mut heroes, pip, brannoc, BondKind::Parent, -9);
        heroes[brannoc].fate = Fate::Dead;
        heroes[wren].fear.broken = true;
        heroes[pip].fear.broken = true;
        let lines = grieve(&content, &mut heroes, brannoc, 1);
        // The rival speaks in bond order; the bearers are told together at the end.
        assert_eq!(
            lines,
            [
                "Ysolde has no one left to be better than.",
                "Wren and Pip grieve for their father, and bear it."
            ]
        );
    }
}
