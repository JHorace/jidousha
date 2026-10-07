//! The `--verify` mode: staged sessions, one per decision row, then the three players,
//! then every frame; failures collected, the verdict first, one frame after.
//!
//! Key types: `Run`, `Gallery`, `Shot`. Key functions: `run`.
//! Depends on: the gate files (`gates`, `gates_play`, `gates_end`, `gates_frames`),
//! `capture`, `checks`. Never depended on outside the game.
//! INVARIANT: the run is deterministic: seed 7, a fixed timestep, scripted input and
//! controllers that send events through the keyboard's own accumulator.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord, FrameRecorder, InputScript, InputSnapshot};

use crate::checks::Checks;
use crate::text::{cue_line, pickup_line, result_lines, status_line_1, status_line_2};
use crate::{WINDOW, config, register};

/// The viewport the recorder draws at: the window's own.
pub const HEADLESS_VIEWPORT: PhysicalSize = WINDOW;
/// The longest any session runs.
pub const MAX_TICKS: u64 = 9300;

/// One headless game and the script feeding it.
pub struct Run {
    /// The simulation.
    pub sim: HeadlessSim,
    /// What the player does, as a function of the tick.
    pub script: InputScript,
    /// The last tick run.
    pub tick: u64,
}

impl Run {
    /// A new game on the shipped config, before any tick.
    pub fn new() -> Run {
        Run {
            sim: headless(config(), register),
            script: InputScript::new(),
            tick: 0,
        }
    }

    /// One tick, fed that tick's scripted input.
    pub fn step(&mut self) {
        self.tick += 1;
        let input = Input::new(self.script.snapshot_at(self.tick));
        self.sim.world_mut().insert_resource(input);
        self.sim.tick();
    }

    /// One tick, fed the input a controller built for it.
    pub fn step_with(&mut self, snapshot: InputSnapshot) {
        self.tick += 1;
        self.sim.world_mut().insert_resource(Input::new(snapshot));
        self.sim.tick();
    }

    /// Run until `tick` has run.
    pub fn to(&mut self, tick: u64) {
        while self.tick < tick {
            self.step();
        }
    }

    /// The world, for reading.
    pub fn world(&self) -> &World {
        self.sim.world()
    }

    /// The camera the recorder draws with.
    pub fn camera(&self) -> Camera {
        Camera {
            viewport: HEADLESS_VIEWPORT,
            ..*self.sim.world().resource::<Camera>()
        }
    }
}

/// A recorded frame and the sentences drawn on it.
pub struct Shot {
    /// What it is a picture of.
    pub label: String,
    /// The frame.
    pub frame: FrameRecord,
    /// Which texture the font is on.
    pub font: BackendTextureId,
    /// Every sentence the game drew on it.
    pub strings: Vec<String>,
}

/// Every frame the sessions recorded, and what they have to say.
#[derive(Default)]
pub struct Gallery {
    /// The frames, for the every-frame gates.
    pub shots: Vec<Shot>,
    /// The summary lines, in order.
    pub summary: Vec<String>,
}

impl Gallery {
    /// Draw one frame of `run` and keep it with its sentences; hand back its index.
    pub fn snap(&mut self, recorder: &mut FrameRecorder, run: &mut Run, label: &str) -> usize {
        let frame = recorder.draw(&mut run.sim);
        let world = run.sim.world();
        let view = world.view();
        let mut strings = vec![
            status_line_1(&view),
            status_line_2(&view),
            cue_line(&view),
            pickup_line(&view),
        ];
        strings.extend(result_lines(&view));
        self.shots.push(Shot {
            label: label.to_owned(),
            frame,
            font: recorder.font_texture(),
            strings,
        });
        self.shots.len() - 1
    }

    /// Add a summary line.
    pub fn say(&mut self, line: String) {
        self.summary.push(line);
    }
}

/// Run every session and gate, print the verdict, the summary and one frame.
pub fn run() -> ExitCode {
    let mut checks = Checks::default();
    let mut gallery = Gallery::default();
    let aiming = crate::gates::session_a(&mut checks, &mut gallery);
    crate::gates_play::session_b(&mut checks, &mut gallery);
    crate::gates_play::session_c(&mut checks, &mut gallery);
    crate::gates_end::session_d(&mut checks, &mut gallery);
    crate::gates_end::session_e(&mut checks, &mut gallery);
    crate::gates_frames::session_f(&mut checks, &mut gallery, aiming);
    let verdict = checks.verdict();
    if checks.failures() == 0 {
        println!(
            "verified brolf over seed {} and {} frames",
            crate::SEED,
            gallery.shots.len()
        );
    } else {
        println!("FAILED brolf: {} checks failed", checks.failures());
    }
    for line in &gallery.summary {
        println!("  {line}");
    }
    if let Some(shot) = gallery.shots.get(aiming) {
        print!("{}", shot.frame.transcript());
    }
    verdict
}
