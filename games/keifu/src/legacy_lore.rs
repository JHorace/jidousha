//! `legacies.json`, typed: what a fulfilled dream leaves (SPEC §14.1) — the blade
//! names, the three heirlooms, the three blessings and their effects, the tale
//! titles, and the sheet's promises.
//!
//! Every number the file carries that CONSTANTS.md or SPEC §14.1 also states — the
//! heirloom bonus and aptitude, each blessing's reach and power, the blade count —
//! is checked against it here, and a disagreement stops the game at startup.

use crate::constants::{
    BLADE_NAMES, BROAD_BLESSING_POWER, LEGACY_HEIRLOOM_BONUS, NARROW_BLESSING_POWER,
};
use crate::content::{id_at, strings, text, text_at};
use crate::ids::{Aptitude, LegacyKind, Tag};
use crate::json::{At, SchemaError};

/// An heirloom a dream forges, as authored (`legacies.json` `heirlooms`).
pub struct HeirloomLore {
    /// The name template ("%'s road-book"); the blade's is descriptive, unused.
    pub name: String,
    /// The original's sprite name.
    pub sprite: String,
    /// What it adds to.
    pub aptitude: Aptitude,
    /// How much.
    pub bonus: i32,
    /// "Forged by % at Emberfall, ..." (fulfiller's full name, year).
    pub provenance: String,
}

/// A blessing's reach, as authored: a fixed tag, the setup place, or everywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeLore {
    /// Against this tag.
    AgainstTag(Tag),
    /// At the avenged (setup) place.
    AtSetupPlace,
    /// On every quest.
    Everywhere,
}

/// A blessing a dream leaves, as authored (`legacies.json` `blessings`).
pub struct BlessingLore {
    /// "%'s rest" (the dreamer's first name).
    pub title: String,
    /// Where it applies.
    pub scope: ScopeLore,
    /// How much.
    pub power: i32,
}

/// `legacies.json`.
pub struct LegacyLore {
    /// "If it is ever done, ..." by `LegacyKind`.
    pub promises: Vec<String>,
    /// "+% % on quests."
    pub heirloom_effect: String,
    /// The ten blade names, in forging order.
    pub blade_names: Vec<String>,
    /// FORGE_A_BLADE's, WALK_EVERY_ROAD's, and every other heirloom dream's.
    pub heirlooms: [HeirloomLore; 3],
    /// QUIET_THE_BARROW's, AVENGE_THE_LOST's, and every other blessing dream's.
    pub blessings: [BlessingLore; 3],
    /// "+% against %", "+% at %", "+% on every quest".
    pub blessing_effects: [String; 3],
    /// SEE_THE_SEA's, ROOF_OF_THE_WORLD's, KNOWN_AT_COURT's, and every other tale.
    pub tale_titles: [String; 4],
}

fn read_heirloom(at: &At<'_>, key: &str, aptitude: Aptitude) -> Result<HeirloomLore, SchemaError> {
    let item = at.key(key)?;
    let lore = HeirloomLore {
        name: text(&item, "name")?,
        sprite: text(&item, "sprite")?,
        aptitude: id_at(&item, "aptitude", Aptitude::find)?,
        bonus: item.key("bonus")?.int()?,
        provenance: text(&item, "provenance")?,
    };
    if lore.aptitude != aptitude || lore.bonus != LEGACY_HEIRLOOM_BONUS {
        return Err(item.reject(format!(
            "{} +{}; SPEC §14.1 and CONSTANTS.md §11 say {} +{LEGACY_HEIRLOOM_BONUS}",
            lore.aptitude.id(),
            lore.bonus,
            aptitude.id()
        )));
    }
    Ok(lore)
}

