//! The top bar (SPEC §5.4) and the family screen's membership (SPEC §19.2), as information.
//!
//! The family screen shows every hero who ever lived, parents above children and
//! spouses linked. Pointing at a hero shows the remembrance (§19.2): the epitaph once
//! one is composed (the dead, the crowned), otherwise for the living their station and
//! the `ui.family.living_*` lines.

use crate::content::Content;
use crate::dream::{progress, told_title};
use crate::hero::{Hero, HeroId};
use crate::house::House;
use crate::ids::{BondKind, Destiny, Place};
use crate::sheet::station;
use crate::text::{capitalized, fmt};
use crate::words::W;

/// The top bar's five readings, in order (SPEC §5.4; the W0 oracle).
///
/// The Door outlook ("Your best four today ...") reads power (§6, §16.4) and lands
/// with W10; the top bar shows it from then.
pub fn top_bar(content: &Content, house: &House) -> [String; 5] {
    let words = &content.words;
    let calendar = house.calendar;
    let year = if calendar.is_last_summer() {
        words[W::TopLastSummer].to_owned()
    } else {
        fmt(
            &words[W::TopYear],
            &[
                &calendar.current_year().to_string(),
                &crate::constants::DOOR_YEARS.to_string(),
            ],
        )
    };
    let season = content.lore.seasons[calendar.season()].clone();
    let renown = fmt(&words[W::TopRenown], &[&house.renown.to_string()]);
    let door = if calendar.door_stands_open() {
        words[W::TopDoorOpen].to_owned()
    } else {
        let years = calendar.years_until_door();
        // SPEC-GAPS KG-2: the year/years word is `lines.turning.year_word(s)`.
        let word = if years == 1 {
            W::YearWord
        } else {
            W::YearsWord
        };
        fmt(
            &words[W::TopDoorCountdown],
            &[&years.to_string(), &words[word]],
        )
    };
    let door_place = &content.lore.places[Place::SealedDoor.index()];
    let tags: Vec<&str> = door_place
        .tags
        .iter()
        .map(|tag| content.lore.tags[tag.index()].title.as_str())
        .collect();
    let locks: Vec<String> = content.door_locks.iter().map(i32::to_string).collect();
    let mut args: Vec<&str> = tags;
    args.extend(locks.iter().map(String::as_str));
    let door_tags = fmt(&words[W::TopDoorTags], &args);
    [year, season, renown, door, door_tags]
}

/// Which generation a hero is drawn in: one below their deepest parent.
pub fn generation(heroes: &[Hero], id: HeroId) -> usize {
    heroes[id]
        .parents
        .iter()
        .flatten()
        .map(|&parent| generation(heroes, parent) + 1)
        .max()
        .unwrap_or(0)
}

/// The tree's rows: per generation, its heroes, with each hero's spouses placed
/// straight after them so a spouse link never crosses a third node.
pub fn tree_rows(heroes: &[Hero]) -> Vec<Vec<HeroId>> {
    let depth = (0..heroes.len())
        .map(|id| generation(heroes, id))
        .max()
        .unwrap_or(0);
    let mut rows = vec![Vec::new(); depth + 1];
    let mut placed = vec![false; heroes.len()];
    for id in 0..heroes.len() {
        if placed[id] {
            continue;
        }
        let row = generation(heroes, id);
        rows[row].push(id);
        placed[id] = true;
        for bond in heroes[id]
            .bonds
            .iter()
            .filter(|b| b.kind == BondKind::Spouse)
        {
            if !placed[bond.other] && generation(heroes, bond.other) == row {
                rows[row].push(bond.other);
                placed[bond.other] = true;
            }
        }
    }
    rows
}

/// Every spouse pair, once, lower id first.
pub fn spouse_pairs(heroes: &[Hero]) -> Vec<(HeroId, HeroId)> {
    let mut pairs = Vec::new();
    for (id, hero) in heroes.iter().enumerate() {
        for bond in hero.bonds.iter().filter(|b| b.kind == BondKind::Spouse) {
            if id < bond.other {
                pairs.push((id, bond.other));
            }
        }
    }
    pairs
}

