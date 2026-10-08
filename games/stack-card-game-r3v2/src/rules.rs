//! The stack: what an item is, which items a card may aim at, and what the
//! stack will do when it resolves — the three functions every other system calls.
//!
//! `forecast` is the game's one resolution-preview function. The stack panel
//! prints it, the Rival and the sequencer roll it forward, and `resolve_top`
//! *is* "apply the forecast's first step" — so what the panel says will happen
//! and what happens cannot come apart (DESIGN.md, "One function per decision
//! row"). `legal_targets` is the same promise for aiming: the marks drawn while
//! choosing, the play's refusal and the resolution's fizzle all read it.

use std::fmt;

use crate::cards::Card;
use crate::duel::{Duel, Phase};

/// One of the two seats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The player.
    You,
    /// The scripted opponent.
    Rival,
}

impl Side {
    /// Both seats, You first — the order every `[_; 2]` in this game is indexed in.
    pub const BOTH: [Side; 2] = [Side::You, Side::Rival];

    /// This seat's index into a `[_; 2]`.
    pub fn index(self) -> usize {
        match self {
            Side::You => 0,
            Side::Rival => 1,
        }
    }

    /// The other seat.
    pub fn other(self) -> Side {
        match self {
            Side::You => Side::Rival,
            Side::Rival => Side::You,
        }
    }

    /// How the screen names it.
    pub fn name(self) -> &'static str {
        match self {
            Side::You => "You",
            Side::Rival => "Rival",
        }
    }
}

/// Which item on the stack: numbered from 1 within a match, never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ItemId(pub u32);

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// One card on the stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item {
    /// Its number.
    pub id: ItemId,
    /// What it is.
    pub card: Card,
    /// Who played it.
    pub owner: Side,
    /// Whose life it moves, for an effect; the owner, meaninglessly, for a stack card.
    pub affects: Side,
    /// What it acts on, for a stack card.
    pub target: Option<ItemId>,
}

/// Whom a card played (or copied) by `owner` affects: damage the other seat,
/// heal your own.
pub fn affects_for(card: Card, owner: Side) -> Side {
    match card {
        Card::Bolt | Card::Blast => owner.other(),
        Card::Mend | Card::Counter | Card::Delay | Card::Redirect | Card::Copy => owner,
    }
}

/// Every item `card` may aim at on `stack`, top-first — the one legality rule.
///
/// Counter, Delay and Copy take any item; Redirect only an effect item; an
/// effect card takes none and never opens choosing.
pub fn legal_targets(card: Card, stack: &[Item]) -> Vec<ItemId> {
    let aims = |item: &Item| match card {
        Card::Counter | Card::Delay | Card::Copy => true,
        Card::Redirect => item.card.is_effect(),
        Card::Bolt | Card::Blast | Card::Mend => false,
    };
    stack
        .iter()
        .rev()
        .filter(|item| aims(item))
        .map(|item| item.id)
        .collect()
}

/// One resolution, as the forecast predicts it and as `resolve_top` applies it.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    /// The item that resolved.
    pub item: ItemId,
    /// What it was.
    pub card: Card,
    /// The outcome line: the log's line and the panel's forecast cell.
    pub says: String,
    /// Both lives afterwards, You first.
    pub life_after: [i32; 2],
    /// The whole stack afterwards, bottom first.
    pub stack_after: Vec<Item>,
    /// The next unused item number afterwards (a Copy uses one).
    pub next_id_after: u32,
}

impl Step {
    /// The stack afterwards as ids, top-first — the order the panel lists it in.
    pub fn ids_after(&self) -> Vec<ItemId> {
        self.stack_after.iter().rev().map(|item| item.id).collect()
    }
}

/// What will become of one item now on the stack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    /// It resolves as the forecast's step at this index.
    At(usize),
    /// A Counter removes it first.
    Voided(ItemId),
    /// The match ends before it is reached.
    NotReached,
}

/// One item's row of the forecast.
#[derive(Clone, Debug, PartialEq)]
pub struct Fate {
    /// Which item.
    pub id: ItemId,
    /// When it resolves, or why it does not.
    pub order: Order,
    /// Its forecast line: its step's line, `voided by #n`, or `not reached`.
    pub says: String,
}

impl Fate {
    /// The order cell: its place in the resolution order, `x` voided, `-` not reached.
    pub fn order_cell(&self) -> String {
        match self.order {
            Order::At(index) => (index + 1).to_string(),
            Order::Voided(_) => "x".to_owned(),
            Order::NotReached => "-".to_owned(),
        }
    }
}

