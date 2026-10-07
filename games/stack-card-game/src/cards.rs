//! The card pool and the two decks.
//!
//! Numbers live here and nowhere else: the rules read `info`, the panel prints
//! `info`, and the verify run asserts the shipped literals in `verify.rs`
//! (never arithmetic over these).

/// One of the two players. `You` is the human, `Npc` the scripted opponent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    You,
    Npc,
}

impl Side {
    pub const BOTH: [Side; 2] = [Side::You, Side::Npc];

    pub fn other(self) -> Side {
        match self {
            Side::You => Side::Npc,
            Side::Npc => Side::You,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Side::You => 0,
            Side::Npc => 1,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Side::You => "YOU",
            Side::Npc => "NPC",
        }
    }
}

/// Every card in the game.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Card {
    Ember,
    Cleaver,
    Siege,
    Mend,
    Negate,
    Hush,
    Flip,
    Bury,
    Raise,
    Redirect,
    Echo,
}

/// What a card is, as the rules and the panel both read it.
pub struct CardInfo {
    pub name: &'static str,
    pub cost: u8,
    /// Sorcery speed: only the active player, only on an empty stack.
    pub sorcery: bool,
    /// Damage dealt to the aimed player, 0 if the card deals none.
    pub damage: i32,
    /// Life gained by the aimed player, 0 if none.
    pub heal: i32,
    /// One line of rules text for the hand.
    pub text: &'static str,
}

pub fn info(card: Card) -> CardInfo {
    let (name, cost, sorcery, damage, heal, text) = match card {
        Card::Ember => ("Ember", 1, false, 2, 0, "2 damage"),
        Card::Cleaver => ("Cleaver", 3, true, 6, 0, "6 damage"),
        Card::Siege => ("Siege", 5, true, 10, 0, "10 damage"),
        Card::Mend => ("Mend", 2, false, 0, 4, "gain 4"),
        Card::Negate => ("Negate", 2, false, 0, 0, "counter item"),
        Card::Hush => ("Hush", 1, false, 0, 0, "counter cost<=2"),
        Card::Flip => ("Flip", 1, false, 0, 0, "reverse below"),
        Card::Bury => ("Bury", 2, false, 0, 0, "item to bottom"),
        Card::Raise => ("Raise", 1, false, 0, 0, "item to top"),
        Card::Redirect => ("Redirect", 2, false, 0, 0, "flip aim"),
        Card::Echo => ("Echo", 2, false, 0, 0, "copy item"),
    };
    CardInfo {
        name,
        cost,
        sorcery,
        damage,
        heal,
        text,
    }
}

fn repeat(deck: &mut Vec<Card>, card: Card, count: usize) {
    for _ in 0..count {
        deck.push(card);
    }
}

/// The 20 cards a side starts with, in pool order (the match shuffles them).
///
/// The decks differ on purpose: the player's "Weaver" is answers and order
/// tricks with little damage; the NPC's "Hammer" is heavy sorceries with a few
/// answers. The design note says why (DESIGN.md, asymmetry).
pub fn deck(side: Side) -> Vec<Card> {
    let mut deck = Vec::new();
    let list: [(Card, usize); 10] = match side {
        Side::You => [
            (Card::Ember, 3),
            (Card::Cleaver, 2),
            (Card::Mend, 2),
            (Card::Negate, 2),
            (Card::Hush, 2),
            (Card::Flip, 2),
            (Card::Bury, 2),
            (Card::Raise, 2),
            (Card::Redirect, 2),
            (Card::Echo, 1),
        ],
        Side::Npc => [
            (Card::Ember, 3),
            (Card::Cleaver, 3),
            (Card::Siege, 2),
            (Card::Mend, 3),
            (Card::Negate, 2),
            (Card::Hush, 2),
            (Card::Redirect, 1),
            (Card::Bury, 2),
            (Card::Echo, 1),
            (Card::Flip, 1),
        ],
    };
    for (card, count) in list {
        repeat(&mut deck, card, count);
    }
    deck
}
