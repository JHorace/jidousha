//! Every tuning constant giri has, in one place (DESIGN.md §6).
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
//! live tuning menu (DESIGN §8a) both need to vary these without rebuilding:
//! `headless(..)` builds a fresh game per candidate and `Startup` takes
//! whatever the harness left in the world. `docs/api/jidousha-testing.md`
//! makes the trade explicit — a game with two numbers should stay with
//! constants; this one has thirteen and its verify mode sweeps them every run.
//!
//! **Three readers, one set of names.** `Field::name` is the name DESIGN gives
//! a constant, and it is the only name there is: the drawer's rows print it,
//! `stamp` writes it lowercased into the compact form a recording and a
//! `?constants=` link carry, and `parse` reads that form back by matching it
//! case-insensitively. A second table of URL keys would be a second name per
//! constant, and the day they disagreed the link would load a set the stamp
//! denied.

use jidousha::prelude::*;

/// The social model's weights — one struct, one shipped set.
///
/// Names follow DESIGN §6. Integers, not floats: every beat is meant
/// to be exactly computable by a player with the sheets in front of them, and
/// a claim like "desperation 8 reaches 6" is a claim about integers. It is
/// also what makes the assertions in `--verify` exact rather than approximate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tuning {
    /// `K_kill` — the desperation at or above which betrayal is on the table.
    pub k_kill: i32,
    /// `K_loyal` — the regard at or above which a character will not betray.
    pub k_loyal: i32,
    /// The verdict boundary (DESIGN §6): a non-negative margin below this is
    /// *reluctant* — in, but barely, and P2's strain reads exactly that.
    pub reluctant_below: i32,
    /// What one dark mark costs a stranger's willingness, before traits.
    pub mark_dark: i32,
    /// What one light mark earns it, before traits.
    pub mark_light: i32,
    /// How hard one gold of promised share pulls, per point of a trait's pot
    /// affinity. The pot enters willingness only through traits in P1
    /// (DESIGN §6).
    pub pot_pull: i32,
    /// Clean jobs it takes to be marked *reliable* (DESIGN §5).
    pub reliable_after: i32,
    /// What a clean shared success adds to regard, both ways, per surviving pair.
    pub bond_gain: i32,
    /// What a surviving witness's regard toward the killer drops by.
    pub witness_grudge: i32,
    /// The extra drop when that witness had positive regard for the victim.
    ///
    /// Bonds propagate consequences (DESIGN §6): harm to someone you are
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
    /// a character at -2, and a willingness of -2 refuses a job with nothing
    /// else wrong with it. At the floor a character still takes a clean job
    /// (0 >= 0) and nothing that costs them.
    pub desperation_floor: i32,
}

impl Resource for Tuning {}

impl Tuning {
    /// The smallest value the drawer's steppers and a `?constants=` link offer.
    ///
    /// A bound on the *tuning surface*, not on the type: the mutation round
    /// deliberately moves a constant to 99 and the floor to -99, because a
    /// perturbation has to be one nothing plausibly authors. What a person can
    /// reach by clicking, and what a shared link may carry, is this range.
    pub const MIN: i32 = 0;
    /// And the largest.
    pub const MAX: i32 = 12;

    /// What the game ships with — the set the four tutorial beats are authored
    /// against, and the set every verify run stamps into its report.
    pub const SHIPPED: Self = Self {
        k_kill: 6,
        k_loyal: 2,
        reluctant_below: 2,
        mark_dark: 1,
        mark_light: 1,
        pot_pull: 1,
        reliable_after: 2,
        bond_gain: 1,
        witness_grudge: 2,
        bonded_grudge: 2,
        desperation_rise: 2,
        desperation_fall: 3,
        desperation_floor: 0,
    };

    /// The constants in effect, as the lines the drawer's stamp and every
    /// verify report print (DESIGN §8a: a run is only reproducible if it says
    /// what it ran with).
    ///
    /// A function rather than a `format!` at each site so a check can ask the
    /// game for the exact text it draws: the font draws an unknown character as
    /// a box at a letter's width, so no assertion over drawn quads can see a
    /// wrong one and the string itself is the only instrument.
    pub fn readout(&self) -> String {
        // Six short lines rather than two long ones: the readout sits under the
        // drawer's third stepper column, which is 264 reference pixels wide,
        // and a line wider than the column runs into the prose band beside it.
        // Nothing asserts a column width for it, so the width is kept here,
        // where the string is.
        format!(
            "K_kill {}  K_loyal {}\n\
             reluctant {}  pot {}\n\
             dark {}  light {}\n\
             reliable {}  bond +{}\n\
             witness {}  bonded {}\n\
             rise +{}  fall -{}  floor {}",
            self.k_kill,
            self.k_loyal,
            self.reluctant_below,
            self.pot_pull,
            self.mark_dark,
            self.mark_light,
            self.reliable_after,
            self.bond_gain,
            -self.witness_grudge,
            -self.bonded_grudge,
            self.desperation_rise,
            self.desperation_fall,
            self.desperation_floor,
        )
    }

