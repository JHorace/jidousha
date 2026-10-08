//! The pool: seven cards, what each costs and does, and the two unequal decks.
//!
//! Plain data. What a card *does* when it resolves is `rules.rs`; this file is
//! only what a card is. The decks are deliberately unequal — the Rival holds
//! 35 points of raw damage to your 14 — because the game's claim is that good
//! sequencing beats raw power, and the sweep in `verify` measures exactly that.

/// One kind of card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Card {
    /// Effect: its target seat loses 3.
    Bolt,
    /// Effect: its target seat loses 5.
    Blast,
    /// Effect: its target seat gains 3.
    Mend,
    /// Stack card: its target leaves the stack unresolved.
    Counter,
    /// Stack card: its target moves to the bottom of the stack.
    Delay,
    /// Stack card: its target effect item hits the other seat instead.
    Redirect,
    /// Stack card: a copy of its target, owned by the caster, goes on top.
    Copy,
}

impl Card {
    /// Every card, in the order the pool table lists them.
    pub const ALL: [Card; 7] = [
        Card::Bolt,
        Card::Blast,
        Card::Mend,
        Card::Counter,
        Card::Delay,
        Card::Redirect,
        Card::Copy,
    ];

    /// Energy to play it.
    pub fn cost(self) -> u32 {
        match self {
            Card::Blast | Card::Copy => 2,
            Card::Bolt | Card::Mend | Card::Counter | Card::Delay | Card::Redirect => 1,
        }
    }

    /// Its name, as every surface prints it.
    pub fn name(self) -> &'static str {
        match self {
            Card::Bolt => "Bolt",
            Card::Blast => "Blast",
            Card::Mend => "Mend",
            Card::Counter => "Counter",
            Card::Delay => "Delay",
            Card::Redirect => "Redirect",
            Card::Copy => "Copy",
        }
    }

    /// The hand panel's one line: what the card does, at most 16 characters.
    pub fn blurb(self) -> &'static str {
        match self {
            Card::Bolt => "Rival loses 3",
            Card::Blast => "Rival loses 5",
            Card::Mend => "you gain 3",
            Card::Counter => "voids an item",
            Card::Delay => "to the bottom",
            Card::Redirect => "flips its target",
            Card::Copy => "copies, as yours",
        }
    }

    /// Whether it acts on a seat (an effect card) rather than on the stack.
    pub fn is_effect(self) -> bool {
        matches!(self, Card::Bolt | Card::Blast | Card::Mend)
    }

    /// How much life it moves when it resolves: negative is damage.
    pub fn life_change(self) -> i32 {
        match self {
            Card::Bolt => -3,
            Card::Blast => -5,
            Card::Mend => 3,
            Card::Counter | Card::Delay | Card::Redirect | Card::Copy => 0,
        }
    }
}

/// Your sixteen: little damage, every answer.
pub const YOUR_DECK: [(Card, usize); 7] = [
    (Card::Bolt, 3),
    (Card::Blast, 1),
    (Card::Mend, 1),
    (Card::Counter, 4),
    (Card::Delay, 3),
    (Card::Redirect, 2),
    (Card::Copy, 2),
];

/// The Rival's sixteen: the damage, and only two kinds of answer.
pub const RIVAL_DECK: [(Card, usize); 5] = [
    (Card::Bolt, 5),
    (Card::Blast, 3),
    (Card::Mend, 2),
    (Card::Counter, 4),
    (Card::Redirect, 2),
];

/// A deck list spelled out card by card, in table order.
pub fn spelled(list: &[(Card, usize)]) -> Vec<Card> {
    list.iter()
        .flat_map(|(card, count)| std::iter::repeat_n(*card, *count))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_decks_hold_sixteen_cards() {
        assert_eq!(spelled(&YOUR_DECK).len(), 16);
        assert_eq!(spelled(&RIVAL_DECK).len(), 16);
    }

    #[test]
    fn every_blurb_fits_sixteen_characters() {
        for card in Card::ALL {
            assert!(card.blurb().len() <= 16, "{:?}", card.blurb());
        }
    }
}
