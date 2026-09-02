//! The people vocabulary: traits, reputation marks, and the trait x mark
//! reaction table (DESIGN.md §4, §5).
//!
//! **Everything here is data the decision function reads — never a branch in a
//! system.** A trait is a row of multipliers and table entries; the one
//! willingness function applies them to its terms, and no gameplay code
//! anywhere asks "is this character greedy" (§4's discipline: traits
//! parameterize, they never branch). The same holds for marks: a mark is a
//! datum on a sheet, and what it *does* is entirely the reaction the table
//! below assigns to whoever is looking at it.
//!
//! **Marks replace the retired public scalar** (DESIGN §5). Public knowledge is
//! qualitative, earned, plural: a murder writes *comrade-killer*, enough clean
//! jobs write *reliable*, and a reaction can attract as well as repel — the
//! pragmatic prefer a known skimmer to a stranger, which is the entry that
//! resolves v1's only-closes-doors problem structurally.
//!
//! The final list is tuning content, not architecture (DESIGN §4): a trait is
//! added by adding a row to `TRAITS`, a reaction by adding a row to
//! `REACTIONS`, and `vocabulary()` is the validation that keeps the tables
//! honest — the sheet cap is enforced there, not in prose.

use crate::checks::Checks;
use crate::sprites::Art;

/// The most traits a character may carry (DESIGN §3: caps are design
/// decisions, not UI accommodations).
pub const TRAIT_CAP: usize = 3;

/// Who a character is — one of a small, closed, data-defined vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraitId {
    /// The pot weighs on them; skim-prone when the ladder lands (P2).
    Greedy,
    /// Bonds weigh double.
    Loyal,
    /// Will not stand next to a thief; refuses charity when goals land (P3).
    Proud,
    /// Fears a killer; danger terms weigh double when danger exists (P2).
    Craven,
    /// Grudges weigh double, and never decay.
    Vengeful,
    /// Reacts to dark marks by kind, not size.
    Pious,
    /// Prefers known quantities — a marked skimmer over a stranger.
    Pragmatic,
    /// Edges weigh half, both ways.
    Cold,
    /// The upright refuse the dark-marked, whatever it costs them.
    Upright,
}

/// One trait: its name, its interim icon role, and its modifiers to the
/// decision function's terms.
///
/// The multipliers are exact rationals (`num`/`den`) rather than floats, so a
/// beat stays exactly computable; the mark-reaction half of a trait lives in
/// [`REACTIONS`], keyed by this id.
#[derive(Clone, Copy, Debug)]
pub struct TraitDef {
    /// Which trait this row defines.
    pub id: TraitId,
    /// The display name, ASCII lowercase — what the chip says.
    pub name: &'static str,
    /// One behavioral line — the gist of who this is, never the interaction
    /// list (UI.md §14; stranger-facing copy). Shown on trait-chip hover.
    pub line: &'static str,
    /// The chip's icon: a *category* icon from the existing library, per the
    /// interim rules (UI.md §13) — the UI session designs real ones.
    pub icon: Art,
    /// Multiplier on positive regard (bonds), as `num/den`.
    pub bond_num: i32,
    /// Its denominator.
    pub bond_den: i32,
    /// Multiplier on negative regard (grudges), as `num/den`.
    pub grudge_num: i32,
    /// Its denominator.
    pub grudge_den: i32,
    /// How strongly the pot pulls this character, in shares: the pot term is
    /// `share x pot_pull x` this affinity. Zero for most — in P1 the pot
    /// enters willingness only through traits (DESIGN §6).
    pub pot_affinity: i32,
}

