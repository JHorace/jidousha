//! The questions a played beat never asks, and the round that says whether any
//! of the asking is an instrument.
//!
//! Three groups:
//!
//! - **the battery** - the decision function asked directly, on rosters built
//!   for the case: the roster-order betrayal that decides who kills whom, the
//!   surviving witness no tutorial beat produces, the two dungeon predicates no
//!   tutorial beat uses, and the arithmetic at the boundaries of all three
//!   clauses. A run only tests the states it reaches, and the margins a correct
//!   game rests on are exactly the states it never reaches.
//! - **the printable check** - the font covers space through `~` and draws
//!   everything else as a box at exactly a letter's advance, so no assertion
//!   over drawn quads can see a wrong character. The string is the only
//!   instrument, and prose is the habit that produces an em dash.
//! - **the mutation round** - every constant in `constants.rs` perturbed in
//!   turn, demanding that some beat or contract notices. A beat that passes
//!   under a mutated constant is a vacuous assertion.

use jidousha::prelude::*;

use crate::beats::{QuestIcon, Requirement};
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::flow::assess;
use crate::model::{
    Character, Desperation, Infamy, RegardEdge, Social, Wealth, betrayals, share_each, willingness,
};
use crate::resolve::resolve;

/// A roster built for one question: `(name, desperation, infamy)` in roster
/// order, then the edges between them by index.
pub(crate) fn bench(
    rows: &[(&'static str, i32, i32)],
    edges: &[(usize, usize, i32)],
) -> (World, Vec<Entity>) {
    let mut world = World::new();
    let mut ids = Vec::new();
    for (roster_index, (name, desperation, infamy)) in rows.iter().enumerate() {
        let entity = world.spawn();
        world.insert(entity, Character { name, roster_index });
        world.insert(entity, Desperation(*desperation));
        world.insert(entity, Infamy(*infamy));
        world.insert(entity, Wealth(0));
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
    let (world, ids) = bench(&[("Kil", 8, 0), ("Vic", 0, 0), ("Wit", 0, 0)], &edges);
    let social = Social::read(&world.view());
    let dungeon = crate::beats::Dungeon {
        name: "the bench",
        blurb: "a roster built for one question",
        icon: QuestIcon::Cave,
        headcount: 3,
        pot: 12,
        cut: 0,
        requires: Requirement::AnyParty,
    };
    let outcome = resolve(&social, tuning, &dungeon, &ids);
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

    // --- willingness: a bond outranks public information ---------------
    let (world, ids) = bench(&[("Clean", 1, 0), ("Known", 0, 3)], &[]);
    let social = Social::read(&world.view());
    let answer = willingness(&social, tuning, ids[0], &ids);
    checks.require(
        answer.total == 1 - tuning.k_inf * 3 && !answer.joins(),
        "an infamy gap no longer gates a cleaner character",
        format!("Clean says {}", answer.arithmetic()),
    );
    let (world, ids) = bench(&[("Clean", 1, 0), ("Known", 0, 3)], &[(0, 1, 5)]);
    let social = Social::read(&world.view());
    let answer = willingness(&social, tuning, ids[0], &ids);
    checks.require(
        answer.joins() && answer.regard_total == 5,
        "a bond no longer overrides the infamy gap it is supposed to outweigh",
        format!(
            "Clean says {} with a regard of 5 in hand",
            answer.arithmetic()
        ),
    );
    // And the reverse direction is untouched, which is what "directed" means.
    checks.require(
        answer
            .terms
            .iter()
            .map(|term| term.member)
            .collect::<Vec<_>>()
            == vec![ids[1]],
        "a willingness sum names the wrong partymates",
        format!(
            "Clean's sum has {} term(s) and the party is 2 including Clean",
            answer.terms.len()
        ),
    );
    let back = willingness(&social, tuning, ids[1], &ids);
    checks.require(
        back.regard_total == 0,
        "regard is being read as symmetric",
        format!(
            "Known says {} about a party they hold nothing about",
            back.arithmetic()
        ),
    );

    // A gap only ever costs the cleaner character. `incompat` is
    // `max(0, .)`-clamped, so the known name standing next to a clean one gets
    // nothing for it - drop the clamp and a reputation becomes a recruiting
    // bonus, which no beat above would notice.
    let (world, ids) = bench(&[("Clean", 0, 0), ("Known", 0, 5)], &[]);
    let social = Social::read(&world.view());
    let downhill = social.incompat(tuning, ids[1], ids[0]);
    checks.require(
        downhill == 0,
        "standing next to a cleaner name pays an infamous character",
        format!(
            "incompat(Known, Clean) is {downhill} across a gap of -5; incompat is clamped at \
             zero and only ever costs the cleaner of the two"
        ),
    );
    checks.require(
        social.incompat(tuning, ids[0], ids[1]) == tuning.k_inf * 5,
        "the gap stopped costing the cleaner character",
        format!(
            "incompat(Clean, Known) is {} across a gap of 5 at a K_inf of {}",
            social.incompat(tuning, ids[0], ids[1]),
            tuning.k_inf
        ),
    );

    // --- betrayal: roster order decides who kills whom ------------------
    //
    // Two characters, identical numbers, both past `K_kill` and both better off
    // alone. Nothing but the stated order separates them, and reversing it
    // reverses the outcome - which is the whole reason the order is stated.
    for reversed in [false, true] {
        let rows: &[(&str, i32, i32)] = if reversed {
            &[("Second", 8, 0), ("First", 8, 0)]
        } else {
            &[("First", 8, 0), ("Second", 8, 0)]
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
    let (world, ids) = bench(&[("Calm", tuning.k_kill - 1, 0), ("Other", 0, 0)], &[]);
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
        &[("Loyal", 9, 0), ("Friend", 0, 0)],
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
    let (world, ids) = bench(&[("Broke", 9, 0), ("Other", 0, 0)], &[]);
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
        outcome.infamy_changes.first().map(|(_, _, after)| *after) == Some(tuning.infamy_per_kill),
        "a witnessed kill did not become public",
        format!(
            "the killer's infamy ended at {:?}, wanted {}",
            outcome.infamy_changes.first().map(|(_, _, after)| *after),
            tuning.infamy_per_kill
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
    // A betrayal ends the bonding: nobody got closer on a job somebody died on.
    // Asked of the edges rather than of the narration, because the witness's
    // own line has the word "bonded" in it for the opposite reason.
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
    let (world, ids) = bench(&[("Clean", 0, 0), ("Known", 0, 4)], &[]);
    let social = Social::read(&world.view());
    for (requirement, party, want) in [
        (Requirement::AnyParty, &ids[..], true),
        (
            Requirement::AtLeastOneInfamous { at_least: 3 },
            &ids[..],
            true,
        ),
        (
            Requirement::AtLeastOneInfamous { at_least: 3 },
            &ids[..1],
            false,
        ),
        (Requirement::NoInfamous { at_least: 3 }, &ids[..1], true),
        (Requirement::NoInfamous { at_least: 3 }, &ids[..], false),
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
    let (world, ids) = bench(&[("Clean", 1, 0), ("Known", 0, 3)], &[]);
    let social = Social::read(&world.view());
    let two_of_three = crate::beats::Dungeon {
        name: "the bench",
        blurb: "a roster built for one question",
        icon: QuestIcon::Cave,
        headcount: 3,
        pot: 12,
        cut: 0,
        requires: Requirement::NoInfamous { at_least: 3 },
    };
    let gate = assess(&social, tuning, &ids, Some(&two_of_three));
    checks.require(
        !gate.can_send && !gate.headcount_ok && gate.blocked == "need 1 more",
        "an under-filled party is not blocked by the headcount it is short of",
        format!("the gate said {:?}", gate.blocked),
    );
    let gate = assess(&social, tuning, &ids[..1], Some(&two_of_three));
    checks.require(
        !gate.can_send && gate.blocked == "need 2 more",
        "the headcount is reported before the predicate that also fails",
        format!("the gate said {:?}", gate.blocked),
    );
    let three = crate::beats::Dungeon {
        headcount: 2,
        ..two_of_three
    };
    let gate = assess(&social, tuning, &ids, Some(&three));
    checks.require(
        !gate.can_send
            && gate.headcount_ok
            && !gate.requirement_ok
            && gate.blocked == "a known face in the party",
        "a party that breaks the dungeon's predicate is not blocked by it",
        format!("the gate said {:?}", gate.blocked),
    );
    let open = crate::beats::Dungeon {
        requires: Requirement::AnyParty,
        ..three
    };
    let gate = assess(
        &social,
        tuning,
        &ids[1..],
        Some(&crate::beats::Dungeon {
            headcount: 1,
            ..open
        }),
    );
    checks.require(
        gate.can_send && gate.blocked.is_empty(),
        "a one-member party that satisfies everything cannot be sent",
        format!("the gate said {:?}", gate.blocked),
    );
    let gate = assess(
        &social,
        tuning,
        &ids[1..],
        Some(&crate::beats::Dungeon {
            headcount: 1,
            ..open
        }),
    );
    checks.require(
        gate.can_send && gate.all_willing && gate.blocked.is_empty(),
        "a party that satisfies everything cannot be sent",
        format!("the gate said {:?}", gate.blocked),
    );

    crate::door::door(checks, tuning);
}
