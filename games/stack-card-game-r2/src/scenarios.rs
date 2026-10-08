//! The verify run's fixed scenarios: the card table, the rules' contracts, one
//! scenario per decision row of the spec's table, and the staged result screens.
//!
//! Expectations here are shipped literals — "2 damage", "resolves 1st, 3rd,
//! 2nd" — never arithmetic over the constant being checked, so a mutated
//! constant is a failed check rather than a check that moved with it.

use jidousha::prelude::*;

use crate::Game;
use crate::cards::{CARDS, Card};
use crate::checks::{Checks, fail};
use crate::duel::{Duel, Effect, Event, Outcome, Side, Step, legal_targets, preview, resolve_top};
use crate::screen::{self, card_box, stack_row};
use crate::verify::{Driver, Shot, judge, shoot};
use crate::{config, register};

/// Every card has one row, every rule fits its card and is printable.
pub(crate) fn tables(checks: &mut Checks) {
    let all = [
        Card::Bolt,
        Card::Blast,
        Card::Surge,
        Card::Mend,
        Card::Cancel,
        Card::Mirror,
        Card::Echo,
        Card::Sink,
        Card::Flip,
    ];
    for card in all {
        let rows = CARDS.iter().filter(|spec| spec.card == card).count();
        checks.require(
            rows == 1,
            "a card does not have exactly one row",
            format!("{card:?} has {rows}"),
        );
    }
    for spec in CARDS {
        checks.require(
            spec.rule.chars().all(|c| (' '..='~').contains(&c)) && spec.rule.len() <= 42,
            "a card's rule will not print on its card",
            format!(
                "{}: {:?} ({} chars, 42 fit)",
                spec.name,
                spec.rule,
                spec.rule.len()
            ),
        );
    }
}

/// Put `card` in `side`'s hand and play it at `target`, whoever held priority.
pub(crate) fn force(duel: &mut Duel, side: Side, card: Card, target: Option<u32>) -> u32 {
    duel.priority = side;
    let seat = duel.seat_mut(side);
    seat.energy = 99;
    seat.hand.push(card);
    let slot = seat.hand.len() - 1;
    match duel.play(side, slot, target) {
        Ok(id) => id,
        Err(refusal) => fail("a staged play was refused", &refusal.line()),
    }
}

fn effects(steps: &[Step]) -> Vec<(u32, Effect)> {
    steps.iter().map(|step| (step.item, step.effect)).collect()
}

fn quiet() -> Duel {
    Duel::staged(Vec::new(), Vec::new(), Rng::from_seed(7))
}

