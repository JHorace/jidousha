//! `--verify`: three players play whole matches, the staged scenes ask each
//! decision row directly, and every recorded frame is held to the layout.
//!
//! The verdict line starts `verified `; the indented summary follows; the
//! last frame's transcript is the evidence (`docs/api/jidousha-testing.md`).

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder};

use crate::checks::{Checks, fail, greater};
use crate::draw::{NAMES, palette, ring_segments};
use crate::events::Event;
use crate::players::{Keyboard, Player};
use crate::rules::{self, Ending, Item};
use crate::scenes::{self, tinted_at};
use crate::sim::Match;
use crate::text::{result_lines, status_lines};
use crate::zone::{next_zone, zone_at};
use crate::{HALF_H, HALF_W, WINDOW, camera, config, register};

/// The seed every full match in the run is played on.
pub const SEED: u64 = 7;

/// The longest a match can last: the zone closes at 120 s.
const MAX_TICKS: u64 = 130 * 60;

/// What one played match left behind.
pub struct Run {
    /// The match as it ended.
    pub game: Match,
    /// The first frame drawn while the player was addressing its ball, and
    /// the match it was drawn from.
    pub aiming: Option<(FrameRecord, Match)>,
    /// Frames sampled once a second while play was live.
    pub live: Vec<FrameRecord>,
    /// The frame the result screen was first drawn on.
    pub result: Option<FrameRecord>,
    /// The status bar's extraction promise on the frame before X was pressed.
    pub promise: Option<String>,
    /// How many times the player stood over its resting ball, and how many
    /// of those it shot from.
    pub approaches: (u32, u32),
    /// The planned landings' distances from the safe ring, averaged.
    pub planned_off: f32,
    /// How far shots stopped from where they were planned, averaged.
    pub landed_off: f32,
}

/// Play one whole match on `seed` with `player` at the keyboard.
pub fn play(player: Player, seed: u64, record: bool) -> Run {
    let mut sim = headless(config(), register);
    sim.world_mut().insert_resource(Match::new(seed));
    let mut recorder = record.then(|| FrameRecorder::new(WINDOW));
    let mut keyboard = Keyboard::default();
    let mut run = Run {
        game: Match::new(seed),
        aiming: None,
        live: Vec::new(),
        result: None,
        promise: None,
        approaches: (0, 0),
        planned_off: 0.0,
        landed_off: 0.0,
    };
    let mut was_addressing = false;
    let mut pending: Option<Vec2> = None;
    let (mut planned_sum, mut landed_sum, mut planned_n, mut landed_n) = (0.0, 0.0, 0, 0);
    for _ in 0..MAX_TICKS {
        let intent = match sim.world().find_resource::<Match>() {
            Some(game) if !game.over() => player.decide(game),
            _ => Default::default(),
        };
        if intent.extract
            && let Some(game) = sim.world().find_resource::<Match>()
            && game.pad_underfoot(0).is_some()
        {
            run.promise = Some(status_lines(game)[1].clone());
        }
        let input = keyboard.input_for(intent);
        sim.world_mut().insert_resource(input);
        sim.tick();
        let game = sim.world().resource::<Match>();
        let addressing = game.addressing(0) && game.golfers[0].ending.is_none();
        if addressing && !was_addressing {
            run.approaches.0 += 1;
        }
        was_addressing = addressing;
        if let Some((
            tick,
            Event::Shot {
                who: 0, planned, ..
            },
        )) = game.log.last().copied()
            && tick == game.tick
        {
            run.approaches.1 += 1;
            let (next, _) = next_zone(&game.schedule, game.tick)
                .unwrap_or((zone_at(&game.schedule, game.tick), game.tick));
            planned_sum += ((planned - next.center).length() - next.radius).max(0.0);
            planned_n += 1;
            pending = Some(planned);
        }
        if let Some(planned) = pending
            && game.golfers[0].ball.vel == Vec2::ZERO
        {
            landed_sum += (game.golfers[0].ball.pos - planned).length();
            landed_n += 1;
            pending = None;
        }
        let over = game.over();
        let tick = game.tick;
        if let Some(recorder) = recorder.as_mut() {
            // The first aim with the zone ring on the course: before that the
            // whole course is safe and the ring lies off it.
            let ring_on_course =
                ring_segments(zone_at(&game_schedule(&sim), tick), false).len() >= 8;
            if addressing && ring_on_course && run.aiming.is_none() {
                let snapshot = sim.world().resource::<Match>().clone();
                run.aiming = Some((recorder.draw(&mut sim), snapshot));
            } else if over {
                run.result = Some(recorder.draw(&mut sim));
            } else if tick.is_multiple_of(60) {
                run.live.push(recorder.draw(&mut sim));
            }
        }
        if over {
            break;
        }
    }
    run.game = sim.world().resource::<Match>().clone();
    run.planned_off = planned_sum / planned_n.max(1) as f32;
    run.landed_off = landed_sum / landed_n.max(1) as f32;
    run
}

