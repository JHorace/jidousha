//! Choosing a move by reading the stack: every affordable play, every legal
//! target, each rolled forward through `preview` and scored.
//!
//! The rival plays `best_choice` for its own side, one move deep. The `--verify`
//! players reuse `options` and `apply` to look further (players.rs).
//!
//! Key functions: `options`, `apply`, `score_now`, `best_choice`.

use crate::rules::{Duel, ItemId, Outcome, Side, legal_targets, pass, play, preview};

/// What a duellist with priority does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Choice {
    Play {
        index: usize,
        target: Option<ItemId>,
    },
    Pass,
}

/// Life margin a won match is worth, so a win outranks any lead.
const WIN: i32 = 100;

/// Every move `side` may make now: each affordable card at each legal target,
/// then passing. Empty when `side` does not hold priority.
pub(crate) fn options(duel: &Duel, side: Side) -> Vec<Choice> {
    if duel.priority != side || duel.outcome.is_some() {
        return Vec::new();
    }
    let me = duel.side(side);
    let mut all = Vec::new();
    for (index, &card) in me.hand.iter().enumerate() {
        if card.cost() > me.focus {
            continue;
        }
        if card.needs_target() {
            for id in legal_targets(duel, side, card) {
                all.push(Choice::Play {
                    index,
                    target: Some(id),
                });
            }
        } else {
            all.push(Choice::Play {
                index,
                target: None,
            });
        }
    }
    all.push(Choice::Pass);
    all
}

/// `duel` after `side` makes `choice` — a copy, so a chooser can look ahead.
pub(crate) fn apply(duel: &Duel, side: Side, choice: Choice) -> Duel {
    let mut next = duel.clone();
    let _ = match choice {
        Choice::Play { index, target } => play(&mut next, side, index, target),
        Choice::Pass => pass(&mut next, side),
    };
    next
}

/// How good `duel` is for `side` if nobody adds to the stack: the life margin
/// after it resolves all the way down, a win or loss outranking any margin.
pub(crate) fn score_now(duel: &Duel, side: Side) -> i32 {
    let ahead = preview(duel);
    let (mine, theirs) = match side {
        Side::You => (ahead.you_life, ahead.rival_life),
        Side::Rival => (ahead.rival_life, ahead.you_life),
    };
    let settled = match ahead.outcome {
        Some(Outcome::Won(winner)) if winner == side => WIN,
        Some(Outcome::Won(_)) => -WIN,
        _ => 0,
    };
    (mine - theirs + settled) * 10
}

/// The focus a choice spends, as a tie-break against wasting cards.
fn spend(duel: &Duel, side: Side, choice: Choice) -> i32 {
    match choice {
        Choice::Play { index, .. } => duel.side(side).hand[index].cost(),
        Choice::Pass => 0,
    }
}

/// The rival's rule, and the one-move reader: play whatever leaves the best
/// resolved margin, if it beats passing; otherwise pass.
pub(crate) fn best_choice(duel: &Duel, side: Side) -> Choice {
    let mut best = Choice::Pass;
    let mut best_score = score_now(duel, side);
    for choice in options(duel, side) {
        if choice == Choice::Pass {
            continue;
        }
        let score = score_now(&apply(duel, side, choice), side) - spend(duel, side, choice);
        if score > best_score {
            best = choice;
            best_score = score;
        }
    }
    best
}
