//! Judging one played beat's world: its `Expect` list, its door probes, and
//! the reasons behind every rendered verdict. The screens-versus-frame half
//! lives in `frames.rs`.
//!
//! The halves are separate because they fail for different reasons and are run
//! at different times: the mutation round judges the world only, because a
//! perturbed constant is a claim about outcomes and a thousand unread frames
//! are a thousand frames to allocate (FINDINGS G-004).

use jidousha::prelude::*;

use crate::beats::{BeatSpec, CHAIN, Expect};
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::flow::Stage;
use crate::model::Social;
use crate::variant::VariantId;
use crate::verify::BeatRun;
use crate::willing::willingness;
use crate::{layout, party};

/// Resolve a beat's authored names against the world it produced.
fn entities(social: &Social, names: &[&str]) -> Option<Vec<Entity>> {
    names
        .iter()
        .map(|name| social.by_name(name).map(|member| member.entity))
        .collect()
}

/// Whether an expectation is about the assembly moment — true of the claims
/// that hold under any variant and any seed (willingness is deterministic),
/// and judged on both rule sets for exactly that reason.
fn assembly_claim(expect: &Expect) -> bool {
    matches!(
        expect,
        Expect::Refuses { .. }
            | Expect::Joins { .. }
            | Expect::WillingnessIs { .. }
            | Expect::VerdictIs { .. }
            | Expect::TopReason { .. }
            | Expect::BandIs { .. }
            | Expect::PressureIs { .. }
    )
}

