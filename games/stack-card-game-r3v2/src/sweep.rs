//! G7: three players over a sweep of seeds, and G6: one whole match driven
//! through the keyboard.
//!
//! The sweep plays `Duel::apply` directly — no sim, so the Rival's wait is
//! irrelevant — and asks the controllers document's three questions: can the
//! game be lost (nothing), does raw power win (brute), does reading the stack
//! win (sequencer). The keyboard match is the same sequencer pressing real keys
//! into the real systems, so "playable end to end" is a fact about the window's
//! code path, not about `apply`.

use jidousha::prelude::*;

use crate::checks::{Album, Checks, Driver};
use crate::duel::{Action, Duel, Outcome, Phase};
use crate::players::{brute_action, nothing_action, rival_action, sequencer_action};
use crate::rules::{Side, forecast, legal_targets};
use crate::screen::banner_line;

/// Seeds the sweep plays.
pub const SEEDS: std::ops::RangeInclusive<u64> = 1..=12;

/// The most actions one match may take before the sweep calls it hung.
const ACTION_CAP: usize = 5_000;

/// The seed the keyboard match is configured with.
pub const MATCH_SEED: u64 = 7;

/// The most ticks the keyboard match may take.
pub const MATCH_TICKS: u64 = 12_000;

/// A player: what Your seat does, given the duel.
pub type Player = fn(&Duel) -> Action;

/// What one player did across the sweep.
#[derive(Default)]
pub struct Tally {
    /// Matches won.
    pub wins: usize,
    /// Matches lost.
    pub losses: usize,
    /// Matches drawn.
    pub draws: usize,
    /// Cards played.
    pub plays: usize,
    /// Of those, plays made onto a non-empty stack.
    pub responses: usize,
    /// Your life minus the Rival's at the end of the forecast, summed over plays.
    pub margin_sum: i32,
    /// Matches that hit the action cap, and actions a player named that were refused.
    pub faults: usize,
}

/// Play one match from `seed` with `player` in Your seat against the Rival.
pub fn play_out(seed: u64, player: Player, tally: &mut Tally) {
    let mut duel = Duel::new(&mut Rng::from_seed(seed));
    for _ in 0..ACTION_CAP {
        if let Phase::Over(outcome) = duel.phase {
            match outcome {
                Outcome::YouWin => tally.wins += 1,
                Outcome::RivalWins => tally.losses += 1,
                Outcome::Draw => tally.draws += 1,
            }
            return;
        }
        let yours = duel.priority == Side::You;
        let action = if yours {
            player(&duel)
        } else {
            rival_action(&duel)
        };
        let onto = !duel.stack.is_empty();
        if duel.apply(action).is_err() {
            tally.faults += 1;
            let _ = duel.apply(Action::Pass);
            continue;
        }
        if yours && let Action::Play { .. } = action {
            tally.plays += 1;
            tally.responses += usize::from(onto);
            let [you, rival] = forecast(&duel).final_life(&duel);
            tally.margin_sum += you - rival;
        }
    }
    tally.faults += 1;
}

/// G7: nothing loses every seed, brute wins fewer than the sequencer, and the
/// sequencer wins at least 7 of 12.
pub fn three_players(checks: &mut Checks) -> Vec<String> {
    let players: [(&str, Player); 3] = [
        ("nothing", nothing_action),
        ("brute", brute_action),
        ("sequencer", sequencer_action),
    ];
    let count = SEEDS.count();
    let mut lines = Vec::new();
    let mut wins = [0; 3];
    for (index, (name, player)) in players.into_iter().enumerate() {
        let mut tally = Tally::default();
        for seed in SEEDS {
            play_out(seed, player, &mut tally);
        }
        wins[index] = tally.wins;
        checks.require(
            tally.faults == 0,
            "the sweep: a player named an illegal action or a match never ended",
            format!("{name}: {} faults over {count} seeds", tally.faults),
        );
        lines.push(format!(
            "{name}: won {} of {count}, lost {}, drew {}",
            tally.wins, tally.losses, tally.draws
        ));
        if name == "sequencer" {
            let average = if tally.plays == 0 {
                0.0
            } else {
                tally.margin_sum as f32 / tally.plays as f32
            };
            lines.push(format!(
                "sequencer: {} plays, {} of them responses, forecast margin {average:.1} per play",
                tally.plays, tally.responses
            ));
        }
    }
    let [nothing, brute, sequencer] = wins;
    checks.require(
        nothing == 0,
        "the sweep: a player who does nothing won a match",
        format!("nothing won {nothing} of {count}"),
    );
    checks.require(
        brute < sequencer,
        "the sweep: raw power won as often as reading the stack",
        format!("brute won {brute}, sequencer {sequencer}, of {count}"),
    );
    checks.require(
        sequencer * 12 >= 7 * count,
        "the sweep: reading the stack does not win most matches",
        format!("sequencer won {sequencer} of {count}; the bar is 7 of 12"),
    );
    lines
}