fn game_schedule(sim: &jidousha::prelude::HeadlessSim) -> crate::zone::Schedule {
    sim.world().resource::<Match>().schedule.clone()
}

/// The player's tokens counted from the log alone, not from its stash.
fn tokens_from_log(game: &Match) -> u32 {
    let mut tokens: i64 = 0;
    for (_, event) in &game.log {
        match *event {
            Event::Sunk { who: 0, .. } => tokens += 3,
            Event::Club { by: 0, stolen, .. } => tokens += i64::from(stolen),
            Event::Club {
                target: 0, stolen, ..
            } => tokens -= i64::from(stolen),
            Event::Eliminated {
                to: Some(0),
                bounty,
                ..
            } => tokens += i64::from(bounty),
            _ => {}
        }
    }
    u32::try_from(tokens).unwrap_or(u32::MAX)
}

/// Row 1, the aiming surface, on the first frame the player addresses.
fn aiming_surface(checks: &mut Checks, frame: &FrameRecord, game: &Match) -> String {
    let tick = game.tick;
    let zone = zone_at(&game.schedule, tick);
    let me = &game.golfers[0];
    let ring_point = |circle: crate::zone::Circle, dashed: bool| {
        ring_segments(circle, dashed)
            .first()
            .map(|(from, to)| (*from + *to) * 0.5)
    };
    let zone_drawn =
        ring_point(zone, false).is_some_and(|at| tinted_at(frame, at, palette::ZONE_RING));
    let next = next_zone(&game.schedule, tick);
    let next_drawn = next
        .and_then(|(circle, _)| ring_point(circle, true))
        .is_some_and(|at| tinted_at(frame, at, palette::NEXT_RING));
    let landing = rules::landing(me.ball.pos, me.aim, me.power, me.item);
    let aim_drawn = tinted_at(frame, (me.ball.pos + landing) * 0.5, palette::AIM);
    let landing_drawn = tinted_at(frame, landing, palette::LANDING);
    let status = status_lines(game)[0].clone();
    let next_radius = next.map_or(-1.0, |(circle, _)| circle.radius);
    // "to r24" or "-> r24": the radius the zone is heading for, in the place
    // the sentence says it is heading.
    let names_next = status.contains(&format!("to r{next_radius:.0}"))
        || status.contains(&format!("-> r{next_radius:.0}"));
    checks.require(
        zone_drawn && next_drawn && aim_drawn && landing_drawn && names_next,
        "the aiming surface is missing something the shot decision needs",
        format!(
            "tick {tick}: zone ring {zone_drawn}, next ring {next_drawn}, aim line {aim_drawn}, \
             landing {landing_drawn}, status names next radius r{next_radius:.0}: {names_next} \
             ({status:?})"
        ),
    );
    format!(
        "tick {tick}: zone r{:.1}, next r{next_radius:.0}, landing ({:.1}, {:.1})",
        zone.radius, landing.x, landing.y
    )
}

