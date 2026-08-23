//! Judging one played beat: its `Expect` list against the world, its door
//! probes against the door rule, and its screens against the frame.
//!
//! The halves are separate because they fail for different reasons and are run
//! at different times: the mutation round judges the world only, because a
//! perturbed constant is a claim about outcomes and a thousand unread frames
//! are a thousand frames to allocate (FINDINGS G-004).

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord};

use crate::beats::{BeatSpec, CHAIN, Expect};
use crate::checks::{Checks, near};
use crate::constants::Tuning;
use crate::flow::Stage;
use crate::model::{Social, willingness};
use crate::verify::BeatRun;
use crate::{layout, party, screens, theme, ui};

/// Resolve a beat's authored names against the world it produced.
fn entities(social: &Social, names: &[&str]) -> Option<Vec<Entity>> {
    names
        .iter()
        .map(|name| social.by_name(name).map(|member| member.entity))
        .collect()
}

/// Judge one beat's `Expect` list against what the run did.
///
/// The list is authored in `beats.rs` by hand, against arithmetic a person did
/// on paper - which is what stops these assertions being the model reading its
/// own answer back.
pub fn judge_world(checks: &mut Checks, spec: &BeatSpec, run: &BeatRun, tuning: &Tuning) {
    let beat = run.index + 1;
    for expect in spec.expect {
        match *expect {
            Expect::Refuses { who, party } | Expect::Joins { who, party } => {
                let wants_join = matches!(expect, Expect::Joins { .. });
                let (Some(who_entity), Some(party)) = (
                    run.at_assembly.by_name(who).map(|member| member.entity),
                    entities(&run.at_assembly, party),
                ) else {
                    checks.require(
                        false,
                        "a beat names somebody its roster does not have",
                        format!("beat {beat} expects {who} and a party of {party:?}"),
                    );
                    continue;
                };
                let answer = willingness(&run.at_assembly, tuning, who_entity, &party);
                checks.require(
                    answer.joins() == wants_join,
                    if wants_join {
                        "somebody the beat says joins will not come"
                    } else {
                        "somebody the beat says refuses came anyway"
                    },
                    format!("beat {beat}: {who} - {}", answer.arithmetic()),
                );
            }
            Expect::WillingnessIs { who, party, total } => {
                let (Some(who_entity), Some(party)) = (
                    run.at_assembly.by_name(who).map(|member| member.entity),
                    entities(&run.at_assembly, party),
                ) else {
                    checks.require(
                        false,
                        "a beat names somebody its roster does not have",
                        format!("beat {beat} expects {who} and a party of {party:?}"),
                    );
                    continue;
                };
                let answer = willingness(&run.at_assembly, tuning, who_entity, &party);
                checks.require(
                    answer.total == total,
                    "a willingness the beat states exactly came out somewhere else",
                    format!(
                        "beat {beat}: {who} - {} , and the beat says {total}",
                        answer.arithmetic()
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
            Expect::Infamy { who, value } => {
                let member = run.after.by_name(who);
                checks.require(
                    member.is_some_and(|member| member.infamy == value),
                    "an infamy ended somewhere the beat does not expect",
                    format!(
                        "beat {beat}: {who} ends at {}, and the beat says {value}",
                        member.map_or(-1, |member| member.infamy)
                    ),
                );
                shows_stat(checks, run, who, 1, value, "infamy");
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
                shows_stat(checks, run, who, 2, value, "wealth");
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
        }
    }

    judge_door(checks, run);

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

/// **The door rule, as the player meets it** (DESIGN §3.2).
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

/// How many of a row's glyphs were drawn, counted inside the row's own box.
///
/// `ctx.text` puts the top-left of the first character's *cell* at `at` and
/// advances along the row, so a run's glyphs share `bounds().min.y` and span
/// exactly `width_of(text)`. Counting *inside that span* rather than "everything
/// to the right of it" is what lets two runs share a row - the status bar puts
/// four on one - without either counting the other's characters.
pub fn glyph_run(frame: &FrameRecord, font: BackendTextureId, run: &ui::TextRun) -> usize {
    let box_ = run.bounds();
    frame
        .quads()
        .iter()
        .filter(|quad| {
            quad.texture == font
                && near(quad.bounds().min.y, box_.min.y)
                // Half a world unit of slack on each side: a row's own glyphs
                // span exactly `width_of(text)` in exact arithmetic, and
                // whether the last one lands a hair over is a rounding question
                // no assertion should turn on. One glyph is nine units wide, so
                // this cannot swallow a neighbouring row's first character.
                && quad.bounds().min.x >= box_.min.x - 0.5
                && quad.bounds().max.x <= box_.max.x + 0.5
        })
        .count()
}

/// Judge what a beat's screens drew.
///
/// Every screen is asked the same question — is every row of what this screen
/// *says* on the frame, all of it — against `screens::content`, which is the
/// same function the draw system renders. One layout, two readers.
pub fn judge_frames(checks: &mut Checks, run: &BeatRun) {
    let beat = run.index + 1;
    for (mode, frame, flow, social, preview) in [
        (
            "the board with the quest taken",
            &run.board_frame,
            &run.board_flow,
            &run.at_assembly,
            &run.board_preview,
        ),
        (
            "the board with the party staged",
            &run.ready_frame,
            &run.ready_flow,
            &run.at_assembly,
            &run.ready,
        ),
        (
            "the resolution takeover",
            &run.report_frame,
            &run.report_flow,
            &run.after,
            &run.report_preview,
        ),
    ] {
        let Some(frame) = frame else { continue };
        let panel = screens::content(flow, social, preview);
        for text_run in &panel.runs {
            let drawn = glyph_run(frame, run.font, text_run);
            checks.require(
                drawn == text_run.text.chars().count(),
                "a row of a screen is not drawn as the string it is",
                format!(
                    "beat {beat}, {mode}: {:?} at ({:.1}, {:.1}) is {} characters and {drawn} \
                     glyphs landed in its box",
                    text_run.text,
                    text_run.at.x,
                    text_run.at.y,
                    text_run.text.chars().count(),
                ),
            );
        }
        for icon in &panel.icons {
            let covered = frame.quads().iter().any(|quad| {
                quad.texture != run.font
                    && near(quad.bounds().min.x, icon.at.x)
                    && near(quad.bounds().min.y, icon.at.y)
            });
            checks.require(
                covered,
                "an icon a screen says it draws is not on the frame",
                format!(
                    "beat {beat}, {mode}: {:?} at ({:.1}, {:.1}) has no quad",
                    icon.art, icon.at.x, icon.at.y
                ),
            );
        }
    }

    // --- a card's edge says whether its character is in (UI.md §2) --------
    //
    // Two channels for one fact: the status line says "in" and the border says
    // it again in teal. Asserted because a border is the one signifier no
    // string check can see, and because the colour is what a player reads at a
    // glance while the arithmetic is what they read on purpose.
    if let Some(frame) = &run.ready_frame {
        let quads = frame.quads();
        for member in &run.at_assembly.members {
            let card = layout::party_card(member.roster_index);
            let edge = quads
                .iter()
                .find(|quad| {
                    quad.texture != run.font
                        && near(quad.bounds().min.x, card.min.x)
                        && near(quad.bounds().min.y, card.min.y)
                        && quad.bounds().size().y < 4.0
                })
                .map(|quad| quad.tint);
            let inside = run.ready_flow.party.contains(&member.entity);
            let wanted = if inside {
                theme::REGARD
            } else if member.alive {
                theme::BORDER
            } else {
                theme::RULE
            };
            checks.require(
                edge == Some(wanted),
                "a party card's edge does not say whether its character is in",
                format!(
                    "beat {beat}: {} is {}in the party and the card's top edge is {edge:?}, \
                     wanted {wanted:?}",
                    member.name,
                    if inside { "" } else { "not " }
                ),
            );
        }
    }

    // --- the send verb exists only while a quest is taken (UI.md §3) -------
    if let Some(frame) = &run.board_frame {
        let button = layout::send_button();
        // *Either* face colour: a button that exists and is disabled is the
        // state UI.md §3 asks for once a quest is taken and the party is still
        // short, and looking only for the live gold would call that absent.
        let drawn = frame.quads().iter().any(|quad| {
            quad.texture != run.font
                && (quad.tint == theme::GOLD || quad.tint == theme::BUTTON_DEAD)
                && near(quad.bounds().min.x, button.min.x)
                && near(quad.bounds().min.y, button.min.y)
        });
        checks.require(
            drawn == run.board_flow.taken.is_some(),
            "the send verb's presence does not follow whether a quest is taken",
            format!(
                "beat {beat}: a quest is {}taken and the button face is {}drawn; UI.md §3 says \
                 it exists only while one is",
                if run.board_flow.taken.is_some() {
                    ""
                } else {
                    "not "
                },
                if drawn { "" } else { "not " },
            ),
        );
        // And it is disabled with a stated reason rather than silently dead.
        checks.require(
            run.board_preview.can_send || !run.board_preview.blocked.is_empty(),
            "the send verb is disabled without saying why",
            format!(
                "beat {beat}: the gate cannot send and its stated reason is {:?}",
                run.board_preview.blocked
            ),
        );
    }

    // --- the takeover replaces the board entirely (UI.md §3) ---------------
    if let Some(frame) = &run.report_frame {
        let middle = layout::design().center();
        let front = frame.covering(middle).into_iter().next();
        checks.require(
            front.is_some_and(|quad| {
                quad.tint == theme::SCRIM || quad.texture == run.font || quad.tint == theme::BAR
            }),
            "the resolution screen does not replace the board it took over",
            format!(
                "beat {beat}: the front-most quad at the middle of the screen is {:?}; the \
                 takeover is a full-screen replacement, not a panel over a board",
                front.map(|quad| quad.tint)
            ),
        );
        let cards = frame
            .quads()
            .iter()
            .filter(|quad| quad.tint == theme::BAR)
            .count();
        checks.require(
            cards >= run.report_flow.events.len(),
            "the resolution screen drew fewer event cards than the run produced",
            format!(
                "beat {beat}: {} events and {cards} card fills",
                run.report_flow.events.len()
            ),
        );
    }
}
