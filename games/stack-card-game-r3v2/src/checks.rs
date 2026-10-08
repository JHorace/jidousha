//! The instrument: failed checks kept rather than exited on, the album of
//! frames every screen-wide check runs over, and the headless driver.
//!
//! Nobody running `--verify` can look at the game, so each failure reports the
//! numbers it judged, and a failed check does not stop the run (jidousha-
//! testing.md, "Collect the failures; do not exit on the first one").

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FrameRecord, FrameRecorder, InputEvent, SnapshotBuilder,
};

use crate::{Flow, WINDOW, config, register};

/// Every failed check, kept.
#[derive(Default)]
pub struct Checks {
    problems: Vec<(String, String)>,
}

impl Checks {
    /// Note a failure unless `ok`.
    pub fn require(&mut self, ok: bool, what: &str, specifics: String) {
        if !ok {
            self.problems.push((what.to_owned(), specifics));
        }
    }

    /// How many checks have failed so far.
    pub fn failed(&self) -> usize {
        self.problems.len()
    }

    /// Print everything that failed, and say whether anything did.
    pub fn verdict(&self) -> ExitCode {
        for (what, specifics) in &self.problems {
            eprintln!(
                "{}",
                message(
                    what,
                    specifics,
                    "the game changed, or the check's expectation is stale",
                    "run `cargo run -p stack_card_game_r3v2` and play the case, then compare \
                     with the assertion above",
                )
            );
        }
        if self.problems.is_empty() {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    }
}

/// Stop the run, for a reading that makes every later one meaningless.
pub fn fail(what: &str, specifics: &str) -> ! {
    eprintln!(
        "{}",
        message(what, specifics, "the game changed", "read the check above")
    );
    std::process::exit(1);
}

/// One recorded frame, the state it was drawn from, and what to call it.
pub struct Shot {
    /// What the frame is of.
    pub label: String,
    /// The frame.
    pub frame: FrameRecord,
    /// The flow it was drawn from.
    pub flow: Flow,
    /// Which texture the font landed on, in the recorder that drew it.
    pub font: BackendTextureId,
}

/// Every frame the run drew, for the checks that hold of every screen.
#[derive(Default)]
pub struct Album {
    /// The shots, in the order they were taken.
    pub shots: Vec<Shot>,
}

/// A headless game driven by key events, one per tick at most.
pub struct Driver {
    sim: HeadlessSim,
    recorder: FrameRecorder,
    keyboard: SnapshotBuilder,
    held: Option<Key>,
    /// Ticks run so far.
    pub ticks: u64,
}

impl Driver {
    /// The game with `staged` in the world before tick 1 (or dealt from
    /// `seed`), run for one tick so `Startup` has happened.
    pub fn new(staged: Option<Flow>, seed: u64) -> Self {
        let mut sim = headless(GameConfig { seed, ..config() }, register);
        if let Some(flow) = staged {
            sim.world_mut().insert_resource(flow);
        }
        let mut driver = Driver {
            sim,
            recorder: FrameRecorder::new(WINDOW),
            keyboard: SnapshotBuilder::new(),
            held: None,
            ticks: 0,
        };
        driver.step();
        driver
    }

    /// One tick, releasing the key pressed on the tick before.
    pub fn step(&mut self) {
        if let Some(key) = self.held.take() {
            self.keyboard.record(InputEvent::KeyReleased(key));
        }
        let snapshot = self.keyboard.first_tick_snapshot();
        self.sim.world_mut().insert_resource(Input::new(snapshot));
        self.sim.tick();
        self.ticks += 1;
    }

    /// Press `key` on the next tick that has no release on it, and run that tick.
    pub fn press(&mut self, key: Key) {
        if self.held.is_some() {
            self.step();
        }
        self.keyboard.record(InputEvent::KeyPressed(key));
        let snapshot = self.keyboard.first_tick_snapshot();
        self.sim.world_mut().insert_resource(Input::new(snapshot));
        self.sim.tick();
        self.ticks += 1;
        self.held = Some(key);
    }

    /// The flow as the last tick left it.
    pub fn flow(&self) -> &Flow {
        self.sim.world().resource::<Flow>()
    }

    /// Tick until You hold priority or the match is over, at most `limit` ticks.
    pub fn until_yours(&mut self, limit: u64) -> bool {
        for _ in 0..limit {
            let duel = &self.flow().duel;
            let yours = duel.priority == crate::rules::Side::You;
            if (yours || duel.phase != crate::duel::Phase::Live) && self.held.is_none() {
                return true;
            }
            self.step();
        }
        false
    }

    /// Draw one frame of the world as it stands, and keep it in `album`.
    pub fn shoot(&mut self, album: &mut Album, label: &str) -> FrameRecord {
        let frame = self.recorder.draw(&mut self.sim);
        album.shots.push(Shot {
            label: label.to_owned(),
            frame: frame.clone(),
            flow: self.flow().clone(),
            font: self.recorder.font_texture(),
        });
        frame
    }

    /// The order the game's systems run in.
    pub fn schedule(&self) -> String {
        self.sim.schedule_debug()
    }

    /// The camera the frames are drawn with: the game's, at the recorder's viewport.
    pub fn camera(&self) -> Camera {
        Camera {
            viewport: WINDOW,
            ..*self.sim.world().resource::<Camera>()
        }
    }
}
