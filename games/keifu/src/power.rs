//! A party's power on a quest (SPEC §6), and the quest card's fear line (§5.4) —
//! the two readings W2's oracle asks of the bonds and fears.
//!
//! W4 owns the quest model, the line-by-line breakdown and the forecast; it
//! extends these functions rather than writing a second sum, so the card, the
//! sheet and the roll keep reading one number.

use crate::blessing::blessing_power;
use crate::bonds::steadying_companion;
use crate::constants::{PATRON_POWER, WOUND_PENALTY, bond_power, fear_penalty};
use crate::content::{Content, QuestTemplate};
use crate::destiny::door_power;
use crate::fear::{fear_power, fears};
use crate::hero::{Hero, HeroId};
use crate::ids::{Aptitude, Place, Tag};
use crate::text::fmt;
use crate::words::W;

/// What a power sum needs to know about a quest.
#[derive(Clone, Copy, Debug)]
pub struct QuestFacts<'q> {
    /// What it needs.
    pub aptitude: Aptitude,
    /// Where it is.
    pub place: Place,
    /// What it carries.
    pub tags: &'q [Tag],
    /// Whether it is one of the Door's locks.
    pub door_lock: bool,
}

impl<'q> QuestFacts<'q> {
    /// An ordinary quest made from `template`.
    pub fn of(template: &'q QuestTemplate) -> Self {
        Self {
            aptitude: template.aptitude,
            place: template.place,
            tags: &template.tags,
            door_lock: false,
        }
    }
}

/// One member's contribution (SPEC §6 lines 1-7, then the floor): effective
/// aptitude, heirloom, fear or conquered fear, wound, the Door's promise, every
/// blessing that applies; never below 0.
pub fn member_power(hero: &Hero, quest: QuestFacts<'_>) -> i32 {
    let mut sum = hero.effective(quest.aptitude);
    if let Some(heirloom) = &hero.heirloom
        && heirloom.aptitude == quest.aptitude
    {
        sum += heirloom.bonus;
    }
    sum += fear_power(hero, quest.tags);
    if hero.wounded {
        sum -= WOUND_PENALTY;
    }
    sum += door_power(hero, quest.door_lock);
    sum += blessing_power(hero, quest.place, quest.tags);
    sum.max(0)
}

/// The bonds inside a party: for each pair (i < j in seat order) the power of
/// their bond, companions adding nothing. Not floored.
pub fn bonds_power(heroes: &[Hero], party: &[HeroId]) -> i32 {
    let mut sum = 0;
    for (i, &a) in party.iter().enumerate() {
        for &b in &party[i + 1..] {
            if let Some(bond) = heroes[a].bond_to(b) {
                sum += bond_power(bond.kind);
            }
        }
    }
    sum
}

/// A party's power (SPEC §6): every member's contribution, the bonds between
/// them, and +1 per patron when anyone is seated.
pub fn party_power(heroes: &[Hero], party: &[HeroId], quest: QuestFacts<'_>, patrons: i32) -> i32 {
    if party.is_empty() {
        return 0;
    }
    let members: i32 = party.iter().map(|&m| member_power(&heroes[m], quest)).sum();
    members + bonds_power(heroes, party) + patrons * PATRON_POWER
}

/// The quest card's "you bring N" (`ui.quest_card.you_bring`).
pub fn you_bring(content: &Content, power: i32) -> String {
    fmt(&content.words[W::QuestCardYouBring], &[&power.to_string()])
}

/// One item of the card's fear line per seated hero who fears the quest, in seat
/// order: "Maren -2", and ", steadied" when a steadying companion sits with them.
pub fn fear_items(
    content: &Content,
    heroes: &[Hero],
    party: &[HeroId],
    tags: &[Tag],
) -> Vec<String> {
    let words = &content.words;
    party
        .iter()
        .filter(|&&m| fears(&heroes[m], tags))
        .map(|&m| {
            let penalty = fear_penalty(heroes[m].fear.dread).to_string();
            let item = fmt(
                &words[W::QuestCardFearfulItem],
                &[&heroes[m].name, &penalty],
            );
            // SPEC-GAPS KG-7: the glue is ", " as MODULES.md's W2 oracle quotes it;
            // KG-8: steadied is SPEC §10.1's steadying companion.
            match steadying_companion(heroes, party, m) {
                Some(_) => format!(
                    "{item}{}{}",
                    content.lore.name_list_separator,
                    &words[W::QuestCardSteadied]
                ),
                None => item,
            }
        })
        .collect()
}

