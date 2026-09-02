//! Pressure: one number per member, and it cannot lie (DESIGN.md §7a).
//!
//! At resolution each party member has **pressure** — an integer computed from
//! already-visible state: the strain their door margin carries in, their
//! desperation, their traits' biases, and the opportunity the pot is. **The
//! band chip and the occurrence roll consume the same numbers this file
//! computes** — `of` is the one function, the party strip's chip and the
//! ladder's rolls both call it, and that single source is what makes the
//! foreshadowing unable to lie (invariant 2). A band computed from a second
//! copy of this arithmetic is the failure mode this file exists to prevent.
//!
//! **The margin's reader** (DESIGN §6: "the margin is no longer discarded at
//! the door"): the strain component maps the willingness margin — recomputed
//! here through the same `willingness` the door and the preview call, so it is
//! the stored answer, not a second opinion — onto named constants: reluctant
//! carries `strain_reluctant` in, eager leaves `strain_eager` outside,
//! comfortable carries nothing.
//!
//! **Deterministic, all of it.** No `Rng` is read here; pressure is what the
//! roll is rolled *against*, and the same inputs produce the same pressures
//! under any seed.

use jidousha::prelude::*;

use crate::beats::Dungeon;
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::model::{Social, share_each};
use crate::traits::TraitId;
use crate::willing::willingness;

/// What a trait's pressure bias fires on (DESIGN §7a: "greedy under a fat pot,
/// vengeful beside a grudge").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BiasKind {
    /// The promised share reaches this many gold.
    FatPot(i32),
    /// The member holds a grudge (negative regard) toward somebody in the
    /// party.
    Grudge,
}

/// One trait's pressure bias — data the pressure function reads, never a
/// branch in it (the §4/§8b discipline, applied a fourth time).
#[derive(Clone, Copy, Debug)]
pub struct PressureBias {
    /// Whose trait.
    pub trait_id: TraitId,
    /// What it fires on.
    pub kind: BiasKind,
    /// What it adds.
    pub delta: i32,
}

/// The pressure-bias table. Tuning content, like the reaction table.
pub const PRESSURES: &[PressureBias] = &[
    // The pot weighs on the greedy: a fat share is a temptation, not only a
    // pull (DESIGN §4's P2 register for greedy).
    PressureBias {
        trait_id: TraitId::Greedy,
        kind: BiasKind::FatPot(3),
        delta: 2,
    },
    // The vengeful beside a grudge are a hazard to the party they joined.
    PressureBias {
        trait_id: TraitId::Vengeful,
        kind: BiasKind::Grudge,
        delta: 2,
    },
];

/// One member's pressure, with every component named — the numbers the chip,
/// the roll and the narration all read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pressure {
    /// Whose.
    pub who: Entity,
    /// The door margin the strain component was mapped from.
    pub margin: i32,
    /// The strain component: reluctant carries pressure in, eager leaves it
    /// outside.
    pub strain: i32,
    /// Desperation times `hunger_weight`.
    pub hunger: i32,
    /// The trait biases that fired.
    pub traits: i32,
    /// Gold gained if one fewer split, times `opportunity_pull`.
    pub opportunity: i32,
    /// The sum, floored at zero — what the roll is rolled against.
    pub total: i32,
}

/// The named margin-to-strain mapping (DESIGN §7a): reluctant (or pushed
/// negative since the door) -> `+strain_reluctant`; eager -> `-strain_eager`;
/// comfortable -> 0.
pub fn strain_of(tuning: &Tuning, margin: i32) -> i32 {
    if margin < tuning.reluctant_below {
        tuning.strain_reluctant
    } else if margin > tuning.eager_above {
        -tuning.strain_eager
    } else {
        0
    }
}

