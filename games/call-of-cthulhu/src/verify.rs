//! `--verify`: the whole game, headless. It drives the real systems with keys and clicks sent
//! as events, reads the screen the player would see, and holds it to the rules, to both
//! decision rows (`decisions.rs`), to the readability floors and to three players
//! (`players.rs`) — then takes a picture (`capture.rs`).
//!
//! INVARIANT: expectations about the rules are shipped literals from `DESIGN.md`, never read
//! back from the constants they check.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::FrameRecord;
use jidousha::ui::{Breach, frame_text_floor, judge_frame, judge_panel};

use crate::checks::Checks;
use crate::decisions::{check_call_decision, check_morning_decision};
use crate::driver::Driver;
use crate::game::{Ending, Game, Stage};
use crate::lore::{BEINGS, Being, FACTS, FACTS_PER_BEING};
use crate::palette;
use crate::players::{Player, choose as player_choose, play, sweep};
use crate::rules::{
    ACTIONS, Action, Kind, answer_outcome, exchange_cost, option_kinds, plan_night,
};
use crate::screens::{FLOORS, MIN_TEXT, NoArt, controls, panel};
use crate::view::UiMap;
use crate::{DEFAULT_SEED, camera};

/// How many seeds each player is swept over.
const SWEEP: u64 = 100;

/// Judge one drawn screen: legible, inside the camera, drawn as stated, in printable ASCII.
/// Returns the frame and how close the nearest quad is to the camera's edge.
pub fn judge_screen(checks: &mut Checks, label: &str, driver: &mut Driver) -> (FrameRecord, f32) {
    let frame = driver.draw();
    let game = driver.game().clone();
    let screen = panel(&game);
    let cam = camera();
    let map = UiMap::for_camera(&cam);
    let view = cam.visible_bounds();
    let named: Vec<(String, Rect)> = controls(&game)
        .into_iter()
        .map(|(rect, choice)| (format!("{choice:?}"), rect))
        .collect();
    for breach in judge_panel(&screen, &FLOORS, &[], &[]) {
        checks.require(false, &format!("{label}: {}", breach.what), breach.detail);
    }
    let _ = named;
    let font = driver.recorder.font_texture();
    for breach in judge_frame(&screen, &frame, font, &map, view) {
        checks.require(false, &format!("{label}: {}", breach.what), breach.detail);
    }
    for breach in frame_text_floor(&frame, font, MIN_TEXT) {
        checks.require(false, &format!("{label}: {}", breach.what), breach.detail);
    }
    for text in screen.all_strings() {
        checks.require(
            text.chars().all(|c| (' '..='~').contains(&c)),
            "a screen carries a character the font cannot draw",
            format!("{label}: {text:?}"),
        );
    }
    let mut clearance = f32::MAX;
    for quad in frame.quads() {
        let bounds = quad.bounds();
        checks.require(
            view.contains_rect(bounds),
            "something is drawn outside the camera",
            format!("{label}: {bounds:?} against {view:?}"),
        );
        let gap = (bounds.min - view.min).min(view.max - bounds.max);
        clearance = clearance.min(gap.x.min(gap.y));
    }
    // The background: the camera's own colour, and dark enough for the text to read on it.
    let cleared = frame.plan.clear_color;
    let brightness = cleared.r.max(cleared.g).max(cleared.b);
    checks.require(
        cleared == palette::BG && brightness < 0.25 && cleared.a > 0.99,
        "the frame is not cleared to a dark background",
        format!("{label}: {cleared:?}, brightest channel {brightness:.3}"),
    );
    (frame, clearance)
}

/// A game staged on a call: `being`'s `fact` on the line, everything known if `known`.
pub fn staged_call(seed: u64, being: Being, fact: usize, known: bool, temper: i32) -> Game {
    let mut driver = Driver::new(seed);
    // Start a night whoever it brings, then replace the call with the one asked for.
    driver.option(ACTIONS.len() - 2);
    let mut game = driver.game().clone();
    if let Some(call) = game.call.as_mut() {
        call.plan.being = being;
        call.plan.order = (0..FACTS_PER_BEING).collect();
        call.exchange = fact;
        call.temper = temper;
        call.remaining = being.call_length();
    }
    if known {
        game.known = [[true; FACTS_PER_BEING]; BEINGS];
    }
    game
}

