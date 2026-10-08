//! The instrument: failed checks kept rather than exited on, the headless
//! driver with its keyboard, and the frame readings the gates share.

use std::collections::BTreeSet;
use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FrameRecord, FrameRecorder, InputEvent, SnapshotBuilder,
};

use crate::model::WINDOW;
use crate::players::{Player, intent_of, keys_for};
use crate::world::{Snap, snap};
use crate::{config, register};

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

    /// How many have failed.
    pub fn failed(&self) -> usize {
        self.problems.len()
    }

    /// Print every failure; say whether there were any.
    pub fn verdict(&self) -> ExitCode {
        for (what, specifics) in &self.problems {
            eprintln!(
                "{}",
                message(
                    what,
                    specifics,
                    "the game changed, or the check's expectation is stale",
                    "run `cargo run -p brolf_r3v2` and play the case, then compare with the \
                     assertion above",
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

/// Frames a recorder keeps before it is replaced, so a long run's memory stays flat.
const FRAMES_PER_RECORDER: usize = 64;

/// The game, headless, driven by key events.
pub struct Driver {
    sim: HeadlessSim,
    recorder: FrameRecorder,
    kept: usize,
    keyboard: SnapshotBuilder,
    held: BTreeSet<Key>,
    tapped: Vec<Key>,
    /// Ticks run.
    pub tick: u64,
}

impl Driver {
    /// A fresh game, not yet ticked.
    pub fn new() -> Self {
        Driver {
            sim: headless(config(), register),
            recorder: FrameRecorder::new(WINDOW),
            kept: 0,
            keyboard: SnapshotBuilder::new(),
            held: BTreeSet::new(),
            tapped: Vec::new(),
            tick: 0,
        }
    }

    /// One tick holding exactly `held` and tapping `taps` (each released next tick).
    pub fn tick_with(&mut self, held: &[Key], taps: &[Key]) {
        for key in std::mem::take(&mut self.tapped) {
            if !held.contains(&key) {
                self.keyboard.record(InputEvent::KeyReleased(key));
            }
        }
        let wanted: BTreeSet<Key> = held.iter().copied().collect();
        for key in self.held.difference(&wanted) {
            self.keyboard.record(InputEvent::KeyReleased(*key));
        }
        for key in wanted.difference(&self.held) {
            self.keyboard.record(InputEvent::KeyPressed(*key));
        }
        self.held = wanted;
        for key in taps {
            if !self.held.contains(key) {
                self.keyboard.record(InputEvent::KeyPressed(*key));
                self.tapped.push(*key);
            }
        }
        let snapshot = self.keyboard.first_tick_snapshot();
        self.sim.world_mut().insert_resource(Input::new(snapshot));
        self.sim.tick();
        self.tick += 1;
    }

    /// One tick with nothing pressed.
    pub fn idle(&mut self) {
        self.tick_with(&[], &[]);
    }

    /// One tick tapping `key`.
    pub fn tap(&mut self, key: Key) {
        self.tick_with(&[], &[key]);
    }

    /// One tick of `player` at the keyboard. Returns whether it tapped Space.
    pub fn play(&mut self, player: Player) -> bool {
        let view = self.snap();
        let intent = intent_of(player, &view);
        let (held, mut taps) = match view.golfer(0) {
            Some(me) => keys_for(&intent, me, view.dt),
            None => (Vec::new(), Vec::new()),
        };
        // A key tapped last tick is still on its way up: never tap it twice running.
        taps.retain(|key| !self.tapped.contains(key));
        let shot = taps.contains(&Key::Space);
        self.tick_with(&held, &taps);
        shot
    }

    /// The world, read.
    pub fn snap(&self) -> Snap {
        snap(&self.sim.world().view())
    }

    /// The world, to stage.
    pub fn world_mut(&mut self) -> &mut World {
        self.sim.world_mut()
    }

    /// The world, to read directly.
    pub fn world(&self) -> &World {
        self.sim.world()
    }

    /// Draw a frame of the world as it stands.
    pub fn draw(&mut self) -> FrameRecord {
        if self.kept >= FRAMES_PER_RECORDER {
            self.recorder = FrameRecorder::new(WINDOW);
            self.kept = 0;
        }
        self.kept += 1;
        self.recorder.draw(&mut self.sim)
    }

    /// The font's texture in this driver's recorder.
    pub fn font(&self) -> BackendTextureId {
        self.recorder.font_texture()
    }

    /// The order the systems run in.
    pub fn schedule(&self) -> String {
        self.sim.schedule_debug()
    }
}

/// The font quads on the text row whose top is `y`.
pub fn glyphs_at(frame: &FrameRecord, font: BackendTextureId, y: f32) -> usize {
    frame
        .quads()
        .iter()
        .filter(|quad| quad.texture == font && (quad.bounds().min.y - y).abs() < 1e-3)
        .count()
}

/// The camera a driver's frames are drawn with: the game's, at the window's size.
pub fn camera(driver: &Driver) -> Camera {
    Camera {
        viewport: WINDOW,
        ..*driver.world().resource::<Camera>()
    }
}
