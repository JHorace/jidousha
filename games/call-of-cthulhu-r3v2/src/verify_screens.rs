//! The screen gates: G3 the call screen (decision row 1), G4 the morning
//! screen (decision row 2), G6 the floors and the screens play never reaches,
//! G8 the sanity bar and G9 the clear colour.

use crate::DIGITS;
use crate::beings::BeingId;
use crate::checks::{Checks, greater, near};
use crate::flow::{
    Choice, Game, MorningAction, Night, Screen, call_options, effect_line, morning_options,
    option_text, step,
};
use crate::players::{drive, game_of, sim_for};
use crate::rules::{AnswerKind, PATIENCE, answer_outcome, tonight};
use crate::screen::{Art, BAR_AT, BAR_H, FLOORS, Identity, palette, screen};
use crate::verify::{Frames, current_block, joined, view};
use jidousha::prelude::*;
use jidousha::testing::{FrameRecorder, InputScript, find_bounds};
use jidousha::ui::{Panel, TextRun, frame_text_floor, judge_frame, judge_panel};

/// A being's drain, as a shipped literal rather than read off the data under test.
fn drain(being: BeingId) -> u32 {
    match being {
        BeingId::Cthulhu => 6,
        BeingId::Nyarlathotep => 4,
        BeingId::Hastur => 5,
    }
}

/// G3: the call screen shows the question, every option, sanity, temper and
/// the known lore before the player answers, and the answer costs exactly what
/// `answer_outcome` says.
pub(crate) fn call_screen(checks: &mut Checks, recorder: &mut FrameRecorder, frames: &mut Frames) {
    let mut sim = sim_for(1928);
    let start = game_of(&sim);
    let being = start.plan().calls[0].being;
    let study = morning_options(&start)
        .iter()
        .position(|a| *a == MorningAction::Study(being));
    let mut preview = start.clone();
    let _ = step(&mut preview, Choice::Pick(study.unwrap_or(0)));
    let wrong = call_options(&preview)
        .iter()
        .position(|(kind, _)| *kind == AnswerKind::Wrong);
    let (Some(study), Some(wrong)) = (study, wrong) else {
        checks.require(
            false,
            "the call-screen check could not find its two options",
            format!("study {study:?}, wrong {wrong:?}"),
        );
        return;
    };
    let script = InputScript::new()
        .press(DIGITS[study], 5)
        .press(DIGITS[wrong], 8);
    let mut shown = None;
    for tick in 2..=8 {
        sim.world_mut()
            .insert_resource(Input::new(script.snapshot_at(tick)));
        sim.tick();
        if tick == 6 {
            let game = game_of(&sim);
            let frame = recorder.draw(&mut sim);
            frames.push((screen(&game), frame.clone()));
            shown = Some((game, frame));
        }
    }
    let Some((game, frame)) = shown else {
        return;
    };
    let block = current_block(&game.transcript);
    let text = joined(&block);
    let data = being.being();
    let options = call_options(&game);
    let mut missing: Vec<String> = Vec::new();
    let mut want = |needle: String| {
        if !text.contains(&needle) {
            missing.push(needle);
        }
    };
    want(data.questions[0].ask.to_owned());
    for (index, (kind, reply)) in options.iter().enumerate() {
        want(option_text(index, *kind, reply));
    }
    want("sanity 90/90".to_owned());
    want("temper 0/3".to_owned());
    want("costs 1 sanity".to_owned());
    want(format!("any other costs {}", drain(being)));
    want(format!("offends it costs {} and", drain(being) * 2));
    let knows = text.find("what you know of");
    let fact = text.find(data.questions[0].fact);
    checks.require(
        missing.is_empty() && knows.is_some() && fact.is_some() && knows < fact,
        "the call screen does not show what the answer turns on",
        format!(
            "{being:?}: missing {missing:?}; 'what you know of' at {knows:?}, the fact at {fact:?}; screen: {block:?}"
        ),
    );
    checks.require(
        options.len() == 4 && block.iter().any(|row| row.starts_with("1. * ")),
        "the studied lore reply is not listed first and starred",
        format!("{} options: {options:?}", options.len()),
    );
    let breaches = judge_frame(
        &screen(&game),
        &frame,
        recorder.font_texture(),
        &Identity,
        view(),
    );
    checks.require(
        breaches.is_empty(),
        "a row of the call screen is not on the recorded frame",
        format!("{breaches:?}"),
    );

    let after = game_of(&sim);
    let outcome = answer_outcome(being, 0, false, AnswerKind::Wrong);
    let temper = after.tempers[being.index()];
    checks.require(
        after.sanity == 90 - outcome.sanity_cost as i32
            && temper == outcome.temper_delta
            && after
                .last_line
                .contains(&format!("-{} sanity", outcome.sanity_cost)),
        "the wrong reply did not cost what answer_outcome said",
        format!(
            "sanity {} (want {}), temper {temper} (want {}), reaction {:?}",
            after.sanity,
            90 - outcome.sanity_cost as i32,
            outcome.temper_delta,
            after.last_line
        ),
    );
}

