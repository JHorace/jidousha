//! The puzzle chain, as data (DESIGN.md §6).
//!
//! A beat is `(initial roster state, dungeon(s), the intended dilemma stated in
//! a sentence, expected-outcome assertions)` and **nothing else** — no code, no
//! system, no branch anywhere that names a beat number. Beats 5+ are authored
//! by adding a `BeatSpec` to `CHAIN` below; that is the whole of what this
//! separation buys, and it is the reason the composition predicates in
//! `Requirement` exist before a beat uses one.
//!
//! The fourth field is the verify scenario: `verify.rs` plays each beat through
//! `InputScript` and evaluates its `Expect` list against the world. So the
//! numbers below are simultaneously the tutorial and the tuning constants'
//! regression harness — a constant that stops producing these outcomes fails
//! the run.

use crate::model::{Member, Social};

/// A character's authored starting state.
#[derive(Clone, Copy, Debug)]
pub struct CharSpec {
    /// The name. ASCII, because the engine's font is (DESIGN §7).
    pub name: &'static str,
    /// Need at the start of the beat.
    pub desperation: i32,
    /// Public reputation at the start of the beat.
    pub infamy: i32,
    /// Accumulated profit at the start of the beat.
    pub wealth: i32,
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
/// The growth axis (DESIGN §5): predicates come from the social vocabulary
/// rather than from combat stats. The two beyond `AnyParty` are unused by the
/// tutorial beats and exercised directly in `verify.rs` — a contract a played
/// beat never reaches is still a contract, and asking it directly is cheaper
/// than authoring a beat to reach it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Requirement {
    /// Headcount only.
    AnyParty,
    /// At least one member whose infamy reaches `at_least` — the underworld
    /// track's shape (DESIGN §4, OPEN): a job that needs a known face.
    AtLeastOneInfamous {
        /// The infamy that counts as a known face.
        at_least: i32,
    },
    /// Nobody whose infamy reaches `at_least` — a job that cannot be seen with
    /// a known face.
    NoInfamous {
        /// The infamy that counts as a known face.
        at_least: i32,
    },
}

impl Requirement {
    /// Whether `party` satisfies this predicate.
    pub fn met(self, social: &Social, party: &[jidousha::prelude::Entity]) -> bool {
        match self {
            Requirement::AnyParty => true,
            Requirement::AtLeastOneInfamous { at_least } => party
                .iter()
                .any(|member| social.infamy(*member) >= at_least),
            Requirement::NoInfamous { at_least } => {
                party.iter().all(|member| social.infamy(*member) < at_least)
            }
        }
    }

    /// The predicate as the dungeon panel states it.
    pub fn describe(self) -> String {
        match self {
            Requirement::AnyParty => "anyone who will come".to_owned(),
            Requirement::AtLeastOneInfamous { at_least } => {
                format!("at least one member of infamy {at_least}+")
            }
            Requirement::NoInfamous { at_least } => {
                format!("nobody of infamy {at_least}+")
            }
        }
    }
}