/// **Pressure** (DESIGN §7a) — the one function.
///
/// The margin comes through `willingness` — the same call the door, the strip
/// and the send gate make, on the same snapshot — so the chip before SEND and
/// the roll at resolution read identical numbers by construction.
pub fn of(
    social: &Social,
    tuning: &Tuning,
    who: Entity,
    party: &[Entity],
    job: &Dungeon,
) -> Pressure {
    let margin = willingness(social, tuning, who, party, Some(job)).margin;
    let strain = strain_of(tuning, margin);
    let hunger = social.desperation(who) * tuning.hunger_weight;
    let share = share_each(
        job.pot,
        job.cut,
        i32::try_from(job.headcount).unwrap_or(i32::MAX),
    );
    let holds_grudge = social.members.iter().any(|member| {
        member.entity != who && party.contains(&member.entity) && {
            social.regard(who, member.entity) < 0
        }
    });
    let member_traits = social.traits(who);
    let traits = PRESSURES
        .iter()
        .filter(|bias| member_traits.contains(&bias.trait_id))
        .filter(|bias| match bias.kind {
            BiasKind::FatPot(at) => share >= at,
            BiasKind::Grudge => holds_grudge,
        })
        .map(|bias| bias.delta)
        .sum::<i32>();
    let count = i32::try_from(party.len()).unwrap_or(i32::MAX);
    let gain =
        (share_each(job.pot, job.cut, count - 1) - share_each(job.pot, job.cut, count)).max(0);
    let opportunity = gain * tuning.opportunity_pull;
    Pressure {
        who,
        margin,
        strain,
        hunger,
        traits,
        opportunity,
        total: (strain + hunger + traits + opportunity).max(0),
    }
}

/// Every party member's pressure, in roster order — what the chip's band is
/// the maximum of, and what the rolls walk.
pub fn party(social: &Social, tuning: &Tuning, members: &[Entity], job: &Dungeon) -> Vec<Pressure> {
    members
        .iter()
        .map(|who| of(social, tuning, *who, members, job))
        .collect()
}

/// The foreshadowing vocabulary (DESIGN §7a): named cutoffs on the party's
/// highest member pressure. The most dangerous person sets the mood.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Band {
    /// Every pressure under `uneasy_at`.
    Calm,
    /// The highest reaches `uneasy_at`.
    Uneasy,
    /// The highest reaches `powder_keg_at` — where murder becomes reachable,
    /// by the same constant.
    PowderKeg,
}

impl Band {
    /// The chip's word.
    pub fn word(self) -> &'static str {
        match self {
            Band::Calm => "calm",
            Band::Uneasy => "uneasy",
            Band::PowderKeg => "powder keg",
        }
    }
}

/// One pressure's band — the cutoffs are the constants the roll gates on.
pub fn band_of(tuning: &Tuning, pressure: i32) -> Band {
    if pressure >= tuning.powder_keg_at {
        Band::PowderKeg
    } else if pressure >= tuning.uneasy_at {
        Band::Uneasy
    } else {
        Band::Calm
    }
}

/// The party's band: its highest member pressure, banded.
pub fn party_band(tuning: &Tuning, pressures: &[Pressure]) -> Band {
    band_of(tuning, pressures.iter().map(|p| p.total).max().unwrap_or(0))
}