/// "L living, G gone. <tales sentence> House renown R." under "Year N." (SPEC §19.2).
pub fn tally(content: &Content, house: &House) -> String {
    let words = &content.words;
    let (living, gone) = house.living_and_gone();
    let tales = match house.tales.len() {
        0 => words[W::FamilyTallyNone].to_owned(),
        1 => words[W::FamilyTallyOne].to_owned(),
        n => fmt(&words[W::FamilyTallyMany], &[&n.to_string()]),
    };
    let sentence = fmt(
        &words[W::FamilyTally],
        &[
            &living.to_string(),
            &gone.to_string(),
            &tales,
            &house.renown.to_string(),
        ],
    );
    fmt(
        &words[W::FamilySubline],
        &[&house.calendar.current_year().to_string(), &sentence],
    )
}

/// The remembrance for a pointed-at hero (SPEC §19.2): their full name, then the epitaph
/// if one is composed, else — for the living — the station and the `living_*` lines.
pub fn remembrance(content: &Content, house: &House, id: HeroId) -> Vec<String> {
    let words = &content.words;
    let heroes = &house.heroes;
    let hero = &heroes[id];
    if let Some(epitaph) = &hero.epitaph {
        return vec![hero.full_name(), epitaph.clone()];
    }
    if !hero.is_living() {
        // SPEC-GAPS KG-57: a summer's dead await their death page, and their epitaph, until
        // the turning; until then they read as their sheet's condition line.
        let sheet = crate::sheet::hero_sheet(content, heroes, id);
        return sheet
            .lines
            .into_iter()
            .take(3)
            .map(|line| line.text)
            .collect();
    }
    let pronouns = &content.lore.pronouns[hero.pronoun.index()];
    let he = capitalized(&pronouns.subject);
    let mut sentence = fmt(&words[W::FamilyLivingStation], &[&station(content, hero)]);
    // The dream fragments carry no leading space and every later one does.
    match &hero.dream {
        Some(dream) if dream.is_fulfilled() => {
            sentence += &(" ".to_owned()
                + &fmt(
                    &words[W::FamilyLivingDone],
                    &[&he, &told_title(content, dream, hero.pronoun)],
                ))
        }
        Some(dream) => {
            sentence += &(" ".to_owned()
                + &fmt(
                    &words[W::FamilyLivingWants],
                    &[
                        &he,
                        &told_title(content, dream, hero.pronoun),
                        &progress(content, dream, hero.pronoun),
                    ],
                ))
        }
        None if !hero.is_adult() => {
            sentence +=
                &(" ".to_owned() + &fmt(&words[W::FamilyLivingTooYoung], &[&he, &pronouns.subject]))
        }
        // SPEC-GAPS KG-4: an undreamt adult has no dream sentence.
        None => {}
    }
    if let Some(burden) = &hero.burden {
        let owner = burden
            .owner
            .map_or(hero.name.as_str(), |o| heroes[o].name.as_str());
        let owner_pronoun = burden.owner.map_or(hero.pronoun, |o| heroes[o].pronoun);
        sentence += &fmt(
            &words[W::FamilyLivingBurden],
            &[&he, owner, &told_title(content, burden, owner_pronoun)],
        );
    }
    let noun = &content.lore.tags[hero.fear.tag.index()].noun;
    let fear = if hero.fear.conquered || hero.fear.born_brave {
        W::FamilyLivingNoFear
    } else if hero.fear.broken {
        W::FamilyLivingBroken
    } else {
        W::FamilyLivingFears
    };
    sentence += &fmt(&words[fear], &[&he, noun]);
    // SPEC-GAPS KG-3: an unspoken destiny is not quoted as something the Seer said.
    if hero.destiny.kind != Destiny::Unspoken {
        let prophecy = &content.destinies[hero.destiny.kind.index()].prophecy;
        sentence += &fmt(&words[W::FamilyLivingSeer], &[prophecy]);
    }
    vec![hero.full_name(), sentence.trim_end().to_owned()]
}
