//! Fear (SPEC §10.2-10.4): what a fear costs or gives on a quest, refusal, the
//! one dread rule, breaking, conquering and shedding.
//!
//! Every source of dread goes through `add_dread`, so "settled, conquered and
//! broken heroes cannot dread" and "at 5 they break" are said once. The rules
//! return the lines they write, in order; the page they land on is a later
//! wave's (the telling is W6, the turning W8).

use crate::constants::{CONQUERED_FEAR_BONUS, DREAD_LIMIT, REST_DREAD_SHED, fear_penalty};
use crate::content::Content;
use crate::hero::{Deed, DeedKind, Hero, HeroId};
use crate::ids::{Place, Tag};
use crate::text::{capitalized, fmt};
use crate::words::W;

/// Why dread came: something at a place, or grief for someone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Occasion {
    /// On a quest, or mended, at this place.
    At(Place),
    /// Grieving this hero.
    Grief(HeroId),
}

/// Whether the hero can gain dread: not settled, not conquered, not broken (SPEC §10.2).
pub fn can_dread(hero: &Hero) -> bool {
    !hero.settled && !hero.fear.conquered && !hero.fear.broken
}

/// Whether the fear costs power on a quest carrying `tags`: the quest carries the
/// tag and the fear is not conquered. Broken fears still count (SPEC §6 line 3).
pub fn fears(hero: &Hero, tags: &[Tag]) -> bool {
    !hero.fear.conquered && tags.contains(&hero.fear.tag)
}

/// Whether the hero refuses a quest carrying `tags`: a broken fear of one of them (SPEC §10.4).
pub fn refuses(hero: &Hero, tags: &[Tag]) -> bool {
    hero.fear.broken && tags.contains(&hero.fear.tag)
}

/// What the fear adds to the hero's power on a quest carrying `tags` (SPEC §6
/// lines 3 and 4): `-(2 + dread/2)` while it is feared, `+2` once it is conquered
/// (born brave included), 0 when the quest does not carry the tag.
pub fn fear_power(hero: &Hero, tags: &[Tag]) -> i32 {
    if !tags.contains(&hero.fear.tag) {
        0
    } else if hero.fear.conquered {
        CONQUERED_FEAR_BONUS
    } else {
        -fear_penalty(hero.fear.dread)
    }
}

/// The dread rule (SPEC §10.2): if the hero can dread and `amount` > 0, dread
/// rises to at most 5, and at 5 the hero breaks. Returns the lines breaking writes.
pub fn add_dread(
    content: &Content,
    heroes: &mut [Hero],
    id: HeroId,
    amount: i32,
    occasion: Occasion,
    year: i32,
) -> Vec<String> {
    let hero = &mut heroes[id];
    if !can_dread(hero) || amount <= 0 {
        return Vec::new();
    }
    hero.fear.dread = (hero.fear.dread + amount).min(DREAD_LIMIT);
    if hero.fear.dread >= DREAD_LIMIT {
        break_hero(content, heroes, id, occasion, year)
    } else {
        Vec::new()
    }
}

/// Breaking (SPEC §10.4): broken, a scar, a BROKEN deed, and `lines.fear.broken`.
pub fn break_hero(
    content: &Content,
    heroes: &mut [Hero],
    id: HeroId,
    occasion: Occasion,
    year: i32,
) -> Vec<String> {
    let words = &content.words;
    let noun = content.lore.tags[heroes[id].fear.tag.index()].noun.clone();
    // SPEC-GAPS KG-12: place for a break at a place, none for grief; weight 0; and
    // `grief.occasion` unread — grief has its own scar and deed strings.
    let (scar, telling, place, other) = match occasion {
        Occasion::Grief(dead) => {
            let name = &heroes[dead].name;
            (
                fmt(&words[W::ScarBrokenByGrief], &[name]),
                fmt(&words[W::DeedBrokenByGrief], &[name]),
                None,
                Some(dead),
            )
        }
        Occasion::At(place) => {
            let at = fmt(
                &words[W::AtPlace],
                &[&content.lore.places[place.index()].name],
            );
            (
                fmt(&words[W::ScarBroken], &[&noun, &at]),
                fmt(&words[W::DeedBroken], &[&noun, &at]),
                Some(place),
                None,
            )
        }
    };
    let hero = &mut heroes[id];
    hero.fear.broken = true;
    hero.scars.push(scar);
    hero.deeds.push(Deed {
        kind: DeedKind::Broken,
        year,
        age: hero.age,
        place,
        weight: 0,
        other,
        telling,
    });
    let pronouns = &content.lore.pronouns[hero.pronoun.index()];
    vec![fmt(
        &words[W::FearBroken],
        &[
            &hero.name,
            &capitalized(&pronouns.subject),
            &noun,
            &pronouns.possessive,
        ],
    )]
}

