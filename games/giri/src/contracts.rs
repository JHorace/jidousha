//! The questions a played beat never asks — the decision function and the
//! resolution on rosters built for the case.
//!
//! Three groups live across three files:
//!
//! - **this file** — the split at its edges, the roster-order betrayal, the
//!   surviving witness no tutorial beat produces, the mark the murder writes,
//!   the dungeon predicates, and the send gate's states. A run only tests the
//!   states it reaches, and the margins a correct game rests on are exactly
//!   the states it never reaches.
//! - **`judgment.rs`** — willingness v2's own battery: trait modifiers, the
//!   trait x mark table (attraction included), verdict boundaries, and the
//!   reasons vocabulary.
//! - **`door.rs`** — the door rule, both directions and both failures.
//!
//! The mutation round runs all three with every constant perturbed in turn;
//! a beat or contract that passes under a mutated constant is a vacuous
//! assertion, and the round is the only thing that says which a check is.

use jidousha::prelude::*;

use crate::beats::{QuestIcon, Requirement};
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::flow::assess;
use crate::model::{
    Character, CleanJobs, Desperation, Marks, RegardEdge, Social, Source, Traits, Wealth,
    betrayals, share_each,
};
use crate::resolve::resolve;
use crate::traits::{MarkId, TraitId};
use crate::variant::VariantId;

/// Resolve a bench under the deterministic variant — v1's rule, which is what
/// most of this battery asserts. The dice are handed in and never read: the
/// deterministic arm draws nothing, and `battery` proves that directly.
pub(crate) fn resolve_v1(
    social: &Social,
    tuning: &Tuning,
    dungeon: &crate::beats::Dungeon,
    party: &[Entity],
) -> crate::resolve::Resolution {
    let mut rng = Rng::from_seed(0);
    resolve(
        social,
        tuning,
        VariantId::Deterministic,
        dungeon,
        party,
        &mut rng,
    )
}

