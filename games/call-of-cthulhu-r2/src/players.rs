//! The players a `--verify` run plays with, and the driver that plays them.
//!
//! Three, because one cannot say whether the game is worth playing: the
//! scholar prepares and answers from lore (it should win), the first-timer
//! studies at random and guesses (the line that says whether a person on a
//! first try has a chance), and the silent one spends its mornings and never
//! says a word on the line (it must lose — silence is not free).
//!
//! Every player reads only what the screen shows: the morning's options and
//! their effect lines, the answers, and the hints `answer_outcome` puts on
//! them when the question is about a known fact. The scholar and the
//! first-timer differ in one more thing a person does: the scholar reads the
//! answers' tone and never picks the rude one; the first-timer does not.

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FrameRecord, FrameRecorder, InputEvent, InputSnapshot, SnapshotBuilder,
};

use crate::lore::{AnswerKind, Being};
use crate::play::{Game, Phase};
use crate::rules::{self, MorningAction};
use crate::screens::Pick;
use crate::{RunSeed, config, register};

/// How long a player looks at a screen before it acts, in ticks.
pub const THINK_TICKS: u64 = 45;
/// The most ticks a session may run before the driver calls it stuck.
pub const MAX_TICKS: u64 = 400_000;

/// Who is playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    Scholar,
    FirstTimer,
    Silent,
}

/// What a player decided to do this tick, as the key a person would press.
fn decide(player: Player, game: &Game, rng: &mut Rng) -> Option<Key> {
    match &game.phase {
        Phase::Morning => {
            let options = rules::morning_options(&game.run);
            let index = match player {
                Player::Scholar => rollout_morning(game, &options, rng),
                Player::FirstTimer => rng.below(options.len() as u32) as usize,
                Player::Silent => 0,
            };
            Some(digit(index))
        }
        Phase::Call(_) => match player {
            Player::Silent => None,
            Player::Scholar => Some(digit(answer_from(game, rng, false))),
            Player::FirstTimer => Some(digit(answer_from(game, rng, false))),
        },
        Phase::Hangup(_) => Some(Key::Enter),
        Phase::Ended(_) => None,
    }
}

/// How many futures the scholar rolls forward per morning option.
const ROLLOUTS: usize = 12;

/// The scholar's morning: try every option on a copy of the day, play each
/// future out with the plain heuristic below, and take the best mean.
///
/// The futures are played with the same rules the game runs — `Game` is plain
/// data, so a copy rolls forward without a sim — and answered the way the
/// scholar answers: from a hint when there is one, else a guess between the
/// two answers that are not rude. They do see which beings call on later
/// nights, which a person cannot; that is the one advantage the scholar has
/// over a careful person, and it is why its line is the upper bound.
fn rollout_morning(game: &Game, options: &[MorningAction], rng: &mut Rng) -> usize {
    let mut best = 0;
    let mut best_value = f32::MIN;
    for index in 0..options.len() {
        let mut total = 0.0;
        for _ in 0..ROLLOUTS {
            let mut future = game.clone();
            future.choose_morning(index);
            total += value(&finish(future, rng));
        }
        let mean = total / ROLLOUTS as f32;
        if mean > best_value {
            best = index;
            best_value = mean;
        }
    }
    best
}

/// A finished run's worth: the sanity it survived with, or how early it died.
fn value(game: &Game) -> f32 {
    match game.phase {
        Phase::Ended(crate::play::Ending::Won { sanity }) => sanity as f32,
        Phase::Ended(crate::play::Ending::Lost { day, .. }) => {
            -30.0 * (rules::DAYS + 1 - day) as f32
        }
        _ => 0.0,
    }
}

/// Play a copy of the game to its end with the heuristic scholar, answering at
/// once.
fn finish(mut game: Game, rng: &mut Rng) -> Game {
    for _ in 0..10_000 {
        match &game.phase {
            Phase::Morning => {
                let options = rules::morning_options(&game.run);
                let index = scholar_morning(&game, &options);
                game.choose_morning(index);
            }
            Phase::Call(_) => {
                let index = answer_from(&game, rng, false);
                game.answer(index);
            }
            Phase::Hangup(_) => game.go_on(),
            Phase::Ended(_) => break,
        }
    }
    game
}

