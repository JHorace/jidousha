//! The social state: what a character is, and the one snapshot every decision
//! reads (DESIGN.md §3).
//!
//! Characters are entities and their state is components; **regard edges are
//! entities too** — `RegardEdge { from, to, value }` — the ECS answer for a
//! sparse directed relation (DESIGN §13). Every query over them is read-pass
//! then write-pass: collect what is needed into a `Social` snapshot, drop the
//! query, then apply.
//!
//! v2 splits the vocabulary from the state: `traits.rs` owns what a trait or a
//! mark *is*, `willing.rs` owns the decision function that reads them, and
//! this file owns what a character *has* — desperation and its source, wealth,
//! traits, marks, the clean-job count, and the edges. The v1 public scalar is
//! gone (DESIGN §5: marks replace it).
//!
//! No randomness anywhere. P1's outcome is a pure function of (beat state,
//! player assignments, tuning constants); the engine's `Rng` exists and giri
//! never reads it — seeds enter with P2, via the engine `Rng` only.

use jidousha::prelude::*;

use crate::traits::{MarkId, TraitId};

/// A character, and where they sit in the beat's roster.
///
/// `roster_index` is the *stated* order — betrayal is evaluated in it, and a
/// query's iteration order is deterministic but not sorted, so every walk over
/// the roster sorts by this rather than by whatever the world hands back.
#[derive(Clone, Copy, Debug)]
pub struct Character {
    /// The name, ASCII, as the sheets draw it.
    pub name: &'static str,
    /// Position in the beat's roster: the betrayal evaluation order.
    pub roster_index: usize,
}
impl Component for Character {}

/// Need. The willingness opener and the betrayal motive.
#[derive(Clone, Copy, Debug)]
pub struct Desperation(pub i32);
impl Component for Desperation {}

/// Why the need presses — bound at character generation (DESIGN §3).
///
/// Flavor-plus-data this phase: the goal machinery that makes a source
/// mechanical is P3, and the sheet showing *why* two identical numbers are two
/// different problems is what the field buys now.
#[derive(Clone, Copy, Debug)]
pub struct Source(pub &'static str);
impl Component for Source {}

/// What profit accumulates into.
#[derive(Clone, Copy, Debug)]
pub struct Wealth(pub i32);
impl Component for Wealth {}

/// Who this character is: at most `TRAIT_CAP` ids from the vocabulary.
#[derive(Clone, Debug, Default)]
pub struct Traits(pub Vec<TraitId>);
impl Component for Traits {}

/// What everyone knows this character did (DESIGN §5). Plural, earned, hard
/// to lose — no eraser exists until goals land (P3).
#[derive(Clone, Debug, Default)]
pub struct Marks(pub Vec<MarkId>);
impl Component for Marks {}

/// Clean jobs walked away from, counting toward the *reliable* mark.
#[derive(Clone, Copy, Debug, Default)]
pub struct CleanJobs(pub i32);
impl Component for CleanJobs {}

/// A character who was killed, and by whom.
///
/// A marker rather than a despawn: the roster still shows Steve, the report
/// still names him, and the edges pointing at him still resolve. A dead
/// character is inspectable, which is invariant 2 applied to the one state a
/// game would otherwise quietly drop.
#[derive(Clone, Copy, Debug)]
pub struct Dead {
    /// Who killed them.
    pub killed_by: Entity,
}
impl Component for Dead {}

/// One directed personal edge: what `from` thinks of `to`.
///
/// Sparse — an absent edge is zero — and asymmetric on purpose.
#[derive(Clone, Copy, Debug)]
pub struct RegardEdge {
    /// Who holds the opinion.
    pub from: Entity,
    /// Who it is about.
    pub to: Entity,
    /// Positive is a bond, negative is a grudge.
    pub value: i32,
}
impl Component for RegardEdge {}

/// One character, read out of the world.
#[derive(Clone, Debug)]
pub struct Member {
    /// The entity it was read from.
    pub entity: Entity,
    /// The name.
    pub name: &'static str,
    /// Roster order.
    pub roster_index: usize,
    /// Need.
    pub desperation: i32,
    /// Why the need presses.
    pub source: &'static str,
    /// Accumulated profit.
    pub wealth: i32,
    /// Who they are.
    pub traits: Vec<TraitId>,
    /// What everyone knows.
    pub marks: Vec<MarkId>,
    /// Clean jobs, counting toward *reliable*.
    pub clean_jobs: i32,
    /// Whether they are still alive.
    pub alive: bool,
    /// Who killed them, if they are not.
    pub killed_by: Option<Entity>,
}

/// The whole social state, read out of the world once.
///
/// The read pass of the read-pass/write-pass pattern: every decision takes one
/// of these and no world at all.
#[derive(Clone, Debug, Default)]
pub struct Social {
    /// Every character, in roster order.
    pub members: Vec<Member>,
    /// Every regard edge that exists. Absent means zero.
    pub edges: Vec<RegardEdge>,
}

impl Social {
    /// Read the world, from either phase.
    ///
    /// One reader, because the UI shows exactly what the simulation decided
    /// from: a preview that disagreed with the resolution would be a lie, and
    /// two collectors is how that starts. An Update system makes the view with
    /// `world.view()`; a Draw system already holds one as `ctx.world`
    /// (ADR-0039).
    pub fn read(world: &WorldView<'_>) -> Self {
        let mut members: Vec<Member> = world
            .query::<(&Character, &Desperation, &Source, &Wealth)>()
            .map(|(entity, character, desperation, source, wealth)| Member {
                entity,
                name: character.name,
                roster_index: character.roster_index,
                desperation: desperation.0,
                source: source.0,
                wealth: wealth.0,
                traits: Vec::new(),
                marks: Vec::new(),
                clean_jobs: 0,
                alive: true,
                killed_by: None,
            })
            .collect();
        for member in &mut members {
            if let Some(traits) = world.find_component::<Traits>(member.entity) {
                member.traits = traits.0.clone();
            }
            if let Some(marks) = world.find_component::<Marks>(member.entity) {
                member.marks = marks.0.clone();
            }
            if let Some(jobs) = world.find_component::<CleanJobs>(member.entity) {
                member.clean_jobs = jobs.0;
            }
            if let Some(dead) = world.find_component::<Dead>(member.entity) {
                member.alive = false;
                member.killed_by = Some(dead.killed_by);
            }
        }
        // Query order is deterministic but not sorted, and betrayal is
        // evaluated in *roster* order, so the sort is load-bearing rather than
        // cosmetic (docs/api: rely on "the same run twice yields the same
        // order", never on "the first one out is the one I spawned first").
        members.sort_by_key(|member| member.roster_index);
        let edges = world
            .query::<&RegardEdge>()
            .map(|(_, edge)| *edge)
            .collect();
        Self { members, edges }
    }

    /// The character behind an entity, if it is one.
    pub fn member(&self, entity: Entity) -> Option<&Member> {
        self.members.iter().find(|member| member.entity == entity)
    }

    /// A name, for narration that has to say who.
    pub fn name(&self, entity: Entity) -> &'static str {
        self.member(entity).map_or("?", |member| member.name)
    }

    /// The character with this name, if the roster has one.
    pub fn by_name(&self, name: &str) -> Option<&Member> {
        self.members.iter().find(|member| member.name == name)
    }

    /// `regard(from -> to)`. Absent is zero. Raw — the traits weigh it in
    /// `willing.rs`, and the betrayal rule reads it unweighted, as v1 did.
    pub fn regard(&self, from: Entity, to: Entity) -> i32 {
        self.edges
            .iter()
            .find(|edge| edge.from == from && edge.to == to)
            .map_or(0, |edge| edge.value)
    }

    /// Need.
    pub fn desperation(&self, who: Entity) -> i32 {
        self.member(who).map_or(0, |member| member.desperation)
    }

    /// Who they are. Empty for an entity that is not a character.
    pub fn traits(&self, who: Entity) -> Vec<TraitId> {
        self.member(who)
            .map_or_else(Vec::new, |member| member.traits.clone())
    }

    /// What everyone knows about them.
    pub fn marks(&self, who: Entity) -> Vec<MarkId> {
        self.member(who)
            .map_or_else(Vec::new, |member| member.marks.clone())
    }

    /// Whether they wear this mark.
    pub fn marked(&self, who: Entity, mark: MarkId) -> bool {
        self.member(who)
            .is_some_and(|member| member.marks.contains(&mark))
    }

    /// Everyone still alive, in roster order.
    pub fn living(&self) -> Vec<Entity> {
        self.members
            .iter()
            .filter(|member| member.alive)
            .map(|member| member.entity)
            .collect()
    }
}

