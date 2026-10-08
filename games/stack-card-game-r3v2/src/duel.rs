//! The match: two seats, the stack, whose turn it is and who holds priority,
//! and the one function every input goes through, `Duel::apply`.
//!
//! Plain `Clone` data with no world in it, so the forecast, the Rival and the
//! check's players roll copies of it forward as often as they like
//! (jidousha-api.md, "Write the two decisions a check will want as free
//! functions"). The rules are DESIGN.md's "The rules", in the order it states
//! them.

use std::fmt;

use jidousha::prelude::*;

use crate::cards::{Card, RIVAL_DECK, YOUR_DECK, spelled};
use crate::rules::{Item, ItemId, Side, affects_for, legal_targets, resolve_top};

/// Life at the start of a match.
pub const STARTING_LIFE: i32 = 15;
/// Energy each seat has at the start of every turn; none carries over.
pub const ENERGY: u32 = 3;
/// Cards dealt to each seat before turn 1.
pub const OPENING_HAND: usize = 4;
/// The most cards a hand holds; a draw into a full hand is skipped.
pub const HAND_CAP: usize = 6;
/// The most items the stack holds; an eleventh play is refused.
pub const STACK_CAP: usize = 10;
/// The last turn; after it ends the higher life wins.
pub const LAST_TURN: u32 = 12;

/// One seat's life, energy and cards.
#[derive(Clone, Debug, PartialEq)]
pub struct Seat {
    /// Life; 0 or below loses.
    pub life: i32,
    /// Energy left this turn.
    pub energy: u32,
    /// The hand, by slot.
    pub hand: Vec<Card>,
    /// The deck; the top card is the last.
    pub deck: Vec<Card>,
}

/// How a match ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// You won.
    YouWin,
    /// The Rival won.
    RivalWins,
    /// Neither did.
    Draw,
}

/// Whether the match is still being played.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// It is.
    Live,
    /// It is over, and this is how.
    Over(Outcome),
}

/// What the seat holding priority does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Let the other seat act; two in a row resolve the top item or end the turn.
    Pass,
    /// Play the card in this hand slot, aimed at `target` if it is a stack card.
    Play {
        /// Which hand slot, from 0.
        slot: usize,
        /// The item a stack card acts on; `None` for an effect card.
        target: Option<ItemId>,
    },
}

/// Why an action was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Illegal {
    /// The match is over.
    MatchOver,
    /// The hand has no such slot.
    NoSuchSlot(usize),
    /// The card costs more energy than is left.
    CannotAfford(Card),
    /// The stack is full.
    StackFull,
    /// A stack card was aimed at something it may not act on, or at nothing.
    BadTarget(Card),
}

impl fmt::Display for Illegal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Illegal::MatchOver => write!(f, "the match is over"),
            Illegal::NoSuchSlot(slot) => write!(f, "there is no hand slot {}", slot + 1),
            Illegal::CannotAfford(card) => write!(f, "{} costs {}", card.name(), card.cost()),
            Illegal::StackFull => write!(f, "the stack already holds {STACK_CAP} items"),
            Illegal::BadTarget(card) => write!(f, "{} has no such target", card.name()),
        }
    }
}

/// One whole match.
#[derive(Clone, Debug, PartialEq)]
pub struct Duel {
    /// You, then the Rival.
    pub seats: [Seat; 2],
    /// The stack, bottom first; the top is the last.
    pub stack: Vec<Item>,
    /// Which turn, from 1.
    pub turn: u32,
    /// Whose turn it is.
    pub active: Side,
    /// Who may act now.
    pub priority: Side,
    /// Passes in a row since the last play or resolution.
    pub passes: u32,
    /// The next item number.
    pub next_id: u32,
    /// Live, or how it ended.
    pub phase: Phase,
    /// One line per resolution, oldest first, kept whole.
    pub log: Vec<String>,
}