/// Every screen the run reaches or never reaches, judged. Returns the summary line.
fn check_screens(checks: &mut Checks) -> String {
    let mut clearance = f32::MAX;
    let mut screens = 0;
    let mut driver = Driver::new(DEFAULT_SEED);
    let (_, c) = judge_screen(checks, "morning, day 1", &mut driver);
    clearance = clearance.min(c);
    screens += 1;
    // Every question of every being, at its widest: all lore known, in wrath and calm.
    for being in Being::ALL {
        for fact in 0..FACTS_PER_BEING {
            for (known, temper) in [(true, 0), (false, 0), (true, being.wrath_at())] {
                let mut d = Driver::new(DEFAULT_SEED);
                d.stage(staged_call(DEFAULT_SEED, being, fact, known, temper));
                let label = format!("{} fact {fact} known {known} temper {temper}", being.name());
                let (_, c) = judge_screen(checks, &label, &mut d);
                clearance = clearance.min(c);
                screens += 1;
            }
        }
    }
    // The morning's worst cases: everything known, composure full, a long refusal note.
    let mut game = Game::new(DEFAULT_SEED);
    game.known = [[true; FACTS_PER_BEING]; BEINGS];
    game.composure = crate::rules::COMPOSURE_MAX;
    game.sanity = 3;
    game.note = "The rite would cost 4 sanity and you have 3. ".repeat(3);
    let mut d = Driver::new(DEFAULT_SEED);
    d.stage(game.clone());
    let (_, c) = judge_screen(checks, "morning, nothing left to learn, shaken", &mut d);
    clearance = clearance.min(c);
    screens += 1;
    for (ending, sanity) in [(Ending::Sound, 40), (Ending::Frayed, 5), (Ending::Lost, 0)] {
        let mut over = Game::new(DEFAULT_SEED);
        over.day = 5;
        over.sanity = sanity;
        over.stage = Stage::Over(ending);
        let mut d = Driver::new(DEFAULT_SEED);
        d.stage(over);
        let (_, c) = judge_screen(checks, &format!("{ending:?} ending"), &mut d);
        clearance = clearance.min(c);
        screens += 1;
    }
    // The two endings that differ must differ.
    let text = |ending: Ending| {
        let mut g = Game::new(DEFAULT_SEED);
        g.stage = Stage::Over(ending);
        crate::decisions::text_of(&panel(&g))
    };
    checks.require(
        text(Ending::Sound) != text(Ending::Frayed) && text(Ending::Frayed) != text(Ending::Lost),
        "two different endings read the same",
        "Sound, Frayed and Lost screens".to_owned(),
    );
    format!(
        "screens: {screens} judged for floors, bounds, text and background; closest quad to the edge {clearance:.2} units"
    )
}

/// The floors bite on the screens they were written for.
fn check_floors_bite(checks: &mut Checks) -> String {
    let style = |size: f32| TextStyle {
        size,
        color: palette::TEXT,
        depth: Depth::layer(1),
        ..TextStyle::default()
    };
    let mut staged: jidousha::ui::Panel<NoArt> = jidousha::ui::Panel::default();
    staged.text(jidousha::ui::TextRun::new(
        Vec2::new(10.0, 10.0),
        "too small",
        style(9.0),
    ));
    staged.text(jidousha::ui::TextRun::new(
        Vec2::new(10.0, 100.0),
        "seed zero",
        style(14.0),
    ));
    staged.text(jidousha::ui::TextRun::new(
        Vec2::new(10.0, 102.0),
        "points at a row",
        style(14.0),
    ));
    staged.text(jidousha::ui::TextRun::new(
        Vec2::new(950.0, 300.0),
        "off the edge",
        style(14.0),
    ));
    let breaches: Vec<Breach> = judge_panel(&staged, &FLOORS, &[], &[]);
    let names: Vec<&str> = breaches.iter().map(|b| b.what).collect();
    for want in [
        "a row of text is smaller than the readability floor allows",
        "two rows of chrome text overlap",
    ] {
        checks.require(
            names.contains(&want),
            "a floor does not bite on the screen it was written for",
            format!("wanted {want:?} among {names:?}"),
        );
    }
    format!(
        "floors: {} breaches named on a staged screen",
        breaches.len()
    )
}

