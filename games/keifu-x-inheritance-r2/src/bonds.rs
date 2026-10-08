//! Bonds (SPEC §12.1, §12.4): forming and changing them, mirrored, under the rank
//! rule; what a bond's kind is told as; and steadying.
//!
//! A bond lives on both heroes. "A's bond to B is PARENT" means B is A's parent,
//! and B's bond to A is the mirror (CHILD). Every function here writes both
//! sides or neither, so the two lists can never disagree.

use crate::constants::{bond_mirror, bond_power, bond_rank};
use crate::content::Content;
use crate::hero::{Bond, Hero, HeroId};
use crate::ids::BondKind;

/// Form(A, B, kind) (SPEC §12.1): nothing if A is B; a new bond on both sides if
/// they have none; otherwise the bond is replaced only by a kind of strictly
/// higher rank, keeping its place in each list.
pub fn form(heroes: &mut [Hero], a: HeroId, b: HeroId, kind: BondKind, year: i32) {
    if a == b {
        return;
    }
    match heroes[a].bond_to(b).map(|bond| bond.kind) {
        None => {
            for (from, to, kind) in [(a, b, kind), (b, a, bond_mirror(kind))] {
                heroes[from].bonds.push(Bond {
                    kind,
                    other: to,
                    since: year,
                    taught: false,
                    shared_successes: 0,
                });
            }
        }
        // SPEC-GAPS KG-9: a replacement is a change, so it sets `since` as Change does.
        Some(held) if bond_rank(kind) > bond_rank(held) => overwrite(heroes, a, b, kind, year),
        Some(_) => {}
    }
}

/// Change(A, B, kind) (SPEC §12.1): overwrite both sides regardless of rank and
/// set `since`. Every caller the spec names changes a bond that exists.
pub fn change(heroes: &mut [Hero], a: HeroId, b: HeroId, kind: BondKind, year: i32) {
    if a == b || heroes[a].bond_to(b).is_none() {
        // SPEC-GAPS KG-9: the spec never changes a bond that is not there.
        panic!(
            "[keifu_x_inheritance_r2] change() was asked to turn {} and {} into {}, and they have no bond\n  \
             likely cause: a rule called change() where SPEC §12.1 calls form()\n  fix: form \
             the bond first, or call form()",
            heroes[a].name,
            heroes[b].name,
            kind.id()
        );
    }
    overwrite(heroes, a, b, kind, year);
}

fn overwrite(heroes: &mut [Hero], a: HeroId, b: HeroId, kind: BondKind, year: i32) {
    for (from, to, kind) in [(a, b, kind), (b, a, bond_mirror(kind))] {
        if let Some(bond) = heroes[from].bonds.iter_mut().find(|bond| bond.other == to) {
            bond.kind = kind;
            bond.since = year;
        }
    }
}

/// Whether a bond of `kind` steadies fear: its power is above 0 (CONSTANTS §7).
pub fn steadies(kind: BondKind) -> bool {
    bond_power(kind) > 0
}

/// The first *living* other member of `party`, in party order, whose bond with
/// `member` steadies (SPEC §10.1 step 3). The quest card's "steadied" asks this.
pub fn steadying_companion(heroes: &[Hero], party: &[HeroId], member: HeroId) -> Option<HeroId> {
    party.iter().copied().find(|&other| {
        other != member
            && heroes[other].is_living()
            && heroes[member]
                .bond_to(other)
                .is_some_and(|bond| steadies(bond.kind))
    })
}