/// A job: what it asks for, what it pays, and what the player keeps.
///
/// Everything visible before assembly, like everything else (DESIGN §5).
#[derive(Clone, Copy, Debug)]
pub struct Dungeon {
    /// What it is called.
    pub name: &'static str,
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
/// `Refuses` and `Joins` are claims about the *assembly* moment — the social
/// state the beat was authored with. Everything else is a claim about the world
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
    /// `who`'s willingness for that party is exactly `total`.
    ///
    /// The sharper form of the two above, and the one a beat wants when the
    /// answer sits on the boundary: "Tim joins" passes at +7 as happily as at
    /// the 0 the beat is about.
    WillingnessIs {
        /// The character asked.
        who: &'static str,
        /// The party, by name.
        party: &'static [&'static str],
        /// The exact sum.
        total: i32,
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
    /// `who`'s infamy ends the beat at `value`.
    Infamy {
        /// The character.
        who: &'static str,
        /// The exact value.
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
    /// The report is the story surface (DESIGN §7) and its arithmetic is what a
    /// player learns the rules from, so the narration is asserted rather than
    /// assumed: a beat that produces the right world state and describes it
    /// wrongly has broken the half of the game a player reads.
    ReportSays {
        /// The fragment.
        fragment: &'static str,
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
    /// Part of the fourth field (DESIGN §6: "the verify scenario"), not of the
    /// rules: nothing in the game reads it, and a player is free to send
    /// anything the gate allows.
    pub send: &'static [&'static str],
    /// What playing it correctly produces.
    pub expect: &'static [Expect],
}

impl BeatSpec {
    /// Where `name` sits in this beat's roster, if it is in it.
    pub fn index_of(&self, name: &str) -> Option<usize> {
        self.roster.iter().position(|spec| spec.name == name)
    }
}

/// The chain. Win is completing it.
///
/// Beats 1-4 are the owner's tutorial, verbatim (DESIGN §6): Steve; Bob kills
/// Steve; Tim refuses / Alex joins; Tim's price is met. Beats 5+ are the next
/// session's, and are added here and nowhere else.
pub const CHAIN: &[BeatSpec] = &[
    BeatSpec {
        title: "Steve",
        dilemma: "One name on the roster, one job, and nothing in the way of it.",
        teaches: "a sheet, a job, and what a share does to need",
        roster: &[CharSpec {
            name: "Steve",
            desperation: 1,
            infamy: 0,
            wealth: 0,
        }],
        edges: &[],
        dungeons: &[Dungeon {
            name: "the sewer job",
            headcount: 1,
            pot: 6,
            cut: 2,
            requires: Requirement::AnyParty,
        }],
        send: &["Steve"],
        expect: &[
            Expect::Joins {
                who: "Steve",
                party: &["Steve"],
            },
            Expect::WillingnessIs {
                who: "Steve",
                party: &["Steve"],
                total: 1,
            },
            Expect::Survives { who: "Steve" },
            Expect::Wealth {
                who: "Steve",
                value: 4,
            },
            // 1 - 3, floored at 0: the share paid off more need than he had.
            Expect::Desperation {
                who: "Steve",
                value: 0,
            },
            Expect::ReportSays {
                fragment: "Steve takes 4",
            },
        ],
    },
    BeatSpec {
        title: "Bob kills Steve",
        dilemma: "The vault needs two. Bob is desperate enough that one of them \
                  comes back, and you can read that off the sheets before you send them.",
        teaches: "the pot is the motive: a fixed pot split among survivors",
        roster: &[
            CharSpec {
                name: "Bob",
                desperation: 8,
                infamy: 0,
                wealth: 0,
            },
            CharSpec {
                name: "Steve",
                desperation: 1,
                infamy: 0,
                wealth: 0,
            },
        ],
        edges: &[],
        dungeons: &[Dungeon {
            name: "the deep vault",
            headcount: 2,
            pot: 6,
            cut: 2,
            requires: Requirement::AnyParty,
        }],
        send: &["Bob", "Steve"],
        expect: &[
            Expect::Joins {
                who: "Bob",
                party: &["Bob", "Steve"],
            },
            Expect::Joins {
                who: "Steve",
                party: &["Bob", "Steve"],
            },
            Expect::Killed {
                victim: "Steve",
                by: "Bob",
            },
            Expect::Survives { who: "Bob" },
            Expect::Wealth {
                who: "Bob",
                value: 4,
            },
            Expect::Infamy {
                who: "Bob",
                value: 3,
            },
            // 8 - 3: a full share, and he is still the most desperate name here.
            Expect::Desperation {
                who: "Bob",
                value: 5,
            },
            // Steve is dead, so no drift touches him.
            Expect::Desperation {
                who: "Steve",
                value: 1,
            },
            Expect::ReportSays {
                fragment: "Bob killed Steve - desperation 8 >= 6, share 2->4, regard 0 < 2",
            },
        ],
    },
    BeatSpec {
        title: "Tim refuses, Alex joins",
        dilemma: "Bob is known now, and the road needs two. Tim will not stand \
                  next to a name worse than his own; Alex has one of his own.",
        teaches: "infamy is a gap, not a level: it gates whoever is cleaner",
        roster: &[
            CharSpec {
                name: "Bob",
                desperation: 4,
                infamy: 3,
                wealth: 0,
            },
            CharSpec {
                name: "Tim",
                desperation: 1,
                infamy: 0,
                wealth: 0,
            },
            CharSpec {
                name: "Alex",
                desperation: 2,
                infamy: 3,
                wealth: 0,
            },
        ],
        edges: &[],
        dungeons: &[Dungeon {
            name: "the long road",
            headcount: 2,
            pot: 8,
            cut: 2,
            requires: Requirement::AnyParty,
        }],
        send: &["Bob", "Alex"],
        expect: &[
            // 1 - 1*(3-0) = -2, against either infamous name.
            Expect::Refuses {
                who: "Tim",
                party: &["Bob", "Tim"],
            },
            Expect::WillingnessIs {
                who: "Tim",
                party: &["Bob", "Tim"],
                total: -2,
            },
            Expect::Refuses {
                who: "Tim",
                party: &["Alex", "Tim"],
            },
            // 2 - 1*max(0, 3-3) = 2: no gap, so no objection.
            Expect::Joins {
                who: "Alex",
                party: &["Bob", "Alex"],
            },
            Expect::WillingnessIs {
                who: "Alex",
                party: &["Bob", "Alex"],
                total: 2,
            },
            Expect::Survives { who: "Bob" },
            Expect::Survives { who: "Alex" },
            Expect::Wealth {
                who: "Bob",
                value: 3,
            },
            // A clean job bonds the pair, both ways.
            Expect::Regard {
                from: "Bob",
                to: "Alex",
                value: 1,
            },
            Expect::Regard {
                from: "Alex",
                to: "Bob",
                value: 1,
            },
            Expect::Desperation {
                who: "Bob",
                value: 1,
            },
            Expect::Desperation {
                who: "Alex",
                value: 0,
            },
            // Tim sat the round out, which is what raises his price.
            Expect::Desperation {
                who: "Tim",
                value: 3,
            },
            Expect::ReportSays {
                fragment: "Bob and Alex bond",
            },
        ],
    },
    BeatSpec {
        title: "Tim's price is met",
        dilemma: "The same road, the same gap, and a Tim who sat out a round. \
                  Everyone has a price; his is a desperation of three.",
        teaches: "refusal is temporary - the roster decays toward willingness",
        roster: &[
            CharSpec {
                name: "Bob",
                desperation: 4,
                infamy: 3,
                wealth: 0,
            },
            CharSpec {
                name: "Tim",
                desperation: 3,
                infamy: 0,
                wealth: 0,
            },
        ],
        edges: &[],
        dungeons: &[Dungeon {
            name: "the second road",
            headcount: 2,
            pot: 8,
            cut: 2,
            requires: Requirement::AnyParty,
        }],
        send: &["Bob", "Tim"],
        expect: &[
            // 3 - 1*(3-0) = 0, and 0 >= 0 joins. The boundary is the beat.
            Expect::Joins {
                who: "Tim",
                party: &["Bob", "Tim"],
            },
            Expect::WillingnessIs {
                who: "Tim",
                party: &["Bob", "Tim"],
                total: 0,
            },
            Expect::Survives { who: "Tim" },
            Expect::Survives { who: "Bob" },
            Expect::Wealth {
                who: "Tim",
                value: 3,
            },
            Expect::Regard {
                from: "Bob",
                to: "Tim",
                value: 1,
            },
            Expect::Regard {
                from: "Tim",
                to: "Bob",
                value: 1,
            },
            Expect::Desperation {
                who: "Tim",
                value: 0,
            },
            Expect::Desperation {
                who: "Bob",
                value: 1,
            },
        ],
    },
];

/// The initial of a name, for the portrait quad.
///
/// giri v1 has no assets at all: a "portrait" is a tinted quad with a letter on
/// it (DESIGN §7).
pub fn initial(name: &str) -> char {
    name.chars().next().unwrap_or('?')
}

/// The sheet line every roster card carries, as one string.
///
/// A function rather than a `format!` inside the draw system, so a check can
/// ask the game for the exact text it draws.
pub fn stat_line(member: &Member) -> String {
    format!(
        "DES {}  INF {}  WLT {}",
        member.desperation, member.infamy, member.wealth
    )
}
