//! The `--verify` mode: the decision gates, the match, the sweep, the staged
//! screens, and the checks that hold of every frame the run drew.
//!
//! `tools/verify stack_card_game_r3v2` runs this. The summary under the
//! `verified` line carries one line per gate; the three decision rows are
//! named `decision 1`, `decision 2` and `decision 3` (DESIGN.md, Gates).

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::ui::*;

use crate::cards::Card;
use crate::checks::{Album, Checks, Driver};
use crate::decisions::{respond_table, table};
use crate::duel::{Outcome, Phase};
use crate::rules::Side;
use crate::screen::{FLOORS, Flat, banner_line, layers, palette, screen};
use crate::{Choosing, Flow};

/// The verdict line's name for the game.
const NAME: &str = "stack_card_game_r3v2";

pub fn run() -> ExitCode {
    let mut checks = Checks::default();
    let mut album = Album::default();
    let mut summary = vec![
        crate::decisions::respond(&mut checks, &mut album),
        crate::decisions::pass(&mut checks, &mut album),
        crate::decisions::target(&mut checks, &mut album),
        crate::sweep::the_match(&mut checks, &mut album),
    ];
    summary.extend(crate::sweep::three_players(&mut checks));
    summary.push(staged_screens(&mut checks, &mut album));
    summary.push(schedule(&mut checks));
    summary.push(bounds(&mut checks, &album));
    summary.push(floors(&mut checks, &album));
    let window = &album.shots[0];
    summary.push(clear_colour(&mut checks, &window.frame));
    let captured = crate::capture::capture_a_frame(&mut checks, &window.frame, window.font);
    let verdict = checks.verdict();
    println!(
        "verified {NAME}: {} frames judged, {} checks failed",
        album.shots.len(),
        checks.failed()
    );
    for line in summary {
        println!("  {line}");
    }
    println!("  capture: {captured}");
    print!("{}", window.frame.transcript());
    verdict
}

/// A finished match with these lives.
fn ended(outcome: Outcome, life: [i32; 2]) -> Flow {
    let mut flow = respond_table();
    flow.duel.stack.clear();
    flow.duel.turn = 12;
    flow.duel.phase = Phase::Over(outcome);
    for side in Side::BOTH {
        flow.duel.seats[side.index()].life = life[side.index()];
    }
    flow.duel.log = vec!["Bolt: Rival 3>0".to_owned(), "Blast: You 15>10".to_owned()];
    flow
}

/// G8: the screens the run never reaches, drawn and judged; and the banners.
fn staged_screens(checks: &mut Checks, album: &mut Album) -> String {
    use Card::*;
    use Side::*;
    let full = table(
        &[Redirect, Counter, Delay, Copy, Blast, Mend],
        3,
        &[
            (10, Redirect, Rival, Rival, Some(9)),
            (9, Copy, You, You, Some(8)),
            (8, Delay, You, You, Some(1)),
            (7, Counter, Rival, Rival, Some(6)),
            (6, Blast, Rival, You, None),
            (5, Bolt, You, Rival, None),
            (4, Mend, Rival, Rival, None),
            (3, Blast, You, Rival, None),
            (2, Bolt, Rival, You, None),
            (1, Bolt, You, Rival, None),
        ],
    );
    let mut choosing = table(
        &[Delay, Redirect],
        0,
        &[
            (3, Counter, Rival, Rival, Some(2)),
            (2, Bolt, You, Rival, None),
            (1, Mend, Rival, Rival, None),
        ],
    );
    choosing.choosing = Some(Choosing { slot: 1, cursor: 1 });
    let screens = [
        ("staged: you win", ended(Outcome::YouWin, [12, 0])),
        ("staged: rival wins", ended(Outcome::RivalWins, [0, 7])),
        ("staged: draw", ended(Outcome::Draw, [3, 3])),
        ("staged: a full stack and a full hand", full),
        ("staged: aiming a Redirect", choosing),
    ];
    let mut banners = Vec::new();
    for (label, flow) in screens {
        let mut driver = Driver::new(Some(flow.clone()), 7);
        let frame = driver.shoot(album, label);
        let font = album.shots[album.shots.len() - 1].font;
        let Some(line) = banner_line(&flow.duel) else {
            continue;
        };
        let panel = screen(&flow);
        let run = panel.runs.iter().find(|run| run.text == line);
        let centre = run.map_or(Vec2::ZERO, |run| run.bounds().center());
        let front = frame.covering(centre).into_iter().next();
        checks.require(
            run.is_some_and(|run| run.style.depth.layer == layers::BANNER)
                && front.is_some_and(|quad| quad.texture == font),
            "staged screens: the banner is not the front-most thing at its own centre",
            format!(
                "{label}: banner {line:?} at {centre:?}; in front there is {:?}",
                front.map(|quad| (quad.texture, quad.tint))
            ),
        );
        banners.push(line);
    }
    let words: Vec<&str> = banners
        .iter()
        .map(|line| line.split("  ").next().unwrap_or(""))
        .collect();
    checks.require(
        words == ["YOU WIN", "RIVAL WINS", "DRAW"],
        "staged screens: the three end banners are not three different screens",
        format!("banners {banners:?}"),
    );
    format!("staged screens: banners {banners:?}; a 10-item stack, a 6-card hand, aiming")
}

