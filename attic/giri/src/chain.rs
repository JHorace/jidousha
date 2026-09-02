//! The puzzle chain, as data (DESIGN.md §10): beats 1-4, the owner's tutorial,
//! re-authored minimally onto the v2 machinery.
//!
//! **Same dilemmas, new causes.** The stories are v1's — Steve; Bob kills
//! Steve; Tim refuses / Alex joins; Tim's price is met — and what produces
//! them changed underneath: Bob's public name is a *comrade-killer mark*
//! rather than a public number, Tim's refusal is his *upright* trait meeting
//! that mark through the reaction table, and the pot pulls the greedy through
//! their trait.
//!
//! **P2: every beat carries a fixed seed and two assertion lists.** `expect`
//! is v1's list, judged under the `deterministic` variant it asserts; `ladder`
//! is the same beat at its authored seed under the shipped rule set — beat 2's
//! murder still happens (the seed is data, picked so the tutorial's story
//! survives the dice), beat 3 teaches the ladder's common rung instead of a
//! quiet walk, and beat 4's powder-keg chip warns about a murder that, this
//! time, does not come. The copy addresses somebody who has never seen the
//! game (the tutorial is the players' docs/api — agreement 10).
//!
//! Beats 5+ are added here and nowhere else; no code names a beat number.

use crate::beats::{BeatSpec, CharSpec, Dungeon, Expect, QuestIcon, Requirement};
use crate::pressure::Band;
use crate::traits::{MarkId, TraitId};
use crate::willing::Verdict;

