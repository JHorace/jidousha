//! The handoff's two decision rows, asserted; the taps, the order and the
//! floors that must bite; and the picture.
//!
//! Row 1 — which answer to give: the call screen shows the question, the
//! answers, sanity, the caller's temper and what the player knows of it
//! before a key is pressed, and the answer then moves sanity and temper by
//! exactly what `answer_outcome` says (and by shipped literals on this seed).
//! Row 2 — how to spend the morning: two runs that differ only in one
//! morning choice; each option's stated effect is on screen at choosing, and
//! the two nights then differ exactly as the two statements say.

use jidousha::prelude::*;
use jidousha::testing::{FrameRecorder, InputScript};
use jidousha::ui::*;

use super::{FLOORS, SEED, digit, drawn_camera, judge, press, run_of, start};
use crate::capture::capture_a_frame;
use crate::checks::{Checks, fail};
use crate::lore::{Being, Kind};
use crate::play::{Command, Ending, Run, Screen};
use crate::players::{Player, play};
use crate::rules::{self, Action, Change, answer_outcome, effect_of, kind_at, text_at};
use crate::screen::{
    UiMap, WINDOW, callers_line, effect_line, hint_line, known_lore, layout, question_line,
    sanity_line, temper_line,
};

/// The seed row 1 is asserted on, and what that seed's first call is.
const ROW1_SEED: u64 = 7;
const ROW1_CALLER: Being = Being::Cthulhu;
/// What the lore answer costs on that call (Cthulhu drains 2).
const ROW1_LORE_COST: i32 = 2;
/// What an insult then costs: the drain plus the surcharge for anger 1.
const ROW1_INSULT_COST: i32 = 4;
/// Cthulhu's temper is 3: the second further insult (anger 2) costs 2 + 4,
/// and the third is wrath — 2 + 6 + 10 — and ends the call.
const ROW1_SECOND_INSULT_COST: i32 = 6;
const ROW1_WRATH_COST: i32 = 18;

/// The seed row 2 is asserted on, and what its morning states for a bargain.
const ROW2_SEED: u64 = 11;
const ROW2_BARGAIN: &str = "tonight: Nyarlathotep calls instead of Yog-Sothoth";

