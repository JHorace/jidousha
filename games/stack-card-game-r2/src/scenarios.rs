//! The verify run's fixed scenarios: one per decision row of the spec's table,
//! played through the real input path, and the staged result screens.
//!
//! Expectations here are shipped literals — "2 damage", "resolves 1st, 3rd,
//! 2nd" — never arithmetic over the constant being checked, so a mutated
//! constant is a failed check rather than a check that moved with it.

use jidousha::prelude::*;

use crate::Game;
use crate::cards::Card;
use crate::checks::{Checks, fail};
use crate::contracts::{effects, force};
use crate::duel::{Duel, Effect, Event, Outcome, Side, Step, legal_targets};
use crate::resolve::preview;
use crate::screen::{self, card_box, stack_row};
use crate::verify::{Driver, Shot, judge, shoot};
use crate::{config, register};

/// What the decision-row scenarios report, and the frame worth a picture.
pub(crate) struct Rows {
    pub(crate) lines: Vec<String>,
    pub(crate) picture: Option<Shot>,
}

/// A headless match on a staged duel: the Weaver's hand and energy, the
/// Brute's hand and energy, as given.
fn stage(you: &[Card], you_energy: u32, npc: &[Card], npc_energy: u32) -> HeadlessSim {
    let mut duel = Duel::staged(Vec::new(), Vec::new(), Rng::from_seed(11));
    duel.seat_mut(Side::You).hand = you.to_vec();
    duel.seat_mut(Side::You).energy = you_energy;
    duel.seat_mut(Side::Npc).hand = npc.to_vec();
    duel.seat_mut(Side::Npc).energy = npc_energy;
    let mut sim = headless(config(), register);
    sim.world_mut().insert_resource(Game {
        duel,
        ..Game::new(0)
    });
    sim
}

/// Tick until the Weaver holds priority with nothing queued, or give up.
fn settle(sim: &mut HeadlessSim, driver: &mut Driver) {
    for _ in 0..400 {
        driver.tick(sim);
        let game = sim.world().resource::<Game>();
        if driver.idle()
            && game.ui.pick.is_none()
            && (game.duel.priority == Side::You || game.duel.over.is_some())
        {
            return;
        }
    }
    fail(
        "a scenario never gave the Weaver priority back",
        "400 ticks with the driver or the Brute still busy",
    );
}

/// Tick until every queued key is pressed and released.
fn drain(sim: &mut HeadlessSim, driver: &mut Driver) {
    while !driver.idle() {
        driver.tick(sim);
    }
}

fn duel_of(sim: &HeadlessSim) -> Duel {
    sim.world().resource::<Game>().duel.clone()
}

/// Pass until the stack is empty; every resolution in order.
fn pass_out(sim: &mut HeadlessSim, driver: &mut Driver) -> Vec<Step> {
    let seen = duel_of(sim).log.len();
    for _ in 0..12 {
        if duel_of(sim).stack.is_empty() {
            break;
        }
        driver.key(Key::Space);
        settle(sim, driver);
    }
    duel_of(sim).log[seen..]
        .iter()
        .filter_map(|event| match event {
            Event::Resolved(step) => Some(*step),
            _ => None,
        })
        .collect()
}

/// The ids in resolution order, as the stack panel states it at a window.
fn shown_order(duel: &Duel) -> Vec<u32> {
    let mut lines = screen::stack_lines(duel);
    lines.retain(|line| line.order.is_some());
    lines.sort_by_key(|line| line.order);
    lines.iter().map(|line| line.id).collect()
}

/// Every stack line's text is a row of the panel, and on the frame.
fn on_screen(checks: &mut Checks, label: &str, shot: &Shot) {
    let panel = screen::panel(&shot.duel, &shot.ui);
    for line in screen::stack_lines(&shot.duel) {
        checks.require(
            panel.runs.iter().any(|run| run.text == line.text),
            "a stack item is missing from the stack panel at a priority window",
            format!("{label}: {:?} not among the panel's rows", line.text),
        );
    }
    judge(checks, label, shot);
}