/// The rules, against shipped literals: every being, every kind.
fn check_rules(checks: &mut Checks) -> String {
    // (being, kind, remaining delta, temper delta, sanity cost) at composure 0, temper 0.
    let table: [(Being, Kind, i32, i32, i32); 12] = [
        (Being::YogSothoth, Kind::Right, -2, -1, 2),
        (Being::YogSothoth, Kind::Guess, 0, 0, 2),
        (Being::YogSothoth, Kind::Insult, 0, 1, 4),
        (Being::YogSothoth, Kind::HangUp, 0, 1, 16),
        (Being::Dagon, Kind::Right, -2, -1, 1),
        (Being::Dagon, Kind::Guess, 0, 0, 1),
        (Being::Dagon, Kind::Insult, 0, 1, 2),
        (Being::Dagon, Kind::HangUp, 0, 1, 8),
        (Being::Nyarlathotep, Kind::Right, -2, -1, 2),
        (Being::Nyarlathotep, Kind::Guess, 0, 0, 2),
        (Being::Nyarlathotep, Kind::Insult, 0, 1, 4),
        (Being::Nyarlathotep, Kind::HangUp, 0, 1, 16),
    ];
    for (being, kind, remaining, temper, cost) in table {
        let o = answer_outcome(being, 0, 0, kind);
        checks.require(
            (o.remaining_delta, o.temper_delta, o.sanity_cost) == (remaining, temper, cost)
                && o.ends_call == (kind == Kind::HangUp),
            "an answer's outcome is not the shipped one",
            format!(
                "{} {kind:?}: got {o:?}, want ({remaining}, {temper}, {cost})",
                being.name()
            ),
        );
    }
    // Composure takes one off each point and stops at one; hanging up is not steadied.
    for (being, composure, cost) in [
        (Being::YogSothoth, 1, 1),
        (Being::YogSothoth, 2, 1),
        (Being::Dagon, 2, 1),
        (Being::Nyarlathotep, 1, 1),
    ] {
        checks.require(
            exchange_cost(being, composure, 0) == cost,
            "composure does not take one off an exchange and stop at one",
            format!(
                "{} composure {composure}: {}",
                being.name(),
                exchange_cost(being, composure, 0)
            ),
        );
    }
    checks.require(
        answer_outcome(Being::YogSothoth, 2, 0, Kind::HangUp).sanity_cost == 16,
        "composure steadied a hang-up",
        format!(
            "{:?}",
            answer_outcome(Being::YogSothoth, 2, 0, Kind::HangUp)
        ),
    );
    // Wrath doubles every exchange: Yog at temper 2, Dagon at 4, Nyarlathotep at 3.
    for (being, wrath, calm_cost) in [
        (Being::YogSothoth, 2, 2),
        (Being::Dagon, 4, 1),
        (Being::Nyarlathotep, 3, 2),
    ] {
        checks.require(
            exchange_cost(being, 0, wrath - 1) == calm_cost
                && exchange_cost(being, 0, wrath) == calm_cost * 2,
            "wrath does not double an exchange where the being's temper breaks",
            format!(
                "{}: below {}, at {}",
                being.name(),
                exchange_cost(being, 0, wrath - 1),
                exchange_cost(being, 0, wrath)
            ),
        );
    }
    // Anger costs more than the drain, everywhere.
    let mut anger_checked = 0;
    for being in Being::ALL {
        for composure in 0..=2 {
            for temper in 0..=being.wrath_at() {
                let calm = answer_outcome(being, composure, temper, Kind::Guess).sanity_cost;
                let insult = answer_outcome(being, composure, temper, Kind::Insult).sanity_cost;
                let hangup = answer_outcome(being, composure, temper, Kind::HangUp).sanity_cost;
                checks.require(
                    insult > calm && hangup > insult,
                    "anger does not cost more than the drain",
                    format!("{} composure {composure} temper {temper}: drain {calm}, insult {insult}, hang-up {hangup}", being.name()),
                );
                anger_checked += 1;
            }
        }
    }
    format!(
        "rules: 12 outcomes, composure, wrath and {anger_checked} cases of anger costing more than the drain"
    )
}

