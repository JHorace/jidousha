//! Every tuning constant ninjo has, in one place (DESIGN.md §3, §4).
//!
//! The substrate's whole vocabulary of numbers lives here and nowhere else: a
//! system that wants a terrain's movement cost reads it off the `Tuning`
//! resource rather than writing a number of its own. Two things depend on that
//! being true.
//!
//! **The fixed scripts are this file's test suite.** A constant change that
//! moves an arrival time fails `--verify`, and the mutation round in
//! `mutation.rs` is that claim run on purpose: it perturbs each constant below
//! and demands an arrival-time or pacing assertion notice.
//!
//! **And a resource rather than a `const` block**, because a sweep and the
//! live tuning drawer both need to vary these without rebuilding:
//! `headless(..)` builds a fresh game per candidate and `Startup` takes
//! whatever the harness left in the world.
//!
//! **Three readers, one set of names.** `Field::name` is the name DESIGN gives
//! a constant, and it is the only name there is: the drawer's rows print it,
//! `stamp` writes it lowercased into the compact form a recording and a
//! `?constants=` link carry, and `parse` reads that form back by matching it
//! case-insensitively.

use jidousha::prelude::*;

/// The substrate's numbers — one struct, one shipped set.
///
/// Integers, not floats: the world clock is integer world-minutes (DESIGN §4)
/// and every arrival time is a sum of these, so a claim like "the Watchtower
/// is 74 minutes by road" is a claim about integers. It is also what makes the
/// assertions in `--verify` exact rather than approximate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tuning {
    /// World-minutes to enter a road tile.
    pub road_cost: i64,
    /// World-minutes to enter a plains tile.
    pub plains_cost: i64,
    /// World-minutes to enter a forest tile.
    pub forest_cost: i64,
    /// World-minutes to enter a rough tile.
    pub rough_cost: i64,
    /// Engine ticks per world-minute at 1x — the clock's base pace.
    ///
    /// The clock accumulates the current speed's multiplier every tick and
    /// carries one world-minute for every `minute_ticks` accumulated. At the
    /// shipped 30 ticks and an accumulation of 12, **1x is 24 world-minutes a
    /// real second** at the engine's fixed sixty — a world-day every minute of
    /// wall time. DESIGN §4's first guess was an order of magnitude slower and
    /// the wave-0a playtest said so; 2x and 4x are exact multiples of this.
    pub minute_ticks: i64,
    /// The 1x speed's per-tick accumulation.
    pub speed_1x: i64,
    /// The 2x speed's per-tick accumulation.
    pub speed_2x: i64,
    /// The 4x speed's per-tick accumulation.
    pub speed_4x: i64,

    // ── the people substrate (GDD §4; wave 0b) ────────────────────────────
    /// What one dark mark costs a stranger's reading of somebody, before
    /// their traits weigh in.
    pub mark_dark: i64,
    /// And what one light mark earns it.
    pub mark_light: i64,
    /// How far a regard edge may run either way when the pair holds no facts.
    pub regard_span: i64,
    /// The floor a bond holds an edge at or above (GDD §4.2: a bond raises an
    /// edge's floor).
    pub bond_floor: i64,
    /// How far below zero a grudge holds an edge's ceiling (a grudge lowers
    /// the ceiling; the value is the depth, because the drawer's range starts
    /// at zero).
    pub grudge_ceiling: i64,
    /// Shared successes a pair needs before a bond can be written.
    pub bond_after: i64,
    /// And the mutual regard both edges need at that moment. Both terms, or
    /// no bond: the rule is repeated success *plus* high regard (GDD §4.3).
    pub bond_regard: i64,
    /// How far one drift moves an edge toward its fact-set baseline.
    pub drift_step: i64,
    /// World-hours between drifts. Slow is the design; the drawer's range is
    /// small, so the cadence is stated in hours and floored at one.
    pub drift_hours: i64,

    // ── the attention architecture (GDD §3; wave 0a) ──────────────────────
    /// How many entries the feed holds (`attention::feed`).
    ///
    /// The feed is a view of the event log, so this is a bound on the *view*
    /// and never on the log: the transcript keeps everything, and the player
    /// sees the newest this many.
    pub feed_cap: i64,
    /// How long a click-to-focus pulse marker lasts, in **tenths of a
    /// wall-second** — presentation, so it is measured in wall time, and in
    /// tenths because the drawer's range is small.
    pub pulse_tenths: i64,

    // ── autonomy: the scorer (GDD §5; wave 1.1) ───────────────────────────
    /// World-hours between one character's rescorings. Stated in hours
    /// because the drawer's range is small, like `drift_hours`.
    pub scorer_hours: i64,
    /// World-minutes of stagger per roster index, so ten people do not all
    /// decide in the same tick. Deterministic, and the only reason the
    /// cadence is not a single moment.
    pub scorer_stagger: i64,
    /// World-hours after a job during which work weighs less — the rest term,
    /// so nobody works forever.
    pub rest_hours: i64,
    /// What one point of desperation adds to a paid candidate. The term that
    /// opens every sum, as it did in giri.
    pub need_weight: i64,
    /// What one point of a want's pressure adds, where its `favors` field
    /// covers the candidate's task type.
    pub want_weight: i64,
    /// What one point of aptitude at the candidate's task type adds.
    pub apt_weight: i64,
    /// What the pot pulls, per ten gold, per point of pot affinity.
    pub pot_weight: i64,
    /// What one point of felt regard toward whoever is at the destination
    /// adds to a visit.
    pub regard_weight: i64,
    /// What a candidate loses while its carrier is still resting.
    pub rest_weight: i64,
    /// What idling scores. A candidate has to beat this to happen at all, so
    /// this is the floor the whole scorer is measured against.
    pub idle_floor: i64,
    /// How long a visit lasts, in world-minutes.
    pub visit_minutes: i64,
    /// What a completed visit adds to the visitor's regard for their host.
    pub visit_regard: i64,
    /// Whether that regard is symmetric: 1 gives the host the same warmth
    /// back, 0 leaves a visit one-sided.
    pub visit_mutual: i64,
    /// Which relationship preset a scenario opens on: 0 is flat (every edge
    /// zero, no facts), 1 is the authored seeds of `CAST.md` §5.
    pub bonds_preset: i64,
    /// How many world-days the alive sweep gives every character to take at
    /// least one job (GDD §9's economy sweep, opening half).
    pub alive_days: i64,
}

