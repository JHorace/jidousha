//! The puzzle chain, as data (DESIGN.md §10): beats 1-4, the owner's tutorial,
//! re-authored minimally onto the v2 machinery.
//!
//! **Same dilemmas, new causes.** The stories are v1's — Steve; Bob kills
//! Steve; Tim refuses / Alex joins; Tim's price is met — and what produces
//! them changed underneath: Bob's public name is a *comrade-killer mark*
//! rather than a public number, Tim's refusal is his *upright* trait meeting
//! that mark through the reaction table, and the pot pulls the greedy through
//! their trait. The assertions migrated with the machinery (marks, verdicts
//! and reasons where the public scalar's numbers were), and every change is listed
//! in the PR that landed this file.
//!
//! Beats 5+ are added here and nowhere else; no code names a beat number.

use crate::beats::{BeatSpec, CharSpec, Dungeon, Expect, QuestIcon, Requirement};
use crate::traits::{MarkId, TraitId};
use crate::willing::Verdict;

/// The chain. Win is completing it.
pub const CHAIN: &[BeatSpec] = &[
    BeatSpec {
        title: "Steve",
        dilemma: "One name on the roster, one job, and nothing in the way of it.",
        teaches: "a sheet, a job, and what a share does to need",
        roster: &[CharSpec {
            name: "Steve",
            desperation: 1,
            source: "rent is due",
            wealth: 0,
            traits: &[TraitId::Greedy],
            marks: &[],
            clean_jobs: 0,
        }],
        edges: &[],
        dungeons: &[Dungeon {
            name: "the sewer job",
            blurb: "A starter job. One warm body will do.",
            icon: QuestIcon::Cave,
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
            // 1 need + 4 promised share through the greedy trait. v1's total
            // was 1; the pot now pulls, and it pulls harder than the hunger.
            Expect::WillingnessIs {
                who: "Steve",
                party: &["Steve"],
                total: 5,
            },
            Expect::VerdictIs {
                who: "Steve",
                party: &["Steve"],
                verdict: Verdict::Joins,
            },
            Expect::TopReason {
                who: "Steve",
                party: &["Steve"],
                fragment: "the money is good",
            },
            Expect::Survives { who: "Steve" },
            Expect::Wealth {
                who: "Steve",
                value: 4,
            },
            // One clean job in the count, and one is not yet a reputation.
            Expect::CleanJobs {
                who: "Steve",
                value: 1,
            },
            Expect::LacksMark {
                who: "Steve",
                mark: MarkId::Reliable,
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
                source: "old debts",
                wealth: 0,
                traits: &[TraitId::Greedy],
                marks: &[],
                clean_jobs: 0,
            },
            CharSpec {
                name: "Steve",
                desperation: 1,
                source: "rent is due",
                wealth: 0,
                traits: &[TraitId::Greedy],
                marks: &[],
                clean_jobs: 0,
            },
        ],
        edges: &[],
        dungeons: &[Dungeon {
            name: "the deep vault",
            blurb: "Two go down. What comes back up is their business.",
            icon: QuestIcon::Vault,
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
            // 8 need + 2 promised share; the need is the louder cause.
            Expect::WillingnessIs {
                who: "Bob",
                party: &["Bob", "Steve"],
                total: 10,
            },
            Expect::TopReason {
                who: "Bob",
                party: &["Bob", "Steve"],
                fragment: "needs the money",
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
            // The murder writes the mark that used to be a public number.
            Expect::HasMark {
                who: "Bob",
                mark: MarkId::ComradeKiller,
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
            Expect::ReportSays {
                fragment: "marked comrade-killer",
            },
        ],
    },
    BeatSpec {
        title: "Tim refuses, Alex joins",
        dilemma: "Bob wears what he did, and the road needs two. Tim will not \
                  stand next to a dark mark; Alex needs the winter paid for.",
        teaches: "marks are public and traits decide who minds them",
        roster: &[
            CharSpec {
                name: "Bob",
                desperation: 4,
                source: "old debts",
                wealth: 0,
                traits: &[TraitId::Greedy],
                marks: &[MarkId::ComradeKiller],
                clean_jobs: 0,
            },
            CharSpec {
                name: "Tim",
                desperation: 1,
                source: "hungry kin",
                wealth: 0,
                traits: &[TraitId::Upright],
                marks: &[],
                clean_jobs: 0,
            },
            CharSpec {
                name: "Alex",
                desperation: 3,
                source: "thin winter",
                wealth: 0,
                traits: &[TraitId::Pragmatic],
                marks: &[MarkId::Skimmer],
                clean_jobs: 0,
            },
        ],
        edges: &[],
        dungeons: &[Dungeon {
            name: "the long road",
            blurb: "Two sets of hands, a long walk, and a pot that splits.",
            icon: QuestIcon::Tower,
            headcount: 2,
            pot: 8,
            cut: 2,
            requires: Requirement::AnyParty,
        }],
        send: &["Bob", "Alex"],
        expect: &[
            // 1 need - 3 reaction (a dark mark at 1, and upright minds it 2
            // more). v1's -2, produced by the trait x mark table.
            Expect::Refuses {
                who: "Tim",
                party: &["Bob", "Tim"],
            },
            Expect::WillingnessIs {
                who: "Tim",
                party: &["Bob", "Tim"],
                total: -2,
            },
            Expect::TopReason {
                who: "Tim",
                party: &["Bob", "Tim"],
                fragment: "won't work with a comrade-killer",
            },
            // The upright mind a skimmer exactly as much: Alex's mark gates
            // Tim too.
            Expect::Refuses {
                who: "Tim",
                party: &["Alex", "Tim"],
            },
            // 3 need - 1 for the mark, and no trait of Alex's minds it more:
            // he joins because the need outweighs the company.
            Expect::Joins {
                who: "Alex",
                party: &["Bob", "Alex"],
            },
            Expect::WillingnessIs {
                who: "Alex",
                party: &["Bob", "Alex"],
                total: 2,
            },
            Expect::VerdictIs {
                who: "Alex",
                party: &["Bob", "Alex"],
                verdict: Verdict::Joins,
            },
            Expect::TopReason {
                who: "Alex",
                party: &["Bob", "Alex"],
                fragment: "needs the money",
            },
            Expect::Survives { who: "Bob" },
            Expect::Survives { who: "Alex" },
            Expect::Wealth {
                who: "Bob",
                value: 3,
            },
            // A clean job bonds the pair, both ways, and counts for both.
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
            Expect::CleanJobs {
                who: "Bob",
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
        dilemma: "The same road, the same mark, and a Tim who sat out a round. \
                  Everyone has a price; his is a desperation of three.",
        teaches: "refusal is temporary - the roster decays toward willingness",
        roster: &[
            CharSpec {
                name: "Bob",
                desperation: 4,
                source: "old debts",
                wealth: 0,
                traits: &[TraitId::Greedy],
                marks: &[MarkId::ComradeKiller],
                clean_jobs: 1,
            },
            CharSpec {
                name: "Tim",
                desperation: 3,
                source: "hungry kin",
                wealth: 0,
                traits: &[TraitId::Upright],
                marks: &[],
                clean_jobs: 0,
            },
        ],
        edges: &[],
        dungeons: &[Dungeon {
            name: "the second road",
            blurb: "The same road again, and a hungrier man to walk it.",
            icon: QuestIcon::Crypt,
            headcount: 2,
            pot: 8,
            cut: 2,
            requires: Requirement::AnyParty,
        }],
        send: &["Bob", "Tim"],
        expect: &[
            // 3 need - 3 reaction = 0, and 0 >= 0 joins. The boundary is the
            // beat — and under the reluctant band it now has a name.
            Expect::Joins {
                who: "Tim",
                party: &["Bob", "Tim"],
            },
            Expect::WillingnessIs {
                who: "Tim",
                party: &["Bob", "Tim"],
                total: 0,
            },
            Expect::VerdictIs {
                who: "Tim",
                party: &["Bob", "Tim"],
                verdict: Verdict::Reluctant,
            },
            Expect::TopReason {
                who: "Tim",
                party: &["Bob", "Tim"],
                fragment: "needs the money",
            },
            // And the other way round: Tim wears nothing, so nothing gates
            // Bob — the pot and his own need are the whole of his sum. Without
            // this, the reaction table could fire on empty sheets and every
            // beat would still pass.
            Expect::WillingnessIs {
                who: "Bob",
                party: &["Bob", "Tim"],
                total: 7,
            },
            Expect::Survives { who: "Tim" },
            Expect::Survives { who: "Bob" },
            Expect::Wealth {
                who: "Tim",
                value: 3,
            },
            // The light side writes too: Bob's second clean job is a
            // reputation for coming back clean, on the same sheet as the
            // murder.
            Expect::HasMark {
                who: "Bob",
                mark: MarkId::Reliable,
            },
            Expect::LacksMark {
                who: "Tim",
                mark: MarkId::Reliable,
            },
            Expect::CleanJobs {
                who: "Bob",
                value: 2,
            },
            Expect::CleanJobs {
                who: "Tim",
                value: 1,
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
            Expect::ReportSays {
                fragment: "marked reliable",
            },
        ],
    },
];
