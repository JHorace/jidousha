//! The quest sheet's power breakdown (SPEC §6, line by line), extending `power.rs`.
//!
//! The sum stays the one number: `party_lines` asserts that its lines add up to
//! `power::party_power` for the same party, so the sheet cannot print a breakdown
//! the card's "you bring" and the roll do not also read.

use crate::blessing::applies;
use crate::constants::{PATRON_POWER, WOUND_PENALTY, bond_power};
use crate::content::Content;
use crate::destiny::door_power;
use crate::fear::fear_power;
use crate::hero::{Hero, HeroId};
use crate::power::{QuestFacts, party_power};
use crate::text::fmt;
use crate::words::W;

/// One line of the quest sheet's power breakdown (SPEC §6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PowerLine {
    /// "Garrick, Might 5", "  carries Thornfall", "Garrick and Maren, parent and child".
    pub text: String,
    /// What it adds.
    pub value: i32,
    /// A member's first line, which carries its value in its text.
    pub head: bool,
}

/// One member's lines, SPEC §6 lines 1-7 in order with each line of 0 omitted,
/// then the floor's line when they sum below nothing.
///
/// SPEC-GAPS KG-26: the arguments of "carries %", "fears %", "has conquered %" and
/// "blessed: %" are the heirloom's name, the tag's noun and the blessing's title.
pub fn member_lines(content: &Content, hero: &Hero, quest: QuestFacts<'_>) -> Vec<PowerLine> {
    let words = &content.words;
    let mut out = Vec::new();
    let mut add = |text: String, value: i32, head: bool| {
        if value != 0 {
            out.push(PowerLine { text, value, head });
        }
    };
    let aptitude = hero.effective(quest.aptitude);
    add(
        fmt(
            &words[W::PowerLineMember],
            &[
                &hero.name,
                &content.lore.aptitudes[quest.aptitude.index()],
                &aptitude.to_string(),
            ],
        ),
        aptitude,
        true,
    );
    if let Some(heirloom) = &hero.heirloom
        && heirloom.aptitude == quest.aptitude
    {
        add(
            fmt(&words[W::PowerLineCarries], &[&heirloom.name]),
            heirloom.bonus,
            false,
        );
    }
    let noun = &content.lore.tags[hero.fear.tag.index()].noun;
    let fear = fear_power(hero, quest.tags);
    let which = if hero.fear.conquered {
        W::PowerLineConquered
    } else {
        W::PowerLineFears
    };
    add(fmt(&words[which], &[noun]), fear, false);
    if hero.wounded {
        add(words[W::PowerLineWounded].to_owned(), -WOUND_PENALTY, false);
    }
    // Variant (genes.rs): the blood's traits, one line.
    add(
        format!("  blood: {}", crate::genes::traits_word(&hero.genes)),
        crate::genes::trait_power(hero, quest),
        false,
    );
    add(
        words[W::PowerLineDoor].to_owned(),
        door_power(hero, quest.door_lock),
        false,
    );
    for blessing in &hero.blessings {
        if applies(blessing, quest.place, quest.tags) {
            add(
                fmt(&words[W::PowerLineBlessed], &[&blessing.title]),
                blessing.power,
                false,
            );
        }
    }
    let sum: i32 = out.iter().map(|line| line.value).sum();
    if sum < 0 {
        out.push(PowerLine {
            text: words[W::PowerLineFloor].to_owned(),
            value: -sum,
            head: false,
        });
    }
    out
}

