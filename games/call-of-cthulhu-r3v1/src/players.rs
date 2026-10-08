//! The three players `--verify` plays with: a scholar, a novice and a mute.
//!
//! Each reads only what the screen shows — tonight's callers, the options and
//! their stated effects, the hint beside each answer — and never the kind of
//! an answer it has no lore for. The scholar plays to win; the novice is a
//! person's first try (the first option every morning, the hints when there
//! are any, else the top answer); the mute never studies and always says the
//! same thing. Only the novice's line says whether the game is worth playing.

use crate::play::{Command, Fate, Run, Screen};
use crate::rules::{self, Action, CallPlan, DAYS, DayState, answer_outcome};
use crate::screen::layout;

/// Who is playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Player {
    Scholar,
    Novice,
    Mute,
}

impl Player {
    pub(crate) const ALL: [Player; 3] = [Player::Scholar, Player::Novice, Player::Mute];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Player::Scholar => "scholar",
            Player::Novice => "novice",
            Player::Mute => "mute",
        }
    }
}

/// What `player` does on the screen `run` is showing.
pub(crate) fn decide(player: Player, run: &Run) -> Command {
    match run.screen {
        Screen::Morning => Command::Choose(morning(player, run)),
        Screen::Calling(_) => Command::Choose(answer(player, run)),
        Screen::CallOver { .. } | Screen::End(_) => Command::Continue,
    }
}

/// Which morning option `player` takes.
fn morning(player: Player, run: &Run) -> usize {
    let options = run.options();
    match player {
        Player::Novice => 0,
        // The last option is always a bargain: a mute never learns anything.
        Player::Mute => options.len().saturating_sub(1),
        Player::Scholar => {
            // Roll tonight forward for every option, and take the cheapest
            // night once what an option leaves for later nights is counted.
            let days_after = f32::from(u16::try_from(DAYS - run.state.day).unwrap_or(0));
            let score = |action: Action| {
                let after = rules::apply(&run.state, action);
                let tonight = rules::tonight(&after);
                let cost: f32 = tonight
                    .calls
                    .iter()
                    .map(|call| expected_call(&after, call, 0, call.being.spec().line, 0))
                    .sum();
                let learned =
                    after.known.iter().sum::<usize>() - run.state.known.iter().sum::<usize>();
                let steadier = after.composure - run.state.composure;
                cost - days_after
                    * (FACT_WORTH * learned as f32 + COMPOSURE_WORTH * steadier as f32)
            };
            (0..options.len())
                .min_by(|a, b| score(options[*a]).total_cmp(&score(options[*b])))
                .unwrap_or(0)
        }
    }
}

/// What the scholar reckons one more learned fact saves, per night left.
const FACT_WORTH: f32 = 2.0;
/// What it reckons one more point of composure saves, per night left.
const COMPOSURE_WORTH: f32 = 5.0;

/// The sanity a call is expected to cost from exchange `asked` on, answering
/// known questions from lore and unknown ones blind (each answer equally
/// likely) — `answer_outcome` rolled forward, nothing simulated twice.
pub(crate) fn expected_call(
    state: &DayState,
    call: &CallPlan,
    asked: usize,
    line: i32,
    anger: i32,
) -> f32 {
    let id = call.question(asked);
    let follow = |row: usize| {
        let outcome = answer_outcome(id, row, anger, state.composure);
        let rest = if outcome.wrath || line + outcome.line_change <= 0 {
            0.0
        } else {
            expected_call(
                state,
                call,
                asked + 1,
                line + outcome.line_change,
                anger + outcome.anger_change,
            )
        };
        outcome.sanity_cost as f32 + rest
    };
    if state.knows(id) {
        (0..3).map(follow).fold(f32::MAX, f32::min)
    } else {
        (0..3).map(follow).sum::<f32>() / 3.0
    }
}

/// Which answer `player` gives.
///
/// With lore, the hints say what each answer does (`answer_outcome`, the same
/// function the screen prints), and anyone but the mute takes the cheapest
/// way to end the call. Without lore every answer reads "outcome unknown".
fn answer(player: Player, run: &Run) -> usize {
    let Screen::Calling(call) = run.screen else {
        return 0;
    };
    let id = run.night.calls[call.slot].question(call.asked);
    match player {
        Player::Mute => 2,
        Player::Scholar | Player::Novice if run.state.knows(id) => (0..3)
            .min_by_key(|row| {
                let outcome = answer_outcome(id, *row, call.anger, run.state.composure);
                (outcome.wrath, outcome.line_change, outcome.sanity_cost)
            })
            .unwrap_or(0),
        // Unknown: the scholar has learned that the middle answer is no safer
        // than any other and takes it anyway; the novice takes the first.
        Player::Scholar => 1,
        Player::Novice => 0,
    }
}

/// How one player's run went.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Report {
    pub(crate) fate: Fate,
    /// Answers given, and how many of them were from lore.
    pub(crate) answers: usize,
    pub(crate) informed: usize,
    /// Answers that angered the caller, and calls that ended in wrath.
    pub(crate) insults: usize,
    pub(crate) wraths: usize,
}

/// The most commands a run can take: a guard against a player that loops.
const COMMAND_CAP: usize = 500;

/// Play one whole run of `player` on `seed`, on the pure state machine.
pub(crate) fn play(player: Player, seed: u64) -> Option<Report> {
    let mut run = Run::new(seed);
    let mut informed = 0;
    for _ in 0..COMMAND_CAP {
        if let Screen::End(fate) = run.screen {
            let insults = run
                .answers
                .iter()
                .filter(|(_, _, outcome)| outcome.anger_change > 0)
                .count();
            let wraths = run
                .answers
                .iter()
                .filter(|(_, _, outcome)| outcome.wrath)
                .count();
            return Some(Report {
                fate,
                answers: run.answers.len(),
                informed,
                insults,
                wraths,
            });
        }
        if let Screen::Calling(call) = run.screen
            && run
                .state
                .knows(run.night.calls[call.slot].question(call.asked))
        {
            informed += 1;
        }
        // Every row decided is a row the screen drew.
        let command = decide(player, &run);
        if let Command::Choose(row) = command
            && row >= layout(&run).choices.len()
        {
            return None;
        }
        run.step(command);
    }
    None
}