/// The lore: five facts a being, each readable, each with three different answers.
fn check_lore(checks: &mut Checks) -> String {
    let mut strings = 0;
    for being in Being::ALL {
        checks.require(
            FACTS[being.index()].len() == 5,
            "a being does not have five facts",
            being.name().to_owned(),
        );
        for fact in &FACTS[being.index()] {
            let all = [
                fact.title,
                fact.lore,
                fact.question,
                fact.right,
                fact.guess,
                fact.insult,
                fact.pleased,
            ];
            for s in all {
                strings += 1;
                checks.require(
                    s.chars().all(|c| (' '..='~').contains(&c)) && !s.is_empty(),
                    "a lore string is empty or has a character the font cannot draw",
                    format!("{}: {s:?}", being.name()),
                );
            }
            checks.require(
                fact.right != fact.guess && fact.guess != fact.insult && fact.right != fact.insult,
                "a question's three answers are not three different answers",
                format!("{}: {}", being.name(), fact.question),
            );
        }
    }
    format!("lore: {strings} strings printable, every question has three different answers")
}

/// Who calls, in what order, and what changes it.
fn check_plan(checks: &mut Checks) -> String {
    // The same seed plans the same night; the seeds between them bring every being and both orders.
    let mut seen_first = [false; BEINGS];
    let mut orders = std::collections::BTreeSet::new();
    for seed in 0..60u64 {
        let game = Game::new(seed);
        let a = plan_night(&game);
        let b = plan_night(&game);
        checks.require(
            a == b,
            "the same game plans two different nights",
            format!("seed {seed}"),
        );
        checks.require(
            a.len() == 2 && a[0].being != a[1].being,
            "a night is not two calls from two different beings",
            format!(
                "seed {seed}: {:?}",
                a.iter().map(|p| p.being).collect::<Vec<_>>()
            ),
        );
        seen_first[a[0].being.index()] = true;
        orders.insert((a[0].being, a[1].being));
    }
    checks.require(
        seen_first == [true; BEINGS] && orders.len() >= 5,
        "sixty seeds do not bring every being first and most pairs",
        format!("{seen_first:?}, {} pairs", orders.len()),
    );
    // Appeasing keeps a being off the line; a grudge puts it first; study puts facts first.
    let mut game = Game::new(7);
    let plain = plan_night(&game);
    game.appeased[plain[0].being.index()] = true;
    let kept = plan_night(&game);
    checks.require(
        kept.iter().all(|p| p.being != plain[0].being) && kept.len() == 2,
        "an appeased being still calls",
        format!("{:?}", kept.iter().map(|p| p.being).collect::<Vec<_>>()),
    );
    let mut game = Game::new(7);
    let angry = plain[1].being;
    game.grudge = Some(angry);
    let with = plan_night(&game);
    checks.require(
        with[0].being == angry && with[0].temper == 1,
        "a grudge does not call first and already irked",
        format!(
            "{:?}",
            with.iter().map(|p| (p.being, p.temper)).collect::<Vec<_>>()
        ),
    );
    game.appeased[angry.index()] = true;
    checks.require(
        plan_night(&game).iter().all(|p| p.being != angry),
        "appeasing a being does not keep its grudge off the line",
        String::new(),
    );
    "plan: same seed same night, every being first somewhere, appeasing and grudges as stated"
        .to_owned()
}