/// What one morning-1 choice led to.
struct Branch {
    morning: String,
    effect: String,
    game: Game,
}

/// Drive seed 1928 with `action` on morning 1, to the first exchange.
fn branch(action: MorningAction) -> Branch {
    let start = game_of(&sim_for(1928));
    let effect = effect_line(&start, action);
    let pick = morning_options(&start)
        .iter()
        .position(|a| *a == action)
        .unwrap_or(0);
    let run = drive(
        1928,
        &mut |_| Choice::Pick(pick),
        &|game| game.screen == Screen::Exchange,
        None,
    );
    let chosen = run
        .game
        .transcript
        .iter()
        .position(|line| line.starts_with("> "))
        .unwrap_or(0);
    Branch {
        morning: joined(&current_block(&run.game.transcript[..chosen])),
        effect,
        game: run.game,
    }
}

/// G4: three runs differing only in the morning-1 choice; each option's
/// stated effect is on screen at choosing, and the night differs as stated.
pub(crate) fn morning_screen(checks: &mut Checks) {
    let start = game_of(&sim_for(1928));
    let x = start.plan().calls[0].being;
    let y = BeingId::ALL[(x.index() + 1) % 3];
    let d = drain(x);
    let a = branch(MorningAction::Study(x));
    let b = branch(MorningAction::Meditate);
    let c = branch(MorningAction::Cult(y));
    let expected = [
        (&a, format!("question 1 costs 1, not {d}")),
        (&b, format!("{} {d}->{}", x.name(), d - 1)),
        (
            &c,
            format!("{} calls tonight instead of {}", y.name(), x.name()),
        ),
    ];
    for (run, needle) in expected {
        checks.require(
            run.effect.contains(&needle) && run.morning.contains(&run.effect),
            "a morning option's effect on tonight is not stated on the option",
            format!(
                "wanted {needle:?} in the effect line {:?}, and the line on the morning screen: {:?}",
                run.effect, run.morning
            ),
        );
    }
    let night = |run: &Branch| {
        let text = joined(&current_block(&run.game.transcript));
        (run.game.caller(), call_options(&run.game).len(), text)
    };
    let (a_caller, a_count, a_text) = night(&a);
    let (b_caller, b_count, b_text) = night(&b);
    let (c_caller, c_count, _) = night(&c);
    checks.require(
        a_caller == Some(x)
            && a_count == 4
            && a_text.contains("1. * ")
            && a_text.contains(&format!("each exchange: -{d} ")),
        "studying did not put the lore reply on tonight's first question",
        format!("caller {a_caller:?}, {a_count} options; screen {a_text:?}"),
    );
    checks.require(
        b_caller == Some(x)
            && b_count == 3
            && b_text.contains(&format!("each exchange: -{} ", d - 1)),
        "steadying the nerves did not make tonight's exchanges cost one less",
        format!("caller {b_caller:?}, {b_count} options; screen {b_text:?}"),
    );
    checks.require(
        c_caller == Some(y) && c_count == 3,
        "the cult visit did not draw its being's call tonight",
        format!("wanted {y:?} with 3 options, got {c_caller:?} with {c_count}"),
    );
}

