//! The three players, and the driver that plays one through the real sim.
//!
//! Each player is a function from the game to the choice it makes on the
//! current screen, read off `morning_options` and `call_options` by kind,
//! never by a hard-coded digit. The driver presses that choice's key through
//! a `SnapshotBuilder` — one press per screen, one idle tick between presses —
//! so every choice goes through the same `play` system a keyboard does.

use crate::beings::BeingId;
use crate::flow::{Choice, Game, MorningAction, Screen, call_options, morning_options};
use crate::rules::{AnswerKind, answer_outcome, total_cost};
use crate::screen::Art;
use crate::{DIGITS, config, register};
use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder, InputEvent, SnapshotBuilder};
use jidousha::ui::Panel;

/// Who is playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    /// Studies tonight's caller and answers from its lore.
    Scholar,
    /// Studies like the scholar, then never uses it: always the plain wrong reply.
    Guesser,
    /// Steadies its nerves, then offends every caller.
    Blasphemer,
}

impl Player {
    pub const ALL: [Player; 3] = [Player::Scholar, Player::Guesser, Player::Blasphemer];

    pub fn name(self) -> &'static str {
        match self {
            Player::Scholar => "scholar",
            Player::Guesser => "guesser",
            Player::Blasphemer => "blasphemer",
        }
    }
}

/// The index of the first option of `kind` on the call screen.
fn reply(game: &Game, kind: AnswerKind) -> Option<usize> {
    call_options(game).iter().position(|(k, _)| *k == kind)
}

/// The index of `action` on the morning screen.
fn morning(game: &Game, action: MorningAction) -> Option<usize> {
    morning_options(game).iter().position(|a| *a == action)
}

/// The first of tonight's callers with fewer than two secrets known, if any.
fn worth_studying(game: &Game) -> Option<BeingId> {
    game.plan()
        .calls
        .iter()
        .map(|call| call.being)
        .find(|being| game.secrets_known(*being) < 2)
}

/// What `player` does on the current screen.
pub fn decide(player: Player, game: &Game) -> Choice {
    match game.screen {
        Screen::Morning => {
            let action = match player {
                Player::Scholar | Player::Guesser => {
                    worth_studying(game).map_or(MorningAction::Meditate, MorningAction::Study)
                }
                Player::Blasphemer => MorningAction::Meditate,
            };
            Choice::Pick(morning(game, action).unwrap_or(0))
        }
        Screen::Exchange => {
            let pick = match player {
                Player::Scholar => reply(game, AnswerKind::Lore).or(reply(game, AnswerKind::Wrong)),
                Player::Guesser => reply(game, AnswerKind::Wrong),
                Player::Blasphemer => reply(game, AnswerKind::Anger),
            };
            Choice::Pick(pick.unwrap_or(0))
        }
        Screen::Dawn | Screen::End { .. } => Choice::Continue,
    }
}

/// What one driven run did.
pub struct Run {
    /// The game when the run stopped.
    pub game: Game,
    /// Questions put to the player, and how many it answered.
    pub asked: u32,
    pub answered: u32,
    /// Replies from the being's lore.
    pub lore: u32,
    /// What `answer_outcome` said the replies would cost, summed before each press.
    pub predicted: u32,
    /// What the replies actually took off sanity.
    pub paid: u32,
    /// The last frame drawn while a call was on the line.
    pub live: Option<FrameRecord>,
    /// Every frame drawn, one per press, with the panel it was drawn from.
    pub frames: Vec<(Panel<Art>, FrameRecord)>,
}

/// The sim for `seed`, with Startup run.
pub fn sim_for(seed: u64) -> HeadlessSim {
    let mut sim = headless(GameConfig { seed, ..config() }, register);
    sim.tick();
    sim
}

/// The game in a sim, which Startup always inserts.
pub fn game_of(sim: &HeadlessSim) -> Game {
    sim.world().resource::<Game>().clone()
}

/// Drive a fresh run on `seed`, asking `policy` for each screen's choice,
/// until `stop` says so or the run ends.
pub fn drive(
    seed: u64,
    policy: &mut dyn FnMut(&Game) -> Choice,
    stop: &dyn Fn(&Game) -> bool,
    mut recorder: Option<&mut FrameRecorder>,
) -> Run {
    let mut sim = sim_for(seed);
    let mut keyboard = SnapshotBuilder::new();
    let mut run = Run {
        game: game_of(&sim),
        asked: 0,
        answered: 0,
        lore: 0,
        predicted: 0,
        paid: 0,
        live: None,
        frames: Vec::new(),
    };
    for _ in 0..400 {
        let game = game_of(&sim);
        if matches!(game.screen, Screen::End { .. }) || stop(&game) {
            break;
        }
        let choice = policy(&game);
        let key = match choice {
            Choice::Pick(index) => DIGITS.get(index).copied().unwrap_or(Key::Digit1),
            Choice::Continue => Key::Enter,
        };
        let on_call = game.screen == Screen::Exchange;
        if on_call {
            run.asked += 1;
            if let (Choice::Pick(index), Some(being)) = (choice, game.caller())
                && let Some((kind, _)) = call_options(&game).get(index).copied()
            {
                let temper = game.tempers[being.index()];
                let outcome = answer_outcome(being, temper, game.meditated, kind);
                run.predicted += total_cost(temper, outcome);
                if kind == AnswerKind::Lore {
                    run.lore += 1;
                }
            }
        }
        keyboard.record(InputEvent::KeyPressed(key));
        sim.world_mut()
            .insert_resource(Input::new(keyboard.first_tick_snapshot()));
        sim.tick();
        let after = game_of(&sim);
        if on_call && after.transcript.len() > game.transcript.len() {
            run.answered += 1;
            run.paid += (game.sanity - after.sanity).max(0) as u32;
        }
        if let Some(recorder) = recorder.as_deref_mut() {
            let frame = recorder.draw(&mut sim);
            if after.screen == Screen::Exchange {
                run.live = Some(frame.clone());
            }
            run.frames.push((crate::screen::screen(&after), frame));
        }
        keyboard.record(InputEvent::KeyReleased(key));
        sim.world_mut()
            .insert_resource(Input::new(keyboard.first_tick_snapshot()));
        sim.tick();
    }
    run.game = game_of(&sim);
    run
}

/// One player's whole run.
pub fn play(seed: u64, player: Player, recorder: Option<&mut FrameRecorder>) -> Run {
    drive(seed, &mut |game| decide(player, game), &|_| false, recorder)
}