/// Judge one beat's `Expect` lists against what the run did.
///
/// The lists are authored in `beats.rs` by hand, against arithmetic a person
/// did on paper - which is what stops these assertions being the model reading
/// its own answer back. Which list depends on the run's variant (DESIGN §8e):
/// the deterministic run keeps v1's whole list; the ladder run takes the
/// assembly-moment claims (variant-independent by willingness determinism)
/// plus the beat's fixed-seed `ladder` list.
pub fn judge_world(checks: &mut Checks, spec: &BeatSpec, run: &BeatRun, tuning: &Tuning) {
    let beat = run.index + 1;
    let expectations: Vec<Expect> = match run.variant {
        VariantId::Deterministic => spec.expect.to_vec(),
        VariantId::Ladder => spec
            .expect
            .iter()
            .filter(|expect| assembly_claim(expect))
            .chain(spec.ladder.iter())
            .copied()
            .collect(),
    };
    // The beat's own job is part of the question (DESIGN §6: willingness
    // takes the quest); every assembly-moment claim is asked against it.
    let job = spec.dungeons.first();
    let ask = |who: &str, party: &[&str]| {
        let who_entity = run.at_assembly.by_name(who).map(|member| member.entity)?;
        let party = entities(&run.at_assembly, party)?;
        Some(willingness(
            &run.at_assembly,
            tuning,
            who_entity,
            &party,
            job,
        ))
    };
    for expect in &expectations {
        match *expect {
            Expect::Refuses { who, party } | Expect::Joins { who, party } => {
                let wants_join = matches!(expect, Expect::Joins { .. });
                let Some(answer) = ask(who, party) else {
                    checks.require(
                        false,
                        "a beat names somebody its roster does not have",
                        format!("beat {beat} expects {who} and a party of {party:?}"),
                    );
                    continue;
                };
                checks.require(
                    answer.joins() == wants_join,
                    if wants_join {
                        "somebody the beat says joins will not come"
                    } else {
                        "somebody the beat says refuses came anyway"
                    },
                    format!("beat {beat}: {who} - {}", answer.breakdown()),
                );
            }
            Expect::WillingnessIs { who, party, total } => {
                let Some(answer) = ask(who, party) else {
                    checks.require(
                        false,
                        "a beat names somebody its roster does not have",
                        format!("beat {beat} expects {who} and a party of {party:?}"),
                    );
                    continue;
                };
                checks.require(
                    answer.margin == total,
                    "a willingness the beat states exactly came out somewhere else",
                    format!(
                        "beat {beat}: {who} - {} , and the beat says {total}",
                        answer.breakdown()
                    ),
                );
            }
            Expect::VerdictIs {
                who,
                party,
                verdict,
            } => {
                let Some(answer) = ask(who, party) else {
                    checks.require(
                        false,
                        "a beat names somebody its roster does not have",
                        format!("beat {beat} expects {who} and a party of {party:?}"),
                    );
                    continue;
                };
                checks.require(
                    answer.verdict == verdict,
                    "a verdict the beat states exactly came out somewhere else",
                    format!(
                        "beat {beat}: {who}'s verdict is {:?} at {} and the beat says \
                         {verdict:?}",
                        answer.verdict,
                        answer.breakdown()
                    ),
                );
            }
            Expect::TopReason {
                who,
                party,
                fragment,
            } => {
                let Some(answer) = ask(who, party) else {
                    checks.require(
                        false,
                        "a beat names somebody its roster does not have",
                        format!("beat {beat} expects {who} and a party of {party:?}"),
                    );
                    continue;
                };
                checks.require(
                    answer.top_reason().contains(fragment),
                    "the leading reason is not the one the beat is about",
                    format!(
                        "beat {beat}: {who}'s top reason is {:?} and the beat wants \
                         {fragment:?} in it (all: {:?})",
                        answer.top_reason(),
                        answer
                            .reasons
                            .iter()
                            .map(crate::willing::Reason::text)
                            .collect::<Vec<_>>()
                    ),
                );
            }
            Expect::Killed { victim, by } => {
                let killer = run.after.by_name(by).map(|member| member.entity);
                let dead = run.after.by_name(victim);
                checks.require(
                    dead.is_some_and(|member| member.killed_by == killer && !member.alive),
                    "the killing the beat is about did not happen",
                    format!(
                        "beat {beat}: {victim} is {}, and the beat says {by} kills them",
                        match dead {
                            None => "not on the roster".to_owned(),
                            Some(member) => match member.killed_by {
                                None => "alive".to_owned(),
                                Some(other) => format!("killed by {}", run.after.name(other)),
                            },
                        }
                    ),
                );
            }
            Expect::Survives { who } => {
                let member = run.after.by_name(who);
                checks.require(
                    member.is_some_and(|member| member.alive),
                    "somebody the beat brings home did not come home",
                    format!("beat {beat}: {who} is {}", describe(run, who)),
                );
            }
            Expect::Desperation { who, value } => {
                let member = run.after.by_name(who);
                checks.require(
                    member.is_some_and(|member| member.desperation == value),
                    "a desperation trajectory ended somewhere the beat does not expect",
                    format!(
                        "beat {beat}: {who} ends at {}, and the beat says {value}",
                        member.map_or(-1, |member| member.desperation)
                    ),
                );
                shows_stat(checks, run, who, 0, value, "desperation");
            }
            Expect::HasMark { who, mark } | Expect::LacksMark { who, mark } => {
                let wants = matches!(expect, Expect::HasMark { .. });
                let member = run.after.by_name(who);
                let worn = member.is_some_and(|member| member.marks.contains(&mark));
                checks.require(
                    member.is_some() && worn == wants,
                    if wants {
                        "a mark the beat writes is not on the sheet"
                    } else {
                        "a mark the beat forbids was written anyway"
                    },
                    format!(
                        "beat {beat}: {who} ends wearing {:?}, and the beat says {mark:?} is \
                         {}on it",
                        member
                            .map(|member| member.marks.clone())
                            .unwrap_or_default(),
                        if wants { "" } else { "not " },
                    ),
                );
                // And the sheet says so, which is invariant 2: what everyone
                // knows is a line on the card.
                if let Some(member) = member {
                    let line = party::marks_line(member);
                    checks.require(
                        line.contains(mark.name()) == wants,
                        "the party card's mark line does not carry what the sheet knows",
                        format!(
                            "beat {beat}: {who}'s mark line is {line:?} and {:?} should \
                             {}be in it",
                            mark.name(),
                            if wants { "" } else { "not " },
                        ),
                    );
                }
            }
            Expect::CleanJobs { who, value } => {
                let member = run.after.by_name(who);
                checks.require(
                    member.is_some_and(|member| member.clean_jobs == value),
                    "a clean-job count ended somewhere the beat does not expect",
                    format!(
                        "beat {beat}: {who} ends at {}, and the beat says {value}",
                        member.map_or(-1, |member| member.clean_jobs)
                    ),
                );
            }
            Expect::Wealth { who, value } => {
                let member = run.after.by_name(who);
                checks.require(
                    member.is_some_and(|member| member.wealth == value),
                    "a wealth ended somewhere the beat does not expect",
                    format!(
                        "beat {beat}: {who} ends at {}, and the beat says {value}",
                        member.map_or(-1, |member| member.wealth)
                    ),
                );
                shows_stat(checks, run, who, 1, value, "wealth");
            }
            Expect::Regard { from, to, value } => {
                let (Some(from_entity), Some(to_entity)) = (
                    run.after.by_name(from).map(|member| member.entity),
                    run.after.by_name(to).map(|member| member.entity),
                ) else {
                    checks.require(
                        false,
                        "a beat names an edge between people its roster does not have",
                        format!("beat {beat}: {from} -> {to}"),
                    );
                    continue;
                };
                let held = run.after.regard(from_entity, to_entity);
                checks.require(
                    held == value,
                    "a regard edge ended somewhere the beat does not expect",
                    format!(
                        "beat {beat}: regard({from}->{to}) is {held}, and the beat says {value}"
                    ),
                );
                // And it is on the sheet, which is invariant 2: an edge that
                // decides an outcome is an edge on screen.
                let line = party::regard_line(&run.after, from_entity);
                checks.require(
                    value == 0 || line.contains(&format!("{to} {value:+}")),
                    "the party card does not show the regard edge the beat ends on",
                    format!(
                        "beat {beat}: {from}'s regard line is {line:?}, which does not carry \
                         {to} {value:+}"
                    ),
                );
            }
            Expect::ReportSays { fragment } => {
                checks.require(
                    run.report.iter().any(|line| line.contains(fragment)),
                    "the report does not narrate what the beat is about",
                    format!(
                        "beat {beat}: no line contains {fragment:?}; the report was {:?}",
                        run.report
                    ),
                );
            }
            Expect::BandIs { band } => {
                // Asserted against the Preview the staged strip drew from -
                // the surface itself, not a recomputation beside it.
                checks.require(
                    run.ready.band == Some(band),
                    "the band chip does not read what the beat says it reads",
                    format!(
                        "beat {beat}: the staged party's band is {:?} and the beat says \
                         {band:?}; the pressures were {:?}",
                        run.ready.band,
                        run.ready
                            .pressures
                            .iter()
                            .map(|p| p.total)
                            .collect::<Vec<_>>()
                    ),
                );
            }
            Expect::PressureIs { who, total } => {
                let entity = run.at_assembly.by_name(who).map(|member| member.entity);
                let found = entity.and_then(|entity| {
                    run.ready
                        .pressures
                        .iter()
                        .find(|pressure| pressure.who == entity)
                        .copied()
                });
                checks.require(
                    found.is_some_and(|pressure| pressure.total == total),
                    "a pressure the beat states exactly came out somewhere else",
                    format!(
                        "beat {beat}: {who}'s pressure is {:?} and the beat says {total} \
                         (strain + hunger + traits + opportunity)",
                        found
                    ),
                );
            }
        }
    }

    judge_door(checks, run);
    judge_reasons(checks, run);

    // The party the send verb saw is in roster order, which is the order
    // betrayal is evaluated in - so a party left in click order is a party
    // whose outcome depends on the order the player happened to click in.
    let order: Vec<usize> = run
        .ready
        .entries
        .iter()
        .filter_map(|entry| {
            run.at_assembly
                .member(entry.who)
                .map(|member| member.roster_index)
        })
        .collect();
    checks.require(
        order.len() == run.ready.entries.len() && order.is_sorted(),
        "the assembled party is not in roster order",
        format!(
            "beat {beat}: the party came out as roster positions {order:?}, and betrayal is \
             evaluated in roster order"
        ),
    );

    // The stage machine: sending reaches the takeover, dismissing it leaves.
    checks.require(
        run.stage_after_send == Stage::Resolution,
        "sending the party did not reach the resolution screen",
        format!(
            "beat {beat}: the stage after the send click is {:?}, and the party was {}",
            run.stage_after_send,
            if run.ready.can_send {
                "sendable"
            } else {
                "blocked"
            }
        ),
    );
    let last_beat = run.index + 1 == CHAIN.len();
    let wanted = if last_beat {
        Stage::Complete
    } else {
        Stage::Board
    };
    checks.require(
        run.stage_at_end == wanted,
        "dismissing the resolution screen went somewhere else",
        format!(
            "beat {beat} of {}: it left the game in {:?} on beat {}, wanted {wanted:?}",
            CHAIN.len(),
            run.stage_at_end,
            run.beat_at_end + 1
        ),
    );
}