impl Duel {
    /// A new match: both decks shuffled (yours first), four cards each, your turn 1.
    pub fn new(rng: &mut Rng) -> Self {
        let mut seat = |list: &[(Card, usize)]| {
            let mut deck = spelled(list);
            for index in (1..deck.len()).rev() {
                let other = rng.below(index as u32 + 1) as usize;
                deck.swap(index, other);
            }
            let hand = (0..OPENING_HAND).filter_map(|_| deck.pop()).collect();
            Seat {
                life: STARTING_LIFE,
                energy: ENERGY,
                hand,
                deck,
            }
        };
        let you = seat(&YOUR_DECK);
        let rival = seat(&RIVAL_DECK);
        Duel {
            seats: [you, rival],
            stack: Vec::new(),
            turn: 1,
            active: Side::You,
            priority: Side::You,
            passes: 0,
            next_id: 1,
            phase: Phase::Live,
            log: Vec::new(),
        }
    }

    /// Both lives, You first.
    pub fn life(&self) -> [i32; 2] {
        [self.seats[0].life, self.seats[1].life]
    }

    /// This seat.
    pub fn seat(&self, side: Side) -> &Seat {
        &self.seats[side.index()]
    }

    /// Whether `side` could play the card in `slot` right now, given priority.
    pub fn playable(&self, side: Side, slot: usize) -> bool {
        let seat = self.seat(side);
        let Some(&card) = seat.hand.get(slot) else {
            return false;
        };
        card.cost() <= seat.energy
            && self.stack.len() < STACK_CAP
            && (card.is_effect() || !legal_targets(card, &self.stack).is_empty())
    }

    /// Do `action` for the seat holding priority.
    pub fn apply(&mut self, action: Action) -> Result<(), Illegal> {
        if self.phase != Phase::Live {
            return Err(Illegal::MatchOver);
        }
        match action {
            Action::Pass => {
                self.passes += 1;
                self.priority = self.priority.other();
                if self.passes >= 2 {
                    self.passes = 0;
                    self.priority = self.active;
                    if resolve_top(self) {
                        self.after_resolution();
                    } else {
                        self.end_turn();
                    }
                }
                Ok(())
            }
            Action::Play { slot, target } => self.play(slot, target),
        }
    }

    fn play(&mut self, slot: usize, target: Option<ItemId>) -> Result<(), Illegal> {
        let side = self.priority;
        let Some(&card) = self.seat(side).hand.get(slot) else {
            return Err(Illegal::NoSuchSlot(slot));
        };
        if card.cost() > self.seat(side).energy {
            return Err(Illegal::CannotAfford(card));
        }
        if self.stack.len() >= STACK_CAP {
            return Err(Illegal::StackFull);
        }
        let aimed = match target {
            None => card.is_effect(),
            Some(id) => legal_targets(card, &self.stack).contains(&id),
        };
        if !aimed {
            return Err(Illegal::BadTarget(card));
        }
        let seat = &mut self.seats[side.index()];
        seat.hand.remove(slot);
        seat.energy -= card.cost();
        self.stack.push(Item {
            id: ItemId(self.next_id),
            card,
            owner: side,
            affects: affects_for(card, side),
            target,
        });
        self.next_id += 1;
        self.passes = 0;
        self.priority = side.other();
        Ok(())
    }

    /// After a resolution: a seat at 0 or below ends the match, the rest of the
    /// stack discarded unresolved.
    fn after_resolution(&mut self) {
        let [you, rival] = self.life();
        let outcome = match (you <= 0, rival <= 0) {
            (true, true) => Outcome::Draw,
            (false, true) => Outcome::YouWin,
            (true, false) => Outcome::RivalWins,
            (false, false) => return,
        };
        self.stack.clear();
        self.phase = Phase::Over(outcome);
    }

    /// Both passed on an empty stack: the next turn, or the end of turn 12.
    fn end_turn(&mut self) {
        if self.turn >= LAST_TURN {
            let [you, rival] = self.life();
            self.phase = Phase::Over(match you.cmp(&rival) {
                std::cmp::Ordering::Greater => Outcome::YouWin,
                std::cmp::Ordering::Less => Outcome::RivalWins,
                std::cmp::Ordering::Equal => Outcome::Draw,
            });
            return;
        }
        self.turn += 1;
        self.active = self.active.other();
        self.priority = self.active;
        self.passes = 0;
        for side in [self.active, self.active.other()] {
            let seat = &mut self.seats[side.index()];
            seat.energy = ENERGY;
            if seat.hand.len() < HAND_CAP
                && let Some(card) = seat.deck.pop()
            {
                seat.hand.push(card);
            }
        }
    }
}

#[cfg(test)]
#[path = "duel_tests.rs"]
mod tests;
