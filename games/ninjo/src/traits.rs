//! The people vocabulary: traits, reputation marks, and the trait x mark
//! reaction table (GDD §3, §4.3, §6).
//!
//! Ported from giri mainline's `traits.rs` by copy-adapt — **not by
//! dependency**. giri stays a separate crate, untouched and green; this file
//! could be read beside it, and the fork would still build with giri deleted.
//! What changed in the port is one field: every row now declares a
//! [`TraitKind`].
//!
//! **Everything here is data a decision function reads — never a branch in a
//! system.** A trait is a row of multipliers and table entries; the functions
//! that will read them (the scorer, needs, resolution — waves 1 and up) apply
//! them to their terms, and no gameplay code anywhere asks "is this character
//! greedy". The `kind` field does not change that and is not an exception to
//! it: **a kind gates which data fields of a row apply, never which code path
//! runs.** [`vocabulary`] enforces exactly that by asserting every row holds
//! the neutral value in every field its kind does not own — so a motivator
//! cannot quietly carry a bond multiplier, and nothing has to test the kind at
//! a call site to know it may add the row's multiplier in.
//!
//! **Marks are the public half** (giri v2 semantics, carried unchanged): a
//! mark is a datum on a sheet, and what it *does* is entirely the reaction
//! [`REACTIONS`] assigns to whoever is looking at it. Reactions open doors as
//! well as close them — the pragmatic prefer a known skimmer to a stranger.
//!
//! **No list cap.** giri capped a sheet at three traits because a card had to
//! hold them; ninjo's attention architecture owns legibility (GDD §3, revisit
//! at the MVP gate), so the cap is gone and `vocabulary` no longer asserts
//! one. The list's *content* — which traits ninjo's cast actually carries — is
//! an open ledger item (GDD §10); the motivator and aptitude rows below are
//! **placeholder content, flagged for the trait-content pass**, authored to
//! prove the format rather than to be played.

use crate::checks::Checks;
use crate::constants::Tuning;
use crate::sprites::Art;

/// Which family a trait belongs to (GDD §3, §6).
///
/// **A kind gates data, not behaviour.** It says which of a [`TraitDef`]'s
/// modifier fields are meaningful for that row — and therefore which
/// authored numbers a reader may expect to find — so that every consumer can
/// apply every field unconditionally: a kind's inapplicable fields are held
/// at their neutral values by [`vocabulary`], and adding a neutral value in
/// is a no-op. Nothing may match on this to pick a code path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraitKind {
    /// Who somebody is. Owns the regard multipliers, the pot's pull, and the
    /// rows of the trait x mark table.
    Personality,
    /// What somebody is after. Owns the upkeep multiplier and the pressure a
    /// want puts on the scorer.
    Motivator,
    /// What somebody is good at. Owns the competence value tasks read.
    Aptitude,
}

impl TraitKind {
    /// Every kind, in declaration order — what a walk over the vocabulary
    /// groups by.
    pub const ALL: &'static [TraitKind] = &[
        TraitKind::Personality,
        TraitKind::Motivator,
        TraitKind::Aptitude,
    ];

    /// The kind's name, as data files and reports spell it.
    pub fn name(self) -> &'static str {
        match self {
            TraitKind::Personality => "personality",
            TraitKind::Motivator => "motivator",
            TraitKind::Aptitude => "aptitude",
        }
    }
}

/// Who a character is, wants, or can do — one of an open, data-defined
/// vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TraitId {
    // ── personality (giri's nine, ported whole) ──────────────────────────
    /// The pot weighs on them; skim-prone when a ladder lands.
    Greedy,
    /// Bonds weigh double.
    Loyal,
    /// Will not stand next to a thief; refuses charity.
    Proud,
    /// Fears a killer; danger terms weigh double when danger exists.
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
    // ── motivator (placeholder content; the format is the point) ─────────
    /// Somebody else eats first: upkeep costs more and the need presses.
    Provider,
    /// Wants to be somebody, and spends like it.
    Ambitious,
    /// Wants to go home, and takes little while they save for it.
    Homesick,
    // ── aptitude (placeholder content; the format is the point) ──────────
    /// Does the heavy work without complaint.
    Strong,
    /// Good hands, quick work.
    Deft,
    /// Reads, counts, remembers.
    Learned,
}