/// **Every rendered verdict carries at least one reason** (DESIGN §14).
///
/// Asked of the staged board's `Preview` — the same resource the strip draws
/// from, and `judge_frames` proves the strip's rows reach the frame glyph for
/// glyph — so a verdict whose reason vanished would fail here before anybody
/// noticed a silent card. Both halves: the answer has reasons, and the status
/// line the card renders carries the leading one as words.
fn judge_reasons(checks: &mut Checks, run: &BeatRun) {
    let beat = run.index + 1;
    for entry in &run.ready.entries {
        checks.require(
            !entry.reasons.is_empty(),
            "a member's verdict rendered with no reason behind it",
            format!(
                "beat {beat}: {}'s answer is {}",
                entry.name,
                entry.breakdown()
            ),
        );
        if let Some(member) = run.at_assembly.member(entry.who) {
            let line = party::status_line(member, &run.ready, true);
            checks.require(
                line.contains(&entry.top_reason()),
                "a member's status line does not carry their leading reason",
                format!(
                    "beat {beat}: {}'s card says {line:?} and the reason is {:?}",
                    entry.name,
                    entry.top_reason()
                ),
            );
        }
    }
    for (who, door) in &run.ready.doors {
        let answer = match door {
            crate::willing::Admission::Admitted(entry)
            | crate::willing::Admission::Refuses(entry) => entry,
            crate::willing::Admission::Blocked { willingness, .. } => willingness,
        };
        let line = door.status_line();
        checks.require(
            !answer.reasons.is_empty() && line.contains(&answer.top_reason()),
            "a door answer rendered without its leading reason",
            format!(
                "beat {beat}: {}'s status line is {line:?} and the answer behind it says \
                 {:?}",
                run.at_assembly.name(*who),
                answer.top_reason()
            ),
        );
    }
}

