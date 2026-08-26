//! Variants: how incompatible mechanics coexist (DESIGN.md §8b, tier 2) — and
//! the one module where variant selection happens.
//!
//! Two variants ship (DESIGN §8e): **ladder** (the default — seeded rolls over
//! the severity ladder) and **deterministic** (v1's betrayal rule, preserved
//! verbatim for comparison playtests — `model::betrayals` is still the v1
//! function, and this module calls it rather than reimplementing it). The rule
//! set is assembled here at chain start; nothing anywhere else branches on a
//! variant id — systems read the *data* this module hands them (`events`, and
//! the `foreshadows` flag the band chip keys off), never the id itself.
//!
//! The variant id is a **simulation input** exactly as the tuning constants
//! are: a resource, part of replay identity, stamped into every recording and
//! verify report, settable on the web with `?variant=` beside `?constants=`
//! and `?seed=`. Changing it mid-session restarts the chain — rule-set
//! assembly happens at chain start, so a new rule set is a new chain.

use jidousha::prelude::*;

use crate::beats::Dungeon;
use crate::constants::Tuning;
use crate::ladder::{self, Rung, RungEvent};
use crate::model::Social;
use crate::pressure::Pressure;

/// Which rule set the chain was started under.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VariantId {
    /// The seeded severity ladder (DESIGN §8) — the shipped default.
    #[default]
    Ladder,
    /// v1's deterministic betrayal rule, preserved for comparison playtests.
    Deterministic,
}

impl Resource for VariantId {}

impl VariantId {
    /// Every variant, in picker order.
    pub const ALL: &'static [VariantId] = &[VariantId::Ladder, VariantId::Deterministic];

    /// The id as stamps, reports and the `?variant=` parameter spell it.
    pub fn key(self) -> &'static str {
        match self {
            VariantId::Ladder => "ladder",
            VariantId::Deterministic => "deterministic",
        }
    }

    /// The variant a `?variant=` value names, if it names one.
    pub fn find(key: &str) -> Option<VariantId> {
        VariantId::ALL
            .iter()
            .copied()
            .find(|variant| variant.key().eq_ignore_ascii_case(key.trim()))
    }

    /// Whether this rule set telegraphs betrayal — what the band chip keys
    /// off. The ladder foreshadows (DESIGN §8: a UI obligation); the
    /// deterministic rule keeps v1's stance that the player does the
    /// arithmetic themselves, so it shows no chip.
    pub fn foreshadows(self) -> bool {
        match self {
            VariantId::Ladder => true,
            VariantId::Deterministic => false,
        }
    }
}

/// **The betrayal rule, selected** — the one `match` on the variant id.
///
/// Both arms answer in the same vocabulary (`RungEvent`), so resolution
/// consumes one list whichever rule produced it. The deterministic arm never
/// touches the `Rng` (its outcome is v1's, a pure function with no dice), and
/// its events carry `rolled: None` — which is what the narration keys off to
/// print v1's exact lines.
pub fn events(
    variant: VariantId,
    social: &Social,
    tuning: &Tuning,
    job: &Dungeon,
    party: &[Entity],
    pressures: &[Pressure],
    rng: &mut Rng,
) -> Vec<RungEvent> {
    match variant {
        VariantId::Ladder => ladder::roll_events(social, tuning, job, party, pressures, rng),
        VariantId::Deterministic => {
            crate::model::betrayals(social, tuning, party, job.pot, job.cut)
                .into_iter()
                .map(|betrayal| RungEvent {
                    who: betrayal.killer,
                    rung: Rung::Murder,
                    pressure: Pressure {
                        who: betrayal.killer,
                        margin: 0,
                        strain: 0,
                        hunger: 0,
                        traits: 0,
                        opportunity: 0,
                        total: 0,
                    },
                    rolled: None,
                    die: 0,
                    victim: Some(betrayal.victim),
                    victim_regard: betrayal.regard,
                    v1: Some(betrayal),
                })
                .collect()
        }
    }
}
