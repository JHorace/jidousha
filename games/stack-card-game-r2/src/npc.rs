//! The Brute's mind: one free function from a duel to an action.
//!
//! Free so that the verify run can ask it what the NPC *will* do, and so the
//! stack-reading controller can roll the NPC forward beside the stack.

use crate::duel::{Duel, Side, legal_targets, life_after};

/// What a player does with priority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Pass,
    Play { slot: usize, target: Option<u32> },
}

/// How much one energy spent is worth against one point of life, in the score.
///
/// Small, so that it only breaks ties between plays that change the life
/// totals equally — the Brute never holds a card back that would gain it
/// a point.
const ENERGY_WEIGHT: i32 = 1;
/// One point of life in the score.
const LIFE_WEIGHT: i32 = 10;

/// Every action `side` could take right now, pass first.
pub(crate) fn options(duel: &Duel, side: Side) -> Vec<Action> {
    let mut all = vec![Action::Pass];
    for slot in 0..duel.seat(side).hand.len() {
        if !duel.playable(side, slot) {
            continue;
        }
        let card = duel.seat(side).hand[slot];
        let targets = legal_targets(duel, card);
        if card.spec().aim == crate::cards::Aim::Nothing {
            all.push(Action::Play { slot, target: None });
        } else {
            for target in targets {
                all.push(Action::Play {
                    slot,
                    target: Some(target),
                });
            }
        }
    }
    all
}

/// The duel after `side` takes `action` — `None` if the rules refuse it.
pub(crate) fn after(duel: &Duel, side: Side, action: Action) -> Option<Duel> {
    let mut next = duel.clone();
    let done = match action {
        Action::Pass => next.pass(side),
        Action::Play { slot, target } => next.play(side, slot, target).map(|_| ()),
    };
    done.ok().map(|()| next)
}

/// The Brute's choice: every legal action, scored by what the stack will do
/// if nobody answers it (`duel::preview`), best first, pass on ties.
///
/// One ply and greedy on purpose: it answers what is on the stack and it
/// plays its threats, but it does not imagine your answer to its answer —
/// which is the thing a stack reader beats it with.
pub(crate) fn choose(duel: &Duel, side: Side) -> Action {
    let mut best = Action::Pass;
    let mut best_score = score(duel, side, 0);
    for action in options(duel, side) {
        let Action::Play { slot, .. } = action else {
            continue;
        };
        let cost = duel.seat(side).hand[slot].cost() as i32;
        let Some(next) = after(duel, side, action) else {
            continue;
        };
        let value = score(&next, side, cost);
        if value > best_score {
            best = action;
            best_score = value;
        }
    }
    best
}

/// `side`'s life lead once the stack has resolved, less what it spent.
fn score(duel: &Duel, side: Side, spent: i32) -> i32 {
    let [you, npc] = life_after(duel);
    let lead = match side {
        Side::You => you - npc,
        Side::Npc => npc - you,
    };
    lead * LIFE_WEIGHT - spent * ENERGY_WEIGHT
}
