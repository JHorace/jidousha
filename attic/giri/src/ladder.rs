//! The betrayal ladder: seeded rolls over pressure-gated rungs (DESIGN.md §8).
//!
//! In **roster order**, each party member rolls once against their pressure
//! (occurrence), and on occurrence a second bounded roll picks a rung from the
//! ones *available at that pressure*, weighted by trait biases. Randomness
//! decides *whether and how bad*; **target selection stays deterministic** —
//! the dice never choose the victim, the relationships do (murder's target is
//! the v1 rule, verbatim).
//!
//! **Murder is structurally gated, not merely rare**: the murder rung is
//! absent from the severity roll below `powder_keg_at` — the same constant the
//! band chip's powder-keg cutoff reads — so "visibly telegraphed before it can
//! happen" is a property of the model, and the sweep asserts it as *exactly
//! zero* murders below the floor, never as a small number.
//!
//! **This is the only file that reads the `Rng`**, and it is called from
//! resolution alone. Pressure (what the roll is rolled against) is computed in
//! `pressure.rs` from start-of-resolution state; at most one event per member;
//! a member murdered before their turn never rolls, and a member bonded
//! (`regard >= K_loyal`) to every partymate holds the line and never rolls
//! either (DESIGN §6: a bond suppresses betrayal).

use jidousha::prelude::*;

use crate::beats::Dungeon;
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::model::{Social, share_each};
use crate::pressure::{self, Pressure};
use crate::traits::TraitId;

/// The four rungs, in severity order (DESIGN §8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rung {
    /// Takes an extra share from the pot before the split.
    Skim,
    /// Walks mid-quest; the quest's success re-evaluates without them.
    Abandon,
    /// The pot is damaged and the job soured.
    Sabotage,
    /// The v1 event, unchanged in its writes; the rare summit.
    Murder,
}

/// One rung's row: its gate, its base weight in the severity roll, and the
/// regard hit it costs with the people it wronged.
#[derive(Clone, Copy, Debug)]
pub struct RungDef {
    /// Which rung.
    pub rung: Rung,
    /// The word sheets and reports use.
    pub name: &'static str,
    /// The pressure at which the rung becomes available. Murder has no row
    /// number here — its floor is `Tuning::powder_keg_at`, read at the gate,
    /// so the band cutoff and the murder floor cannot be two numbers.
    pub floor: Option<i32>,
    /// Base weight in the severity roll.
    pub weight: i32,
    /// What the wronged drop toward the betrayer. Murder's is the v1 witness
    /// machinery (`witness_grudge`/`bonded_grudge`), not this column.
    pub grudge: i32,
}

/// The ladder, as data. Severity order; the sweep asserts skims dominate.
pub const RUNGS: &[RungDef] = &[
    RungDef {
        rung: Rung::Skim,
        name: "skim",
        floor: Some(0),
        weight: 8,
        grudge: 1,
    },
    RungDef {
        rung: Rung::Abandon,
        name: "abandon",
        floor: Some(3),
        weight: 3,
        grudge: 2,
    },
    RungDef {
        rung: Rung::Sabotage,
        name: "sabotage",
        floor: Some(6),
        weight: 2,
        grudge: 3,
    },
    RungDef {
        rung: Rung::Murder,
        name: "murder",
        floor: None,
        weight: 1,
        grudge: 0,
    },
];

impl Rung {
    /// This rung's row.
    pub fn def(self) -> &'static RungDef {
        RUNGS
            .iter()
            .find(|def| def.rung == self)
            .unwrap_or(&RUNGS[0])
    }
}

/// One trait's thumb on the severity roll (DESIGN §8: greedy skims, craven
/// abandons, vengeful escalates). Data, like every trait effect.
#[derive(Clone, Copy, Debug)]
pub struct SeverityBias {
    /// Whose trait.
    pub trait_id: TraitId,
    /// Which rung it leans toward.
    pub rung: Rung,
    /// Added to that rung's weight when this member rolls severity.
    pub delta: i32,
}

