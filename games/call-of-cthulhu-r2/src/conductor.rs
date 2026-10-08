//! A scripted session for the checks that ask one question at one moment:
//! press this, tap that, photograph what is on screen and the state it was
//! drawn from.

use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FrameRecord, FrameRecorder, InputEvent, SnapshotBuilder,
};
use jidousha::ui::Panel;

use crate::play::Game;
use crate::screens::{self, Art, WINDOW};
use crate::{RunSeed, config, register};

/// One sim, its keyboard, and a recorder.
pub struct Conductor {
    sim: HeadlessSim,
    keyboard: SnapshotBuilder,
    recorder: FrameRecorder,
}

impl Conductor {
    /// A run on `seed`, ticked once so Startup has made the game.
    pub fn new(seed: u64) -> Self {
        let mut sim = headless(config(), register);
        sim.world_mut().insert_resource(RunSeed(seed));
        let mut conductor = Self {
            sim,
            keyboard: SnapshotBuilder::new(),
            recorder: FrameRecorder::new(WINDOW),
        };
        conductor.idle(1);
        conductor
    }

    pub fn game(&self) -> &Game {
        self.sim.world().resource::<Game>()
    }

    pub fn game_mut(&mut self) -> &mut Game {
        self.sim.world_mut().resource_mut::<Game>()
    }

    pub fn font(&self) -> BackendTextureId {
        self.recorder.font_texture()
    }

    /// `ticks` ticks of the player doing nothing.
    pub fn idle(&mut self, ticks: u64) {
        for _ in 0..ticks {
            let snapshot = self.keyboard.first_tick_snapshot();
            self.sim.world_mut().insert_resource(Input::new(snapshot));
            self.sim.tick();
        }
    }

    /// Press `key` for one tick and let it go on the next.
    pub fn press(&mut self, key: Key) {
        self.keyboard.record(InputEvent::KeyPressed(key));
        self.idle(1);
        self.keyboard.record(InputEvent::KeyReleased(key));
        self.idle(1);
    }

    /// Tap the primary pointer at a world point, through the game's camera.
    pub fn tap(&mut self, world: Vec2) {
        let screen = crate::camera().world_to_screen(world);
        let id = PointerId::PRIMARY;
        self.keyboard
            .record(InputEvent::PointerMoved { id, screen });
        self.keyboard.record(InputEvent::ButtonPressed {
            id,
            button: PointerButton::Primary,
        });
        self.idle(1);
        self.keyboard.record(InputEvent::ButtonReleased {
            id,
            button: PointerButton::Primary,
        });
        self.idle(1);
    }

    /// Photograph the screen: the frame, the panel it was drawn from, the game.
    pub fn photo(&mut self) -> Photo {
        let frame = self.recorder.draw(&mut self.sim);
        let game = self.game().clone();
        Photo {
            panel: screens::screen(&game),
            game,
            frame,
        }
    }
}

/// One photographed moment.
pub struct Photo {
    pub game: Game,
    pub panel: Panel<Art>,
    pub frame: FrameRecord,
}

impl Photo {
    /// Every string the screen draws, one per row.
    pub fn strings(&self) -> Vec<String> {
        self.panel.all_strings().map(str::to_owned).collect()
    }

    /// Whether some row of the screen says exactly `text`.
    pub fn shows(&self, text: &str) -> bool {
        self.panel.all_strings().any(|row| row == text)
    }

    /// Whether some row of the screen contains `text`.
    pub fn mentions(&self, text: &str) -> bool {
        self.panel.all_strings().any(|row| row.contains(text))
    }
}
