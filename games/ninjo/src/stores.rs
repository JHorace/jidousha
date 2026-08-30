//! The shared state: regard edges, bonds and grudges, marks (GDD §4).
//!
//! **This is how the modules couple.** GDD §1: "coupling through shared state
//! and events only; no module reads another's interior." These three stores
//! are that shared state, and everything in waves 1 and up — the scorer,
//! petitions, asks, parties, knowledge — reads and writes them rather than
//! each other.
//!
//! **The stores live in sim state** (`Sim` owns a [`Shared`]) because replay
//! determinism is THE contract (GDD §1). A store beside the world rather than
//! in it is a store a replay does not carry, and the failure is silent: the
//! transcript matches and the relationships do not.
//!
//! **Nothing writes them but the functions below.** The three vectors are
//! private to this module, so the write rules of GDD §4.3 are enforced by
//! Rust rather than by a comment: the only way to add a bond is
//! [`Shared::record_shared_success`], the only way to add a grudge is
//! [`Shared::record_grudge`], and **no function removes a fact at all** —
//! "facts do not decay" is the absence of an eraser, not a rule somebody has
//! to remember. Most of the callers arrive in later waves; wave 0b builds the
//! doors and the checks that walk through them.
//!
//! **Regard is the master currency** (GDD §4.2): directed integer edges,
//! char->char and char->player, default 0, range bounded by the pair's facts.
//! A bond raises an edge's floor; a grudge lowers its ceiling — the scalar is
//! the mood, and the facts bound its range. [`Shared::drift`] is the slow pull
//! toward the baseline those bounds imply; it is integer, deterministic,
//! world-time addressed through the one scheduler, and it never writes a fact.

use crate::constants::Tuning;
use crate::traits::MarkId;

/// Who an edge or a fact points at (GDD §4.2: char->player and char->char).
///
/// The player is not a character — they have no wallet, no traits and no
/// home — but they are on the receiving end of regard, so they are a target
/// and not a row of the registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Regarded {
    /// The player: the person everyone asks things of.
    Player,
    /// Another character, by index into the registry.
    Person(usize),
}

impl Regarded {
    /// How a log line or a report names this target.
    pub fn label(self, names: &[&str]) -> String {
        match self {
            Regarded::Player => "the player".to_owned(),
            Regarded::Person(index) => names.get(index).map_or("?", |name| *name).to_owned(),
        }
    }
}

/// Which pair-fact a pair holds (GDD §4.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairFact {
    /// Repeated shared success plus high mutual regard.
    Bond,
    /// A betrayal-class act, acting against a petition, or egregious or
    /// repeated petition failure.
    Grudge,
}

/// Why a grudge was written — data on the row, never a branch.
///
/// The three sources GDD §4.3 names. A reader prints it; nothing decides
/// anything differently for one cause than for another, which is what keeps
/// adding a fourth source a data change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrudgeCause {
    /// A betrayal-class act.
    Betrayal,
    /// Acting against this character's petition.
    AgainstPetition,
    /// Egregious or repeated petition failure.
    PetitionFailed,
}

impl GrudgeCause {
    /// The cause as one ASCII phrase.
    pub fn name(self) -> &'static str {
        match self {
            GrudgeCause::Betrayal => "betrayal",
            GrudgeCause::AgainstPetition => "acted against their petition",
            GrudgeCause::PetitionFailed => "petition failed",
        }
    }
}

/// What a pair holds, both facts read at once — the shape [`bounds`] takes.
///
/// A pair may hold **both**: a friend who wronged you is a real state, and the
/// bounds it produces cross. [`bounds`] says what that means rather than
/// forbidding it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FactSet {
    /// Whether the pair holds a bond.
    pub bond: bool,
    /// Whether the pair holds a grudge.
    pub grudge: bool,
}

/// The range an edge may sit in, given its pair's facts (GDD §4.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bounds {
    /// The lowest the edge may go. Raised by a bond.
    pub floor: i64,
    /// The highest. Lowered by a grudge.
    pub ceiling: i64,
}

