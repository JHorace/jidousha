//! What a beat is, as data (DESIGN.md §10, §13): the authoring types, the
//! dungeon predicates, and the assertion vocabulary. The chain itself is
//! `chain.rs`, re-exported here so every reader keeps one path to it.
//!
//! A beat is `(initial roster state, dungeon(s), the intended dilemma stated in
//! a sentence, expected-outcome assertions)` and **nothing else** — no code, no
//! system, no branch anywhere that names a beat number.
//!
//! The fourth field is the verify scenario: `verify.rs` plays each beat through
//! `InputScript` and evaluates its `Expect` list against the world. So the
//! numbers in the chain are simultaneously the tutorial and the tuning
//! constants' regression harness — a constant that stops producing these
//! outcomes fails the run.

use crate::model::Social;
use crate::pressure::Band;
use crate::traits::{MarkId, MarkTone, TraitId};
use crate::willing::Verdict;

pub use crate::chain::CHAIN;

/// A character's authored starting state — the sheet, at the beat's opening.
#[derive(Clone, Copy, Debug)]
pub struct CharSpec {
    /// The name. ASCII, because the engine's font is (DESIGN §12).
    pub name: &'static str,
    /// Need at the start of the beat.
    pub desperation: i32,
    /// Why the need presses — bound at generation (DESIGN §3). Short, because
    /// the sheet's source row is one card column wide.
    pub source: &'static str,
    /// Accumulated profit at the start of the beat.
    pub wealth: i32,
    /// Who they are: at most `traits::TRAIT_CAP` ids, validated in
    /// `traits::vocabulary`.
    pub traits: &'static [TraitId],
    /// What everyone already knows about them.
    pub marks: &'static [MarkId],
    /// Clean jobs already walked away from, counting toward *reliable*.
    pub clean_jobs: i32,
}

/// An authored regard edge: `from` thinks `value` of `to`.
#[derive(Clone, Copy, Debug)]
pub struct EdgeSpec {
    /// Who holds the opinion.
    pub from: &'static str,
    /// Who it is about.
    pub to: &'static str,
    /// Positive is a bond, negative is a grudge.
    pub value: i32,
}

/// What a dungeon asks of a party beyond its headcount.
///
/// The growth axis (DESIGN §5): predicates come from the social vocabulary.
/// v1's known-face predicates migrated to marks — a job that needs a known
/// face is a job that needs a **dark mark** on somebody in the party (the
/// underworld register, DESIGN §5). The two beyond `AnyParty` are unused by
/// the tutorial beats and exercised directly in `judgment.rs` — a contract a
/// played beat never reaches is still a contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Requirement {
    /// Headcount only.
    AnyParty,
    /// At least one member wearing a dark mark — the underworld track's shape.
    NeedsDarkMark,
    /// Nobody wearing a dark mark — a job that cannot be seen with one.
    NoDarkMarks,
}

impl Requirement {
    /// Whether `party` satisfies this predicate.
    pub fn met(self, social: &Social, party: &[jidousha::prelude::Entity]) -> bool {
        let dark = |entity: &jidousha::prelude::Entity| {
            social
                .marks(*entity)
                .iter()
                .any(|mark| mark.tone() == MarkTone::Dark)
        };
        match self {
            Requirement::AnyParty => true,
            Requirement::NeedsDarkMark => party.iter().any(dark),
            Requirement::NoDarkMarks => !party.iter().any(dark),
        }
    }

    /// Why a party fails this predicate, as the send verb states it (UI.md §3).
    ///
    /// The reason a button is disabled, not the requirement it is about. A
    /// player who has read the panel knows the requirement; what they need
    /// from the button is which way theirs falls short.
    pub fn shortfall(self) -> &'static str {
        match self {
            // Unreachable: a predicate every party meets is never the reason.
            Requirement::AnyParty => "nothing",
            Requirement::NeedsDarkMark => "no dark mark in the party",
            Requirement::NoDarkMarks => "a dark mark in the party",
        }
    }

    /// The predicate as the dungeon panel states it.
    pub fn describe(self) -> String {
        match self {
            Requirement::AnyParty => "anyone who will come".to_owned(),
            Requirement::NeedsDarkMark => "somebody wearing a dark mark".to_owned(),
            Requirement::NoDarkMarks => "nobody wearing a dark mark".to_owned(),
        }
    }
}

/// Which dungeon icon a job carries.
///
/// A quest *type*, not a quest: UI.md §2 gives one icon per type and requires
/// it to be unique per type, so this enum and `assets/quest_*.png` are one
/// list. Presentation only — nothing in the rules reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestIcon {
    /// A mouth in a hillside.
    Cave,
    /// A stone that is somebody's.
    Crypt,
    /// Something that watches a road.
    Tower,
    /// Something with a door and a lock.
    Vault,
}

/// A job: what it asks for, what it pays, and what the player keeps.
///
/// Everything visible before assembly, like everything else (DESIGN §7).
#[derive(Clone, Copy, Debug)]
pub struct Dungeon {
    /// What it is called.
    pub name: &'static str,
    /// A sentence for the info panel (UI.md §3). Flavour; no rule reads it.
    pub blurb: &'static str,
    /// Which icon it carries.
    pub icon: QuestIcon,
    /// How many bodies it takes.
    pub headcount: usize,
    /// The whole pot.
    pub pot: i32,
    /// What the player takes off the top before the split.
    pub cut: i32,
    /// What it asks of the party's composition.
    pub requires: Requirement,
}

