//! The Door's prologue (SPEC §16.2 step 2, `lineage/door.jai:184-299`): who went down, and
//! what each carried.
//!
//! `lines.door.prologue` names the party; one `lines.door.brought` per member, in party
//! order — age and descent, then the fragments that apply, appended in the stated order:
//! the Door's promise, the heirloom, a fear of the Door's dark or cold, each blessing that
//! counts everywhere, a wound; one `lines.door.went_beside` per member with a shown bond to
//! a later member; and `lines.door.summary`, the party's power at each lock, read off the
//! same outlook the card shows (`door::outlook`).

use crate::bonds::kinship_telling;
use crate::constants::{
    CONQUERED_FEAR_BONUS, DOOR_DESTINY_POWER, WOUND_PENALTY, bond_power, fear_penalty,
};
use crate::content::Content;
use crate::door::outlook;
use crate::hero::{DeedKind, Hero, HeroId, Scope, kin};
use crate::house::House;
use crate::ids::{BondKind, Destiny, Place, Pronoun};
use crate::text::{capitalized, fmt, name_list};
use crate::words::W;

/// The prologue's lines for `party`, in order.
pub fn prologue(content: &Content, house: &House, party: &[HeroId]) -> Vec<String> {
    let words = &content.words;
    let heroes = &house.heroes;
    let full: Vec<String> = party.iter().map(|&m| heroes[m].full_name()).collect();
    let full: Vec<&str> = full.iter().map(String::as_str).collect();
    let mut lines = vec![fmt(&words[W::DoorPrologue], &[&name_list(content, &full)])];
    for &member in party {
        lines.push(brought(content, heroes, member));
    }
    for (at, &member) in party.iter().enumerate() {
        let items: Vec<String> = party[at + 1..]
            .iter()
            .filter_map(|&later| companion(content, heroes, member, later))
            .collect();
        if !items.is_empty() {
            let items: Vec<&str> = items.iter().map(String::as_str).collect();
            lines.push(fmt(
                &words[W::DoorWentBeside],
                &[&heroes[member].name, &name_list(content, &items)],
            ));
        }
    }
    let seen = outlook(content, heroes, party, house.patrons);
    let tags = &content.lore.places[Place::SealedDoor.index()].tags;
    let noun = |at: usize| content.lore.tags[tags[at].index()].noun.clone();
    let demands: Vec<String> = content
        .door
        .locks
        .iter()
        .map(|l| l.demand.to_string())
        .collect();
    let powers: Vec<String> = seen.powers.iter().map(i32::to_string).collect();
    lines.push(fmt(
        &words[W::DoorSummary],
        &[
            &demands[0],
            &demands[1],
            &demands[2],
            &powers[0],
            &powers[1],
            &powers[2],
            &capitalized(&noun(0)),
            &noun(1),
        ],
    ));
    lines
}

/// The first names of `ids`, as a name list.
fn names(content: &Content, heroes: &[Hero], ids: &[HeroId]) -> String {
    let names: Vec<&str> = ids.iter().map(|&id| heroes[id].name.as_str()).collect();
    name_list(content, &names)
}

/// The descent phrase (SPEC §16.2): come up the road, of the house before its years were
/// counted, the son or daughter of the parents, or of the parents and the grandparents.
/// SPEC-GAPS KG-66: parents by the `parents` field (as KG-63 has them); grandparents are
/// the parents' parents in that order, each once; all by first name.
fn descent(content: &Content, heroes: &[Hero], member: HeroId) -> String {
    let words = &content.words;
    let hero = &heroes[member];
    if let Some(arrived) = hero.deeds.iter().find(|d| d.kind == DeedKind::Arrived) {
        return fmt(&words[W::DoorDescentArrived], &[&arrived.year.to_string()]);
    }
    let parents: Vec<HeroId> = hero.parents.iter().flatten().copied().collect();
    if parents.is_empty() {
        return words[W::DoorDescentNone].to_owned();
    }
    let he = hero.pronoun == Pronoun::He;
    let child = &words[if he {
        W::DoorDescentChildHe
    } else {
        W::DoorDescentChildShe
    }];
    let mut grandparents: Vec<HeroId> = Vec::new();
    for &parent in &parents {
        for &grand in heroes[parent].parents.iter().flatten() {
            if !grandparents.contains(&grand) {
                grandparents.push(grand);
            }
        }
    }
    if grandparents.is_empty() {
        return fmt(
            &words[W::DoorDescentParents],
            &[child, &names(content, heroes, &parents)],
        );
    }
    let grandchild = &words[if he {
        W::DoorDescentGrandchildHe
    } else {
        W::DoorDescentGrandchildShe
    }];
    fmt(
        &words[W::DoorDescentGrandparents],
        &[
            child,
            &names(content, heroes, &parents),
            grandchild,
            &names(content, heroes, &grandparents),
        ],
    )
}

