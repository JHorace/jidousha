//! The committed presets: named constants sets, as data (giri's tier-1
//! variants discipline, carried into the fork).
//!
//! A preset is a `Tuning` with a name on it and nothing else — no flag, no
//! binary, no branch anywhere that names a preset. Adding one is adding a row
//! to `PRESETS` below, and the drawer grows a button for it because it walks
//! this table rather than a list of its own.
//!
//! **Every preset is `Tuning::SHIPPED` with the rows it means moved**, by
//! functional update rather than by transcription: two spellings of the
//! shipped set is one spelling that can go stale, the fixed verify scripts are
//! authored against the one in `constants.rs`, and a constant added there
//! reaches every preset without this file being edited.

use crate::constants::Tuning;

/// One named constants set.
#[derive(Clone, Copy, Debug)]
pub struct Preset {
    /// What the drawer's button says. ASCII and short - the button is 120
    /// reference pixels wide.
    pub name: &'static str,
    /// The set it loads.
    pub tuning: Tuning,
}

/// Every committed preset, in the order the drawer draws them.
///
/// `DEFAULT` first, because a preset row's first job is to put back what the
/// game ships with after an experiment.
pub const PRESETS: &[Preset] = &[
    Preset {
        name: "DEFAULT",
        tuning: Tuning::SHIPPED,
    },
    // Hard country: roads are the only sane way anywhere — everything off
    // them costs half again or more, so routing decisions get louder.
    Preset {
        name: "MIRE",
        tuning: Tuning {
            plains_cost: 6,
            forest_cost: 11,
            rough_cost: 15,
            ..Tuning::SHIPPED
        },
    },
    // A faster wall clock: the same world at double pace, for a playtest
    // that wants to feel a whole day pass — the speeds double and the fast
    // forward doubles with them.
    Preset {
        name: "BRISK",
        tuning: Tuning {
            speed_1x: 2,
            speed_2x: 4,
            speed_4x: 8,
            ..Tuning::SHIPPED
        },
    },
];

/// The preset with this name, if there is one.
pub fn find(name: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|preset| preset.name == name)
}
