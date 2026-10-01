//! Moments and the requirement predicates over them (SPEC §9.1, §9.3).
//!
//! A moment is something a hero can witness: a resolved quest (with its place,
//! tags, outcome and party), a winter action, or the turning of the year. Whether
//! a dream's stage is met by a moment is `stage_met`, the one function witnessing
//! (`witness`) and dream calls (`calls`) both ask, so a card cannot say a quest
//! calls a dreamer that the roll would not move.
//!
//! Nothing in this build raises a moment yet: quests (W4-W6) and the hearth and
//! the turning (W7, W8) are their sources. Verify stages them.

use crate::content::Content;
use crate::dream::Dream;
use crate::dream_lore::{PlaceParam, Predicate, Requirement, TagParam};
use crate::hero::{Hero, HeroId};
use crate::ids::{BondKind, Outcome, Place, Tag, WinterAction};

/// A resolved quest, as a moment (SPEC §7.1 step 12).
#[derive(Clone, Copy, Debug)]
pub struct QuestMoment<'m> {
    /// Where it was.
    pub place: Place,
    /// What it carried.
    pub tags: &'m [Tag],
    /// How it went.
    pub outcome: Outcome,
    /// Who went, in party order. A witness in it was present.
    pub party: &'m [HeroId],
}

/// Something a hero witnesses.
#[derive(Clone, Copy, Debug)]
pub enum Moment<'m> {
    /// A resolved quest.
    Quest(QuestMoment<'m>),
    /// A winter action this hero took (SPEC §11.3).
    Winter(WinterAction),
    /// The turning of the year (SPEC §18 step 9).
    YearTurn,
}

/// Whether `stage` of `dream`, held by `hero`, is met by `moment`.
pub fn stage_met(
    content: &Content,
    heroes: &[Hero],
    hero: HeroId,
    dream: &Dream,
    stage: usize,
    moment: &Moment<'_>,
) -> bool {
    let requirement = &content.dreams[dream.kind.index()].stages[stage].requirement;
    requirement_met(heroes, hero, dream, requirement, moment)
}

fn requirement_met(
    heroes: &[Hero],
    hero: HeroId,
    dream: &Dream,
    requirement: &Requirement,
    moment: &Moment<'_>,
) -> bool {
    match requirement {
        Requirement::One(predicate) => holds(heroes, hero, dream, *predicate, moment),
        Requirement::All(parts) => parts
            .iter()
            .all(|part| requirement_met(heroes, hero, dream, part, moment)),
    }
}

/// The setup a SETUP_* parameter names. Content validation lets SETUP_* appear only
/// in a dream with a setup, and `Dream::build` refuses such a dream without one.
fn setup_of(dream: &Dream) -> crate::dream::Setup {
    match dream.setup {
        Some(setup) => setup,
        None => panic!(
            "[keifu] {} names its setup and holds none\n  likely cause: a dream built \
             without Dream::build\n  fix: build every dream with Dream::build",
            dream.kind.id()
        ),
    }
}

/// Whether the hero taught someone in `party` (the hero's bond to them has taught).
fn taught_one_of(heroes: &[Hero], hero: HeroId, party: &[HeroId]) -> bool {
    party
        .iter()
        .any(|&member| heroes[hero].bond_to(member).is_some_and(|bond| bond.taught))
}

