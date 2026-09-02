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
//! **Where the other two came from.** `CUTTHROAT` and `GENTLE` began as the
//! approved mockup's two starting points and were re-derived for v2's terms in
//! the P1 session: the constants the mockup had words for keep its intent
//! (cheap lives and long memories; room to breathe), and the five v2 constants
//! follow the same intent — a cutthroat world where marks weigh heavy, money
//! talks and a reputation for reliability is hard-won; a gentle one where a
//! light mark counts double and one clean job is enough to be somebody.

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
    // almost nothing, everybody who watched holds it against you, a dark mark
    // costs double, the pot shouts, and *reliable* takes three clean jobs.
    // The P2 half follows the same intent: pressed people crack sooner (a
    // reluctant join carries more in, the roll forgives nothing), the bands
    // trip earlier, and a sabotage guts the pot.
    Preset {
        name: "CUTTHROAT",
        tuning: Tuning {
            k_kill: 4,
            k_loyal: 4,
            reluctant_below: 3,
            mark_dark: 2,
            mark_light: 1,
            pot_pull: 2,
            reliable_after: 3,
            bond_gain: 1,
            witness_grudge: 4,
            bonded_grudge: 4,
            desperation_rise: 2,
            desperation_fall: 1,
            desperation_floor: 0,
            strain_reluctant: 4,
            strain_eager: 1,
            eager_above: 5,
            hunger_weight: 1,
            opportunity_pull: 2,
            uneasy_at: 3,
            powder_keg_at: 6,
            occurrence_die: 12,
            occurrence_calm: 0,
            sabotage_loss: 9,
        },
    },
    // Room to breathe: nobody is desperate enough to turn until 8, a job pays
    // the need down three points, bonds form twice as fast, a light mark
    // counts double, and one clean job is enough to be marked reliable. The
    // P2 half breathes too: eagerness buys real calm, the roll forgives more,
    // the powder keg is further away, and a sabotage stings instead of guts.
    Preset {
        name: "GENTLE",
        tuning: Tuning {
            k_kill: 8,
            k_loyal: 2,
            reluctant_below: 1,
            mark_dark: 1,
            mark_light: 2,
            pot_pull: 1,
            reliable_after: 1,
            bond_gain: 2,
            witness_grudge: 2,
            bonded_grudge: 2,
            desperation_rise: 1,
            desperation_fall: 3,
            desperation_floor: 0,
            strain_reluctant: 2,
            strain_eager: 3,
            eager_above: 3,
            hunger_weight: 1,
            opportunity_pull: 1,
            uneasy_at: 5,
            powder_keg_at: 10,
            occurrence_die: 12,
            occurrence_calm: 3,
            sabotage_loss: 3,
        },
    },
];

/// The preset with this name, if there is one.
pub fn find(name: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|preset| preset.name == name)
}
