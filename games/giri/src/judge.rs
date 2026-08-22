//! Judging one played beat: its `Expect` list against the world, and its
//! panels against the frame.
//!
//! The two halves are separate because they fail for different reasons and are
//! run at different times: the mutation round judges the world only, because a
//! perturbed constant is a claim about outcomes and a thousand unread frames
//! are a thousand frames to allocate.

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord};

use crate::beats::{BeatSpec, CHAIN, Expect, stat_line};
use crate::checks::{Checks, near};
use crate::constants::Tuning;
use crate::flow::{Flow, Stage};
use crate::model::{Social, willingness};
use crate::ui;
use crate::verify::BeatRun;

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
                // And the sheet says so, which is invariant 2: a number that
                // decides an outcome is a number on screen. The string is the
                // only instrument - no assertion over drawn quads can read one.
                checks.require(
                    member
                        .is_some_and(|member| stat_line(member).contains(&format!("DES {value}"))),
                    "the sheet does not show the desperation the beat ends on",
                    format!(
                        "beat {beat}: {who}'s sheet line is {:?}, which does not carry DES {value}",
                        member.map(stat_line)
                    ),
                );
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
                checks.require(
                    member
                        .is_some_and(|member| stat_line(member).contains(&format!("INF {value}"))),
                    "the sheet does not show the infamy the beat ends on",
                    format!(
                        "beat {beat}: {who}'s sheet line is {:?}, which does not carry INF {value}",
                        member.map(stat_line)
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

    // The stage machine: sending reaches the report, continuing leaves it.
    checks.require(
        run.stage_after_send == Stage::Report,
        "sending the party did not reach the report",
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
        Stage::Assembly
    };
    checks.require(
        run.stage_at_end == wanted,
        "continuing out of the report went somewhere else",
        format!(
            "beat {beat} of {}: continuing left the game in {:?} on beat {}, wanted {wanted:?}",
            CHAIN.len(),
            run.stage_at_end,
            run.beat_at_end + 1
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

/// How many glyphs were drawn as one row starting at `at`.
///
/// `ctx.text` puts the top-left of the first character's *cell* at `at` and
/// advances along the row, so a run's glyphs all share `bounds().min.y` and
/// start at `at.x`. Counting them is how a check reads "this string was drawn,
/// all of it" off a frame that cannot tell it which characters they were.
pub fn glyph_run(frame: &FrameRecord, font: BackendTextureId, at: Vec2) -> usize {
    frame
        .quads()
        .iter()
        .filter(|quad| {
            quad.texture == font
                && near(quad.bounds().min.y, at.y)
                && quad.bounds().min.x >= at.x - 0.001
        })
        .count()
}

/// Judge what a beat drew.
pub fn judge_frames(checks: &mut Checks, spec: &BeatSpec, run: &BeatRun) {
    let beat = run.index + 1;
    // Every sheet: name, stats, edges and status, all of it on screen. The
    // count is exact because `ctx.text` submits one quad per character, spaces
    // included, and none of these strings has a line break in it.
    if let Some(frame) = &run.ready_frame {
        for (index, member) in run.at_assembly.members.iter().enumerate() {
            let card = ui::card_rect(index);
            let drawn = frame
                .quads()
                .iter()
                .filter(|quad| quad.texture == run.font && card.contains_rect(quad.bounds()))
                .count();
            let wanted = 1
                + member.name.chars().count()
                + stat_line(member).chars().count()
                + ui::regard_line(&run.at_assembly, member.entity)
                    .chars()
                    .count()
                + ui::status_line(
                    &run.at_assembly,
                    member,
                    run.ready.entries.iter().any(|e| e.who == member.entity),
                )
                .chars()
                .count();
            checks.require(
                drawn == wanted,
                "a roster sheet is not drawing everything it says it draws",
                format!(
                    "beat {beat}: {}'s card holds {drawn} glyphs and its four lines plus the \
                     portrait initial are {wanted}",
                    member.name
                ),
            );
        }
        // And the assembled party's willingness, row by row, in the same
        // arithmetic the send gate used.
        let runs = ui::assembly_runs(spec, &Flow::default(), &run.ready);
        for text_run in &runs {
            checks.require(
                glyph_run(frame, run.font, text_run.at) == text_run.text.chars().count(),
                "a row of the assembly panel is not drawn as the string it is",
                format!(
                    "beat {beat}: {:?} at ({:.2}, {:.2}) is {} characters and {} glyphs were \
                     drawn on that row",
                    text_run.text,
                    text_run.at.x,
                    text_run.at.y,
                    text_run.text.chars().count(),
                    glyph_run(frame, run.font, text_run.at)
                ),
            );
        }
    }
    // The refusal, with its arithmetic, before anything was committed.
    if let (Some(frame), Some(probe)) = (&run.probe_frame, &run.probe) {
        let refusing: Vec<&crate::model::Willingness> = probe
            .entries
            .iter()
            .filter(|entry| !entry.joins())
            .collect();
        checks.require(
            !refusing.is_empty(),
            "the beat's refusal probe selected nobody who refuses",
            format!(
                "beat {beat}: every one of {:?} was willing",
                probe.entries.iter().map(|e| e.name).collect::<Vec<_>>()
            ),
        );
        for entry in refusing {
            let line = ui::willingness_line(entry);
            let row = ui::assembly_runs(spec, &Flow::default(), probe)
                .into_iter()
                .find(|text_run| text_run.text == line);
            let drawn = row
                .as_ref()
                .map_or(0, |text_run| glyph_run(frame, run.font, text_run.at));
            checks.require(
                drawn == line.chars().count(),
                "a refusal is not shown with its arithmetic before commitment",
                format!(
                    "beat {beat}: {line:?} is {} characters and {drawn} glyphs were drawn on \
                     its row",
                    line.chars().count()
                ),
            );
        }
        checks.require(
            !probe.can_send,
            "a party somebody refuses can be sent anyway",
            format!("beat {beat}: the gate said {:?}", probe.blocked),
        );
    }
    // A half-filled party says so, in the numbers the panels show.
    if let (Some(frame), Some(_)) = (&run.partial_frame, spec.send.get(1)) {
        let glyphs = frame
            .quads()
            .iter()
            .filter(|quad| quad.texture == run.font)
            .count();
        checks.require(
            glyphs > 0,
            "the half-filled assembly screen drew no text at all",
            format!("beat {beat}: {glyphs} glyphs"),
        );
    }
    // The report, row by row - the story surface, drawn.
    if let Some(frame) = &run.report_frame {
        let flow = Flow {
            report: run.report.clone(),
            ..Flow::default()
        };
        for text_run in ui::report_runs(&flow) {
            let drawn = glyph_run(frame, run.font, text_run.at);
            checks.require(
                drawn == text_run.text.chars().count(),
                "a row of the resolution report is not drawn as the string it is",
                format!(
                    "beat {beat}: {:?} is {} characters and {drawn} glyphs were drawn at \
                     ({:.2}, {:.2})",
                    text_run.text,
                    text_run.text.chars().count(),
                    text_run.at.x,
                    text_run.at.y
                ),
            );
        }
        // The bands, where the sort disagrees with the submission order:
        // `draw_headline` submits its glyphs *before* `draw_backdrop` submits
        // the bar behind them, so only TEXT sorting over PANEL puts the bar
        // first. Where a game's submission order already agrees with its
        // bands, no assertion over a recorded frame can see a band at all.
        let quads = frame.quads();
        let headline_at = Vec2::new(ui::ROSTER_X, -crate::HALF_H + 0.35);
        let bar = quads
            .iter()
            .position(|quad| quad.tint == ui::PANEL_FILL && quad.bounds().size().x > 20.0);
        let glyph = quads
            .iter()
            .position(|quad| quad.texture == run.font && near(quad.bounds().min.y, headline_at.y));
        checks.require(
            bar.is_some() && glyph.is_some() && bar < glyph,
            "the headline bar is drawn over the headline instead of behind it",
            format!(
                "beat {beat}: as indices into the draw order, the bar is {bar:?} and the \
                 headline's first glyph is {glyph:?}; the game submits the glyphs first, so \
                 only PANEL sorting under TEXT can put the bar first"
            ),
        );
        let front = frame
            .covering(headline_at + Vec2::new(0.1, 0.2))
            .into_iter()
            .next();
        checks.require(
            front.is_some_and(|quad| quad.texture == run.font),
            "the headline is not the front-most thing where the game draws it",
            format!(
                "beat {beat}: the front-most quad at the headline's first cell is {:?}",
                front.map(|quad| quad.tint)
            ),
        );
    }
}
