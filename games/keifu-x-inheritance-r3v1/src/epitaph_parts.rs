//! An epitaph's parts (SPEC §20 "Parts", `lineage/epitaph.jai:154-310`): each part's
//! selection rule, branch by branch, in the order the spec lists them — ORIGIN, ROADS,
//! TRIUMPH, FEAR and DREAM here; PROPHECY, LOVE, END and LEFT in `epitaph_ends.rs` — and
//! the helpers they share. A part's sentence is `""` when its rule gives none.

use crate::content::Content;
use crate::dream::told_title;
use crate::epitaph::Wording;
use crate::epitaph_ends::{end, left, love, prophecy};
use crate::epitaph_lore::E;
use crate::hero::{Deed, DeedKind, DreamFate, Fate, Hero, HeroId};
use crate::ids::{BondKind, Part, Pronoun};
use crate::text::{capitalized, count_words, fmt, name_list, year_telling};

/// The hero's pronoun forms, as the templates' `args` name them.
pub struct Forms {
    /// "He"/"She".
    pub he: String,
    /// "His"/"Her".
    pub his: String,
    /// he/she.
    pub subject: String,
    /// him/her.
    pub object: String,
    /// his/her.
    pub possessive: String,
}

pub fn forms(content: &Content, pronoun: Pronoun) -> Forms {
    let lore = &content.lore.pronouns[pronoun.index()];
    Forms {
        he: capitalized(&lore.subject),
        his: capitalized(&lore.possessive),
        subject: lore.subject.clone(),
        object: lore.object.clone(),
        possessive: lore.possessive.clone(),
    }
}

/// `v1` or `v0` by the part's coin.
pub fn variant(wording: Wording, part: Part, v1: E, v0: E) -> E {
    if wording.coin(part) { v1 } else { v0 }
}

/// The first deed of `kind`, in the order they happened.
pub fn first_deed(hero: &Hero, kind: DeedKind) -> Option<&Deed> {
    hero.deeds.iter().find(|deed| deed.kind == kind)
}

/// A place a deed names: the mid-sentence `name` ("the Barrow"), as SPEC-GAPS KG-17 reads
/// "place name". A deed the rules record with no place is a bug in the recording.
fn deed_place<'c>(content: &'c Content, hero: &Hero, deed: &Deed) -> &'c str {
    let Some(place) = deed.place else {
        panic!(
            "[keifu_x_inheritance_r3v1] {}'s {:?} deed has no place, and the epitaph names it\n  likely cause: \
             the deed was recorded off a quest\n  fix: SPEC §7.1 and §7.3 record the quest's place",
            hero.name, deed.kind
        );
    };
    &content.lore.places[place.index()].name
}

/// A year telling without its leading "in " (the `args` of `fear.conquered_0` and
/// `fear.broken_0`: "until year 4", "After year 4").
fn bare_year(content: &Content, year: i32) -> String {
    let telling = year_telling(content, year);
    match telling.strip_prefix("in ") {
        Some(bare) => bare.to_owned(),
        None => telling,
    }
}

/// Every part's sentence for `id` under `wording`, by `Part::index`; "" for an empty part.
pub fn parts(content: &Content, heroes: &[Hero], id: HeroId, wording: Wording) -> [String; 9] {
    let mut out: [String; 9] = Default::default();
    for part in Part::ALL {
        out[part.index()] = match part {
            Part::Origin => origin(content, heroes, id, wording),
            Part::Roads => roads(content, heroes, id, wording),
            Part::Triumph => triumph(content, &heroes[id], wording),
            Part::Fear => fear(content, heroes, id, wording),
            Part::Dream => dream(content, heroes, id, wording),
            Part::Prophecy => prophecy(content, heroes, id),
            Part::Love => love(content, heroes, id, wording),
            Part::End => end(content, &heroes[id], wording),
            Part::Left => left(content, heroes, id),
        };
    }
    out
}

