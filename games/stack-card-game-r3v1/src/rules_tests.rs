//! Behavioural tests of the duel's rules, one sentence each: ordering,
//! fizzling, Turn, Echo, legality and the two-pass rule.
//!
//! Key items: `duel_with` (a stack built by hand).

use super::*;

fn duel_with(stack: &[(Card, Side, Option<u32>)]) -> Duel {
    let mut duel = deal(&mut Rng::from_seed(3));
    duel.stack = stack
        .iter()
        .enumerate()
        .map(|(at, &(card, controller, target))| Item {
            id: ItemId(at as u32 + 1),
            card,
            controller,
            target: target.map(ItemId),
            copy: false,
        })
        .collect();
    duel.next_id = stack.len() as u32 + 1;
    duel
}

#[test]
fn the_same_seed_deals_the_same_hands() {
    assert_eq!(deal(&mut Rng::from_seed(9)), deal(&mut Rng::from_seed(9)));
}

#[test]
fn a_ward_resolving_after_the_hit_saves_nothing() {
    let mut duel = duel_with(&[
        (Card::Ward, Side::You, None),
        (Card::Strike, Side::Rival, None),
    ]);
    let ahead = preview(&duel);
    while resolve_top(&mut duel).is_some() {}
    assert_eq!(duel.you.life, 10);
    assert_eq!(ahead.you_life, 10);
}

#[test]
fn a_turned_haymaker_hits_its_caster() {
    let mut duel = duel_with(&[
        (Card::Haymaker, Side::Rival, None),
        (Card::Turn, Side::You, Some(1)),
    ]);
    while resolve_top(&mut duel).is_some() {}
    assert_eq!((duel.you.life, duel.rival.life), (12, 7));
}

#[test]
fn a_manipulation_whose_target_left_the_stack_fizzles() {
    let mut duel = duel_with(&[
        (Card::Strike, Side::Rival, None),
        (Card::Bury, Side::You, Some(1)),
        (Card::Cancel, Side::Rival, Some(1)),
    ]);
    let _ = resolve_top(&mut duel);
    assert_eq!(
        resolve_top(&mut duel).map(|step| step.effect),
        Some(Effect::Fizzled)
    );
}

#[test]
fn echo_puts_a_copy_for_its_caster_on_top() {
    let mut duel = duel_with(&[
        (Card::Strike, Side::Rival, None),
        (Card::Echo, Side::You, Some(1)),
    ]);
    let _ = resolve_top(&mut duel);
    let top = duel.stack.last().copied();
    assert_eq!(
        top.map(|item| (item.card, item.controller, item.copy)),
        Some((Card::Strike, Side::You, true))
    );
}

#[test]
fn bury_cannot_target_the_bottom_item_and_turn_cannot_target_your_own() {
    let duel = duel_with(&[
        (Card::Strike, Side::You, None),
        (Card::Haymaker, Side::Rival, None),
    ]);
    assert_eq!(legal_targets(&duel, Side::You, Card::Bury), vec![ItemId(2)]);
    assert_eq!(legal_targets(&duel, Side::You, Card::Turn), vec![ItemId(2)]);
    assert_eq!(
        legal_targets(&duel, Side::You, Card::Cancel),
        vec![ItemId(1), ItemId(2)]
    );
}

#[test]
fn two_passes_resolve_the_top_and_return_priority_to_the_leader() {
    let mut duel = duel_with(&[(Card::Strike, Side::Rival, None)]);
    duel.leader = Side::Rival;
    duel.priority = Side::You;
    assert_eq!(pass(&mut duel, Side::You), Ok(()));
    assert_eq!(pass(&mut duel, Side::Rival), Ok(()));
    assert!(duel.stack.is_empty());
    assert_eq!(
        (duel.you.life, duel.priority, duel.passed),
        (10, Side::Rival, false)
    );
}