/// One scenario per row of the spec's decision table.
pub(crate) fn decision_rows(checks: &mut Checks) -> Rows {
    let mut lines = Vec::new();

    // Row 1 — respond, and with what: three items, every one and its order on
    // the panel, then resolved against the preview.
    let mut sim = stage(&[Card::Bolt, Card::Cancel], 3, &[Card::Cancel], 3);
    let mut driver = Driver::new();
    settle(&mut sim, &mut driver);
    let duel = duel_of(&sim);
    driver.act(
        &duel,
        crate::npc::Action::Play {
            slot: 0,
            target: None,
        },
    );
    settle(&mut sim, &mut driver);
    let duel = duel_of(&sim);
    let brute_cancel = duel
        .stack
        .last()
        .map(|item| (item.card, item.caster, item.id));
    checks.require(
        brute_cancel.map(|found| (found.0, found.1)) == Some((Card::Cancel, Side::Npc)),
        "the Brute does not answer a Bolt with its Cancel",
        format!("top of the stack {brute_cancel:?}"),
    );
    let target = brute_cancel.map(|found| found.2);
    driver.act(&duel, crate::npc::Action::Play { slot: 0, target });
    settle(&mut sim, &mut driver);
    let window = shoot(&mut sim);
    let ahead = preview(&window.duel);
    let shown: Vec<String> = screen::stack_lines(&window.duel)
        .iter()
        .map(|line| line.text.clone())
        .collect();
    checks.require(
        window.duel.stack.len() >= 3,
        "the three-item scenario did not build three items",
        format!("{} items: {shown:?}", window.duel.stack.len()),
    );
    on_screen(checks, "row 1, three on the stack", &window);
    checks.require(
        shown.len() == 3
            && shown[0].starts_with("1st")
            && shown[1].starts_with("--")
            && shown[2].starts_with("2nd"),
        "the panel does not state the resolution order",
        format!("rows top first {shown:?}; want 1st, --, 2nd"),
    );
    let resolved = pass_out(&mut sim, &mut driver);
    checks.require(
        resolved == ahead && resolved.last().map(|step| step.life) == Some([20, 18]),
        "row 1: the stack did not resolve as the panel previewed",
        format!(
            "previewed {:?}\nresolved {:?}",
            effects(&ahead),
            effects(&resolved)
        ),
    );
    lines.push(format!(
        "row 1 respond: {} items shown with their order, {} of {} steps as previewed",
        window.duel.stack.len(),
        resolved.iter().zip(&ahead).filter(|(a, b)| a == b).count(),
        ahead.len()
    ));

    // Row 2 — pass and let it resolve: the copy and the Surges, order-dependent.
    let mut sim = stage(&[Card::Bolt, Card::Surge, Card::Echo], 9, &[], 0);
    let mut driver = Driver::new();
    settle(&mut sim, &mut driver);
    for _ in 0..2 {
        let duel = duel_of(&sim);
        driver.act(
            &duel,
            crate::npc::Action::Play {
                slot: 0,
                target: None,
            },
        );
        settle(&mut sim, &mut driver);
    }
    let duel = duel_of(&sim);
    let surge = duel.stack.last().map(|item| item.id);
    driver.act(
        &duel,
        crate::npc::Action::Play {
            slot: 0,
            target: surge,
        },
    );
    settle(&mut sim, &mut driver);
    let window = shoot(&mut sim);
    on_screen(checks, "row 2, before the pass", &window);
    let ahead = preview(&window.duel);
    let pass_says = screen::pass_line(&window.duel);
    let resolved = pass_out(&mut sim, &mut driver);
    let amounts: Vec<Effect> = resolved.iter().map(|step| step.effect).collect();
    checks.require(
        resolved == ahead
            && pass_says == "pass: top item resolves"
            && amounts.len() == 4
            && amounts[1]
                == Effect::Damage {
                    to: Side::Npc,
                    amount: 3,
                }
            && resolved.last().map(|step| step.life) == Some([20, 13]),
        "row 2: passing did not resolve exactly as previewed",
        format!(
            "pass line {pass_says:?}\npreviewed {:?}\nresolved {amounts:?}",
            effects(&ahead)
        ),
    );
    lines.push(format!(
        "row 2 pass: {} steps resolved exactly as previewed ({pass_says})",
        resolved.len()
    ));

    // Row 3 — aim: a Sink by keys (one step down, onto the Surge under a Bolt),
    // then a Cancel by taps; the marked rows are the legal ones, and the order
    // shown is the order that resolves.
    let hand = [
        Card::Bolt,
        Card::Surge,
        Card::Bolt,
        Card::Sink,
        Card::Bolt,
        Card::Surge,
        Card::Cancel,
    ];
    let mut sim = stage(&hand, 12, &[], 0);
    let mut driver = Driver::new();
    settle(&mut sim, &mut driver);
    for _ in 0..3 {
        let duel = duel_of(&sim);
        driver.act(
            &duel,
            crate::npc::Action::Play {
                slot: 0,
                target: None,
            },
        );
        settle(&mut sim, &mut driver);
    }
    driver.key(Key::Digit1);
    drain(&mut sim, &mut driver);
    let aiming = shoot(&mut sim);
    let marked = screen::marked(&aiming.duel, &aiming.ui);
    let legal = legal_targets(&aiming.duel, Card::Sink);
    let marks_drawn = screen::panel(&aiming.duel, &aiming.ui)
        .runs
        .iter()
        .filter(|run| run.text == ">")
        .count();
    checks.require(
        marked == legal && legal.len() == 3 && marks_drawn == 3,
        "row 3: the marked targets are not the legal ones",
        format!("marked {marked:?}, legal {legal:?}, {marks_drawn} marks drawn"),
    );
    let cursor = screen::aimed(&aiming.duel, &aiming.ui);
    checks.require(
        cursor == Some(3),
        "row 3: the aim cursor does not start on the top of the stack",
        format!("cursor on {cursor:?}, the top is #3 (Bolt over Surge #2 over Bolt #1)"),
    );
    judge(checks, "row 3, aiming", &aiming);
    driver.key(Key::ArrowDown);
    driver.key(Key::Enter);
    settle(&mut sim, &mut driver);
    let window = shoot(&mut sim);
    on_screen(checks, "row 3, after the Sink", &window);
    let order = shown_order(&window.duel);
    let resolved = pass_out(&mut sim, &mut driver);
    let went: Vec<u32> = resolved.iter().map(|step| step.item).collect();
    checks.require(
        order == went && went == vec![4, 3, 1, 2],
        "row 3: the Sink's order shown is not the order that resolved",
        format!("shown {order:?}, resolved {went:?}, want [4, 3, 1, 2]"),
    );
    let sink_line = format!("Sink: shown {order:?} resolved {went:?}");

    for _ in 0..2 {
        let duel = duel_of(&sim);
        driver.act(
            &duel,
            crate::npc::Action::Play {
                slot: 0,
                target: None,
            },
        );
        settle(&mut sim, &mut driver);
    }
    // By taps: the Cancel's card, then the Bolt's row (second from the top).
    driver.tap(card_box(0).center());
    driver.tap(stack_row(1).center());
    settle(&mut sim, &mut driver);
    let window = shoot(&mut sim);
    on_screen(checks, "row 3, after the Cancel", &window);
    let top = window
        .duel
        .stack
        .last()
        .map(|item| (item.card, item.target));
    let order = shown_order(&window.duel);
    let resolved = pass_out(&mut sim, &mut driver);
    let went: Vec<u32> = resolved.iter().map(|step| step.item).collect();
    checks.require(
        top == Some((Card::Cancel, Some(5))) && order == went && went == vec![7, 6],
        "row 3: a tapped Cancel did not land on the tapped row, or resolved off its shown order",
        format!("top {top:?}, shown {order:?}, resolved {went:?}, want [7, 6]"),
    );
    lines.push(format!(
        "row 3 aim: marks == legal ({} of 3); {sink_line}; Cancel by tap: shown {order:?} \
         resolved {went:?}",
        marks_drawn
    ));

    Rows {
        lines,
        picture: Some(aiming),
    }
}

