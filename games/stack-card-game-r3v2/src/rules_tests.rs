//! G10's contracts for the stack's rules, each named as the sentence it checks.

use super::*;
use crate::duel::{Duel, Phase, Seat};

fn item(id: u32, card: Card, owner: Side, target: Option<u32>) -> Item {
    Item {
        id: ItemId(id),
        card,
        owner,
        affects: affects_for(card, owner),
        target: target.map(ItemId),
    }
}

/// A live duel at 15 life each with this stack, bottom first.
fn duel(stack: Vec<Item>) -> Duel {
    let seat = || Seat {
        life: 15,
        energy: 3,
        hand: Vec::new(),
        deck: Vec::new(),
    };
    let next_id = stack.iter().map(|item| item.id.0).max().unwrap_or(0) + 1;
    Duel {
        seats: [seat(), seat()],
        stack,
        turn: 1,
        active: Side::You,
        priority: Side::You,
        passes: 0,
        next_id,
        phase: Phase::Live,
        log: Vec::new(),
    }
}

#[test]
fn a_counter_whose_target_already_resolved_fizzles_and_changes_no_life() {
    // Bottom first: the Counter aimed at #2, then the Bolt #2 above it — so the
    // Bolt resolves first and the Counter finds nothing to void.
    let duel = duel(vec![
        item(1, Card::Counter, Side::Rival, Some(2)),
        item(2, Card::Bolt, Side::You, None),
    ]);
    let steps = forecast(&duel).steps;
    assert_eq!(steps[1].says, "Counter: fizzles");
    assert_eq!(steps[1].life_after, steps[0].life_after);
}

#[test]
fn redirect_lists_only_effect_items_as_legal() {
    let stack = vec![
        item(1, Card::Bolt, Side::You, None),
        item(2, Card::Counter, Side::Rival, Some(1)),
        item(3, Card::Mend, Side::Rival, None),
    ];
    assert_eq!(
        legal_targets(Card::Redirect, &stack),
        vec![ItemId(3), ItemId(1)]
    );
}

#[test]
fn delay_moves_its_target_to_the_bottom_and_nothing_else_moves() {
    let mut duel = duel(vec![
        item(1, Card::Bolt, Side::You, None),
        item(2, Card::Mend, Side::Rival, None),
        item(3, Card::Blast, Side::Rival, None),
        item(4, Card::Delay, Side::You, Some(3)),
    ]);
    assert!(resolve_top(&mut duel));
    let order: Vec<u32> = duel.stack.iter().map(|item| item.id.0).collect();
    assert_eq!(order, vec![3, 1, 2]);
}

#[test]
fn copy_of_a_bolt_owned_by_the_rival_makes_a_bolt_owned_by_you_that_hits_the_rival() {
    let mut duel = duel(vec![
        item(1, Card::Bolt, Side::Rival, None),
        item(2, Card::Copy, Side::You, Some(1)),
    ]);
    assert!(resolve_top(&mut duel));
    let copy = duel.stack[1];
    assert_eq!(
        (copy.id, copy.card, copy.owner, copy.affects),
        (ItemId(3), Card::Bolt, Side::You, Side::Rival)
    );
}

#[test]
fn the_forecast_of_an_empty_stack_is_empty() {
    let forecast = forecast(&duel(Vec::new()));
    assert!(forecast.steps.is_empty() && forecast.fates.is_empty());
}

#[test]
fn the_forecast_stops_at_the_step_that_ends_the_match() {
    let mut duel = duel(vec![
        item(1, Card::Bolt, Side::You, None),
        item(2, Card::Blast, Side::You, None),
    ]);
    duel.seats[1].life = 4;
    let forecast = forecast(&duel);
    assert_eq!(forecast.steps.len(), 1);
    assert_eq!(forecast.fates[1].says, "not reached");
}