/// The kinship telling of `other`, as a bond of `kind` names them: "daughter",
/// "teacher", "friend" (`bonds.json` `kinship_tellings`).
pub fn kinship_telling<'c>(content: &'c Content, kind: BondKind, other: &Hero) -> &'c str {
    &content.bonds.kinship[kind.index()][other.pronoun.index()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{founded, id};

    #[test]
    fn a_new_bond_is_formed_on_both_sides_with_the_mirror_kind() {
        let (_, mut heroes) = founded();
        let (odo, pip) = (id(&heroes, "Odo"), id(&heroes, "Pip"));
        form(&mut heroes, pip, odo, BondKind::Mentor, 3);
        let pip_side = heroes[pip].bond_to(odo).map(|b| (b.kind, b.since));
        let odo_side = heroes[odo].bond_to(pip).map(|b| (b.kind, b.since));
        assert_eq!(pip_side, Some((BondKind::Mentor, 3)));
        assert_eq!(odo_side, Some((BondKind::Student, 3)));
    }

    #[test]
    fn a_bond_to_oneself_is_never_formed() {
        let (_, mut heroes) = founded();
        let odo = id(&heroes, "Odo");
        let before = heroes[odo].bonds.len();
        form(&mut heroes, odo, odo, BondKind::Friend, 1);
        assert_eq!(heroes[odo].bonds.len(), before);
    }

    #[test]
    fn a_higher_rank_replaces_a_bond_in_place_and_an_equal_or_lower_one_does_not() {
        let (_, mut heroes) = founded();
        let (garrick, odo) = (id(&heroes, "Garrick"), id(&heroes, "Odo"));
        let place = heroes[garrick].bonds.iter().position(|b| b.other == odo);
        // Friend (1) to rival (1): equal rank, kept.
        form(&mut heroes, garrick, odo, BondKind::Rival, 2);
        assert_eq!(
            heroes[garrick].bond_to(odo).map(|b| b.kind),
            Some(BondKind::Friend)
        );
        // Friend (1) to companion (0): lower, kept.
        form(&mut heroes, garrick, odo, BondKind::Companion, 2);
        assert_eq!(
            heroes[garrick].bond_to(odo).map(|b| b.kind),
            Some(BondKind::Friend)
        );
        // Friend (1) to student (2): higher, replaced where it stood, both sides.
        form(&mut heroes, garrick, odo, BondKind::Student, 2);
        assert_eq!(
            heroes[garrick].bond_to(odo).map(|b| b.kind),
            Some(BondKind::Student)
        );
        assert_eq!(
            heroes[odo].bond_to(garrick).map(|b| b.kind),
            Some(BondKind::Mentor)
        );
        assert_eq!(
            heroes[garrick].bonds.iter().position(|b| b.other == odo),
            place
        );
        assert_eq!(heroes[garrick].bond_to(odo).map(|b| b.since), Some(2));
    }

    #[test]
    fn a_mentorship_does_not_replace_a_parent_or_a_spouse() {
        let (_, mut heroes) = founded();
        let (garrick, maren) = (id(&heroes, "Garrick"), id(&heroes, "Maren"));
        form(&mut heroes, maren, garrick, BondKind::Mentor, 1);
        assert_eq!(
            heroes[maren].bond_to(garrick).map(|b| b.kind),
            Some(BondKind::Parent)
        );
        let (brannoc, aud) = (id(&heroes, "Brannoc"), id(&heroes, "Aud"));
        form(&mut heroes, brannoc, aud, BondKind::Mentor, 1);
        assert_eq!(
            heroes[brannoc].bond_to(aud).map(|b| b.kind),
            Some(BondKind::Spouse)
        );
    }

    #[test]
    fn a_change_overwrites_both_sides_whatever_the_rank_and_sets_since() {
        let (_, mut heroes) = founded();
        let (brannoc, ysolde) = (id(&heroes, "Brannoc"), id(&heroes, "Ysolde"));
        change(&mut heroes, brannoc, ysolde, BondKind::Friend, 4);
        assert_eq!(
            heroes[brannoc].bond_to(ysolde).map(|b| (b.kind, b.since)),
            Some((BondKind::Friend, 4))
        );
        assert_eq!(
            heroes[ysolde].bond_to(brannoc).map(|b| (b.kind, b.since)),
            Some((BondKind::Friend, 4))
        );
        let (garrick, maren) = (id(&heroes, "Garrick"), id(&heroes, "Maren"));
        change(&mut heroes, garrick, maren, BondKind::Rival, 5);
        assert_eq!(
            heroes[maren].bond_to(garrick).map(|b| b.kind),
            Some(BondKind::Rival)
        );
    }

    #[test]
    #[should_panic(expected = "they have no bond")]
    fn changing_a_bond_that_does_not_exist_panics() {
        let (_, mut heroes) = founded();
        let (odo, wren) = (id(&heroes, "Odo"), id(&heroes, "Wren"));
        change(&mut heroes, odo, wren, BondKind::Friend, 1);
    }

    #[test]
    fn friends_spouses_teachers_students_parents_and_children_steady_and_others_do_not() {
        let steady: Vec<bool> = BondKind::ALL.iter().map(|k| steadies(*k)).collect();
        // COMPANION, FRIEND, RIVAL, SPOUSE, MENTOR, STUDENT, PARENT, CHILD.
        assert_eq!(steady, [false, true, false, true, true, true, true, true]);
    }

    #[test]
    fn the_steadying_companion_is_the_first_living_bonded_member_in_seat_order() {
        let (_, mut heroes) = founded();
        let (garrick, maren, odo, ysolde) = (
            id(&heroes, "Garrick"),
            id(&heroes, "Maren"),
            id(&heroes, "Odo"),
            id(&heroes, "Ysolde"),
        );
        assert_eq!(
            steadying_companion(&heroes, &[ysolde, odo, maren, garrick], garrick),
            Some(odo)
        );
        assert_eq!(
            steadying_companion(&heroes, &[maren, odo, garrick], garrick),
            Some(maren)
        );
        // A rival or a stranger does not steady.
        let brannoc = id(&heroes, "Brannoc");
        assert_eq!(
            steadying_companion(&heroes, &[brannoc, ysolde], ysolde),
            None
        );
        // The dead do not steady.
        heroes[odo].fate = crate::hero::Fate::Dead;
        assert_eq!(
            steadying_companion(&heroes, &[odo, maren, garrick], garrick),
            Some(maren)
        );
    }

    #[test]
    fn a_bonds_kinship_telling_follows_the_other_heros_pronoun() {
        let (content, heroes) = founded();
        let (garrick, maren, odo) = (
            id(&heroes, "Garrick"),
            id(&heroes, "Maren"),
            id(&heroes, "Odo"),
        );
        assert_eq!(
            kinship_telling(&content, BondKind::Child, &heroes[maren]),
            "daughter"
        );
        assert_eq!(
            kinship_telling(&content, BondKind::Parent, &heroes[garrick]),
            "father"
        );
        assert_eq!(
            kinship_telling(&content, BondKind::Friend, &heroes[odo]),
            "friend"
        );
        assert_eq!(
            kinship_telling(&content, BondKind::Mentor, &heroes[odo]),
            "teacher"
        );
        assert_eq!(
            kinship_telling(&content, BondKind::Companion, &heroes[odo]),
            "friend"
        );
    }
}