/// The heuristic morning: study whoever calls tonight with the dearest drain
/// and something left to learn; with nothing to study, train; else disrupt
/// the dearest caller.
fn scholar_morning(game: &Game, options: &[MorningAction]) -> usize {
    let run = &game.run;
    let callers = rules::tonight(run).callers();
    let dearest = |being: &Being| rules::drain(*being, run.temper_of(*being), run.composure);
    let mut studyable: Vec<Being> = callers
        .iter()
        .copied()
        .filter(|being| options.contains(&MorningAction::Study(*being)))
        .collect();
    studyable.sort_by_key(|being| -dearest(being));
    let want = match studyable.first() {
        _ if options.contains(&MorningAction::Train) => MorningAction::Train,
        Some(being) => MorningAction::Study(*being),
        None => {
            let mut by_cost = callers.clone();
            by_cost.sort_by_key(|being| -dearest(being));
            match by_cost.first() {
                Some(being) => MorningAction::Disrupt(*being),
                None => options.first().copied().unwrap_or(MorningAction::Train),
            }
        }
    };
    options.iter().position(|each| *each == want).unwrap_or(0)
}

/// Pick an answer from what the screen shows: the hinted lore answer if there
/// is one, else a guess — over all three if `blind`, else over the two that
/// are not rude.
fn answer_from(game: &Game, rng: &mut Rng, blind: bool) -> usize {
    let Some(asking) = game.asking() else {
        return 0;
    };
    let Phase::Call(call) = &game.phase else {
        return 0;
    };
    if game.run.knows(call.being, asking.fact) {
        // The hint says which answer shortens the call most; take it.
        let mut best = 0;
        let mut best_change = i32::MAX;
        for index in 0..3 {
            if let Some((_, outcome)) = game.preview(index)
                && outcome.interest_change < best_change
            {
                best = index;
                best_change = outcome.interest_change;
            }
        }
        return best;
    }
    let candidates: Vec<usize> = (0..3)
        .filter(|index| blind || asking.order[*index] != AnswerKind::Insult)
        .collect();
    candidates[rng.below(candidates.len() as u32) as usize]
}

fn digit(index: usize) -> Key {
    const DIGITS: [Key; 7] = [
        Key::Digit1,
        Key::Digit2,
        Key::Digit3,
        Key::Digit4,
        Key::Digit5,
        Key::Digit6,
        Key::Digit7,
    ];
    DIGITS.get(index).copied().unwrap_or(Key::Digit9)
}

/// One photographed moment: the frame, and the game state it was drawn from.
pub struct Shot {
    pub tick: u64,
    pub game: Game,
    pub frame: FrameRecord,
}

/// What one session did.
pub struct Session {
    pub seed: u64,
    pub ticks: u64,
    /// The game at the end.
    pub game: Game,
    /// One shot each time a decision screen first appears (if recorded).
    pub shots: Vec<Shot>,
    /// How many answers of each kind were given: lore, wrong, insult.
    pub answers: [u32; 3],
    /// Answers given where the screen showed the hint.
    pub hinted: u32,
    /// Predicted sanity cost minus applied cost, summed over every answer —
    /// the player's aim: what it was shown versus what happened.
    pub aim_error: i32,
    /// The schedule, in run order, as the sim reports it.
    pub schedule: String,
    /// Which backend texture the font landed on, if frames were recorded.
    pub font: Option<BackendTextureId>,
}

