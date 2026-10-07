//! Births (SPEC §17.3, `lineage/passage.jai:336-422`, `generation/hero-context/
//! hero-context.jai:100-146`): which wed pairs may have a child this turning, the
//! child they have, and the page that tells it.
//!
//! Each pair is asked once, from the partner created first, over the household as it
//! stood before any birth; the house's room and the yard's are asked before each roll,
//! and a full house stops every birth after it. The child takes a share of both
//! parents' base aptitudes, the house of the parent created first [emergent] (OQ-12),
//! maybe a parent's fear, and both parents' blessings.

use jidousha::prelude::Rng;

use crate::blessing::blessing_effect;
use crate::bonds::form;
use crate::chance::{between, chance, index};
use crate::constants::{
    BIRTH_CHANCE_PERCENT, BORN_BRAVE_CHANCE, BROKEN_FEAR_CHANCE, CHILDREN_PER_PAIR,
    CONQUERED_FEAR_BONUS, HOUSEHOLD_LIMIT, INHERITED_FEAR_CHANCE, MARRYING_AGE,
    NEWBORN_APTITUDE_LEAST, NEWBORN_APTITUDE_SHARE, PARENT_AGE_HIGH, SURPASSING_CHILD_BONUS,
    YARD_SPOTS,
};
use crate::content::Content;
use crate::heirs::deed;
use crate::hero::{DeedKind, HeroId};
use crate::house::House;
use crate::ids::{Aptitude, BondKind, Destiny, Pool, Pronoun, Tag};
use crate::newcomers::{fear_of, newcomer, roll_pronoun};
use crate::passage::{PageKind, TurnPage};
use crate::text::{capitalized, fmt, name_list};
use crate::words::W;

/// `hero`'s first living spouse, in bond order, and the year they wed.
fn first_living_spouse(house: &House, hero: HeroId) -> Option<(HeroId, i32)> {
    house.heroes[hero]
        .bonds
        .iter()
        .find(|b| b.kind == BondKind::Spouse && house.heroes[b.other].is_living())
        .map(|b| (b.other, b.since))
}

/// Whether `first` and their first living spouse may have a child this turning (SPEC
/// §17.3): asked from the one created first; not in the wedding's year; both 18..45
/// after this turning's ageing; fewer than three children together, living or dead.
/// Returns the spouse.
pub fn may_have_a_child(house: &House, first: HeroId) -> Option<HeroId> {
    let year = house.calendar.current_year();
    let (spouse, since) = first_living_spouse(house, first)?;
    let of_age = |id: HeroId| (MARRYING_AGE..=PARENT_AGE_HIGH).contains(&house.heroes[id].age);
    let together = house
        .heroes
        .iter()
        .filter(|h| h.parents.contains(&Some(first)) && h.parents.contains(&Some(spouse)))
        .count();
    (first < spouse
        && since < year
        && of_age(first)
        && of_age(spouse)
        && together < CHILDREN_PER_PAIR)
        .then_some(spouse)
}

/// The births of this turning, in creation order of the partner created first (SPEC
/// §18 step 5): a page for each.
pub fn births(content: &Content, house: &mut House, rng: &mut Rng) -> Vec<TurnPage> {
    let mut pages = Vec::new();
    let before: Vec<HeroId> = (0..house.heroes.len())
        .filter(|&id| house.heroes[id].is_living())
        .collect();
    for first in before {
        let Some(spouse) = may_have_a_child(house, first) else {
            continue;
        };
        let living = house.heroes.iter().filter(|h| h.is_living()).count();
        let children = house
            .heroes
            .iter()
            .filter(|h| h.is_living() && !h.is_adult())
            .count();
        if living >= HOUSEHOLD_LIMIT || children >= YARD_SPOTS {
            break;
        }
        if chance(rng, BIRTH_CHANCE_PERCENT as f32 / 100.0) {
            pages.push(born(content, house, first, spouse, rng));
        }
    }
    pages
}