/// A game staged on a call with `being`, every secret known.
fn staged_call(being: BeingId, exchange: u8, last_line: &str) -> Game {
    let mut game = Game::new([
        being,
        BeingId::ALL[(being.index() + 1) % 3],
        BeingId::ALL[(being.index() + 2) % 3],
    ]);
    game.known = [[true; 3]; 3];
    game.day = 4;
    game.screen = Screen::Exchange;
    game.last_line = last_line.to_owned();
    let mut plan = tonight(4, game.rotation, game.tempers, false, None);
    plan.calls.swap(0, 1);
    plan.calls[0].being = being;
    game.night = Some(Night {
        plan,
        call: 0,
        exchange,
        patience: PATIENCE,
        summary: Vec::new(),
        call_start: game.sanity,
    });
    game
}

/// The screens a run never reaches, staged.
fn staged() -> Vec<Game> {
    let scream = "Nyarlathotep SCREAMS. -12 sanity and -25 more, temper 3/3. The line goes dead.";
    let mut games = Vec::new();
    for being in BeingId::ALL {
        games.push(staged_call(being, 0, scream));
        games.push(staged_call(being, 1, scream));
        games.push(staged_call(being, 2, scream));
    }
    let mut dawn = staged_call(BeingId::Cthulhu, 0, "");
    dawn.screen = Screen::Dawn;
    if let Some(night) = dawn.night.as_mut() {
        night.summary = vec![(BeingId::Cthulhu, 6, 36), (BeingId::Nyarlathotep, 4, 16)];
    }
    games.push(dawn);
    let mut won = staged_call(BeingId::Hastur, 0, "");
    won.screen = Screen::End { won: true };
    games.push(won);
    let mut lost = staged_call(BeingId::Nyarlathotep, 0, "");
    lost.screen = Screen::End { won: false };
    lost.sanity = -7;
    games.push(lost);
    let mut late = staged_call(BeingId::Hastur, 0, "");
    late.day = 5;
    late.screen = Screen::Morning;
    late.night = None;
    late.tempers = [3, 2, 1];
    games.push(late);
    games
}

