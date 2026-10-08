//! The `--verify` mode: six headless runs, the gates over them, a verdict,
//! the summary and run A's tick-49 transcript.
//!
//! Runs **A** and **B** are scripted and staged through `world_mut` after named
//! ticks, so every decision row of the spec is reached on purpose rather than
//! hoped for: A covers the zone, contact and equipment, B extraction and the
//! result screen. **P** is the Planner (the NPC policy in seat 0, through the
//! keyboard), **C** the Chaser, **I** the Idle, and **P2** the Planner again,
//! which must replay P exactly.

use std::collections::BTreeMap;
use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord, FrameRecorder, InputScript};

use crate::checks::Checks;
use crate::players::{Chaser, Idle, Planner, Player, Scripted};
use crate::rules::{COURSE_CENTER, Course, Item, atan_to};
use crate::screen::whole;
use crate::sim::{Ball, Golfer, Snapshot, read_snapshot, spawn_pickup};
use crate::{WINDOW, config, register};

/// The seed every run plays — the window's own.
pub const VERIFY_SEED: u64 = 7;
/// The recorder's viewport: the window's, so there is one camera.
pub const HEADLESS_VIEWPORT: PhysicalSize = WINDOW;
/// The longest any match may run before the check calls it stuck.
pub const MATCH_TICKS: u64 = 6200;
/// How long run A lasts.
pub const RUN_A_TICKS: u64 = 800;

/// Something done to the world after a named tick.
pub type Stage = fn(&mut World);

/// What one run did, tick by tick.
pub struct Session {
    /// The world after each tick: `snaps[0]` is after tick 1.
    pub snaps: Vec<Snapshot>,
    /// The status panel's strings after each tick.
    pub panels: Vec<Vec<String>>,
    /// The frames asked for, by tick.
    pub frames: BTreeMap<u64, FrameRecord>,
    /// The last frame drawn while the match was live.
    pub last_live: Option<(u64, FrameRecord)>,
    /// The last frame drawn.
    pub last: Option<(u64, FrameRecord)>,
    /// The tick the match ended on, if it did.
    pub over: Option<u64>,
    pub font: Option<BackendTextureId>,
    pub schedule: String,
}

impl Session {
    /// The world after `tick`.
    pub fn at(&self, tick: u64) -> &Snapshot {
        let index = (tick.max(1) - 1) as usize;
        &self.snaps[index.min(self.snaps.len() - 1)]
    }

    /// The panel after `tick`.
    pub fn panel(&self, tick: u64) -> &[String] {
        let index = (tick.max(1) - 1) as usize;
        &self.panels[index.min(self.panels.len() - 1)]
    }

    /// The last tick played.
    pub fn ticks(&self) -> u64 {
        self.snaps.len() as u64
    }
}

/// How a run is played: the stages, the frames to keep, and when to stop.
pub struct Plan<'a> {
    pub stages: &'a [(u64, Stage)],
    pub keep: &'a [u64],
    pub record: bool,
    /// Stop at the match's end, or play exactly this many ticks.
    pub until_over: bool,
    pub ticks: u64,
}

/// Draws a recorded run, replacing the recorder before its history grows large.
struct Drawer {
    recorder: FrameRecorder,
    drawn: usize,
}

impl Drawer {
    fn draw(&mut self, sim: &mut HeadlessSim) -> FrameRecord {
        if self.drawn >= 200 {
            self.recorder = FrameRecorder::new(HEADLESS_VIEWPORT);
            self.drawn = 0;
        }
        self.drawn += 1;
        self.recorder.draw(sim)
    }
}

