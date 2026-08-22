//! Resolution: what a dungeon does to the people sent into it (DESIGN.md §5).
//!
//! One pure function of `(social snapshot, tuning, dungeon, party)`, producing
//! a `Resolution` — every consequence as data, plus the mechanical narration
//! the report draws. Nothing here touches the world; `apply` is the write pass
//! that does, and it only copies numbers the function already decided.
//!
//! That split is what lets `--verify` ask the model questions a played beat
//! never reaches — a surviving witness to a killing, a party of five with two
//! desperate members — without scripting a beat to produce them.
//!
//! **The stated order** (DESIGN §5): requirements are checked at assembly, so a
//! party that arrives here succeeds; then betrayal evaluation in roster order,
//! then payout, then bond drift, then round-end desperation drift. v1 has no
//! resolution failure.

use jidousha::prelude::*;

use crate::beats::Dungeon;
use crate::constants::Tuning;
use crate::model::{
    Betrayal, Dead, Desperation, Infamy, RegardChange, RegardEdge, Resolution, Social, Wealth,
    betrayals, share_each,
};

/// Run a dungeon. `party` is in roster order.
pub fn resolve(
    social: &Social,
    tuning: &Tuning,
    dungeon: &Dungeon,
    party: &[Entity],
) -> Resolution {
    let mut out = Resolution {
        party: party.to_vec(),
        ..Resolution::default()
    };
    let names = |entity: Entity| social.name(entity);
    out.lines.push(format!(
        "{} - pot {}, your cut {}, party of {}",
        dungeon.name,
        dungeon.pot,
        dungeon.cut,
        party.len()
    ));

    // --- betrayal, in roster order -----------------------------------
    out.betrayals = betrayals(social, tuning, party, dungeon.pot, dungeon.cut);
    for Betrayal {
        killer,
        victim,
        desperation,
        share_before,
        share_after,
        regard,
    } in out.betrayals.iter().copied()
    {
        out.lines.push(format!(
            "{} killed {} - desperation {} >= {}, share {}->{}, regard {} < {}",
            names(killer),
            names(victim),
            desperation,
            tuning.k_kill,
            share_before,
            share_after,
            regard,
            tuning.k_loyal,
        ));
    }
    out.survivors = party
        .iter()
        .copied()
        .filter(|member| {
            !out.betrayals
                .iter()
                .any(|betrayal| betrayal.victim == *member)
        })
        .collect();

    // --- payout -------------------------------------------------------
    let survivor_count = i32::try_from(out.survivors.len()).unwrap_or(i32::MAX);
    let share = share_each(dungeon.pot, dungeon.cut, survivor_count);
    for member in out.survivors.iter().copied() {
        out.payouts.push((member, share));
        out.lines.push(format!(
            "{} takes {} - {} split {} way{}",
            names(member),
            share,
            (dungeon.pot - dungeon.cut).max(0),
            survivor_count,
            if survivor_count == 1 { "" } else { "s" },
        ));
    }

    // --- bond drift ---------------------------------------------------
    //
    // "Shared success without betrayal raises mutual regard between all
    // surviving pairs" (DESIGN §3.2). Read per *run* rather than per pair: a
    // job somebody was killed on is not a job the survivors got closer on.
    if out.betrayals.is_empty() {
        let survivors = out.survivors.clone();
        for (index, first) in survivors.iter().copied().enumerate() {
            for second in survivors.iter().copied().skip(index + 1) {
                push_regard(&mut out, social, first, second, tuning.bond_gain);
                push_regard(&mut out, social, second, first, tuning.bond_gain);
                out.lines.push(format!(
                    "{} and {} bond - regard +{} both ways, a clean job",
                    names(first),
                    names(second),
                    tuning.bond_gain
                ));
            }
        }
    }

    // --- what a betrayal costs the betrayer ---------------------------
    for betrayal in out.betrayals.clone() {
        let before = social.infamy(betrayal.killer);
        let after = before + tuning.infamy_per_kill;
        out.infamy_changes.push((betrayal.killer, before, after));
        out.lines.push(format!(
            "{}'s infamy {}->{} - a witnessed kill is public",
            names(betrayal.killer),
            before,
            after
        ));
        // Each surviving witness holds it against the killer personally, and
        // holds it harder if they were bonded to the victim: relationships are
        // what make events travel (DESIGN §3.3.3).
        for witness in out.survivors.clone() {
            if witness == betrayal.killer {
                continue;
            }
            let bonded = social.regard(witness, betrayal.victim) > 0;
            let drop = tuning.witness_grudge + if bonded { tuning.bonded_grudge } else { 0 };
            let before = current_regard(&out, social, witness, betrayal.killer);
            push_regard(&mut out, social, witness, betrayal.killer, -drop);
            out.lines.push(format!(
                "{} saw it - regard toward {} {}->{}{}",
                names(witness),
                names(betrayal.killer),
                before,
                before - drop,
                if bonded {
                    format!(
                        ", bonded to {} so -{} more",
                        names(betrayal.victim),
                        tuning.bonded_grudge
                    )
                } else {
                    String::new()
                },
            ));
        }
    }

    // --- round-end desperation drift ----------------------------------
    //
    // Every living roster member, not only the party: non-participants do not
    // profit, so the roster decays toward willingness and refusal is always
    // temporary (DESIGN §4).
    for member in &social.members {
        if !member.alive || out.betrayals.iter().any(|b| b.victim == member.entity) {
            continue;
        }
        let profited = out
            .payouts
            .iter()
            .any(|(who, amount)| *who == member.entity && *amount > 0);
        let before = member.desperation;
        let after = if profited {
            (before - tuning.desperation_fall).max(tuning.desperation_floor)
        } else {
            (before + tuning.desperation_rise).max(tuning.desperation_floor)
        };
        out.desperation_changes.push((member.entity, before, after));
        let why = if profited {
            "profited"
        } else if party.contains(&member.entity) {
            "came back empty"
        } else {
            "sat out"
        };
        out.lines.push(format!(
            "{} {} - desperation {}->{}",
            member.name, why, before, after
        ));
    }
    out
}