/// Every frame on screen, the status bar in its band, and the margin.
fn layout(
    checks: &mut Checks,
    frames: &[&FrameRecord],
    font: jidousha::testing::BackendTextureId,
) -> f32 {
    let view = camera().visible_bounds();
    let band = view.min.y + view.size().y * 0.12;
    let mut clearance = f32::MAX;
    for frame in frames {
        for quad in frame.quads() {
            let bounds = quad.bounds();
            let gap = (bounds.min - view.min).min(view.max - bounds.max);
            // The status band is full-bleed on purpose and flush with three
            // edges; the margin worth printing is everything else's.
            if quad.tint != palette::PANEL {
                clearance = clearance.min(gap.x.min(gap.y));
            }
            if !view.contains_rect(bounds) {
                checks.require(
                    false,
                    "drawn off screen",
                    format!("{bounds:?} against {view:?}"),
                );
            }
        }
        let status_glyphs = frame
            .quads()
            .iter()
            .filter(|quad| quad.texture == font && greater(band, quad.bounds().max.y))
            .count();
        let cleared = frame.plan.clear_color;
        checks.require(
            status_glyphs > 40 && cleared.r.max(cleared.g).max(cleared.b) < 0.25,
            "the status bar is not in the top band, or the course is too bright for the rings",
            format!(
                "{status_glyphs} glyphs inside the top 12% (above y {band:.2}); clear colour {cleared:?}"
            ),
        );
    }
    clearance
}