impl Bounds {
    /// The bounds as an ordered pair, which is what a clamp needs.
    ///
    /// `floor` and `ceiling` cross when a pair holds both facts; the interval
    /// is then the one between them, which is where [`Bounds::baseline`]
    /// lands.
    pub fn interval(self) -> (i64, i64) {
        (self.floor.min(self.ceiling), self.floor.max(self.ceiling))
    }

    /// The value drift pulls toward: **zero, held inside the bounds**.
    ///
    /// A mood with no facts behind it decays to indifference. A bond will not
    /// let it fall past the floor the bond sets, so the pull stops there; a
    /// grudge will not let it rise past its ceiling, likewise. A pair holding
    /// both is pulled to the middle of the interval its crossed bounds leave —
    /// the arithmetic answer to "they are both", and the reason the crossed
    /// case needs no special rule anywhere else.
    pub fn baseline(self) -> i64 {
        if self.floor <= self.ceiling {
            0.clamp(self.floor, self.ceiling)
        } else {
            (self.floor + self.ceiling) / 2
        }
    }

    /// `value`, held inside the interval.
    pub fn hold(self, value: i64) -> i64 {
        let (low, high) = self.interval();
        value.clamp(low, high)
    }
}

/// The bounds a fact-set implies at these constants (GDD §4.2).
///
/// With no facts the edge may run the whole span either way. A bond raises
/// the floor to `bond_floor`; a grudge lowers the ceiling to
/// `-grudge_ceiling`. Named drawer constants throughout, and integers.
pub fn bounds(tuning: &Tuning, facts: FactSet) -> Bounds {
    Bounds {
        floor: if facts.bond {
            tuning.bond_floor
        } else {
            -tuning.regard_span
        },
        ceiling: if facts.grudge {
            -tuning.grudge_ceiling
        } else {
            tuning.regard_span
        },
    }
}

/// One drift of one edge: `value` moved at most `step` toward its baseline,
/// never past it, and never outside the bounds.
///
/// The whole of the drift arithmetic, as a free function so a check can ask it
/// directly and the scheduled sweep and the assertion cannot be two rules.
/// Integer, monotone, and a fixed point at the baseline — drift converges and
/// then stops, so a world nobody touches settles rather than oscillating.
pub fn drift_toward(value: i64, bounds: Bounds, step: i64) -> i64 {
    let target = bounds.baseline();
    let step = step.max(0);
    let moved = match value.cmp(&target) {
        std::cmp::Ordering::Less => (value + step).min(target),
        std::cmp::Ordering::Greater => (value - step).max(target),
        std::cmp::Ordering::Equal => value,
    };
    bounds.hold(moved)
}

/// How many world-minutes pass between drifts.
///
/// `drift_hours` is the drawer's name for it because the drawer's range is
/// small and a cadence in minutes would not fit it. Floored at one hour: a
/// zero would ask the scheduler for an occurrence due at the moment it is
/// scheduled, forever.
pub fn drift_interval(tuning: &Tuning) -> u64 {
    let hours = tuning.drift_hours.max(1);
    u64::try_from(hours).unwrap_or(1) * 60
}

/// One directed edge. Sparse: an absent edge is zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edge {
    /// Who holds the opinion, by registry index.
    pub from: usize,
    /// Who it is about.
    pub to: Regarded,
    /// Positive is warmth, negative is ill will.
    pub value: i64,
}

/// One pair-fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FactRow {
    /// Who holds it.
    pub from: usize,
    /// About whom.
    pub to: Regarded,
    /// Which fact.
    pub fact: PairFact,
}

/// One person-fact: what everyone knows about somebody.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MarkRow {
    /// Who wears it, by registry index.
    pub who: usize,
    /// Which mark.
    pub mark: MarkId,
}

/// How many shared successes a pair has to their name — the counter the bond
/// threshold reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SuccessRow {
    from: usize,
    to: Regarded,
    count: i64,
}

/// The three stores, and the only functions that write them.
///
/// Every field is private. Reads are the methods below; writes are the four
/// named after the events that cause them, plus [`Shared::drift`]. There is no
/// eraser, no `clear`, and no `&mut` accessor — the GDD's write rules are
/// spelled here in what exists.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Shared {
    edges: Vec<Edge>,
    facts: Vec<FactRow>,
    successes: Vec<SuccessRow>,
    marks: Vec<MarkRow>,
    drifts: u64,
}