impl Resource for Tuning {}

impl Tuning {
    /// The smallest value the drawer's steppers and a `?constants=` link offer.
    ///
    /// A bound on the *tuning surface*, not on the type: the mutation round
    /// deliberately moves a constant to 99, because a perturbation has to be
    /// one nothing plausibly authors. What a person can reach by clicking, and
    /// what a shared link may carry, is this range.
    pub const MIN: i64 = 0;
    /// And the largest — wide enough for `speed_4x` at its shipped 48 and
    /// `visit_minutes` at 45.
    pub const MAX: i64 = 60;

    /// What the game ships with — the set the scenario's fixed scripts are
    /// authored against, and the set every verify run stamps into its report.
    pub const SHIPPED: Self = Self {
        road_cost: 2,
        plains_cost: 4,
        forest_cost: 7,
        rough_cost: 10,
        minute_ticks: 30,
        speed_1x: 12,
        speed_2x: 24,
        speed_4x: 48,
        mark_dark: 1,
        mark_light: 1,
        regard_span: 10,
        bond_floor: 2,
        grudge_ceiling: 2,
        bond_after: 2,
        bond_regard: 3,
        drift_step: 1,
        drift_hours: 4,
        feed_cap: 10,
        pulse_tenths: 25,
        scorer_hours: 4,
        scorer_stagger: 24,
        rest_hours: 6,
        need_weight: 2,
        want_weight: 2,
        apt_weight: 3,
        pot_weight: 1,
        regard_weight: 2,
        rest_weight: 12,
        idle_floor: 3,
        visit_minutes: 45,
        visit_regard: 1,
        visit_mutual: 1,
        bonds_preset: 1,
        alive_days: 3,
    };