/// What `from` thinks of `to` right now, counting changes this resolution has
/// already decided but not yet applied.
fn current_regard(out: &Resolution, social: &Social, from: Entity, to: Entity) -> i32 {
    out.regard_changes
        .iter()
        .rev()
        .find(|change| change.from == from && change.to == to)
        .map_or_else(|| social.regard(from, to), |change| change.after)
}

/// Record an edge moving by `delta`.
fn push_regard(out: &mut Resolution, social: &Social, from: Entity, to: Entity, delta: i32) {
    let before = current_regard(out, social, from, to);
    out.regard_changes.push(RegardChange {
        from,
        to,
        before,
        after: before + delta,
    });
}

/// The write pass: put a resolution into the world.
///
/// Structural changes only after the reads are done, and every number here was
/// decided by `resolve` — nothing recomputes a rule at the write site, which is
/// the way the report and the world cannot disagree.
pub fn apply(world: &mut World, resolution: &Resolution) {
    for (who, amount) in resolution.payouts.iter().copied() {
        if let Some(wealth) = world.find_component_mut::<Wealth>(who) {
            wealth.0 += amount;
        }
    }
    for betrayal in &resolution.betrayals {
        world.insert(
            betrayal.victim,
            Dead {
                killed_by: betrayal.killer,
            },
        );
    }
    for (who, _, after) in resolution.infamy_changes.iter().copied() {
        if let Some(infamy) = world.find_component_mut::<Infamy>(who) {
            infamy.0 = after;
        }
    }
    for (who, _, after) in resolution.desperation_changes.iter().copied() {
        if let Some(desperation) = world.find_component_mut::<Desperation>(who) {
            desperation.0 = after;
        }
    }
    for change in &resolution.regard_changes {
        set_regard(world, change.from, change.to, change.after);
    }
}

/// Write one directed edge, creating the edge entity if the pair had none.
///
/// Read pass then write pass (DESIGN §9): the query that finds the edge borrows
/// the world, so what it finds is collected and the query dropped before
/// anything is written.
fn set_regard(world: &mut World, from: Entity, to: Entity, value: i32) {
    let existing = world
        .query::<&RegardEdge>()
        .find(|(_, edge)| edge.from == from && edge.to == to)
        .map(|(entity, _)| entity);
    match existing {
        Some(entity) => world.component_mut::<RegardEdge>(entity).value = value,
        None => {
            let entity = world.spawn();
            world.insert(entity, RegardEdge { from, to, value });
        }
    }
}
