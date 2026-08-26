//! Resolution: what a dungeon does to the people sent into it (DESIGN.md §7,
//! §8).
//!
//! One pure function of `(social snapshot, tuning, variant, dungeon, party,
//! rng)`, producing a `Resolution` — every consequence as data, plus the
//! mechanical narration the report draws. Nothing here touches the world;
//! `apply` is the write pass that does, and it only copies numbers the
//! function already decided.
//!
//! **The betrayal rule is the variant's** (DESIGN §8b, §8e): `variant::events`
//! hands back one event list whichever rule set the chain was started under —
//! the seeded ladder, or v1's deterministic rule preserved verbatim. This is
//! the **only place the `Rng` is read** in the whole game: the rolls happen
//! inside the ladder's event function and nowhere else, which is what keeps
//! willingness deterministic under any seed.
//!
//! **The stated order** (DESIGN §7): requirements are checked at assembly, so
//! a party that arrives here starts its quest; then betrayal events in roster
//! order against start-of-resolution state; then the desertion re-evaluation
//! (an abandoned quest can now fail — P2's one new failure path, and it is
//! loud); then payout through the skim and sabotage arithmetic; then bond
//! drift and clean-job counting (a quest with any betrayal in it bonds
//! nobody); then marks, grudges and round-end desperation drift.
//!
//! **The resolution is the reputation system's pen** (DESIGN §5, §8): every
//! rung writes its mark — *skimmer*, *deserter*, *saboteur*, *comrade-killer*
//! — and its regard edges, and narrates every number it read.

use jidousha::prelude::*;

use crate::beats::Dungeon;
use crate::constants::Tuning;
use crate::ladder::{self, Rung, RungEvent};
use crate::model::{
    Betrayal, CleanJobs, Dead, Desperation, Marks, RegardEdge, Social, Wealth, share_each,
};
use crate::pressure;
use crate::traits::MarkId;
use crate::variant::VariantId;

