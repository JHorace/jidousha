//! `--verify`: three players play whole runs, the staged scenes ask each
//! decision row directly, and every recorded screen is held to the kit's
//! floors.
//!
//! The verdict line starts `verified `; the indented summary follows; the
//! last frame's transcript is the evidence.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder};
use jidousha::ui::{Floors, Panel, frame_text_floor, judge_frame, judge_panel};

use crate::checks::{Checks, fail};
use crate::content::Theme;
use crate::players::{Player, Typist};
use crate::rules::{self, Consequence};
use crate::scenes::{self, screen_text, type_command};
use crate::screens::{Art, DESIGN, OneToOne, palette, screen};
use crate::sim::{Event, Game, Phase, Serve, Who};
use crate::{Command, WINDOW, camera, config, register};

/// The seed every full run is played on.
pub const SEED: u64 = 3;

/// The longest a run can take, in ticks: far more than seven days of typing.
const MAX_TICKS: u64 = 20_000;

/// The floors every screen is judged against.
pub const FLOORS: Floors = Floors {
    // A literal, not `SMALL`: a floor that moves with the size it judges
    // cannot see that size shrink.
    min_text: 12.0,
    chrome: Rect {
        min: Vec2::ZERO,
        max: DESIGN,
    },
    world: Rect {
        min: Vec2::ZERO,
        max: DESIGN,
    },
};

/// What one played run left behind.
pub struct Run {
    /// The run as it ended.
    pub game: Game,
    /// A frame per screen change, with the panel it was drawn from.
    pub shots: Vec<(FrameRecord, Panel<Art>)>,
    /// The most nemeses active at once during service.
    pub most_active: usize,
}

/// Play one whole run with `player` at the keyboard.
pub fn play(player: Player, seed: u64, record: bool) -> Run {
    let mut sim = headless(config(), register);
    sim.world_mut().insert_resource(Game::new(seed));
    let mut recorder = record.then(|| FrameRecorder::new(WINDOW));
    let mut typist = Typist::default();
    let mut shots = Vec::new();
    let mut most_active = 0;
    let mut last_screen = (0, 0, Phase::Ledger, 99);
    for _ in 0..MAX_TICKS {
        let command = match sim.world().find_resource::<Game>() {
            Some(game) if !typist.releasing() => player.decide(game),
            _ => None,
        };
        let input = typist.input_for(command);
        sim.world_mut().insert_resource(input);
        sim.tick();
        let game = sim.world().resource::<Game>();
        if game.phase == Phase::Service {
            most_active = most_active.max(game.active().len());
        }
        let here = (game.day, game.round, game.phase, game.queue.len());
        let over = matches!(game.phase, Phase::Over(_));
        if here != last_screen
            && let Some(recorder) = recorder.as_mut()
        {
            let panel = screen(sim.world().resource::<Game>());
            shots.push((recorder.draw(&mut sim), panel));
            last_screen = here;
        }
        if over {
            break;
        }
    }
    Run {
        game: sim.world().resource::<Game>().clone(),
        shots,
        most_active,
    }
}

