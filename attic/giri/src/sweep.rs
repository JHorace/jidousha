//! Distribution sweeps (DESIGN.md §8f): the ladder's statistics, asserted as
//! bands.
//!
//! Two hundred seeds per authored scenario, resolved as pure function calls —
//! no sims, no frames — tallying how often anybody betrays and which rungs
//! land. The assertions are **bands**, not exact counts (a seeded distribution
//! is a distribution): a calm party betrays rarely, a powder-keg party
//! betrays in most runs, skims dominate the severities, murders are rare —
//! with **one exception stated exactly**: *zero* murders below the powder-keg
//! floor, over every event of every scenario, because that claim is
//! structural (`ladder::available`) and a band there would be a hole in the
//! design's one hard promise.
//!
//! Each scenario also asserts its band chip deterministically — the same
//! `pressure::party_band` the strip draws — which is what lets the mutation
//! round see a perturbed cutoff without paying for a sweep per perturbation.

use jidousha::prelude::*;

use crate::beats::Dungeon;
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::ladder::Rung;
use crate::model::Social;
use crate::pressure::{self, Band};
use crate::resolve::resolve;
use crate::traits::TraitId;
use crate::variant::VariantId;

/// How many seeds one scenario sweeps.
pub const SEEDS: u64 = 200;

/// One authored sweep scenario: a roster, a job, the band its chip must read,
/// and the band its betrayal rate must land in (percent, inclusive).
struct Scenario {
    name: &'static str,
    rows: &'static [(&'static str, i32, &'static [TraitId], &'static [MarkId])],
    edges: &'static [(usize, usize, i32)],
    pot: i32,
    cut: i32,
    band: Band,
    betrayed_pct: (u64, u64),
}

use crate::traits::MarkId;

/// The authored scenarios. Each is built to sit in one band on purpose; the
/// pressures behind the bands are asserted per scenario, so a constant that
/// moves a scenario out of its band is caught deterministically too.
const SCENARIOS: &[Scenario] = &[
    // Comfortable margins, modest need, thin opportunity: pressure 3 a head.
    Scenario {
        name: "calm",
        rows: &[
            ("Ann", 2, &[], &[]),
            ("Ben", 2, &[], &[]),
            ("Cal", 2, &[], &[]),
        ],
        edges: &[],
        pot: 6,
        cut: 2,
        band: Band::Calm,
        betrayed_pct: (2, 40),
    },
    // A reluctant pair pressed into service: strain is most of the danger.
    Scenario {
        name: "pressed",
        rows: &[("Dun", 1, &[], &[]), ("Eve", 1, &[], &[])],
        edges: &[],
        pot: 6,
        cut: 2,
        band: Band::Uneasy,
        betrayed_pct: (10, 65),
    },
    // Hungry, eager, and a pot worth turning on: the summit is reachable.
    Scenario {
        name: "powder-keg",
        rows: &[
            ("Fay", 8, &[], &[]),
            ("Gil", 8, &[], &[]),
            ("Hew", 8, &[], &[]),
        ],
        edges: &[],
        pot: 12,
        cut: 0,
        band: Band::PowderKeg,
        betrayed_pct: (70, 100),
    },
];

/// Build one scenario's world.
fn build(scenario: &Scenario) -> (Social, Vec<Entity>, Dungeon) {
    let (world, ids) = crate::contracts::bench(scenario.rows, scenario.edges);
    let social = Social::read(&world.view());
    let job = crate::contracts::bench_job(scenario.rows.len(), scenario.pot, scenario.cut);
    (social, ids, job)
}

