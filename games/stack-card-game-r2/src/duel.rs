//! The rules, as a plain value: two players, one stack, priority.
//!
//! Nothing here knows about a world, a screen or a key. The ECS shell in
//! `main.rs` holds one `Duel` and calls `play` and `pass`; the screen and the
//! NPC read it; the verify run builds them by hand. Two free functions carry
//! the decisions the spec's table names: `legal` (what a card may be aimed at,
//! read by the target marking, by `play` and again at resolution) and
//! `resolve::resolve_top` (what the top item does — `resolve::preview` folds
//! it over a copy of the duel for the stack panel, and the real resolution
//! runs it on the duel itself, so the panel and the outcome cannot disagree).

use jidousha::prelude::*;

use crate::cards::{Aim, BRUTE_DECK, Card, WEAVER_DECK, spell_out};
use crate::resolve::resolve_top;

/// Life each player starts with.
pub(crate) const START_LIFE: i32 = 20;
/// Energy both players refill to at the start of every turn.
pub(crate) const ENERGY: u32 = 3;
/// Cards both players draw up to at the start of every turn.
pub(crate) const HAND: usize = 4;
/// Turns in a match, counting both players' — the match ends after this many.
pub(crate) const TURNS: u32 = 20;

/// One of the two players.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    /// You, the Weaver.
    You,
    /// The NPC, the Brute.
    Npc,
}

impl Side {
    pub(crate) fn other(self) -> Side {
        match self {
            Side::You => Side::Npc,
            Side::Npc => Side::You,
        }
    }

    /// The name every surface prints.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Side::You => "WEAVER",
            Side::Npc => "BRUTE",
        }
    }

    fn index(self) -> usize {
        match self {
            Side::You => 0,
            Side::Npc => 1,
        }
    }
}

/// One card on the stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Item {
    /// Unique for the whole match; what a target names.
    pub(crate) id: u32,
    pub(crate) card: Card,
    pub(crate) caster: Side,
    /// Who its damage or life lands on.
    pub(crate) hits: Side,
    /// The stack item it is aimed at, for cards that aim at one.
    pub(crate) target: Option<u32>,
    /// An Echo's copy: it goes nowhere when it leaves the stack.
    pub(crate) copy: bool,
}

/// What one resolution did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Effect {
    Damage {
        to: Side,
        amount: i32,
    },
    Heal {
        to: Side,
        amount: i32,
    },
    Countered {
        target: u32,
        card: Card,
    },
    Redirected {
        target: u32,
        card: Card,
        to: Side,
    },
    Copied {
        target: u32,
        card: Card,
        copy: u32,
    },
    Sunk {
        target: u32,
        card: Card,
    },
    Flipped {
        items: usize,
    },
    /// Its target had left the stack, or was no longer one it may aim at.
    Fizzled,
}

/// One item resolving: which, and what it did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Step {
    pub(crate) item: u32,
    pub(crate) card: Card,
    pub(crate) caster: Side,
    pub(crate) effect: Effect,
    /// Both players' life after it, `[you, npc]`.
    pub(crate) life: [i32; 2],
}

/// One player's side of the table.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Seat {
    pub(crate) life: i32,
    pub(crate) energy: u32,
    pub(crate) hand: Vec<Card>,
    pub(crate) deck: Vec<Card>,
    pub(crate) discard: Vec<Card>,
}

/// How a match ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Won(Side),
    Draw,
}

/// Something that happened, in the order it happened — the log the screen's
/// feed and the verify run both read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    Played {
        side: Side,
        card: Card,
        item: u32,
        target: Option<u32>,
    },
    Passed {
        side: Side,
    },
    Resolved(Step),
    TurnBegan {
        turn: u32,
        active: Side,
    },
}

/// Why `play` or `pass` refused — always a fact the screen can print.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    Over,
    NoPriority,
    NoSuchCard,
    TooDear { card: Card, cost: u32, energy: u32 },
    BadTarget { card: Card },
}

impl Refusal {
    pub(crate) fn line(&self) -> String {
        match self {
            Refusal::Over => "the match is over".to_owned(),
            Refusal::NoPriority => "wait - the BRUTE holds priority".to_owned(),
            Refusal::NoSuchCard => "no card in that slot".to_owned(),
            Refusal::TooDear { card, cost, energy } => {
                format!("{} costs {cost}, you have {energy} energy", card.name())
            }
            Refusal::BadTarget { card } => format!("{} has nothing it may aim at", card.name()),
        }
    }
}

