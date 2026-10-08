//! G10's contracts for the match's rules, each named as the sentence it checks.

use super::*;

fn fresh() -> Duel {
    Duel::new(&mut Rng::from_seed(3))
}

#[test]
fn a_stack_of_ten_refuses_an_eleventh_play() {
    let mut duel = fresh();
    duel.stack = (1..=10)
        .map(|id| Item {
            id: ItemId(id),
            card: Card::Bolt,
            owner: Side::Rival,
            affects: Side::You,
            target: None,
        })
        .collect();
    duel.seats[0].hand = vec![Card::Bolt];
    assert_eq!(
        duel.apply(Action::Play {
            slot: 0,
            target: None
        }),
        Err(Illegal::StackFull)
    );
}

#[test]
fn both_passing_on_an_empty_stack_ends_the_turn_and_refills_both_energies_to_three() {
    let mut duel = fresh();
    duel.seats[0].energy = 0;
    duel.seats[1].energy = 1;
    assert_eq!(duel.apply(Action::Pass), Ok(()));
    assert_eq!(duel.apply(Action::Pass), Ok(()));
    assert_eq!((duel.turn, duel.active), (2, Side::Rival));
    assert_eq!([duel.seats[0].energy, duel.seats[1].energy], [3, 3]);
}

#[test]
fn a_draw_into_a_hand_of_six_is_skipped() {
    let mut duel = fresh();
    duel.seats[1].hand = vec![Card::Bolt; HAND_CAP];
    let deck = duel.seats[1].deck.len();
    assert_eq!(duel.apply(Action::Pass), Ok(()));
    assert_eq!(duel.apply(Action::Pass), Ok(()));
    assert_eq!(duel.seats[1].hand.len(), HAND_CAP);
    assert_eq!(duel.seats[1].deck.len(), deck);
}

#[test]
fn the_same_seed_deals_the_same_hands_twice() {
    assert_eq!(fresh(), fresh());
    assert_ne!(fresh().seats, Duel::new(&mut Rng::from_seed(4)).seats);
}
