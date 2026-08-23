//! The social model: what a character is, what regard is, and the one decision
//! function that fires at three moments (DESIGN.md §3).
//!
//! Characters are entities and their scalars are components; **regard edges are
//! entities too** — `RegardEdge { from, to, value }` — which is the ECS answer
//! for a sparse directed relation (DESIGN §9). Every query over them is
//! read-pass then write-pass: collect what is needed into a `Social` snapshot,
//! drop the query, then apply.
//!
//! Everything below the snapshot is a **free function of plain data**. Nothing
//! here touches a `World`, which is what lets the UI's willingness preview and
//! the resolution call the same `willingness` — the preview cannot disagree
//! with the simulation because there is only one of it — and what lets
//! `--verify` ask the contracts directly instead of hoping a played beat
//! reaches them.
//!
//! No randomness anywhere. v1's outcome is a pure function of (beat state,
//! player assignments, tuning constants); the engine's `Rng` resource exists
//! and giri never reads it.

use jidousha::prelude::*;

use crate::constants::Tuning;

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

/// Need. The willingness override and the betrayal motive.
#[derive(Clone, Copy, Debug)]
pub struct Desperation(pub i32);
impl Component for Desperation {}

/// Public knowledge: the global projection of witnessed acts.
#[derive(Clone, Copy, Debug)]
pub struct Infamy(pub i32);
impl Component for Infamy {}

/// What profit accumulates into.
#[derive(Clone, Copy, Debug)]
pub struct Wealth(pub i32);
impl Component for Wealth {}

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
#[derive(Clone, Copy, Debug)]
pub struct Member {
    /// The entity it was read from.
    pub entity: Entity,
    /// The name.
    pub name: &'static str,
    /// Roster order.
    pub roster_index: usize,
    /// Need.
    pub desperation: i32,
    /// Public reputation.
    pub infamy: i32,
    /// Accumulated profit.
    pub wealth: i32,
    /// Whether they are still alive.
    pub alive: bool,
    /// Who killed them, if they are not.
    pub killed_by: Option<Entity>,
}

/// The whole social state, read out of the world once.
///
/// The read pass of the read-pass/write-pass pattern: every decision below
/// takes one of these and no world at all.
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
        let rows: Vec<(Entity, Character, i32, i32, i32)> = world
            .query::<(&Character, &Desperation, &Infamy, &Wealth)>()
            .map(|(entity, character, desperation, infamy, wealth)| {
                (entity, *character, desperation.0, infamy.0, wealth.0)
            })
            .collect();
        let dead: Vec<(Entity, Entity)> = rows
            .iter()
            .filter_map(|(entity, ..)| {
                world
                    .find_component::<Dead>(*entity)
                    .map(|dead| (*entity, dead.killed_by))
            })
            .collect();
        let edges = world
            .query::<&RegardEdge>()
            .map(|(_, edge)| *edge)
            .collect();
        Self::assemble(rows, &dead, edges)
    }

    fn assemble(
        rows: Vec<(Entity, Character, i32, i32, i32)>,
        dead: &[(Entity, Entity)],
        edges: Vec<RegardEdge>,
    ) -> Self {
        let mut members: Vec<Member> = rows
            .into_iter()
            .map(|(entity, character, desperation, infamy, wealth)| {
                let killed_by = dead
                    .iter()
                    .find(|(who, _)| *who == entity)
                    .map(|(_, killer)| *killer);
                Member {
                    entity,
                    name: character.name,
                    roster_index: character.roster_index,
                    desperation,
                    infamy,
                    wealth,
                    alive: killed_by.is_none(),
                    killed_by,
                }
            })
            .collect();
        // Query order is deterministic but not sorted, and betrayal is
        // evaluated in *roster* order, so the sort is load-bearing rather than
        // cosmetic (docs/api: rely on "the same run twice yields the same
        // order", never on "the first one out is the one I spawned first").
        members.sort_by_key(|member| member.roster_index);
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

    /// `regard(from -> to)`. Absent is zero.
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

    /// Public reputation.
    pub fn infamy(&self, who: Entity) -> i32 {
        self.member(who).map_or(0, |member| member.infamy)
    }

    /// Everyone still alive, in roster order.
    pub fn living(&self) -> Vec<Entity> {
        self.members
            .iter()
            .filter(|member| member.alive)
            .map(|member| member.entity)
            .collect()
    }

    /// `incompat(c, m) = K_inf * max(0, infamy(m) - infamy(c))`.
    ///
    /// What a stranger's *worse* reputation costs. A character with infamy of
    /// their own has no gap to mind, which is the whole of why Alex joins Bob
    /// and Tim does not.
    pub fn incompat(&self, tuning: &Tuning, who: Entity, other: Entity) -> i32 {
        tuning.k_inf * (self.infamy(other) - self.infamy(who)).max(0)
    }
}