/// The pressure model, asked directly — every component and boundary, on
/// benches the tutorial never builds. Runs under the mutation round with every
/// constant perturbed, which is what makes each claim an instrument.
pub fn battery(checks: &mut Checks, tuning: &Tuning) {
    use crate::contracts::{bench, bench_job};

    // --- the strain mapping, at its boundaries ----------------------------
    for (label, margin, want) in [
        ("a negative margin", -1, tuning.strain_reluctant),
        (
            "a margin just under the reluctant boundary",
            tuning.reluctant_below - 1,
            tuning.strain_reluctant,
        ),
        (
            "a margin at the reluctant boundary",
            tuning.reluctant_below,
            0,
        ),
        ("a margin at the eager boundary", tuning.eager_above, 0),
        (
            "a margin past the eager boundary",
            tuning.eager_above + 1,
            -tuning.strain_eager,
        ),
    ] {
        let got = strain_of(tuning, margin);
        checks.require(
            got == want,
            "the margin-to-strain mapping moved off its named constants",
            format!("{label} ({margin}) maps to {got}, wanted {want}"),
        );
    }

    // --- the components, each alone ---------------------------------------
    // Hunger: a lone desperation, on a comfortable margin.
    let (world, ids) = bench(&[("Hungry", tuning.reluctant_below.max(0), &[], &[])], &[]);
    let social = crate::model::Social::read(&world.view());
    let job = bench_job(1, 0, 0);
    let lone = of(&social, tuning, ids[0], &ids, &job);
    checks.require(
        lone.hunger == social.desperation(ids[0]) * tuning.hunger_weight
            && lone.opportunity == 0
            && lone.traits == 0,
        "a lone member's pressure is not desperation times its weight",
        format!("the components came out {lone:?}"),
    );
    // Opportunity: the gain if one fewer split, exactly.
    let (world, ids) = bench(&[("A", 2, &[], &[]), ("B", 2, &[], &[])], &[]);
    let social = crate::model::Social::read(&world.view());
    let job = bench_job(2, 6, 2);
    let opp = of(&social, tuning, ids[0], &ids, &job);
    let gain = share_each(6, 2, 1) - share_each(6, 2, 2);
    checks.require(
        opp.opportunity == gain * tuning.opportunity_pull,
        "the opportunity term is not the betrayal gain times its pull",
        format!(
            "a pot of 6 less 2 between 2 has a gain of {gain} and the term came out {}",
            opp.opportunity
        ),
    );
    // The trait biases: greedy under a fat pot, vengeful beside a grudge.
    let (world, ids) = bench(
        &[
            ("Grim", 2, &[TraitId::Greedy], &[]),
            ("Sore", 2, &[TraitId::Vengeful], &[]),
            ("Flat", 2, &[], &[]),
        ],
        &[(1, 0, -1)],
    );
    let social = crate::model::Social::read(&world.view());
    let fat = bench_job(3, 12, 0);
    let greedy = of(&social, tuning, ids[0], &ids, &fat);
    let vengeful = of(&social, tuning, ids[1], &ids, &fat);
    let flat = of(&social, tuning, ids[2], &ids, &fat);
    checks.require(
        greedy.traits == 2 && vengeful.traits == 2 && flat.traits == 0,
        "the trait pressure biases are not the table's",
        format!(
            "under a fat pot with a grudge in the party, greedy {} vengeful {} plain {}",
            greedy.traits, vengeful.traits, flat.traits
        ),
    );
    // The floor: pressure never goes below zero.
    let (world, ids) = bench(&[("Glad", tuning.eager_above + 8, &[], &[])], &[]);
    let social = crate::model::Social::read(&world.view());
    let job = bench_job(1, 0, 0);
    let glad = of(&social, tuning, ids[0], &ids, &job);
    checks.require(
        glad.total >= 0,
        "a pressure total went below zero",
        format!("the eager loner came out {glad:?}; the floor is 0"),
    );

    // --- the bands sit exactly on their cutoffs ---------------------------
    for (pressure, want) in [
        (tuning.uneasy_at - 1, Band::Calm),
        (tuning.uneasy_at, Band::Uneasy),
        (tuning.powder_keg_at - 1, Band::Uneasy),
        (tuning.powder_keg_at, Band::PowderKeg),
    ] {
        let got = band_of(tuning, pressure);
        checks.require(
            got == want,
            "a band cutoff is not the constant that names it",
            format!("pressure {pressure} reads {got:?}, wanted {want:?}"),
        );
    }
    // The party band is the highest member's, not an average.
    let (world, ids) = bench(
        &[
            ("Still", tuning.reluctant_below.max(0), &[], &[]),
            (
                "Keg",
                tuning.powder_keg_at + tuning.occurrence_calm + 4,
                &[],
                &[],
            ),
        ],
        &[],
    );
    let social = crate::model::Social::read(&world.view());
    let job = bench_job(2, 0, 0);
    let pressures = party(&social, tuning, &ids, &job);
    let calm_one = pressures.first().map_or(0, |p| p.total);
    let keg_one = pressures.get(1).map_or(0, |p| p.total);
    checks.require(
        keg_one >= tuning.powder_keg_at
            && band_of(tuning, calm_one) != Band::PowderKeg
            && party_band(tuning, &pressures) == Band::PowderKeg,
        "the party band is not set by its most dangerous member",
        format!(
            "pressures {calm_one} and {keg_one} read {:?} together",
            party_band(tuning, &pressures)
        ),
    );
}