/// A roster built for one question: `(name, desperation, traits, marks)` in
/// roster order, then the edges between them by index.
pub(crate) fn bench(
    rows: &[(&'static str, i32, &'static [TraitId], &'static [MarkId])],
    edges: &[(usize, usize, i32)],
) -> (World, Vec<Entity>) {
    let mut world = World::new();
    let mut ids = Vec::new();
    for (roster_index, (name, desperation, traits, marks)) in rows.iter().enumerate() {
        let entity = world.spawn();
        world.insert(entity, Character { name, roster_index });
        world.insert(entity, Desperation(*desperation));
        world.insert(entity, Source("the bench"));
        world.insert(entity, Wealth(0));
        world.insert(entity, Traits(traits.to_vec()));
        world.insert(entity, Marks(marks.to_vec()));
        world.insert(entity, CleanJobs(0));
        ids.push(entity);
    }
    for (from, to, value) in edges.iter().copied() {
        let entity = world.spawn();
        world.insert(
            entity,
            RegardEdge {
                from: ids[from],
                to: ids[to],
                value,
            },
        );
    }
    (world, ids)
}

/// Set one bench character's clean-job count after the fact.
pub(crate) fn set_clean_jobs(world: &mut World, who: Entity, count: i32) {
    if let Some(jobs) = world.find_component_mut::<CleanJobs>(who) {
        jobs.0 = count;
    }
}

/// A dungeon for a bench, with everything visible and nothing required.
pub(crate) fn bench_job(headcount: usize, pot: i32, cut: i32) -> crate::beats::Dungeon {
    crate::beats::Dungeon {
        name: "the bench",
        blurb: "a roster built for one question",
        icon: QuestIcon::Cave,
        headcount,
        pot,
        cut,
        requires: Requirement::AnyParty,
    }
}

/// The bench a surviving witness needs: a killer past `K_kill`, a victim with
/// nothing protecting them, and a third the killer is loyal enough to spare.
///
/// `bonded_to_victim` is the one thing that differs between the two runs the
/// grudge assertions compare.
fn bench_kill(
    tuning: &Tuning,
    bonded_to_victim: bool,
) -> (Social, Vec<Entity>, crate::resolve::Resolution) {
    let mut edges = vec![(0usize, 2usize, tuning.k_loyal + 1)];
    if bonded_to_victim {
        edges.push((2, 1, 2));
    }
    let (world, ids) = bench(
        &[
            ("Kil", 8, &[], &[]),
            ("Vic", 0, &[], &[]),
            ("Wit", 0, &[], &[]),
        ],
        &edges,
    );
    let social = Social::read(&world.view());
    let outcome = resolve_v1(&social, tuning, &bench_job(3, 12, 0), &ids);
    (social, ids, outcome)
}

/// Ask the model the questions play cannot produce.
pub fn battery(checks: &mut Checks, tuning: &Tuning) {
    // --- the split, at its edges --------------------------------------
    for (pot, cut, survivors, want) in [
        (6, 2, 2, 2),
        (6, 2, 1, 4),
        (6, 2, 0, 0),
        (2, 6, 1, 0),
        (7, 0, 2, 3),
    ] {
        let got = share_each(pot, cut, survivors);
        checks.require(
            got == want,
            "the split is not the arithmetic the economy is",
            format!("pot {pot} less a cut of {cut} among {survivors} is {got}, wanted {want}"),
        );
    }

    // --- betrayal: roster order decides who kills whom ------------------
    //
    // Two characters, identical numbers, both past `K_kill` and both better off
    // alone. Nothing but the stated order separates them, and reversing it
    // reverses the outcome - which is the whole reason the order is stated.
    for reversed in [false, true] {
        let rows: &[(&'static str, i32, &'static [TraitId], &'static [MarkId])] = if reversed {
            &[("Second", 8, &[], &[]), ("First", 8, &[], &[])]
        } else {
            &[("First", 8, &[], &[]), ("Second", 8, &[], &[])]
        };
        let (world, ids) = bench(rows, &[]);
        let social = Social::read(&world.view());
        let done = betrayals(&social, tuning, &ids, 6, 2);
        let killer = done.first().map(|betrayal| social.name(betrayal.killer));
        checks.require(
            done.len() == 1 && killer == Some(rows[0].0),
            "betrayal is no longer evaluated in roster order",
            format!(
                "with the roster {:?}, {} killing(s) happened and the killer was {killer:?}; \
                 the first name in the roster evaluates first and the second is dead before \
                 its turn",
                rows.iter().map(|row| row.0).collect::<Vec<_>>(),
                done.len()
            ),
        );
    }
    // Each clause on its own, because a rule that fires when any one of three
    // holds passes every test where all three do.
    let (world, ids) = bench(
        &[
            ("Calm", tuning.k_kill - 1, &[], &[]),
            ("Other", 0, &[], &[]),
        ],
        &[],
    );
    let social = Social::read(&world.view());
    checks.require(
        betrayals(&social, tuning, &ids, 6, 2).is_empty(),
        "a character below K_kill betrayed anyway",
        format!(
            "desperation {} against a K_kill of {}",
            tuning.k_kill - 1,
            tuning.k_kill
        ),
    );
    let (world, ids) = bench(
        &[("Loyal", 9, &[], &[]), ("Friend", 0, &[], &[])],
        &[(0, 1, tuning.k_loyal)],
    );
    let social = Social::read(&world.view());
    checks.require(
        betrayals(&social, tuning, &ids, 6, 2).is_empty(),
        "regard at K_loyal did not suppress a betrayal",
        format!(
            "regard {} against a K_loyal of {}",
            tuning.k_loyal, tuning.k_loyal
        ),
    );
    let (world, ids) = bench(&[("Broke", 9, &[], &[]), ("Other", 0, &[], &[])], &[]);
    let social = Social::read(&world.view());
    checks.require(
        betrayals(&social, tuning, &ids, 2, 2).is_empty(),
        "a killing with nothing to gain happened anyway",
        "a pot of 2 with a cut of 2 leaves 0 to split, so no death changes a share".to_owned(),
    );

    // --- the aftermath a beat never reaches: a surviving witness --------
    //
    // Three in, one killed, and the killer spares the third because they are
    // bonded to them. Run twice - once with the survivor bonded to the victim
    // and once not - because the thing that has to be true lives *between* the
    // two runs: a grudge stated as "equals witness_grudge + bonded_grudge"
    // moves when those constants move, and passes for a model that has stopped
    // applying either of them.
    let (plain_social, plain_ids, plain) = bench_kill(tuning, false);
    let (social, ids, outcome) = bench_kill(tuning, true);
    for (label, outcome, ids) in [("bonded", &outcome, &ids), ("unbonded", &plain, &plain_ids)] {
        checks.require(
            outcome.betrayals.len() == 1
                && outcome.betrayals.first().map(|b| b.victim) == Some(ids[1])
                && outcome.party == *ids
                && outcome.survivors == vec![ids[0], ids[2]],
            "the bench killing did not happen the way the rules say",
            format!(
                "the {label} run had {} killing(s) and {} survivor(s); one killing and two \
                 survivors is the case - the killer spares the witness because regard toward \
                 them reaches K_loyal",
                outcome.betrayals.len(),
                outcome.survivors.len()
            ),
        );
        // The murder writes the mark - the reputation system's pen (DESIGN
        // §5), where v1 moved a public scalar.
        checks.require(
            outcome
                .mark_writes
                .iter()
                .any(|(who, mark)| *who == ids[0] && *mark == MarkId::ComradeKiller),
            "a witnessed kill did not write the comrade-killer mark",
            format!(
                "the {label} run wrote {:?}; the killer's sheet is where everyone learns \
                 what he is",
                outcome.mark_writes
            ),
        );
        checks.require(
            outcome
                .lines
                .iter()
                .any(|line| line.contains("marked comrade-killer")),
            "the report does not narrate the mark the kill wrote",
            format!("the {label} run's narration is {:?}", outcome.lines),
        );
    }
    let drop_of = |outcome: &crate::resolve::Resolution, ids: &[Entity]| {
        outcome
            .regard_changes
            .iter()
            .find(|change| change.from == ids[2] && change.to == ids[0])
            .map(|change| (change.before, change.after - change.before))
    };
    let bonded_drop = drop_of(&outcome, &ids);
    let plain_drop = drop_of(&plain, &plain_ids);
    checks.require(
        plain_drop.is_some_and(|(before, delta)| before == 0 && delta < 0),
        "a surviving witness does not hold a killing against the killer at all",
        format!(
            "the witness's regard toward the killer moved by {:?}, from a bench where it \
             started at zero",
            plain_drop.map(|(_, delta)| delta)
        ),
    );
    checks.require(
        match (bonded_drop, plain_drop) {
            (Some((_, bonded)), Some((_, plain))) => bonded < plain,
            _ => false,
        },
        "being bonded to the victim costs the killer nothing extra",
        format!(
            "a witness bonded to the victim moved by {:?} and one who was not by {:?}; harm \
             to a bonded character is what makes a relationship propagate a consequence",
            bonded_drop.map(|(_, delta)| delta),
            plain_drop.map(|(_, delta)| delta),
        ),
    );
    let wanted = -(tuning.witness_grudge + tuning.bonded_grudge);
    checks.require(
        bonded_drop.is_some_and(|(before, delta)| before + delta == wanted),
        "the bonded witness's grudge is not the two drops the constants name",
        format!(
            "it ended at {:?}, wanted {wanted} - the witness's {} plus the bonded {}",
            bonded_drop.map(|(before, delta)| before + delta),
            tuning.witness_grudge,
            tuning.bonded_grudge
        ),
    );
    checks.require(
        outcome
            .lines
            .iter()
            .any(|line| line.contains("bonded to Vic"))
            && !plain.lines.iter().any(|line| line.contains("bonded to")),
        "the report does not name why the witness's grudge is the size it is",
        format!(
            "the bonded run said {:?} and the unbonded run said {:?}",
            outcome
                .lines
                .iter()
                .filter(|line| line.contains("saw it"))
                .collect::<Vec<_>>(),
            plain
                .lines
                .iter()
                .filter(|line| line.contains("saw it"))
                .collect::<Vec<_>>(),
        ),
    );
    // A betrayal ends the bonding: nobody got closer on a job somebody died on
    // - and nobody's clean-job count moved either.
    let gained: Vec<i32> = outcome
        .regard_changes
        .iter()
        .filter(|change| change.after > change.before)
        .map(|change| change.after - change.before)
        .collect();
    checks.require(
        gained.is_empty(),
        "survivors bonded over a job somebody was killed on",
        format!("regard rose by {gained:?} on a run with a killing in it"),
    );
    checks.require(
        outcome.clean_job_changes.is_empty(),
        "a job somebody was killed on counted as clean",
        format!(
            "the clean-job counts moved: {:?}; the reliable count is for jobs everyone \
             walked away from",
            outcome.clean_job_changes
        ),
    );
    // The floor: a full share against no need at all leaves nothing below zero.
    let witness_drift = outcome
        .desperation_changes
        .iter()
        .find(|(who, _, _)| *who == ids[2]);
    checks.require(
        witness_drift.map(|(_, _, after)| *after) == Some(tuning.desperation_floor),
        "desperation fell through the floor",
        format!(
            "the witness ended at {:?} from 0, and the floor is {}",
            witness_drift.map(|(_, _, after)| *after),
            tuning.desperation_floor
        ),
    );
    let _ = (plain_social, social);

    // --- the dungeon predicates no tutorial beat uses -------------------
    //
    // v1's known-face predicates migrated to marks: a job that needs a known
    // face needs a dark mark on somebody (DESIGN §5).
    let (world, ids) = bench(
        &[
            ("Clean", 0, &[], &[]),
            ("Known", 0, &[], &[MarkId::ComradeKiller]),
            ("Bright", 0, &[], &[MarkId::Reliable]),
        ],
        &[],
    );
    let social = Social::read(&world.view());
    for (requirement, party, want) in [
        (Requirement::AnyParty, &ids[..], true),
        (Requirement::NeedsDarkMark, &ids[..2], true),
        (Requirement::NeedsDarkMark, &ids[..1], false),
        // A light mark is not a dark one: reliable does not open the
        // underworld's door.
        (Requirement::NeedsDarkMark, &ids[2..], false),
        (Requirement::NoDarkMarks, &ids[..1], true),
        (Requirement::NoDarkMarks, &ids[2..], true),
        (Requirement::NoDarkMarks, &ids[..2], false),
    ] {
        let met = requirement.met(&social, party);
        checks.require(
            met == want,
            "a dungeon predicate answered the wrong way about a party",
            format!(
                "{} against a party of {} said {met}, wanted {want}",
                requirement.describe(),
                party.len()
            ),
        );
    }

    // --- the gate, including the states play does not stop on -----------
    let (world, ids) = bench(
        &[
            ("Clean", 1, &[], &[]),
            ("Known", 1, &[], &[MarkId::ComradeKiller]),
        ],
        &[],
    );
    let social = Social::read(&world.view());
    let two_of_three = crate::beats::Dungeon {
        requires: Requirement::NoDarkMarks,
        ..bench_job(3, 12, 0)
    };
    let gate = assess(
        &social,
        tuning,
        VariantId::default(),
        &ids,
        Some(&two_of_three),
    );
    checks.require(
        !gate.can_send && !gate.headcount_ok && gate.blocked == "need 1 more",
        "an under-filled party is not blocked by the headcount it is short of",
        format!("the gate said {:?}", gate.blocked),
    );
    let gate = assess(
        &social,
        tuning,
        VariantId::default(),
        &ids[..1],
        Some(&two_of_three),
    );
    checks.require(
        !gate.can_send && gate.blocked == "need 2 more",
        "the headcount is reported before the predicate that also fails",
        format!("the gate said {:?}", gate.blocked),
    );
    let three = crate::beats::Dungeon {
        headcount: 2,
        ..two_of_three
    };
    let gate = assess(&social, tuning, VariantId::default(), &ids, Some(&three));
    checks.require(
        !gate.can_send
            && gate.headcount_ok
            && !gate.requirement_ok
            && gate.blocked == "a dark mark in the party",
        "a party that breaks the dungeon's predicate is not blocked by it",
        format!("the gate said {:?}", gate.blocked),
    );
    let open = crate::beats::Dungeon {
        requires: Requirement::AnyParty,
        headcount: 1,
        ..three
    };
    let gate = assess(
        &social,
        tuning,
        VariantId::default(),
        &ids[1..],
        Some(&open),
    );
    checks.require(
        gate.can_send && gate.all_willing && gate.blocked.is_empty(),
        "a one-member party that satisfies everything cannot be sent",
        format!("the gate said {:?}", gate.blocked),
    );

    // --- the deterministic variant is preserved, not reimplemented ----------
    //
    // Two claims: it never reads the dice (two far-apart seeds, identical
    // resolution), and the ladder is replay-exact (one seed twice, identical
    // resolution).
    let (world, ids) = bench(
        &[
            ("Hot", 9, &[], &[]),
            ("Vic", 0, &[], &[]),
            ("Wit", 0, &[], &[]),
        ],
        &[],
    );
    let social = Social::read(&world.view());
    let job = bench_job(3, 12, 0);
    let mut rng_a = Rng::from_seed(3);
    let mut rng_b = Rng::from_seed(987_654_321);
    let det_a = resolve(
        &social,
        tuning,
        VariantId::Deterministic,
        &job,
        &ids,
        &mut rng_a,
    );
    let det_b = resolve(
        &social,
        tuning,
        VariantId::Deterministic,
        &job,
        &ids,
        &mut rng_b,
    );
    checks.require(
        det_a.lines == det_b.lines && !det_a.betrayals.is_empty(),
        "the deterministic variant's outcome moved with the seed",
        format!(
            "seed 3 narrates {:?} and seed 987654321 narrates {:?}; v1's rule reads no dice",
            det_a.lines, det_b.lines
        ),
    );
    let mut rng_c = Rng::from_seed(11);
    let mut rng_d = Rng::from_seed(11);
    let lad_c = resolve(&social, tuning, VariantId::Ladder, &job, &ids, &mut rng_c);
    let lad_d = resolve(&social, tuning, VariantId::Ladder, &job, &ids, &mut rng_d);
    checks.require(
        lad_c.lines == lad_d.lines,
        "the ladder is not replay-exact at a fixed seed",
        format!(
            "seed 11 twice narrated {:?} and then {:?}",
            lad_c.lines, lad_d.lines
        ),
    );

    crate::pressure::battery(checks, tuning);
    crate::ladder::battery(checks, tuning);
    crate::judgment::battery(checks, tuning);
    crate::door::door(checks, tuning);
}