/// What one survivor takes: the pot after the player's cut, split among them.
///
/// Integer division, floored, and zero for an empty party — the arithmetic the
/// whole economy is, and the reason a desperate character has a motive at all.
pub fn share_each(pot: i32, cut: i32, survivors: i32) -> i32 {
    if survivors <= 0 {
        return 0;
    }
    (pot - cut).max(0) / survivors
}

/// One killing, with every number the rule looked at.
#[derive(Clone, Copy, Debug)]
pub struct Betrayal {
    /// Who did it.
    pub killer: Entity,
    /// Who it was done to.
    pub victim: Entity,
    /// The killer's desperation, which had to reach `K_kill`.
    pub desperation: i32,
    /// Their share before.
    pub share_before: i32,
    /// Their share after.
    pub share_after: i32,
    /// `regard(killer -> victim)`, which had to be below `K_loyal`.
    pub regard: i32,
}

/// **Betrayal** (DESIGN §6's retained v1 rule, deterministic for P1 and
/// scheduled for replacement by the ladder in P2 — kept, not polished).
///
/// ```text
/// betray(c, t) iff desperation(c) >= K_kill
///            and shareGain(c | t dead) > 0
///            and regard(c->t) < K_loyal
/// ```
///
/// The order is the party's roster order at both levels, kills take effect
/// immediately, and a character killed before their own turn never evaluates.
/// The margin `willing.rs` computes is **not** an input here — strain becomes
/// a betrayal input only when the ladder lands (P2).
pub fn betrayals(
    social: &Social,
    tuning: &crate::constants::Tuning,
    party: &[Entity],
    pot: i32,
    cut: i32,
) -> Vec<Betrayal> {
    let mut alive: Vec<Entity> = party.to_vec();
    let mut done = Vec::new();
    for &killer in party {
        if !alive.contains(&killer) {
            continue;
        }
        for &victim in party {
            if victim == killer || !alive.contains(&victim) {
                continue;
            }
            let desperation = social.desperation(killer);
            let regard = social.regard(killer, victim);
            let count = i32::try_from(alive.len()).unwrap_or(i32::MAX);
            let share_before = share_each(pot, cut, count);
            let share_after = share_each(pot, cut, count - 1);
            let motivated = desperation >= tuning.k_kill;
            let profitable = share_after > share_before;
            let disloyal = regard < tuning.k_loyal;
            if motivated && profitable && disloyal {
                alive.retain(|entity| *entity != victim);
                done.push(Betrayal {
                    killer,
                    victim,
                    desperation,
                    share_before,
                    share_after,
                    regard,
                });
            }
        }
    }
    done
}