    /// One field, by the name DESIGN gives it — so a sweep, a mutation round
    /// and the tuning menu can walk the set rather than naming thirteen fields
    /// in three places.
    pub fn field_mut(&mut self, field: Field) -> &mut i32 {
        match field {
            Field::KKill => &mut self.k_kill,
            Field::KLoyal => &mut self.k_loyal,
            Field::ReluctantBelow => &mut self.reluctant_below,
            Field::MarkDark => &mut self.mark_dark,
            Field::MarkLight => &mut self.mark_light,
            Field::PotPull => &mut self.pot_pull,
            Field::ReliableAfter => &mut self.reliable_after,
            Field::BondGain => &mut self.bond_gain,
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

    /// The whole set on one line, in the compact form a link carries and a log
    /// line records: `k_kill:6,k_loyal:2,...` (UI.md §12).
    ///
    /// `readout` is the same fact laid out for a person to read in a column;
    /// this is the same fact laid out for a URL and for `parse` to read back.
    /// Both walk `Field::ALL`, so a constant added to the enum appears in both
    /// without either being edited.
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
    /// `?constants=k_kill:4` — mean "the shipped set with one thing moved". A
    /// key that is not a constant, a value that is not a number, a value
    /// outside the drawer's range, and a key given twice are all refusals
    /// (UI.md §12: rejected loudly, never silently clamped).
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
            let Ok(number) = value.parse::<i32>() else {
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
        value: i32,
    },
}

impl ConstantsError {
    /// What the page says: what happened, and what to write instead.
    ///
    /// ASCII and one line, because it is drawn with the same font every other
    /// string is (DESIGN §12) and the drawer gives it two wrapped rows.
    pub fn message(&self) -> String {
        match self {
            ConstantsError::Empty => {
                "?constants= was empty - write it as k_kill:6,k_loyal:2 or leave it off".to_owned()
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
    /// `K_kill`.
    KKill,
    /// `K_loyal`.
    KLoyal,
    /// The reluctant verdict's boundary.
    ReluctantBelow,
    /// A dark mark's base cost.
    MarkDark,
    /// A light mark's base pull.
    MarkLight,
    /// The pot's pull per share gold, through traits.
    PotPull,
    /// Clean jobs to a *reliable* mark.
    ReliableAfter,
    /// The clean-success bond.
    BondGain,
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
        Field::KKill,
        Field::KLoyal,
        Field::ReluctantBelow,
        Field::MarkDark,
        Field::MarkLight,
        Field::PotPull,
        Field::ReliableAfter,
        Field::BondGain,
        Field::WitnessGrudge,
        Field::BondedGrudge,
        Field::DesperationRise,
        Field::DesperationFall,
        Field::DesperationFloor,
    ];

    /// The name DESIGN gives this constant.
    pub fn name(self) -> &'static str {
        match self {
            Field::KKill => "K_kill",
            Field::KLoyal => "K_loyal",
            Field::ReluctantBelow => "reluctant_below",
            Field::MarkDark => "mark_dark",
            Field::MarkLight => "mark_light",
            Field::PotPull => "pot_pull",
            Field::ReliableAfter => "reliable_after",
            Field::BondGain => "bond_gain",
            Field::WitnessGrudge => "witness_grudge",
            Field::BondedGrudge => "bonded_grudge",
            Field::DesperationRise => "desperation_rise",
            Field::DesperationFall => "desperation_fall",
            Field::DesperationFloor => "desperation_floor",
        }
    }

    /// The same name, lowercased — what a stamp writes and a link carries.
    ///
    /// Derived rather than tabled: `K_kill` and `k_kill` are one name in two
    /// cases, and a second table would be a second name (see the module
    /// header).
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

    /// What this constant does, in one line — the drawer's hover text
    /// (UI.md §12).
    ///
    /// Written for a person who is about to move it: what goes up when the
    /// number goes up. Kept under fifty characters, which is the width the
    /// drawer's hint row has.
    pub fn meaning(self) -> &'static str {
        match self {
            Field::KKill => "desperation at which betrayal is on the table",
            Field::KLoyal => "regard at which a partymate is safe",
            Field::ReluctantBelow => "margins under this join reluctantly",
            Field::MarkDark => "what one dark mark costs a stranger",
            Field::MarkLight => "what one light mark earns with strangers",
            Field::PotPull => "pull per share gold, through pot traits",
            Field::ReliableAfter => "clean jobs it takes to be marked reliable",
            Field::BondGain => "regard a clean shared job leaves, both ways",
            Field::WitnessGrudge => "regard a witness drops toward the killer",
            Field::BondedGrudge => "the extra drop if they loved the victim",
            Field::DesperationRise => "desperation a round without profit adds",
            Field::DesperationFall => "desperation a paid survivor sheds",
            Field::DesperationFloor => "how low desperation is allowed to go",
        }
    }
}