/// One predicate of SPEC §9.1's table.
fn holds(
    heroes: &[Hero],
    hero: HeroId,
    dream: &Dream,
    predicate: Predicate,
    moment: &Moment<'_>,
) -> bool {
    let me = &heroes[hero];
    // The predicates that hold at any moment.
    match predicate {
        Predicate::RenownIsAtLeast(renown) => return me.renown >= renown,
        Predicate::IsWed => return me.bonds.iter().any(|b| b.kind == BondKind::Spouse),
        Predicate::HasAChild => return me.bonds.iter().any(|b| b.kind == BondKind::Child),
        Predicate::HasAGrownChild => {
            return me.bonds.iter().any(|b| {
                b.kind == BondKind::Child
                    && heroes[b.other].is_living()
                    && heroes[b.other].is_adult()
            });
        }
        _ => {}
    }
    match (predicate, moment) {
        (Predicate::SpentTheWinter(want), Moment::Winter(action)) => *action == want,
        (Predicate::TaughtTheYoung, Moment::Winter(action)) => {
            matches!(action, WinterAction::Teach | WinterAction::MindAChild)
        }
        (_, Moment::Quest(quest)) => {
            let present = quest.party.contains(&hero);
            let questing = quest.place != Place::SealedDoor;
            match predicate {
                Predicate::WentToPlace(place) => {
                    let place = match place {
                        PlaceParam::Fixed(place) => place,
                        PlaceParam::Setup => setup_of(dream).place,
                    };
                    present && quest.place == place
                }
                Predicate::FacedTag(tag) => {
                    let tag = match tag {
                        TagParam::Fixed(tag) => tag,
                        TagParam::Setup => setup_of(dream).tag,
                    };
                    present && quest.tags.contains(&tag)
                }
                Predicate::FaredAtLeast(outcome) => present && quest.outcome >= outcome,
                Predicate::WalkedANewRoad => {
                    present && questing && !me.roads_walked.contains(&quest.place)
                }
                Predicate::WalkedEveryRoad => {
                    present
                        && questing
                        && Place::ALL
                            .iter()
                            .filter(|place| **place != Place::SealedDoor)
                            .all(|place| *place == quest.place || me.roads_walked.contains(place))
                }
                Predicate::StoodBesideAStudent => {
                    present && taught_one_of(heroes, hero, quest.party)
                }
                Predicate::StudentFaredAlone => {
                    !present
                        && quest.outcome >= Outcome::Success
                        && taught_one_of(heroes, hero, quest.party)
                }
                _ => false,
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bonds::form;
    use crate::dream::Setup;
    use crate::testkit::{founded, id};

    fn quest<'m>(
        place: Place,
        tags: &'m [Tag],
        outcome: Outcome,
        party: &'m [HeroId],
    ) -> Moment<'m> {
        Moment::Quest(QuestMoment {
            place,
            tags,
            outcome,
            party,
        })
    }

    /// The truth of one predicate for `hero` (dream: Maren's, which has a setup).
    fn truth(heroes: &[Hero], hero: HeroId, predicate: Predicate, moment: &Moment<'_>) -> bool {
        let dream = heroes[id(heroes, "Maren")]
            .dream
            .clone()
            .expect("Maren dreams");
        holds(heroes, hero, &dream, predicate, moment)
    }

    #[test]
    fn place_tag_and_outcome_hold_only_for_one_present() {
        let (_, heroes) = founded();
        let (garrick, odo) = (id(&heroes, "Garrick"), id(&heroes, "Odo"));
        let party = [odo, garrick];
        let at = |outcome| quest(Place::Barrow, &[Tag::Dark, Tag::Undead], outcome, &party);
        let barrow = Predicate::WentToPlace(PlaceParam::Fixed(Place::Barrow));
        let undead = Predicate::FacedTag(TagParam::Fixed(Tag::Undead));
        let success = Predicate::FaredAtLeast(Outcome::Success);
        for p in [barrow, undead, success] {
            assert!(truth(&heroes, garrick, p, &at(Outcome::Success)), "{p:?}");
            assert!(
                !truth(&heroes, id(&heroes, "Maren"), p, &at(Outcome::Success)),
                "absent: {p:?}"
            );
        }
        assert!(!truth(
            &heroes,
            garrick,
            Predicate::WentToPlace(PlaceParam::Fixed(Place::Deepwood)),
            &at(Outcome::Triumph)
        ));
        assert!(!truth(
            &heroes,
            garrick,
            Predicate::FacedTag(TagParam::Fixed(Tag::Water)),
            &at(Outcome::Triumph)
        ));
        let fared: Vec<bool> = Outcome::ALL
            .iter()
            .map(|o| truth(&heroes, garrick, success, &at(*o)))
            .collect();
        assert_eq!(fared, [false, false, true, true]);
        let triumph = Predicate::FaredAtLeast(Outcome::Triumph);
        assert!(!truth(&heroes, garrick, triumph, &at(Outcome::Success)));
        // A quest predicate never holds at a winter or a turning.
        for moment in [Moment::Winter(WinterAction::Train), Moment::YearTurn] {
            assert!(!truth(&heroes, garrick, barrow, &moment));
            assert!(!truth(&heroes, garrick, success, &moment));
        }
    }

    #[test]
    fn the_setup_place_and_tag_are_the_avenged_ones() {
        let (content, heroes) = founded();
        let maren = id(&heroes, "Maren");
        let party = [maren];
        let mut dream = heroes[maren].dream.clone().expect("Maren dreams");
        let coast = quest(Place::DrownedCoast, &[Tag::Water], Outcome::Success, &party);
        let pass = quest(Place::HighPass, &[Tag::Water], Outcome::Success, &party);
        assert!(stage_met(&content, &heroes, maren, &dream, 1, &coast));
        assert!(!stage_met(&content, &heroes, maren, &dream, 1, &pass));
        assert!(stage_met(&content, &heroes, maren, &dream, 2, &coast));
        dream.setup = Some(Setup {
            place: Place::DrownedCoast,
            tag: Tag::Beasts,
            lost: None,
        });
        assert!(
            !stage_met(&content, &heroes, maren, &dream, 2, &coast),
            "the setup tag, not Water"
        );
        dream.setup = Some(Setup {
            place: Place::HighPass,
            tag: Tag::Water,
            lost: None,
        });
        assert!(
            stage_met(&content, &heroes, maren, &dream, 1, &pass),
            "the setup place, not the Coast"
        );
        assert!(!stage_met(&content, &heroes, maren, &dream, 1, &coast));
        assert!(
            !stage_met(&content, &heroes, maren, &dream, 2, &coast),
            "the setup tag, not Water"
        );
    }

    #[test]
    fn a_new_road_is_one_not_walked_before_and_never_the_door() {
        let (_, heroes) = founded();
        let ysolde = id(&heroes, "Ysolde");
        let party = [ysolde];
        let new = Predicate::WalkedANewRoad;
        assert!(truth(
            &heroes,
            ysolde,
            new,
            &quest(Place::Barrow, &[], Outcome::Disaster, &party)
        ));
        assert!(!truth(
            &heroes,
            ysolde,
            new,
            &quest(Place::Deepwood, &[], Outcome::Triumph, &party)
        ));
        assert!(!truth(
            &heroes,
            ysolde,
            new,
            &quest(Place::SealedDoor, &[], Outcome::Triumph, &party)
        ));
        assert!(!truth(
            &heroes,
            ysolde,
            new,
            &quest(Place::Barrow, &[], Outcome::Triumph, &[])
        ));
    }

    #[test]
    fn every_road_counts_the_one_being_walked_and_needs_all_six() {
        let (_, mut heroes) = founded();
        let garrick = id(&heroes, "Garrick");
        let party = [garrick];
        let every = Predicate::WalkedEveryRoad;
        // Garrick has walked the Barrow, the Pass, the Deepwood and the Court.
        assert!(!truth(
            &heroes,
            garrick,
            every,
            &quest(Place::Emberfall, &[], Outcome::Success, &party)
        ));
        heroes[garrick].roads_walked.push(Place::DrownedCoast);
        assert!(truth(
            &heroes,
            garrick,
            every,
            &quest(Place::Emberfall, &[], Outcome::Disaster, &party)
        ));
        assert!(!truth(
            &heroes,
            garrick,
            every,
            &quest(Place::SealedDoor, &[], Outcome::Triumph, &party)
        ));
        assert!(!truth(
            &heroes,
            garrick,
            every,
            &quest(Place::Emberfall, &[], Outcome::Success, &[])
        ));
        heroes[garrick].roads_walked.push(Place::Emberfall);
        assert!(truth(
            &heroes,
            garrick,
            every,
            &quest(Place::Barrow, &[], Outcome::Success, &party)
        ));
    }

    #[test]
    fn winter_predicates_hold_for_their_actions_only() {
        let (_, heroes) = founded();
        let odo = id(&heroes, "Odo");
        let train = Predicate::SpentTheWinter(WinterAction::Train);
        let young = Predicate::TaughtTheYoung;
        let holds_for = |p| -> Vec<bool> {
            WinterAction::ALL
                .iter()
                .map(|a| truth(&heroes, odo, p, &Moment::Winter(*a)))
                .collect()
        };
        assert_eq!(holds_for(train), [false, true, false, false, false, false]);
        assert_eq!(holds_for(young), [false, false, true, false, true, false]);
        assert!(!truth(&heroes, odo, young, &Moment::YearTurn));
        let party = [odo];
        assert!(!truth(
            &heroes,
            odo,
            young,
            &quest(Place::Barrow, &[], Outcome::Triumph, &party)
        ));
    }

    #[test]
    fn a_student_counts_beside_or_alone_only_if_the_hero_taught_them() {
        let (_, mut heroes) = founded();
        let (odo, wren, pip) = (id(&heroes, "Odo"), id(&heroes, "Wren"), id(&heroes, "Pip"));
        form(&mut heroes, wren, odo, crate::ids::BondKind::Mentor, 1);
        let beside = Predicate::StoodBesideAStudent;
        let alone = Predicate::StudentFaredAlone;
        let together = [wren, odo];
        let apart = [pip, wren];
        let at = |outcome, party| quest(Place::Barrow, &[], outcome, party);
        assert!(
            !truth(&heroes, odo, beside, &at(Outcome::Triumph, &together)),
            "untaught"
        );
        assert!(
            !truth(&heroes, odo, alone, &at(Outcome::Triumph, &apart)),
            "untaught"
        );
        if let Some(bond) = heroes[odo].bonds.iter_mut().find(|b| b.other == wren) {
            bond.taught = true;
        }
        assert!(truth(
            &heroes,
            odo,
            beside,
            &at(Outcome::Disaster, &together)
        ));
        assert!(
            !truth(&heroes, odo, beside, &at(Outcome::Triumph, &apart)),
            "not present"
        );
        assert!(truth(&heroes, odo, alone, &at(Outcome::Success, &apart)));
        assert!(!truth(&heroes, odo, alone, &at(Outcome::Setback, &apart)));
        assert!(
            !truth(&heroes, odo, alone, &at(Outcome::Triumph, &together)),
            "present"
        );
        assert!(
            !truth(&heroes, odo, alone, &at(Outcome::Triumph, &[pip])),
            "no student there"
        );
    }

    #[test]
    fn renown_wedlock_and_children_hold_at_every_kind_of_moment() {
        let (_, mut heroes) = founded();
        let (garrick, odo, brannoc, wren) = (
            id(&heroes, "Garrick"),
            id(&heroes, "Odo"),
            id(&heroes, "Brannoc"),
            id(&heroes, "Wren"),
        );
        let party = [odo];
        let moments = [
            quest(Place::Barrow, &[], Outcome::Disaster, &party),
            Moment::Winter(WinterAction::Rest),
            Moment::YearTurn,
        ];
        for moment in &moments {
            // Garrick: renown 6, widowed, a living adult child. Odo: renown 3, unwed, childless.
            assert!(truth(
                &heroes,
                garrick,
                Predicate::RenownIsAtLeast(4),
                moment
            ));
            assert!(!truth(&heroes, odo, Predicate::RenownIsAtLeast(4), moment));
            assert!(truth(&heroes, garrick, Predicate::IsWed, moment));
            assert!(!truth(&heroes, odo, Predicate::IsWed, moment));
            assert!(truth(&heroes, garrick, Predicate::HasAChild, moment));
            assert!(!truth(&heroes, odo, Predicate::HasAChild, moment));
            assert!(truth(&heroes, garrick, Predicate::HasAGrownChild, moment));
            // Brannoc's only child is Wren, eight.
            assert!(truth(&heroes, brannoc, Predicate::HasAChild, moment));
            assert!(!truth(&heroes, brannoc, Predicate::HasAGrownChild, moment));
        }
        heroes[odo].renown = 4;
        assert!(truth(
            &heroes,
            odo,
            Predicate::RenownIsAtLeast(4),
            &Moment::YearTurn
        ));
        heroes[wren].age = 12;
        assert!(truth(
            &heroes,
            brannoc,
            Predicate::HasAGrownChild,
            &Moment::YearTurn
        ));
        heroes[wren].fate = crate::hero::Fate::Dead;
        assert!(!truth(
            &heroes,
            brannoc,
            Predicate::HasAGrownChild,
            &Moment::YearTurn
        ));
        assert!(
            truth(&heroes, brannoc, Predicate::HasAChild, &Moment::YearTurn),
            "living or dead"
        );
    }
}