/// One trait: its family, its name, its interim icon role, and its modifiers.
///
/// The multipliers are exact rationals (`num`/`den`) rather than floats, so an
/// outcome stays exactly computable; the mark-reaction half of a personality
/// lives in [`REACTIONS`], keyed by this id.
///
/// **Every field is here for every row.** Which of them a row is allowed to
/// move is its [`TraitKind`]'s business, asserted in [`vocabulary`]; a reader
/// applies them all and the neutral ones do nothing.
#[derive(Clone, Copy, Debug)]
pub struct TraitDef {
    /// Which trait this row defines.
    pub id: TraitId,
    /// The display name, ASCII lowercase — what a chip says.
    pub name: &'static str,
    /// Which family it belongs to, and so which fields below it may move.
    pub kind: TraitKind,
    /// One behavioral line — the gist of who this is, stranger-facing, never
    /// the interaction list (UI.md §14, carried from giri).
    pub line: &'static str,
    /// The chip's icon: a *category* icon from the existing library, per the
    /// interim rules — the UI session designs real ones.
    pub icon: Art,
    /// **Personality.** Multiplier on positive regard (bonds), as `num/den`.
    pub bond_num: i64,
    /// Its denominator.
    pub bond_den: i64,
    /// **Personality.** Multiplier on negative regard (grudges), as `num/den`.
    pub grudge_num: i64,
    /// Its denominator.
    pub grudge_den: i64,
    /// **Personality.** How strongly a promised share pulls this character,
    /// per gold: the pot term is `share x pot_pull x` this affinity.
    pub pot_affinity: i64,
    /// **Motivator.** Multiplier on this character's upkeep, as `num/den` —
    /// what a want costs to carry (GDD §5's needs module reads it).
    pub upkeep_num: i64,
    /// Its denominator.
    pub upkeep_den: i64,
    /// **Motivator.** What this want adds to the scorer's weight for the
    /// actions that serve it (GDD §5's autonomy module reads it).
    pub pressure: i64,
    /// **Aptitude.** Competence at the work this trait names — what task
    /// resolution reads (GDD §5's resolution module).
    pub aptitude: i64,
}

/// The neutral row: what every field holds when its kind does not own it.
///
/// Stated once, so [`vocabulary`]'s neutrality assertion and the authored rows
/// below cannot drift apart, and so a field added to [`TraitDef`] has exactly
/// one place to declare what "does nothing" means for it.
pub const NEUTRAL: TraitDef = TraitDef {
    id: TraitId::Greedy,
    name: "",
    kind: TraitKind::Personality,
    line: "",
    icon: Art::Coin,
    bond_num: 1,
    bond_den: 1,
    grudge_num: 1,
    grudge_den: 1,
    pot_affinity: 0,
    upkeep_num: 1,
    upkeep_den: 1,
    pressure: 0,
    aptitude: 0,
};

