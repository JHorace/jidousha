//! The two decision rows of the handoff, each as a scripted run read off the screen the
//! player sees (`screens::panel`, judged against the recorded frame in `verify.rs`).
//!
//! *The answer on the line*: the question, the three answers, sanity, the drain, the temper
//! and the known lore are on the screen before an answer; the numbers a known answer states
//! are what happens when it is given; an unknown answer states nothing.
//! *The morning*: each option states its effect on tonight, and the night that follows is
//! the night it stated, which differs from the other morning's night exactly as stated.
//!
//! INVARIANT: the per-being drains and wrath levels below are shipped literals from
//! `DESIGN.md`, not read from `Being`, so a changed constant cannot move the expectation.

use crate::checks::Checks;
use crate::driver::Driver;
use crate::game::{Game, Stage};
use crate::lore::{BEINGS, Being, FACTS};
use crate::rules::{
    ACTIONS, Action, Kind, answer_outcome, describe_action, option_kinds, plan_night,
};
use crate::screens::{Screen, panel};

/// Every row of a screen, one string, whitespace collapsed: the screen as a reader reads it.
pub fn text_of(screen: &Screen) -> String {
    let joined: Vec<&str> = screen.runs.iter().map(|r| r.text.as_str()).collect();
    joined
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The rows under option `n` (1-based) of a call screen, up to the next option, joined.
pub fn hint_of(screen: &Screen, n: usize) -> String {
    let head = format!("[{n}] ");
    let mut out = Vec::new();
    let mut inside = false;
    for run in &screen.runs {
        if run.text.starts_with(&head) {
            inside = true;
            continue;
        }
        if inside && run.text.starts_with('[') {
            break;
        }
        if inside && run.at.x < 560.0 {
            out.push(run.text.as_str());
        }
    }
    out.join(" ")
}

/// The number after `marker` in `text`, e.g. "Costs " -> 2.
fn number_after(text: &str, marker: &str) -> Option<i32> {
    let rest = text.split_once(marker)?.1;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

/// Shipped per-exchange drains at composure 0, from DESIGN.md's table.
const DRAINS: [(Being, i32); BEINGS] = [
    (Being::YogSothoth, 2),
    (Being::Dagon, 1),
    (Being::Nyarlathotep, 2),
];

fn drain_of(being: Being) -> i32 {
    DRAINS
        .iter()
        .find(|(b, _)| *b == being)
        .map_or(0, |(_, d)| *d)
}

/// The morning action that studies `being`, as a key index.
fn study_key(being: Being) -> usize {
    ACTIONS
        .iter()
        .position(|a| *a == Action::Study(being))
        .unwrap_or(0)
}

/// Row one: the answer on the line.
pub fn check_call_decision(checks: &mut Checks, seed: u64) -> String {
    let mut notes = Vec::new();
    // A morning that studies tonight's first caller, so the first questions are known.
    let mut run = Driver::new(seed);
    let first = plan_night(run.game())[0].being;
    run.option(study_key(first));
    let in_call = matches!(run.game().stage, Stage::Call);
    checks.require(
        in_call,
        "studying tonight's first caller does not put the first call on the line",
        format!("seed {seed:#x}: stage {:?}", run.game().stage),
    );
    if !in_call {
        return "call decision: not reached".to_owned();
    }
    let mut answered = 0;
    let mut kinds_seen = [false; 3];
    for exchange in 0..3 {
        let game = run.game().clone();
        let Some(call) = game.call.clone() else {
            break;
        };
        let screen = panel(&game);
        let text = text_of(&screen);
        let fact = &FACTS[call.plan.being.index()][call.fact_index()];
        let known = game.known[call.plan.being.index()][call.fact_index()];
        checks.require(
            known,
            "a studied fact is not known on the call screen",
            format!(
                "exchange {exchange}: fact {} of {}",
                call.fact_index(),
                call.plan.being.name()
            ),
        );
        // The question, the answers, sanity, the drain, the temper and the known lore.
        for (what, needle) in [
            ("the question", fact.question.to_owned()),
            ("the right answer", fact.right.to_owned()),
            ("the wrong answer", fact.guess.to_owned()),
            ("the insulting answer", fact.insult.to_owned()),
            (
                "sanity",
                format!("Sanity {} / {}", game.sanity, crate::rules::START_SANITY),
            ),
            (
                "the drain",
                format!(
                    "Every exchange costs {} sanity.",
                    drain_of(call.plan.being)
                        * if call.temper >= call.plan.being.wrath_at() {
                            2
                        } else {
                            1
                        }
                ),
            ),
            (
                "the temper",
                format!(
                    "Temper: {}",
                    crate::screens::temper_word(call.plan.being, call.temper)
                ),
            ),
        ] {
            checks.require(
                text.contains(&needle),
                "the call screen does not show what a choice turns on",
                format!("exchange {exchange}: {what} {needle:?} missing from {text:?}"),
            );
        }
        for known_fact in game.known_facts(call.plan.being) {
            let lore = FACTS[call.plan.being.index()][known_fact].lore;
            let flat = lore.split_whitespace().collect::<Vec<_>>().join(" ");
            checks.require(
                text.contains(&flat),
                "a fact the player knows is not on the call screen",
                format!("exchange {exchange}: {flat:?}"),
            );
        }
        // Each option states what it does, and what it states is what happens.
        let kinds = option_kinds(game.seed, game.day, call.slot, call.exchange);
        let right = kinds.iter().position(|k| *k == Kind::Right).unwrap_or(0);
        let hint = hint_of(&screen, right + 1);
        let stated_cost = number_after(&hint, "Costs ");
        let stated_shorter = number_after(&hint, "Shortens the call by ");
        checks.require(
            hint.starts_with("Right.") && stated_cost == Some(drain_of(call.plan.being)),
            "the known right answer does not state the shipped cost of an exchange",
            format!(
                "exchange {exchange}: {hint:?}, drain wanted {}",
                drain_of(call.plan.being)
            ),
        );
        let before = (game.sanity, call.remaining, call.temper);
        run.option(right);
        let after_game = run.game().clone();
        match &after_game.call {
            Some(after) if after.slot == call.slot => {
                let delta = (
                    before.0 - after_game.sanity,
                    after.remaining - before.1,
                    after.temper - before.2,
                );
                // Temper cannot fall below calm: a right answer to a calm being stays calm.
                let want_temper = if before.2 > 0 { -1 } else { 0 };
                checks.require(
                    Some(delta.0) == stated_cost
                        && Some(-delta.1) == stated_shorter
                        && delta.2 == want_temper,
                    "answering a known fact does not do what its hint stated",
                    format!("exchange {exchange}: stated cost {stated_cost:?} shorter {stated_shorter:?}; happened {delta:?}"),
                );
            }
            // The call ended: the answer shortened it to nothing, and cost what was stated.
            _ => {
                checks.require(
                    Some(before.0 - after_game.sanity) == stated_cost
                        && stated_shorter.is_some_and(|s| s >= before.1),
                    "the answer that ends a call does not do what its hint stated",
                    format!("exchange {exchange}: stated cost {stated_cost:?} shorter {stated_shorter:?}, call had {} left, sanity {}->{}", before.1, before.0, after_game.sanity),
                );
                answered += 1;
                break;
            }
        }
        answered += 1;
        // The three kinds, on the screen, exist: record that each was shown.
        for (i, kind) in kinds.iter().enumerate() {
            let shown = hint_of(&screen, i + 1);
            kinds_seen[match kind {
                Kind::Right => 0,
                Kind::Guess => 1,
                _ => 2,
            }] |= !shown.is_empty();
        }
    }
    checks.require(
        kinds_seen == [true; 3],
        "a known question does not state all three of its answers",
        format!("{kinds_seen:?}"),
    );
    notes.push(format!(
        "{answered} known answers stated their cost and shortening and did as stated"
    ));

    // The same screen with nothing known: the options state nothing, and nothing leaks.
    let mut blind = Driver::new(seed);
    blind.option(6);
    let game = blind.game().clone();
    if let Some(call) = game.call.clone() {
        let screen = panel(&game);
        let text = text_of(&screen);
        let unknown = !game.known[call.plan.being.index()][call.fact_index()];
        checks.require(
            unknown && text.contains("Nothing. Every answer is a guess."),
            "with no lore the call screen does not say every answer is a guess",
            format!("{text:?}"),
        );
        let leaks = (1..=3).any(|n| {
            let hint = hint_of(&screen, n);
            hint.contains("Costs") || hint.contains("Shortens") || hint.contains("Right.")
        });
        checks.require(
            !leaks,
            "an answer the player has no lore for states its kind or its numbers",
            format!(
                "{:?}",
                (1..=3).map(|n| hint_of(&screen, n)).collect::<Vec<_>>()
            ),
        );
        // What happens still follows the one function.
        let kinds = option_kinds(game.seed, game.day, call.slot, call.exchange);
        let outcome = answer_outcome(call.plan.being, game.composure, call.temper, kinds[0]);
        blind.option(0);
        let lost = game.sanity - blind.game().sanity;
        checks.require(
            lost == outcome.sanity_cost,
            "an unknown answer's cost is not answer_outcome's",
            format!("lost {lost}, outcome {outcome:?}"),
        );
        notes.push("unknown answers state nothing and cost what answer_outcome says".to_owned());
    }
    format!("call decision: {}", notes.join("; "))
}

/// The names in a "Tonight: A, then B." line, in order.
fn parse_tonight(text: &str) -> Option<Vec<String>> {
    let start = text.find("Tonight: ")? + "Tonight: ".len();
    let rest = &text[start..];
    let end = rest.find('.')?;
    let line = &rest[..end];
    if line == "the phone is silent" {
        return Some(Vec::new());
    }
    Some(line.split(", then ").map(str::to_owned).collect())
}

/// Row two: how to spend the morning.
pub fn check_morning_decision(checks: &mut Checks, seed: u64) -> String {
    let base = Driver::new(seed);
    let baseline = plan_night(base.game());
    let screen = panel(base.game());
    let text = text_of(&screen);
    // Every option states its effect on the screen.
    for (i, action) in ACTIONS.iter().enumerate() {
        let effect = describe_action(base.game(), *action);
        let flat = effect.text.split_whitespace().collect::<Vec<_>>().join(" ");
        checks.require(
            text.contains(&effect.label) && text.contains(&flat),
            "a morning option does not state its effect on tonight",
            format!("option {}: {:?} / {flat:?}", i + 1, effect.label),
        );
    }
    let due: Vec<Being> = baseline.iter().map(|p| p.being).collect();
    let spared = due[0];
    let mut notes = Vec::new();
    // Run A trains; run B appeases the first caller. They differ only in that choice.
    let train = ACTIONS
        .iter()
        .position(|a| *a == Action::Train)
        .unwrap_or(0);
    let appease = ACTIONS
        .iter()
        .position(|a| *a == Action::Appease(spared))
        .unwrap_or(0);
    let mut results: Vec<Vec<String>> = Vec::new();
    for (name, key) in [("train", train), ("appease", appease)] {
        let mut run = Driver::new(seed);
        let stated = text_of(&panel(run.game()));
        let option_text = effect_after(&stated, key);
        let stated_night = option_text.as_deref().and_then(parse_tonight);
        run.option(key);
        let night: Vec<String> = run
            .game()
            .night
            .iter()
            .map(|p| p.being.name().to_owned())
            .collect();
        checks.require(
            stated_night.as_ref() == Some(&night),
            "the night that follows is not the night the morning option stated",
            format!("{name}: stated {stated_night:?}, night {night:?}"),
        );
        results.push(night);
    }
    let (trained, appeased) = (&results[0], &results[1]);
    let expected_trained: Vec<String> = due.iter().map(|b| b.name().to_owned()).collect();
    checks.require(
        *trained == expected_trained && !appeased.contains(&spared.name().to_owned()),
        "two runs that differ only in appeasing the first caller do not differ by that caller",
        format!(
            "trained {trained:?}, appeased {appeased:?}, spared {}",
            spared.name()
        ),
    );
    notes.push(format!(
        "trained night {trained:?} against appeased night {appeased:?}: {} did not call",
        spared.name()
    ));
    // Studying: the studied facts are the first questions of that being's call tonight.
    let mut study = Driver::new(seed);
    let target = due[due.len() - 1];
    let key = study_key(target);
    let stated = text_of(&panel(study.game()));
    let option_text = effect_after(&stated, key).unwrap_or_default();
    checks.require(
        option_text.contains(&format!("{} asks about", target.name())),
        "a study option does not say its being asks about what it teaches",
        option_text.clone(),
    );
    study.option(key);
    let game: &Game = study.game();
    let plan = game.night.iter().find(|p| p.being == target);
    let studied = game
        .studied_today
        .as_ref()
        .map(|(_, facts)| facts.clone())
        .unwrap_or_default();
    checks.require(
        plan.is_some_and(|p| p.order.starts_with(&studied)) && !studied.is_empty(),
        "tonight's questions do not start with what the morning taught",
        format!("{plan:?} studied {studied:?}"),
    );
    notes.push(format!(
        "studying {} put its {} facts first tonight",
        target.name(),
        studied.len()
    ));
    format!("morning decision: {}", notes.join("; "))
}

/// The stated effect of option `key` on a morning screen's text: from its label to the next option.
fn effect_after(text: &str, key: usize) -> Option<String> {
    let head = format!("[{}] ", key + 1);
    let start = text.find(&head)? + head.len();
    let rest = &text[start..];
    let end = (key + 2..=8)
        .filter_map(|n| rest.find(&format!("[{n}] ")))
        .min()
        .or_else(|| rest.find("Press 1-8"))
        .unwrap_or(rest.len());
    Some(rest[..end].trim().to_owned())
}
