//! The mutation round: break every constant on purpose and check something
//! notices.
//!
//! A script that passes under a mutated constant is a vacuous assertion, and
//! this is the only thing that says which of the two a check is. Each of the
//! eight constants is moved to a value nothing plausibly authors, and one of
//! the three instruments must complain: the exact-time order script (terrain
//! costs move arrival minutes), the pacing probes (the clock constants move
//! the tick-for-minute arithmetic), or the path battery (a cost that only a
//! route's literal sees).

use crate::checks::Checks;
use crate::constants::{Field, Tuning};
use crate::sweep::{self, Session, conduct};
use crate::verify;

/// Break every constant on purpose and check the run notices.
pub fn mutation_round(checks: &mut Checks) -> String {
    let mut noticed = 0;
    let mut missed: Vec<&'static str> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    for field in Field::ALL.iter().copied() {
        let mutated = Tuning::SHIPPED.with(field, perturbation(field));
        let mut probe = Checks::default();
        // The fast script: the whole order timeline at 4x. A stalled clock
        // (a zeroed speed) simply runs the cap out and fails the timeline,
        // which is a notice too.
        let script = sweep::speed_scripts().remove(1).1;
        let conducted = conduct(&Session::plain(mutated, &script, 25_000));
        sweep::judge_orders(&mut probe, &conducted, "mutated");
        sweep::judge_pacing(&mut probe, mutated, sweep::SHIPPED_PACING);
        verify::path_contracts_at(&mut probe, mutated);
        let shipped = Tuning::SHIPPED.field(field);
        if probe.failures() > 0 {
            noticed += 1;
            notes.push(format!(
                "{} {shipped}->{}: {} checks noticed",
                field.name(),
                perturbation(field),
                probe.failures()
            ));
        } else {
            missed.push(field.name());
        }
        checks.require(
            probe.failures() > 0,
            "a tuning constant can be changed without any check noticing",
            format!(
                "{} moved from {shipped} to {} and the order script, the pacing probes and \
                 the path battery all still passed; a check that survives its own constant \
                 moving is not measuring it",
                field.name(),
                perturbation(field),
            ),
        );
    }
    let mut summary = format!("{noticed} of {} constants noticed", Field::ALL.len());
    if !missed.is_empty() {
        summary.push_str(&format!("; nothing noticed {missed:?}"));
    }
    summary
}

/// What each constant is moved to, and why that value has to matter.
fn perturbation(field: Field) -> i64 {
    match field {
        // The road becomes a crawl: every all-road arrival minute moves.
        Field::RoadCost => 99,
        // Plains are free: routes and arrival sums both move.
        Field::PlainsCost => 0,
        // The forest is free: the Deep Cave slog stops costing.
        Field::ForestCost => 0,
        // The rough is free: only the path battery's rough crossing sees it,
        // which is why that literal exists.
        Field::RoughCost => 0,
        // A minute takes three times the ticks: the pacing probes break.
        Field::MinuteTicks => 99,
        // Each speed stops carrying time: its pacing probe reads zero.
        Field::Speed1x => 0,
        Field::Speed2x => 0,
        Field::Speed4x => 0,
    }
}
