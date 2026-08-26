//! Willingness v2, asked directly: trait modifiers, the trait x mark table,
//! verdict boundaries, and the reasons vocabulary (DESIGN §4, §5, §6).
//!
//! The tutorial exercises one trait pair and two marks; the vocabulary is
//! nine traits and seven marks, and the margins the design rests on — a dark
//! mark repels a stranger, a light one pulls, the pragmatic *prefer* a known
//! skimmer, the reluctant band sits exactly where `reluctant_below` says —
//! live on benches here. The mutation round runs this file with every
//! constant perturbed, which is what makes each claim an instrument.

use crate::checks::Checks;
use crate::constants::Tuning;
use crate::contracts::{bench, bench_job, resolve_v1, set_clean_jobs};
use crate::model::Social;
use crate::traits::{MarkId, TraitId, reaction_delta};
use crate::willing::{Cause, Verdict, willingness};

/// The willingness v2 battery.
pub fn battery(checks: &mut Checks, tuning: &Tuning) {
    // --- the pot, through traits (DESIGN §6: pot-through-traits) ----------
    let (world, ids) = bench(
        &[
            ("Hungry", 0, &[TraitId::Greedy], &[]),
            ("Plain", 0, &[], &[]),
        ],
        &[],
    );
    let social = Social::read(&world.view());
    let job = bench_job(2, 10, 2);
    let greedy = willingness(&social, tuning, ids[0], &ids, Some(&job));
    let plain = willingness(&social, tuning, ids[1], &ids, Some(&job));
    checks.require(
        greedy.pot_total == 4 * tuning.pot_pull && greedy.pot_total > 0,
        "the pot does not pull a greedy character by the promised share",
        format!(
            "a 4g share pulls {} at a pot_pull of {}; greedy's affinity is 1 and the pot \
             term is share x pot_pull x affinity",
            greedy.pot_total, tuning.pot_pull
        ),
    );
    checks.require(
        plain.pot_total == 0,
        "the pot pulls a character with no pot trait",
        format!(
            "Plain's pot term is {}; in P1 the pot enters willingness only through traits",
            plain.pot_total
        ),
    );
    checks.require(
        greedy.reasons.iter().any(|r| r.cause == Cause::PotPull),
        "the pot pulled and no reason says so",
        format!("greedy's reasons are {:?}", greedy.reasons),
    );
    // And with no job taken, nothing pulls.
    let unpressed = willingness(&social, tuning, ids[0], &ids, None);
    checks.require(
        unpressed.pot_total == 0,
        "a pot pulls before any quest is taken",
        format!("with no job the pot term is {}", unpressed.pot_total),
    );

    // --- regard through the trait multipliers ------------------------------
    for (label, traits, edge, want) in [
        ("loyal doubles a bond", &[TraitId::Loyal][..], 2, 4),
        ("loyal leaves a grudge alone", &[TraitId::Loyal][..], -2, -2),
        (
            "vengeful doubles a grudge",
            &[TraitId::Vengeful][..],
            -2,
            -4,
        ),
        (
            "vengeful leaves a bond alone",
            &[TraitId::Vengeful][..],
            2,
            2,
        ),
        ("cold halves a bond", &[TraitId::Cold][..], 4, 2),
        ("cold halves a grudge", &[TraitId::Cold][..], -4, -2),
        ("no trait weighs regard at all", &[][..], 3, 3),
    ] {
        let (world, ids) = bench(
            &[("Asker", 0, traits, &[]), ("Other", 0, &[], &[])],
            &[(0, 1, edge)],
        );
        let social = Social::read(&world.view());
        let answer = willingness(&social, tuning, ids[0], &ids, None);
        checks.require(
            answer.regard_total == want,
            "a trait's regard multiplier is not the one its row states",
            format!(
                "{label}: an edge of {edge} weighs {}, wanted {want}",
                answer.regard_total
            ),
        );
    }

    // --- the trait x mark table --------------------------------------------
    let react = |traits: &'static [TraitId], marks: &'static [MarkId]| {
        let (world, ids) = bench(&[("Asker", 0, traits, &[]), ("Worn", 0, &[], marks)], &[]);
        let social = Social::read(&world.view());
        willingness(&social, tuning, ids[0], &ids, None)
    };
    // A dark mark repels a stranger, by the base the constant names.
    let stranger = react(&[], &[MarkId::ComradeKiller]);
    checks.require(
        stranger.reaction_total == -tuning.mark_dark && stranger.reaction_total < 0,
        "a dark mark does not repel a stranger",
        format!(
            "the reaction is {} at a mark_dark of {}",
            stranger.reaction_total, tuning.mark_dark
        ),
    );
    // A light mark pulls, by its base.
    let bright = react(&[], &[MarkId::Reliable]);
    checks.require(
        bright.reaction_total == tuning.mark_light && bright.reaction_total > 0,
        "a light mark does not pull a stranger",
        format!(
            "the reaction is {} at a mark_light of {}",
            bright.reaction_total, tuning.mark_light
        ),
    );
    checks.require(
        bright.reasons.iter().any(|r| {
            matches!(
                r.cause,
                Cause::MarkFor {
                    mark: MarkId::Reliable,
                    ..
                }
            )
        }),
        "a light mark pulled and no reason says so",
        format!("the reasons are {:?}", bright.reasons),
    );
    // The upright mind a dark mark more than a stranger does.
    let upright = react(&[TraitId::Upright], &[MarkId::ComradeKiller]);
    checks.require(
        upright.reaction_total
            == -tuning.mark_dark + reaction_delta(TraitId::Upright, MarkId::ComradeKiller)
            && upright.reaction_total < stranger.reaction_total,
        "the upright do not mind a comrade-killer more than a stranger does",
        format!(
            "upright reacts {} and a stranger {}",
            upright.reaction_total, stranger.reaction_total
        ),
    );
    // **The attraction case** (DESIGN §5): the pragmatic *prefer* a known
    // skimmer — the table's delta turns the dark base positive.
    let pragmatic = react(&[TraitId::Pragmatic], &[MarkId::Skimmer]);
    let stranger_to_skimmer = react(&[], &[MarkId::Skimmer]);
    checks.require(
        pragmatic.reaction_total == -tuning.mark_dark + 2
            && pragmatic.reaction_total > stranger_to_skimmer.reaction_total
            && pragmatic.reaction_total > 0,
        "a known skimmer no longer attracts the pragmatic",
        format!(
            "pragmatic reacts {} and a stranger {}; the table's +2 has to beat the dark \
             base or the v1 only-closes-doors problem is back",
            pragmatic.reaction_total, stranger_to_skimmer.reaction_total
        ),
    );
    checks.require(
        pragmatic.reasons.iter().any(|r| {
            matches!(
                r.cause,
                Cause::MarkFor {
                    mark: MarkId::Skimmer,
                    ..
                }
            )
        }) && pragmatic.top_reason().contains("prefers a known skimmer"),
        "the attraction has no words",
        format!(
            "the pragmatic's reasons are {:?} and the top is {:?}",
            pragmatic.reasons,
            pragmatic.top_reason()
        ),
    );
    // Marks stack: two dark marks are two reactions.
    let twice = react(&[], &[MarkId::Skimmer, MarkId::Deserter]);
    checks.require(
        twice.reaction_total == -2 * tuning.mark_dark,
        "two dark marks do not cost two reactions",
        format!(
            "the reaction to a skimmer-deserter is {} at a mark_dark of {}",
            twice.reaction_total, tuning.mark_dark
        ),
    );

    // --- the per-member breakdown, which is rung 2's data (UI.md §5) -------
    //
    // One term per partymate, in roster order, and the totals are their sums.
    // Directedness rides along: the asker's terms are about what *they* hold
    // and see, and the mirror answer holds nothing.
    let (world, ids) = bench(
        &[
            ("Asker", 0, &[], &[]),
            ("Marked", 0, &[], &[MarkId::Skimmer]),
            ("Held", 0, &[], &[]),
        ],
        &[(0, 2, 3)],
    );
    let social = Social::read(&world.view());
    let answer = willingness(&social, tuning, ids[0], &ids, None);
    checks.require(
        answer.terms.len() == 2
            && answer.terms.first().is_some_and(|term| {
                term.member == ids[1] && term.reaction == -tuning.mark_dark && term.regard == 0
            })
            && answer.terms.get(1).is_some_and(|term| {
                term.member == ids[2] && term.reaction == 0 && term.regard == 3
            })
            && answer.reaction_total == answer.terms.iter().map(|term| term.reaction).sum()
            && answer.regard_total == answer.terms.iter().map(|term| term.regard).sum(),
        "the per-member terms are not the sums they add up to",
        format!(
            "the terms are {:?} against totals m{} r{}",
            answer.terms, answer.reaction_total, answer.regard_total
        ),
    );
    let back = willingness(&social, tuning, ids[2], &ids, None);
    checks.require(
        back.regard_total == 0,
        "regard is being read as symmetric",
        format!(
            "Held answers {} about a party they hold nothing about",
            back.breakdown()
        ),
    );

    // --- verdict boundaries (DESIGN §6) ------------------------------------
    let (world, ids) = bench(
        &[("Sour", 0, &[], &[]), ("Other", 0, &[], &[])],
        &[(0, 1, -1)],
    );
    let social = Social::read(&world.view());
    let sour = willingness(&social, tuning, ids[0], &ids, None);
    checks.require(
        sour.margin == -1 && sour.verdict == Verdict::Refuses && !sour.joins(),
        "a negative margin is not a refusal",
        format!("a margin of {} came out {:?}", sour.margin, sour.verdict),
    );
    checks.require(
        sour.top_reason().contains("despises Other"),
        "a grudge refusal has no words",
        format!("the top reason is {:?}", sour.top_reason()),
    );
    let (world, ids) = bench(&[("Zero", 0, &[], &[])], &[]);
    let social = Social::read(&world.view());
    let zero = willingness(&social, tuning, ids[0], &ids, None);
    let wanted = if tuning.reluctant_below > 0 {
        Verdict::Reluctant
    } else {
        Verdict::Joins
    };
    checks.require(
        zero.margin == 0 && zero.joins() && zero.verdict == wanted,
        "a margin of zero is not on the joining side of the boundary",
        format!(
            "it came out {:?} at a reluctant_below of {}; zero joins, and joins reluctantly \
             while the band exists",
            zero.verdict, tuning.reluctant_below
        ),
    );
    // The fallback reason: nothing pulls, and the card still says something.
    checks.require(
        zero.reasons.len() == 1 && zero.reasons[0].cause == Cause::Indifferent,
        "a verdict with no causes did not fall back to the indifferent reason",
        format!("the reasons are {:?}", zero.reasons),
    );
    let (world, ids) = bench(&[("Keen", tuning.reluctant_below, &[], &[])], &[]);
    let social = Social::read(&world.view());
    let keen = willingness(&social, tuning, ids[0], &ids, None);
    checks.require(
        keen.verdict == Verdict::Joins,
        "a margin at reluctant_below is still called reluctant",
        format!(
            "a margin of {} came out {:?}; the band is margins strictly below {}",
            keen.margin, keen.verdict, tuning.reluctant_below
        ),
    );

    // --- reasons order by contribution -------------------------------------
    let (world, ids) = bench(
        &[("Torn", 1, &[], &[]), ("Friend", 0, &[], &[])],
        &[(0, 1, 5)],
    );
    let social = Social::read(&world.view());
    let torn = willingness(&social, tuning, ids[0], &ids, None);
    checks.require(
        torn.top_reason().contains("trusts Friend"),
        "the strongest cause is not the leading reason",
        format!(
            "with need 1 and a bond of 5 the top reason is {:?}",
            torn.top_reason()
        ),
    );
    let (world, ids) = bench(
        &[("Broke", 5, &[], &[]), ("Friend", 0, &[], &[])],
        &[(0, 1, 1)],
    );
    let social = Social::read(&world.view());
    let broke = willingness(&social, tuning, ids[0], &ids, None);
    checks.require(
        broke.top_reason().contains("needs the money"),
        "the strongest cause is not the leading reason",
        format!(
            "with need 5 and a bond of 1 the top reason is {:?}",
            broke.top_reason()
        ),
    );

    // --- the clean-job count and the reliable mark (DESIGN §5) -------------
    let (mut world, ids) = bench(&[("Almost", 0, &[], &[]), ("Fresh", 0, &[], &[])], &[]);
    set_clean_jobs(&mut world, ids[0], tuning.reliable_after - 1);
    let social = Social::read(&world.view());
    let outcome = resolve_v1(&social, tuning, &bench_job(2, 6, 2), &ids);
    checks.require(
        outcome
            .clean_job_changes
            .iter()
            .any(|(who, before, after)| {
                *who == ids[0]
                    && *before == tuning.reliable_after - 1
                    && *after == tuning.reliable_after
            }),
        "a clean job did not count for a survivor",
        format!("the counts moved {:?}", outcome.clean_job_changes),
    );
    checks.require(
        outcome
            .mark_writes
            .iter()
            .any(|(who, mark)| *who == ids[0] && *mark == MarkId::Reliable),
        "the clean-job count reached the threshold and wrote no reliable mark",
        format!(
            "the writes are {:?} at a reliable_after of {}",
            outcome.mark_writes, tuning.reliable_after
        ),
    );
    checks.require(
        outcome
            .lines
            .iter()
            .any(|line| line.contains("marked reliable")),
        "the reliable mark was written and the report does not say so",
        format!("the narration is {:?}", outcome.lines),
    );
    let fresh_marked = outcome
        .mark_writes
        .iter()
        .any(|(who, mark)| *who == ids[1] && *mark == MarkId::Reliable);
    checks.require(
        fresh_marked == (1 >= tuning.reliable_after),
        "the reliable mark does not follow the count",
        format!(
            "Fresh's first clean job {} the mark at a reliable_after of {}",
            if fresh_marked {
                "wrote"
            } else {
                "did not write"
            },
            tuning.reliable_after
        ),
    );
    // A mark is a fact, not a counter: already reliable writes nothing.
    let (mut world, ids) = bench(
        &[
            ("Steady", 0, &[], &[MarkId::Reliable]),
            ("Other", 0, &[], &[]),
        ],
        &[],
    );
    set_clean_jobs(&mut world, ids[0], tuning.reliable_after);
    let social = Social::read(&world.view());
    let outcome = resolve_v1(&social, tuning, &bench_job(2, 6, 2), &ids);
    checks.require(
        !outcome
            .mark_writes
            .iter()
            .any(|(who, mark)| *who == ids[0] && *mark == MarkId::Reliable),
        "a mark already on the sheet was written again",
        format!("the writes are {:?}", outcome.mark_writes),
    );
    // And the same for the murder's mark: a second kill blackens nothing new.
    let (world, ids) = bench(
        &[
            ("Twice", 9, &[], &[MarkId::ComradeKiller]),
            ("Vic", 0, &[], &[]),
        ],
        &[],
    );
    let social = Social::read(&world.view());
    let outcome = resolve_v1(&social, tuning, &bench_job(2, 6, 0), &ids);
    checks.require(
        outcome.betrayals.len() == 1
            && !outcome
                .mark_writes
                .iter()
                .any(|(_, mark)| *mark == MarkId::ComradeKiller),
        "a killer already marked comrade-killer was marked again",
        format!(
            "{} killing(s), and the writes are {:?}",
            outcome.betrayals.len(),
            outcome.mark_writes
        ),
    );
}