    /// The constants in effect, as the lines the drawer's stamp and every
    /// verify report print (a run is only reproducible if it says what it ran
    /// with).
    pub fn readout(&self) -> String {
        // Two constants to a line and never wider than the drawer's stamp
        // column, which is what is left of the screen beside three columns of
        // steppers. `floors.rs` fails a line that outgrows it.
        format!(
            "road {} plains {}\n\
             forest {} rough {}\n\
             minute {} ticks\n\
             speeds {}/{}/{}\n\
             marks -{}/+{}\n\
             span {} floor {}\n\
             ceil -{} after {}@{}\n\
             drift {}/{}h\n\
             feed {} pulse {}\n\
             score {}h stag {}m\n\
             rest {}h w{}\n\
             need{} want{} apt{}\n\
             pot{} regard{} idle{}\n\
             visit {}m +{} both{}\n\
             bonds {} alive {}d",
            self.road_cost,
            self.plains_cost,
            self.forest_cost,
            self.rough_cost,
            self.minute_ticks,
            self.speed_1x,
            self.speed_2x,
            self.speed_4x,
            self.mark_dark,
            self.mark_light,
            self.regard_span,
            self.bond_floor,
            self.grudge_ceiling,
            self.bond_after,
            self.bond_regard,
            self.drift_step,
            self.drift_hours,
            self.feed_cap,
            self.pulse_tenths,
            self.scorer_hours,
            self.scorer_stagger,
            self.rest_hours,
            self.rest_weight,
            self.need_weight,
            self.want_weight,
            self.apt_weight,
            self.pot_weight,
            self.regard_weight,
            self.idle_floor,
            self.visit_minutes,
            self.visit_regard,
            self.visit_mutual,
            self.bonds_preset,
            self.alive_days,
        )
    }

    /// One field, by the name DESIGN gives it — so a sweep, a mutation round
    /// and the tuning drawer can walk the set rather than naming eight fields
    /// in three places.
    pub fn field_mut(&mut self, field: Field) -> &mut i64 {
        match field {
            Field::RoadCost => &mut self.road_cost,
            Field::PlainsCost => &mut self.plains_cost,
            Field::ForestCost => &mut self.forest_cost,
            Field::RoughCost => &mut self.rough_cost,
            Field::MinuteTicks => &mut self.minute_ticks,
            Field::Speed1x => &mut self.speed_1x,
            Field::Speed2x => &mut self.speed_2x,
            Field::Speed4x => &mut self.speed_4x,
            Field::MarkDark => &mut self.mark_dark,
            Field::MarkLight => &mut self.mark_light,
            Field::RegardSpan => &mut self.regard_span,
            Field::BondFloor => &mut self.bond_floor,
            Field::GrudgeCeiling => &mut self.grudge_ceiling,
            Field::BondAfter => &mut self.bond_after,
            Field::BondRegard => &mut self.bond_regard,
            Field::DriftStep => &mut self.drift_step,
            Field::DriftHours => &mut self.drift_hours,
            Field::FeedCap => &mut self.feed_cap,
            Field::PulseTenths => &mut self.pulse_tenths,
            Field::ScorerHours => &mut self.scorer_hours,
            Field::ScorerStagger => &mut self.scorer_stagger,
            Field::RestHours => &mut self.rest_hours,
            Field::NeedWeight => &mut self.need_weight,
            Field::WantWeight => &mut self.want_weight,
            Field::AptWeight => &mut self.apt_weight,
            Field::PotWeight => &mut self.pot_weight,
            Field::RegardWeight => &mut self.regard_weight,
            Field::RestWeight => &mut self.rest_weight,
            Field::IdleFloor => &mut self.idle_floor,
            Field::VisitMinutes => &mut self.visit_minutes,
            Field::VisitRegard => &mut self.visit_regard,
            Field::VisitMutual => &mut self.visit_mutual,
            Field::BondsPreset => &mut self.bonds_preset,
            Field::AliveDays => &mut self.alive_days,
        }
    }