/// The keys that make the game do `action`: the slot's digit, one Down per
/// step from the first legal target to the chosen one, Enter.
pub fn keys_for(action: Action, duel: &Duel) -> Vec<Key> {
    const DIGITS: [Key; 6] = [
        Key::Digit1,
        Key::Digit2,
        Key::Digit3,
        Key::Digit4,
        Key::Digit5,
        Key::Digit6,
    ];
    let Action::Play { slot, target } = action else {
        return vec![Key::Space];
    };
    let mut keys = vec![DIGITS[slot.min(5)]];
    if let Some(target) = target {
        let card = duel.seat(Side::You).hand[slot];
        let at = legal_targets(card, &duel.stack)
            .iter()
            .position(|id| *id == target)
            .unwrap_or(0);
        keys.extend(std::iter::repeat_n(Key::ArrowDown, at));
        keys.push(Key::Enter);
    }
    keys
}

/// G6: one whole match on `MATCH_SEED`, the sequencer at the keyboard and the
/// Rival in its own system, to the result screen and back to a new match.
pub fn the_match(checks: &mut Checks, album: &mut Album) -> String {
    let mut driver = Driver::new(None, MATCH_SEED);
    let mut thinking_shot = false;
    let mut keys_pressed = 0;
    // Every Rival item seen on the stack: the Rival's own system playing, which
    // nothing else in the run drives (the sweep calls its rule directly).
    let mut rival_items = std::collections::BTreeSet::new();
    while driver.ticks < MATCH_TICKS {
        let duel = driver.flow().duel.clone();
        if duel.phase != Phase::Live {
            break;
        }
        if duel.priority == Side::You {
            for key in keys_for(sequencer_action(&duel), &duel) {
                driver.press(key);
                keys_pressed += 1;
            }
        } else {
            if !thinking_shot && !duel.stack.is_empty() {
                driver.shoot(album, "the match: the Rival thinking");
                thinking_shot = true;
            }
            driver.step();
        }
        let stack = &driver.flow().duel.stack;
        rival_items.extend(
            stack
                .iter()
                .filter(|item| item.owner == Side::Rival)
                .map(|item| item.id),
        );
    }
    checks.require(
        rival_items.len() >= 5,
        "the match: the Rival barely played through its own system",
        format!(
            "{} Rival items reached the stack in the keyboard match",
            rival_items.len()
        ),
    );
    let ended = driver.flow().clone();
    driver.shoot(album, "the match: the result");
    let banner = banner_line(&ended.duel);
    checks.require(
        banner.is_some(),
        "the match: the keyboard match did not reach its result screen",
        format!(
            "after {} ticks it is turn {}, life {:?}",
            driver.ticks,
            ended.duel.turn,
            ended.duel.life()
        ),
    );
    driver.press(Key::Enter);
    let again = &driver.flow().duel;
    let hands = [again.seats[0].hand.len(), again.seats[1].hand.len()];
    checks.require(
        again.phase == Phase::Live && again.turn == 1 && hands == [4, 4],
        "the match: Enter on the result screen did not start a new match",
        format!(
            "phase {:?}, turn {}, hands {hands:?}",
            again.phase, again.turn
        ),
    );
    format!(
        "the match: seed {MATCH_SEED}, {} ticks, {keys_pressed} keys, {} Rival plays seen, turn {}, \
         ended {:?}",
        driver.ticks,
        rival_items.len(),
        ended.duel.turn,
        banner.unwrap_or_default()
    )
}