pub fn run() -> ExitCode {
    let mut checks = Checks::default();
    let good = play(Player::Good, SEED, true);
    let chaser = play(Player::Chaser, SEED, false);
    let idle = play(Player::Idle, SEED, false);
    let font = FrameRecorder::new(WINDOW).font_texture();

    // One full match, end to end, to its result screen.
    let ending = good.game.golfers[0].ending;
    let Some(result) = good.result.as_ref() else {
        fail(
            "the good player's match never reached its result screen",
            &format!("tick {} of {MAX_TICKS}", good.game.tick),
        );
    };
    let lines = result_lines(&good.game);
    checks.require(
        matches!(ending, Some((Ending::Extracted, _, _))),
        "the good player did not extract",
        format!("ending {ending:?}"),
    );
    // Row 4: the promise on the status bar is what the result screen kept.
    let promise = good.promise.clone().unwrap_or_default();
    let promised = promise
        .strip_prefix("EXTRACT keeps ")
        .and_then(|rest| rest.split(" (").next())
        .unwrap_or("");
    let kept = lines.get(1).cloned().unwrap_or_default();
    let counted = tokens_from_log(&good.game);
    let item = good
        .game
        .log
        .iter()
        .rev()
        .find_map(|(_, event)| match event {
            Event::Took { who: 0, item } => Some(*item),
            _ => None,
        });
    let truth = rules::Kept {
        tokens: counted,
        item,
    }
    .line();
    checks.require(
        !promised.is_empty() && kept == format!("kept {promised}") && promised == truth,
        "the result screen did not keep exactly what the status bar promised",
        format!("promised {promised:?}, result {kept:?}, counted from the log {truth:?}"),
    );
    let result_glyphs = result
        .quads()
        .iter()
        .filter(|quad| quad.texture == font)
        .count();

    let aim = match good.aiming.as_ref() {
        Some((frame, game)) => aiming_surface(&mut checks, frame, game),
        None => {
            checks.require(
                false,
                "the good player never addressed its ball",
                String::new(),
            );
            String::new()
        }
    };
    let grace = scenes::grace(&mut checks);
    let club = scenes::club(&mut checks);
    let gear = scenes::equipment(&mut checks);
    let contracts = scenes::equipment_contracts(&mut checks);

    // Three players: the good one leaves with something, the idle one loses.
    let idle_ending = idle.game.golfers[0].ending.map(|(ending, _, _)| ending);
    checks.require(
        matches!(idle_ending, Some(Ending::Eliminated | Ending::Outlasted)),
        "the idle player was not beaten",
        format!("idle ending {idle_ending:?}"),
    );
    let contact = good
        .game
        .log
        .iter()
        .filter(|(_, event)| matches!(event, Event::Club { .. } | Event::Strike { .. }))
        .count();
    checks.require(
        contact > 0,
        "no NPC clubbed or struck anything in a whole match",
        format!("{contact} contacts in {} ticks", good.game.tick),
    );

    let mut frames: Vec<&FrameRecord> = good.live.iter().collect();
    frames.push(result);
    if let Some((frame, _)) = good.aiming.as_ref() {
        frames.push(frame);
    }
    let clearance = layout(&mut checks, &frames, font);

    let mut strings: Vec<String> = status_lines(&good.game).to_vec();
    strings.extend(result_lines(&good.game));
    strings.extend(NAMES.iter().map(|name| (*name).to_owned()));
    strings.extend([Item::Heavy, Item::Helmet, Item::Driver].map(rules::describe));
    if let Some(promise) = &good.promise {
        strings.push(promise.clone());
    }
    for text in &strings {
        let stray = text.chars().find(|glyph| !(' '..='~').contains(glyph));
        checks.require(
            stray.is_none(),
            "a string the game draws has a character the font cannot draw",
            format!("{text:?} contains {stray:?}"),
        );
    }

    let order = headless(config(), register).schedule_debug();
    let read = order.find("read_the_player");
    let step = order.find("step_the_match");
    checks.require(
        read.is_some() && step.is_some() && read < step,
        "the match steps before it reads this tick's keys",
        format!("read_the_player at {read:?}, step_the_match at {step:?}"),
    );
    checks.require(
        HALF_W > 31.0 && HALF_H > 15.0,
        "the course does not fit the camera",
        format!("camera half extents {HALF_W:.2} x {HALF_H:.2}"),
    );

    let picture = good.aiming.as_ref().map_or(result, |(frame, _)| frame);
    let captured = crate::capture::capture_a_frame(&mut checks, picture, font, "brolf_r3v1");
    let verdict = checks.verdict();
    let ending_of = |run: &Run| {
        run.game.golfers[0]
            .ending
            .map_or("none".to_owned(), |(ending, kept, tick)| {
                format!("{} on tick {tick}, kept {}", ending.name(), kept.line())
            })
    };
    println!(
        "verified brolf_r3v1 seed {SEED}: {} checks, {} failed",
        checks.ran,
        checks.failed()
    );
    for (player, run) in [
        (Player::Good, &good),
        (Player::Chaser, &chaser),
        (Player::Idle, &idle),
    ] {
        let name = player.name();
        println!(
            "  {name}: {} | stood over its ball {} times, shot {} | planned landings {:.2} outside the next ring | landed {:.2} from plan",
            ending_of(run),
            run.approaches.0,
            run.approaches.1,
            run.planned_off,
            run.landed_off
        );
    }
    let npcs: Vec<String> = good.game.golfers[1..]
        .iter()
        .zip(&NAMES[1..])
        .map(|(golfer, name)| {
            let state = golfer
                .ending
                .map_or("in".to_owned(), |(ending, _, _)| ending.name().to_owned());
            format!("{name} {state}")
        })
        .collect();
    println!(
        "  npcs at the good player's end: {}; {contact} contacts",
        npcs.join(", ")
    );
    println!("  row 1 aiming: {aim}");
    println!("  row 1 grace: {grace}");
    println!("  row 2 club: {club}");
    println!("  row 3 equipment: {gear}");
    println!("  equipment contracts: {contracts}");
    println!("  row 4 extract: promised {promised:?}, result {kept:?}, log says {truth:?}");
    println!("  result screen: {result_glyphs} glyphs");
    println!(
        "  closest quad to the edge, the full-bleed status band aside: {clearance:.2} world units"
    );
    println!("  capture: {captured}");
    print!("{}", result.transcript());
    verdict
}