/// Play one session to its end. `record` photographs every new screen.
pub fn play(player: Player, seed: u64, record: bool) -> Session {
    let mut sim = headless(config(), register);
    sim.world_mut().insert_resource(RunSeed(seed));
    let mut recorder = record.then(|| FrameRecorder::new(crate::screens::WINDOW));
    let mut keyboard = SnapshotBuilder::new();
    let mut rng = Rng::from_seed(seed ^ 0x5EED_F5C4_01A5);
    let mut held: Option<Key> = None;
    let mut seen_at = 0;
    let mut last_phase: Option<Phase> = None;
    let mut shots = Vec::new();
    let mut answers = [0; 3];
    let mut hinted = 0;
    let mut aim_error = 0;
    let mut tick = 0;
    while tick < MAX_TICKS {
        tick += 1;
        if let Some(key) = held.take() {
            keyboard.record(InputEvent::KeyReleased(key));
        } else if let Some(game) = sim.world().find_resource::<Game>() {
            if matches!(game.phase, Phase::Ended(_)) {
                break;
            }
            if last_phase.as_ref() != Some(&game.phase)
                && !same_screen(last_phase.as_ref(), &game.phase)
            {
                seen_at = tick;
                last_phase = Some(game.phase.clone());
                if let Some(recorder) = recorder.as_mut() {
                    shots.push(Shot {
                        tick,
                        game: game.clone(),
                        frame: recorder.draw(&mut sim),
                    });
                }
            }
            let game = sim.world().resource::<Game>();
            if tick - seen_at >= THINK_TICKS
                && let Some(key) = decide(player, game, &mut rng)
            {
                if let Some(index) = answer_index(key)
                    && let Some((kind, outcome)) = game.preview(index)
                {
                    answers[kind_index(kind)] += 1;
                    if let Phase::Call(call) = &game.phase
                        && let Some(asking) = game.asking()
                        && game.run.knows(call.being, asking.fact)
                    {
                        hinted += 1;
                    }
                    let before = game.run.sanity;
                    sim.world_mut()
                        .insert_resource(Input::new(press(&mut keyboard, key)));
                    sim.tick();
                    let after = sim.world().resource::<Game>();
                    // The last answer of a lost run is clamped at zero, so it
                    // says nothing about aim.
                    if !matches!(after.phase, Phase::Ended(_)) {
                        aim_error += outcome.sanity_cost - (before - after.run.sanity);
                    }
                    held = Some(key);
                    continue;
                }
                keyboard.record(InputEvent::KeyPressed(key));
                held = Some(key);
            }
        }
        sim.world_mut()
            .insert_resource(Input::new(keyboard.first_tick_snapshot()));
        sim.tick();
    }
    let game = sim.world().resource::<Game>().clone();
    let font = recorder.as_ref().map(FrameRecorder::font_texture);
    Session {
        font,
        seed,
        ticks: tick,
        game,
        shots,
        answers,
        hinted,
        aim_error,
        schedule: sim.schedule_debug(),
    }
}

/// Two phases that are the same screen as far as a photograph cares: one call,
/// the moment its question changes, is a new screen; a silence tick is not.
fn same_screen(before: Option<&Phase>, now: &Phase) -> bool {
    match (before, now) {
        (Some(Phase::Call(a)), Phase::Call(b)) => {
            a.slot == b.slot && a.asked == b.asked && a.last == b.last
        }
        _ => false,
    }
}

fn press(keyboard: &mut SnapshotBuilder, key: Key) -> InputSnapshot {
    keyboard.record(InputEvent::KeyPressed(key));
    keyboard.first_tick_snapshot()
}

fn answer_index(key: Key) -> Option<usize> {
    [Key::Digit1, Key::Digit2, Key::Digit3]
        .iter()
        .position(|each| *each == key)
}

pub fn kind_index(kind: AnswerKind) -> usize {
    match kind {
        AnswerKind::Lore => 0,
        AnswerKind::Wrong => 1,
        AnswerKind::Insult => 2,
    }
}

/// A screen's pick, as the key a person would press for it.
pub fn key_for(pick: Pick) -> Key {
    match pick {
        Pick::Morning(index) | Pick::Answer(index) => digit(index),
        Pick::GoOn => Key::Enter,
    }
}