/// Conquering (SPEC §10.3), at `place`: conquered, dread 0, a CONQUERED_FEAR deed
/// and `lines.fear.conquered`. From then the fear gives +2 and takes no dread.
pub fn conquer(
    content: &Content,
    heroes: &mut [Hero],
    id: HeroId,
    place: Place,
    year: i32,
) -> Vec<String> {
    let words = &content.words;
    let hero = &mut heroes[id];
    let tag = &content.lore.tags[hero.fear.tag.index()];
    let at = fmt(
        &words[W::AtPlace],
        &[&content.lore.places[place.index()].name],
    );
    hero.fear.conquered = true;
    hero.fear.dread = 0;
    hero.deeds.push(Deed {
        kind: DeedKind::ConqueredFear,
        year,
        age: hero.age,
        place: Some(place),
        weight: 0,
        other: None,
        telling: fmt(&words[W::DeedConqueredFear], &[&tag.noun, &at]),
    });
    let possessive = &content.lore.pronouns[hero.pronoun.index()].possessive;
    vec![fmt(
        &words[W::FearConquered],
        &[
            &hero.name,
            possessive,
            &tag.noun,
            &CONQUERED_FEAR_BONUS.to_string(),
            &tag.title,
            &capitalized(possessive),
        ],
    )]
}

/// Whether resting by the fire would shed dread: not broken, and some to shed
/// (SPEC-GAPS KG-11: nothing is shed at 0). The fire's preview asks this (`plans::rest`).
pub fn can_shed(hero: &Hero) -> bool {
    !hero.fear.broken && hero.fear.dread > 0
}