/// The severity-bias table.
pub const SEVERITIES: &[SeverityBias] = &[
    SeverityBias {
        trait_id: TraitId::Greedy,
        rung: Rung::Skim,
        delta: 4,
    },
    SeverityBias {
        trait_id: TraitId::Craven,
        rung: Rung::Abandon,
        delta: 4,
    },
    SeverityBias {
        trait_id: TraitId::Vengeful,
        rung: Rung::Sabotage,
        delta: 2,
    },
    SeverityBias {
        trait_id: TraitId::Vengeful,
        rung: Rung::Murder,
        delta: 2,
    },
];

/// One betrayal event, with every number the rolls looked at — what the
/// narration prints and the sweep tallies.
#[derive(Clone, Copy, Debug)]
pub struct RungEvent {
    /// Who broke.
    pub who: Entity,
    /// How badly.
    pub rung: Rung,
    /// The pressure the occurrence roll was rolled against.
    pub pressure: Pressure,
    /// The occurrence roll (`None` for the deterministic variant's events,
    /// which no die produced — the narration keys off this, not off a
    /// variant flag).
    pub rolled: Option<u32>,
    /// The die it was rolled on.
    pub die: i32,
    /// Murder's victim — picked by the v1 deterministic rule, never by dice.
    pub victim: Option<Entity>,
    /// `regard(who -> victim)` at the pick, for the narration.
    pub victim_regard: i32,
    /// The v1 rule's own record, when that rule produced this event (the
    /// deterministic variant) — the numbers its narration line is preserved
    /// from, byte for byte.
    pub v1: Option<crate::model::Betrayal>,
}

/// Whether this rung is on the table at this pressure — **the murder gate**.
///
/// Murder reads `powder_keg_at` itself: the same constant the band chip's top
/// cutoff is, so the model cannot reach murder without the chip having said
/// powder keg. A gate on availability, not a low weight (DESIGN §8).
pub fn available(tuning: &Tuning, rung: Rung, pressure: i32) -> bool {
    match rung.def().floor {
        Some(floor) => pressure >= floor,
        None => pressure >= tuning.powder_keg_at,
    }
}

/// Murder's target under the v1 rule, against the members still present: in
/// roster order, the first other member whose death raises the killer's share
/// and whose regard sits under `K_loyal`. `None` means murder is off the
/// table for this member — infeasible, not improbable.
pub fn murder_target(
    social: &Social,
    tuning: &Tuning,
    job: &Dungeon,
    present: &[Entity],
    killer: Entity,
) -> Option<(Entity, i32)> {
    let count = i32::try_from(present.len()).unwrap_or(i32::MAX);
    let profitable = share_each(job.pot, job.cut, count - 1) > share_each(job.pot, job.cut, count);
    if !profitable {
        return None;
    }
    present
        .iter()
        .copied()
        .filter(|victim| *victim != killer)
        .map(|victim| (victim, social.regard(killer, victim)))
        .find(|(_, regard)| *regard < tuning.k_loyal)
}

/// The severity weights this member rolls over: every available rung, its base
/// weight plus the member's trait biases, floored at zero — and murder only
/// when a target exists.
fn weights(
    social: &Social,
    tuning: &Tuning,
    job: &Dungeon,
    present: &[Entity],
    who: Entity,
    pressure: i32,
) -> Vec<(Rung, i32)> {
    let traits = social.traits(who);
    RUNGS
        .iter()
        .filter(|def| available(tuning, def.rung, pressure))
        .filter(|def| {
            def.rung != Rung::Murder || murder_target(social, tuning, job, present, who).is_some()
        })
        .map(|def| {
            let bias: i32 = SEVERITIES
                .iter()
                .filter(|row| row.rung == def.rung && traits.contains(&row.trait_id))
                .map(|row| row.delta)
                .sum();
            (def.rung, (def.weight + bias).max(0))
        })
        .filter(|(_, weight)| *weight > 0)
        .collect()
}