    /// One field, by name. `Tuning` is `Copy`, so this is the reader that
    /// `field_mut` would otherwise need a second match to provide.
    pub fn field(mut self, field: Field) -> i64 {
        *self.field_mut(field)
    }

    /// This set with one field replaced — what a mutation round varies.
    pub fn with(mut self, field: Field, value: i64) -> Self {
        *self.field_mut(field) = value;
        self
    }

    /// The whole set on one line, in the compact form a link carries and a log
    /// line records: `road_cost:2,plains_cost:4,...`.
    pub fn stamp(&self) -> String {
        Field::ALL
            .iter()
            .map(|field| format!("{}:{}", field.key(), self.field(*field)))
            .collect::<Vec<_>>()
            .join(",")
    }

    /// The set a `?constants=` parameter asks for, or why it was refused.
    ///
    /// Every key is optional and names a constant this set overrides; anything
    /// unnamed keeps the value `self` has, which is what makes a short link —
    /// `?constants=road_cost:1` — mean "the shipped set with one thing moved".
    /// A key that is not a constant, a value that is not a number, a value
    /// outside the drawer's range, and a key given twice are all refusals —
    /// rejected loudly, never silently clamped.
    pub fn parse(self, text: &str) -> Result<Self, ConstantsError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(ConstantsError::Empty);
        }
        let mut out = self;
        let mut seen: Vec<Field> = Vec::new();
        for term in text.split(',') {
            let term = term.trim();
            let Some((key, value)) = term.split_once(':') else {
                return Err(ConstantsError::Pair(term.to_owned()));
            };
            let (key, value) = (key.trim(), value.trim());
            let Some(field) = Field::find(key) else {
                return Err(ConstantsError::UnknownKey(key.to_owned()));
            };
            if seen.contains(&field) {
                return Err(ConstantsError::Repeated(field.key()));
            }
            seen.push(field);
            let Ok(number) = value.parse::<i64>() else {
                return Err(ConstantsError::NotANumber {
                    key: field.key(),
                    value: value.to_owned(),
                });
            };
            if !(Self::MIN..=Self::MAX).contains(&number) {
                return Err(ConstantsError::OutOfRange {
                    key: field.key(),
                    value: number,
                });
            }
            *out.field_mut(field) = number;
        }
        Ok(out)
    }
}

/// Why a `?constants=` parameter was refused.
///
/// One variant per way a link can be wrong, because the message a player reads
/// on the page has to name *which* thing was wrong: "rejected" with no key in
/// it is a link nobody can fix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstantsError {
    /// `?constants=` with nothing after it.
    Empty,
    /// A term with no `:` in it.
    Pair(String),
    /// A key that is not one of `Field::ALL`.
    UnknownKey(String),
    /// A key given twice, which would make the link's meaning depend on order.
    Repeated(String),
    /// A value that is not an integer.
    NotANumber {
        /// The constant it was given for.
        key: String,
        /// What was written instead of a number.
        value: String,
    },
    /// A value outside the range the drawer offers.
    OutOfRange {
        /// The constant it was given for.
        key: String,
        /// The number asked for.
        value: i64,
    },
}

impl ConstantsError {
    /// What the page says: what happened, and what to write instead.
    ///
    /// ASCII and one line, because it is drawn with the same font every other
    /// string is and the drawer gives it two wrapped rows.
    pub fn message(&self) -> String {
        match self {
            ConstantsError::Empty => {
                "?constants= was empty - write it as road_cost:2,plains_cost:4 or leave it off"
                    .to_owned()
            }
            ConstantsError::Pair(term) => format!(
                "?constants= term {term:?} has no ':' - each term is a name, a colon, a number"
            ),
            ConstantsError::UnknownKey(key) => format!(
                "?constants= names {key:?}, which is not a constant - the drawer lists every name"
            ),
            ConstantsError::Repeated(key) => {
                format!("?constants= gives {key:?} twice - name each constant once")
            }
            ConstantsError::NotANumber { key, value } => {
                format!("?constants= gives {key} the value {value:?}, which is not a whole number")
            }
            ConstantsError::OutOfRange { key, value } => format!(
                "?constants= gives {key} the value {value}, outside the range {} to {}",
                Tuning::MIN,
                Tuning::MAX
            ),
        }
    }
}

