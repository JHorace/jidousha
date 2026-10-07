//! The founding household (SPEC §4), read from `content/household.json`.
//!
//! `read_household` validates the file into `Founding`; `found` turns that into
//! heroes, in creation order, with their dreams advanced, their bonds formed and
//! mirrored in formation order, and the dead marked as the spec says.

use crate::constants::HOUSE_RENOWN_AT_START;
use crate::content::{Content, id_at, strings, text};
use crate::dream::{Dream, Setup};
use crate::hero::{Bond, DestinyState, Fate, Fear, Heirloom, Hero, HeroId};
use crate::ids::{
    Aptitude, BondKind, Destiny, DreamKind, LegacyKind, Place, Pronoun, Tag, Vocation,
};
use crate::json::{At, SchemaError};

/// A founding dream as authored.
pub struct FoundingDream {
    /// Which.
    pub kind: DreamKind,
    /// AVENGE_THE_LOST's place, tag and the lost hero's key.
    pub setup: Option<(Place, Tag, String)>,
    /// The current stage index.
    pub stage: usize,
}

/// One founding hero as authored, with every field the file sets.
pub struct FoundingHero {
    /// The file's key ("garrick").
    pub key: String,
    /// The hero, minus the fields that name other heroes.
    pub hero: Hero,
    /// The dream, built at founding.
    pub dream: Option<FoundingDream>,
    /// Parent keys.
    pub parents: Vec<Option<String>>,
    /// Where a dead hero fell, if among a place's fallen.
    pub fallen_at: Option<Place>,
}

/// `household.json`, validated.
pub struct Founding {
    /// In creation order.
    pub heroes: Vec<FoundingHero>,
    /// (hero, other, kind, since), in formation order.
    pub bonds: Vec<(String, String, BondKind, i32)>,
    /// Keys of the dead at start.
    pub dead_at_start: Vec<String>,
}

/// Read and check `household.json`.
pub fn read_household(at: &At<'_>) -> Result<Founding, SchemaError> {
    let renown = at.key("house_renown_at_start")?.int()?;
    if renown != HOUSE_RENOWN_AT_START {
        return Err(at.reject(format!(
            "house_renown_at_start {renown} disagrees with CONSTANTS.md §1 ({HOUSE_RENOWN_AT_START})"
        )));
    }
    let mut heroes = Vec::new();
    for item in at.key("heroes")?.items()? {
        heroes.push(read_hero(&item)?);
    }
    let mut bonds = Vec::new();
    for bond in at.key("bonds")?.items()? {
        bonds.push((
            text(&bond, "hero")?,
            text(&bond, "other")?,
            id_at(&bond, "kind", BondKind::find)?,
            bond.key("since")?.int()?,
        ));
    }
    Ok(Founding {
        heroes,
        bonds,
        dead_at_start: strings(at, "dead_at_start")?,
    })
}