/// A claim about what a beat does, checked by `--verify`.
///
/// `Refuses`, `Joins`, `WillingnessIs`, `VerdictIs` and `TopReason` are claims
/// about the *assembly* moment — the social state the beat was authored with,
/// against the beat's own job. Everything else is a claim about the world
/// after the dungeon resolved.
#[derive(Clone, Copy, Debug)]
pub enum Expect {
    /// `who` will not join a party of these names.
    Refuses {
        /// The character asked.
        who: &'static str,
        /// The party, by name.
        party: &'static [&'static str],
    },
    /// `who` will join a party of these names.
    Joins {
        /// The character asked.
        who: &'static str,
        /// The party, by name.
        party: &'static [&'static str],
    },
    /// `who`'s margin for that party is exactly `total`.
    ///
    /// The sharper form of the two above, and the one a beat wants when the
    /// answer sits on the boundary: "Tim joins" passes at +7 as happily as at
    /// the 0 the beat is about.
    WillingnessIs {
        /// The character asked.
        who: &'static str,
        /// The party, by name.
        party: &'static [&'static str],
        /// The exact margin.
        total: i32,
    },
    /// `who`'s verdict for that party is exactly this (DESIGN §6): a beat
    /// about reluctance has to say so, or the reluctant band could vanish and
    /// every joins/refuses claim would still pass.
    VerdictIs {
        /// The character asked.
        who: &'static str,
        /// The party, by name.
        party: &'static [&'static str],
        /// The judgment.
        verdict: Verdict,
    },
    /// `who`'s leading reason for that party contains this text — the words a
    /// player reads, asserted as words (DESIGN §14).
    TopReason {
        /// The character asked.
        who: &'static str,
        /// The party, by name.
        party: &'static [&'static str],
        /// A fragment of the fixed-vocabulary string.
        fragment: &'static str,
    },
    /// `victim` is dead, killed by `by`.
    Killed {
        /// Who died.
        victim: &'static str,
        /// Who did it.
        by: &'static str,
    },
    /// `who` came back alive.
    Survives {
        /// The character.
        who: &'static str,
    },
    /// `who`'s desperation ends the beat at `value`.
    Desperation {
        /// The character.
        who: &'static str,
        /// The exact value.
        value: i32,
    },
    /// `who` ends the beat wearing this mark (DESIGN §5).
    HasMark {
        /// The character.
        who: &'static str,
        /// The mark.
        mark: MarkId,
    },
    /// `who` ends the beat *not* wearing it — the half that catches a mark
    /// written too eagerly.
    LacksMark {
        /// The character.
        who: &'static str,
        /// The mark.
        mark: MarkId,
    },
    /// `who`'s clean-job count ends the beat at `value`.
    CleanJobs {
        /// The character.
        who: &'static str,
        /// The exact count.
        value: i32,
    },
    /// `who`'s wealth ends the beat at `value`.
    Wealth {
        /// The character.
        who: &'static str,
        /// The exact value.
        value: i32,
    },
    /// `regard(from -> to)` ends the beat at `value`.
    Regard {
        /// Who holds the opinion.
        from: &'static str,
        /// Who it is about.
        to: &'static str,
        /// The exact value.
        value: i32,
    },
    /// Some line of the resolution report contains this text.
    ///
    /// The report is the story surface (DESIGN §12) and its arithmetic is what a
    /// player learns the rules from, so the narration is asserted rather than
    /// assumed: a beat that produces the right world state and describes it
    /// wrongly has broken the half of the game a player reads.
    ReportSays {
        /// The fragment.
        fragment: &'static str,
    },
    /// The staged party's band chip reads exactly this (DESIGN §7a) — an
    /// assembly-moment claim about the surface the foreshadowing lives on.
    /// Only meaningful under a rule set that foreshadows, so it belongs on
    /// the `ladder` list.
    BandIs {
        /// The band.
        band: Band,
    },
    /// `who`'s pressure against the staged party is exactly `total` — the
    /// number the occurrence roll consumes, hand-computed from the sheet and
    /// the constants. The instrument that catches any pressure-model constant
    /// moving.
    PressureIs {
        /// The member.
        who: &'static str,
        /// The exact pressure.
        total: i32,
    },
}

/// One authored dilemma.
pub struct BeatSpec {
    /// What the beat is called, on screen.
    pub title: &'static str,
    /// The intended dilemma, in a sentence.
    pub dilemma: &'static str,
    /// The one concept it introduces.
    pub teaches: &'static str,
    /// The roster, in roster order — which is the betrayal evaluation order.
    pub roster: &'static [CharSpec],
    /// The regard edges that exist at the start. Absent is zero.
    pub edges: &'static [EdgeSpec],
    /// The jobs on offer. The player picks one; every beat here offers one.
    pub dungeons: &'static [Dungeon],
    /// The party the verify run assembles, by name - the intended solution.
    ///
    /// Part of the fourth field (DESIGN §10: "the verify scenario"), not of the
    /// rules: nothing in the game reads it, and a player is free to send
    /// anything the gate allows.
    pub send: &'static [&'static str],
    /// The beat's fixed seed (DESIGN §12): the dice this scenario rolls when
    /// no `?seed=` overrides it. Authored data, like the roster — chosen so
    /// the ladder tells this beat's story, and any other seed is a legal
    /// playthrough rather than the tutorial's.
    pub seed: u64,
    /// What playing it correctly produces **under the deterministic variant**
    /// — v1's assertions, preserved with the rule they assert.
    pub expect: &'static [Expect],
    /// What playing it at the fixed seed produces **under the ladder**
    /// (DESIGN §8e: the ladder beats are fixed-seed). Assembly-moment claims
    /// from `expect` hold under both variants and are re-judged there;
    /// resolution claims live here, per rule set.
    pub ladder: &'static [Expect],
}

impl BeatSpec {
    /// Where `name` sits in this beat's roster, if it is in it.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.roster.iter().position(|spec| spec.name == name)
    }
}