/// The trait vocabulary — the whole of it, as data.
pub const TRAITS: &[TraitDef] = &[
    // ── personality ──────────────────────────────────────────────────────
    TraitDef {
        id: TraitId::Greedy,
        name: "greedy",
        kind: TraitKind::Personality,
        line: "the money talks louder than the company",
        icon: Art::Coin,
        pot_affinity: 1,
        ..NEUTRAL
    },
    TraitDef {
        id: TraitId::Loyal,
        name: "loyal",
        kind: TraitKind::Personality,
        line: "holds hard to the people they trust",
        icon: Art::Heart,
        bond_num: 2,
        ..NEUTRAL
    },
    TraitDef {
        id: TraitId::Proud,
        name: "proud",
        kind: TraitKind::Personality,
        line: "won't stand next to a thief",
        icon: Art::Eye,
        ..NEUTRAL
    },
    TraitDef {
        id: TraitId::Craven,
        name: "craven",
        kind: TraitKind::Personality,
        line: "runs when it turns dangerous",
        icon: Art::Flame,
        ..NEUTRAL
    },
    TraitDef {
        id: TraitId::Vengeful,
        name: "vengeful",
        kind: TraitKind::Personality,
        line: "never lets a wrong go",
        icon: Art::Skull,
        grudge_num: 2,
        ..NEUTRAL
    },
    TraitDef {
        id: TraitId::Pious,
        name: "pious",
        kind: TraitKind::Personality,
        line: "counts sins, not their size",
        icon: Art::Eye,
        ..NEUTRAL
    },
    TraitDef {
        id: TraitId::Pragmatic,
        name: "pragmatic",
        kind: TraitKind::Personality,
        line: "prefers a known quantity to a stranger",
        icon: Art::Coin,
        ..NEUTRAL
    },
    TraitDef {
        id: TraitId::Cold,
        name: "cold",
        kind: TraitKind::Personality,
        line: "feelings weigh little, either way",
        icon: Art::Skull,
        bond_den: 2,
        grudge_den: 2,
        ..NEUTRAL
    },
    TraitDef {
        id: TraitId::Upright,
        name: "upright",
        kind: TraitKind::Personality,
        line: "won't work with criminals",
        icon: Art::Eye,
        ..NEUTRAL
    },
    // ── motivator — PLACEHOLDER CONTENT (GDD §10's trait-content pass) ────
    // Three rows, authored to prove the format: a motivator moves the upkeep
    // multiplier and the pressure, and nothing else. What ninjo's cast
    // actually wants is written when the content pass runs.
    TraitDef {
        id: TraitId::Provider,
        name: "provider",
        kind: TraitKind::Motivator,
        line: "somebody else eats before they do",
        icon: Art::Heart,
        upkeep_num: 3,
        upkeep_den: 2,
        pressure: 2,
        ..NEUTRAL
    },
    TraitDef {
        id: TraitId::Ambitious,
        name: "ambitious",
        kind: TraitKind::Motivator,
        line: "means to be somebody, and spends like it",
        icon: Art::Coin,
        upkeep_num: 2,
        upkeep_den: 1,
        pressure: 3,
        ..NEUTRAL
    },
    TraitDef {
        id: TraitId::Homesick,
        name: "homesick",
        kind: TraitKind::Motivator,
        line: "saving for a road out of here",
        icon: Art::Flame,
        upkeep_num: 1,
        upkeep_den: 2,
        pressure: 1,
        ..NEUTRAL
    },
    // ── aptitude — PLACEHOLDER CONTENT (GDD §10's trait-content pass) ─────
    // Three rows: an aptitude moves the competence value a task reads, and
    // nothing else.
    TraitDef {
        id: TraitId::Strong,
        name: "strong",
        kind: TraitKind::Aptitude,
        line: "carries what two of them would",
        icon: Art::Skull,
        aptitude: 2,
        ..NEUTRAL
    },
    TraitDef {
        id: TraitId::Deft,
        name: "deft",
        kind: TraitKind::Aptitude,
        line: "good hands, and quick with them",
        icon: Art::Eye,
        aptitude: 2,
        ..NEUTRAL
    },
    TraitDef {
        id: TraitId::Learned,
        name: "learned",
        kind: TraitKind::Aptitude,
        line: "reads, counts, and remembers",
        icon: Art::Flame,
        aptitude: 3,
        ..NEUTRAL
    },
];

impl TraitId {
    /// This trait's row of the vocabulary.
    pub fn def(self) -> &'static TraitDef {
        // A linear walk; `vocabulary()` asserts every id has exactly one row,
        // so the fallback is unreachable in a valid table.
        TRAITS
            .iter()
            .find(|def| def.id == self)
            .unwrap_or(&TRAITS[0])
    }

    /// The chip's text.
    pub fn name(self) -> &'static str {
        self.def().name
    }

    /// Which family it belongs to.
    pub fn kind(self) -> TraitKind {
        self.def().kind
    }

    /// The chip's interim icon role.
    pub fn icon(self) -> Art {
        self.def().icon
    }
}

/// Which side of the ledger a mark sits on (giri v2 semantics, unchanged).
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
///
/// A person-fact (GDD §4.3). Marks do not decay; the erasure rules are
/// post-MVP design, and title-marks arrive with aspirations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkId {
    /// Took an extra share, quietly.
    Skimmer,
    /// Walked mid-quest.
    Deserter,
    /// The work suffered; someone got hurt.
    Saboteur,
    /// The summit: killed a partymate.
    ComradeKiller,
    /// N clean jobs.
    Reliable,
    /// Held when holding cost something.
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
/// **Reactions open doors as well as close them**: a positive delta is an
/// attraction, and `(Pragmatic, Skimmer, +2)` against a base of `-mark_dark`
/// is the entry that makes a known skimmer *preferable* to a stranger.
#[derive(Clone, Copy, Debug)]
pub struct Reaction {
    /// Whose trait is reacting. A personality — `vocabulary` asserts it.
    pub trait_id: TraitId,
    /// To whose mark.
    pub mark: MarkId,
    /// Added to the tone's base reaction.
    pub delta: i64,
}