/// The chain. Win is completing it.
pub const CHAIN: &[BeatSpec] = &[
    BeatSpec {
        title: "Steve",
        dilemma: "Steve wants work and the sewer wants a body. Click the job card to \
                  take the job, click Steve's card to add him, then press SEND PARTY.",
        teaches: "the flame on a sheet is need, and a paid job feeds it",
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
            blurb: "One person, easy coin. Everybody comes home from this one.",
            icon: QuestIcon::Cave,
            headcount: 1,
            pot: 6,
            cut: 2,
            requires: Requirement::AnyParty,
        }],
        send: &["Steve"],
        seed: 1,
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
        // A party of one has nobody to betray: the ladder rolls nothing, and
        // the beat resolves exactly as v1 did — at any seed, which is why this
        // one is not delicate about its number.
        ladder: &[
            // Eager (margin 5) buys off the fat-pot temptation almost whole:
            // -2 + 1 need + 2 greedy = 1.
            Expect::PressureIs {
                who: "Steve",
                total: 1,
            },
            Expect::BandIs { band: Band::Calm },
            Expect::Survives { who: "Steve" },
            Expect::Wealth {
                who: "Steve",
                value: 4,
            },
            Expect::CleanJobs {
                who: "Steve",
                value: 1,
            },
            Expect::Desperation {
                who: "Steve",
                value: 0,
            },
            Expect::ReportSays {
                fragment: "the party read calm",
            },
            Expect::ReportSays {
                fragment: "Steve takes 4",
            },
        ],
    },
    BeatSpec {
        title: "Bob kills Steve",
        dilemma: "This vault needs two, and the chip under the party will read POWDER \
                  KEG: Bob's need beside that pot is how people die. Send them anyway \
                  - and read what it cost.",
        teaches: "the pot is the motive: fewer survivors means bigger shares",
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
        seed: 60,
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
        // The same murder, told by the dice the seed fixes: Bob stands at the
        // powder-keg cutoff exactly (-2 eager + 8 need + 2 opportunity), the
        // chip says so before SEND, and at this seed the occurrence roll lands
        // under his pressure and the severity roll finds the summit.
        ladder: &[
            Expect::PressureIs {
                who: "Bob",
                total: 8,
            },
            Expect::PressureIs {
                who: "Steve",
                total: 3,
            },
            Expect::BandIs {
                band: Band::PowderKeg,
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
            Expect::HasMark {
                who: "Bob",
                mark: MarkId::ComradeKiller,
            },
            Expect::Desperation {
                who: "Bob",
                value: 5,
            },
            Expect::Desperation {
                who: "Steve",
                value: 1,
            },
            Expect::ReportSays {
                fragment: "the party read powder keg",
            },
            Expect::ReportSays {
                fragment: "Bob killed Steve - pressure 8 at powder keg",
            },
            Expect::ReportSays {
                fragment: "marked comrade-killer",
            },
        ],
    },
    BeatSpec {
        title: "Tim refuses, Alex joins",
        dilemma: "Bob's sheet now says what he did, and Tim will not stand next to it - \
                  click Tim to hear him say so. Alex minds it less than he minds a thin \
                  winter: send Bob and Alex.",
        teaches: "marks are public, and traits decide who minds them",
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
            blurb: "Two pairs of hands and a long walk. The pot splits when they get back.",
            icon: QuestIcon::Tower,
            headcount: 2,
            pot: 8,
            cut: 2,
            requires: Requirement::AnyParty,
        }],
        send: &["Bob", "Alex"],
        seed: 0,
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
        // The ladder's common rung, taught early: at this seed Bob skims -
        // greedy under an uneasy pressure of 7 - and Alex walks home shorted.
        // Small betrayals are what teach the odds (DESIGN §8).
        ladder: &[
            Expect::PressureIs {
                who: "Bob",
                total: 7,
            },
            Expect::PressureIs {
                who: "Alex",
                total: 6,
            },
            Expect::BandIs { band: Band::Uneasy },
            Expect::Survives { who: "Bob" },
            Expect::Survives { who: "Alex" },
            // The skim's arithmetic: a share of 3 off the top, then 3 gold
            // split 2 ways - Bob 4, Alex 1.
            Expect::Wealth {
                who: "Bob",
                value: 4,
            },
            Expect::Wealth {
                who: "Alex",
                value: 1,
            },
            Expect::HasMark {
                who: "Bob",
                mark: MarkId::Skimmer,
            },
            // The shorted hold it against him; nobody bonds on a robbed job.
            Expect::Regard {
                from: "Alex",
                to: "Bob",
                value: -1,
            },
            Expect::Regard {
                from: "Bob",
                to: "Alex",
                value: 0,
            },
            Expect::CleanJobs {
                who: "Bob",
                value: 0,
            },
            Expect::Desperation {
                who: "Bob",
                value: 1,
            },
            Expect::Desperation {
                who: "Alex",
                value: 0,
            },
            Expect::Desperation {
                who: "Tim",
                value: 3,
            },
            Expect::ReportSays {
                fragment: "Bob skimmed the pot - pressure 7",
            },
            Expect::ReportSays {
                fragment: "marked skimmer",
            },
            Expect::ReportSays {
                fragment: "the party read uneasy",
            },
        ],
    },
    BeatSpec {
        title: "Tim's price is met",
        dilemma: "The same road, and Tim is hungrier for having sat out. Click him: the \
                  same company he refused is now barely worth bearing - and pressing \
                  reluctant people is what the chip prices in.",
        teaches: "refusal is temporary - need rises every round a person sits out",
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
        seed: 4,
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
        // The warning that does not come true: a reluctant Tim under a fat pot
        // stands at pressure 9, the chip says powder keg — and at this seed
        // both rolls miss, the job runs clean, and the reliable mark lands
        // exactly as under v1. A powder keg is a probability, not a promise.
        ladder: &[
            Expect::PressureIs {
                who: "Tim",
                total: 9,
            },
            Expect::PressureIs {
                who: "Bob",
                total: 7,
            },
            Expect::BandIs {
                band: Band::PowderKeg,
            },
            Expect::Survives { who: "Tim" },
            Expect::Survives { who: "Bob" },
            Expect::Wealth {
                who: "Tim",
                value: 3,
            },
            Expect::HasMark {
                who: "Bob",
                mark: MarkId::Reliable,
            },
            Expect::CleanJobs {
                who: "Bob",
                value: 2,
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
                fragment: "the party read powder keg",
            },
            Expect::ReportSays {
                fragment: "marked reliable",
            },
        ],
    },
];