/// The stack resolved top-first with nobody playing anything more.
#[derive(Clone, Debug, PartialEq)]
pub struct Forecast {
    /// Every resolution, in order, stopping at the one that ends the match.
    pub steps: Vec<Step>,
    /// One row per item on the stack now, top-first.
    pub fates: Vec<Fate>,
}

impl Forecast {
    /// Both lives once the forecast has run out — now, if nothing resolves.
    pub fn final_life(&self, duel: &Duel) -> [i32; 2] {
        self.steps
            .last()
            .map_or(duel.life(), |step| step.life_after)
    }
}

/// What the stack will do: every resolution in order, and each item's fate.
pub fn forecast(duel: &Duel) -> Forecast {
    let mut stack = duel.stack.clone();
    let mut life = duel.life();
    let mut next_id = duel.next_id;
    let mut steps = Vec::new();
    let mut voided: Vec<(ItemId, ItemId)> = Vec::new();
    if duel.phase == Phase::Live {
        while let Some(top) = stack.pop() {
            let (says, removed) = resolve_one(top, &mut stack, &mut life, &mut next_id);
            if let Some(gone) = removed {
                voided.push((gone, top.id));
            }
            steps.push(Step {
                item: top.id,
                card: top.card,
                says,
                life_after: life,
                stack_after: stack.clone(),
                next_id_after: next_id,
            });
            if life.iter().any(|&each| each <= 0) {
                break;
            }
        }
    }
    let fates = duel
        .stack
        .iter()
        .rev()
        .map(|item| {
            let resolved = steps.iter().position(|step| step.item == item.id);
            let by = voided.iter().find(|(gone, _)| *gone == item.id);
            let (order, says) = match (resolved, by) {
                (Some(index), _) => (Order::At(index), steps[index].says.clone()),
                (None, Some((_, by))) => (Order::Voided(*by), format!("voided by {by}")),
                (None, None) => (Order::NotReached, "not reached".to_owned()),
            };
            Fate {
                id: item.id,
                order,
                says,
            }
        })
        .collect();
    Forecast { steps, fates }
}

/// Resolve the top item for real: apply the forecast's first step, exactly.
///
/// Returns false when there was nothing to resolve. Ending the match is the
/// duel's business (`Duel::after_resolution`), not this function's.
pub fn resolve_top(duel: &mut Duel) -> bool {
    let Some(step) = forecast(duel).steps.into_iter().next() else {
        return false;
    };
    duel.stack = step.stack_after;
    for side in Side::BOTH {
        duel.seats[side.index()].life = step.life_after[side.index()];
    }
    duel.next_id = step.next_id_after;
    duel.log.push(step.says);
    true
}

/// Resolve `top`, already popped, against the rest of the stack.
///
/// Returns its outcome line and, for a Counter that landed, the item it removed.
fn resolve_one(
    top: Item,
    stack: &mut Vec<Item>,
    life: &mut [i32; 2],
    next_id: &mut u32,
) -> (String, Option<ItemId>) {
    let name = top.card.name();
    if top.card.is_effect() {
        let seat = top.affects.index();
        let before = life[seat];
        life[seat] += top.card.life_change();
        return (
            format!("{name}: {} {before}>{}", top.affects.name(), life[seat]),
            None,
        );
    }
    let landed = top
        .target
        .filter(|target| legal_targets(top.card, stack).contains(target));
    let Some(target) = landed else {
        return (format!("{name}: fizzles"), None);
    };
    let Some(at) = stack.iter().position(|item| item.id == target) else {
        return (format!("{name}: fizzles"), None);
    };
    match top.card {
        Card::Counter => {
            stack.remove(at);
            (format!("{name}: voids {target}"), Some(target))
        }
        Card::Delay => {
            let moved = stack.remove(at);
            stack.insert(0, moved);
            (format!("{name}: {target} to bottom"), None)
        }
        Card::Redirect => {
            stack[at].affects = stack[at].affects.other();
            (
                format!("{name}: {target}>{}", stack[at].affects.name()),
                None,
            )
        }
        Card::Copy => {
            let original = stack[at];
            let copy = Item {
                id: ItemId(*next_id),
                card: original.card,
                owner: top.owner,
                affects: affects_for(original.card, top.owner),
                target: original.target,
            };
            *next_id += 1;
            stack.push(copy);
            (format!("{name}: {target} as {}", copy.id), None)
        }
        Card::Bolt | Card::Blast | Card::Mend => (format!("{name}: fizzles"), None),
    }
}

#[cfg(test)]
#[path = "rules_tests.rs"]
mod tests;
