//! Resolution: what the top item of the stack does, and the preview that
//! folds that one step over a copy of the duel.
//!
//! `resolve_top` is the one step. The real resolution (`Duel::pass`) runs it
//! on the duel; `preview` runs it on a clone until the stack is empty, and the
//! stack panel and the NPC both read that fold — so what the panel says the
//! stack will do and what it then does are the same function's answer.

use crate::cards::{Aim, BLAST_DAMAGE, BOLT_DAMAGE, Card, MEND_LIFE, SURGE_BASE};
use crate::duel::{Duel, Effect, Item, Side, Step, lands_on, legal};

/// Take the top item off the stack and do what it says.
///
/// The one resolution step: the real resolution (`Duel::pass`) runs it on the
/// duel, and `preview` runs it on a copy.
pub(crate) fn resolve_top(duel: &mut Duel) -> Option<Step> {
    let item = duel.stack.pop()?;
    let aimed = item.card.spec().aim != Aim::Nothing;
    let effect = if aimed && !item.target.is_some_and(|id| legal(duel, item.card, id)) {
        Effect::Fizzled
    } else {
        apply(duel, &item)
    };
    duel.discard(&item);
    Some(Step {
        item: item.id,
        card: item.card,
        caster: item.caster,
        effect,
        life: duel.life(),
    })
}

fn apply(duel: &mut Duel, item: &Item) -> Effect {
    let target = item.target.unwrap_or(0);
    let position = duel.stack.iter().position(|other| other.id == target);
    match item.card {
        Card::Bolt => damage(duel, item.hits, BOLT_DAMAGE),
        Card::Blast => damage(duel, item.hits, BLAST_DAMAGE),
        Card::Surge => {
            let amount = SURGE_BASE + duel.stack.len() as i32;
            damage(duel, item.hits, amount)
        }
        Card::Mend => {
            duel.seat_mut(item.hits).life += MEND_LIFE;
            Effect::Heal {
                to: item.hits,
                amount: MEND_LIFE,
            }
        }
        Card::Cancel => match position {
            Some(at) => {
                let gone = duel.stack.remove(at);
                duel.discard(&gone);
                Effect::Countered {
                    target,
                    card: gone.card,
                }
            }
            None => Effect::Fizzled,
        },
        Card::Mirror => match position {
            Some(at) => {
                let turned = &mut duel.stack[at];
                turned.hits = turned.hits.other();
                Effect::Redirected {
                    target,
                    card: turned.card,
                    to: turned.hits,
                }
            }
            None => Effect::Fizzled,
        },
        Card::Echo => match position {
            Some(at) => {
                let original = duel.stack[at];
                let copy = duel.next_id;
                duel.next_id += 1;
                duel.stack.push(Item {
                    id: copy,
                    card: original.card,
                    caster: item.caster,
                    hits: lands_on(original.card, item.caster),
                    target: original.target,
                    copy: true,
                });
                Effect::Copied {
                    target,
                    card: original.card,
                    copy,
                }
            }
            None => Effect::Fizzled,
        },
        Card::Sink => match position {
            Some(at) => {
                let sunk = duel.stack.remove(at);
                duel.stack.insert(0, sunk);
                Effect::Sunk {
                    target,
                    card: sunk.card,
                }
            }
            None => Effect::Fizzled,
        },
        Card::Flip => {
            duel.stack.reverse();
            Effect::Flipped {
                items: duel.stack.len(),
            }
        }
    }
}

fn damage(duel: &mut Duel, to: Side, amount: i32) -> Effect {
    duel.harm(to, amount);
    Effect::Damage { to, amount }
}

/// What the stack will do if both players pass from here on, in the order it
/// will do it — the stack panel's preview, and what the NPC scores its plays by.
pub(crate) fn preview(duel: &Duel) -> Vec<Step> {
    let mut ahead = duel.clone();
    let mut steps = Vec::new();
    while let Some(step) = resolve_top(&mut ahead) {
        steps.push(step);
    }
    steps
}

/// The life both players will have when the stack has resolved, `[you, npc]`.
pub(crate) fn life_after(duel: &Duel) -> [i32; 2] {
    preview(duel).last().map_or(duel.life(), |step| step.life)
}

/// One line saying what a step did, as the feed and the panel print it.
pub(crate) fn describe(effect: Effect) -> String {
    match effect {
        Effect::Damage { to, amount } => format!("{amount} damage to {}", to.name()),
        Effect::Heal { to, amount } => format!("{} gains {amount}", to.name()),
        Effect::Countered { target, card } => format!("counters {} #{target}", card.name()),
        Effect::Redirected { target, card, to } => {
            format!("turns {} #{target} onto {}", card.name(), to.name())
        }
        Effect::Copied { target, card, copy } => {
            format!("copies {} #{target} as #{copy}", card.name())
        }
        Effect::Sunk { target, card } => format!("sinks {} #{target}", card.name()),
        Effect::Flipped { items } => format!("reverses {items} items"),
        Effect::Fizzled => "fizzles".to_owned(),
    }
}