/// The trait x mark table — tuning content, one row per cell that is not the
/// tone's base. Ported from giri v2 unchanged.
pub const REACTIONS: &[Reaction] = &[
    // The upright refuse the dark-marked.
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
pub fn reaction_delta(trait_id: TraitId, mark: MarkId) -> i64 {
    REACTIONS
        .iter()
        .filter(|cell| cell.trait_id == trait_id && cell.mark == mark)
        .map(|cell| cell.delta)
        .sum()
}

/// The trait-filtered reaction to one mark: the tone's base, plus every table
/// cell the looker's traits hold for it (giri's `willing.rs::reaction_to`,
/// ported).
///
/// Walked over **every** trait the looker carries, whatever its kind: a
/// motivator or an aptitude simply has no cell in the table, so it adds
/// nothing. That is the no-branch discipline in one line — nobody filters the
/// list by kind first.
pub fn reaction_to(tuning: &Tuning, traits: &[TraitId], mark: MarkId) -> i64 {
    let base = match mark.tone() {
        MarkTone::Dark => -tuning.mark_dark,
        MarkTone::Light => tuning.mark_light,
        MarkTone::Ambiguous => 0,
    };
    base + traits
        .iter()
        .map(|trait_id| reaction_delta(*trait_id, mark))
        .sum::<i64>()
}

/// Regard as this character's traits weigh it: bonds through the bond
/// multipliers, grudges through the grudge multipliers, exactly (giri's
/// `willing.rs::weighted_regard`, ported).
///
/// The *felt* value of an edge, as against the stored one. The stores keep the
/// raw integer (`stores.rs`); this is what the scorer will read and what a
/// character sheet shows, and both go through the lens.
pub fn weighted_regard(raw: i64, traits: &[TraitId]) -> i64 {
    if raw == 0 {
        return 0;
    }
    let (mut num, mut den) = (1i64, 1i64);
    for def in traits.iter().map(|id| id.def()) {
        if raw > 0 {
            num *= def.bond_num;
            den *= def.bond_den;
        } else {
            num *= def.grudge_num;
            den *= def.grudge_den;
        }
    }
    raw * num / den
}

/// What this character's upkeep costs, as their motivators multiply it
/// (GDD §5: needs are trait-modulated).
///
/// Walked over every trait for the same reason [`reaction_to`] is: a
/// personality's upkeep multiplier is 1/1 by the neutrality rule, so it drops
/// out without anybody testing its kind.
pub fn upkeep_of(base: i64, traits: &[TraitId]) -> i64 {
    let (mut num, mut den) = (base, 1i64);
    for def in traits.iter().map(|id| id.def()) {
        num *= def.upkeep_num;
        den *= def.upkeep_den;
    }
    num / den
}

/// What this character brings to a task: the sum of their aptitude values
/// (GDD §5: resolution reads aptitude-kind traits).
pub fn competence_of(traits: &[TraitId]) -> i64 {
    traits.iter().map(|id| id.def().aptitude).sum()
}

/// The vocabulary's own validation — the data-shape claims prose cannot hold.
///
/// The kind discipline is enforced here rather than trusted: every row holds
/// [`NEUTRAL`]'s value in every field its kind does not own, which is what
/// lets every consumer apply every field without asking what kind the row is.
pub fn vocabulary(checks: &mut Checks) {
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
                 for a note band (UI.md §14)",
                def.id,
                def.line,
                def.line.chars().count()
            ),
        );
        checks.require(
            def.bond_den > 0 && def.grudge_den > 0 && def.upkeep_den > 0,
            "a trait multiplies by a rational with no denominator",
            format!(
                "{:?} has bond {}/{}, grudge {}/{}, upkeep {}/{}",
                def.id,
                def.bond_num,
                def.bond_den,
                def.grudge_num,
                def.grudge_den,
                def.upkeep_num,
                def.upkeep_den
            ),
        );
        // The lookup resolves each id to its own row. `TraitId::def` is a
        // linear walk with a fallback, so an id with no row would quietly
        // answer with the first trait's numbers - which is a character
        // silently becoming greedy.
        checks.require(
            def.id.name() == def.name && def.id.kind() == def.kind,
            "a trait id does not resolve to its own row",
            format!(
                "{:?} looks up as {:?} ({}) and its row reads {:?} ({})",
                def.id,
                def.id.name(),
                def.id.kind().name(),
                def.name,
                def.kind.name()
            ),
        );

        // The interim icon: a chip's picture is one of the existing library's
        // category icons (UI.md §13), and a chip is a square 16 units across.
        // Square and a whole divisor of 16, or the chip stretches it or draws
        // it at a fraction — which the engine samples nearest, wobble and all.
        let texels = def.id.icon().texels();
        checks.require(
            texels.width == texels.height && texels.width > 0 && 16 % texels.width == 0,
            "a trait's chip icon is not a square whole-scale picture",
            format!(
                "{:?} draws {:?}, which is {}x{} texels; a chip is 16 units square and the \
                 engine samples nearest",
                def.id, def.icon, texels.width, texels.height
            ),
        );

        // The kind discipline: a row moves only the fields its kind owns.
        let owned = match def.kind {
            TraitKind::Personality => "bond/grudge multipliers and pot_affinity",
            TraitKind::Motivator => "upkeep multiplier and pressure",
            TraitKind::Aptitude => "aptitude",
        };
        let personality_neutral = def.bond_num == NEUTRAL.bond_num
            && def.bond_den == NEUTRAL.bond_den
            && def.grudge_num == NEUTRAL.grudge_num
            && def.grudge_den == NEUTRAL.grudge_den
            && def.pot_affinity == NEUTRAL.pot_affinity;
        let motivator_neutral = def.upkeep_num == NEUTRAL.upkeep_num
            && def.upkeep_den == NEUTRAL.upkeep_den
            && def.pressure == NEUTRAL.pressure;
        let aptitude_neutral = def.aptitude == NEUTRAL.aptitude;
        let held = match def.kind {
            TraitKind::Personality => motivator_neutral && aptitude_neutral,
            TraitKind::Motivator => personality_neutral && aptitude_neutral,
            TraitKind::Aptitude => personality_neutral && motivator_neutral,
        };
        checks.require(
            held,
            "a trait moves a field its kind does not own",
            format!(
                "{:?} is a {} and so owns only the {owned}; it reads bond {}/{} grudge {}/{} \
                 pot {} upkeep {}/{} pressure {} aptitude {}. A kind gates which fields \
                 apply, and every consumer applies them all - a stray value would act \
                 without anybody choosing to apply it",
                def.id,
                def.kind.name(),
                def.bond_num,
                def.bond_den,
                def.grudge_num,
                def.grudge_den,
                def.pot_affinity,
                def.upkeep_num,
                def.upkeep_den,
                def.pressure,
                def.aptitude
            ),
        );
    }
    // Every kind has at least one row: a family with no members is a format
    // nothing proves.
    for kind in TraitKind::ALL.iter().copied() {
        let rows = TRAITS.iter().filter(|def| def.kind == kind).count();
        checks.require(
            rows > 0,
            "a trait kind has no rows in the vocabulary",
            format!(
                "{} has {rows} rows; GDD §6 names three kinds and each needs at least one \
                 row for the format to be proved",
                kind.name()
            ),
        );
    }
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
        checks.require(
            cell.trait_id.kind() == TraitKind::Personality,
            "the trait x mark table has a cell for a trait that is not a personality",
            format!(
                "({:?}, {:?}) reacts, and {:?} is a {}; a reaction is how a *personality* \
                 reads a public fact",
                cell.trait_id,
                cell.mark,
                cell.trait_id,
                cell.trait_id.kind().name()
            ),
        );
    }
}

