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
    Betrayal, Dead, Desperation, Infamy, RegardEdge, Social, Wealth, betrayals, share_each,
};

// ── what a resolution *is*: the record the write pass and the screens read ──
//
// These live here rather than in `model.rs` because `resolve` is the only thing
// that builds them, and because they are the shape of an outcome rather than
// the shape of the world. `model.rs` owns state and the decision function; this
// file owns what one dungeon did with them.

/// One regard edge moving, and why.
#[derive(Clone, Copy, Debug)]
pub struct RegardChange {
    /// Who holds the opinion.
    pub from: Entity,
    /// Who it is about.
    pub to: Entity,
    /// What it was.
    pub before: i32,
    /// What it becomes.
    pub after: i32,
}

/// What an event card on the resolution screen is about (UI.md §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    /// A betrayal: skull-marked, ember-bordered.
    Kill,
    /// Money changing hands: coin-marked.
    Coin,
    /// A consequence that is neither, drawn with the signifier it is about.
    Word,
}

/// One card on the resolution screen: what happened, and the arithmetic under
/// it in small text (UI.md §3).
#[derive(Clone, Debug)]
pub struct EventCard {
    /// Which signifier the card carries.
    pub kind: EventKind,
    /// The sentence.
    pub text: String,
    /// The rule inputs beneath it, if the event has any worth naming.
    pub sub: Option<String>,
}

/// Which way a drift-ledger line reads for the people in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriftTone {
    /// Need going up - the hungry-wait line, and every other cost.
    Cost,
    /// Need coming down.
    Relief,
    /// A regard edge moving.
    Regard,
    /// An infamy moving.
    Infamy,
}

/// One line of the drift ledger.
#[derive(Clone, Debug)]
pub struct DriftLine {
    /// How it reads.
    pub tone: DriftTone,
    /// What it says.
    pub text: String,
}

/// Everything one dungeon did, as data — before any of it touches the world.
#[derive(Clone, Debug, Default)]
pub struct Resolution {
    /// The party, in roster order.
    pub party: Vec<Entity>,
    /// Who came back.
    pub survivors: Vec<Entity>,
    /// Every killing, in the order they were evaluated.
    pub betrayals: Vec<Betrayal>,
    /// What each survivor took.
    pub payouts: Vec<(Entity, i32)>,
    /// Every edge that moved.
    pub regard_changes: Vec<RegardChange>,
    /// Every infamy that moved: who, from, to.
    pub infamy_changes: Vec<(Entity, i32, i32)>,
    /// Every desperation that moved: who, from, to.
    pub desperation_changes: Vec<(Entity, i32, i32)>,
    /// The mechanical narration, one line per consequence.
    ///
    /// The ASCII story surface DESIGN §7 mandates, and what the log drawer and
    /// every `Expect::ReportSays` read. The takeover draws `events` and `drift`
    /// instead — the same consequences, laid out rather than listed — so the
    /// two are built together from one pass and cannot describe different runs.
    pub lines: Vec<String>,
    /// The event cards, in the order the rules produced them.
    pub events: Vec<EventCard>,
    /// The drift ledger, after the cards.
    pub drift: Vec<DriftLine>,
}

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
        // The same killing as a card: the sentence a player reads, and the
        // three clauses that produced it in small text under it (UI.md §3).
        out.events.push(EventCard {
            kind: EventKind::Kill,
            text: format!("{} turned on {}.", names(killer), names(victim)),
            sub: Some(format!(
                "desperation {desperation} >= {} - regard {regard} < {} - share {share_before}g \
                 -> {share_after}g",
                tuning.k_kill, tuning.k_loyal
            )),
        });
    }
    // Absence of an event is also information (UI.md §3).
    if out.betrayals.is_empty() {
        out.events.push(EventCard {
            kind: EventKind::Word,
            text: "No blood spilled. Everyone walked back out.".to_owned(),
            sub: None,
        });
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
                out.drift.push(DriftLine {
                    tone: DriftTone::Regard,
                    text: format!(
                        "shared work: {} and {} regard +{} both ways",
                        names(first),
                        names(second),
                        tuning.bond_gain
                    ),
                });
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
        out.events.push(EventCard {
            kind: EventKind::Word,
            text: format!("Word gets out about {}.", names(betrayal.killer)),
            sub: Some(format!(
                "infamy {before}->{after} - every witness holds it against them personally"
            )),
        });
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
            out.drift.push(DriftLine {
                tone: DriftTone::Regard,
                text: format!(
                    "{} saw it: regard toward {} {}->{}",
                    names(witness),
                    names(betrayal.killer),
                    before,
                    before - drop
                ),
            });
        }
        out.drift.push(DriftLine {
            tone: DriftTone::Infamy,
            text: format!(
                "{} infamy {}->{}",
                names(betrayal.killer),
                social.infamy(betrayal.killer),
                social.infamy(betrayal.killer) + tuning.infamy_per_kill
            ),
        });
    }

    // The payout is the last card, after what the killing cost: a player reads
    // the column in the order the rules fired, and the money is what the whole
    // column was for.
    out.events.push(EventCard {
        kind: EventKind::Coin,
        text: format!("Your cut: {}g. Each survivor takes {share}g.", dungeon.cut),
        sub: Some(format!(
            "pot {}g - cut {}g = {}g split {survivor_count} way{}",
            dungeon.pot,
            dungeon.cut,
            (dungeon.pot - dungeon.cut).max(0),
            if survivor_count == 1 { "" } else { "s" },
        )),
    });

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
        if before != after {
            out.drift.push(DriftLine {
                tone: if after < before {
                    DriftTone::Relief
                } else {
                    DriftTone::Cost
                },
                text: format!("{} {why}: desperation {before}->{after}", member.name),
            });
        }
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