/// Same seed and same inputs replay the same run, and the key path is the pure path.
fn check_replay(checks: &mut Checks) -> String {
    let mut line = Vec::new();
    for seed in 0..4u64 {
        let pure = play(seed, Player::Reader);
        let mut run = Driver::new(seed);
        for _ in 0..800 {
            if matches!(run.game().stage, Stage::Over(_)) {
                break;
            }
            let choice = player_choose(Player::Reader, run.game());
            if let crate::game::Choice::Option(n) = choice {
                run.option(n);
            }
        }
        let keyed = (
            run.game().sanity,
            run.game().record.len(),
            matches!(run.game().stage, Stage::Over(e) if e != Ending::Lost),
        );
        let wanted = (pure.sanity, pure.answers, pure.ending != Ending::Lost);
        checks.require(
            keyed == wanted,
            "playing by key presses does not replay the pure run",
            format!("seed {seed}: keys {keyed:?}, pure {wanted:?}"),
        );
        let again = play(seed, Player::Reader);
        checks.require(
            again == pure,
            "the same seed and inputs do not replay the same run",
            format!("seed {seed}"),
        );
        line.push(format!("{seed}:{}", if wanted.2 { "won" } else { "lost" }));
    }
    format!(
        "replay: key presses and pure play agree, and repeat exactly ({})",
        line.join(" ")
    )
}

/// The three players: only the middle one can say the game is worth playing.
fn check_players(checks: &mut Checks) -> Vec<String> {
    let mut lines = Vec::new();
    let mut wins = [0usize; 3];
    for (i, player) in Player::ALL.iter().enumerate() {
        let (won, total, mean) = sweep(*player, 0..SWEEP);
        wins[i] = won;
        let sample = play(0, *player);
        checks.require(
            sample.refused == 0 && (0..SWEEP).all(|s| play(s, *player).refused == 0),
            "a player made a choice the game refused",
            format!("{} seed 0: {sample:?}", player.name()),
        );
        lines.push(format!(
            "player {}: won {won} of {total}, survivors end at {mean:.1} sanity; seed 0: {:?} on day {}, {} answers, {} right, {} insults, {} hang-ups",
            player.name(),
            sample.ending,
            sample.days,
            sample.answers,
            sample.right,
            sample.insults,
            sample.hang_ups
        ));
    }
    let [reader, chaser, hang] = wins;
    checks.require(
        reader >= 60 && reader > chaser + 25,
        "the player who reads the lore does not clearly beat the one who answers the first thing",
        format!("reader {reader}, chaser {chaser} of {SWEEP}"),
    );
    checks.require(
        chaser <= 40,
        "a player who never studies wins too often: the lore does not matter",
        format!("chaser {chaser} of {SWEEP}"),
    );
    checks.require(
        hang == 0,
        "hanging up on everyone can win: the game cannot be lost that way",
        format!("hang-up {hang} of {SWEEP}"),
    );
    // A win and a loss, reached by the real input path.
    let win_seed = (0..SWEEP).find(|&s| play(s, Player::Reader).ending != Ending::Lost);
    let loss_seed = (0..SWEEP).find(|&s| play(s, Player::HangUp).ending == Ending::Lost);
    checks.require(
        win_seed.is_some() && loss_seed.is_some(),
        "no seed wins for the reader or loses for the hang-up player",
        format!("{win_seed:?} {loss_seed:?}"),
    );
    for (seed, player, want_win) in [
        (win_seed, Player::Reader, true),
        (loss_seed, Player::HangUp, false),
    ] {
        let Some(seed) = seed else { continue };
        let mut run = Driver::new(seed);
        for _ in 0..800 {
            if matches!(run.game().stage, Stage::Over(_)) {
                break;
            }
            if let crate::game::Choice::Option(n) = player_choose(player, run.game()) {
                run.option(n);
            }
        }
        let ended = match run.game().stage {
            Stage::Over(e) => Some(e),
            _ => None,
        };
        checks.require(
            ended.is_some_and(|e| (e != Ending::Lost) == want_win),
            "a scripted run did not end the way it was meant to",
            format!("{} seed {seed}: {ended:?}", player.name()),
        );
        // Surviving means five days: the run that is won ends on the fifth.
        checks.require(
            !want_win || run.game().day == 5,
            "a won run does not end on day 5",
            format!("{} seed {seed}: day {}", player.name(), run.game().day),
        );
        lines.push(format!(
            "scripted by keys: {} seed {seed} ends {ended:?} on day {}",
            player.name(),
            run.game().day
        ));
    }
    lines
}