/// Every string a panel draws, joined, so a wrapped paragraph reads whole.
fn shown(run: &Run) -> String {
    layout(run)
        .panel
        .all_strings()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Require that `run`'s screen draws every one of `facts`, and that the frame shows the panel.
fn on_screen(checks: &mut Checks, what: &str, sim: &mut HeadlessSim, facts: &[String]) {
    let run = run_of(sim);
    let text = shown(&run);
    for fact in facts {
        checks.require(
            text.contains(fact.as_str()),
            "a decision is offered without a fact it turns on",
            format!("{what}: {fact:?} is not among {text:?}"),
        );
    }
    let mut recorder = FrameRecorder::new(WINDOW);
    let frame = recorder.draw(sim);
    judge(
        checks,
        what,
        &run,
        &frame,
        recorder.font_texture(),
        &drawn_camera(sim),
    );
}

/// Row 1: which answer to give the being on the line.
pub(super) fn row_answer(checks: &mut Checks, summary: &mut Vec<String>) {
    let mut sim = start(ROW1_SEED);
    let morning = run_of(&sim);
    let caller = morning.night.calls[0].being;
    checks.require(
        caller == ROW1_CALLER,
        "row 1: the seed's first caller moved",
        format!("seed {ROW1_SEED} rings {caller:?} first, want {ROW1_CALLER:?}"),
    );
    let Some(study) = morning
        .options()
        .iter()
        .position(|option| *option == Action::Study(caller))
    else {
        fail(
            "row 1: the morning offers no study of tonight's first caller",
            &format!("{:?}", morning.options()),
        );
    };
    press(&mut sim, digit(study));
    let before = run_of(&sim);
    let Screen::Calling(call) = before.screen else {
        fail(
            "row 1: choosing a morning did not ring the first call",
            &format!("{:?}", before.screen),
        );
    };
    let id = before.night.calls[0].question(0);
    let mut facts = vec![
        question_line(&before),
        sanity_line(&before),
        temper_line(&before),
    ];
    facts.extend((0..3).map(|row| format!("{}  {}", row + 1, text_at(id, row))));
    facts.extend((0..3).map(|row| hint_line(&before, row)));
    facts.extend(known_lore(&before, caller));
    checks.require(
        !known_lore(&before, caller).is_empty(),
        "row 1: the studied lore is not shown on the call",
        format!("{:?}", before.state.known),
    );
    on_screen(checks, "row 1, the call screen", &mut sim, &facts);

    // The answer the hints say ends the call sooner, read off the screen.
    let Some(row) = (0..3).find(|row| hint_line(&before, *row).starts_with("ends sooner")) else {
        fail(
            "row 1: no hint says which answer ends the call sooner",
            &format!(
                "{:?}",
                (0..3)
                    .map(|row| hint_line(&before, row))
                    .collect::<Vec<_>>()
            ),
        );
    };
    let expected = answer_outcome(id, row, call.anger, before.state.composure);
    press(&mut sim, digit(row));
    let after = run_of(&sim);
    let paid = before.sanity - after.sanity;
    checks.require(
        paid == expected.sanity_cost && paid == ROW1_LORE_COST,
        "row 1: the lore answer did not cost what its hint said",
        format!(
            "paid {paid}, answer_outcome says {}, shipped {ROW1_LORE_COST}",
            expected.sanity_cost
        ),
    );
    let Screen::Calling(next) = after.screen else {
        fail(
            "row 1: the call ended after one lore answer",
            &format!("{:?}", after.screen),
        );
    };
    checks.require(
        next.line - call.line == expected.line_change
            && next.anger - call.anger == expected.anger_change
            && expected.line_change == -2,
        "row 1: the lore answer did not move the line and temper as stated",
        format!(
            "line {} -> {}, temper {} -> {}, stated {expected:?}",
            call.line, next.line, call.anger, next.anger
        ),
    );

    // Then an insult: temper rises by one and the surcharge is paid.
    let id = after.night.calls[0].question(next.asked);
    let Some(insult) = (0..3).find(|row| kind_at(id, *row) == Kind::Insult) else {
        fail("row 1: a question has no insult", &format!("{id:?}"));
    };
    let expected = answer_outcome(id, insult, next.anger, after.state.composure);
    press(&mut sim, digit(insult));
    let angered = run_of(&sim);
    let paid = after.sanity - angered.sanity;
    let anger = match angered.screen {
        Screen::Calling(call) => call.anger,
        Screen::CallOver { call, .. } => call.anger,
        _ => -1,
    };
    checks.require(
        paid == expected.sanity_cost && paid == ROW1_INSULT_COST && anger == next.anger + 1,
        "row 1: an insult did not cost and anger as answer_outcome says",
        format!(
            "paid {paid} (stated {}, shipped {ROW1_INSULT_COST}), temper {} -> {anger}",
            expected.sanity_cost, next.anger
        ),
    );
    // Keep insulting: the call ends in wrath exactly when anger reaches its temper.
    let mut paid = Vec::new();
    let mut ended = None;
    for _ in 0..4 {
        let now = run_of(&sim);
        let Screen::Calling(call) = now.screen else {
            ended = Some(now.screen);
            break;
        };
        let id = now.night.calls[0].question(call.asked);
        let row = (0..3)
            .find(|row| kind_at(id, *row) == Kind::Insult)
            .unwrap_or(0);
        press(&mut sim, digit(row));
        paid.push(now.sanity - run_of(&sim).sanity);
    }
    checks.require(
        paid == [ROW1_SECOND_INSULT_COST, ROW1_WRATH_COST]
            && matches!(ended, Some(Screen::CallOver { ending: Ending::Wrath, call, .. }) if call.anger == ROW1_CALLER.spec().temper),
        "row 1: wrath did not come when anger reached the caller's temper",
        format!("paid {paid:?}, want [{ROW1_SECOND_INSULT_COST}, {ROW1_WRATH_COST}]; ended {ended:?}"),
    );
    summary.push(format!("row 1 (answer): facts on screen before answering; lore paid {ROW1_LORE_COST}, insult paid {ROW1_INSULT_COST} and temper +1"));
}

/// Play the night a sim is on, answering every question with its harmless
/// wrong answer, and say per call who called, what its first exchange cost,
/// and which fact its first question was about.
fn night_of(sim: &mut HeadlessSim) -> Vec<(Being, i32, usize)> {
    let mut seen = Vec::new();
    for _ in 0..100 {
        let run = run_of(sim);
        match run.screen {
            Screen::Calling(call) => {
                let id = run.night.calls[call.slot].question(call.asked);
                let row = (0..3)
                    .find(|row| kind_at(id, *row) == Kind::Wrong)
                    .unwrap_or(0);
                press(sim, digit(row));
                if call.asked == 0 {
                    seen.push((call.being, run.sanity - run_of(sim).sanity, id.fact));
                }
            }
            Screen::CallOver { call, .. } if call.slot + 1 == run.night.calls.len() => break,
            Screen::CallOver { .. } => press(sim, Key::Space),
            Screen::Morning | Screen::End(_) => break,
        }
    }
    seen
}

/// Row 2: how to spend the morning.
pub(super) fn row_morning(checks: &mut Checks, summary: &mut Vec<String>) {
    let (mut trained, mut bargained) = (start(ROW2_SEED), start(ROW2_SEED));
    let morning = run_of(&trained);
    let options = morning.options();
    let first = morning.night.calls[0].being;
    let (Some(train), Some(bargain)) = (
        options.iter().position(|option| *option == Action::Train),
        options
            .iter()
            .position(|option| *option == Action::Bargain(first)),
    ) else {
        fail(
            "row 2: the first morning lacks a training or a bargain",
            &format!("{options:?}"),
        );
    };
    let stated_bargain = effect_line(&morning, bargain);
    checks.require(
        stated_bargain == ROW2_BARGAIN,
        "row 2: the bargain states another effect on this seed",
        format!("{stated_bargain:?}, shipped {ROW2_BARGAIN:?}"),
    );
    let facts = [
        callers_line(&morning),
        effect_line(&morning, train),
        stated_bargain.clone(),
    ];
    on_screen(checks, "row 2, the morning", &mut trained, &facts);

    press(&mut trained, digit(train));
    press(&mut bargained, digit(bargain));
    let (a, b) = (night_of(&mut trained), night_of(&mut bargained));
    let for_train = effect_of(&morning.state, &rules::apply(&morning.state, Action::Train));
    let for_bargain = effect_of(
        &morning.state,
        &rules::apply(&morning.state, Action::Bargain(first)),
    );

    // Who called: the bargain's stated swaps, and nothing else.
    for slot in 0..a.len().max(b.len()) {
        let (x, y) = (
            a.get(slot).map(|call| call.0),
            b.get(slot).map(|call| call.0),
        );
        let stated = for_bargain.iter().any(|change| matches!(change, Change::Caller { slot: s, from, to } if *s == slot && Some(*from) == x && Some(*to) == y));
        checks.require(
            x == y || stated,
            "row 2: the nights' callers differ where no option said they would",
            format!("slot {slot}: trained {x:?}, bargained {y:?}; stated {for_bargain:?}"),
        );
        checks.require(
            x != y || !stated,
            "row 2: a stated swap of callers did not happen",
            format!("slot {slot}: both {x:?}"),
        );
    }
    // What each exchange drained, for whoever called both nights: the training's stated drains.
    for (being, trained_cost, trained_fact) in &a {
        let Some((_, bargained_cost, bargained_fact)) = b.iter().find(|call| call.0 == *being)
        else {
            continue;
        };
        let stated = for_train.iter().find_map(|change| match change {
            Change::Drain {
                being: who,
                from,
                to,
            } if who == being => Some((*from, *to)),
            _ => None,
        });
        checks.require(
            stated == Some((*bargained_cost, *trained_cost)),
            "row 2: the drain differs from what the training stated",
            format!(
                "{being:?}: drained {trained_cost} trained, {bargained_cost} not; stated {stated:?}"
            ),
        );
        checks.require(
            trained_fact == bargained_fact,
            "row 2: a caller opened on another topic though no option said so",
            format!("{being:?}: {trained_fact} vs {bargained_fact}"),
        );
    }
    summary.push(format!(
        "row 2 (morning): trained {a:?} vs bargained {b:?}, as stated"
    ));
}

/// A tap chooses the row it lands on; the systems run in the order chosen;
/// the room is dark; the same seed plays the same run.
pub(super) fn taps_and_order(checks: &mut Checks, summary: &mut Vec<String>) {
    let mut sim = start(SEED);
    let morning = run_of(&sim);
    let camera = drawn_camera(&sim);
    let map = UiMap::for_camera(&camera);
    let Some(target) = layout(&morning).choices.get(1).copied() else {
        fail("the morning has no second option to tap", "");
    };
    let tick = sim.world().resource::<Time>().tick + 1;
    let script = InputScript::new()
        .pointer_at(tick, camera.world_to_screen(map.to_world(target.center())))
        .click(PointerButton::Primary, tick);
    sim.world_mut()
        .insert_resource(Input::new(script.snapshot_at(tick)));
    sim.tick();
    let tapped = run_of(&sim);
    let mut want = morning.clone();
    want.step(Command::Choose(1));
    checks.require(
        tapped == want,
        "a tap on the second option did not choose it",
        format!("state {:?}, want {:?}", tapped.state, want.state),
    );

    let order = sim.schedule_debug();
    let (update, take, draw) = (
        order.find("Update"),
        order.find("take_the_call"),
        order.find("draw_the_screen"),
    );
    checks.require(
        update.is_some() && take.is_some() && draw.is_some() && update < take && take < draw,
        "the systems do not run in the order the game registers",
        order.clone(),
    );

    let mut recorder = FrameRecorder::new(WINDOW);
    let cleared = recorder.draw(&mut sim).plan.clear_color;
    let brightest = cleared.r.max(cleared.g).max(cleared.b);
    checks.require(
        brightest < 0.15 && cleared.a > 0.99,
        "the room is not dark enough for pale text",
        format!("brightest channel {brightest:.3}, alpha {:.2}", cleared.a),
    );

    let (once, twice) = (play(Player::Novice, SEED), play(Player::Novice, SEED));
    checks.require(
        once.is_some() && once == twice,
        "the same seed did not play the same run",
        format!("{once:?} vs {twice:?}"),
    );
    summary.push("a tap chooses its row; schedule, dark room and replay hold".to_owned());
}

/// The floors bite from this game's own layout: a row too small, two rows atop each other.
pub(super) fn floors_bite(checks: &mut Checks) {
    let small = TextStyle {
        size: 10.0,
        depth: Depth::layer(1),
        ..TextStyle::default()
    };
    let body = TextStyle {
        size: 16.0,
        depth: Depth::layer(1),
        ..TextStyle::default()
    };
    let mut staged: Panel<crate::screen::Art> = Panel::default();
    staged.text(TextRun::new(
        Vec2::new(32.0, 100.0),
        "too small to read",
        small,
    ));
    staged.text(TextRun::new(
        Vec2::new(32.0, 200.0),
        "1  The one who lies dreaming",
        body,
    ));
    staged.text(TextRun::new(
        Vec2::new(40.0, 204.0),
        "ends sooner: line -2",
        body,
    ));
    let breaches = judge_panel(&staged, &FLOORS, &[], &[]);
    for name in [
        "a row of text is smaller than the readability floor allows",
        "two rows of chrome text overlap",
    ] {
        checks.require(
            breaches.iter().any(|breach| breach.what == name),
            "a floor does not bite on the screen it was written for",
            format!("{name:?} not among {breaches:?}"),
        );
    }
}

/// The first call of row 1's seed, photographed.
pub(super) fn picture(checks: &mut Checks) -> String {
    let mut sim = start(ROW1_SEED);
    let morning = run_of(&sim);
    let study = morning
        .options()
        .iter()
        .position(|option| *option == Action::Study(ROW1_CALLER))
        .unwrap_or(0);
    press(&mut sim, digit(study));
    let mut recorder = FrameRecorder::new(WINDOW);
    let frame = recorder.draw(&mut sim);
    capture_a_frame(checks, &frame, recorder.font_texture())
}