/// Play one session with `player` in seat 0.
pub fn play(player: &mut dyn Player, plan: &Plan) -> Session {
    let mut sim = headless(config(), register);
    let mut drawer = plan.record.then(|| Drawer {
        recorder: FrameRecorder::new(HEADLESS_VIEWPORT),
        drawn: 0,
    });
    let mut session = Session {
        snaps: Vec::new(),
        panels: Vec::new(),
        frames: BTreeMap::new(),
        last_live: None,
        last: None,
        over: None,
        font: drawer.as_ref().map(|d| d.recorder.font_texture()),
        schedule: sim.schedule_debug(),
    };
    let mut last_snap: Option<Snapshot> = None;
    for tick in 1..=plan.ticks {
        let input = player.snapshot(tick, last_snap.as_ref());
        sim.world_mut().insert_resource(Input::new(input));
        sim.tick();
        for (at, stage) in plan.stages {
            if *at == tick {
                stage(sim.world_mut());
            }
        }
        let Some(snap) = read_snapshot(&sim.world().view()) else {
            crate::checks::fail(
                "the world had no course after a tick",
                &format!("tick {tick}: Startup should have inserted `Course` on tick 1"),
            );
        };
        session
            .panels
            .push(whole(&snap).all_strings().map(str::to_owned).collect());
        let over = snap.over.is_some();
        if let Some(drawer) = drawer.as_mut() {
            let frame = drawer.draw(&mut sim);
            if plan.keep.contains(&tick) {
                session.frames.insert(tick, frame.clone());
            }
            if !over {
                session.last_live = Some((tick, frame.clone()));
            }
            session.last = Some((tick, frame));
        }
        if over && session.over.is_none() {
            session.over = Some(tick);
        }
        session.snaps.push(snap.clone());
        last_snap = Some(snap);
        if over && plan.until_over {
            break;
        }
    }
    session
}

// --- staging helpers ----------------------------------------------------------

/// Change the golfer in `seat`.
pub fn with_golfer(
    world: &mut World,
    seat: usize,
    change: impl FnOnce(&mut Golfer, &mut Transform),
) {
    for (_, golfer, transform) in world.query_mut::<(&mut Golfer, &mut Transform)>() {
        if golfer.seat == seat {
            change(golfer, transform);
            return;
        }
    }
}

/// Put `owner`'s ball at rest at `at`.
pub fn place_ball(world: &mut World, owner: usize, at: Vec2) {
    for (_, ball, transform) in world.query_mut::<(&mut Ball, &mut Transform)>() {
        if ball.owner == owner {
            ball.roll = None;
            transform.pos = at;
        }
    }
}

fn course(world: &World) -> Course {
    world.resource::<Course>().clone()
}

fn golfer_pos(world: &World, seat: usize) -> Vec2 {
    world
        .query::<(&Golfer, &Transform)>()
        .find(|(_, g, _)| g.seat == seat)
        .map_or(Vec2::ZERO, |(_, _, t)| t.pos)
}

fn ball_pos(world: &World, owner: usize) -> Vec2 {
    world
        .query::<(&Ball, &Transform)>()
        .find(|(_, b, _)| b.owner == owner)
        .map_or(Vec2::ZERO, |(_, _, t)| t.pos)
}

/// Run A, after tick 1: N1 beside the player holding Spikes, a Driver at the
/// player's feet, the player aimed at the course centre.
fn a_after_1(world: &mut World) {
    let me = golfer_pos(world, 0);
    with_golfer(world, 1, |g, t| {
        t.pos = me + Vec2::new(1.0, 0.0);
        g.held = vec![Item::Spikes];
    });
    spawn_pickup(world, me + Vec2::new(0.0, -0.8), Item::Driver);
    let aim = atan_to(ball_pos(world, 0), COURSE_CENTER);
    with_golfer(world, 0, |g, _| g.aim = aim);
}

/// Run A, after tick 185: N1 back at its seat, so it does not club the Driver
/// off the player the moment its stun ends.
fn a_after_185(world: &mut World) {
    let seat = course(world).seats[1];
    with_golfer(world, 1, |_, t| t.pos = seat);
}

/// Run A, after tick 300: N1's ball at rest beside the player, the player
/// aimed along +X, and no NPC able to club during the sledge.
fn a_after_300(world: &mut World) {
    let me = golfer_pos(world, 0);
    place_ball(world, 1, me + Vec2::new(0.6, 0.0));
    with_golfer(world, 0, |g, _| g.aim = Radians::ZERO);
    // Whatever N1 looted since its stun, it holds nothing now: the sledge is
    // asserted at the full 13.5, which a Heavy would halve.
    with_golfer(world, 1, |g, _| g.held.clear());
    for seat in 1..4 {
        with_golfer(world, seat, |g, _| g.cooldown = 10_000);
    }
}