/// The trait arithmetic, at a stated constants set — **every expectation a
/// shipped literal**, never derived from `tuning`.
///
/// A check that recomputes its expectation from the constant under test cannot
/// see that constant move, and the mutation round runs this battery at moved
/// constants to see exactly that (`mutation.rs`). It is also the proof of the
/// no-branch claim: the mixed-kind rows below are the same call with a
/// motivator and an aptitude in the list, and they change nothing.
pub fn arithmetic(checks: &mut Checks, tuning: &Tuning) {
    let mut judge = |what: &'static str, got: i64, want: i64, why: &str| {
        checks.require(
            got == want,
            what,
            format!("{why}: the arithmetic answers {got} and the shipped set says {want}"),
        );
    };

    // Reactions: the tone's base, then the table's cells on top of it.
    judge(
        "a stranger's reaction to a dark mark is not the shipped cost",
        reaction_to(tuning, &[], MarkId::Skimmer),
        -1,
        "reaction_to([], skimmer) is -mark_dark",
    );
    judge(
        "a stranger's reaction to a light mark is not the shipped gain",
        reaction_to(tuning, &[], MarkId::Reliable),
        1,
        "reaction_to([], reliable) is +mark_light",
    );
    judge(
        "an ambiguous mark is not neutral to somebody with no opinion of it",
        reaction_to(tuning, &[], MarkId::Survivor),
        0,
        "reaction_to([], survivor) has no tone and no cell",
    );
    judge(
        "the upright do not refuse a thief by the table's cell",
        reaction_to(tuning, &[TraitId::Upright], MarkId::Skimmer),
        -3,
        "-mark_dark plus the (upright, skimmer) cell of -2",
    );
    judge(
        "a known skimmer is not preferable to a pragmatic stranger",
        reaction_to(tuning, &[TraitId::Pragmatic], MarkId::Skimmer),
        1,
        "-mark_dark plus the (pragmatic, skimmer) cell of +2 - the entry that \
         makes reactions open doors as well as close them",
    );
    judge(
        "a motivator or an aptitude changed a mark reaction",
        reaction_to(tuning, &[TraitId::Deft, TraitId::Provider], MarkId::Skimmer),
        -1,
        "a kind gates data, not behaviour: neither row has a cell, so the sum \
         is the bare tone",
    );

    // Regard, as a personality weighs it.
    for (raw, carried, want, why) in [
        (4, &[TraitId::Loyal][..], 8, "the loyal weigh a bond double"),
        (-4, &[TraitId::Loyal][..], -4, "and weigh a grudge plainly"),
        (
            -4,
            &[TraitId::Vengeful][..],
            -8,
            "the vengeful weigh a grudge double",
        ),
        (4, &[TraitId::Cold][..], 2, "the cold halve warmth"),
        (-4, &[TraitId::Cold][..], -2, "and halve ill will"),
        (
            0,
            &[TraitId::Loyal][..],
            0,
            "nothing multiplies to something",
        ),
        (
            4,
            &[TraitId::Loyal, TraitId::Strong][..],
            8,
            "an aptitude does not weigh regard",
        ),
    ] {
        judge(
            "regard is not weighed the way the carrier's traits say",
            weighted_regard(raw, carried),
            want,
            why,
        );
    }

    // Upkeep, as a motivator multiplies it.
    for (carried, want, why) in [
        (&[][..], 4, "nobody's upkeep is the base"),
        (&[TraitId::Provider][..], 6, "a provider carries half again"),
        (&[TraitId::Ambitious][..], 8, "the ambitious carry double"),
        (&[TraitId::Homesick][..], 2, "the homesick take half"),
        (
            &[TraitId::Greedy][..],
            4,
            "a personality does not move upkeep",
        ),
    ] {
        judge(
            "upkeep is not multiplied the way the carrier's motivators say",
            upkeep_of(4, carried),
            want,
            why,
        );
    }

    // Competence, as aptitudes sum it.
    for (carried, want, why) in [
        (&[][..], 0, "nobody brings nothing"),
        (
            &[TraitId::Strong, TraitId::Deft][..],
            4,
            "two aptitudes sum",
        ),
        (
            &[TraitId::Greedy, TraitId::Provider][..],
            0,
            "a personality and a motivator bring no competence",
        ),
    ] {
        judge(
            "competence is not the sum of the carrier's aptitudes",
            competence_of(carried),
            want,
            why,
        );
    }
}