/// One partymate's contribution to a willingness sum.
#[derive(Clone, Copy, Debug)]
pub struct MemberTerm {
    /// Who the term is about.
    pub member: Entity,
    /// `regard(c -> m)`.
    pub regard: i32,
    /// `incompat(c, m)`, subtracted.
    pub incompat: i32,
}

/// A character's answer to "will you join this party", with its arithmetic.
///
/// The arithmetic is carried rather than recomputed because the UI shows the
/// refusal's terms and the simulation gates on the total: one call, one answer,
/// and no way for the preview to say something the resolution disagrees with.
#[derive(Clone, Debug)]
pub struct Willingness {
    /// Who was asked.
    pub who: Entity,
    /// Their name.
    pub name: &'static str,
    /// The desperation that opened the sum.
    pub desperation: i32,
    /// The sum of regard toward the rest of the party.
    pub regard_total: i32,
    /// The sum of incompatibility with the rest of the party.
    pub incompat_total: i32,
    /// `desperation + regard_total - incompat_total`.
    pub total: i32,
    /// One term per other party member, in roster order.
    pub terms: Vec<MemberTerm>,
}

impl Willingness {
    /// Joins iff willingness >= 0.
    pub fn joins(&self) -> bool {
        self.total >= 0
    }

    /// The sum as the UI and the report print it.
    ///
    /// Handed back as a string so a check can ask the game for the exact text
    /// it draws: no assertion over drawn quads can see a wrong character.
    ///
    /// **The compact form is the only form** (UI.md §5, rung 1: "the status
    /// line shows the raw sum"). A party card is fourteen columns wide, so a
    /// spelled-out `des 5 + regard 2 - incompat 4 = 3` wraps to three lines and
    /// a second, shorter spelling for the card would be two strings for one
    /// number - which is how a preview starts disagreeing with a report. The
    /// per-member breakdown lives in `terms`, and UI.md §5's rung 2 is where it
    /// reaches the surface.
    pub fn arithmetic(&self) -> String {
        format!(
            "{}{:+}-{} = {}",
            self.desperation, self.regard_total, self.incompat_total, self.total
        )
    }
}

/// The answer at the door (DESIGN §3.2, the door rule).
///
/// Joining is gated **in both directions**: the newcomer has to consent, and
/// no incumbent's willingness may go negative. The two failures are different
/// things to a player - one is somebody saying no, the other is somebody
/// already inside saying no on their behalf - so they are different variants
/// rather than one bool, and each carries the arithmetic the UI names its
/// reason with.
#[derive(Clone, Debug)]
pub enum Admission {
    /// Both directions consent; the candidate is in.
    Admitted(Willingness),
    /// Rule 1: the newcomer will not come. Their sum.
    Refuses(Willingness),
    /// Rule 2: an incumbent would go negative and blocks the arrival.
    Blocked {
        /// Who is blocking.
        blocker: Entity,
        /// Their name, for the line that says so.
        name: &'static str,
        /// **The blocker's** sum, not the newcomer's - it is their objection.
        willingness: Willingness,
    },
}

impl Admission {
    /// Whether the candidate may be added.
    pub fn admitted(&self) -> bool {
        matches!(self, Admission::Admitted(_))
    }

    /// The status line UI.md §4 states for a character who is not in the party.
    ///
    /// Exactly one of three, and the grammar is the specification's: a bounced
    /// click surfaces this same string in the toast and in the log, so the
    /// card, the toast and the log cannot describe one refusal three ways.
    pub fn status_line(&self) -> String {
        match self {
            Admission::Admitted(entry) => format!("would join - {}", entry.arithmetic()),
            Admission::Refuses(entry) => format!("refuses - {}", entry.arithmetic()),
            Admission::Blocked {
                name, willingness, ..
            } => format!("{name} blocks - {}", willingness.arithmetic()),
        }
    }