/// A child born to `first` and `second` (SPEC §17.3), and the page that tells it.
fn born(
    content: &Content,
    house: &mut House,
    first: HeroId,
    second: HeroId,
    rng: &mut Rng,
) -> TurnPage {
    let words = &content.words;
    let year = house.calendar.current_year();
    let pronoun = roll_pronoun(rng);
    let name = house.bags.name(pronoun, rng);
    // Variant: the house name is the family parent's, the first-created's when both are.
    let family = if house.heroes[first].family || !house.heroes[second].family {
        house.heroes[first].house.clone()
    } else {
        house.heroes[second].house.clone()
    };
    let mut aptitudes = [0; 3];
    for aptitude in Aptitude::ALL {
        let shared = house.heroes[first].base(*aptitude) + house.heroes[second].base(*aptitude);
        aptitudes[aptitude.index()] =
            (shared / NEWBORN_APTITUDE_SHARE + between(rng, 0, 1)).max(NEWBORN_APTITUDE_LEAST);
    }
    for parent in [first, second] {
        let hero = &mut house.heroes[parent];
        let childless = !hero.bonds.iter().any(|b| b.kind == BondKind::Child);
        if hero.destiny.kind == Destiny::ChildWillSurpassYou && childless {
            aptitudes
                .iter_mut()
                .for_each(|a| *a += SURPASSING_CHILD_BONUS);
            hero.destiny.fulfilled = true;
        }
    }
    let mut fear = fear_of(Tag::ALL[index(rng, Tag::ALL.len())]);
    let mut brave_from = None;
    for parent in [first, second] {
        let theirs = &house.heroes[parent].fear;
        if theirs.conquered {
            if chance(rng, BORN_BRAVE_CHANCE as f32) {
                fear = fear_of(theirs.tag);
                fear.conquered = true;
                fear.born_brave = true;
                brave_from = Some(parent);
                break;
            }
            continue;
        }
        let p = if theirs.broken {
            BROKEN_FEAR_CHANCE
        } else {
            INHERITED_FEAR_CHANCE
        };
        if chance(rng, p as f32) {
            fear = fear_of(theirs.tag);
            break;
        }
    }
    let mut child = newcomer(name, family, pronoun, 0, year, fear);
    child.aptitudes = aptitudes;
    child.parents = [Some(first), Some(second)];
    child.family = house.heroes[first].family || house.heroes[second].family;
    for parent in [first, second] {
        for blessing in &house.heroes[parent].blessings {
            if !child.blessings.iter().any(|b| b.title == blessing.title) {
                child.blessings.push(blessing.clone());
            }
        }
    }
    let id = house.heroes.len();
    house.heroes.push(child);
    form(&mut house.heroes, id, first, BondKind::Parent, year);
    form(&mut house.heroes, id, second, BondKind::Parent, year);
    let telling = fmt(&words[W::DeedChildBorn], &[&house.heroes[id].name]);
    for parent in [first, second] {
        deed(
            &mut house.heroes[parent],
            DeedKind::ChildBorn,
            year,
            Some(id),
            telling.clone(),
        );
    }
    let pool = house.writing.pick(content, Pool::Births, rng);
    let heroes = &house.heroes;
    let child = &heroes[id];
    let forms = &content.lore.pronouns[child.pronoun.index()];
    let tag = &content.lore.tags[child.fear.tag.index()];
    let mut lines = vec![
        fmt(pool, &[&heroes[first].name, &heroes[second].name]),
        fmt(&words[W::BirthNamed], &[&forms.object, &child.name]),
    ];
    // [emergent] the fear is put down to a parent whose tag it is, the second parent
    // first, even when the tag was rolled at random (OQ-18).
    let alike = [second, first]
        .into_iter()
        .find(|&p| heroes[p].fear.tag == child.fear.tag);
    lines.push(match (brave_from, alike) {
        (Some(parent), _) => fmt(
            &words[W::BirthBornBrave],
            &[
                &heroes[parent].name,
                &content.lore.pronouns[heroes[parent].pronoun.index()].possessive,
                &tag.noun,
                &CONQUERED_FEAR_BONUS.to_string(),
                &tag.title,
            ],
        ),
        (None, Some(parent)) => fmt(
            &words[W::BirthParentFear],
            &[
                &capitalized(&forms.subject),
                &forms.possessive,
                &words[if heroes[parent].pronoun == Pronoun::He {
                    W::BirthFather
                } else {
                    W::BirthMother
                }],
                &tag.noun,
            ],
        ),
        (None, None) => fmt(&words[W::BirthOwnFear], &[&forms.subject, &tag.noun]),
    });
    match child.blessings.as_slice() {
        [] => {}
        [one] => lines.push(fmt(
            &words[W::BirthBlessingOne],
            &[
                &capitalized(&forms.subject),
                &one.title,
                &blessing_effect(content, one),
            ],
        )),
        many => {
            let items: Vec<String> = many
                .iter()
                .map(|b| {
                    fmt(
                        &words[W::BirthBlessingItem],
                        &[&b.title, &blessing_effect(content, b)],
                    )
                })
                .collect();
            let items: Vec<&str> = items.iter().map(String::as_str).collect();
            lines.push(fmt(
                &words[W::BirthBlessingMany],
                &[
                    &capitalized(&forms.subject),
                    // SPEC-GAPS KG-50: the count as a numeral.
                    &many.len().to_string(),
                    &name_list(content, &items),
                ],
            ));
        }
    }
    TurnPage {
        kind: PageKind::Birth,
        title: fmt(&words[W::BirthTitle], &[&child.name, &child.house]),
        lines,
        bequest: None,
        about: Some(id),
    }
}
