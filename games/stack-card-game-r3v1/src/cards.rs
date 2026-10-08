//! The seven cards and the deck: cost, name, the hand's one-line blurb, and
//! which cards are payloads (they change life or shield) and which are
//! manipulations (they name a stack item). DESIGN.md's card table, as code.
//!
//! Key items: `Card`, `DECK`.

/// The deck both duellists shuffle, before the shuffle.
pub(crate) const DECK: [Card; 14] = [
    Card::Strike,
    Card::Strike,
    Card::Strike,
    Card::Haymaker,
    Card::Haymaker,
    Card::Ward,
    Card::Ward,
    Card::Cancel,
    Card::Cancel,
    Card::Bury,
    Card::Bury,
    Card::Turn,
    Card::Turn,
    Card::Echo,
];

/// The seven cards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Card {
    Strike,
    Haymaker,
    Ward,
    Cancel,
    Bury,
    Turn,
    Echo,
}

impl Card {
    pub(crate) fn cost(self) -> i32 {
        match self {
            Card::Strike | Card::Ward | Card::Bury => 1,
            Card::Cancel | Card::Turn | Card::Echo => 2,
            Card::Haymaker => 3,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Card::Strike => "Strike",
            Card::Haymaker => "Haymaker",
            Card::Ward => "Ward",
            Card::Cancel => "Cancel",
            Card::Bury => "Bury",
            Card::Turn => "Turn",
            Card::Echo => "Echo",
        }
    }

    /// The card's effect in a dozen characters, for the hand.
    pub(crate) fn blurb(self) -> &'static str {
        match self {
            Card::Strike => "2 dmg to foe",
            Card::Haymaker => "5 dmg to foe",
            Card::Ward => "+3 shield",
            Card::Cancel => "remove item",
            Card::Bury => "item to base",
            Card::Turn => "flip a hit",
            Card::Echo => "copy payload",
        }
    }

    /// Damage a payload deals when it resolves.
    pub(crate) fn damage(self) -> i32 {
        match self {
            Card::Strike => 2,
            Card::Haymaker => 5,
            _ => 0,
        }
    }

    /// A payload changes life or shield; everything else changes the stack.
    pub(crate) fn is_payload(self) -> bool {
        matches!(self, Card::Strike | Card::Haymaker | Card::Ward)
    }

    /// Whether playing it names a stack item.
    pub(crate) fn needs_target(self) -> bool {
        !self.is_payload()
    }
}