/// The whole match.
#[derive(Clone, Debug)]
pub(crate) struct Duel {
    pub(crate) seats: [Seat; 2],
    /// Bottom first, top last.
    pub(crate) stack: Vec<Item>,
    pub(crate) turn: u32,
    pub(crate) active: Side,
    pub(crate) priority: Side,
    /// Passes in a row since anything was played or resolved.
    pub(crate) passes: u8,
    pub(crate) over: Option<Outcome>,
    pub(crate) log: Vec<Event>,
    pub(crate) next_id: u32,
    rng: Rng,
}

impl Duel {
    /// A fresh match, both decks shuffled from `seed`, the Weaver to begin.
    pub(crate) fn new(seed: u64) -> Duel {
        let mut rng = Rng::from_seed(seed);
        let mut you = spell_out(WEAVER_DECK);
        let mut npc = spell_out(BRUTE_DECK);
        shuffle(&mut you, &mut rng);
        shuffle(&mut npc, &mut rng);
        Duel::staged(you, npc, rng)
    }

    /// A match whose decks are exactly these, top of deck last — the verify
    /// run's fixed scenarios. Both seats draw their opening hands from them.
    pub(crate) fn staged(you: Vec<Card>, npc: Vec<Card>, rng: Rng) -> Duel {
        let seat = |deck: Vec<Card>| Seat {
            life: START_LIFE,
            energy: 0,
            hand: Vec::new(),
            deck,
            discard: Vec::new(),
        };
        let mut duel = Duel {
            seats: [seat(you), seat(npc)],
            stack: Vec::new(),
            turn: 0,
            active: Side::Npc,
            priority: Side::Npc,
            passes: 0,
            over: None,
            log: Vec::new(),
            next_id: 1,
            rng,
        };
        duel.begin_turn();
        duel
    }

    pub(crate) fn seat(&self, side: Side) -> &Seat {
        &self.seats[side.index()]
    }

    pub(crate) fn seat_mut(&mut self, side: Side) -> &mut Seat {
        &mut self.seats[side.index()]
    }

    pub(crate) fn life(&self) -> [i32; 2] {
        [self.seats[0].life, self.seats[1].life]
    }

    /// The item with this id, if it is still on the stack.
    pub(crate) fn find_item(&self, id: u32) -> Option<&Item> {
        self.stack.iter().find(|item| item.id == id)
    }

    /// Whether `side` could play the card in `slot` right now, at some target.
    pub(crate) fn playable(&self, side: Side, slot: usize) -> bool {
        let Some(&card) = self.seat(side).hand.get(slot) else {
            return false;
        };
        self.over.is_none()
            && self.priority == side
            && card.cost() <= self.seat(side).energy
            && match card.spec().aim {
                Aim::Nothing => card != Card::Flip || self.stack.len() >= 2,
                _ => !legal_targets(self, card).is_empty(),
            }
    }

    /// Whether `side` has anything at all it could play.
    pub(crate) fn has_play(&self, side: Side) -> bool {
        (0..self.seat(side).hand.len()).any(|slot| self.playable(side, slot))
    }

    /// Put the card in `slot` on top of the stack, aimed at `target`, and hand
    /// priority across.
    pub(crate) fn play(
        &mut self,
        side: Side,
        slot: usize,
        target: Option<u32>,
    ) -> Result<u32, Refusal> {
        if self.over.is_some() {
            return Err(Refusal::Over);
        }
        if self.priority != side {
            return Err(Refusal::NoPriority);
        }
        let Some(&card) = self.seat(side).hand.get(slot) else {
            return Err(Refusal::NoSuchCard);
        };
        let energy = self.seat(side).energy;
        if card.cost() > energy {
            return Err(Refusal::TooDear {
                card,
                cost: card.cost(),
                energy,
            });
        }
        let aimed_right = match card.spec().aim {
            Aim::Nothing => target.is_none() && (card != Card::Flip || self.stack.len() >= 2),
            _ => target.is_some_and(|id| legal(self, card, id)),
        };
        if !aimed_right {
            return Err(Refusal::BadTarget { card });
        }
        let seat = self.seat_mut(side);
        seat.hand.remove(slot);
        seat.energy -= card.cost();
        let id = self.next_id;
        self.next_id += 1;
        self.stack.push(Item {
            id,
            card,
            caster: side,
            hits: lands_on(card, side),
            target,
            copy: false,
        });
        self.log.push(Event::Played {
            side,
            card,
            item: id,
            target,
        });
        self.priority = side.other();
        self.passes = 0;
        Ok(id)
    }

