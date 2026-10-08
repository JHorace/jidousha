//! The players the verify run plays with: a Planner that plays to win, a
//! Chaser that golfs and nothing else, an Idle that stands there, and the
//! scripted players of the staged runs.
//!
//! The Planner and the Chaser decide an `Intent` and press keys for it through
//! one `SnapshotBuilder` — events, not states — so they meet every rule the
//! keyboard does, the aim's 2.5-degree step included. Only the Planner's
//! policy is the NPCs' own (`npc::plan` for seat 0); the asymmetry between
//! its instant aim and the arrows is what its third number measures.

use jidousha::prelude::*;
use jidousha::testing::{InputEvent, InputScript, InputSnapshot, SnapshotBuilder};

use crate::npc;
use crate::rules::{Fate, atan_to, charge_for, course, effects, landing_of, polar, shot_speed_for};
use crate::sim::{Aim, Intent, Snapshot};

/// The aim's dead band: half of one 2.5-degree step.
const DEAD_BAND_DEGREES: f32 = 1.25;

/// Anything that can stand in seat 0.
pub trait Player {
    /// The input for the next tick, having seen the world after the last one
    /// (`None` before tick 1, when Startup has not run).
    fn snapshot(&mut self, tick: u64, snap: Option<&Snapshot>) -> InputSnapshot;
    /// What the player learned about itself, one line.
    fn report(&self) -> String;
}

/// A keyboard driven by intents: holds what is wanted, taps F, E and X.
struct Keys {
    builder: SnapshotBuilder,
    held: Vec<Key>,
    tapped: Vec<Key>,
}

impl Default for Keys {
    fn default() -> Self {
        Keys {
            builder: SnapshotBuilder::new(),
            held: Vec::new(),
            tapped: Vec::new(),
        }
    }
}

impl Keys {
    fn set(&mut self, key: Key, want: bool) {
        let holding = self.held.contains(&key);
        if want && !holding {
            self.builder.record(InputEvent::KeyPressed(key));
            self.held.push(key);
        } else if !want && holding {
            self.builder.record(InputEvent::KeyReleased(key));
            self.held.retain(|k| *k != key);
        }
    }

    fn tap(&mut self, key: Key) {
        self.builder.record(InputEvent::KeyPressed(key));
        self.tapped.push(key);
    }

    /// Press the keys for `intent`, given the golfer's current aim. Returns
    /// whether Space is held after this tick's keys.
    fn press(&mut self, intent: &Intent, aim_now: Radians) -> bool {
        for key in std::mem::take(&mut self.tapped) {
            self.builder.record(InputEvent::KeyReleased(key));
        }
        let walk = intent.walk;
        self.set(Key::W, walk.y < -0.3);
        self.set(Key::S, walk.y > 0.3);
        self.set(Key::A, walk.x < -0.3);
        self.set(Key::D, walk.x > 0.3);
        let off = match intent.aim {
            Aim::Set(want) => signed_degrees(want, aim_now),
            Aim::Keep | Aim::Rotate(_) => 0.0,
        };
        self.set(Key::ArrowRight, off > DEAD_BAND_DEGREES);
        self.set(Key::ArrowLeft, off < -DEAD_BAND_DEGREES);
        let settled = off.abs() <= DEAD_BAND_DEGREES;
        let space = intent.space_held && settled;
        self.set(Key::Space, space);
        if intent.club {
            self.tap(Key::F);
        }
        if intent.take {
            self.tap(Key::E);
        }
        if intent.extract {
            self.tap(Key::X);
        }
        space
    }

    fn snapshot(&mut self) -> InputSnapshot {
        self.builder.first_tick_snapshot()
    }
}

/// `want - now`, in degrees, the short way round.
pub fn signed_degrees(want: Radians, now: Radians) -> f32 {
    (want.to_degrees() - now.to_degrees() + 540.0).rem_euclid(360.0) - 180.0
}

/// A strike the controller released, followed until the ball stops.
#[derive(Clone, Copy)]
struct Shot {
    target: Vec2,
    points: u32,
    planned: Vec2,
    released_at: u64,
    met: bool,
}

/// The three numbers, and the bookkeeping behind them.
#[derive(Default)]
struct Tally {
    intended: u32,
    met: u32,
    aimed_off: f32,
    landed_off: f32,
    landed: u32,
    holed: u32,
    pending: Option<Shot>,
    spacing: bool,
}

impl Tally {
    /// Follow last tick's release, then note whether this tick releases.
    fn follow(&mut self, tick: u64, snap: &Snapshot, plan: Option<&npc::Plan>, space: bool) {
        let ball = snap.ball(0);
        if let Some(mut shot) = self.pending {
            if !shot.met && tick == shot.released_at + 1 {
                // `act` struck on the release tick, so the ball rolls now —
                // unless a roll that short has already stopped.
                shot.met = !ball.at_rest || ball.pos.distance(shot.planned) < 0.6;
                if shot.met {
                    self.met += 1;
                    self.aimed_off += shot.planned.distance(shot.target);
                }
            }
            if shot.met && ball.at_rest {
                // A holed ball is lifted out beside the cup, so where it lies
                // says nothing about the aim.
                if snap.golfer(0).points > shot.points {
                    self.holed += 1;
                } else {
                    self.landed_off += ball.pos.distance(shot.planned);
                    self.landed += 1;
                }
                self.pending = None;
            } else if !shot.met {
                self.pending = None;
            } else {
                self.pending = Some(shot);
            }
        }
        let golfer = snap.golfer(0);
        if self.spacing && !space && golfer.charge > 0 && golfer.fate == Fate::Playing {
            self.intended += 1;
            if let Some(plan) = plan
                && let (Some(target), Aim::Set(aim)) = (plan.target, plan.intent.aim)
            {
                // Planned at the intended angle and the charge actually held,
                // so the gap to where it stops is the aim's dead band alone.
                let mine = effects(&golfer.held);
                let speed = shot_speed_for(&mine, &mine, true, golfer.charge);
                let landing = landing_of(ball.pos, polar(1.0, aim), speed, course(), snap.dt);
                self.pending = Some(Shot {
                    target,
                    points: golfer.points,
                    planned: landing.at,
                    released_at: tick,
                    met: false,
                });
            }
        }
        self.spacing = space;
    }