impl Shared {
    /// The stores at the scenario's opening: empty.
    ///
    /// Every edge is zero and nobody holds a fact, because every writer is an
    /// event and no event has happened. An authored starting relationship is a
    /// scenario-file feature (GDD §6) and arrives with the scenario format.
    pub fn opening() -> Self {
        Self::default()
    }

    // ── reads ────────────────────────────────────────────────────────────

    /// `regard(from -> to)`. Absent is zero. **Raw** — the traits weigh it in
    /// `traits::weighted_regard`, and the lens is where a screen gets either.
    pub fn regard(&self, from: usize, to: Regarded) -> i64 {
        self.edges
            .iter()
            .find(|edge| edge.from == from && edge.to == to)
            .map_or(0, |edge| edge.value)
    }

    /// Every edge that exists, in the order it was first written.
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// What this pair holds.
    pub fn facts(&self, from: usize, to: Regarded) -> FactSet {
        let held = |fact: PairFact| {
            self.facts
                .iter()
                .any(|row| row.from == from && row.to == to && row.fact == fact)
        };
        FactSet {
            bond: held(PairFact::Bond),
            grudge: held(PairFact::Grudge),
        }
    }

    /// Every pair-fact that exists, in write order.
    pub fn all_facts(&self) -> &[FactRow] {
        &self.facts
    }

    /// How many shared successes this pair has to their name.
    pub fn shared_successes(&self, from: usize, to: Regarded) -> i64 {
        self.successes
            .iter()
            .find(|row| row.from == from && row.to == to)
            .map_or(0, |row| row.count)
    }

    /// What everyone knows about `who`, in the order it was earned.
    pub fn marks(&self, who: usize) -> Vec<MarkId> {
        self.marks
            .iter()
            .filter(|row| row.who == who)
            .map(|row| row.mark)
            .collect()
    }

    /// Whether `who` wears this mark.
    pub fn marked(&self, who: usize, mark: MarkId) -> bool {
        self.marks
            .iter()
            .any(|row| row.who == who && row.mark == mark)
    }

    /// Every person-fact that exists, in write order.
    pub fn all_marks(&self) -> &[MarkRow] {
        &self.marks
    }

    /// How many times [`Shared::drift`] has run.
    ///
    /// Not decoration: drift is scheduled through the one scheduler like every
    /// other occurrence, and this is what a check counts to prove the cadence
    /// constant is doing something. Drift emits no event — it is ambient, and
    /// the feed is for things that happen *to* somebody.
    pub fn drifts(&self) -> u64 {
        self.drifts
    }

    // ── writes: one function per event class GDD §4.3 names ──────────────

    /// Move an edge by `delta`, held inside the bounds its facts imply.
    ///
    /// The general regard write: petition satisfied, voiced petition failed,
    /// the wage offer against expectation at dispatch, a witnessed act through
    /// the reaction table. Each of those is a caller in a later wave that
    /// decides *how much*; the bound is decided here, once, so no caller can
    /// push an edge past what the facts allow.
    ///
    /// Returns the value the edge ended at.
    pub fn adjust_regard(&mut self, tuning: &Tuning, from: usize, to: Regarded, delta: i64) -> i64 {
        let held = bounds(tuning, self.facts(from, to)).hold(self.regard(from, to) + delta);
        match self
            .edges
            .iter_mut()
            .find(|edge| edge.from == from && edge.to == to)
        {
            Some(edge) => edge.value = held,
            None => self.edges.push(Edge {
                from,
                to,
                value: held,
            }),
        }
        held
    }

