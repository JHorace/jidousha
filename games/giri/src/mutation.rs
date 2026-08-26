//! The mutation round: break every constant on purpose and check something
//! notices.
//!
//! A beat that passes under a mutated constant is a vacuous assertion, and this
//! is the only thing that says which of the two a check is. It runs in-process
//! because the constants are a resource rather than a `const` block: forty
//! candidate settings are forty `headless(..)` sims in one run, with no frames
//! recorded (FINDINGS G-004).

use crate::beats::CHAIN;
use crate::checks::Checks;
use crate::constants::{Field, Tuning};
use crate::contracts::battery;
use crate::judge::judge_world;
use crate::verify::play;

/// Break every constant on purpose and check the beats notice.
///
/// A beat that passes under a mutated constant is a vacuous assertion, and this
/// is the only thing that says which of the two a check is. It runs in-process
/// because the constants are a resource rather than a `const` block: forty
/// candidate settings are forty `headless(..)` sims in one run.
pub fn mutation_round(checks: &mut Checks) -> String {
    let mut noticed_by_beat = 0;
    let mut noticed_by_battery = 0;
    let mut missed: Vec<&'static str> = Vec::new();
    let mut only_contracts: Vec<String> = Vec::new();
    for field in Field::ALL.iter().copied() {
        let mutated = Tuning::SHIPPED.with(field, perturbation(field));
        let mut beat_failures = 0;
        for index in 0..CHAIN.len() {
            let mut probe = Checks::default();
            // Both rule sets (DESIGN §8e): the v1 constants are the
            // deterministic beats' to notice, the pressure and band constants
            // are the ladder beats' — a perturbed cutoff must break a band.
            let played = play(index, mutated, false);
            let deterministic = crate::verify::play_variant(
                index,
                mutated,
                crate::variant::VariantId::Deterministic,
                false,
            );
            if let Some(spec) = CHAIN.get(index) {
                judge_world(&mut probe, spec, &played, &mutated);
                judge_world(&mut probe, spec, &deterministic, &mutated);
            }
            beat_failures += probe.failures();
        }
        let mut probe = Checks::default();
        battery(&mut probe, &mutated);
        let battery_failures = probe.failures();
        let shipped = Tuning::SHIPPED.field(field);
        if beat_failures > 0 {
            noticed_by_beat += 1;
        } else if battery_failures > 0 {
            noticed_by_battery += 1;
            // Worth saying which check caught it: a constant no *beat* notices
            // is a constant the tutorial does not yet exercise, and that is a
            // fact about the chain rather than about the check.
            only_contracts.push(format!(
                "{} {shipped}->{}: {}",
                field.name(),
                perturbation(field),
                probe.first_failure().unwrap_or_default()
            ));
        } else {
            missed.push(field.name());
        }
        checks.require(
            beat_failures + battery_failures > 0,
            "a tuning constant can be changed without any check noticing",
            format!(
                "{} moved from {shipped} to {} and every beat and every contract still \
                 passed; a check that survives its own constant moving is not measuring it",
                field.name(),
                perturbation(field),
            ),
        );
    }
    let mut summary = format!(
        "{} of {} constants noticed ({noticed_by_beat} by a beat, {noticed_by_battery} by the \
         contract battery only)",
        noticed_by_beat + noticed_by_battery,
        Field::ALL.len(),
    );
    if !missed.is_empty() {
        summary.push_str(&format!("; nothing noticed {missed:?}"));
    }
    for line in &only_contracts {
        summary.push_str(&format!("\n    no beat exercises {line}"));
    }
    summary
}

/// What each constant is moved to, and why that value has to matter.
fn perturbation(field: Field) -> i32 {
    match field {
        // Nobody is ever desperate enough: Bob does not kill Steve.
        Field::KKill => 99,
        // Nobody is ever loyal enough: the report's own boundary moves.
        Field::KLoyal => 99,
        // The reluctant band vanishes: Tim's price is met without a flinch.
        Field::ReluctantBelow => 0,
        // A dark mark repels nobody: Tim stops refusing Bob.
        Field::MarkDark => 0,
        // A light mark pulls nobody.
        Field::MarkLight => 0,
        // The pot pulls nobody: Steve's willingness loses its share term.
        Field::PotPull => 0,
        // Reliability is unreachable: Bob's second clean job writes nothing.
        Field::ReliableAfter => 99,
        // A clean job leaves no bond behind.
        Field::BondGain => 0,
        // Witnesses hold nothing against the killer.
        Field::WitnessGrudge => 0,
        // Being bonded to the victim adds nothing.
        Field::BondedGrudge => 0,
        // Sitting out costs nothing, so no price is ever met.
        Field::DesperationRise => 0,
        // Profiting relieves nothing.
        Field::DesperationFall => 0,
        // Desperation falls through the floor into refusing clean work.
        Field::DesperationFloor => -99,
        // A reluctant join carries a riot in: beat 4's quiet seed erupts.
        Field::StrainReluctant => 99,
        // Eagerness buys total calm: beat 2's powder keg goes out.
        Field::StrainEager => 99,
        // Everybody with any margin is eager: beat 3's pressures drop.
        Field::EagerAbove => 0,
        // Hunger stops pressing: every ladder pressure loses its spine.
        Field::HungerWeight => 0,
        // The pot stops tempting.
        Field::OpportunityPull => 0,
        // Nothing ever reads uneasy: beat 3's band assert breaks.
        Field::UneasyAt => 99,
        // The powder keg is unreachable: beat 2's murder cannot happen.
        Field::PowderKegAt => 99,
        // The die vanishes: nobody ever rolls, so beat 2's murder is off.
        Field::OccurrenceDie => 0,
        // The roll forgives everything: same effect, other constant.
        Field::OccurrenceCalm => 12,
        // The fraction pins to the drawer's ceiling: the pot is destroyed
        // whole, and the battery's fixed-seed sabotage arithmetic breaks.
        Field::SabotageLoss => 99,
    }
}
