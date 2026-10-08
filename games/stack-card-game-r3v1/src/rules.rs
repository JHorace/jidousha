//! The duel, as plain data and pure functions: cards, the stack, priority,
//! resolution, and the one preview every reader of the stack shares.
//!
//! Nothing here touches a `World`. The systems in `main.rs` call these
//! functions, the screen reads them, the rival decides with them, and the
//! `--verify` players roll them forward — so the panel, the rival and the
//! resolution cannot disagree about what the stack will do (DESIGN.md,
//! "Decision surfaces").
//!
//! Key types: `Duel`, `Item`, `Step`, `Preview`. Key functions: `deal`,
//! `play`, `pass`, `resolve_top`, `preview`, `legal_targets`.

use jidousha::prelude::*;

pub(crate) use crate::cards::{Card, DECK};

/// Life a duellist starts the match with.
pub(crate) const STARTING_LIFE: i32 = 12;
/// Focus every duellist has to spend each round.
pub(crate) const FOCUS_PER_ROUND: i32 = 3;
/// Cards in the opening hand.
pub(crate) const OPENING_HAND: usize = 5;
/// Cards drawn at the start of every round after the first.
pub(crate) const DRAW_PER_ROUND: usize = 2;
/// The most cards a hand holds; a draw past it is lost.
pub(crate) const HAND_LIMIT: usize = 7;
/// Life lost for each draw from an empty deck.
pub(crate) const EXHAUSTION: i32 = 1;
/// Shield a Ward gives.
pub(crate) const WARD_SHIELD: i32 = 3;

/// Which duellist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Side {
    You,
    Rival,
}

impl Side {
    pub(crate) fn opponent(self) -> Side {
        match self {
            Side::You => Side::Rival,
            Side::Rival => Side::You,
        }
    }

    /// How the side is written on screen and in the log.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Side::You => "you",
            Side::Rival => "rival",
        }
    }
}

/// A stack item's identity, stable while the stack is reordered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ItemId(pub(crate) u32);

/// One thing on the stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Item {
    pub(crate) id: ItemId,
    pub(crate) card: Card,
    /// Who it works for when it resolves — Turn changes this.
    pub(crate) controller: Side,
    /// The item it acts on, for a manipulation.
    pub(crate) target: Option<ItemId>,
    /// Put there by Echo rather than played from a hand.
    pub(crate) copy: bool,
}

/// One duellist's state.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Duelist {
    pub(crate) life: i32,
    pub(crate) shield: i32,
    pub(crate) focus: i32,
    pub(crate) hand: Vec<Card>,
    /// The top of the deck is the end of the `Vec`.
    pub(crate) deck: Vec<Card>,
}

/// How a match ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Won(Side),
    Draw,
}

/// What one resolution did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Effect {
    Damage {
        to: Side,
        dealt: i32,
        soaked: i32,
    },
    Shield {
        to: Side,
        amount: i32,
    },
    Cancelled {
        target: Item,
    },
    Buried {
        target: Item,
    },
    Turned {
        target: Item,
        now: Side,
    },
    Echoed {
        target: Item,
        copy: ItemId,
    },
    /// The target had already left the stack.
    Fizzled,
}

/// One item resolving: what it was and what it did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Step {
    pub(crate) item: Item,
    pub(crate) effect: Effect,
}

impl Step {
    /// What it did, as the panel and the log both say it.
    pub(crate) fn describe(&self) -> String {
        match self.effect {
            Effect::Damage { to, dealt, soaked } => {
                let who = if to == Side::You {
                    "you take"
                } else {
                    "rival takes"
                };
                if soaked > 0 {
                    format!("{who} {dealt} ({soaked} soaked)")
                } else {
                    format!("{who} {dealt}")
                }
            }
            Effect::Shield { to, amount } => format!("{} +{amount} shield", to.name()),
            Effect::Cancelled { target } => format!("removes {}", target.card.name()),
            Effect::Buried { target } => format!("sinks {} to base", target.card.name()),
            Effect::Turned { target, now } => {
                format!("{} now works for {}", target.card.name(), now.name())
            }
            Effect::Echoed { target, .. } => format!("copies {}", target.card.name()),
            Effect::Fizzled => "fizzles: target gone".to_owned(),
        }
    }
}

