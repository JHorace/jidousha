//! The card pool: one row per card, and the two decks built from it.
//!
//! Every number a card has lives in its row, so the rules, the screen and the
//! checks read one table. `games/stack-card-game-r2/DESIGN.md` says what each
//! card is *for*; this file says what it does.

/// Every card there is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Card {
    Bolt,
    Blast,
    Surge,
    Mend,
    Cancel,
    Mirror,
    Echo,
    Sink,
    Flip,
}

/// What a card must be aimed at when it is played.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Aim {
    /// Nothing: it acts on a player, or on the stack as a whole.
    Nothing,
    /// Any item on the stack.
    AnyItem,
    /// An item that deals damage.
    DamageItem,
    /// Any item that is not an Echo.
    NotEcho,
}

/// One card's row.
pub(crate) struct Spec {
    pub(crate) card: Card,
    pub(crate) name: &'static str,
    pub(crate) cost: u32,
    pub(crate) aim: Aim,
    /// The rule as the hand row prints it — short, ASCII, and the whole rule.
    pub(crate) rule: &'static str,
}

/// The table. Order is the order a card list is printed in.
pub(crate) const CARDS: &[Spec] = &[
    Spec {
        card: Card::Bolt,
        name: "Bolt",
        cost: 1,
        aim: Aim::Nothing,
        rule: "2 damage to the foe",
    },
    Spec {
        card: Card::Blast,
        name: "Blast",
        cost: 3,
        aim: Aim::Nothing,
        rule: "6 damage to the foe",
    },
    Spec {
        card: Card::Surge,
        name: "Surge",
        cost: 2,
        aim: Aim::Nothing,
        rule: "1 damage +1 per item left on stack",
    },
    Spec {
        card: Card::Mend,
        name: "Mend",
        cost: 1,
        aim: Aim::Nothing,
        rule: "gain 3 life",
    },
    Spec {
        card: Card::Cancel,
        name: "Cancel",
        cost: 2,
        aim: Aim::AnyItem,
        rule: "counter an item",
    },
    Spec {
        card: Card::Mirror,
        name: "Mirror",
        cost: 1,
        aim: Aim::DamageItem,
        rule: "a damage item hits the other side",
    },
    Spec {
        card: Card::Echo,
        name: "Echo",
        cost: 2,
        aim: Aim::NotEcho,
        rule: "copy an item on top, as yours",
    },
    Spec {
        card: Card::Sink,
        name: "Sink",
        cost: 1,
        aim: Aim::AnyItem,
        rule: "move an item to the bottom",
    },
    Spec {
        card: Card::Flip,
        name: "Flip",
        cost: 1,
        aim: Aim::Nothing,
        rule: "reverse the stack below (2+ items)",
    },
];

/// Damage a card deals when it resolves, before anything on the stack adds to it.
pub(crate) const BOLT_DAMAGE: i32 = 2;
pub(crate) const BLAST_DAMAGE: i32 = 6;
pub(crate) const SURGE_BASE: i32 = 1;
pub(crate) const MEND_LIFE: i32 = 3;

impl Card {
    /// This card's row. Every card has one; `table_faults` proves it.
    pub(crate) fn spec(self) -> &'static Spec {
        let mut index = 0;
        while index < CARDS.len() {
            if CARDS[index].card == self {
                return &CARDS[index];
            }
            index += 1;
        }
        // INVARIANT: every variant has a row; `table_faults` in the verify run
        // asserts it, so reaching here is a table edited without its check.
        panic!("card {self:?} has no row in CARDS — add one")
    }

    pub(crate) fn name(self) -> &'static str {
        self.spec().name
    }

    pub(crate) fn cost(self) -> u32 {
        self.spec().cost
    }

    /// Whether this card deals damage when it resolves — what Mirror may aim at.
    pub(crate) fn deals_damage(self) -> bool {
        matches!(self, Card::Bolt | Card::Blast | Card::Surge)
    }
}

/// The Weaver's deck: few threats, many answers.
pub(crate) const WEAVER_DECK: &[(Card, usize)] = &[
    (Card::Bolt, 3),
    (Card::Surge, 2),
    (Card::Mend, 1),
    (Card::Cancel, 3),
    (Card::Mirror, 3),
    (Card::Echo, 2),
    (Card::Sink, 1),
    (Card::Flip, 1),
];

/// The Brute's deck: raw power, a few answers.
pub(crate) const BRUTE_DECK: &[(Card, usize)] = &[
    (Card::Blast, 5),
    (Card::Bolt, 5),
    (Card::Mend, 2),
    (Card::Cancel, 3),
    (Card::Mirror, 1),
];

/// A deck list, spelled out card by card.
pub(crate) fn spell_out(list: &[(Card, usize)]) -> Vec<Card> {
    list.iter()
        .flat_map(|&(card, copies)| std::iter::repeat_n(card, copies))
        .collect()
}