    /// **The bond write rule** (GDD §4.3): repeated shared success plus high
    /// mutual regard.
    ///
    /// One call is one success shared by `a` and `b`. The counter goes up both
    /// ways; the bond is written — both ways, because a bond is a pair-fact
    /// and a one-sided bond is a category error — the first time the pair has
    /// `bond_after` successes **and** both edges are at or above
    /// `bond_regard`. A threshold event, not a decay: nothing about the count
    /// is undone by a later failure, and nothing here writes an edge.
    ///
    /// Returns whether this call is the one that wrote the bond.
    pub fn record_shared_success(&mut self, tuning: &Tuning, a: usize, b: usize) -> bool {
        if a == b {
            return false;
        }
        self.bump_success(a, Regarded::Person(b));
        self.bump_success(b, Regarded::Person(a));
        let enough = self.shared_successes(a, Regarded::Person(b)) >= tuning.bond_after
            && self.shared_successes(b, Regarded::Person(a)) >= tuning.bond_after;
        let mutual = self.regard(a, Regarded::Person(b)) >= tuning.bond_regard
            && self.regard(b, Regarded::Person(a)) >= tuning.bond_regard;
        if !(enough && mutual) {
            return false;
        }
        let already =
            self.facts(a, Regarded::Person(b)).bond && self.facts(b, Regarded::Person(a)).bond;
        self.write_fact(a, Regarded::Person(b), PairFact::Bond);
        self.write_fact(b, Regarded::Person(a), PairFact::Bond);
        !already
    }

    /// **The grudge write rule** (GDD §4.3): a betrayal-class act, acting
    /// against a character's petition, or egregious or repeated petition
    /// failure.
    ///
    /// Directed and one-sided on purpose — the wronged hold it, and whether
    /// the wrongdoer holds anything back is their own business. Writing the
    /// fact re-holds the edge inside its new ceiling immediately: a betrayal
    /// *is* a grudge at the moment it happens, and letting drift walk warmth
    /// down over the next several hours would be the sim telling a lie about
    /// what just occurred.
    ///
    /// Returns whether this call is the one that wrote the fact.
    pub fn record_grudge(
        &mut self,
        tuning: &Tuning,
        wronged: usize,
        against: Regarded,
        cause: GrudgeCause,
    ) -> bool {
        if against == Regarded::Person(wronged) {
            return false;
        }
        let _ = cause;
        let fresh = !self.facts(wronged, against).grudge;
        self.write_fact(wronged, against, PairFact::Grudge);
        // Re-hold, through the ordinary write, at the bounds the new fact
        // implies. A zero delta, so this only ever tightens.
        self.adjust_regard(tuning, wronged, against, 0);
        fresh
    }

    /// **The mark write rule**: a public fact, earned, plural, idempotent.
    ///
    /// Wearing a mark twice would double every reaction to it, so a repeat is
    /// a no-op rather than a second row. Marks do not decay and nothing here
    /// removes one.
    ///
    /// Returns whether this call is the one that wrote it.
    pub fn write_mark(&mut self, who: usize, mark: MarkId) -> bool {
        if self.marked(who, mark) {
            return false;
        }
        self.marks.push(MarkRow { who, mark });
        true
    }

    /// **The drift** (GDD §4.2): every edge pulled one `drift_step` toward the
    /// baseline its facts imply.
    ///
    /// Runs on the scheduler's clock, not on a tick, so it is speed-invariant
    /// like everything else. Writes no fact and creates no edge: drift is the
    /// mood settling, and a pair with nothing between them has no mood to
    /// settle. An edge that reaches its baseline stays there.
    pub fn drift(&mut self, tuning: &Tuning) {
        self.drifts += 1;
        // Read the fact-sets first: `facts` borrows the whole store, and the
        // walk below needs it mutable. One pass, allocated once - the edge
        // list is sparse by construction.
        let targets: Vec<Bounds> = self
            .edges
            .iter()
            .map(|edge| bounds(tuning, self.facts(edge.from, edge.to)))
            .collect();
        for (edge, bounds) in self.edges.iter_mut().zip(targets) {
            edge.value = drift_toward(edge.value, bounds, tuning.drift_step);
        }
    }

    fn bump_success(&mut self, from: usize, to: Regarded) {
        match self
            .successes
            .iter_mut()
            .find(|row| row.from == from && row.to == to)
        {
            Some(row) => row.count += 1,
            None => self.successes.push(SuccessRow { from, to, count: 1 }),
        }
    }

    fn write_fact(&mut self, from: usize, to: Regarded, fact: PairFact) {
        if self
            .facts
            .iter()
            .any(|row| row.from == from && row.to == to && row.fact == fact)
        {
            return;
        }
        self.facts.push(FactRow { from, to, fact });
    }
}