/// The three result screens, staged, judged, and told apart.
pub(crate) fn result_screens(checks: &mut Checks) -> (String, f32) {
    let mut titles = Vec::new();
    let mut clearance = f32::MAX;
    for (outcome, want) in [
        (Outcome::Won(Side::You), "YOU"),
        (Outcome::Won(Side::Npc), "BRUTE WINS"),
        (Outcome::Draw, "DRAW"),
    ] {
        let mut sim = stage(&[Card::Bolt], 3, &[], 0);
        sim.tick();
        {
            let game = sim.world_mut().resource_mut::<Game>();
            game.duel.over = Some(outcome);
            game.duel.turn = 20;
            game.ui = screen::Ui::default();
        }
        let shot = shoot(&mut sim);
        clearance = clearance.min(judge(checks, &format!("{outcome:?} screen"), &shot));
        let [title, _] = screen::result_lines(&shot.duel, outcome);
        checks.require(
            title.contains(want),
            "a result banner names the wrong result",
            format!("{outcome:?}: {title:?}, want it to say {want:?}"),
        );
        titles.push(title);
    }
    checks.require(
        titles[0] != titles[1] && titles[1] != titles[2] && titles[0] != titles[2],
        "two different results draw the same banner",
        format!("{titles:?}"),
    );
    (titles.join(" / "), clearance)
}

