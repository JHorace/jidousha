//! The headless driver every check uses: the real game, the real systems, and input sent
//! as events through the engine's own accumulator, so a key goes through the same edge
//! rules a keyboard does and a click lands where the screen drew its row.

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder, InputEvent, SnapshotBuilder};

use crate::game::Game;
use crate::view::UiMap;
use crate::{RunSeed, WINDOW, camera, config, register};

/// A running game, a recorder of its frames, and a keyboard and mouse.
pub struct Driver {
    /// The simulation.
    pub sim: HeadlessSim,
    /// Every frame drawn so far.
    pub recorder: FrameRecorder,
    events: SnapshotBuilder,
}

impl Driver {
    /// A game from `seed`, started: `Startup` has run and the first screen is up.
    pub fn new(seed: u64) -> Self {
        let mut sim = headless(config(), register);
        sim.world_mut().insert_resource(RunSeed(seed));
        sim.tick();
        Self {
            sim,
            recorder: FrameRecorder::new(WINDOW),
            events: SnapshotBuilder::new(),
        }
    }

    /// The game as it stands.
    pub fn game(&self) -> &Game {
        self.sim.world().resource::<Game>()
    }

    fn tick_with_events(&mut self) {
        let snapshot = self.events.first_tick_snapshot();
        self.sim.world_mut().insert_resource(Input::new(snapshot));
        self.sim.tick();
    }

    /// Tap `key`: down on one tick, up on the next.
    pub fn press(&mut self, key: Key) {
        self.events.record(InputEvent::KeyPressed(key));
        self.tick_with_events();
        self.events.record(InputEvent::KeyReleased(key));
        self.tick_with_events();
    }

    /// Click the design-space point `at`, through the camera the game draws with.
    pub fn click(&mut self, at: Vec2) {
        let cam = camera();
        let screen = cam.world_to_screen(UiMap::for_camera(&cam).to_world_point(at));
        self.events.record(InputEvent::PointerMoved {
            id: PointerId::PRIMARY,
            screen,
        });
        self.events.record(InputEvent::ButtonPressed {
            id: PointerId::PRIMARY,
            button: PointerButton::Primary,
        });
        self.tick_with_events();
        self.events.record(InputEvent::ButtonReleased {
            id: PointerId::PRIMARY,
            button: PointerButton::Primary,
        });
        self.tick_with_events();
    }

    /// Press the number key for option `n` (0-based).
    pub fn option(&mut self, n: usize) {
        const DIGITS: [Key; 8] = [
            Key::Digit1,
            Key::Digit2,
            Key::Digit3,
            Key::Digit4,
            Key::Digit5,
            Key::Digit6,
            Key::Digit7,
            Key::Digit8,
        ];
        if let Some(&key) = DIGITS.get(n) {
            self.press(key);
        }
    }

    /// Draw one frame of the screen as it is.
    pub fn draw(&mut self) -> FrameRecord {
        self.recorder.draw(&mut self.sim)
    }

    /// Replace the game, for staging a screen the run never reaches.
    pub fn stage(&mut self, game: Game) {
        self.sim.world_mut().insert_resource(game);
    }
}