/// **The rolls** (DESIGN §8): roster order, one occurrence roll per member
/// against their pressure, one bounded severity roll on occurrence. Start-of-
/// resolution state throughout; the `present` set shrinks as murders and
/// desertions land, which is the only context later members inherit.
pub fn roll_events(
    social: &Social,
    tuning: &Tuning,
    job: &Dungeon,
    party: &[Entity],
    pressures: &[Pressure],
    rng: &mut Rng,
) -> Vec<RungEvent> {
    let mut events = Vec::new();
    // Betrayal needs somebody to betray.
    if party.len() < 2 {
        return events;
    }
    let mut present: Vec<Entity> = party.to_vec();
    for member in party.iter().copied() {
        if !present.contains(&member) {
            continue;
        }
        // A member bonded to everybody still here holds the line (DESIGN §6).
        let bonded_to_all = present
            .iter()
            .all(|other| *other == member || social.regard(member, *other) >= tuning.k_loyal);
        if bonded_to_all {
            continue;
        }
        let Some(pressure) = pressures.iter().find(|p| p.who == member).copied() else {
            continue;
        };
        let chance = pressure.total - tuning.occurrence_calm;
        if chance <= 0 || tuning.occurrence_die <= 0 {
            continue;
        }
        let rolled = rng.below(tuning.occurrence_die.unsigned_abs());
        if i64::from(rolled) >= i64::from(chance) {
            continue;
        }
        let table = weights(social, tuning, job, &present, member, pressure.total);
        let total: i32 = table.iter().map(|(_, weight)| weight).sum();
        if total <= 0 {
            continue;
        }
        let mut pick = rng.below(total.unsigned_abs()) as i32;
        let mut chosen = table.first().map(|(rung, _)| *rung).unwrap_or(Rung::Skim);
        for (rung, weight) in &table {
            if pick < *weight {
                chosen = *rung;
                break;
            }
            pick -= weight;
        }
        let (victim, victim_regard) = if chosen == Rung::Murder {
            match murder_target(social, tuning, job, &present, member) {
                Some((victim, regard)) => (Some(victim), regard),
                None => (None, 0),
            }
        } else {
            (None, 0)
        };
        events.push(RungEvent {
            who: member,
            rung: chosen,
            pressure,
            rolled: Some(rolled),
            die: tuning.occurrence_die,
            victim,
            victim_regard,
            v1: None,
        });
        match chosen {
            Rung::Abandon => present.retain(|entity| *entity != member),
            Rung::Murder => {
                if let Some(victim) = victim {
                    present.retain(|entity| *entity != victim);
                }
            }
            _ => {}
        }
    }
    events
}

/// The skim's pot arithmetic (DESIGN §8): each skimmer takes one share off the
/// top before the split. Returns `(everyone's split share, the skim itself)`.
pub fn skim_shares(pot: i32, cut: i32, paid: i32, skimmers: i32) -> (i32, i32) {
    let skim = share_each(pot, cut, paid);
    if paid <= 0 {
        return (0, 0);
    }
    let pool = (pot - cut - skim * skimmers).max(0);
    (pool / paid, skim)
}

/// What a sabotage destroys: the named fraction, in twelfths of the pot.
pub fn sabotage_damage(tuning: &Tuning, pot: i32) -> i32 {
    (pot * tuning.sabotage_loss.clamp(0, 12)) / 12
}