/// Keys and clicks reach the game, and a refusal says so.
fn check_input(checks: &mut Checks) -> String {
    // A click on a morning cell is the key for it.
    let mut by_click = Driver::new(DEFAULT_SEED);
    let train = ACTIONS
        .iter()
        .position(|a| *a == Action::Train)
        .unwrap_or(0);
    by_click.click(crate::screens::morning_cell(train).center());
    let mut by_key = Driver::new(DEFAULT_SEED);
    by_key.option(train);
    checks.require(
        by_click.game().composure == 1
            && by_click.game().composure == by_key.game().composure
            && by_click.game().night == by_key.game().night,
        "a click on a morning option does not do what its key does",
        format!(
            "click composure {}, key composure {}",
            by_click.game().composure,
            by_key.game().composure
        ),
    );
    // A click on "Hang up" hangs up.
    by_click.click(crate::screens::call_row(3).center());
    checks.require(
        by_click
            .game()
            .record
            .last()
            .is_some_and(|r| r.kind == Kind::HangUp),
        "a click on Hang up does not hang up",
        format!("{:?}", by_click.game().record.last()),
    );
    // A refusal tells the player and changes nothing.
    let mut full = Driver::new(DEFAULT_SEED);
    let rest = ACTIONS.iter().position(|a| *a == Action::Rest).unwrap_or(0);
    let before = full.game().clone();
    full.option(rest);
    checks.require(
        full.game().note == "You are as rested as you will be."
            && full.game().sanity == before.sanity
            && matches!(full.game().stage, Stage::Morning),
        "resting at full sanity is not refused aloud",
        format!("note {:?}", full.game().note),
    );
    // A click that lands on no row does nothing at all.
    full.click(Vec2::new(5.0, 530.0));
    checks.require(
        full.game().sanity == before.sanity && matches!(full.game().stage, Stage::Morning),
        "a click on nothing changed the game",
        String::new(),
    );
    // R begins again from the Over screen, with the next seed.
    let mut over = Driver::new(DEFAULT_SEED);
    let mut game = over.game().clone();
    game.stage = Stage::Over(Ending::Lost);
    over.stage(game);
    over.press(Key::R);
    checks.require(
        over.game().day == 1
            && matches!(over.game().stage, Stage::Morning)
            && over.game().seed == DEFAULT_SEED + 1,
        "R does not begin another run from the end screen",
        format!("day {} seed {:#x}", over.game().day, over.game().seed),
    );
    "input: a click is its key, hang up by click, a refusal is aloud, a click on nothing is nothing, R begins again".to_owned()
}