/// Row 1: on seed 3 the good player meets a round with an order whose unmet
/// outcome spawns; the row says so, the order is left unmet, and a nemesis
/// is born in that theme with a theme demand of 4 when they come back.
fn spawn_row(checks: &mut Checks) -> String {
    let mut sim = scenes::staged(Game::new(SEED));
    let mut typist = Typist::default();
    let mut found: Option<(usize, Theme, u8)> = None;
    for _ in 0..4000 {
        let game = sim.world().resource::<Game>();
        if matches!(game.phase, Phase::Over(_)) {
            break;
        }
        if game.phase == Phase::Service && game.queue.iter().all(|order| order.serve == Serve::Skip)
        {
            let risky = (0..game.queue.len()).find(|&index| {
                let order = game.queue[index];
                rules::outcome(game, &order, Serve::Skip).consequence == Consequence::Spawn
            });
            if let Some(index) = risky {
                let order = game.queue[index];
                if let Some((theme, severity)) = rules::outcome(game, &order, Serve::Skip).failure {
                    found = Some((index, theme, severity));
                    break;
                }
            }
        }
        let command = if typist.releasing() {
            None
        } else {
            Player::Good.decide(game)
        };
        let input = typist.input_for(command);
        sim.world_mut().insert_resource(input);
        sim.tick();
    }
    let Some((index, theme, severity)) = found else {
        checks.require(
            false,
            "no order on seed 3 ever risked a spawn",
            String::new(),
        );
        return "none found".to_owned();
    };
    scenes::photograph(&mut sim, checks, "the spawn warning");
    let text = screen_text(sim.world().resource::<Game>());
    let warning = format!("{}. SKIP", index + 1);
    let says = format!("if unmet: sev {severity} {}", theme.name());
    checks.require(
        text.contains(&warning) && text.contains(&says) && text.contains("SPAWNS A NEMESIS"),
        "the spawn warning is not on the order before the choice",
        format!(
            "want {says:?} and SPAWNS A NEMESIS on row {}; screen:\n{text}",
            index + 1
        ),
    );
    let born_before = sim.world().resource::<Game>().nemeses.len();
    type_command(&mut sim, &mut typist, Command::Go);
    let game = sim.world().resource::<Game>();
    let born = game.nemeses.get(born_before).cloned();
    let titled = born.as_ref().is_some_and(|nemesis| {
        nemesis.theme == theme
            && theme
                .titles()
                .iter()
                .any(|title| nemesis.title.starts_with(title))
    });
    checks.require(
        titled,
        "leaving the order unmet did not spawn a nemesis titled for its theme",
        format!("theme {theme:?}, new nemesis {born:?}"),
    );
    // Play on to their first visit and read their demand.
    let target = born_before;
    let mut demand = None;
    for _ in 0..4000 {
        let game = sim.world().resource::<Game>();
        if let Some(order) = game
            .queue
            .iter()
            .find(|order| order.who == Who::Nemesis(target))
        {
            demand = Some(order.demands[theme.index()]);
            break;
        }
        if matches!(game.phase, Phase::Over(_)) {
            break;
        }
        let command = if typist.releasing() {
            None
        } else {
            Player::Good.decide(game)
        };
        let input = typist.input_for(command);
        sim.world_mut().insert_resource(input);
        sim.tick();
    }
    checks.require(
        demand == Some(4),
        "the new nemesis does not come back with the sensitivity in their theme",
        format!("theme demand on their first visit: {demand:?}, want Some(4)"),
    );
    format!(
        "row {} {} sev {severity} -> {} -> back demanding {:?}",
        index + 1,
        theme.name(),
        born.map_or("nobody".to_owned(), |nemesis| nemesis.title),
        demand
    )
}

/// Every shot through the kit's floors and the camera.
fn floors(checks: &mut Checks, shots: &[(FrameRecord, Panel<Art>)]) -> f32 {
    let font = FrameRecorder::new(WINDOW).font_texture();
    let view = camera().visible_bounds();
    let mut clearance = f32::MAX;
    for (frame, panel) in shots {
        for breach in judge_panel(panel, &FLOORS, &[], &[]) {
            checks.require(false, breach.what, breach.detail);
        }
        for breach in judge_frame(panel, frame, font, &OneToOne, view) {
            checks.require(false, breach.what, breach.detail);
        }
        for breach in frame_text_floor(frame, font, FLOORS.min_text) {
            checks.require(false, breach.what, breach.detail);
        }
        let mut off = Vec::new();
        for quad in frame.quads() {
            let bounds = quad.bounds();
            if !view.contains_rect(bounds) {
                off.push(bounds);
            }
            let gap = (bounds.min - view.min).min(view.max - bounds.max);
            clearance = clearance.min(gap.x.min(gap.y));
        }
        checks.require(
            off.is_empty(),
            "drawn off screen",
            format!(
                "{} quads outside {view:?}, first {:?}",
                off.len(),
                off.first()
            ),
        );
        let clipped: Vec<&str> = panel
            .all_strings()
            .filter(|text| text.ends_with("..."))
            .collect();
        checks.require(
            clipped.is_empty(),
            "a row on screen was cut short, hiding what it says",
            format!("{clipped:?}"),
        );
        let stray: Vec<&str> = panel
            .all_strings()
            .filter(|text| text.chars().any(|glyph| !(' '..='~').contains(&glyph)))
            .collect();
        checks.require(
            stray.is_empty(),
            "a string the game draws has a character the font cannot draw",
            format!("{stray:?}"),
        );
        let cleared = frame.plan.clear_color;
        checks.require(
            cleared == palette::FLOOR && cleared.r.max(cleared.g).max(cleared.b) < 0.2,
            "the floor is not dark enough for light text",
            format!("clear colour {cleared:?}"),
        );
    }
    clearance
}

