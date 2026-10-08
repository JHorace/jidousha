//! `wanderers.json`, typed (SPEC §17.2, §17.4): a wanderer's ages, the three standings
//! the house's renown draws them from, and the dreams a newcomer can roll.
//!
//! Every number the file carries that CONSTANTS.md also states — the ages, each
//! standing's threshold and ranges, the teacher's age — is checked against it here,
//! and a disagreement stops the game at startup.

use crate::constants::{TEACHER_AGE_FOR_DREAM, WANDERER_AGES, WANDERER_STANDINGS};
use crate::content::{strings, text};
use crate::ids::DreamKind;
use crate::json::{At, SchemaError};

/// One standing (CONSTANTS §10): who a house of this renown draws.
pub struct Standing {
    /// The least house renown that draws it.
    pub renown_at_least: i32,
    /// The gift aptitude's range, inclusive.
    pub gift: (i32, i32),
    /// The other two aptitudes' range, inclusive.
    pub other: (i32, i32),
    /// "The house has a fair name, and drew a fair hand."
    pub telling: String,
}

/// `wanderers.json`.
pub struct WandererLore {
    /// The three standings, lowest threshold first.
    pub standings: Vec<Standing>,
    /// The rollable dreams, in index order (SPEC §17.4).
    pub dreams: Vec<DreamKind>,
}

/// Read `wanderers.json`, held to CONSTANTS §10.
pub fn read_wanderers(at: &At<'_>) -> Result<WandererLore, SchemaError> {
    let ages = (at.key("age_low")?.int()?, at.key("age_high")?.int()?);
    if ages != WANDERER_AGES {
        return Err(at.reject(format!(
            "ages {ages:?}; CONSTANTS.md §10 says {WANDERER_AGES:?}"
        )));
    }
    let teacher = at.key("teacher_dream_minimum_age")?.int()?;
    if teacher != TEACHER_AGE_FOR_DREAM {
        return Err(at.reject(format!(
            "teacher_dream_minimum_age {teacher}; CONSTANTS.md §10 says {TEACHER_AGE_FOR_DREAM}"
        )));
    }
    let items = at.key("standings")?.items()?;
    if items.len() != WANDERER_STANDINGS.len() {
        return Err(at.reject(format!("{} standings; CONSTANTS.md §10 has 3", items.len())));
    }
    let mut standings = Vec::new();
    for (item, want) in items.iter().zip(WANDERER_STANDINGS) {
        let row = (
            item.key("renown_at_least")?.int()?,
            item.key("gift_low")?.int()?,
            item.key("gift_high")?.int()?,
            item.key("other_low")?.int()?,
            item.key("other_high")?.int()?,
        );
        if row != want {
            return Err(item.reject(format!("standing {row:?}; CONSTANTS.md §10 says {want:?}")));
        }
        standings.push(Standing {
            renown_at_least: row.0,
            gift: (row.1, row.2),
            other: (row.3, row.4),
            telling: text(item, "telling")?,
        });
    }
    let dreams = strings(at, "dreams")?
        .iter()
        .map(|id| DreamKind::find(id).ok_or_else(|| at.reject(format!("dream {id:?}"))))
        .collect::<Result<Vec<_>, _>>()?;
    // SPEC §17.4: the eight wanderer dreams, every dream but AVENGE_THE_LOST.
    let eight: Vec<DreamKind> = DreamKind::ALL
        .iter()
        .copied()
        .filter(|kind| *kind != DreamKind::AvengeTheLost)
        .collect();
    if dreams != eight {
        return Err(at.reject(format!(
            "dreams {dreams:?}; SPEC §17.4 rolls the eight but AVENGE_THE_LOST, in order"
        )));
    }
    Ok(WandererLore { standings, dreams })
}

/// The standing house renown `renown` draws: the highest whose threshold it meets.
pub fn standing(lore: &WandererLore, renown: i32) -> &Standing {
    match lore
        .standings
        .iter()
        .rev()
        .find(|s| renown >= s.renown_at_least)
    {
        Some(standing) => standing,
        // INVARIANT: the lowest threshold is 0 (checked at load) and renown is floored at 0.
        None => panic!(
            "[keifu_x_inheritance_r3v1] house renown {renown} meets no wanderer standing\n  likely cause: renown \
             went below 0\n  fix: change renown only through House::add_renown"
        ),
    }
}