/// The trait vocabulary — the whole of it, as data.
pub const TRAITS: &[TraitDef] = &[
    TraitDef {
        id: TraitId::Greedy,
        name: "greedy",
        line: "the money talks louder than the company",
        icon: Art::Coin,
        bond_num: 1,
        bond_den: 1,
        grudge_num: 1,
        grudge_den: 1,
        pot_affinity: 1,
    },
    TraitDef {
        id: TraitId::Loyal,
        name: "loyal",
        line: "holds hard to the people they trust",
        icon: Art::Heart,
        bond_num: 2,
        bond_den: 1,
        grudge_num: 1,
        grudge_den: 1,
        pot_affinity: 0,
    },
    TraitDef {
        id: TraitId::Proud,
        name: "proud",
        line: "won't stand next to a thief",
        icon: Art::Eye,
        bond_num: 1,
        bond_den: 1,
        grudge_num: 1,
        grudge_den: 1,
        pot_affinity: 0,
    },
    TraitDef {
        id: TraitId::Craven,
        name: "craven",
        line: "runs when it turns dangerous",
        icon: Art::Flame,
        bond_num: 1,
        bond_den: 1,
        grudge_num: 1,
        grudge_den: 1,
        pot_affinity: 0,
    },
    TraitDef {
        id: TraitId::Vengeful,
        name: "vengeful",
        line: "never lets a wrong go",
        icon: Art::Skull,
        bond_num: 1,
        bond_den: 1,
        grudge_num: 2,
        grudge_den: 1,
        pot_affinity: 0,
    },
    TraitDef {
        id: TraitId::Pious,
        name: "pious",
        line: "counts sins, not their size",
        icon: Art::Eye,
        bond_num: 1,
        bond_den: 1,
        grudge_num: 1,
        grudge_den: 1,
        pot_affinity: 0,
    },
    TraitDef {
        id: TraitId::Pragmatic,
        name: "pragmatic",
        line: "prefers a known quantity to a stranger",
        icon: Art::Coin,
        bond_num: 1,
        bond_den: 1,
        grudge_num: 1,
        grudge_den: 1,
        pot_affinity: 0,
    },
    TraitDef {
        id: TraitId::Cold,
        name: "cold",
        line: "feelings weigh little, either way",
        icon: Art::Skull,
        bond_num: 1,
        bond_den: 2,
        grudge_num: 1,
        grudge_den: 2,
        pot_affinity: 0,
    },
    TraitDef {
        id: TraitId::Upright,
        name: "upright",
        line: "won't work with criminals",
        icon: Art::Eye,
        bond_num: 1,
        bond_den: 1,
        grudge_num: 1,
        grudge_den: 1,
        pot_affinity: 0,
    },
];

impl TraitId {
    /// This trait's row of the vocabulary.
    pub fn def(self) -> &'static TraitDef {
        // A linear walk over nine entries; `vocabulary()` asserts every id has
        // exactly one row, so the fallback is unreachable in a valid table.
        TRAITS
            .iter()
            .find(|def| def.id == self)
            .unwrap_or(&TRAITS[0])
    }

    /// The chip's text.
    pub fn name(self) -> &'static str {
        self.def().name
    }
}

/// Which side of the ledger a mark sits on (DESIGN §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkTone {
    /// Repels by default: `mark_dark` is what one costs.
    Dark,
    /// Attracts by default: `mark_light` is what one earns.
    Light,
    /// Neither, until a trait says otherwise.
    Ambiguous,
}

/// What everyone knows about a character — qualitative, earned, plural.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkId {
    /// Took an extra share, quietly. (Written by the ladder — P2.)
    Skimmer,
    /// Walked mid-quest. (P2.)
    Deserter,
    /// The quest suffered; someone got hurt. (P2.)
    Saboteur,
    /// The summit: killed a partymate. Written by murder, this phase.
    ComradeKiller,
    /// N clean jobs — written by the counting, this phase.
    Reliable,
    /// Held when holding cost something. (Written by the ladder era — P2.)
    KeptTheLine,
    /// Parties die around this one. Ambiguous on purpose.
    Survivor,
}

impl MarkId {
    /// Every mark, in the order sheets list them.
    pub const ALL: &'static [MarkId] = &[
        MarkId::Skimmer,
        MarkId::Deserter,
        MarkId::Saboteur,
        MarkId::ComradeKiller,
        MarkId::Reliable,
        MarkId::KeptTheLine,
        MarkId::Survivor,
    ];

    /// The mark as sheets and reasons print it.
    pub fn name(self) -> &'static str {
        match self {
            MarkId::Skimmer => "skimmer",
            MarkId::Deserter => "deserter",
            MarkId::Saboteur => "saboteur",
            MarkId::ComradeKiller => "comrade-killer",
            MarkId::Reliable => "reliable",
            MarkId::KeptTheLine => "kept-the-line",
            MarkId::Survivor => "survivor",
        }
    }

    /// Which side of the ledger.
    pub fn tone(self) -> MarkTone {
        match self {
            MarkId::Skimmer | MarkId::Deserter | MarkId::Saboteur | MarkId::ComradeKiller => {
                MarkTone::Dark
            }
            MarkId::Reliable | MarkId::KeptTheLine => MarkTone::Light,
            MarkId::Survivor => MarkTone::Ambiguous,
        }
    }
}