fn read_hero(item: &At<'_>) -> Result<FoundingHero, SchemaError> {
    let count = |key: &str| -> Result<i32, SchemaError> {
        Ok(match item.find(key)? {
            Some(value) => value.int()?,
            None => 0,
        })
    };
    let aptitudes_at = item.key("aptitudes")?;
    let mut aptitudes = [0; 3];
    for aptitude in Aptitude::ALL {
        aptitudes[aptitude.index()] = aptitudes_at.key(aptitude.id())?.int()?;
    }
    let fear = item.key("fear")?;
    let fate = match text(item, "fate")?.as_str() {
        "LIVING" => Fate::Living,
        "DEAD" => Fate::Dead,
        "DEPARTED" => Fate::Departed,
        other => return Err(item.reject(format!("fate {other:?} is not LIVING/DEAD/DEPARTED"))),
    };
    let dream = match item.find("dream")? {
        None => None,
        Some(dream) => Some(FoundingDream {
            kind: id_at(&dream, "kind", DreamKind::find)?,
            setup: match dream.find("setup")? {
                None => None,
                Some(setup) => Some((
                    id_at(&setup, "place", Place::find)?,
                    id_at(&setup, "tag", Tag::find)?,
                    text(&setup, "lost")?,
                )),
            },
            stage: dream.key("stage")?.int()? as usize,
        }),
    };
    let heirloom = match item.find("heirloom")? {
        None => None,
        Some(h) => Some(Heirloom {
            name: text(&h, "name")?,
            sprite: text(&h, "sprite")?,
            aptitude: id_at(&h, "aptitude", Aptitude::find)?,
            bonus: h.key("bonus")?.int()?,
            provenance: text(&h, "provenance")?,
        }),
    };
    let places = |key: &str| -> Result<Vec<Place>, SchemaError> {
        strings(item, key)?
            .iter()
            .map(|id| Place::find(id).ok_or_else(|| item.reject(format!("{key}: place {id:?}"))))
            .collect()
    };
    let optional_text = |key: &str| -> Result<String, SchemaError> {
        Ok(match item.find(key)? {
            Some(value) => value.str()?,
            None => String::new(),
        })
    };
    let mut hero = Hero {
        key: text(item, "key")?,
        name: text(item, "name")?,
        house: text(item, "house")?,
        family: true,
        pronoun: id_at(item, "pronoun", Pronoun::find)?,
        vocation: id_at(item, "vocation", Vocation::find)?,
        age: item.key("age")?.int()?,
        born_year: item.key("born_year")?.int()?,
        aptitudes,
        dream: None,
        burden: None,
        traits: Vec::new(),
        fear: Fear {
            tag: id_at(&fear, "tag", Tag::find)?,
            dread: fear.key("dread")?.int()?,
            courage: 0,
            conquered: false,
            broken: false,
            born_brave: false,
        },
        destiny: DestinyState {
            kind: id_at(item, "destiny", Destiny::find)?,
            fulfilled: match item.find("destiny_fulfilled")? {
                Some(flag) => flag.bool()?,
                None => false,
            },
            blood_of: None,
        },
        bonds: Vec::new(),
        heirloom,
        blessings: Vec::new(),
        scars: match item.find("scars")? {
            Some(_) => strings(item, "scars")?,
            None => Vec::new(),
        },
        deeds: Vec::new(),
        legacy: (LegacyKind::None, String::new()),
        renown: count("renown")?,
        wounded: false,
        settled: false,
        fate,
        fate_year: count("fate_year")?,
        fate_age: count("fate_age")?,
        fate_telling: optional_text("fate_telling")?,
        death_place: None,
        death_tag: None,
        grieved: false,
        bequest_decided: false,
        bequest_heir: None,
        bequest_heirloom: None,
        dream_fate: crate::hero::DreamFate::Undecided,
        laid_year: None,
        wording: None,
        epitaph: None,
        parents: [None, None],
        roads_walked: places("roads_walked")?,
        quests_faced: count("quests_faced")?,
        fears_faced: count("fears_faced")?,
        winters_taught: 0,
    };
    if item.find("trait")?.is_some() {
        hero.traits = vec![id_at(item, "trait", crate::ids::Trait::find)?];
    }
    let parents = item
        .key("parents")?
        .items()?
        .iter()
        .map(|parent| match parent.value {
            crate::json::Json::Null => Ok(None),
            _ => parent.str().map(Some),
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(FoundingHero {
        key: hero.key.clone(),
        hero,
        dream,
        parents,
        fallen_at: match item.find("fallen_at")? {
            Some(_) => Some(id_at(item, "fallen_at", Place::find)?),
            None => None,
        },
    })
}

/// The heroes of the founding, in creation order, and each place's fallen.
pub struct Founded {
    /// In creation order; ids are indices.
    pub heroes: Vec<Hero>,
    /// Per place, the heroes who died there.
    pub fallen: Vec<Vec<HeroId>>,
}

/// Found the household exactly as `household.json` says (SPEC §4).
///
/// The file has already passed its schema; what can fail here is a reference
/// between heroes — a parent, a lost hero, a bond — naming a key that is not a
/// founding hero, or a pair bonded twice.
pub fn found(content: &Content) -> Result<Founded, String> {
    let founding = &content.founding;
    let id_of = |key: &str| -> Result<HeroId, String> {
        founding
            .heroes
            .iter()
            .position(|h| h.key == key)
            .ok_or_else(|| format!("household.json names {key:?}, which is not a founding hero"))
    };
    let mut heroes: Vec<Hero> = founding.heroes.iter().map(|f| f.hero.clone()).collect();
    let mut fallen = vec![Vec::new(); Place::ALL.len()];
    for (id, authored) in founding.heroes.iter().enumerate() {
        for (slot, parent) in authored.parents.iter().enumerate().take(2) {
            heroes[id].parents[slot] = parent.as_deref().map(id_of).transpose()?;
        }
        if let Some(dream) = &authored.dream {
            let (setup, lost_pronoun) = match &dream.setup {
                None => (None, None),
                Some((place, tag, lost)) => {
                    let lost = id_of(lost)?;
                    let setup = Setup {
                        place: *place,
                        tag: *tag,
                        lost: Some(lost),
                    };
                    (Some(setup), Some(founding.heroes[lost].hero.pronoun))
                }
            };
            let mut built = Dream::build(content, dream.kind, setup, lost_pronoun)?;
            built.advance_to_stage(dream.stage);
            heroes[id].dream = Some(built);
        }
        if let Some(place) = authored.fallen_at {
            fallen[place.index()].push(id);
            heroes[id].death_place = Some(place);
        }
    }
    for (hero, other, kind, since) in &founding.bonds {
        let (a, b) = (id_of(hero)?, id_of(other)?);
        if a == b || heroes[a].bond_to(b).is_some() {
            return Err(format!(
                "household.json bonds {hero:?} and {other:?} twice, or to themselves"
            ));
        }
        let mirror = crate::constants::bond_mirror(*kind);
        for (from, to, kind) in [(a, b, *kind), (b, a, mirror)] {
            heroes[from].bonds.push(Bond {
                kind,
                other: to,
                since: *since,
                taught: false,
                shared_successes: 0,
            });
        }
    }
    for key in &founding.dead_at_start {
        let id = id_of(key)?;
        if heroes[id].fate != Fate::Dead {
            return Err(format!(
                "{key:?} is listed dead at start and its fate is not DEAD"
            ));
        }
        heroes[id].grieved = true;
        heroes[id].bequest_decided = true;
    }
    Ok(Founded { heroes, fallen })
}