/// What a morning does to the state, the end of a night, a grudge, the bars and the temper words.
fn check_state(checks: &mut Checks) -> String {
    // The endings split at 12 sanity.
    checks.require(
        crate::rules::ending_for(12) == Ending::Sound
            && crate::rules::ending_for(11) == Ending::Frayed,
        "a survived run does not end sound at 12 sanity and frayed at 11",
        format!(
            "{:?} {:?}",
            crate::rules::ending_for(12),
            crate::rules::ending_for(11)
        ),
    );
    // Each morning option does what it stated: appeasing costs 4, resting gives 8 up to 60,
    // training adds one composure, studying teaches three facts.
    let seed = DEFAULT_SEED;
    let key = |action: Action| ACTIONS.iter().position(|a| *a == action).unwrap_or(0);
    let mut appease = Driver::new(seed);
    let spared = plan_night(appease.game())[0].being;
    appease.option(key(Action::Appease(spared)));
    checks.require(
        appease.game().sanity == 56 && appease.game().appeased[spared.index()],
        "appeasing a cult does not cost 4 sanity and keep its being off tonight",
        format!("sanity {}", appease.game().sanity),
    );
    for (start, want) in [(50, 58), (56, 60)] {
        let mut rest = Driver::new(seed);
        let mut game = rest.game().clone();
        game.sanity = start;
        rest.stage(game);
        rest.option(key(Action::Rest));
        checks.require(
            rest.game().sanity == want,
            "resting does not give 8 sanity up to the most there is",
            format!("from {start}: {}", rest.game().sanity),
        );
    }
    let mut train = Driver::new(seed);
    train.option(key(Action::Train));
    checks.require(
        train.game().composure == 1,
        "training does not add a point of composure",
        format!("{}", train.game().composure),
    );
    let mut study = Driver::new(seed);
    let target = plan_night(study.game())[0].being;
    study.option(key(Action::Study(target)));
    checks.require(
        study.game().known_facts(target) == vec![0, 1, 2],
        "a morning's study does not teach the first three unknown facts",
        format!("{:?}", study.game().known_facts(target)),
    );
    // A night ends clean: the next morning has nothing appeased or studied, and it is day 2.
    let mut night = Driver::new(seed);
    night.option(key(Action::Appease(spared)));
    for _ in 0..200 {
        if matches!(night.game().stage, Stage::Morning) {
            break;
        }
        night.option(0);
    }
    checks.require(
        matches!(night.game().stage, Stage::Morning)
            && night.game().day == 2
            && night.game().appeased == [false; BEINGS]
            && night.game().studied_today.is_none(),
        "a night does not end in a clean second morning",
        format!(
            "day {} stage {:?} appeased {:?}",
            night.game().day,
            night.game().stage,
            night.game().appeased
        ),
    );
    // Grudges: hanging up leaves one, a call left in wrath leaves one, a clean call leaves none.
    let mut grudge = Driver::new(seed);
    grudge.option(key(Action::Train));
    let first = grudge.game().night[0].being;
    grudge.option(3);
    checks.require(
        grudge.game().grudge == Some(first),
        "hanging up does not leave a grudge",
        format!("{:?}", grudge.game().grudge),
    );
    let mut wrath = Driver::new(seed);
    let being = Being::YogSothoth;
    wrath.stage(staged_call(seed, being, 0, true, being.wrath_at() + 1));
    let mut game = wrath.game().clone();
    if let Some(call) = game.call.as_mut() {
        call.remaining = 1;
    }
    wrath.stage(game);
    let right = option_kinds(
        seed,
        wrath.game().day,
        wrath.game().call.as_ref().map_or(0, |c| c.slot),
        0,
    )
    .iter()
    .position(|k| *k == Kind::Right)
    .unwrap_or(0);
    wrath.option(right);
    checks.require(
        wrath.game().grudge == Some(being),
        "a call that ends with the being still in wrath leaves no grudge",
        format!("{:?}", wrath.game().grudge),
    );
    let mut calm = Driver::new(seed);
    calm.stage(staged_call(seed, being, 0, true, 0));
    let mut game = calm.game().clone();
    if let Some(call) = game.call.as_mut() {
        call.remaining = 1;
    }
    game.grudge = Some(being);
    calm.stage(game);
    let right = option_kinds(
        seed,
        calm.game().day,
        calm.game().call.as_ref().map_or(0, |c| c.slot),
        0,
    )
    .iter()
    .position(|k| *k == Kind::Right)
    .unwrap_or(0);
    calm.option(right);
    checks.require(
        calm.game().grudge.is_none(),
        "a call that ends calm does not settle the being's grudge",
        format!("{:?}", calm.game().grudge),
    );
    // The bars: sanity as a share of 60, the temper pips lit as the temper.
    let mut shown = Game::new(seed);
    shown.sanity = 30;
    let bars = crate::screens::bars(&shown);
    checks.require(
        bars.len() == 2 && (bars[1].0.size().x - 150.0).abs() < 0.01,
        "the sanity bar is not half full at 30 of 60",
        format!(
            "{:?}",
            bars.iter().map(|b| b.0.size().x).collect::<Vec<_>>()
        ),
    );
    let on_call = staged_call(seed, Being::Nyarlathotep, 0, true, 2);
    let lit = crate::screens::bars(&on_call)
        .iter()
        .filter(|b| b.1 == palette::WARN)
        .count();
    checks.require(
        lit == 2,
        "the temper pips do not light as many as the temper",
        format!("{lit} lit at temper 2"),
    );
    // The temper in words: calm, irked below the break, IN WRATH at it.
    for (being, words) in [
        (Being::YogSothoth, ["calm", "irked", "IN WRATH"]),
        (Being::Dagon, ["calm", "irked", "irked"]),
        (Being::Nyarlathotep, ["calm", "irked", "irked"]),
    ] {
        for (temper, want) in words.iter().enumerate() {
            checks.require(
                crate::screens::temper_word(being, temper as i32) == *want,
                "the temper is not put into the shipped word",
                format!(
                    "{} temper {temper}: {}",
                    being.name(),
                    crate::screens::temper_word(being, temper as i32)
                ),
            );
        }
    }
    "state: morning effects applied as stated, a clean night, grudges, bars and temper words"
        .to_owned()
}