/// A party's whole breakdown (SPEC §6): each member's lines in seat order, then a
/// line per bonded pair (i < j, companions omitted as adding 0), then the patrons.
///
/// INVARIANT: the lines add up to `party_power` for the same party — asserted, so a
/// breakdown and the sum cannot drift apart.
pub fn party_lines(
    content: &Content,
    heroes: &[Hero],
    party: &[HeroId],
    quest: QuestFacts<'_>,
    patrons: i32,
) -> Vec<PowerLine> {
    if party.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<PowerLine> = party
        .iter()
        .flat_map(|&m| member_lines(content, &heroes[m], quest))
        .collect();
    for (i, &a) in party.iter().enumerate() {
        for &b in &party[i + 1..] {
            let Some(bond) = heroes[a].bond_to(b) else {
                continue;
            };
            let value = bond_power(bond.kind);
            if value != 0 {
                out.push(PowerLine {
                    text: fmt(
                        &content.words[W::PowerLineBond],
                        &[
                            &heroes[a].name,
                            &heroes[b].name,
                            &content.bonds.pairs[bond.kind.index()],
                        ],
                    ),
                    value,
                    head: false,
                });
            }
        }
    }
    if patrons > 0 {
        out.push(PowerLine {
            text: content.words[W::PowerLinePatron].to_owned(),
            value: patrons * PATRON_POWER,
            head: false,
        });
    }
    let total: i32 = out.iter().map(|line| line.value).sum();
    let power = party_power(heroes, party, quest, patrons);
    assert_eq!(
        total,
        power,
        "[keifu_x_inheritance_r3v1] the power breakdown adds up to {total} and the sum is {power}\n  \
         likely cause: a SPEC §6 line was changed in member_power or party_power and not \
         in member_lines or party_lines (or the reverse)\n  fix: make the two agree; \
         lines: {:?}",
        out.iter()
            .map(|l| format!("{} {}", l.text, l.value))
            .collect::<Vec<_>>()
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bonds::form;
    use crate::hero::{Blessing, Scope};
    use crate::ids::{Aptitude, BondKind, Place, Tag};
    use crate::testkit::{founded, id};

    fn lines(
        content: &Content,
        heroes: &[Hero],
        party: &[HeroId],
        q: QuestFacts<'_>,
        patrons: i32,
    ) -> Vec<(String, i32)> {
        party_lines(content, heroes, party, q, patrons)
            .into_iter()
            .map(|l| (l.text, l.value))
            .collect()
    }

    #[test]
    fn garrick_and_brannoc_on_grave_goods_read_might_thornfall_and_might() {
        let (content, heroes) = founded();
        let grave = content
            .quest_templates
            .iter()
            .find(|t| t.title == "Grave goods")
            .expect("the opening Barrow quest");
        let party = [id(&heroes, "Garrick"), id(&heroes, "Brannoc")];
        assert_eq!(
            lines(&content, &heroes, &party, QuestFacts::of(grave), 0),
            [
                ("Garrick, Might 5".to_owned(), 5),
                ("  carries Thornfall".to_owned(), 1),
                ("Brannoc, Might 6".to_owned(), 6),
            ]
        );
    }

    #[test]
    fn every_line_in_order_the_floor_bonds_and_patrons() {
        let (content, mut heroes) = founded();
        let (garrick, maren, ysolde, brannoc) = (
            id(&heroes, "Garrick"),
            id(&heroes, "Maren"),
            id(&heroes, "Ysolde"),
            id(&heroes, "Brannoc"),
        );
        let tags = [Tag::Dark, Tag::Water];
        let lock = QuestFacts {
            aptitude: Aptitude::Might,
            place: Place::SealedDoor,
            tags: &tags,
            door_lock: true,
        };
        heroes[garrick].wounded = true;
        heroes[garrick].blessings.push(Blessing {
            title: "Patience".into(),
            scope: Scope::Everywhere,
            power: 1,
        });
        heroes[ysolde].destiny.kind = crate::ids::Destiny::OpenTheSealedDoor;
        heroes[maren].fear.conquered = true;
        let got = lines(
            &content,
            &heroes,
            &[garrick, maren, ysolde, brannoc],
            lock,
            2,
        );
        let want: Vec<(String, i32)> = [
            ("Garrick, Might 5", 5),
            ("  carries Thornfall", 1),
            ("  fears deep water", -3),
            ("  is wounded", -2),
            ("  blessed: Patience", 1),
            ("Maren, Might 4", 4),
            ("  has conquered deep water", 2),
            ("Ysolde, Might 2", 2),
            ("  fears the dark", -2),
            ("  was promised to the Door", 5),
            ("Brannoc, Might 6", 6),
            ("Garrick and Maren, parent and child", 2),
            ("Ysolde and Brannoc, rivals", -1),
            ("A patron at Court", 2),
        ]
        .into_iter()
        .map(|(t, v)| (t.to_owned(), v))
        .collect();
        assert_eq!(got, want);
        // A member below nothing is raised to exactly nothing by one line.
        heroes[ysolde].destiny.kind = crate::ids::Destiny::Unspoken;
        heroes[ysolde].wounded = true;
        let alone = lines(
            &content,
            &heroes,
            &[ysolde],
            QuestFacts {
                door_lock: false,
                ..lock
            },
            0,
        );
        assert_eq!(
            alone,
            [
                ("Ysolde, Might 2".to_owned(), 2),
                ("  fears the dark".to_owned(), -2),
                ("  is wounded".to_owned(), -2),
                ("  can do no worse than nothing".to_owned(), 2),
            ]
        );
        // A companion pair adds nothing and has no line.
        form(&mut heroes, garrick, brannoc, BondKind::Companion, 1);
        let pair = lines(
            &content,
            &heroes,
            &[garrick, brannoc],
            QuestFacts {
                door_lock: false,
                ..lock
            },
            0,
        );
        assert!(pair.iter().all(|(t, _)| !t.contains(" and ")), "{pair:?}");
    }

    #[test]
    fn a_member_with_nothing_in_the_aptitude_has_no_first_line_and_nobody_has_no_lines() {
        let (content, mut heroes) = founded();
        let odo = id(&heroes, "Odo");
        // Odo is a veteran: Might 0 - 1 is floored at 0 before any line is read.
        heroes[odo].aptitudes = [0, 0, 0];
        let q = QuestFacts {
            aptitude: Aptitude::Might,
            place: Place::Deepwood,
            tags: &[],
            door_lock: false,
        };
        assert!(lines(&content, &heroes, &[odo], q, 0).is_empty());
        assert!(lines(&content, &heroes, &[], q, 3).is_empty());
    }
}