/// **The door rule, as the player meets it** (DESIGN §6).
///
/// Both probes are the same two people in the two possible orders, so what is
/// checked is the rule's order-symmetry rather than one outcome twice: with the
/// clean one at the door the newcomer refuses, with the clean one already
/// inside the incumbent blocks, and both leave the party exactly as it was. A
/// rule that admitted either would let a party be sent that its own members
/// will not stand in.
fn judge_door(checks: &mut Checks, run: &BeatRun) {
    let beat = run.index + 1;
    if let (Some(bounce), Some(who)) = (&run.refusal, run.refusal_name) {
        let said = bounce.toast.clone().unwrap_or_default();
        checks.require(
            said.contains(who) && said.contains("refuses"),
            "a newcomer who refuses was not bounced with their own arithmetic",
            format!(
                "beat {beat}: the toast said {said:?}; it has to name {who} and say they refuse"
            ),
        );
        checks.require(
            !bounce.party.contains(&who),
            "somebody who refuses was added to the party anyway",
            format!("beat {beat}: the party came out {:?}", bounce.party),
        );
        checks.require(
            bounce.party.len() == 1,
            "a bounced click changed the party it bounced off",
            format!(
                "beat {beat}: one member was staged before the bounce and the party is {:?}",
                bounce.party
            ),
        );
        checks.require(
            bounce
                .logged
                .as_deref()
                .is_some_and(|line| line.contains(who)),
            "a bounced click did not reach the log",
            format!("beat {beat}: the newest log line is {:?}", bounce.logged),
        );
    }
    if let (Some(bounce), Some((blocked, blocker))) = (&run.veto, run.veto_names) {
        let said = bounce.toast.clone().unwrap_or_default();
        checks.require(
            said.contains(blocker) && said.contains(blocked),
            "an incumbent's veto did not name the blocker and the blocked",
            format!(
                "beat {beat}: the toast said {said:?}; {blocker} is the incumbent who blocks \
                 {blocked}"
            ),
        );
        checks.require(
            !bounce.party.contains(&blocked) && bounce.party.contains(&blocker),
            "an incumbent's veto did not keep the party it is about",
            format!(
                "beat {beat}: the party came out {:?}; {blocker} was in it and {blocked} was \
                 turned away at the door",
                bounce.party
            ),
        );
        checks.require(
            bounce
                .logged
                .as_deref()
                .is_some_and(|line| line.contains(blocker) && line.contains(blocked)),
            "an incumbent's veto did not reach the log",
            format!("beat {beat}: the newest log line is {:?}", bounce.logged),
        );
    }
}

/// A stat the beat states exactly is a stat on the party card.
///
/// Invariant 2 applied to the strip: a number that decides an outcome is a
/// number on screen, beside the icon that says which number it is.
fn shows_stat(checks: &mut Checks, run: &BeatRun, who: &str, slot: usize, value: i32, what: &str) {
    let Some(member) = run.after.by_name(who) else {
        return;
    };
    let index = member.roster_index;
    let stats = party::stats_of(layout::party_card(index), member, member.alive);
    let shown = stats.get(slot).map(|stat| stat.value.text.clone());
    checks.require(
        shown.as_deref() == Some(value.to_string().as_str()),
        "the party card does not show the stat the beat ends on",
        format!(
            "beat {}: {who}'s {what} slot reads {shown:?} and the beat says {value}",
            run.index + 1
        ),
    );
}

fn describe(run: &BeatRun, who: &str) -> String {
    match run.after.by_name(who) {
        None => "not on the roster".to_owned(),
        Some(member) if member.alive => "alive".to_owned(),
        Some(member) => format!(
            "dead, killed by {}",
            member
                .killed_by
                .map_or("?", |killer| run.after.name(killer))
        ),
    }
}