/// The order the systems run in is the order they were added.
fn check_schedule(checks: &mut Checks) -> String {
    let driver = Driver::new(DEFAULT_SEED);
    let order = driver.sim.schedule_debug();
    let (start, input, draw) = (
        order.find("start"),
        order.find("apply_input"),
        order.find("draw_screen"),
    );
    checks.require(
        start.is_some() && input.is_some() && draw.is_some() && start < input && input < draw,
        "the systems do not run startup, then input, then drawing",
        order.clone(),
    );
    "schedule: start, then apply_input, then draw_screen".to_owned()
}

/// Run every check; print the verdict line, the summary, and the frame transcript.
pub fn run() -> ExitCode {
    let mut checks = Checks::default();
    let mut summary = vec![
        check_schedule(&mut checks),
        check_lore(&mut checks),
        check_rules(&mut checks),
        check_plan(&mut checks),
        check_call_decision(&mut checks, DEFAULT_SEED),
        check_morning_decision(&mut checks, DEFAULT_SEED),
        check_input(&mut checks),
        check_state(&mut checks),
        check_replay(&mut checks),
    ];
    summary.extend(check_players(&mut checks));
    summary.push(check_screens(&mut checks));
    summary.push(check_floors_bite(&mut checks));
    let mut driver = Driver::new(DEFAULT_SEED);
    driver.option(ACTIONS.len() - 2);
    let frame = driver.draw();
    let (_, clearance) = judge_screen(&mut checks, "the capture's screen", &mut driver);
    summary.push(format!(
        "capture screen: closest quad to the edge {clearance:.2} units"
    ));
    let captured =
        crate::capture::capture_frame(&mut checks, &frame, driver.recorder.font_texture());
    let (passed, failed) = checks.counts();
    if failed == 0 {
        println!(
            "verified call-of-cthulhu: the rules, both decision rows, three players and every screen hold, {passed} checks"
        );
    } else {
        println!(
            "verify call-of-cthulhu FAILED: {failed} of {} checks",
            passed + failed
        );
    }
    for line in &summary {
        println!("  {line}");
    }
    println!("  capture: {captured}");
    println!();
    println!("{}", frame.transcript());
    checks.verdict()
}