/// Run A, after tick 400: the player's ball at rest outside zone 0.
fn a_after_400(world: &mut World) {
    place_ball(world, 0, Vec2::new(-13.0, 7.0));
}

/// Run A's stages, in order.
pub const RUN_A_STAGES: &[(u64, Stage)] = &[
    (1, a_after_1),
    (185, a_after_185),
    (300, a_after_300),
    (400, a_after_400),
];

/// Run A's script: club at 5, take at 10, two half-charge strikes.
pub fn script_a() -> InputScript {
    InputScript::new()
        .press(Key::F, 5)
        .press(Key::E, 10)
        .hold(Key::Space, 20..50)
        .hold(Key::Space, 310..340)
}

/// Run B, after tick 1: the player and its ball at zone 1's centre, inside
/// every zone through phase 1.
fn b_after_1(world: &mut World) {
    let center = course(world).zones[1].center;
    with_golfer(world, 0, |_, t| t.pos = center);
    place_ball(world, 0, center + Vec2::new(0.3, 0.0));
}

/// Run B, after tick 1930: the player on the open pad with its ball, four
/// points and two items, and a Helmet at its feet.
fn b_after_1930(world: &mut World) {
    let pad = course(world).pad.center;
    with_golfer(world, 0, |g, t| {
        g.points = 4;
        g.held = vec![Item::Heavy, Item::Spikes];
        g.out_ticks = 0;
        g.fate = crate::rules::Fate::Playing;
        t.pos = pad + Vec2::new(-0.3, 0.0);
    });
    place_ball(world, 0, pad + Vec2::new(0.3, 0.0));
    spawn_pickup(world, pad + Vec2::new(-0.3, 0.8), Item::Helmet);
}

/// Run B's stages.
pub const RUN_B_STAGES: &[(u64, Stage)] = &[(1, b_after_1), (1930, b_after_1930)];

/// Run B's script: extract at 1935.
pub fn script_b() -> InputScript {
    InputScript::new().press(Key::X, 1935)
}

/// Every run the gates read.
pub struct Runs {
    pub a: Session,
    pub b: Session,
    pub p: Session,
    pub p2: Session,
    pub c: Session,
    pub i: Session,
    pub planner: Planner,
    pub reports: Vec<String>,
}

/// The `--verify` entry point.
pub fn run() -> ExitCode {
    let mut checks = Checks::default();
    let a = play(
        &mut Scripted(script_a()),
        &Plan {
            stages: RUN_A_STAGES,
            keep: &[2, 49],
            record: true,
            until_over: false,
            ticks: RUN_A_TICKS,
        },
    );
    let b = play(
        &mut Scripted(script_b()),
        &Plan {
            stages: RUN_B_STAGES,
            keep: &[1931],
            record: true,
            until_over: true,
            ticks: MATCH_TICKS,
        },
    );
    let free = |record: bool| Plan {
        stages: &[],
        keep: &[],
        record,
        until_over: true,
        ticks: MATCH_TICKS,
    };
    let mut planner = Planner::default();
    let p = play(&mut planner, &free(true));
    let mut again = Planner::default();
    let p2 = play(&mut again, &free(true));
    let mut chaser = Chaser::default();
    let c = play(&mut chaser, &free(false));
    let mut idle = Idle;
    let i = play(&mut idle, &free(false));
    let reports = vec![planner.report(), chaser.report(), idle.report()];
    let runs = Runs {
        a,
        b,
        p,
        p2,
        c,
        i,
        planner,
        reports,
    };
    let summary = crate::gates::judge(&mut checks, &runs);
    let ticks: u64 = [&runs.a, &runs.b, &runs.p, &runs.p2, &runs.c, &runs.i]
        .iter()
        .map(|s| s.ticks())
        .sum();
    println!("verified brolf_r2 over {ticks} ticks (seed {VERIFY_SEED})");
    for line in summary {
        println!("  {line}");
    }
    println!();
    if let Some(frame) = runs.a.frames.get(&49) {
        println!("run A, tick 49:");
        println!("{}", frame.transcript());
    }
    checks.verdict()
}