pub fn run() -> ExitCode {
    let mut checks = Checks::default();
    let good = play(Player::Good, SEED, true);
    let first = play(Player::FirstTimer, SEED, false);
    let idle = play(Player::Idle, SEED, true);
    let font = FrameRecorder::new(WINDOW).font_texture();

    let ending = |run: &Run| match run.game.phase {
        Phase::Over(true) => format!("WON with ${}", run.game.money),
        Phase::Over(false) => format!("LOST on day {} with ${}", run.game.day, run.game.money),
        _ => format!("unfinished on day {}", run.game.day),
    };
    checks.require(
        good.game.phase == Phase::Over(true),
        "the good player did not survive seven days",
        ending(&good),
    );
    checks.require(
        idle.game.phase == Phase::Over(false),
        "the idle player was not closed down",
        ending(&idle),
    );
    let crowd = good
        .most_active
        .max(first.most_active)
        .max(idle.most_active);
    checks.require(
        crowd >= 2,
        "no run ever had two nemeses at once",
        format!(
            "most at once: good {}, first-timer {}, idle {}",
            good.most_active, first.most_active, idle.most_active
        ),
    );
    let most = good
        .most_active
        .max(first.most_active)
        .max(idle.most_active);
    checks.require(
        most <= 3,
        "more than three nemeses were active at once",
        format!("most at once: {most}"),
    );
    let Some((last, _)) = good.shots.last() else {
        fail("the good run recorded no frames", "the recorder was on");
    };

    let spawn = spawn_row(&mut checks);
    let tally = scenes::tally(&mut checks);
    let (overwhelm, overwhelm_frame) = scenes::overwhelm(&mut checks);
    let following = scenes::following(&mut checks);
    let spend = scenes::spend(&mut checks);
    let night = scenes::night(&mut checks);

    let mut shots: Vec<(FrameRecord, Panel<Art>)> = good.shots.clone();
    shots.extend(idle.shots.iter().cloned());
    let clearance = floors(&mut checks, &shots);

    // The nemesis card, mid-decision, is the picture worth having.
    let picture = &overwhelm_frame;
    let captured =
        crate::capture::capture_a_frame(&mut checks, picture, font, "restaurant_nemesis_r3v1");
    let verdict = checks.verdict();
    println!(
        "verified restaurant_nemesis_r3v1 seed {SEED}: {} checks, {} failed",
        checks.ran,
        checks.failed()
    );
    for (player, run) in [
        (Player::Good, &good),
        (Player::FirstTimer, &first),
        (Player::Idle, &idle),
    ] {
        let made = run.game.nemeses.len();
        let beaten = run.game.nemeses.iter().filter(|n| !n.active()).count();
        let served = run
            .game
            .log
            .iter()
            .filter(|(_, _, event)| {
                matches!(
                    event,
                    Event::Served {
                        satisfied: true,
                        ..
                    }
                )
            })
            .count();
        println!(
            "  {}: {} | {served} orders satisfied | nemeses made {made}, beaten {beaten}, most at once {}",
            player.name(),
            ending(run),
            run.most_active
        );
    }
    println!("  row 1 spawn: {spawn}");
    println!("  row 2 tally: {tally}");
    println!("  row 2 overwhelm: {overwhelm}");
    println!("  row 2 following: {following}");
    println!("  row 3 spend: {spend}");
    println!("  overnight: {night}");
    println!("  screens judged: {}", shots.len());
    println!("  closest quad to the edge: {clearance:.2} design units");
    println!("  capture: {captured}");
    print!("{}", last.transcript());
    verdict
}