/// The rules asked directly, for the cases a played match may never reach.
pub(crate) fn contracts(checks: &mut Checks) {
    // Mirror turns a Bolt onto its own caster: 2 damage, to the Weaver.
    let mut duel = quiet();
    let bolt = force(&mut duel, Side::You, Card::Bolt, None);
    force(&mut duel, Side::Npc, Card::Mirror, Some(bolt));
    let before = (duel.stack.clone(), duel.life(), duel.log.len());
    let steps = preview(&duel);
    checks.require(
        before == (duel.stack.clone(), duel.life(), duel.log.len()),
        "previewing the stack changed the duel",
        format!(
            "{before:?} became {:?}",
            (duel.stack.clone(), duel.life(), duel.log.len())
        ),
    );
    checks.require(
        steps.last().map(|step| step.life) == Some([18, 20]),
        "Mirror does not turn a Bolt onto its caster",
        format!("{steps:?}"),
    );

    // Surge with three others under it deals 4.
    let mut duel = quiet();
    for _ in 0..3 {
        force(&mut duel, Side::Npc, Card::Mend, None);
    }
    force(&mut duel, Side::You, Card::Surge, None);
    let first = resolve_top(&mut duel).map(|step| step.effect);
    checks.require(
        first
            == Some(Effect::Damage {
                to: Side::Npc,
                amount: 4,
            }),
        "Surge does not count the items left under it",
        format!("{first:?}"),
    );

    // Flip over A, B, C makes them resolve A, B, C — bottom first.
    let mut duel = quiet();
    let ids: Vec<u32> = (0..3)
        .map(|_| force(&mut duel, Side::You, Card::Mend, None))
        .collect();
    let flip = force(&mut duel, Side::Npc, Card::Flip, None);
    let order: Vec<u32> = preview(&duel).iter().map(|step| step.item).collect();
    checks.require(
        order == vec![flip, ids[0], ids[1], ids[2]],
        "Flip does not reverse the stack below it",
        format!("resolves {order:?}, items {ids:?}, flip {flip}"),
    );

    // Echo's copy resolves first, as the Echo's caster's.
    let mut duel = quiet();
    let blast = force(&mut duel, Side::Npc, Card::Blast, None);
    force(&mut duel, Side::You, Card::Echo, Some(blast));
    let steps = preview(&duel);
    checks.require(
        steps
            .get(1)
            .map(|step| (step.card, step.caster, step.effect))
            == Some((
                Card::Blast,
                Side::You,
                Effect::Damage {
                    to: Side::Npc,
                    amount: 6,
                },
            ))
            && steps.last().map(|step| step.life) == Some([14, 14]),
        "Echo's copy does not resolve first, as its caster's",
        format!("{steps:?}"),
    );

    // A Cancel whose target is gone fizzles.
    let mut duel = quiet();
    let bolt = force(&mut duel, Side::You, Card::Bolt, None);
    force(&mut duel, Side::Npc, Card::Cancel, Some(bolt));
    let second = force(&mut duel, Side::You, Card::Cancel, Some(bolt));
    let steps = effects(&preview(&duel));
    checks.require(
        steps.get(1).map(|step| step.1) == Some(Effect::Fizzled) && steps[0].0 == second,
        "a Cancel whose target already left does not fizzle",
        format!("{steps:?}"),
    );

    // The rules refuse what the screen would never offer.
    let mut duel = quiet();
    let bolt = force(&mut duel, Side::You, Card::Bolt, None);
    force(&mut duel, Side::Npc, Card::Cancel, Some(bolt));
    checks.require(
        legal_targets(&duel, Card::Mirror) == vec![bolt]
            && legal_targets(&duel, Card::Sink).len() == 2,
        "Mirror may aim at a Cancel, or Sink may not aim at everything",
        format!(
            "Mirror {:?}, Sink {:?}",
            legal_targets(&duel, Card::Mirror),
            legal_targets(&duel, Card::Sink)
        ),
    );
    duel.priority = Side::Npc;
    duel.seat_mut(Side::You).hand.push(Card::Blast);
    let slot = duel.seat(Side::You).hand.len() - 1;
    checks.require(
        duel.play(Side::You, slot, None).is_err(),
        "a play without priority was accepted",
        "the Weaver played while the Brute held priority".to_owned(),
    );

    // A turn ends on two passes over an empty stack; the last turn ends the match.
    let mut duel = quiet();
    let turn = duel.turn;
    let (a, b) = (duel.pass(Side::You), duel.pass(Side::Npc));
    checks.require(
        a.is_ok() && b.is_ok() && duel.turn == turn + 1 && duel.active == Side::Npc,
        "two passes on an empty stack do not end the turn",
        format!("turn {turn} -> {}, active {:?}", duel.turn, duel.active),
    );
    duel.turn = 20;
    duel.seat_mut(Side::Npc).life = 19;
    let (holder, other) = (duel.priority, duel.priority.other());
    let _ = (duel.pass(holder), duel.pass(other));
    checks.require(
        duel.over == Some(Outcome::Won(Side::You)),
        "the match does not end after turn 20 on life",
        format!(
            "over {:?} at turn {}, life {:?}",
            duel.over,
            duel.turn,
            duel.life()
        ),
    );
}

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

    // Row 3 — aim: a Sink by keys, then a Cancel by taps; the marked rows are
    // the legal ones, and the order shown is the order that resolves.
    let hand = [
        Card::Bolt,
        Card::Surge,
        Card::Sink,
        Card::Bolt,
        Card::Surge,
        Card::Cancel,
    ];
    let mut sim = stage(&hand, 12, &[], 0);
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
        marked == legal && legal.len() == 2 && marks_drawn == 2,
        "row 3: the marked targets are not the legal ones",
        format!("marked {marked:?}, legal {legal:?}, {marks_drawn} marks drawn"),
    );
    judge(checks, "row 3, aiming", &aiming);
    driver.key(Key::Enter);
    settle(&mut sim, &mut driver);
    let window = shoot(&mut sim);
    on_screen(checks, "row 3, after the Sink", &window);
    let order = shown_order(&window.duel);
    let resolved = pass_out(&mut sim, &mut driver);
    let went: Vec<u32> = resolved.iter().map(|step| step.item).collect();
    checks.require(
        order == went && went == vec![3, 1, 2],
        "row 3: the Sink's order shown is not the order that resolved",
        format!("shown {order:?}, resolved {went:?}, want [3, 1, 2]"),
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
        top == Some((Card::Cancel, Some(4))) && order == went && went == vec![6, 5],
        "row 3: a tapped Cancel did not land on the tapped row, or resolved off its shown order",
        format!("top {top:?}, shown {order:?}, resolved {went:?}, want [6, 5]"),
    );
    lines.push(format!(
        "row 3 aim: marks == legal ({} of 2); {sink_line}; Cancel by tap: shown {order:?} \
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