    fn line(&self) -> String {
        let mean = |sum: f32, n: u32| if n == 0 { 0.0 } else { sum / n as f32 };
        format!(
            "met {} of {} strikes, planned landings {:.2} from target, landed {:.2} from planned ({} holed)",
            self.met,
            self.intended,
            mean(self.aimed_off, self.met),
            mean(self.landed_off, self.landed),
            self.holed,
        )
    }
}

/// The NPC policy in seat 0, through the keyboard: the player that plays to win.
#[derive(Default)]
pub struct Planner {
    keys: Keys,
    tally: Tally,
}

impl Planner {
    /// Strikes it meant to release, and how many of them found a ball.
    pub fn met(&self) -> (u32, u32) {
        (self.tally.met, self.tally.intended)
    }

    /// The mean distance a ball stopped from where its strike was planned.
    pub fn landed_off(&self) -> f32 {
        if self.tally.landed == 0 {
            0.0
        } else {
            self.tally.landed_off / self.tally.landed as f32
        }
    }
}

impl Player for Planner {
    fn snapshot(&mut self, tick: u64, snap: Option<&Snapshot>) -> InputSnapshot {
        if let Some(snap) = snap {
            let plan = npc::plan(snap, 0);
            let space = self.keys.press(&plan.intent, snap.golfer(0).aim);
            self.tally.follow(tick, snap, Some(&plan), space);
        }
        self.keys.snapshot()
    }

    fn report(&self) -> String {
        format!("planner: {}", self.tally.line())
    }
}

/// The first-try player: walks to its ball and putts at the nearest cup,
/// never clubs, takes or extracts, and ignores the zone.
#[derive(Default)]
pub struct Chaser {
    keys: Keys,
    tally: Tally,
}

/// The Chaser's whole policy.
pub fn chase(snap: &Snapshot) -> (Intent, Option<npc::Plan>) {
    let me = snap.golfer(0);
    let ball = snap.ball(0);
    if me.fate != Fate::Playing {
        return (Intent::IDLE, None);
    }
    let walk_to = |to: Vec2| Intent {
        walk: (to - me.pos).normalize_or_zero(),
        ..Intent::IDLE
    };
    if !ball.at_rest {
        return (walk_to(ball.landing.map_or(ball.pos, |l| l.at)), None);
    }
    if snap.reach(0).ball != Some(0) {
        return (walk_to(ball.pos), None);
    }
    let Some(cup) = snap
        .course
        .cups
        .iter()
        .copied()
        .min_by(|a, b| a.distance(ball.pos).total_cmp(&b.distance(ball.pos)))
    else {
        return (Intent::IDLE, None);
    };
    let aim = atan_to(ball.pos, cup);
    let want = charge_for(ball.pos.distance(cup), 1.0, snap.dt);
    let intent = Intent {
        aim: Aim::Set(aim),
        space_held: me.charge < want,
        ..Intent::IDLE
    };
    let speed = crate::rules::shot_speed(want);
    let landing = crate::rules::landing_of(
        ball.pos,
        crate::rules::polar(1.0, aim),
        speed,
        crate::rules::course(),
        snap.dt,
    );
    let plan = npc::Plan {
        intent,
        target: Some(cup),
        landing: Some(landing),
        want,
    };
    (intent, Some(plan))
}

impl Player for Chaser {
    fn snapshot(&mut self, tick: u64, snap: Option<&Snapshot>) -> InputSnapshot {
        if let Some(snap) = snap {
            let (intent, plan) = chase(snap);
            let space = self.keys.press(&intent, snap.golfer(0).aim);
            self.tally.follow(tick, snap, plan.as_ref(), space);
        }
        self.keys.snapshot()
    }

    fn report(&self) -> String {
        format!("chaser: {}", self.tally.line())
    }
}

/// The player who is there and does nothing.
pub struct Idle;

impl Player for Idle {
    fn snapshot(&mut self, _tick: u64, _snap: Option<&Snapshot>) -> InputSnapshot {
        InputSnapshot::new()
    }

    fn report(&self) -> String {
        "idle: pressed nothing".to_owned()
    }
}

/// A blind script, for the staged runs.
pub struct Scripted(pub InputScript);

impl Player for Scripted {
    fn snapshot(&mut self, tick: u64, _snap: Option<&Snapshot>) -> InputSnapshot {
        self.0.snapshot_at(tick)
    }

    fn report(&self) -> String {
        "scripted".to_owned()
    }
}