/// The card's fear line, "Fear: Maren -2, steadied, ...", or `None` when nobody
/// seated fears the quest.
pub fn fear_line(
    content: &Content,
    heroes: &[Hero],
    party: &[HeroId],
    tags: &[Tag],
) -> Option<String> {
    let items = fear_items(content, heroes, party, tags);
    if items.is_empty() {
        return None;
    }
    Some(fmt(
        &content.words[W::QuestCardFear],
        &[&items.join(&content.lore.name_list_separator)],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bonds::form;
    use crate::ids::BondKind;
    use crate::testkit::{founded, id};

    fn bell(content: &Content) -> &QuestTemplate {
        content
            .quest_templates
            .iter()
            .find(|t| t.title == "The bell under the tide")
            .expect("the opening Coast quest")
    }

    #[test]
    fn maren_and_garrick_on_the_bell_bring_four_and_are_both_steadied() {
        let (content, heroes) = founded();
        let (maren, garrick) = (id(&heroes, "Maren"), id(&heroes, "Garrick"));
        let quest = QuestFacts::of(bell(&content));
        // MODULES.md W2: "you bring 4" (1 + 1 + 2 for parent and child).
        assert_eq!(member_power(&heroes[maren], quest), 1);
        assert_eq!(member_power(&heroes[garrick], quest), 1);
        assert_eq!(bonds_power(&heroes, &[maren, garrick]), 2);
        assert_eq!(
            you_bring(&content, party_power(&heroes, &[maren, garrick], quest, 0)),
            "you bring 4"
        );
        assert_eq!(
            fear_items(&content, &heroes, &[maren, garrick], quest.tags),
            ["Maren -2, steadied", "Garrick -3, steadied"]
        );
        assert_eq!(
            fear_items(&content, &heroes, &[garrick, maren], quest.tags),
            ["Garrick -3, steadied", "Maren -2, steadied"]
        );
    }

    #[test]
    fn a_fearful_hero_alone_or_beside_a_stranger_is_not_steadied() {
        let (content, heroes) = founded();
        let (garrick, ysolde) = (id(&heroes, "Garrick"), id(&heroes, "Ysolde"));
        let tags = [Tag::Water];
        assert_eq!(
            fear_line(&content, &heroes, &[garrick], &tags).as_deref(),
            Some("Fear: Garrick -3")
        );
        assert_eq!(
            fear_line(&content, &heroes, &[ysolde, garrick], &tags).as_deref(),
            Some("Fear: Garrick -3")
        );
        assert_eq!(fear_line(&content, &heroes, &[ysolde], &tags), None);
        let odo = id(&heroes, "Odo");
        assert_eq!(
            fear_line(&content, &heroes, &[odo, garrick], &tags).as_deref(),
            Some("Fear: Garrick -3, steadied")
        );
    }

    #[test]
    fn a_conquered_fear_is_not_on_the_fear_line_and_adds_two() {
        let (content, mut heroes) = founded();
        let garrick = id(&heroes, "Garrick");
        heroes[garrick].fear.conquered = true;
        let tags = [Tag::Water];
        assert_eq!(fear_line(&content, &heroes, &[garrick], &tags), None);
        let quest = QuestFacts {
            place: Place::Deepwood,
            aptitude: Aptitude::Spirit,
            tags: &tags,
            door_lock: false,
        };
        assert_eq!(member_power(&heroes[garrick], quest), 6);
    }

    #[test]
    fn a_members_contribution_never_falls_below_nothing() {
        let (_, mut heroes) = founded();
        let pip = id(&heroes, "Pip");
        heroes[pip].age = 12;
        heroes[pip].wounded = true;
        heroes[pip].fear.dread = 5;
        let quest = QuestFacts {
            place: Place::Deepwood,
            aptitude: Aptitude::Might,
            tags: &[Tag::Dark],
            door_lock: false,
        };
        assert_eq!(member_power(&heroes[pip], quest), 0);
    }

    #[test]
    fn the_heirloom_the_wound_and_the_door_each_count_on_their_own_quests() {
        let (_, mut heroes) = founded();
        let (garrick, ysolde) = (id(&heroes, "Garrick"), id(&heroes, "Ysolde"));
        let might = QuestFacts {
            place: Place::Deepwood,
            aptitude: Aptitude::Might,
            tags: &[],
            door_lock: false,
        };
        let wits = QuestFacts {
            place: Place::Deepwood,
            aptitude: Aptitude::Wits,
            tags: &[],
            door_lock: false,
        };
        // Garrick: Might 7 - 2 (elder) + 1 Thornfall; Wits 4 + 1 (elder), no heirloom.
        assert_eq!(member_power(&heroes[garrick], might), 6);
        assert_eq!(member_power(&heroes[garrick], wits), 5);
        heroes[garrick].wounded = true;
        assert_eq!(member_power(&heroes[garrick], might), 4);
        let lock = QuestFacts {
            place: Place::SealedDoor,
            aptitude: Aptitude::Wits,
            tags: &[Tag::Dark, Tag::Cold],
            door_lock: true,
        };
        // Ysolde: Wits 5, fears Dark at dread 0 (-2), +5 at a lock.
        assert_eq!(member_power(&heroes[ysolde], lock), 8);
        assert_eq!(
            member_power(
                &heroes[ysolde],
                QuestFacts {
                    door_lock: false,
                    ..lock
                }
            ),
            3
        );
    }

    #[test]
    fn a_blessing_adds_its_power_where_it_applies_before_the_floor() {
        let (content, mut heroes) = founded();
        let (garrick, pip) = (id(&heroes, "Garrick"), id(&heroes, "Pip"));
        let lamps = content
            .quest_templates
            .iter()
            .find(|t| t.title == "The lamps in the Barrow")
            .expect("a Barrow quest against the Undead");
        let grave = content
            .quest_templates
            .iter()
            .find(|t| t.title == "Grave goods")
            .expect("the opening Barrow quest");
        // Garrick: Spirit 4 on the lamps (Dark, Undead); Might 7 - 2 + 1 on grave goods.
        assert_eq!(member_power(&heroes[garrick], QuestFacts::of(lamps)), 4);
        assert_eq!(member_power(&heroes[garrick], QuestFacts::of(grave)), 6);
        let rest = crate::hero::Blessing {
            title: "Garrick's rest".into(),
            scope: crate::hero::Scope::AgainstTag(Tag::Undead),
            power: 2,
        };
        heroes[garrick].blessings.push(rest.clone());
        assert_eq!(member_power(&heroes[garrick], QuestFacts::of(lamps)), 6);
        assert_eq!(
            member_power(&heroes[garrick], QuestFacts::of(grave)),
            6,
            "grave goods carries no Undead"
        );
        // A child of twelve, wounded, afraid of the dark: -1 + 1... the blessing counts before the floor.
        heroes[pip].age = 12;
        heroes[pip].wounded = true;
        heroes[pip].blessings.push(rest);
        // Pip: Spirit 2 - 1 (youth) - 2 (Dark, dread 0) - 2 (wounded) + 2 = -1, floored to 0.
        assert_eq!(member_power(&heroes[pip], QuestFacts::of(lamps)), 0);
        heroes[pip].wounded = false;
        assert_eq!(member_power(&heroes[pip], QuestFacts::of(lamps)), 1);
    }

    #[test]
    fn bonds_add_their_power_once_per_pair_rivals_subtract_and_patrons_add_one_each() {
        let (_, mut heroes) = founded();
        let (garrick, maren, odo, ysolde, brannoc) = (
            id(&heroes, "Garrick"),
            id(&heroes, "Maren"),
            id(&heroes, "Odo"),
            id(&heroes, "Ysolde"),
            id(&heroes, "Brannoc"),
        );
        assert_eq!(bonds_power(&heroes, &[garrick, maren, odo]), 3);
        assert_eq!(bonds_power(&heroes, &[brannoc, ysolde]), -1);
        form(&mut heroes, odo, ysolde, BondKind::Companion, 1);
        assert_eq!(bonds_power(&heroes, &[odo, ysolde]), 0);
        let quest = QuestFacts {
            place: Place::Deepwood,
            aptitude: Aptitude::Wits,
            tags: &[],
            door_lock: false,
        };
        let alone = party_power(&heroes, &[ysolde], quest, 0);
        assert_eq!(party_power(&heroes, &[ysolde], quest, 2), alone + 2);
        assert_eq!(
            party_power(&heroes, &[], quest, 2),
            0,
            "an empty party has no power, patrons or not"
        );
    }
}