/// The ladder's own battery: the gates, the target rule, and the rung
/// arithmetic, asked without dice wherever a question is deterministic.
pub fn battery(checks: &mut Checks, tuning: &Tuning) {
    use crate::contracts::{bench, bench_job};

    // --- the murder gate is the band cutoff, structurally ------------------
    checks.require(
        !available(tuning, Rung::Murder, tuning.powder_keg_at - 1)
            && available(tuning, Rung::Murder, tuning.powder_keg_at),
        "murder is reachable below the powder-keg cutoff",
        format!(
            "at pressure {} murder is {}available and at {} it is {}available; the floor is \
             the band cutoff itself",
            tuning.powder_keg_at - 1,
            if available(tuning, Rung::Murder, tuning.powder_keg_at - 1) {
                ""
            } else {
                "not "
            },
            tuning.powder_keg_at,
            if available(tuning, Rung::Murder, tuning.powder_keg_at) {
                ""
            } else {
                "not "
            },
        ),
    );
    // The lower rungs sit on their own stated floors.
    for def in RUNGS {
        if let Some(floor) = def.floor {
            checks.require(
                available(tuning, def.rung, floor)
                    && (floor == 0 || !available(tuning, def.rung, floor - 1)),
                "a rung's availability is not its stated floor",
                format!("{} has a floor of {floor}", def.name),
            );
        }
    }

    // --- the target rule: deterministic, and the dice never touch it -------
    let (world, ids) = bench(
        &[
            ("Kil", 0, &[], &[]),
            ("Dear", 0, &[], &[]),
            ("Mark", 0, &[], &[]),
        ],
        &[(0, 1, tuning.k_loyal)],
    );
    let social = Social::read(&world.view());
    let job = bench_job(3, 12, 0);
    let target = murder_target(&social, tuning, &job, &ids, ids[0]);
    checks.require(
        target.map(|(victim, _)| victim) == Some(ids[2]),
        "murder's target is not the v1 rule",
        format!(
            "with the second name protected at K_loyal {}, the target came out {:?}; the rule \
             is the first name in roster order under the bar",
            tuning.k_loyal,
            target.map(|(victim, _)| social.name(victim)),
        ),
    );
    // No profit, no target: a pot the cut eats leaves murder infeasible.
    let dry = bench_job(3, 2, 2);
    checks.require(
        murder_target(&social, tuning, &dry, &ids, ids[0]).is_none(),
        "a murder with nothing to gain found a target anyway",
        "a pot of 2 under a cut of 2 leaves no share to grow".to_owned(),
    );
    // Everybody protected: infeasible even at powder-keg pressure.
    let (world, ids) = bench(
        &[("Kil", 9, &[], &[]), ("Dear", 0, &[], &[])],
        &[(0, 1, tuning.k_loyal)],
    );
    let social = Social::read(&world.view());
    checks.require(
        murder_target(&social, tuning, &job, &ids, ids[0]).is_none(),
        "regard at K_loyal no longer protects a would-be victim",
        format!("the only partymate is held at {}", tuning.k_loyal),
    );

    // --- the rung arithmetic ------------------------------------------------
    let (each, skim) = skim_shares(8, 2, 2, 1);
    checks.require(
        skim == 3 && each == 1,
        "the skim's pot arithmetic moved",
        format!(
            "a pot of 8 less a cut of 2 between 2 with one skimmer pays {each} each and the \
             skim is {skim}; the skimmer takes a share off the top and the rest split what \
             is left"
        ),
    );
    let (clean_each, _) = skim_shares(8, 2, 2, 0);
    checks.require(
        clean_each == share_each(8, 2, 2),
        "the split with no skimmers is not the plain split",
        format!("{clean_each} against {}", share_each(8, 2, 2)),
    );
    checks.require(
        sabotage_damage(tuning, 12) == tuning.sabotage_loss && sabotage_damage(tuning, 0) == 0,
        "the sabotage's named fraction is not twelfths of the pot",
        format!(
            "a pot of 12 loses {} at a sabotage_loss of {}",
            sabotage_damage(tuning, 12),
            tuning.sabotage_loss
        ),
    );
    checks.require(
        tuning.sabotage_loss <= 0 || sabotage_damage(tuning, 12) > 0,
        "a sabotage with a stated loss destroys nothing",
        format!(
            "sabotage_loss is {} and a pot of 12 loses {}; a rung whose consequence is a no-op \
             is a silent failure path",
            tuning.sabotage_loss,
            sabotage_damage(tuning, 12)
        ),
    );

    // --- one sabotage, at a fixed seed, with hard numbers -------------------
    //
    // No beat sabotages at its authored seed, so the fraction's arithmetic is
    // pinned here: a vengeful member at pressure 7 sours a 6g job at seed 1,
    // and the pot loses exactly 3 — the shipped fraction's half, stated as a
    // number so a moved `sabotage_loss` breaks something (the beats pin the
    // other constants the same way).
    let (world, ids) = bench(
        &[("Sly", 7, &[TraitId::Vengeful], &[]), ("Vic", 0, &[], &[])],
        &[(0, 1, -1)],
    );
    let social = Social::read(&world.view());
    let sab_job = bench_job(2, 6, 6);
    let mut rng = Rng::from_seed(1);
    let soured = crate::resolve::resolve(
        &social,
        tuning,
        crate::variant::VariantId::Ladder,
        &sab_job,
        &ids,
        &mut rng,
    );
    checks.require(
        soured
            .rungs
            .iter()
            .any(|event| event.who == ids[0] && event.rung == Rung::Sabotage),
        "the fixed-seed sabotage did not happen",
        format!(
            "seed 1 on the sabotage bench produced {:?}; a pressure constant or a severity \
             weight moved",
            soured
                .rungs
                .iter()
                .map(|event| event.rung.def().name)
                .collect::<Vec<_>>()
        ),
    );
    checks.require(
        soured
            .lines
            .iter()
            .any(|line| line.contains("the pot loses 3 of 6")),
        "the sabotage's named fraction is not the shipped half",
        format!("the narration was {:?}", soured.lines),
    );
    checks.require(
        soured
            .mark_writes
            .iter()
            .any(|(who, mark)| *who == ids[0] && *mark == crate::traits::MarkId::Saboteur),
        "a sabotage did not write the saboteur mark",
        format!("the writes were {:?}", soured.mark_writes),
    );
    checks.require(
        soured.regard_changes.iter().any(|change| {
            change.from == ids[1]
                && change.to == ids[0]
                && change.after - change.before == -Rung::Sabotage.def().grudge
        }),
        "a sabotage cost the saboteur nothing with the people it wronged",
        format!("the regard changes were {:?}", soured.regard_changes),
    );

    // --- the rolls' structural guarantees, across seeds ---------------------
    // A solo party never betrays; a fully-bonded member never rolls; nobody
    // gets two events; and no murder ever lands below the floor. Forty seeds
    // of a hot bench is not a distribution claim (the sweep owns those) - it
    // is the structure exercised under dice at all.
    let (world, solo) = bench(&[("Alone", 12, &[], &[])], &[]);
    let social = Social::read(&world.view());
    let solo_job = bench_job(1, 12, 0);
    for seed in 0..8 {
        let mut rng = Rng::from_seed(seed);
        let pressures = pressure::party(&social, tuning, &solo, &solo_job);
        let events = roll_events(&social, tuning, &solo_job, &solo, &pressures, &mut rng);
        checks.require(
            events.is_empty(),
            "a party of one betrayed somebody",
            format!(
                "seed {seed} produced {} event(s) for a lone member",
                events.len()
            ),
        );
    }
    let (world, ids) = bench(
        &[
            ("Hot", 9, &[], &[]),
            ("True", 9, &[], &[]),
            ("Vic", 9, &[], &[]),
        ],
        &[(1, 0, tuning.k_loyal), (1, 2, tuning.k_loyal)],
    );
    let social = Social::read(&world.view());
    let hot_job = bench_job(3, 12, 0);
    let pressures = pressure::party(&social, tuning, &ids, &hot_job);
    let mut any_event = false;
    for seed in 0..40 {
        let mut rng = Rng::from_seed(seed);
        let events = roll_events(&social, tuning, &hot_job, &ids, &pressures, &mut rng);
        any_event |= !events.is_empty();
        for member in &ids {
            checks.require(
                events.iter().filter(|event| event.who == *member).count() <= 1,
                "a member betrayed twice in one quest",
                format!(
                    "seed {seed}: {} has {} events; the ladder is one event per member at most",
                    social.name(*member),
                    events.iter().filter(|event| event.who == *member).count()
                ),
            );
        }
        checks.require(
            !events.iter().any(|event| event.who == ids[1]),
            "a member bonded to the whole party betrayed it",
            format!(
                "seed {seed}: the fully-bonded member rolled an event; regard at K_loyal \
                 toward everybody is the suppression DESIGN §6 names"
            ),
        );
        for event in &events {
            checks.require(
                event.rung != Rung::Murder || event.pressure.total >= tuning.powder_keg_at,
                "a murder landed below the powder-keg floor",
                format!(
                    "seed {seed}: a murder at pressure {}; the floor is {}",
                    event.pressure.total, tuning.powder_keg_at
                ),
            );
        }
    }
    checks.require(
        any_event || tuning.occurrence_die <= 0 || tuning.occurrence_calm >= 12,
        "forty seeds of a powder-keg bench produced no betrayal at all",
        "the hot bench exists to exercise the rolls; if it never fires, the dice are not \
         being read"
            .to_owned(),
    );
}