/// The small promises of the screen, each staged where it bites: marks only on
/// legal rows, what a pass says, an unaffordable card refused out loud, and a
/// player with nothing to play passed for.
pub(crate) fn small_screens(checks: &mut Checks) {
    // A Mirror aimed over a Bolt and a Cancel: one mark, on the Bolt's row.
    let mut sim = stage(&[Card::Mirror, Card::Blast], 2, &[], 0);
    sim.tick();
    {
        let duel = &mut sim.world_mut().resource_mut::<Game>().duel;
        let bolt = force(duel, Side::Npc, Card::Bolt, None);
        force(duel, Side::Npc, Card::Cancel, Some(bolt));
        duel.priority = Side::You;
        duel.passes = 0;
        duel.seat_mut(Side::You).energy = 2;
        duel.seat_mut(Side::Npc).hand.clear();
        duel.seat_mut(Side::Npc).energy = 0;
    }
    let mut driver = Driver::new();
    driver.key(Key::Digit1);
    drain(&mut sim, &mut driver);
    let shot = shoot(&mut sim);
    let panel = screen::panel(&shot.duel, &shot.ui);
    let marks: Vec<f32> = panel
        .runs
        .iter()
        .filter(|run| run.text == ">")
        .map(|run| run.at.y)
        .collect();
    let bolt_row = stack_row(1).min.y;
    checks.require(
        marks.len() == 1 && marks[0] >= bolt_row && marks[0] < bolt_row + screen::ROW,
        "a mark is drawn on a row the card may not aim at",
        format!("marks at y {marks:?}; the Bolt's row starts at {bolt_row}"),
    );
    let says = screen::pass_line(&shot.duel);
    checks.require(
        says == "pass: BRUTE may respond",
        "the pass line does not say the Brute may still respond",
        format!(
            "{says:?} with {} on the stack and no pass yet",
            shot.duel.stack.len()
        ),
    );

    // An unaffordable Blast, picked and played: refused, and said.
    driver.key(Key::Escape);
    driver.key(Key::Digit2);
    driver.key(Key::Enter);
    drain(&mut sim, &mut driver);
    let shot = shoot(&mut sim);
    let said = screen::panel(&shot.duel, &shot.ui)
        .all_strings()
        .any(|text| text == "Blast costs 3, you have 2 energy");
    checks.require(
        said && !shot.duel.playable(Side::You, 1),
        "an unaffordable card is offered, or its refusal is not said",
        format!("notice {:?}", shot.ui.notice),
    );
    judge(checks, "a refusal on screen", &shot);

    // Nothing playable: the Weaver is passed for within a second.
    let mut sim = stage(&[Card::Blast], 2, &[], 0);
    let mut driver = Driver::new();
    for _ in 0..60 {
        driver.tick(&mut sim);
    }
    let passed = duel_of(&sim)
        .log
        .contains(&Event::Passed { side: Side::You });
    checks.require(
        passed,
        "a player with nothing playable is left waiting",
        format!("log {:?}", duel_of(&sim).log),
    );
}