/// Shedding by the fire (SPEC §10.2): 1 dread, never below 0, not if broken.
/// Returns whether any was shed (the rest line says so, W7).
pub fn shed_dread(hero: &mut Hero) -> bool {
    if !can_shed(hero) {
        return false;
    }
    hero.fear.dread = (hero.fear.dread - REST_DREAD_SHED).max(0);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::Tag;
    use crate::testkit::{founded, id};

    #[test]
    fn a_fear_costs_two_plus_half_the_dread_and_a_conquered_one_gives_two() {
        let (_, mut heroes) = founded();
        let garrick = id(&heroes, "Garrick");
        let water = [Tag::Water];
        let mut by_dread = Vec::new();
        for dread in 0..=5 {
            heroes[garrick].fear.dread = dread;
            by_dread.push(fear_power(&heroes[garrick], &water));
        }
        assert_eq!(by_dread, [-2, -2, -3, -3, -4, -4]);
        assert_eq!(fear_power(&heroes[garrick], &[Tag::Dark]), 0);
        heroes[garrick].fear.broken = true;
        assert_eq!(
            fear_power(&heroes[garrick], &water),
            -4,
            "a broken fear still costs"
        );
        heroes[garrick].fear.conquered = true;
        assert_eq!(fear_power(&heroes[garrick], &water), 2);
        assert_eq!(fear_power(&heroes[garrick], &[Tag::Cold]), 0);
    }

    #[test]
    fn only_a_broken_fear_refuses_and_only_quests_carrying_the_tag() {
        let (_, mut heroes) = founded();
        let maren = id(&heroes, "Maren");
        heroes[maren].fear.dread = 4;
        assert!(!refuses(&heroes[maren], &[Tag::Water]));
        heroes[maren].fear.broken = true;
        assert!(refuses(&heroes[maren], &[Tag::Dark, Tag::Water]));
        assert!(!refuses(&heroes[maren], &[Tag::Dark, Tag::Cold]));
        assert!(fears(&heroes[maren], &[Tag::Water]));
        heroes[maren].fear.conquered = true;
        assert!(!fears(&heroes[maren], &[Tag::Water]));
    }

    #[test]
    fn dread_rises_by_the_amount_and_stops_at_five_where_the_hero_breaks() {
        let (content, mut heroes) = founded();
        let garrick = id(&heroes, "Garrick");
        let at = Occasion::At(Place::DrownedCoast);
        assert!(add_dread(&content, &mut heroes, garrick, 2, at, 1).is_empty());
        assert_eq!(heroes[garrick].fear.dread, 4);
        assert!(!heroes[garrick].fear.broken);
        let lines = add_dread(&content, &mut heroes, garrick, 3, at, 1);
        assert_eq!(heroes[garrick].fear.dread, 5);
        assert!(heroes[garrick].fear.broken);
        assert_eq!(
            lines,
            [
                "Something in Garrick gave way. He will not go against deep water again, for anyone. It is a scar now, and his children may be born with it."
            ]
        );
        assert_eq!(
            heroes[garrick].scars.last().map(String::as_str),
            Some("Broken by deep water at the Drowned Coast")
        );
        let deed = heroes[garrick].deeds.last().cloned();
        assert_eq!(
            deed.as_ref().map(|d| (d.kind, d.year, d.age, d.place)),
            Some((DeedKind::Broken, 1, 62, Some(Place::DrownedCoast)))
        );
        assert_eq!(
            deed.map(|d| d.telling),
            Some("was broken by deep water at the Drowned Coast".to_owned())
        );
    }

    #[test]
    fn no_dread_comes_to_the_settled_the_conquered_the_broken_or_from_nothing() {
        let (content, mut heroes) = founded();
        let maren = id(&heroes, "Maren");
        let at = Occasion::At(Place::Barrow);
        add_dread(&content, &mut heroes, maren, 0, at, 1);
        add_dread(&content, &mut heroes, maren, -1, at, 1);
        assert_eq!(heroes[maren].fear.dread, 1);
        for flag in 0..3 {
            let mut hero = heroes.clone();
            match flag {
                0 => hero[maren].settled = true,
                1 => hero[maren].fear.conquered = true,
                _ => hero[maren].fear.broken = true,
            }
            assert!(!can_dread(&hero[maren]));
            add_dread(&content, &mut hero, maren, 9, at, 1);
            assert_eq!(hero[maren].fear.dread, 1, "flag {flag}");
            assert!(hero[maren].scars.is_empty());
        }
        assert!(can_dread(&heroes[maren]));
    }

    #[test]
    fn breaking_by_grief_names_the_dead_in_the_scar_and_the_deed() {
        let (content, mut heroes) = founded();
        let (maren, garrick) = (id(&heroes, "Maren"), id(&heroes, "Garrick"));
        let lines = break_hero(&content, &mut heroes, maren, Occasion::Grief(garrick), 3);
        assert_eq!(heroes[maren].scars, ["Broken by grief for Garrick"]);
        let deed = &heroes[maren].deeds[0];
        assert_eq!(
            (deed.kind, deed.other, deed.place, deed.telling.as_str()),
            (
                DeedKind::Broken,
                Some(garrick),
                None,
                "was broken by grief for Garrick"
            )
        );
        assert_eq!(
            lines,
            [
                "Something in Maren gave way. She will not go against deep water again, for anyone. It is a scar now, and her children may be born with it."
            ]
        );
    }

    #[test]
    fn conquering_clears_dread_records_the_deed_and_says_so() {
        let (content, mut heroes) = founded();
        let garrick = id(&heroes, "Garrick");
        let lines = conquer(&content, &mut heroes, garrick, Place::DrownedCoast, 2);
        assert!(heroes[garrick].fear.conquered);
        assert_eq!(heroes[garrick].fear.dread, 0);
        assert_eq!(
            lines,
            [
                "Garrick has conquered his fear of deep water. What was a weight is a strength: +2 against Water from now on. His children may be born brave."
            ]
        );
        let deed = &heroes[garrick].deeds[0];
        assert_eq!(
            (deed.kind, deed.year, deed.place, deed.telling.as_str()),
            (
                DeedKind::ConqueredFear,
                2,
                Some(Place::DrownedCoast),
                "conquered the fear of deep water at the Drowned Coast"
            )
        );
    }

    #[test]
    fn resting_sheds_one_dread_but_not_below_zero_and_not_for_the_broken() {
        let (_, mut heroes) = founded();
        let garrick = id(&heroes, "Garrick");
        assert!(shed_dread(&mut heroes[garrick]));
        assert_eq!(heroes[garrick].fear.dread, 1);
        assert!(shed_dread(&mut heroes[garrick]));
        assert!(!shed_dread(&mut heroes[garrick]));
        assert_eq!(heroes[garrick].fear.dread, 0);
        heroes[garrick].fear.dread = 5;
        heroes[garrick].fear.broken = true;
        assert!(!shed_dread(&mut heroes[garrick]));
        assert_eq!(heroes[garrick].fear.dread, 5);
    }
}
