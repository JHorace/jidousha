//! `lore.json`: the world's fixed vocabulary — aptitudes, tags, places, phases,
//! vocations, pronouns, seasons, and the words the text conventions assemble.

use crate::constants::{PHASE_ADJUSTMENT, PHASE_FROM_AGE};
use crate::content::{strings, table, text, text_at};
use crate::ids::{Aptitude, Phase, Place, Tag};
use crate::json::{At, SchemaError};

/// A tag's words.
pub struct TagLore {
    /// "Water".
    pub title: String,
    /// "deep water".
    pub noun: String,
}

/// A place's words and tags.
pub struct PlaceLore {
    /// "The Barrow".
    pub title: String,
    /// "the Barrow".
    pub name: String,
    /// What a quest there carries.
    pub tags: Vec<Tag>,
}

/// A phase's words.
pub struct PhaseLore {
    /// "Elder".
    pub title: String,
    /// "an elder".
    pub telling: String,
    /// "{He} teaches well, ..." with brace placeholders.
    pub note: String,
}

/// A pronoun's three forms.
pub struct PronounLore {
    /// he / she.
    pub subject: String,
    /// him / her.
    pub object: String,
    /// his / her.
    pub possessive: String,
}

/// `lore.json`.
pub struct Lore {
    /// Aptitude titles, by `Aptitude`.
    pub aptitudes: Vec<String>,
    /// By `Tag`.
    pub tags: Vec<TagLore>,
    /// By `Place`.
    pub places: Vec<PlaceLore>,
    /// By `Phase`.
    pub phases: Vec<PhaseLore>,
    /// Vocation titles, by `Vocation`.
    pub vocations: Vec<String>,
    /// By `Pronoun`.
    pub pronouns: Vec<PronounLore>,
    /// By calendar month.
    pub seasons: Vec<String>,
    /// 0..=12 as words.
    pub count_words: Vec<String>,
    /// "% times".
    pub count_words_beyond: String,
    /// "before the first year".
    pub year_before_first: String,
    /// "in the last summer".
    pub year_last_summer: String,
    /// "in year %".
    pub year_in: String,
    /// "No one".
    pub party_of_no_one: String,
    /// "no one yet".
    pub name_list_empty: String,
    /// ", ".
    pub name_list_separator: String,
    /// " and ".
    pub name_list_last_separator: String,
    /// `phase_effect_fragments`, by key.
    pub phase_effect: Vec<(String, String)>,
}

pub fn read_lore(at: &At<'_>) -> Result<Lore, SchemaError> {
    let phases = table(at, "phases", Phase::ALL, Phase::id)?;
    for (index, phase) in phases.iter().enumerate() {
        let from = phase.key("from_age")?.int()?;
        let adjustment = phase.key("adjustment")?;
        let mut row = [0; 3];
        for aptitude in Aptitude::ALL {
            row[aptitude.index()] = adjustment.key(aptitude.id())?.int()?;
        }
        if from != PHASE_FROM_AGE[index] || row != PHASE_ADJUSTMENT[index] {
            return Err(phase.reject(format!(
                "from_age {from} and adjustment {row:?} disagree with CONSTANTS.md §2 \
                 ({} and {:?})",
                PHASE_FROM_AGE[index], PHASE_ADJUSTMENT[index]
            )));
        }
    }
    let year_tellings = at.key("year_tellings")?;
    let pronouns = at.key("pronouns")?;
    Ok(Lore {
        aptitudes: titles(table(at, "aptitudes", Aptitude::ALL, Aptitude::id)?)?,
        tags: table(at, "tags", Tag::ALL, Tag::id)?
            .iter()
            .map(|t| {
                Ok(TagLore {
                    title: text(t, "title")?,
                    noun: text(t, "noun")?,
                })
            })
            .collect::<Result<_, SchemaError>>()?,
        places: table(at, "places", Place::ALL, Place::id)?
            .iter()
            .map(|p| {
                let tags = strings(p, "tags")?
                    .iter()
                    .map(|id| Tag::find(id).ok_or_else(|| p.reject(format!("tag {id:?}"))))
                    .collect::<Result<_, _>>()?;
                Ok(PlaceLore {
                    title: text(p, "title")?,
                    name: text(p, "name")?,
                    tags,
                })
            })
            .collect::<Result<_, SchemaError>>()?,
        phases: phases
            .iter()
            .map(|p| {
                Ok(PhaseLore {
                    title: text(p, "title")?,
                    telling: text(p, "telling")?,
                    note: text(p, "note")?,
                })
            })
            .collect::<Result<_, SchemaError>>()?,
        vocations: titles(table(
            at,
            "vocations",
            crate::ids::Vocation::ALL,
            crate::ids::Vocation::id,
        )?)?,
        pronouns: crate::ids::Pronoun::ALL
            .iter()
            .map(|p| {
                let forms = pronouns.key(p.id())?;
                Ok(PronounLore {
                    subject: text(&forms, "subject")?,
                    object: text(&forms, "object")?,
                    possessive: text(&forms, "possessive")?,
                })
            })
            .collect::<Result<_, SchemaError>>()?,
        seasons: strings(at, "seasons")?,
        count_words: strings(at, "count_words")?,
        count_words_beyond: text_at(at, "count_words_beyond")?,
        year_before_first: text(&year_tellings, "before_first_year")?,
        year_last_summer: text(&year_tellings, "last_summer")?,
        year_in: text(&year_tellings, "in_year")?,
        party_of_no_one: text_at(at, "party_of_no_one")?,
        name_list_empty: text_at(at, "name_list_empty")?,
        name_list_separator: text_at(at, "name_list_separator")?,
        name_list_last_separator: text_at(at, "name_list_last_separator")?,
        phase_effect: at
            .key("phase_effect_fragments")?
            .entries()?
            .into_iter()
            .filter(|(key, _)| !key.starts_with('_'))
            .map(|(key, value)| Ok((key, value.str()?)))
            .collect::<Result<_, SchemaError>>()?,
    })
}

fn titles(items: Vec<At<'_>>) -> Result<Vec<String>, SchemaError> {
    items.iter().map(|item| text(item, "title")).collect()
}
