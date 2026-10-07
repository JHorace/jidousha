//! The players a check drives the human seat with (controllers doc: three, not
//! one). All are pure functions of `Core`; `skilled` reads `preview`, the same
//! function the panel does, which is what makes it a player of the *stack*.

use crate::cards::{Side, info};
use crate::rules::{Action, Core, legal_actions, preview};

const ME: Side = Side::You;

/// Never plays anything: proves the match can be lost.
pub fn blind(_core: &Core) -> Action {
    Action::Pass
}

/// Spreads damage: casts the biggest damage card it can, never answers.
pub fn power(core: &Core) -> Action {
    let mut best: Option<(i32, Action)> = None;
    for action in legal_actions(core, ME) {
        if let Action::Play { hand_index, .. } = action {
            let data = info(core.hands[ME.index()][hand_index]);
            if data.damage > 0 && best.is_none_or(|(damage, _)| data.damage > damage) {
                best = Some((data.damage, action));
            }
        }
    }
    best.map_or(Action::Pass, |(_, action)| action)
}

/// Score of a position from `ME`'s seat once everything on the stack has
/// resolved: life difference, less a price per mana spent and a penalty for
/// being left with no mana to answer with.
fn score_after(core: &Core, action: Action) -> f32 {
    let mut trial = core.clone();
    let mut spent = 0.0;
    if let Action::Play { hand_index, target } = action {
        spent = f32::from(info(core.hands[ME.index()][hand_index]).cost);
        if trial.try_play(ME, hand_index, target).is_err() {
            return f32::MIN;
        }
    }
    let steps = preview(&trial);
    let life = steps.last().map_or(trial.life, |step| step.life_after);
    let mut score = (life[0] - life[1]) as f32 - 0.7 * spent;
    if life[0] <= 0 {
        score -= 100.0;
    }
    if life[1] <= 0 {
        score += 100.0;
    }
    let mana_left = trial.mana[ME.index()];
    if mana_left < 2 && core.active != ME {
        score -= 1.0;
    }
    score
}

/// One-ply search over every legal action, scored by what the preview says the
/// stack will do. Passing is the baseline an action has to beat.
pub fn skilled(core: &Core) -> Action {
    let mut best = (score_after(core, Action::Pass), Action::Pass);
    for action in legal_actions(core, ME) {
        if action == Action::Pass {
            continue;
        }
        let score = score_after(core, action);
        if score > best.0 + 0.5 {
            best = (score, action);
        }
    }
    best.1
}

/// Plays a whole match: `you` drives the human seat, `npc::choose` the other.
/// Returns the finished core.
pub fn play_match(seed: u64, you: fn(&Core) -> Action) -> Core {
    let mut core = Core::new(seed);
    // Every action either spends a card, or passes; a match is far shorter.
    for _ in 0..5000 {
        if core.outcome.is_some() {
            return core;
        }
        let side = core.priority;
        let action = match side {
            Side::You => you(&core),
            Side::Npc => crate::npc::choose(&core),
        };
        if let Err(error) = core.try_act(side, action) {
            panic!("a player chose an illegal action: {error}");
        }
    }
    panic!("a match did not finish in 5000 actions; a player is looping");
}