/// Run the sweeps, assert the bands, and hand back the report lines.
pub fn run(checks: &mut Checks, tuning: &Tuning) -> String {
    let mut lines: Vec<String> = vec![format!(
        "  sweeps: {} seeds each (0..{}), variant {}, constants {}",
        SEEDS,
        SEEDS,
        VariantId::default().key(),
        tuning.stamp(),
    )];
    let mut rung_totals: Vec<(Rung, u64)> = vec![
        (Rung::Skim, 0),
        (Rung::Abandon, 0),
        (Rung::Sabotage, 0),
        (Rung::Murder, 0),
    ];
    let mut events_total: u64 = 0;
    let mut murders_below_floor: u64 = 0;
    let mut murders_at_all: u64 = 0;

    for scenario in SCENARIOS {
        let (social, ids, job) = build(scenario);
        // The chip's band, deterministically — the same numbers the rolls
        // below consume, which is the one-source claim itself.
        let pressures = pressure::party(&social, tuning, &ids, &job);
        let band = pressure::party_band(tuning, &pressures);
        checks.require(
            band == scenario.band,
            "a sweep scenario's band chip does not read what it was authored to",
            format!(
                "{}: pressures {:?} read {band:?}, authored {:?} - a band cutoff or a \
                 pressure constant moved",
                scenario.name,
                pressures.iter().map(|p| p.total).collect::<Vec<_>>(),
                scenario.band
            ),
        );

        let mut betrayed_runs: u64 = 0;
        let mut rungs_here: Vec<(Rung, u64)> = rung_totals.iter().map(|(r, _)| (*r, 0)).collect();
        for seed in 0..SEEDS {
            let mut rng = Rng::from_seed(seed);
            let outcome = resolve(&social, tuning, VariantId::Ladder, &job, &ids, &mut rng);
            if !outcome.rungs.is_empty() {
                betrayed_runs += 1;
                checks.require(
                    events_within_one_per_member(&outcome.rungs, &ids),
                    "a sweep run gave one member two betrayal events",
                    format!("{} at seed {seed}", scenario.name),
                );
            }
            for event in &outcome.rungs {
                events_total += 1;
                for (rung, count) in rungs_here.iter_mut() {
                    if *rung == event.rung {
                        *count += 1;
                    }
                }
                if event.rung == Rung::Murder {
                    murders_at_all += 1;
                    if event.pressure.total < tuning.powder_keg_at {
                        murders_below_floor += 1;
                    }
                }
            }
        }
        for (rung, count) in &rungs_here {
            for (total_rung, total) in rung_totals.iter_mut() {
                if total_rung == rung {
                    *total += count;
                }
            }
        }
        let pct = betrayed_runs * 100 / SEEDS;
        let (low, high) = scenario.betrayed_pct;
        checks.require(
            (low..=high).contains(&pct),
            "a sweep scenario's betrayal rate left its band",
            format!(
                "{}: {betrayed_runs} of {SEEDS} runs betrayed ({pct}%), authored band \
                 {low}%-{high}% - a band, not an exact count, because a seeded \
                 distribution is a distribution",
                scenario.name
            ),
        );
        lines.push(format!(
            "    {} ({:?}): {betrayed_runs}/{SEEDS} runs betrayed ({pct}%) - {}",
            scenario.name,
            band,
            rungs_here
                .iter()
                .map(|(rung, count)| format!("{} {count}", rung.def().name))
                .collect::<Vec<_>>()
                .join(", "),
        ));
    }

    // Skims dominate the severities overall (DESIGN §8f) - the ladder's
    // bottom rung is its common one, which is what teaches the distribution.
    let skims = rung_totals
        .iter()
        .find(|(rung, _)| *rung == Rung::Skim)
        .map_or(0, |(_, count)| *count);
    checks.require(
        events_total > 0 && skims * 2 >= events_total,
        "skims do not dominate the severity distribution",
        format!(
            "{skims} skims of {events_total} events; the ladder's base weights make the \
             small betrayal the common one"
        ),
    );
    // Murders are rare - a band.
    checks.require(
        murders_at_all * 100 <= events_total * 15,
        "murders are not rare",
        format!("{murders_at_all} murders of {events_total} events; the summit is the rare rung"),
    );
    // And the powder-keg sweep must actually reach the summit sometimes, or
    // the murder path is dead code wearing a probability.
    checks.require(
        murders_at_all > 0,
        "no sweep ever reached a murder",
        format!(
            "{events_total} events over {} scenarios and none was a murder; the powder-keg \
             scenario exists to reach the summit",
            SCENARIOS.len()
        ),
    );
    // **Exactly zero murders below the floor** (DESIGN §8f): exact, not
    // statistical — the one claim the sweep states as a count.
    checks.require(
        murders_below_floor == 0,
        "a murder landed below the powder-keg floor",
        format!(
            "{murders_below_floor} murder(s) at pressure under {} across every sweep; the \
             gate is structural and this must be exactly zero",
            tuning.powder_keg_at
        ),
    );
    lines.push(format!(
        "    overall: {events_total} events - {} - murders below floor 0 (exact)",
        rung_totals
            .iter()
            .map(|(rung, count)| format!("{} {count}", rung.def().name))
            .collect::<Vec<_>>()
            .join(", "),
    ));
    lines.join("\n")
}

/// At most one event per member (DESIGN §8b's rule, held across every seed).
fn events_within_one_per_member(events: &[crate::ladder::RungEvent], members: &[Entity]) -> bool {
    members
        .iter()
        .all(|member| events.iter().filter(|event| event.who == *member).count() <= 1)
}