/// One cell of the trait x mark table: what carrying `trait_id` adds to the
/// base reaction when looking at somebody wearing `mark`.
///
/// **Reactions open doors as well as close them** (DESIGN §5): a positive
/// delta is an attraction, and `(Pragmatic, Skimmer, +2)` against a base of
/// `-mark_dark` is the entry that makes a known skimmer *preferable* to a
/// stranger.
#[derive(Clone, Copy, Debug)]
pub struct Reaction {
    /// Whose trait is reacting.
    pub trait_id: TraitId,
    /// To whose mark.
    pub mark: MarkId,
    /// Added to the tone's base reaction.
    pub delta: i32,
}

/// The trait x mark table — tuning content, one row per cell that is not the
/// tone's base.
pub const REACTIONS: &[Reaction] = &[
    // The upright refuse the dark-marked (DESIGN §5's own example).
    Reaction {
        trait_id: TraitId::Upright,
        mark: MarkId::Skimmer,
        delta: -2,
    },
    Reaction {
        trait_id: TraitId::Upright,
        mark: MarkId::Deserter,
        delta: -2,
    },
    Reaction {
        trait_id: TraitId::Upright,
        mark: MarkId::Saboteur,
        delta: -2,
    },
    Reaction {
        trait_id: TraitId::Upright,
        mark: MarkId::ComradeKiller,
        delta: -2,
    },
    // The pragmatic prefer a known quantity: the attraction entry.
    Reaction {
        trait_id: TraitId::Pragmatic,
        mark: MarkId::Skimmer,
        delta: 2,
    },
    // The pious react to the kind, not the size: every dark mark is one more
    // sin, flatly.
    Reaction {
        trait_id: TraitId::Pious,
        mark: MarkId::Skimmer,
        delta: -1,
    },
    Reaction {
        trait_id: TraitId::Pious,
        mark: MarkId::Deserter,
        delta: -1,
    },
    Reaction {
        trait_id: TraitId::Pious,
        mark: MarkId::Saboteur,
        delta: -1,
    },
    Reaction {
        trait_id: TraitId::Pious,
        mark: MarkId::ComradeKiller,
        delta: -1,
    },
    // The proud will not stand with a thief.
    Reaction {
        trait_id: TraitId::Proud,
        mark: MarkId::Skimmer,
        delta: -2,
    },
    // The craven fear a killer more than they mind a thief.
    Reaction {
        trait_id: TraitId::Craven,
        mark: MarkId::ComradeKiller,
        delta: -2,
    },
    // The craven keep clear of the cursed, too.
    Reaction {
        trait_id: TraitId::Craven,
        mark: MarkId::Survivor,
        delta: -1,
    },
    // The pragmatic read a survivor as somebody who comes back.
    Reaction {
        trait_id: TraitId::Pragmatic,
        mark: MarkId::Survivor,
        delta: 1,
    },
];

/// What `trait_id` adds to the reaction to `mark`, beyond the tone's base.
pub fn reaction_delta(trait_id: TraitId, mark: MarkId) -> i32 {
    REACTIONS
        .iter()
        .filter(|cell| cell.trait_id == trait_id && cell.mark == mark)
        .map(|cell| cell.delta)
        .sum()
}