    /// The toast a bounced click raises, naming who and why.
    pub fn bounce(&self, candidate: &str) -> Option<String> {
        match self {
            Admission::Admitted(_) => None,
            Admission::Refuses(entry) => Some(format!(
                "{candidate} refuses this company - {}",
                entry.arithmetic()
            )),
            Admission::Blocked {
                name, willingness, ..
            } => Some(format!(
                "{name} will not work with {candidate} - {}",
                willingness.arithmetic()
            )),
        }
    }
}

/// **The door** (DESIGN §3.2): may `candidate` be added to `party`?
///
/// ```text
/// admit(c, P) iff willingness(c, P + {c}) >= 0
///             and for every m in P: willingness(m, P + {c}) >= 0
/// ```
///
/// Order-symmetric about who is at the door, which is the rule's whole point:
/// Tim in the party blocks Bob by the same numbers that make Tim refuse when
/// Bob is in the party first. Incumbents are walked in roster order so the
/// *named* blocker is stable - two incumbents could both object, and a UI that
/// named whichever the query happened to hand back first would name a different
/// one on a different day.
///
/// **Consent is evaluated at the door only.** Nothing re-runs this when a
/// member leaves, so removing a bonded partner can leave a member behind whose
/// willingness is now negative. That is decided (owner, 2026-08-23): blocking
/// is more legible than members walking out, and party state staying monotonic
/// under the player's own actions is worth more than the drama.
pub fn admit(social: &Social, tuning: &Tuning, candidate: Entity, party: &[Entity]) -> Admission {
    let mut with = party.to_vec();
    if !with.contains(&candidate) {
        with.push(candidate);
    }
    let newcomer = willingness(social, tuning, candidate, &with);
    if !newcomer.joins() {
        return Admission::Refuses(newcomer);
    }
    for member in &social.members {
        if !party.contains(&member.entity) || member.entity == candidate {
            continue;
        }
        let incumbent = willingness(social, tuning, member.entity, &with);
        if !incumbent.joins() {
            return Admission::Blocked {
                blocker: member.entity,
                name: member.name,
                willingness: incumbent,
            };
        }
    }
    Admission::Admitted(newcomer)
}

/// **Willingness** (DESIGN §3.2, first firing moment): character `who` asked to
/// join `party`.
///
/// ```text
/// willingness(c, P) = desperation(c) + sum regard(c->m) - sum incompat(c, m)
/// joins iff willingness >= 0
/// ```
///
/// The sum runs over the party *other than* `c`; both self-terms are zero
/// either way (`regard(c->c)` is absent, and `incompat(c, c)` is `K_inf` times
/// `max(0, 0)`), so this is DESIGN's formula and not a variant of it.
pub fn willingness(social: &Social, tuning: &Tuning, who: Entity, party: &[Entity]) -> Willingness {
    let mut terms = Vec::new();
    for member in &social.members {
        if member.entity == who || !party.contains(&member.entity) {
            continue;
        }
        terms.push(MemberTerm {
            member: member.entity,
            regard: social.regard(who, member.entity),
            incompat: social.incompat(tuning, who, member.entity),
        });
    }
    let desperation = social.desperation(who);
    let regard_total: i32 = terms.iter().map(|term| term.regard).sum();
    let incompat_total: i32 = terms.iter().map(|term| term.incompat).sum();
    Willingness {
        who,
        name: social.name(who),
        desperation,
        regard_total,
        incompat_total,
        total: desperation + regard_total - incompat_total,
        terms,
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

/// **Betrayal** (DESIGN §3.2, second firing moment), evaluated in roster order.
///
/// ```text
/// betray(c, t) iff desperation(c) >= K_kill
///            and shareGain(c | t dead) > 0
///            and regard(c->t) < K_loyal
/// ```
///
/// **The order is the party's roster order, and it decides outcomes**, so it is
/// stated here and tested directly in `verify.rs` rather than left to whatever
/// a query hands back. `c` walks the party in roster order and, for each `c`,
/// `t` walks it in roster order too; a kill takes effect immediately, so the
/// share arithmetic every later evaluation sees is the one the earlier kills
/// left. A character killed before their own turn never evaluates — which is
/// the whole of why the order matters when two desperate characters are in one
/// party.
pub fn betrayals(
    social: &Social,
    tuning: &Tuning,
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