/// The whole match.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Duel {
    pub(crate) you: Duelist,
    pub(crate) rival: Duelist,
    /// Bottom first: the last item is the top, and resolves next.
    pub(crate) stack: Vec<Item>,
    pub(crate) priority: Side,
    /// The other duellist passed last, so a pass now resolves or ends the round.
    pub(crate) passed: bool,
    pub(crate) leader: Side,
    pub(crate) round: u32,
    pub(crate) next_id: u32,
    /// Every resolution, oldest first.
    pub(crate) resolved: Vec<Step>,
    pub(crate) outcome: Option<Outcome>,
}

/// Why a play was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Refusal {
    NotYourPriority,
    MatchOver,
    NoSuchCard,
    TooExpensive,
    IllegalTarget,
}

/// Shuffle a deck from the run's seeded generator (Fisher-Yates).
fn shuffled(rng: &mut Rng) -> Vec<Card> {
    let mut deck = DECK.to_vec();
    for index in (1..deck.len()).rev() {
        let other = rng.below(index as u32 + 1) as usize;
        deck.swap(index, other);
    }
    deck
}

fn duelist(rng: &mut Rng) -> Duelist {
    let mut deck = shuffled(rng);
    let at = deck.len() - OPENING_HAND;
    let hand = deck.split_off(at);
    Duelist {
        life: STARTING_LIFE,
        shield: 0,
        focus: FOCUS_PER_ROUND,
        hand,
        deck,
    }
}

/// A fresh match: both decks shuffled, opening hands drawn, you lead round 1.
pub(crate) fn deal(rng: &mut Rng) -> Duel {
    let you = duelist(rng);
    let rival = duelist(rng);
    Duel {
        you,
        rival,
        stack: Vec::new(),
        priority: Side::You,
        passed: false,
        leader: Side::You,
        round: 1,
        next_id: 1,
        resolved: Vec::new(),
        outcome: None,
    }
}

impl Duel {
    pub(crate) fn side(&self, side: Side) -> &Duelist {
        match side {
            Side::You => &self.you,
            Side::Rival => &self.rival,
        }
    }

    pub(crate) fn side_mut(&mut self, side: Side) -> &mut Duelist {
        match side {
            Side::You => &mut self.you,
            Side::Rival => &mut self.rival,
        }
    }

    /// Where an item sits, counted from the bottom.
    pub(crate) fn position(&self, id: ItemId) -> Option<usize> {
        self.stack.iter().position(|item| item.id == id)
    }

    /// Settle the match if somebody is out of life.
    fn settle(&mut self) {
        let you_out = self.you.life <= 0;
        let rival_out = self.rival.life <= 0;
        self.outcome = match (you_out, rival_out) {
            (true, true) => Some(Outcome::Draw),
            (true, false) => Some(Outcome::Won(Side::Rival)),
            (false, true) => Some(Outcome::Won(Side::You)),
            (false, false) => self.outcome,
        };
    }
}

/// The stack items `card`, played now by `caster`, may name — the one legality
/// rule the target marks and the resolution both read.
pub(crate) fn legal_targets(duel: &Duel, caster: Side, card: Card) -> Vec<ItemId> {
    duel.stack
        .iter()
        .enumerate()
        .filter(|(at, item)| match card {
            Card::Cancel => true,
            Card::Bury => *at > 0,
            Card::Turn => item.card.is_payload() && item.controller != caster,
            Card::Echo => item.card.is_payload(),
            Card::Strike | Card::Haymaker | Card::Ward => false,
        })
        .map(|(_, item)| item.id)
        .collect()
}

/// Play hand card `index` for `side`, naming `target` if the card needs one.
pub(crate) fn play(
    duel: &mut Duel,
    side: Side,
    index: usize,
    target: Option<ItemId>,
) -> Result<(), Refusal> {
    if duel.outcome.is_some() {
        return Err(Refusal::MatchOver);
    }
    if duel.priority != side {
        return Err(Refusal::NotYourPriority);
    }
    let Some(&card) = duel.side(side).hand.get(index) else {
        return Err(Refusal::NoSuchCard);
    };
    if card.cost() > duel.side(side).focus {
        return Err(Refusal::TooExpensive);
    }
    let target = if card.needs_target() {
        match target {
            Some(id) if legal_targets(duel, side, card).contains(&id) => Some(id),
            _ => return Err(Refusal::IllegalTarget),
        }
    } else {
        None
    };
    let duelist = duel.side_mut(side);
    duelist.hand.remove(index);
    duelist.focus -= card.cost();
    let id = ItemId(duel.next_id);
    duel.next_id += 1;
    duel.stack.push(Item {
        id,
        card,
        controller: side,
        target,
        copy: false,
    });
    duel.priority = side.opponent();
    duel.passed = false;
    Ok(())
}

