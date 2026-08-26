//! The committed presets: named constants sets, as data (DESIGN §8b tier 1).
//!
//! **Tier 1 of DESIGN §8b is "different numbers: not a variant".** A preset is
//! a `Tuning` with a name on it and nothing else — no flag, no binary, no
//! branch anywhere that names a preset. Adding one is adding a row to `PRESETS`
//! below, and the drawer grows a button for it because it walks this table
//! rather than a list of its own (UI.md §12).
//!
//! `DEFAULT` is `Tuning::SHIPPED` by reference rather than by transcription:
//! two spellings of the shipped set is one spelling that can go stale, and the
//! four tutorial beats are authored against the one in `constants.rs`.
//!
//! **Where the other two came from.** `CUTTHROAT` and `GENTLE` are the two
//! starting points the approved mockup offers, carried over term for term. The
//! mockup's eight names are a subset of this module's ten, in its own spelling:
//! `d_profit` is `desperation_fall`, `d_wait` is `desperation_rise`, `r_bond`
//! is `bond_gain`, `r_grudge` is `witness_grudge` and `i_kill` is
//! `infamy_per_kill`. Two constants the mockup has no term for are settled
//! here, and both follow the shipped set's own relations: `bonded_grudge`
//! matches `witness_grudge` (in `SHIPPED` they are equal — a bond doubles a
//! grudge rather than scaling it independently), and `desperation_floor` stays
//! at 0, which is the only value at which a character at the floor still takes
//! clean work (DESIGN §3.2).
//!
//! The mockup's own `DEFAULT` column is *not* carried over: it is the mockup's
//! toy set, the beats are authored against `Tuning::SHIPPED`, and a `DEFAULT`
//! button that did not restore the shipped values would be a button that lies.

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
    // Cheap lives, long memories: betrayal starts two points of desperation
    // sooner, loyalty has to be twice as strong to stop it, a paid job relieves
    // almost nothing, and everybody who watched holds it against you.
    Preset {
        name: "CUTTHROAT",
        tuning: Tuning {
            k_inf: 1,
            k_kill: 4,
            k_loyal: 4,
            bond_gain: 1,
            infamy_per_kill: 3,
            witness_grudge: 4,
            bonded_grudge: 4,
            desperation_rise: 2,
            desperation_fall: 1,
            desperation_floor: 0,
        },
    },
    // Room to breathe: nobody is desperate enough to turn until 8, a job pays
    // the need down three points, bonds form twice as fast, and a known face
    // costs three times as much willingness to stand next to.
    Preset {
        name: "GENTLE",
        tuning: Tuning {
            k_inf: 3,
            k_kill: 8,
            k_loyal: 2,
            bond_gain: 2,
            infamy_per_kill: 1,
            witness_grudge: 2,
            bonded_grudge: 2,
            desperation_rise: 1,
            desperation_fall: 3,
            desperation_floor: 0,
        },
    },
];

/// The preset with this name, if there is one.
pub fn find(name: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|preset| preset.name == name)
}