/// One member's `lines.door.brought` with every fragment that applies, in order.
fn brought(content: &Content, heroes: &[Hero], member: HeroId) -> String {
    let words = &content.words;
    let hero = &heroes[member];
    let pronouns = &content.lore.pronouns[hero.pronoun.index()];
    let he = capitalized(&pronouns.subject);
    let mut line = fmt(
        &words[W::DoorBrought],
        &[
            &hero.name,
            &hero.age.to_string(),
            &descent(content, heroes, member),
        ],
    );
    let door = DOOR_DESTINY_POWER.to_string();
    if hero.destiny.kind == Destiny::OpenTheSealedDoor {
        line += &match &hero.destiny.blood_of {
            Some(promised) => fmt(
                &words[W::DoorBroughtBlood],
                &[promised, &pronouns.subject, promised, &door],
            ),
            None => fmt(&words[W::DoorBroughtPromise], &[&door]),
        };
    }
    if let Some(heirloom) = &hero.heirloom {
        let own = format!("{}'s ", hero.name);
        let mine = format!("{} own ", pronouns.possessive);
        line += &fmt(
            &words[W::DoorBroughtHeirloom],
            &[
                &he,
                &heirloom.name.replace(&own, &mine),
                &heirloom.provenance,
            ],
        );
    }
    let tags = &content.lore.places[Place::SealedDoor.index()].tags;
    if tags.contains(&hero.fear.tag) {
        let noun = &content.lore.tags[hero.fear.tag.index()].noun;
        let bonus = CONQUERED_FEAR_BONUS.to_string();
        line += &if hero.fear.born_brave {
            fmt(&words[W::DoorBroughtBornBrave], &[&he, noun, &bonus])
        } else if hero.fear.conquered {
            match hero
                .deeds
                .iter()
                .find(|d| d.kind == DeedKind::ConqueredFear)
            {
                Some(conquest) => fmt(
                    &words[W::DoorBroughtConqueredYear],
                    &[
                        &he,
                        &pronouns.possessive,
                        noun,
                        &conquest.year.to_string(),
                        &bonus,
                    ],
                ),
                None => fmt(
                    &words[W::DoorBroughtConquered],
                    &[&he, &pronouns.possessive, noun, &bonus],
                ),
            }
        } else {
            fmt(
                &words[W::DoorBroughtAfraid],
                &[
                    &he,
                    noun,
                    &pronouns.possessive,
                    &fear_penalty(hero.fear.dread).to_string(),
                ],
            )
        };
    }
    for blessing in hero
        .blessings
        .iter()
        .filter(|b| b.scope == Scope::Everywhere)
    {
        line += &fmt(
            &words[W::DoorBroughtBlessing],
            &[
                &blessing.title,
                &pronouns.object,
                &blessing.power.to_string(),
            ],
        );
    }
    if hero.wounded {
        line += &fmt(
            &words[W::DoorBroughtWounded],
            &[&he, &WOUND_PENALTY.to_string()],
        );
    }
    line
}

/// The kinship between two party members (SPEC §21, "Kinship between"): a spouse, parent
/// or child bond names that kinship; else kin by a shared parent, brother or sister; else
/// the bond's own telling; with no bond, "companion".
fn kinship_between(content: &Content, heroes: &[Hero], a: HeroId, b: HeroId) -> String {
    let bond = heroes[a].bond_to(b).map(|bond| bond.kind);
    match bond {
        Some(kind @ (BondKind::Spouse | BondKind::Parent | BondKind::Child)) => {
            kinship_telling(content, kind, &heroes[b]).to_owned()
        }
        _ if kin(heroes, a, b) => content.bonds.sibling[heroes[b].pronoun.index()].clone(),
        Some(kind) => kinship_telling(content, kind, &heroes[b]).to_owned(),
        None => content.bonds.no_bond.clone(),
    }
}

/// `lines.door.companion` for `member`'s shown bond to `later`, if they hold one (SPEC
/// §16.2: a bond that is not COMPANION): "his daughter Maren (+2)".
fn companion(content: &Content, heroes: &[Hero], member: HeroId, later: HeroId) -> Option<String> {
    let bond = heroes[member].bond_to(later)?;
    if bond.kind == BondKind::Companion {
        return None;
    }
    let power = bond_power(bond.kind);
    let possessive = &content.lore.pronouns[heroes[member].pronoun.index()].possessive;
    Some(fmt(
        &content.words[W::DoorCompanion],
        &[
            possessive,
            &kinship_between(content, heroes, member, later),
            &heroes[later].name,
            if power > 0 { "+" } else { "" },
            &power.to_string(),
        ],
    ))
}