/// Which tuning constant, by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// A road tile's entry cost.
    RoadCost,
    /// A plains tile's entry cost.
    PlainsCost,
    /// A forest tile's entry cost.
    ForestCost,
    /// A rough tile's entry cost.
    RoughCost,
    /// Ticks per world-minute at 1x.
    MinuteTicks,
    /// The 1x accumulation.
    Speed1x,
    /// The 2x accumulation.
    Speed2x,
    /// The 4x accumulation.
    Speed4x,
    /// What a dark mark costs.
    MarkDark,
    /// What a light mark earns.
    MarkLight,
    /// The factless bound on a regard edge.
    RegardSpan,
    /// The floor a bond sets.
    BondFloor,
    /// The depth of the ceiling a grudge sets.
    GrudgeCeiling,
    /// Shared successes before a bond.
    BondAfter,
    /// Mutual regard needed with them.
    BondRegard,
    /// How far one drift moves an edge.
    DriftStep,
    /// World-hours between drifts.
    DriftHours,
    /// How many entries the feed holds.
    FeedCap,
    /// How long a focus pulse lasts, in tenths of a second.
    PulseTenths,
    /// World-hours between rescorings.
    ScorerHours,
    /// Stagger per roster index, in world-minutes.
    ScorerStagger,
    /// World-hours of rest after a job.
    RestHours,
    /// What one point of desperation adds.
    NeedWeight,
    /// What one point of a want's pressure adds.
    WantWeight,
    /// What one point of aptitude adds.
    AptWeight,
    /// What the pot pulls, per ten gold per affinity.
    PotWeight,
    /// What one point of felt regard adds to a visit.
    RegardWeight,
    /// What resting costs a work candidate.
    RestWeight,
    /// What idling scores.
    IdleFloor,
    /// How long a visit lasts.
    VisitMinutes,
    /// What a visit earns.
    VisitRegard,
    /// Whether a visit's regard is symmetric.
    VisitMutual,
    /// Which relationship preset a scenario opens on.
    BondsPreset,
    /// How many world-days the alive sweep allows.
    AliveDays,
}