/// `side` passes priority. The second pass in a row resolves the top item, or
/// ends the round on an empty stack.
pub(crate) fn pass(duel: &mut Duel, side: Side) -> Result<(), Refusal> {
    if duel.outcome.is_some() {
        return Err(Refusal::MatchOver);
    }
    if duel.priority != side {
        return Err(Refusal::NotYourPriority);
    }
    if !duel.passed {
        duel.passed = true;
        duel.priority = side.opponent();
        return Ok(());
    }
    if duel.stack.is_empty() {
        end_round(duel);
    } else {
        let _ = resolve_top(duel);
        duel.priority = duel.leader;
        duel.passed = false;
    }
    Ok(())
}

/// Resolve the top item and say what it did; `None` on an empty stack.
pub(crate) fn resolve_top(duel: &mut Duel) -> Option<Step> {
    let item = duel.stack.pop()?;
    let found = item.target.and_then(|id| duel.position(id));
    let effect = match (item.card, found) {
        (Card::Strike | Card::Haymaker, _) => {
            let to = item.controller.opponent();
            let hit = duel.side_mut(to);
            let damage = item.card.damage();
            let soaked = damage.min(hit.shield);
            hit.shield -= soaked;
            hit.life -= damage - soaked;
            Effect::Damage {
                to,
                dealt: damage - soaked,
                soaked,
            }
        }
        (Card::Ward, _) => {
            duel.side_mut(item.controller).shield += WARD_SHIELD;
            Effect::Shield {
                to: item.controller,
                amount: WARD_SHIELD,
            }
        }
        (_, None) => Effect::Fizzled,
        (Card::Cancel, Some(at)) => Effect::Cancelled {
            target: duel.stack.remove(at),
        },
        (Card::Bury, Some(at)) => {
            let target = duel.stack.remove(at);
            duel.stack.insert(0, target);
            Effect::Buried { target }
        }
        (Card::Turn, Some(at)) => {
            let target = duel.stack[at];
            duel.stack[at].controller = item.controller;
            Effect::Turned {
                target,
                now: item.controller,
            }
        }
        (Card::Echo, Some(at)) => {
            let target = duel.stack[at];
            let copy = ItemId(duel.next_id);
            duel.next_id += 1;
            duel.stack.push(Item {
                id: copy,
                card: target.card,
                controller: item.controller,
                target: None,
                copy: true,
            });
            Effect::Echoed { target, copy }
        }
    };
    let step = Step { item, effect };
    duel.resolved.push(step);
    duel.settle();
    Some(step)
}

/// Close the round: shields fall, focus refills, leadership swaps, both draw.
fn end_round(duel: &mut Duel) {
    duel.round += 1;
    duel.leader = duel.leader.opponent();
    duel.priority = duel.leader;
    duel.passed = false;
    for side in [Side::You, Side::Rival] {
        let duelist = duel.side_mut(side);
        duelist.shield = 0;
        duelist.focus = FOCUS_PER_ROUND;
        for _ in 0..DRAW_PER_ROUND {
            match duelist.deck.pop() {
                Some(card) if duelist.hand.len() < HAND_LIMIT => duelist.hand.push(card),
                Some(_) => {}
                None => duelist.life -= EXHAUSTION,
            }
        }
    }
    duel.settle();
}

/// What the stack will do if both duellists pass all the way down.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Preview {
    /// Every resolution in order — copies Echo makes included.
    pub(crate) steps: Vec<Step>,
    pub(crate) you_life: i32,
    pub(crate) rival_life: i32,
    pub(crate) outcome: Option<Outcome>,
}

impl Preview {
    /// Which place in the order `id` resolves at (0 = first), if it does.
    pub(crate) fn order_of(&self, id: ItemId) -> Option<usize> {
        self.steps.iter().position(|step| step.item.id == id)
    }
}

/// The one resolution-preview function: the stack panel, the rival and the
/// verify players all read it, and it is `resolve_top`, run on a copy.
pub(crate) fn preview(duel: &Duel) -> Preview {
    let mut ahead = duel.clone();
    let mut steps = Vec::new();
    while ahead.outcome.is_none() {
        let Some(step) = resolve_top(&mut ahead) else {
            break;
        };
        steps.push(step);
    }
    Preview {
        steps,
        you_life: ahead.you.life,
        rival_life: ahead.rival.life,
        outcome: ahead.outcome,
    }
}

#[cfg(test)]
#[path = "rules_tests.rs"]
mod tests;
