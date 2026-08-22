//! Every tuning constant giri has, in one place (DESIGN.md §3.2).
//!
//! The social model's whole vocabulary of weights lives here and nowhere else:
//! a system that wants `K_kill` reads it off the `Tuning` resource rather than
//! writing a number of its own. Two things depend on that being true.
//!
//! **The beats are this file's test suite.** A constant change that breaks a
//! beat's intended dilemma fails `--verify`, and the mutation round in
//! `verify.rs` is that claim run on purpose: it perturbs each constant below
//! and demands a beat notice.
//!
//! **And a resource rather than a `const` block**, because a sweep and the
//! live tuning menu (DESIGN §8a, next session) both need to vary these without
//! rebuilding: `headless(..)` builds a fresh game per candidate and `Startup`
//! takes whatever the harness left in the world. `docs/api/jidousha-testing.md`
//! makes the trade explicit — a game with two numbers should stay with
//! constants; this one has ten and its verify mode sweeps them every run.

use jidousha::prelude::*;

/// The social model's weights — one struct, one shipped set.
///
/// Names follow DESIGN §3.2 exactly: `K_inf`, `K_kill`, `K_loyal`, and the
/// drift magnitudes. Integers, not floats: every beat is meant to be exactly
/// computable by a player with the sheets in front of them, and a claim like
/// "desperation 8 reaches 6" is a claim about integers. It is also what makes
/// the assertions in `--verify` exact rather than approximate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tuning {
    /// `K_inf` — how much one point of infamy *gap* costs a character's
    /// willingness. `incompat(c, m) = K_inf * max(0, infamy(m) - infamy(c))`.
    pub k_inf: i32,
    /// `K_kill` — the desperation at or above which betrayal is on the table.
    pub k_kill: i32,
    /// `K_loyal` — the regard at or above which a character will not betray.
    pub k_loyal: i32,
    /// What a clean shared success adds to regard, both ways, per surviving pair.
    pub bond_gain: i32,
    /// What a witnessed kill adds to the killer's *public* infamy.
    pub infamy_per_kill: i32,
    /// What a surviving witness's regard toward the killer drops by.
    pub witness_grudge: i32,
    /// The extra drop when that witness had positive regard for the victim.
    ///
    /// Bonds propagate consequences (DESIGN §3.3.3): harm to someone you are
    /// bonded to is a grudge against whoever did it, on top of the witness's.
    pub bonded_grudge: i32,
    /// What a round of not profiting adds to desperation.
    pub desperation_rise: i32,
    /// What profiting takes off it.
    pub desperation_fall: i32,
    /// The floor desperation clamps at.
    ///
    /// Not in DESIGN's formulas, and needed the moment `desperation_fall`
    /// exceeds a character's desperation: without it a profitable round leaves
    /// a character at -2, and `willingness = -2 + 0 - 0` refuses a job with no
    /// infamy gap and nothing else wrong with it. At the floor a character
    /// still takes a clean job (0 >= 0) and nothing that costs them.
    pub desperation_floor: i32,
}

impl Resource for Tuning {}

impl Tuning {
    /// What the game ships with — the set the four tutorial beats are authored
    /// against, and the set every verify run stamps into its report.
    pub const SHIPPED: Self = Self {
        k_inf: 1,
        k_kill: 6,
        k_loyal: 2,
        bond_gain: 1,
        infamy_per_kill: 3,
        witness_grudge: 2,
        bonded_grudge: 2,
        desperation_rise: 2,
        desperation_fall: 3,
        desperation_floor: 0,
    };

    /// The constants in effect, as the two lines the UI and every verify report
    /// print (DESIGN §8a: a run is only reproducible if it says what it ran
    /// with).
    ///
    /// A function rather than a `format!` at each site so a check can ask the
    /// game for the exact text it draws: the font draws an unknown character as
    /// a box at a letter's width, so no assertion over drawn quads can see a
    /// wrong one and the string itself is the only instrument.
    pub fn readout(&self) -> String {
        // Four short lines rather than two long ones: the readout sits in the
        // roster column, and a line wider than the column runs under the panel
        // beside it. Nothing asserts a column width for it - the bounds check
        // sees the camera's edge and this never reaches it - so the width is
        // kept here, where the string is.
        format!(
            "K_inf {}  K_kill {}  K_loyal {}\n\
             bond +{}  infamy/kill +{}\n\
             witness {}  bonded {}\n\
             rise +{}  fall -{}  floor {}",
            self.k_inf,
            self.k_kill,
            self.k_loyal,
            self.bond_gain,
            self.infamy_per_kill,
            -self.witness_grudge,
            -self.bonded_grudge,
            self.desperation_rise,
            self.desperation_fall,
            self.desperation_floor,
        )
    }

    /// One field, by the name DESIGN §3.2 gives it — so a sweep, a mutation
    /// round and (next session) the tuning menu can walk the set rather than
    /// naming ten fields in three places.
    pub fn field_mut(&mut self, field: Field) -> &mut i32 {
        match field {
            Field::KInf => &mut self.k_inf,
            Field::KKill => &mut self.k_kill,
            Field::KLoyal => &mut self.k_loyal,
            Field::BondGain => &mut self.bond_gain,
            Field::InfamyPerKill => &mut self.infamy_per_kill,
            Field::WitnessGrudge => &mut self.witness_grudge,
            Field::BondedGrudge => &mut self.bonded_grudge,
            Field::DesperationRise => &mut self.desperation_rise,
            Field::DesperationFall => &mut self.desperation_fall,
            Field::DesperationFloor => &mut self.desperation_floor,
        }
    }

    /// One field, by name. `Tuning` is `Copy`, so this is the reader that
    /// `field_mut` would otherwise need a second match to provide.
    pub fn field(mut self, field: Field) -> i32 {
        *self.field_mut(field)
    }

    /// This set with one field replaced — what a mutation round varies.
    pub fn with(mut self, field: Field, value: i32) -> Self {
        *self.field_mut(field) = value;
        self
    }
}

/// Which tuning constant, by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// `K_inf`.
    KInf,
    /// `K_kill`.
    KKill,
    /// `K_loyal`.
    KLoyal,
    /// The clean-success bond.
    BondGain,
    /// The killer's infamy.
    InfamyPerKill,
    /// The witness's grudge.
    WitnessGrudge,
    /// The bonded witness's extra grudge.
    BondedGrudge,
    /// The idle round's desperation rise.
    DesperationRise,
    /// The profitable round's desperation fall.
    DesperationFall,
    /// The floor.
    DesperationFloor,
}

impl Field {
    /// Every field, in declaration order — what a sweep walks.
    pub const ALL: &'static [Field] = &[
        Field::KInf,
        Field::KKill,
        Field::KLoyal,
        Field::BondGain,
        Field::InfamyPerKill,
        Field::WitnessGrudge,
        Field::BondedGrudge,
        Field::DesperationRise,
        Field::DesperationFall,
        Field::DesperationFloor,
    ];

    /// The name DESIGN §3.2 gives this constant.
    pub fn name(self) -> &'static str {
        match self {
            Field::KInf => "K_inf",
            Field::KKill => "K_kill",
            Field::KLoyal => "K_loyal",
            Field::BondGain => "bond_gain",
            Field::InfamyPerKill => "infamy_per_kill",
            Field::WitnessGrudge => "witness_grudge",
            Field::BondedGrudge => "bonded_grudge",
            Field::DesperationRise => "desperation_rise",
            Field::DesperationFall => "desperation_fall",
            Field::DesperationFloor => "desperation_floor",
        }
    }
}