impl Field {
    /// Every field, in declaration order — what a sweep walks.
    pub const ALL: &'static [Field] = &[
        Field::RoadCost,
        Field::PlainsCost,
        Field::ForestCost,
        Field::RoughCost,
        Field::MinuteTicks,
        Field::Speed1x,
        Field::Speed2x,
        Field::Speed4x,
        Field::MarkDark,
        Field::MarkLight,
        Field::RegardSpan,
        Field::BondFloor,
        Field::GrudgeCeiling,
        Field::BondAfter,
        Field::BondRegard,
        Field::DriftStep,
        Field::DriftHours,
        Field::FeedCap,
        Field::PulseTenths,
        Field::ScorerHours,
        Field::ScorerStagger,
        Field::RestHours,
        Field::NeedWeight,
        Field::WantWeight,
        Field::AptWeight,
        Field::PotWeight,
        Field::RegardWeight,
        Field::RestWeight,
        Field::IdleFloor,
        Field::VisitMinutes,
        Field::VisitRegard,
        Field::VisitMutual,
        Field::BondsPreset,
        Field::AliveDays,
    ];

    /// The name DESIGN gives this constant.
    pub fn name(self) -> &'static str {
        match self {
            Field::RoadCost => "road_cost",
            Field::PlainsCost => "plains_cost",
            Field::ForestCost => "forest_cost",
            Field::RoughCost => "rough_cost",
            Field::MinuteTicks => "minute_ticks",
            Field::Speed1x => "speed_1x",
            Field::Speed2x => "speed_2x",
            Field::Speed4x => "speed_4x",
            Field::MarkDark => "mark_dark",
            Field::MarkLight => "mark_light",
            Field::RegardSpan => "regard_span",
            Field::BondFloor => "bond_floor",
            Field::GrudgeCeiling => "grudge_ceiling",
            Field::BondAfter => "bond_after",
            Field::BondRegard => "bond_regard",
            Field::DriftStep => "drift_step",
            Field::DriftHours => "drift_hours",
            Field::FeedCap => "feed_cap",
            Field::PulseTenths => "pulse_tenths",
            Field::ScorerHours => "scorer_hours",
            Field::ScorerStagger => "scorer_stagger",
            Field::RestHours => "rest_hours",
            Field::NeedWeight => "need_weight",
            Field::WantWeight => "want_weight",
            Field::AptWeight => "apt_weight",
            Field::PotWeight => "pot_weight",
            Field::RegardWeight => "regard_weight",
            Field::RestWeight => "rest_weight",
            Field::IdleFloor => "idle_floor",
            Field::VisitMinutes => "visit_minutes",
            Field::VisitRegard => "visit_regard",
            Field::VisitMutual => "visit_mutual",
            Field::BondsPreset => "bonds_preset",
            Field::AliveDays => "alive_days",
        }
    }

    /// The same name, lowercased — what a stamp writes and a link carries.
    ///
    /// Derived rather than tabled, so a second table cannot become a second
    /// name (the names here are already lowercase; the derivation keeps the
    /// rule giri established).
    pub fn key(self) -> String {
        self.name().to_ascii_lowercase()
    }

    /// The field a `?constants=` key names, if it names one.
    pub fn find(key: &str) -> Option<Field> {
        Field::ALL
            .iter()
            .copied()
            .find(|field| field.name().eq_ignore_ascii_case(key))
    }

    /// What this constant does, in one line — the drawer's hover text.
    ///
    /// Written for a person who is about to move it: what goes up when the
    /// number goes up. Kept under fifty characters, which is the width the
    /// drawer's hint row has.
    pub fn meaning(self) -> &'static str {
        match self {
            Field::RoadCost => "world-minutes to enter a road tile",
            Field::PlainsCost => "world-minutes to enter a plains tile",
            Field::ForestCost => "world-minutes to enter a forest tile",
            Field::RoughCost => "world-minutes to enter a rough tile",
            Field::MinuteTicks => "engine ticks per world-minute at 1x",
            Field::Speed1x => "clock accumulation per tick at 1x",
            Field::Speed2x => "clock accumulation per tick at 2x",
            Field::Speed4x => "clock accumulation per tick at 4x",
            Field::MarkDark => "what a dark mark costs a stranger",
            Field::MarkLight => "what a light mark earns one",
            Field::RegardSpan => "how far regard runs with no facts",
            Field::BondFloor => "the floor a bond holds an edge at",
            Field::GrudgeCeiling => "how far a grudge caps an edge below 0",
            Field::BondAfter => "shared successes before a bond",
            Field::BondRegard => "mutual regard a bond also needs",
            Field::DriftStep => "how far one drift moves an edge",
            Field::DriftHours => "world-hours between regard drifts",
            Field::FeedCap => "how many entries the feed holds",
            Field::PulseTenths => "focus pulse, in tenths of a second",
            Field::ScorerHours => "world-hours between rescorings",
            Field::ScorerStagger => "minutes of stagger per roster index",
            Field::RestHours => "world-hours of rest after a job",
            Field::NeedWeight => "what one point of desperation adds",
            Field::WantWeight => "what one point of a want adds",
            Field::AptWeight => "what one point of aptitude adds",
            Field::PotWeight => "pot pull per ten gold per affinity",
            Field::RegardWeight => "what felt regard adds to a visit",
            Field::RestWeight => "what resting costs a work candidate",
            Field::IdleFloor => "what idling scores - the bar to beat",
            Field::VisitMinutes => "how long a visit lasts",
            Field::VisitRegard => "what a visit earns the visitor",
            Field::VisitMutual => "1 if a visit's warmth is symmetric",
            Field::BondsPreset => "0 flat relationships, 1 authored",
            Field::AliveDays => "days the alive sweep allows a job",
        }
    }
}