/// ORIGIN (`:154-177`): an arrival; nothing for the dead before the first year; born in
/// play to parents; a child of the old house; a founder. SPEC-GAPS KG-63: "a parent" is
/// the `parents` field, named by first name in its order.
fn origin(content: &Content, heroes: &[Hero], id: HeroId, wording: Wording) -> String {
    let lore = &content.epitaph;
    let hero = &heroes[id];
    let f = forms(content, hero.pronoun);
    if let Some(arrived) = first_deed(hero, DeedKind::Arrived) {
        let (year, age) = (arrived.year.to_string(), arrived.age.to_string());
        return if wording.coin(Part::Origin) {
            fmt(&lore[E::OriginArrived1], &[&f.he, &f.he, &year, &age])
        } else {
            fmt(&lore[E::OriginArrived0], &[&f.he, &year, &age])
        };
    }
    if !hero.is_living() && hero.fate_year < 1 {
        return String::new();
    }
    let parents: Vec<&str> = hero
        .parents
        .iter()
        .flatten()
        .map(|&p| heroes[p].name.as_str())
        .collect();
    if hero.born_year >= 1 && !parents.is_empty() {
        let key = variant(wording, Part::Origin, E::OriginBorn1, E::OriginBorn0);
        let names = name_list(content, &parents);
        return fmt(&lore[key], &[&f.he, &hero.born_year.to_string(), &names]);
    }
    if !parents.is_empty() {
        return fmt(
            &lore[E::OriginChildOfOld],
            &[&f.he, &name_list(content, &parents)],
        );
    }
    let key = variant(wording, Part::Origin, E::OriginFounder1, E::OriginFounder0);
    fmt(&lore[key], &[&f.he])
}

/// ROADS (`:187-219`): never quested (a living child's teachers, a dead child, an adult
/// who kept the house); quested before any FIRST_QUEST deed was kept (a founder, KG-40);
/// once (SPEC-GAPS KG-59: `quests_faced` is exactly one); many.
fn roads(content: &Content, heroes: &[Hero], id: HeroId, wording: Wording) -> String {
    let lore = &content.epitaph;
    let hero = &heroes[id];
    let f = forms(content, hero.pronoun);
    if hero.quests_faced == 0 {
        if hero.is_living() && !hero.is_adult() {
            let teachers: Vec<&str> = hero
                .bonds
                .iter()
                .filter(|bond| bond.kind == BondKind::Mentor)
                .map(|bond| heroes[bond.other].name.as_str())
                .collect();
            if teachers.is_empty() {
                return String::new();
            }
            return fmt(
                &lore[E::RoadsChildhoodTeachers],
                &[&name_list(content, &teachers), &f.object],
            );
        }
        if !hero.is_living() && !hero.is_adult() {
            return fmt(&lore[E::RoadsNeverOldEnough], &[&f.he]);
        }
        let key = variant(wording, Part::Roads, E::RoadsKeptHouse1, E::RoadsNeverWent0);
        return fmt(&lore[key], &[&f.he]);
    }
    let count = count_words(content, hero.quests_faced as usize);
    let Some(first) = first_deed(hero, DeedKind::FirstQuest) else {
        return fmt(&lore[E::RoadsCountOnly], &[&f.he, &count]);
    };
    let (age, place) = (first.age.to_string(), deed_place(content, hero, first));
    if hero.quests_faced == 1 {
        return fmt(&lore[E::RoadsOnce], &[&f.he, &age, place]);
    }
    if wording.coin(Part::Roads) {
        fmt(&lore[E::RoadsMany1], &[&f.his, place, &age, &f.he, &count])
    } else {
        fmt(&lore[E::RoadsMany0], &[&f.he, &age, place, &count])
    }
}

/// TRIUMPH (`:221-229`): the TRIUMPH deed of greatest weight, the later on a tie.
fn triumph(content: &Content, hero: &Hero, wording: Wording) -> String {
    let lore = &content.epitaph;
    let mut best: Option<&Deed> = None;
    for deed in hero.deeds.iter().filter(|d| d.kind == DeedKind::Triumph) {
        if best.is_none_or(|b| deed.weight >= b.weight) {
            best = Some(deed);
        }
    }
    let Some(best) = best else {
        return String::new();
    };
    let f = forms(content, hero.pronoun);
    let place = deed_place(content, hero, best);
    let year = year_telling(content, best.year);
    if wording.coin(Part::Triumph) {
        fmt(&lore[E::TriumphSpokenOf1], &[&f.he, place, &year])
    } else {
        fmt(&lore[E::TriumphBestDay0], &[&f.his, place, &year])
    }
}