// ── what a resolution *is*: the record the write pass and the screens read ──

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
    /// A betrayal: skull-marked, ember-bordered — any rung of the ladder.
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
    /// A mark being written.
    Mark,
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
    /// Who came back **with a share coming**: the party less the murdered and
    /// the deserters. A deserter is alive and unpaid; a victim is neither.
    pub survivors: Vec<Entity>,
    /// Every betrayal event, in the order the rule produced them.
    pub rungs: Vec<RungEvent>,
    /// Every killing, in the order they were evaluated — the murder events'
    /// v1-shaped record, which is what the write pass reads `Dead` from.
    pub betrayals: Vec<Betrayal>,
    /// Whether the quest failed — a desertion left it short (DESIGN §8c).
    pub failed: bool,
    /// What the player's cut actually was: the stated cut, or nothing from a
    /// failed job.
    pub cut_taken: i32,
    /// What each survivor took.
    pub payouts: Vec<(Entity, i32)>,
    /// Every edge that moved.
    pub regard_changes: Vec<RegardChange>,
    /// Every mark written: who, and what everyone now knows.
    pub mark_writes: Vec<(Entity, MarkId)>,
    /// Every clean-job count that moved: who, from, to.
    pub clean_job_changes: Vec<(Entity, i32, i32)>,
    /// Every desperation that moved: who, from, to.
    pub desperation_changes: Vec<(Entity, i32, i32)>,
    /// The mechanical narration, one line per consequence.
    ///
    /// The ASCII story surface DESIGN §12 mandates, and what the log drawer and
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
///
/// The `rng` is read only by the ladder variant's rolls, inside
/// `variant::events`; the deterministic variant never draws, so its outcome is
/// v1's exactly, whatever the seed.
pub fn resolve(
    social: &Social,
    tuning: &Tuning,
    variant: VariantId,
    dungeon: &Dungeon,
    party: &[Entity],
    rng: &mut Rng,
) -> Resolution {
    let mut out = Resolution {
        party: party.to_vec(),
        cut_taken: dungeon.cut,
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

    // --- the pressures, and the band the chip promised --------------------
    //
    // The same `pressure::party` the strip's chip read before SEND, on the
    // same snapshot — one source, so the warning and the roll cannot disagree
    // (DESIGN §7a).
    let pressures = pressure::party(social, tuning, party, dungeon);
    if variant.foreshadows() {
        let band = pressure::party_band(tuning, &pressures);
        out.lines.push(format!(
            "the party read {} - highest pressure {}",
            band.word(),
            pressures.iter().map(|p| p.total).max().unwrap_or(0),
        ));
    }

    // --- betrayal, in roster order (the variant's rule) --------------------
    out.rungs = crate::variant::events(variant, social, tuning, dungeon, party, &pressures, rng);

    // Walk the events once to name each one and keep the running membership
    // the murder arithmetic reads. Rolls used start-of-resolution state; what
    // shrinks here is only who is still standing in the room.
    let mut murdered: Vec<Entity> = Vec::new();
    let mut deserters: Vec<Entity> = Vec::new();
    let mut skimmers: Vec<Entity> = Vec::new();
    let mut saboteurs: Vec<Entity> = Vec::new();
    let mut present: Vec<Entity> = party.to_vec();
    for event in out.rungs.clone() {
        match event.rung {
            Rung::Murder => {
                let Some(victim) = event.victim else { continue };
                let record = event.v1.unwrap_or_else(|| {
                    // The ladder's murder: same writes as v1, and the share
                    // numbers the narration quotes are the ones the killer saw.
                    let count = i32::try_from(present.len()).unwrap_or(i32::MAX);
                    Betrayal {
                        killer: event.who,
                        victim,
                        desperation: social.desperation(event.who),
                        share_before: share_each(dungeon.pot, dungeon.cut, count),
                        share_after: share_each(dungeon.pot, dungeon.cut, count - 1),
                        regard: event.victim_regard,
                    }
                });
                murdered.push(victim);
                present.retain(|entity| *entity != victim);
                if event.rolled.is_some() {
                    // The ladder's line names the roll; the writes below are
                    // v1's unchanged.
                    out.lines.push(format!(
                        "{} killed {} - pressure {} at powder keg, roll {} of {}, regard {} \
                         < {}, share {}->{}",
                        names(record.killer),
                        names(victim),
                        event.pressure.total,
                        event.rolled.unwrap_or(0),
                        event.die,
                        record.regard,
                        tuning.k_loyal,
                        record.share_before,
                        record.share_after,
                    ));
                    out.events.push(EventCard {
                        kind: EventKind::Kill,
                        text: format!("{} turned on {}.", names(record.killer), names(victim)),
                        sub: Some(format!(
                            "pressure {} - roll {} of {} - regard {} < {} - share {}g -> {}g",
                            event.pressure.total,
                            event.rolled.unwrap_or(0),
                            event.die,
                            record.regard,
                            tuning.k_loyal,
                            record.share_before,
                            record.share_after,
                        )),
                    });
                } else {
                    // v1's line, byte for byte — the deterministic variant is
                    // preserved, not reimplemented.
                    out.lines.push(format!(
                        "{} killed {} - desperation {} >= {}, share {}->{}, regard {} < {}",
                        names(record.killer),
                        names(victim),
                        record.desperation,
                        tuning.k_kill,
                        record.share_before,
                        record.share_after,
                        record.regard,
                        tuning.k_loyal,
                    ));
                    out.events.push(EventCard {
                        kind: EventKind::Kill,
                        text: format!("{} turned on {}.", names(record.killer), names(victim)),
                        sub: Some(format!(
                            "desperation {} >= {} - regard {} < {} - share {}g -> {}g",
                            record.desperation,
                            tuning.k_kill,
                            record.regard,
                            tuning.k_loyal,
                            record.share_before,
                            record.share_after,
                        )),
                    });
                }
                out.betrayals.push(record);
            }
            Rung::Abandon => {
                deserters.push(event.who);
                present.retain(|entity| *entity != event.who);
                out.lines.push(format!(
                    "{} walked out mid-quest - pressure {}, roll {} of {}",
                    names(event.who),
                    event.pressure.total,
                    event.rolled.unwrap_or(0),
                    event.die,
                ));
                out.events.push(EventCard {
                    kind: EventKind::Kill,
                    text: format!("{} walked out mid-quest.", names(event.who)),
                    sub: Some(format!(
                        "pressure {} - roll {} of {} - no share, and the job re-counts its \
                         hands",
                        event.pressure.total,
                        event.rolled.unwrap_or(0),
                        event.die,
                    )),
                });
            }
            Rung::Skim => {
                skimmers.push(event.who);
                out.lines.push(format!(
                    "{} skimmed the pot - pressure {}, roll {} of {}",
                    names(event.who),
                    event.pressure.total,
                    event.rolled.unwrap_or(0),
                    event.die,
                ));
                out.events.push(EventCard {
                    kind: EventKind::Kill,
                    text: format!("{} skimmed the pot.", names(event.who)),
                    sub: Some(format!(
                        "pressure {} - roll {} of {} - a share off the top before the split",
                        event.pressure.total,
                        event.rolled.unwrap_or(0),
                        event.die,
                    )),
                });
            }
            Rung::Sabotage => {
                saboteurs.push(event.who);
                let damage = ladder::sabotage_damage(tuning, dungeon.pot);
                out.lines.push(format!(
                    "{} soured the job - pressure {}, roll {} of {}, the pot loses {} of {}",
                    names(event.who),
                    event.pressure.total,
                    event.rolled.unwrap_or(0),
                    event.die,
                    damage,
                    dungeon.pot,
                ));
                out.events.push(EventCard {
                    kind: EventKind::Kill,
                    text: format!("{} soured the job.", names(event.who)),
                    sub: Some(format!(
                        "pressure {} - roll {} of {} - the pot loses {damage}g of {}g",
                        event.pressure.total,
                        event.rolled.unwrap_or(0),
                        event.die,
                        dungeon.pot,
                    )),
                });
            }
        }
    }
    // Absence of an event is also information (UI.md §3).
    if out.rungs.is_empty() {
        out.events.push(EventCard {
            kind: EventKind::Word,
            text: "No blood spilled. Everyone walked back out.".to_owned(),
            sub: None,
        });
    }
    out.survivors = party
        .iter()
        .copied()
        .filter(|member| !murdered.contains(member) && !deserters.contains(member))
        .collect();

    // --- the desertion re-evaluation (DESIGN §8c) --------------------------
    //
    // The quest's success re-evaluates against the remaining party: headcount
    // and predicates, with the murdered still counted — they did the work
    // before they died, which is v1's own semantics kept.
    let remaining: Vec<Entity> = party
        .iter()
        .copied()
        .filter(|member| !deserters.contains(member))
        .collect();
    out.failed = !deserters.is_empty()
        && (remaining.len() < dungeon.headcount || !dungeon.requires.met(social, &remaining));
    if out.failed {
        out.cut_taken = 0;
        out.lines.push(format!(
            "{} failed - {} of {} hands left to finish it",
            dungeon.name,
            remaining.len(),
            dungeon.headcount,
        ));
    }

    // --- payout, through the sabotage and the skim -------------------------
    let damage = i32::try_from(saboteurs.len()).unwrap_or(i32::MAX)
        * ladder::sabotage_damage(tuning, dungeon.pot);
    let pot_after = (dungeon.pot - damage).max(0);
    let survivor_count = i32::try_from(out.survivors.len()).unwrap_or(i32::MAX);
    let skimmer_count = i32::try_from(skimmers.len()).unwrap_or(i32::MAX);
    let (share, skim) = if out.failed {
        (0, 0)
    } else {
        ladder::skim_shares(pot_after, dungeon.cut, survivor_count, skimmer_count)
    };
    if !out.failed {
        for member in out.survivors.iter().copied() {
            let skimmed = skimmers.contains(&member);
            let amount = share + if skimmed { skim } else { 0 };
            out.payouts.push((member, amount));
            if skimmed {
                out.lines.push(format!(
                    "{} takes {} - a split of {} plus the {} they skimmed",
                    names(member),
                    amount,
                    share,
                    skim,
                ));
            } else {
                out.lines.push(format!(
                    "{} takes {} - {} split {} way{}",
                    names(member),
                    amount,
                    (pot_after - dungeon.cut - skim * skimmer_count).max(0),
                    survivor_count,
                    if survivor_count == 1 { "" } else { "s" },
                ));
            }
        }
    }
    // --- bond drift and the clean-job count ---------------------------
    //
    // "Shared success without betrayal raises mutual regard between all
    // surviving pairs" (DESIGN §6). Read per *run* rather than per pair: a
    // job somebody was killed, robbed or walked out on is not a job the
    // survivors got closer on — and not a job anybody's clean-job count moves
    // for either.
    if out.rungs.is_empty() && !out.failed {
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
        // The counting is the light side's pen (DESIGN §5): enough clean jobs
        // and everyone knows this one comes back clean.
        for member in survivors {
            let before = social.member(member).map_or(0, |member| member.clean_jobs);
            let after = before + 1;
            out.clean_job_changes.push((member, before, after));
            if after >= tuning.reliable_after && !social.marked(member, MarkId::Reliable) {
                write_mark(&mut out, social, member, MarkId::Reliable);
                out.lines.push(format!(
                    "{} is marked reliable - {after} clean jobs",
                    names(member)
                ));
                out.events.push(EventCard {
                    kind: EventKind::Word,
                    text: format!("{} is somebody you can send now.", names(member)),
                    sub: Some(format!(
                        "marked reliable - {after} clean jobs walked away from clean"
                    )),
                });
            }
        }
    }

    // --- what a betrayal costs the betrayer ---------------------------
    //
    // The lower rungs first, in event order: the mark, and the regard of the
    // people it wronged. Murder's block below is v1's, unchanged.
    for event in out.rungs.clone() {
        let (mark, wronged): (MarkId, Vec<Entity>) = match event.rung {
            // The shorted: everybody still splitting the pot.
            Rung::Skim => (
                MarkId::Skimmer,
                out.survivors
                    .iter()
                    .copied()
                    .filter(|member| *member != event.who)
                    .collect(),
            ),
            // Those left holding the job.
            Rung::Abandon => (
                MarkId::Deserter,
                out.survivors
                    .iter()
                    .copied()
                    .filter(|member| *member != event.who)
                    .collect(),
            ),
            Rung::Sabotage => (
                MarkId::Saboteur,
                out.survivors
                    .iter()
                    .copied()
                    .filter(|member| *member != event.who)
                    .collect(),
            ),
            Rung::Murder => continue,
        };
        if !social.marked(event.who, mark)
            && !out
                .mark_writes
                .iter()
                .any(|(who, written)| *who == event.who && *written == mark)
        {
            write_mark(&mut out, social, event.who, mark);
            out.lines.push(format!(
                "{} is marked {} - {}",
                names(event.who),
                mark.name(),
                match event.rung {
                    Rung::Skim => "a shorted split is public",
                    Rung::Abandon => "an empty place in the line is public",
                    _ => "a soured job is public",
                },
            ));
        }
        let drop = event.rung.def().grudge;
        if drop > 0 {
            for holder in wronged {
                let before = current_regard(&out, social, holder, event.who);
                push_regard(&mut out, social, holder, event.who, -drop);
                out.lines.push(format!(
                    "{} holds it against {} - regard {}->{}",
                    names(holder),
                    names(event.who),
                    before,
                    before - drop,
                ));
                out.drift.push(DriftLine {
                    tone: DriftTone::Regard,
                    text: format!(
                        "{}: regard toward {} {}->{}",
                        names(holder),
                        names(event.who),
                        before,
                        before - drop
                    ),
                });
            }
        }
    }
    for betrayal in out.betrayals.clone() {
        // The murder writes the mark (DESIGN §5): qualitative, public, and on
        // the sheet from here on. Written once — a mark is a fact, not a
        // counter.
        if !social.marked(betrayal.killer, MarkId::ComradeKiller)
            && !out
                .mark_writes
                .iter()
                .any(|(who, mark)| *who == betrayal.killer && *mark == MarkId::ComradeKiller)
        {
            write_mark(&mut out, social, betrayal.killer, MarkId::ComradeKiller);
            out.lines.push(format!(
                "{} is marked comrade-killer - a witnessed kill is public",
                names(betrayal.killer)
            ));
            out.events.push(EventCard {
                kind: EventKind::Word,
                text: format!("Word gets out about {}.", names(betrayal.killer)),
                sub: Some("marked comrade-killer - every sheet shows it from here on".to_owned()),
            });
        }
        // Each surviving witness holds it against the killer personally, and
        // holds it harder if they were bonded to the victim: relationships are
        // what make events travel (DESIGN §6).
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
    }

    // The payout is the last card, after what the betrayals cost: a player
    // reads the column in the order the rules fired, and the money is what the
    // whole column was for.
    if out.failed {
        out.events.push(EventCard {
            kind: EventKind::Coin,
            text: "The job fell apart. Nobody gets paid.".to_owned(),
            sub: Some(format!(
                "abandoned mid-quest - {} of {} hands left to finish it",
                remaining.len(),
                dungeon.headcount,
            )),
        });
    } else {
        out.events.push(EventCard {
            kind: EventKind::Coin,
            text: format!("Your cut: {}g. Each survivor takes {share}g.", dungeon.cut),
            sub: Some(if damage > 0 || skimmer_count > 0 {
                format!(
                    "pot {}g - spoiled {damage}g - skimmed {}g - cut {}g = {}g split \
                     {survivor_count} way{}",
                    dungeon.pot,
                    skim * skimmer_count,
                    dungeon.cut,
                    (pot_after - dungeon.cut - skim * skimmer_count).max(0),
                    if survivor_count == 1 { "" } else { "s" },
                )
            } else {
                format!(
                    "pot {}g - cut {}g = {}g split {survivor_count} way{}",
                    dungeon.pot,
                    dungeon.cut,
                    (dungeon.pot - dungeon.cut).max(0),
                    if survivor_count == 1 { "" } else { "s" },
                )
            }),
        });
    }

    // --- round-end desperation drift ----------------------------------
    //
    // Every living roster member, not only the party: non-participants do not
    // profit, so the roster decays toward willingness and refusal is always
    // temporary (DESIGN §11). A deserter is alive, unpaid, and gets hungrier —
    // their hunger still rises (DESIGN §8c).
    for member in &social.members {
        if !member.alive || murdered.contains(&member.entity) {
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
        } else if deserters.contains(&member.entity) {
            "walked out empty-handed"
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

/// Record a mark being written, with its ledger line.
fn write_mark(out: &mut Resolution, social: &Social, who: Entity, mark: MarkId) {
    out.mark_writes.push((who, mark));
    out.drift.push(DriftLine {
        tone: DriftTone::Mark,
        text: format!("{} marked {}", social.name(who), mark.name()),
    });
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
    for (who, mark) in resolution.mark_writes.iter().copied() {
        if let Some(marks) = world.find_component_mut::<Marks>(who)
            && !marks.0.contains(&mark)
        {
            marks.0.push(mark);
        }
    }
    for (who, _, after) in resolution.clean_job_changes.iter().copied() {
        if let Some(jobs) = world.find_component_mut::<CleanJobs>(who) {
            jobs.0 = after;
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
/// Read pass then write pass (DESIGN §13): the query that finds the edge borrows
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