/// The vocabulary's own validation — the data-shape claims prose cannot hold.
///
/// The sheet cap is enforced here rather than trusted (DESIGN §3): every
/// authored character in the chain is walked, and a fifth trait or a repeated
/// mark is a failed run, not a crowded card.
pub fn vocabulary(checks: &mut Checks) {
    // Every trait id has exactly one row, and names are chip-shaped.
    for (index, def) in TRAITS.iter().enumerate() {
        checks.require(
            TRAITS.iter().filter(|other| other.id == def.id).count() == 1,
            "a trait has two rows in the vocabulary",
            format!("{:?} appears more than once in TRAITS", def.id),
        );
        checks.require(
            !def.name.is_empty()
                && def
                    .name
                    .chars()
                    .all(|glyph| glyph.is_ascii_lowercase() || glyph == '-'),
            "a trait's name is not chip-shaped ASCII",
            format!("TRAITS[{index}] is named {:?}", def.name),
        );
        checks.require(
            !def.line.is_empty()
                && def.line.chars().all(|glyph| (' '..='~').contains(&glyph))
                && def.line.chars().count() <= 56,
            "a trait's description is not one stranger-facing ASCII line",
            format!(
                "{:?}'s line is {:?} ({} chars); one behavioral line, ASCII, short enough \
                 for the note band (UI.md §14)",
                def.id,
                def.line,
                def.line.chars().count()
            ),
        );
        checks.require(
            def.bond_den > 0 && def.grudge_den > 0,
            "a trait multiplies regard by a rational with no denominator",
            format!(
                "{:?} has bond {}/{} and grudge {}/{}",
                def.id, def.bond_num, def.bond_den, def.grudge_num, def.grudge_den
            ),
        );
    }
    checks.require(
        (8..=20).contains(&TRAITS.len()),
        "the trait vocabulary is outside the size the design names",
        format!(
            "TRAITS has {} rows; DESIGN §4 says a small closed vocabulary, order of 12-20, \
             and P1 starts with 8-12",
            TRAITS.len()
        ),
    );
    // Every mark has a printable name.
    for mark in MarkId::ALL.iter().copied() {
        checks.require(
            mark.name()
                .chars()
                .all(|glyph| glyph.is_ascii_lowercase() || glyph == '-'),
            "a mark's name is not sheet-shaped ASCII",
            format!("{mark:?} is named {:?}", mark.name()),
        );
    }
    // One cell per (trait, mark): a duplicated cell would sum silently.
    for (index, cell) in REACTIONS.iter().enumerate() {
        checks.require(
            REACTIONS
                .iter()
                .filter(|other| other.trait_id == cell.trait_id && other.mark == cell.mark)
                .count()
                == 1,
            "the trait x mark table has two cells for one pair",
            format!(
                "REACTIONS[{index}] repeats ({:?}, {:?})",
                cell.trait_id, cell.mark
            ),
        );
        checks.require(
            cell.delta != 0,
            "the trait x mark table carries a cell that does nothing",
            format!(
                "({:?}, {:?}) has a delta of 0; the base reaction is the tone's, and a zero \
                 row is a row somebody will edit instead of delete",
                cell.trait_id, cell.mark
            ),
        );
    }
    // The sheet caps, over every authored character in the chain.
    for (beat, spec) in crate::beats::CHAIN.iter().enumerate() {
        for character in spec.roster {
            checks.require(
                character.traits.len() <= TRAIT_CAP,
                "a character carries more traits than the sheet cap allows",
                format!(
                    "beat {}: {} has {} traits and the cap is {TRAIT_CAP} (DESIGN §3: caps \
                     are design decisions)",
                    beat + 1,
                    character.name,
                    character.traits.len()
                ),
            );
            for (index, trait_id) in character.traits.iter().enumerate() {
                checks.require(
                    !character.traits[..index].contains(trait_id),
                    "a character carries the same trait twice",
                    format!(
                        "beat {}: {} repeats {:?}",
                        beat + 1,
                        character.name,
                        trait_id
                    ),
                );
            }
            for (index, mark) in character.marks.iter().enumerate() {
                checks.require(
                    !character.marks[..index].contains(mark),
                    "a character carries the same mark twice",
                    format!("beat {}: {} repeats {:?}", beat + 1, character.name, mark),
                );
            }
            checks.require(
                !character.source.is_empty(),
                "a character's desperation has no source",
                format!(
                    "beat {}: {}'s source is empty; DESIGN §3 binds one at generation",
                    beat + 1,
                    character.name
                ),
            );
        }
    }
}
