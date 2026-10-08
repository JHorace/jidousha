//! The rules asked directly: the card table, and the contracts a played match
//! may never reach — a Mirror, a Surge's count, a Flip, an Echo's copy, a
//! fizzle, the refusals, the turn and the match ending.
//!
//! Expectations are shipped literals, never arithmetic over the constant under
//! test, so a mutated constant fails a check rather than moving it.

use jidousha::prelude::*;

use crate::cards::{CARDS, Card};
use crate::checks::{Checks, fail};
use crate::duel::{Duel, Effect, Outcome, Side, Step, legal_targets};
use crate::resolve::{preview, resolve_top};

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

pub(crate) fn effects(steps: &[Step]) -> Vec<(u32, Effect)> {
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
    let refilled = [Side::You, Side::Npc].map(|side| {
        let seat = duel.seat(side);
        (seat.energy, seat.hand.len())
    });
    checks.require(
        refilled == [(3, 0), (3, 0)],
        "a new turn does not refill both players to 3 energy",
        format!("(energy, hand) for each, from empty decks: {refilled:?}"),
    );
    duel.turn = 19;
    duel.seat_mut(Side::Npc).life = 19;
    let (holder, other) = (duel.priority, duel.priority.other());
    let _ = (duel.pass(holder), duel.pass(other));
    let at_twenty = (duel.turn, duel.over);
    let (holder, other) = (duel.priority, duel.priority.other());
    let _ = (duel.pass(holder), duel.pass(other));
    checks.require(
        at_twenty == (20, None) && duel.over == Some(Outcome::Won(Side::You)),
        "the match does not end after turn 20, on life",
        format!(
            "after turn 19: {at_twenty:?}; after turn 20: over {:?} at turn {}, life {:?}",
            duel.over,
            duel.turn,
            duel.life()
        ),
    );

    // Life at zero ends the match at once, mid-turn.
    let mut duel = quiet();
    for _ in 0..4 {
        force(&mut duel, Side::Npc, Card::Blast, None);
    }
    while resolve_top(&mut duel).is_some() {}
    checks.require(
        duel.over == Some(Outcome::Won(Side::Npc)) && duel.turn == 1,
        "life at zero does not end the match",
        format!(
            "over {:?} at turn {}, life {:?}",
            duel.over,
            duel.turn,
            duel.life()
        ),
    );

    // A full deck draws a hand of 4 on the first turn.
    let duel = Duel::new(3);
    let hands = [
        duel.seat(Side::You).hand.len(),
        duel.seat(Side::Npc).hand.len(),
    ];
    checks.require(
        hands == [4, 4] && duel.seat(Side::You).deck.len() == 12,
        "the first turn does not deal four cards each",
        format!(
            "hands {hands:?}, Weaver's deck {}",
            duel.seat(Side::You).deck.len()
        ),
    );
}