fn read_blessing(
    at: &At<'_>,
    key: &str,
    want: (ScopeLore, i32),
) -> Result<BlessingLore, SchemaError> {
    let item = at.key(key)?;
    let scope = match text(&item, "scope")?.as_str() {
        "AGAINST_TAG" => ScopeLore::AgainstTag(id_at(&item, "tag", Tag::find)?),
        "AT_PLACE" if text(&item, "place")? == "SETUP_PLACE" => ScopeLore::AtSetupPlace,
        "EVERYWHERE" => ScopeLore::Everywhere,
        other => return Err(item.reject(format!("scope {other:?} is not one SPEC §14.1 names"))),
    };
    let lore = BlessingLore {
        title: text(&item, "title")?,
        scope,
        power: item.key("power")?.int()?,
    };
    if (lore.scope, lore.power) != want {
        return Err(item.reject(format!(
            "{:?} {}; SPEC §14.1 and CONSTANTS.md §11 say {:?} {}",
            lore.scope, lore.power, want.0, want.1
        )));
    }
    Ok(lore)
}

/// `legacies.json`, checked against SPEC §14.1 and CONSTANTS §11.
pub fn read_legacies(at: &At<'_>) -> Result<LegacyLore, SchemaError> {
    let promises = at.key("promises")?;
    let heirlooms = at.key("heirlooms")?;
    let blessings = at.key("blessings")?;
    let effects = at.key("blessing_effects")?;
    let tales = at.key("tale_titles")?;
    let blade_names = strings(at, "blade_names")?;
    if blade_names.len() != BLADE_NAMES {
        return Err(at.reject(format!(
            "{} blade names; CONSTANTS.md §11 says {BLADE_NAMES}",
            blade_names.len()
        )));
    }
    Ok(LegacyLore {
        promises: LegacyKind::ALL
            .iter()
            .map(|kind| text(&promises, kind.id()))
            .collect::<Result<_, _>>()?,
        heirloom_effect: text_at(at, "heirloom_effect")?,
        blade_names,
        heirlooms: [
            read_heirloom(&heirlooms, "FORGE_A_BLADE", Aptitude::Might)?,
            read_heirloom(&heirlooms, "WALK_EVERY_ROAD", Aptitude::Wits)?,
            read_heirloom(&heirlooms, "otherwise", Aptitude::Spirit)?,
        ],
        blessings: [
            read_blessing(
                &blessings,
                "QUIET_THE_BARROW",
                (ScopeLore::AgainstTag(Tag::Undead), NARROW_BLESSING_POWER),
            )?,
            read_blessing(
                &blessings,
                "AVENGE_THE_LOST",
                (ScopeLore::AtSetupPlace, NARROW_BLESSING_POWER),
            )?,
            read_blessing(
                &blessings,
                "otherwise",
                (ScopeLore::Everywhere, BROAD_BLESSING_POWER),
            )?,
        ],
        blessing_effects: [
            text(&effects, "AGAINST_TAG")?,
            text(&effects, "AT_PLACE")?,
            text(&effects, "EVERYWHERE")?,
        ],
        tale_titles: [
            text(&tales, "SEE_THE_SEA")?,
            text(&tales, "ROOF_OF_THE_WORLD")?,
            text(&tales, "KNOWN_AT_COURT")?,
            text(&tales, "otherwise")?,
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::DreamKind;

    #[test]
    fn every_legacy_is_the_one_the_spec_groups_it_with() {
        let content = crate::content::load().expect("the content loads");
        let kinds: Vec<LegacyKind> = DreamKind::ALL
            .iter()
            .map(|kind| content.dreams[kind.index()].legacy)
            .collect();
        use LegacyKind::{Blessing, Heirloom, Tale};
        assert_eq!(
            kinds,
            [
                Tale, Tale, Heirloom, Blessing, Blessing, Tale, Heirloom, Blessing, Heirloom
            ]
        );
        let legacies = &content.legacies;
        assert_eq!(legacies.blade_names.len(), 10);
        assert_eq!(legacies.blade_names[0], "Emberwake");
        assert_eq!(legacies.blade_names[9], "Undertow");
        assert_eq!(legacies.blessings.each_ref().map(|b| b.power), [2, 2, 1]);
        assert_eq!(legacies.heirlooms.each_ref().map(|h| h.bonus), [2, 2, 2]);
    }
}
