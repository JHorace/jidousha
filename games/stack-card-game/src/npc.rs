//! The scripted opponent: a pure function of the visible state, no randomness.
//!
//! It plays the stack rather than the hand: it answers big incoming damage with
//! Redirect / Negate, hushes cheap healing, doubles its own damage with Echo
//! after you pass, and keeps mana open on your turn.

use crate::cards::{Card, Side, info};
use crate::rules::{Action, Core, Fate, Item, fate, legal_targets, preview};

const ME: Side = Side::Npc;

/// Incoming damage at or above this is worth an answer.
const WORTH_ANSWERING: i32 = 4;
/// Heal below this life.
const HEAL_BELOW: i32 = 12;

fn find_card(core: &Core, card: Card) -> Option<usize> {
    core.hands[ME.index()].iter().position(|&held| held == card)
}

fn can_afford(core: &Core, card: Card) -> bool {
    u32::from(info(card).cost) <= core.mana[ME.index()]
}

/// A play of `card` at `target`, if the card is held, affordable and legal.
fn play(core: &Core, card: Card, target: Option<&Item>) -> Option<Action> {
    let hand_index = find_card(core, card)?;
    if !can_afford(core, card) {
        return None;
    }
    let target = target.map(|item| item.id);
    match core.options(ME, hand_index) {
        Ok(Some(legal)) => {
            let t = target?;
            legal.contains(&t).then_some(Action::Play {
                hand_index,
                target: Some(t),
            })
        }
        Ok(None) => Some(Action::Play {
            hand_index,
            target: None,
        }),
        Err(_) => None,
    }
}

/// What the NPC does with priority. Pass is the answer when nothing is worth it.
pub fn choose(core: &Core) -> Action {
    let steps = preview(core);
    let resolves = |item: &Item| matches!(fate(&steps, item.id), Some(Fate::Resolves(_)));

    // 1. Answer damage aimed at me, biggest first.
    let mut threats: Vec<&Item> = core
        .stack
        .iter()
        .filter(|item| {
            item.owner == Side::You
                && item.aim == Some(ME)
                && info(item.card).damage >= WORTH_ANSWERING
                && resolves(item)
        })
        .collect();
    threats.sort_by_key(|item| -info(item.card).damage);
    if let Some(threat) = threats.first() {
        for card in [Card::Redirect, Card::Negate, Card::Hush] {
            if let Some(action) = play(core, card, Some(threat)) {
                return action;
            }
        }
    }

    // 2. Hush a heal of yours that would resolve.
    if let Some(heal) = core
        .stack
        .iter()
        .find(|item| item.owner == Side::You && item.card == Card::Mend && resolves(item))
        && let Some(action) = play(core, Card::Hush, Some(heal))
    {
        return action;
    }

    // 3. Double my own damage once, after the player has had priority.
    let echo_on_stack = core
        .stack
        .iter()
        .any(|item| item.owner == ME && item.card == Card::Echo);
    if !echo_on_stack
        && let Some(mine) = core.stack.iter().rev().find(|item| {
            item.owner == ME
                && item.aim == Some(Side::You)
                && info(item.card).damage >= 3
                && resolves(item)
                && legal_targets(&core.stack, Card::Echo, ME).contains(&item.id)
        })
        && core.passes > 0
        && let Some(action) = play(core, Card::Echo, Some(mine))
    {
        return action;
    }

    // 4. On an empty stack: heal when low, then cast the biggest affordable sorcery.
    if core.stack.is_empty() {
        let has_answer = core.hands[ME.index()]
            .iter()
            .any(|card| matches!(card, Card::Negate | Card::Redirect));
        if core.life[ME.index()] <= HEAL_BELOW
            && let Some(action) = play(core, Card::Mend, None)
        {
            return action;
        }
        for card in [Card::Siege, Card::Cleaver] {
            if core.active == ME
                && can_afford(core, card)
                && (core.mana[ME.index()] >= u32::from(info(card).cost) + 2 || !has_answer)
                && let Some(action) = play(core, card, None)
            {
                return action;
            }
        }
        if core.active == ME
            && (core.mana[ME.index()] >= 3 || !has_answer)
            && let Some(action) = play(core, Card::Ember, None)
        {
            return action;
        }
    }
    Action::Pass
}