/// The stores' arithmetic and write rules, at a stated constants set —
/// **every expectation a shipped literal**, never derived from `tuning`.
///
/// The same discipline `verify::path_contracts_at` holds for the pathfinder:
/// a check that recomputes its expectation from the constant under test cannot
/// see that constant move, and the mutation round runs this battery at moved
/// constants for exactly that reason.
pub fn judge_at(checks: &mut crate::checks::Checks, tuning: &Tuning) {
    let none = FactSet::default();
    let bond = FactSet {
        bond: true,
        grudge: false,
    };
    let grudge = FactSet {
        bond: false,
        grudge: true,
    };
    let both = FactSet {
        bond: true,
        grudge: true,
    };

    // ── the bounds a fact-set implies (GDD §4.2) ─────────────────────────
    for (what, facts, floor, ceiling, baseline) in [
        ("no facts", none, -10, 10, 0),
        ("a bond", bond, 2, 10, 2),
        ("a grudge", grudge, -10, -2, -2),
        ("both", both, 2, -2, 0),
    ] {
        let got = bounds(tuning, facts);
        checks.require(
            got.floor == floor && got.ceiling == ceiling && got.baseline() == baseline,
            "a fact-set does not bound a regard edge the way the shipped set says",
            format!(
                "with {what} the bounds are {}..{} at baseline {}; the shipped set says \
                 {floor}..{ceiling} at {baseline} - a bond raises the floor, a grudge lowers \
                 the ceiling (GDD §4.2)",
                got.floor,
                got.ceiling,
                got.baseline()
            ),
        );
    }

    // ── the drift, at its bounds ─────────────────────────────────────────
    // **Asserted at the bounds, exhaustively**: over every fact-set and every
    // value from well outside the widest span, one drift never lands outside
    // the interval the facts allow, and never moves away from the baseline.
    for facts in [none, bond, grudge, both] {
        let limits = bounds(tuning, facts);
        let (low, high) = limits.interval();
        for value in -20..=20i64 {
            let moved = drift_toward(value, limits, tuning.drift_step);
            checks.require(
                (low..=high).contains(&moved),
                "one drift left a regard edge outside the bounds its facts allow",
                format!(
                    "{value} drifted to {moved} with bond={} grudge={}, and the interval is \
                     {low}..{high}",
                    facts.bond, facts.grudge
                ),
            );
            let target = limits.baseline();
            let before = (value - target).abs();
            let after = (moved - target).abs();
            checks.require(
                after <= before,
                "one drift moved a regard edge away from its baseline",
                format!(
                    "{value} drifted to {moved} against a baseline of {target} with bond={} \
                     grudge={}",
                    facts.bond, facts.grudge
                ),
            );
        }
    }
    // And the shipped step, walked: a mood with nothing behind it decays to
    // indifference one point at a time, and stops there.
    let walk = |start: i64, facts: FactSet, steps: usize| {
        let limits = bounds(tuning, facts);
        let mut value = start;
        let mut seen = vec![value];
        for _ in 0..steps {
            value = drift_toward(value, limits, tuning.drift_step);
            seen.push(value);
        }
        seen
    };
    for (what, start, facts, want) in [
        ("warmth with no facts", 4i64, none, vec![4, 3, 2, 1, 0, 0]),
        (
            "ill will with no facts",
            -4,
            none,
            vec![-4, -3, -2, -1, 0, 0],
        ),
        (
            "warmth over a bond's floor",
            6,
            bond,
            vec![6, 5, 4, 3, 2, 2],
        ),
        (
            "ill will under a grudge's ceiling",
            -6,
            grudge,
            vec![-6, -5, -4, -3, -2, -2],
        ),
        ("a pair holding both facts", 2, both, vec![2, 1, 0, 0, 0, 0]),
    ] {
        let got = walk(start, facts, want.len() - 1);
        checks.require(
            got == want,
            "regard does not drift toward its fact-set baseline at the shipped step",
            format!("{what} from {start} walks {got:?} and the shipped set says {want:?}"),
        );
    }
    // A value outside its bounds is held at once rather than walked in.
    // **Unreachable through the write API** - every write holds the edge, and
    // both fact writes re-hold it at the bounds the new fact implies - so this
    // is the definition of a state the stores do not produce, asserted so that
    // it stays a definition rather than becoming a surprise.
    checks.require(
        drift_toward(0, bounds(tuning, bond), tuning.drift_step) == 2
            && drift_toward(0, bounds(tuning, grudge), tuning.drift_step) == -2,
        "an out-of-bounds edge is not held at its bound by a drift",
        format!(
            "0 under a bond drifts to {} and under a grudge to {}; the shipped bounds are \
             2 and -2",
            drift_toward(0, bounds(tuning, bond), tuning.drift_step),
            drift_toward(0, bounds(tuning, grudge), tuning.drift_step)
        ),
    );
    checks.require(
        drift_interval(tuning) == 240,
        "the drift cadence is not the shipped one",
        format!(
            "drift_interval is {} world-minutes and the shipped four hours is 240",
            drift_interval(tuning)
        ),
    );

    // ── the write rules (GDD §4.3) ───────────────────────────────────────
    let (a, b) = (0usize, 1usize);
    let them = Regarded::Person(b);
    let him = Regarded::Person(a);

    // An edge is held inside its bounds by the write, not by the reader.
    let mut store = Shared::opening();
    checks.require(
        store.regard(a, them) == 0 && store.edges().is_empty(),
        "the stores do not open empty",
        format!("{} edges at the opening", store.edges().len()),
    );
    let held = store.adjust_regard(tuning, a, them, 99);
    checks.require(
        held == 10 && store.regard(a, them) == 10,
        "a regard write was not held inside the factless span",
        format!("+99 from zero landed at {held}; the shipped span is 10"),
    );

    // The bond rule: repeated shared success **plus** high mutual regard.
    let mut store = Shared::opening();
    store.adjust_regard(tuning, a, them, 3);
    store.adjust_regard(tuning, b, him, 3);
    let first = store.record_shared_success(tuning, a, b);
    checks.require(
        !first && !store.facts(a, them).bond,
        "one shared success wrote a bond",
        format!(
            "the pair has {} successes and bond={}; the shipped rule needs 2",
            store.shared_successes(a, them),
            store.facts(a, them).bond
        ),
    );
    let second = store.record_shared_success(tuning, a, b);
    checks.require(
        second && store.facts(a, them).bond && store.facts(b, him).bond,
        "the shared-success threshold did not write the bond, both ways",
        format!(
            "after 2 successes at mutual regard 3 the facts are a->b {:?} and b->a {:?}",
            store.facts(a, them),
            store.facts(b, him)
        ),
    );
    checks.require(
        !store.record_shared_success(tuning, a, b),
        "a bond was written twice",
        "a third shared success reported writing the bond again".to_owned(),
    );
    // And the bond now holds the edge's floor up.
    let floored = store.adjust_regard(tuning, a, them, -99);
    checks.require(
        floored == 2,
        "a bond does not hold its edge's floor",
        format!("-99 through a bond landed at {floored}; the shipped floor is 2"),
    );

    // Repeated success alone is not a bond: the regard half is load-bearing.
    let mut cold = Shared::opening();
    cold.record_shared_success(tuning, a, b);
    cold.record_shared_success(tuning, a, b);
    cold.record_shared_success(tuning, a, b);
    checks.require(
        !cold.facts(a, them).bond,
        "shared success alone wrote a bond",
        format!(
            "three successes at regard {} wrote bond={}; the rule is repeated success \
             *plus* high mutual regard (GDD §4.3)",
            cold.regard(a, them),
            cold.facts(a, them).bond
        ),
    );

    // The grudge rule, and its immediate re-hold.
    let mut wronged = Shared::opening();
    wronged.adjust_regard(tuning, a, them, 8);
    let fresh = wronged.record_grudge(tuning, a, them, GrudgeCause::Betrayal);
    checks.require(
        fresh && wronged.facts(a, them).grudge && wronged.regard(a, them) == -2,
        "a grudge did not take effect the moment it was written",
        format!(
            "after a betrayal the edge reads {} with grudge={}; a grudge lowers the ceiling \
             to -2 at once, and letting drift walk warmth down over hours would be the sim \
             lying about what just happened",
            wronged.regard(a, them),
            wronged.facts(a, them).grudge
        ),
    );
    checks.require(
        !wronged.record_grudge(tuning, a, them, GrudgeCause::PetitionFailed),
        "a grudge was written twice",
        "a second betrayal-class act reported writing the same fact again".to_owned(),
    );
    // Facts do not decay: drift moves the mood and leaves the fact standing.
    let before = wronged.all_facts().to_vec();
    wronged.drift(tuning);
    checks.require(
        wronged.all_facts() == before.as_slice() && wronged.drifts() == 1,
        "drift touched a fact, or did not count itself",
        format!(
            "after one drift there are {} facts (was {}) and the counter reads {}",
            wronged.all_facts().len(),
            before.len(),
            wronged.drifts()
        ),
    );

    // Nobody bonds with or resents themselves.
    let mut alone = Shared::opening();
    checks.require(
        !alone.record_shared_success(tuning, a, a)
            && !alone.record_grudge(tuning, a, him, GrudgeCause::Betrayal)
            && alone.all_facts().is_empty(),
        "a character holds a pair-fact about themselves",
        format!(
            "{} facts after two self-directed writes",
            alone.all_facts().len()
        ),
    );

    // Marks: earned, plural, idempotent, and never removed.
    let mut public = Shared::opening();
    checks.require(
        public.write_mark(a, crate::traits::MarkId::Skimmer)
            && !public.write_mark(a, crate::traits::MarkId::Skimmer)
            && public.write_mark(a, crate::traits::MarkId::Reliable),
        "the mark store is not an idempotent set of earned facts",
        format!(
            "writing skimmer twice and reliable once left {:?}",
            public.marks(a)
        ),
    );
    checks.require(
        public.all_marks().len() == 2,
        "the mark store does not hold exactly the facts that were written",
        format!(
            "{} rows after two distinct marks and one repeat",
            public.all_marks().len()
        ),
    );
    checks.require(
        public.marks(a)
            == vec![
                crate::traits::MarkId::Skimmer,
                crate::traits::MarkId::Reliable,
            ]
            && public.marks(b).is_empty()
            && public.marked(a, crate::traits::MarkId::Skimmer)
            && !public.marked(b, crate::traits::MarkId::Skimmer),
        "a mark is not a person-fact about exactly one person",
        format!(
            "a wears {:?} and b wears {:?}",
            public.marks(a),
            public.marks(b)
        ),
    );

    // Every grudge cause prints, and every one writes the same fact: the
    // cause is data on the row, and nothing decides differently for one.
    let names = ["Alex", "Bob"];
    checks.require(
        Regarded::Player.label(&names) == "the player"
            && Regarded::Person(1).label(&names) == "Bob"
            && Regarded::Person(9).label(&names) == "?",
        "a regard target does not name itself the way a report needs",
        format!(
            "the player reads {:?}, person 1 reads {:?} and person 9 reads {:?}",
            Regarded::Player.label(&names),
            Regarded::Person(1).label(&names),
            Regarded::Person(9).label(&names)
        ),
    );
    for cause in [
        GrudgeCause::Betrayal,
        GrudgeCause::AgainstPetition,
        GrudgeCause::PetitionFailed,
    ] {
        checks.require(
            !cause.name().is_empty()
                && cause
                    .name()
                    .chars()
                    .all(|glyph| (' '..='~').contains(&glyph)),
            "a grudge cause does not print as one ASCII phrase",
            format!("{cause:?} reads {:?}", cause.name()),
        );
        let mut once = Shared::opening();
        checks.require(
            once.record_grudge(tuning, a, them, cause) && once.facts(a, them).grudge,
            "a grudge cause GDD §4.3 names does not write a grudge",
            format!("{cause:?} left the facts {:?}", once.facts(a, them)),
        );
    }

    // The player is a target like anyone else.
    let mut toward_player = Shared::opening();
    toward_player.adjust_regard(tuning, a, Regarded::Player, -4);
    checks.require(
        toward_player.regard(a, Regarded::Player) == -4 && toward_player.regard(a, them) == 0,
        "an edge toward the player is not its own edge",
        format!(
            "a->player reads {} and a->b reads {}",
            toward_player.regard(a, Regarded::Player),
            toward_player.regard(a, them)
        ),
    );
}
