//! Who decides what: the Rival's rule, and the three check players.
//!
//! Every player is a pure `fn(&Duel) -> Action`, so the game's system, the
//! verify sweep and the sequencer's look-ahead all ask the same function
//! (jidousha-controllers.md, "Aim at where the target will be": run the
//! opponent's own rule forward). The Rival is greedy on purpose — the skill
//! the game tests is reading its rule off the stack (DESIGN.md, "The Rival").

use crate::cards::Card;
use crate::duel::{Action, Duel, Phase};
use crate::rules::{ItemId, Side, forecast, legal_targets};

/// Ticks the Rival waits after receiving priority, so a person can read the stack.
pub const RIVAL_THINK_TICKS: u32 = 45;

/// The Rival's life at or below which it spends a Mend on its own turn.
pub const RIVAL_MEND_AT: i32 = 9;

/// Rival life minus Your life once `duel`'s stack has resolved.
pub fn margin(duel: &Duel) -> i32 {
    let [you, rival] = forecast(duel).final_life(duel);
    rival - you
}

/// The first hand slot of `side` holding `card` that it could play now.
fn slot_of(duel: &Duel, side: Side, card: Card) -> Option<usize> {
    let seat = duel.seat(side);
    seat.hand
        .iter()
        .position(|&held| held == card)
        .filter(|&slot| duel.playable(side, slot))
}

/// The item whose removal helps the Rival most, if removing any helps at all.
///
/// Ties go to the top-most item: the stack is walked top-first and only a
/// strictly larger rise replaces the one held.
pub fn threat(duel: &Duel) -> Option<ItemId> {
    let base = margin(duel);
    let mut best: Option<(ItemId, i32)> = None;
    for item in duel.stack.iter().rev() {
        let mut without = duel.clone();
        without.stack.retain(|each| each.id != item.id);
        let rise = margin(&without) - base;
        if rise > 0 && best.is_none_or(|(_, held)| rise > held) {
            best = Some((item.id, rise));
        }
    }
    best.map(|(id, _)| id)
}

/// The Rival: answer the biggest threat on the stack, attack on its own turn.
pub fn rival_action(duel: &Duel) -> Action {
    let me = Side::Rival;
    if duel.phase != Phase::Live || duel.priority != me {
        return Action::Pass;
    }
    if !duel.stack.is_empty() {
        let Some(aim) = threat(duel) else {
            return Action::Pass;
        };
        if let Some(slot) = slot_of(duel, me, Card::Counter) {
            return Action::Play {
                slot,
                target: Some(aim),
            };
        }
        let aimable = legal_targets(Card::Redirect, &duel.stack).contains(&aim);
        if let Some(slot) = slot_of(duel, me, Card::Redirect).filter(|_| aimable) {
            return Action::Play {
                slot,
                target: Some(aim),
            };
        }
        return Action::Pass;
    }
    if duel.active == me {
        let mend = duel.seat(me).life <= RIVAL_MEND_AT;
        let pick = [Card::Blast, Card::Bolt]
            .into_iter()
            .find_map(|card| slot_of(duel, me, card))
            .or_else(|| slot_of(duel, me, Card::Mend).filter(|_| mend));
        if let Some(slot) = pick {
            return Action::Play { slot, target: None };
        }
    }
    Action::Pass
}

/// The player who is there and does nothing: proves the match can be lost.
pub fn nothing_action(_duel: &Duel) -> Action {
    Action::Pass
}

/// Raw power with no reading: its biggest damage card on its own empty stack,
/// a Mend if nothing else, and never a response.
pub fn brute_action(duel: &Duel) -> Action {
    let me = Side::You;
    if duel.phase != Phase::Live || duel.priority != me || duel.active != me {
        return Action::Pass;
    }
    if !duel.stack.is_empty() {
        return Action::Pass;
    }
    [Card::Blast, Card::Bolt, Card::Mend]
        .into_iter()
        .find_map(|card| slot_of(duel, me, card))
        .map_or(Action::Pass, |slot| Action::Play { slot, target: None })
}

/// Every action Your seat could take now: Pass, then each playable slot ×
/// each legal target, in slot order and target order.
pub fn your_options(duel: &Duel) -> Vec<Action> {
    let me = Side::You;
    let mut options = vec![Action::Pass];
    for (slot, &card) in duel.seat(me).hand.iter().enumerate() {
        if !duel.playable(me, slot) {
            continue;
        }
        if card.is_effect() {
            options.push(Action::Play { slot, target: None });
        } else {
            let mut aims = legal_targets(card, &duel.stack);
            aims.sort();
            options.extend(aims.into_iter().map(|id| Action::Play {
                slot,
                target: Some(id),
            }));
        }
    }
    options
}

/// How the sequencer rates one option: higher is better.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rating {
    /// 10 × (Your life − Rival life once the stack resolves) + 2 × energy the
    /// Rival spent replying − energy the option costs.
    pub score: i32,
    /// Cards left in the Rival's hand after its reply: fewer is better.
    pub rival_cards: usize,
}

/// Rate one option: apply it to a copy, let the Rival reply once by its own
/// rule if it then holds priority, and read the forecast.
///
/// The reply is `rival_action` whole, so it includes the Rival's rule 2 (an
/// own-turn attack) whenever that applies — DESIGN.md's open call, taken.
pub fn rate(duel: &Duel, option: Action) -> Option<Rating> {
    let mut after = duel.clone();
    after.apply(option).ok()?;
    let cost = match option {
        Action::Pass => 0,
        Action::Play { slot, .. } => duel.seat(Side::You).hand[slot].cost() as i32,
    };
    let mut spent = 0;
    if after.phase == Phase::Live && after.priority == Side::Rival {
        let before = after.seat(Side::Rival).energy;
        after.apply(rival_action(&after)).ok()?;
        spent = before.saturating_sub(after.seat(Side::Rival).energy) as i32;
    }
    let [you, rival] = forecast(&after).final_life(&after);
    Some(Rating {
        score: 10 * (you - rival) + 2 * spent - cost,
        rival_cards: after.seat(Side::Rival).hand.len(),
    })
}

/// The sequencer: the best-rated option. Ties go to fewer Rival cards, then to
/// Pass, then to the lowest slot and the lowest target — `your_options` is
/// already in that order, so the first strictly better rating wins.
pub fn sequencer_action(duel: &Duel) -> Action {
    if duel.phase != Phase::Live || duel.priority != Side::You {
        return Action::Pass;
    }
    let mut best: Option<(Action, Rating)> = None;
    for option in your_options(duel) {
        let Some(rating) = rate(duel, option) else {
            continue;
        };
        let better = best.is_none_or(|(_, held)| {
            rating.score > held.score
                || (rating.score == held.score && rating.rival_cards < held.rival_cards)
        });
        if better {
            best = Some((option, rating));
        }
    }
    best.map_or(Action::Pass, |(option, _)| option)
}