/// FEAR (`:231-264`): born brave; conquered; broken by grief; broken; faced; a child's;
/// kept from it.
fn fear(content: &Content, heroes: &[Hero], id: HeroId, wording: Wording) -> String {
    let lore = &content.epitaph;
    let hero = &heroes[id];
    let f = forms(content, hero.pronoun);
    let noun = &content.lore.tags[hero.fear.tag.index()].noun;
    if hero.fear.born_brave {
        return fmt(&lore[E::FearBornBrave], &[&f.he, noun, &f.object]);
    }
    if let Some(deed) = first_deed(hero, DeedKind::ConqueredFear) {
        return if wording.coin(Part::Fear) {
            let year = year_telling(content, deed.year);
            fmt(
                &lore[E::FearConquered1],
                &[&f.he, &f.possessive, noun, &year],
            )
        } else {
            let year = bare_year(content, deed.year);
            fmt(&lore[E::FearConquered0], &[&f.he, noun, &year, &f.subject])
        };
    }
    let broken: Vec<&Deed> = hero
        .deeds
        .iter()
        .filter(|d| d.kind == DeedKind::Broken)
        .collect();
    if let Some(mourned) = broken.iter().find_map(|d| d.other) {
        return fmt(
            &lore[E::FearBrokenByGrief],
            &[&heroes[mourned].name, &f.object, &f.subject, noun],
        );
    }
    if let Some(deed) = broken.first() {
        return if wording.coin(Part::Fear) {
            let year = capitalized(&year_telling(content, deed.year));
            fmt(
                &lore[E::FearBroken1],
                &[&year, noun, &f.object, &f.subject, noun],
            )
        } else {
            let year = bare_year(content, deed.year);
            fmt(&lore[E::FearBroken0], &[&f.he, noun, &year, &f.subject])
        };
    }
    if hero.fears_faced > 0 {
        return if wording.coin(Part::Fear) {
            let count = count_words(content, hero.fears_faced as usize);
            fmt(&lore[E::FearFaced1], &[&f.he, noun, &f.he, &count])
        } else {
            fmt(&lore[E::FearFaced0], &[&f.he, noun, &f.possessive])
        };
    }
    if !hero.is_adult() {
        return fmt(&lore[E::FearChild], &[&f.subject, noun]);
    }
    fmt(&lore[E::FearKeptFrom], &[&f.he, noun])
}

/// DREAM (`:271-310`), from the own dream only (OQ-2): fulfilled; passed on to an heir;
/// left to no one; laid to rest; still living; left for a crown; died before it was done.
fn dream(content: &Content, heroes: &[Hero], id: HeroId, wording: Wording) -> String {
    let lore = &content.epitaph;
    let hero = &heroes[id];
    let Some(dream) = &hero.dream else {
        return String::new();
    };
    let f = forms(content, hero.pronoun);
    let owner_pronoun = dream.owner.map_or(hero.pronoun, |o| heroes[o].pronoun);
    let title = told_title(content, dream, owner_pronoun);
    let suffix = match dream.owner {
        Some(owner) => fmt(
            &lore[E::DreamInheritedSuffix],
            &[&heroes[owner].name, &f.object],
        ),
        None => String::new(),
    };
    if dream.is_fulfilled() {
        return if wording.coin(Part::Dream) {
            fmt(&lore[E::DreamFulfilled1], &[&f.his, &title, &f.he])
        } else {
            fmt(&lore[E::DreamFulfilled0], &[&f.he, &title, &suffix])
        };
    }
    match (hero.dream_fate, hero.bequest_heir) {
        (DreamFate::PassedOn, Some(heir)) => {
            let heir = &heroes[heir].name;
            return if wording.coin(Part::Dream) {
                fmt(&lore[E::DreamPassed1], &[&f.his, &title, heir])
            } else {
                fmt(&lore[E::DreamPassed0], &[&f.he, &title, &suffix, heir])
            };
        }
        (DreamFate::LeftToNoOne, _) => {
            return fmt(&lore[E::DreamLeft], &[&f.he, &title, &suffix]);
        }
        (DreamFate::LaidToRest, _) => {
            let Some(year) = hero.laid_year else {
                panic!(
                    "[keifu_x_inheritance_r3v1] {}'s dream is laid to rest in no year\n  likely cause: the fate \
                     was set without the year\n  fix: SPEC §14.4 sets both together",
                    hero.name
                );
            };
            return fmt(
                &lore[E::DreamLaid],
                &[&f.he, &title, &suffix, &year.to_string()],
            );
        }
        _ => {}
    }
    let key = match hero.fate {
        Fate::Living => {
            let telling = &lore.progress_tellings[dream.current.min(2)];
            return fmt(&lore[E::DreamLiving], &[&f.he, &title, &suffix, telling]);
        }
        Fate::Departed => E::DreamCrowned,
        Fate::Dead => E::DreamDied,
    };
    fmt(&lore[key], &[&f.he, &title, &suffix])
}