/// G9: the order the systems run in.
fn schedule(checks: &mut Checks) -> String {
    let order = Driver::new(None, 7).schedule();
    let startup = order.find("set_the_table");
    let keys = order.find("read_player_input");
    let rival = order.find("let_the_rival_act");
    let update = order.find("Update");
    let ok = matches!((startup, update, keys, rival),
        (Some(s), Some(u), Some(k), Some(r)) if s < u && u < k && k < r);
    checks.require(
        ok,
        "the schedule: the player's keys are not read before the Rival acts",
        format!(
            "set_the_table {startup:?}, Update {update:?}, read_player_input {keys:?}, \
                 let_the_rival_act {rival:?}"
        ),
    );
    "schedule: set_the_table in Startup; read_player_input before let_the_rival_act".to_owned()
}

/// G1: nothing drawn outside the camera, on any frame; and by how much.
fn bounds(checks: &mut Checks, album: &Album) -> String {
    let camera = Driver::new(None, 7).camera();
    let view = camera.visible_bounds();
    let mut clearance = f32::MAX;
    for shot in &album.shots {
        let quads = shot.frame.quads();
        let outside: Vec<Rect> = quads
            .iter()
            .map(|quad| quad.bounds())
            .filter(|bounds| !view.contains_rect(*bounds))
            .collect();
        checks.require(
            outside.is_empty(),
            "bounds: something was drawn outside what the camera shows",
            format!(
                "{}: {} quads outside {view:?}, first {:?}",
                shot.label,
                outside.len(),
                outside.first()
            ),
        );
        for quad in &quads {
            let bounds = quad.bounds();
            let gap = (bounds.min - view.min).min(view.max - bounds.max);
            clearance = clearance.min(gap.x.min(gap.y));
        }
    }
    format!(
        "bounds: {} frames inside {:.0}x{:.0}; closest quad to the edge {clearance:.2} units",
        album.shots.len(),
        view.size().x,
        view.size().y
    )
}

/// G2: the kit's floors over every frame, and the overlap floor seen to bite.
fn floors(checks: &mut Checks, album: &Album) -> String {
    let view = Driver::new(None, 7).camera().visible_bounds();
    let mut rows = 0;
    for shot in &album.shots {
        let panel = screen(&shot.flow);
        rows += panel.runs.len();
        let mut breaches = judge_panel(&panel, &FLOORS, &[], &[]);
        breaches.extend(judge_frame(&panel, &shot.frame, shot.font, &Flat, view));
        breaches.extend(frame_text_floor(&shot.frame, shot.font, FLOORS.min_text));
        for breach in breaches {
            checks.require(
                false,
                "floors: a screen breaks a readability floor",
                format!("{}: {} - {}", shot.label, breach.what, breach.detail),
            );
        }
        let stray: Vec<&str> = panel
            .all_strings()
            .filter(|text| !text.chars().all(|c| (' '..='~').contains(&c)))
            .collect();
        let cut: Vec<&str> = panel
            .all_strings()
            .filter(|text| text.ends_with("..."))
            .collect();
        checks.require(
            stray.is_empty() && cut.is_empty(),
            "floors: a row has a character the font cannot draw, or was clipped",
            format!("{}: unprintable {stray:?}, clipped {cut:?}", shot.label),
        );
    }
    let mut staged: Panel<crate::screen::Art> = Panel::default();
    let ink = crate::screen::style(14.0, palette::INK);
    staged.text(TextRun::new(
        Vec2::new(28.0, 432.0),
        "Bolt: Rival 15>12",
        ink,
    ));
    staged.text(TextRun::new(
        Vec2::new(28.0, 434.0),
        "Blast: You 15>10",
        ink,
    ));
    let bites = judge_panel(&staged, &FLOORS, &[], &[])
        .iter()
        .any(|breach| breach.what == "two rows of chrome text overlap");
    checks.require(
        bites,
        "floors: the overlap floor does not fail on two rows two units apart",
        "judged a staged panel of two log rows at y 432 and 434".to_owned(),
    );
    format!(
        "floors: {} frames, {rows} rows judged; the overlap floor bites on a staged pair",
        album.shots.len()
    )
}

/// G11: the clear colour, against the constant and against the requirement.
fn clear_colour(checks: &mut Checks, frame: &jidousha::testing::FrameRecord) -> String {
    let cleared = frame.plan.clear_color;
    let brightest = cleared.r.max(cleared.g).max(cleared.b);
    checks.require(
        cleared == palette::TABLE,
        "clear colour: the table is not cleared to the palette's colour",
        format!("cleared {cleared:?}, palette {:?}", palette::TABLE),
    );
    checks.require(
        brightest < 0.25 && cleared.a > 0.99,
        "clear colour: the table is not dark enough for the white rows",
        format!("brightest channel {brightest:.3} at alpha {:.2}", cleared.a),
    );
    format!("clear colour: brightest channel {brightest:.2}")
}