/// G6: every recorded and staged frame against the floors; returns the
/// clearance between everything drawn and the view's edge.
pub(crate) fn floors(
    checks: &mut Checks,
    recorder: &mut FrameRecorder,
    recorded: &Frames,
) -> String {
    let mut sim = sim_for(1928);
    let mut frames: Frames = recorded.clone();
    for game in staged() {
        let panel = screen(&game);
        sim.world_mut().insert_resource(game);
        frames.push((panel, recorder.draw(&mut sim)));
    }
    let late_studies = staged()
        .last()
        .map(|game| {
            morning_options(game)
                .iter()
                .filter(|a| matches!(a, MorningAction::Study(_)))
                .count()
        })
        .unwrap_or(9);
    checks.require(
        late_studies == 0,
        "the staged day-5 morning still offers studies",
        format!("{late_studies} study options"),
    );
    let bounds = view();
    let font = recorder.font_texture();
    let mut clearance = f32::MAX;
    for (index, (panel, frame)) in frames.iter().enumerate() {
        let panel_breaches = judge_panel(panel, &FLOORS, &[], &[]);
        checks.require(
            panel_breaches.is_empty(),
            "a screen breaks a readability floor",
            format!("frame {index}: {panel_breaches:?}"),
        );
        let frame_breaches = judge_frame(panel, frame, font, &Identity, bounds);
        checks.require(
            frame_breaches.is_empty(),
            "a screen's rows are not where its panel says",
            format!("frame {index}: {frame_breaches:?}"),
        );
        let small = frame_text_floor(frame, font, 12.0);
        checks.require(
            small.is_empty(),
            "a glyph was drawn below 12 units",
            format!("frame {index}: {small:?}"),
        );
        let ascii = panel
            .all_strings()
            .flat_map(str::chars)
            .all(|c| (' '..='~').contains(&c));
        checks.require(
            ascii,
            "a screen holds a character the built-in face cannot draw",
            format!(
                "frame {index}: {:?}",
                panel.all_strings().collect::<Vec<_>>()
            ),
        );
        match find_bounds(frame.quads()) {
            Some(drawn) => {
                let margin = (drawn.min.x - bounds.min.x)
                    .min(drawn.min.y - bounds.min.y)
                    .min(bounds.max.x - drawn.max.x)
                    .min(bounds.max.y - drawn.max.y);
                clearance = clearance.min(margin);
                checks.require(
                    !greater(0.0, margin),
                    "something was drawn outside the camera",
                    format!("frame {index}: drawn {drawn:?}, view {bounds:?}"),
                );
            }
            None => checks.require(false, "a frame drew nothing", format!("frame {index}")),
        }
    }
    let small = TextStyle {
        size: 12.0,
        color: palette::DIM,
        depth: Depth::layer(crate::screen::layers::TEXT),
        ..TextStyle::default()
    };
    let mut overlap: Panel<Art> = Panel::default();
    overlap.text(TextRun::new(Vec2::new(24.0, 300.0), "sanity 90/90", small));
    overlap.text(TextRun::new(Vec2::new(24.0, 302.0), "temper 0/3", small));
    let bites = judge_panel(&overlap, &FLOORS, &[], &[])
        .iter()
        .any(|breach| breach.what == "two rows of chrome text overlap");
    checks.require(
        bites,
        "the overlap floor does not fail on the screen it was written for",
        "two rows at y 300 and 302".to_owned(),
    );
    format!("{clearance:.1} units")
}

/// G8: the bar's fill is half the track at sanity 45, and ember at 20.
pub(crate) fn sanity_bar(checks: &mut Checks, recorder: &mut FrameRecorder) {
    let mut sim = sim_for(1928);
    let mut game = game_of(&sim);
    game.sanity = 45;
    sim.world_mut().insert_resource(game.clone());
    let frame = recorder.draw(&mut sim);
    let y = BAR_AT.y + BAR_H * 0.5;
    let fill = frame
        .covering(Vec2::new(BAR_AT.x + 1.0, y))
        .first()
        .map(|q| q.bounds().size().x);
    let track = frame
        .covering(Vec2::new(BAR_AT.x + 399.0, y))
        .first()
        .map(|q| q.bounds().size().x);
    checks.require(
        fill.is_some_and(|w| near(w, 200.0, 0.5)) && track.is_some_and(|w| near(w, 400.0, 0.5)),
        "the sanity bar does not show half at sanity 45",
        format!("fill {fill:?} wide (want 200), track {track:?} wide (want 400)"),
    );
    game.sanity = 20;
    sim.world_mut().insert_resource(game);
    let frame = recorder.draw(&mut sim);
    let tint = frame
        .covering(Vec2::new(BAR_AT.x + 1.0, y))
        .first()
        .map(|q| q.tint);
    checks.require(
        tint == Some(palette::EMBER),
        "the sanity bar is not ember when sanity is low",
        format!("tint at sanity 20: {tint:?}"),
    );
}

/// G9: the background is the night, dark enough for ink to read on.
pub(crate) fn clear_colour(checks: &mut Checks, frames: &Frames) {
    let Some((_, frame)) = frames.first() else {
        checks.require(
            false,
            "no frame to read the clear colour off",
            String::new(),
        );
        return;
    };
    let clear = frame.plan.clear_color;
    let brightest = clear.r.max(clear.g).max(clear.b);
    checks.require(
        clear == palette::NIGHT && greater(0.2, brightest) && greater(clear.a, 0.99),
        "the background is not a dark night",
        format!("clear colour {clear:?}, brightest channel {brightest}"),
    );
}