    /// Hand priority across. The second pass in a row resolves the top item,
    /// or ends the turn if there is none.
    pub(crate) fn pass(&mut self, side: Side) -> Result<(), Refusal> {
        if self.over.is_some() {
            return Err(Refusal::Over);
        }
        if self.priority != side {
            return Err(Refusal::NoPriority);
        }
        self.log.push(Event::Passed { side });
        self.passes += 1;
        self.priority = side.other();
        if self.passes < 2 {
            return Ok(());
        }
        self.passes = 0;
        if self.stack.is_empty() {
            self.begin_turn();
        } else {
            if let Some(step) = resolve_top(self) {
                self.log.push(Event::Resolved(step));
            }
            self.priority = self.active;
        }
        Ok(())
    }

    fn begin_turn(&mut self) {
        if self.turn >= TURNS {
            let [you, npc] = self.life();
            self.over = Some(if you > npc {
                Outcome::Won(Side::You)
            } else if npc > you {
                Outcome::Won(Side::Npc)
            } else {
                Outcome::Draw
            });
            return;
        }
        self.turn += 1;
        self.active = if self.turn % 2 == 1 {
            Side::You
        } else {
            Side::Npc
        };
        self.priority = self.active;
        self.passes = 0;
        for side in [Side::You, Side::Npc] {
            self.seat_mut(side).energy = ENERGY;
            self.draw_up(side);
        }
        self.log.push(Event::TurnBegan {
            turn: self.turn,
            active: self.active,
        });
    }

    fn draw_up(&mut self, side: Side) {
        while self.seat(side).hand.len() < HAND {
            if self.seat(side).deck.is_empty() {
                let mut discards = std::mem::take(&mut self.seat_mut(side).discard);
                if discards.is_empty() {
                    return;
                }
                shuffle(&mut discards, &mut self.rng);
                self.seat_mut(side).deck = discards;
            }
            let seat = self.seat_mut(side);
            if let Some(card) = seat.deck.pop() {
                seat.hand.push(card);
            }
        }
    }

    pub(crate) fn discard(&mut self, item: &Item) {
        if !item.copy {
            self.seat_mut(item.caster).discard.push(item.card);
        }
    }

    pub(crate) fn harm(&mut self, to: Side, amount: i32) {
        self.seat_mut(to).life -= amount;
        if self.seat(to).life <= 0 && self.over.is_none() {
            self.over = Some(Outcome::Won(to.other()));
        }
    }
}

/// Who a card's damage or life lands on when `caster` plays it.
pub(crate) fn lands_on(card: Card, caster: Side) -> Side {
    if card == Card::Mend {
        caster
    } else {
        caster.other()
    }
}

/// Whether `card` may be aimed at the stack item `id` — the one legality rule.
///
/// CONTRACT: the target marking, `Duel::play` and `resolve_top`'s fizzle check
/// all call this, so an item marked on screen is an item the play accepts and
/// an item that is still legal when the card resolves.
pub(crate) fn legal(duel: &Duel, card: Card, id: u32) -> bool {
    let Some(item) = duel.find_item(id) else {
        return false;
    };
    match card.spec().aim {
        Aim::Nothing => false,
        Aim::AnyItem => true,
        Aim::DamageItem => item.card.deals_damage(),
        Aim::NotEcho => item.card != Card::Echo,
    }
}

/// Every stack item `card` may be aimed at, top first — the order the panel
/// lists them and the aim cursor walks them.
pub(crate) fn legal_targets(duel: &Duel, card: Card) -> Vec<u32> {
    duel.stack
        .iter()
        .rev()
        .filter(|item| legal(duel, card, item.id))
        .map(|item| item.id)
        .collect()
}

/// Fisher-Yates over the duel's own generator, so a seed is a whole match.
fn shuffle(cards: &mut [Card], rng: &mut Rng) {
    for index in (1..cards.len()).rev() {
        let other = rng.below(index as u32 + 1) as usize;
        cards.swap(index, other);
    }
}
