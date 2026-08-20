//! The players inside the check: one that can win, one that plays like a person
//! on their first try, and one that does nothing.
//!
//! Three rather than one, because one controller only ever says whether the game
//! is beatable *by that controller*. `Mode::Rollout` clears the mechanics,
//! `Mode::Idle` proves the game can be lost, and `Mode::Chaser` — a paddle that
//! simply follows the ball — is the only one of the three that can say whether
//! the game is worth playing.
//!
//! Nothing here ticks the simulation. There is no way to fork a running world,
//! so this file rolls the game forward through the game's own `step_ball`,
//! `opponent_target` and `chase` — which is why those are free functions in
//! `main.rs` rather than branches inside the systems that call them.

use jidousha::prelude::*;

use crate::{
    BALL_RADIUS, Ball, MAX_BALL_SPEED, OPPONENT_SPEED, PADDLE_LIMIT, PADDLE_SIZE, PLAYER_SPEED,
    Paddle, Round, SPEEDUP, Side, Stage, Tally, chase, face_of, opponent_target, paddle_home,
    rebound, step_ball,
};

/// How far in from a paddle's tip a planned contact must land, as a fraction of
/// its reach.
///
/// "Take the best available" loses matches: the sharpest return is always the
/// one struck at the very last millimetre, so an unconstrained search resolves
/// every time to standing on the boundary of what is possible, where half a tick
/// of overshoot is a clean miss rather than a worse result. Constrain first,
/// optimise inside what survives.
const EDGE_MARGIN: f32 = 0.25;

/// How many futures a decision looks at.
///
/// Every one of them is a lattice point — a place the paddle can actually stand
/// — so the arithmetic is about a future that can happen.
const CANDIDATES: usize = 13;

/// How many ticks a rollout will run before giving up on the ball arriving.
const ROLLOUT_CAP: u32 = 400;

/// Which player is at the keyboard.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Rolls the game forward and plays the shot that lands furthest from the
    /// opponent's paddle. The one that clears the mechanics.
    Rollout,
    /// Puts the middle of the paddle on the ball. What a person does first try,
    /// and the only one of the three that can say the game is playable.
    Chaser,
    /// Never touches a key. Proves the game can be lost.
    Idle,
}

/// What a controller says about itself, printed every run.
///
/// One number is not the contract: `met 27 of 27` prints happily beside a 0-0
/// match, because reaching a ball and threatening with it are different claims.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Report {
    /// Balls that started travelling towards this paddle.
    pub(crate) approaches: u32,
    /// How many of them it returned.
    pub(crate) met: u32,
    /// Shots planned, and how far from the opponent they were believed to land.
    pub(crate) planned: u32,
    /// The sum of those distances, in world units.
    pub(crate) threat_total: f32,
    /// Shots whose landing was measured against the plan that produced them.
    pub(crate) aimed: u32,
    /// The sum of |planned landing - actual landing|, in world units.
    pub(crate) error_total: f32,
}

impl Report {
    /// How far the shots it chose were believed to land from the opponent.
    pub(crate) fn threat(&self) -> f32 {
        if self.planned == 0 {
            0.0
        } else {
            self.threat_total / self.planned as f32
        }
    }

    /// How far its shots landed from where it planned them to.
    pub(crate) fn aim_error(&self) -> f32 {
        if self.aimed == 0 {
            0.0
        } else {
            self.error_total / self.aimed as f32
        }
    }

    /// The three numbers, as the three lines a verdict prints.
    pub(crate) fn lines(&self, who: &str) -> [String; 3] {
        [
            format!(
                "  {who}: met {} of {} approaches",
                self.met, self.approaches
            ),
            format!(
                "  {who}: planned {} returns aimed to land {:.2} from the opponent",
                self.planned,
                self.threat()
            ),
            format!(
                "  {who}: shots landed {:.2} from where they were planned to",
                self.aim_error()
            ),
        ]
    }
}

/// A shot this controller decided to play.
#[derive(Clone, Copy, Debug)]
struct Plan {
    /// The paddle centre it chose to stand at — a lattice point.
    stand: f32,
    /// Where it believed the return would cross the opponent's plane.
    landing: f32,
    /// How far that was from where the opponent would be standing.
    threat: f32,
}

/// One player, driving the left paddle.
pub(crate) struct Player {
    /// Which of the three this is.
    mode: Mode,
    /// The plan the most recent decision made.
    plan: Option<Plan>,
    /// The plan that was standing when the ball was last struck.
    struck: Option<Plan>,
    /// Whether the ball was coming this way on the previous tick.
    was_approaching: bool,
    /// Whether the current approach has been returned yet.
    pending: bool,
    /// Returns this paddle had made as of the previous tick.
    returns_seen: u32,
    /// What it has to say about itself.
    report: Report,
}

impl Player {
    /// A player of the given kind, having done nothing yet.
    pub(crate) fn new(mode: Mode) -> Player {
        Player {
            mode,
            plan: None,
            struck: None,
            was_approaching: false,
            pending: false,
            returns_seen: 0,
            report: Report::default(),
        }
    }

    /// The three numbers.
    pub(crate) fn report(&self) -> Report {
        self.report
    }

    /// Which key this player wants held this tick, having looked at the world.
    ///
    /// `None` on the way into tick 1, where `Startup` has not run and there is
    /// nothing at all to look at.
    pub(crate) fn decide(&mut self, world: &World) -> Option<Key> {
        if self.mode == Mode::Idle {
            return None;
        }
        // On the way into tick 1 there is nothing to look at: `Startup` runs
        // inside that first `tick()`, so this read happens once against an
        // empty world and has to answer rather than index into it.
        let view = View::of(world)?;
        self.book_keeping(&view);
        if view.stage != Stage::Rally {
            return None;
        }
        let target = match self.mode {
            Mode::Idle => return None,
            Mode::Chaser => view.ball_pos.y,
            Mode::Rollout => self.plan_a_shot(&view),
        };
        steer(view.mine, target, view.dt)
    }

    /// Count approaches and returns, and measure the last shot against its plan.
    fn book_keeping(&mut self, view: &View) {
        let approaching = view.stage == Stage::Rally && view.ball_vel.x < 0.0;
        if approaching && !self.was_approaching {
            self.report.approaches += 1;
            self.pending = true;
            self.struck = None;
        }
        // A point ended without the ball coming back: the approach was missed.
        if self.pending && view.stage != Stage::Rally {
            self.pending = false;
        }
        self.was_approaching = approaching;

        let returns = view.returns;
        if returns > self.returns_seen {
            self.returns_seen = returns;
            if self.pending {
                self.report.met += 1;
                self.pending = false;
            }
            // The ball has just left the paddle, so this tick's velocity is the
            // shot that was actually produced. Roll it out and compare it with
            // the plan that was standing when it was struck — the number nobody
            // writes unprompted, and the one that says whether a controller's
            // aims are worth anything.
            if let Some(plan) = self.struck.take() {
                let landed =
                    land_against_opponent(view.ball_pos, view.ball_vel, view.theirs, view.dt);
                if let Some((landing, _)) = landed {
                    self.report.aimed += 1;
                    self.report.error_total += (landing - plan.landing).abs();
                }
            }
        }
    }

    /// Choose where to stand, and remember why.
    fn plan_a_shot(&mut self, view: &View) -> f32 {
        // A ball going the other way is not a shot to plan. Sit where the next
        // one is most likely to need us, which is the middle.
        if view.ball_vel.x >= 0.0 {
            self.plan = None;
            return 0.0;
        }
        let plane = face_of(Vec2::new(paddle_home(Side::Left), view.mine), Side::Left).plane;
        let Some(arrival) = roll_to_plane(view.ball_pos, view.ball_vel, plane, -1.0, view.dt)
        else {
            self.plan = None;
            return view.ball_pos.y;
        };
        let reach = PADDLE_SIZE.y * 0.5 + BALL_RADIUS;
        let step = PLAYER_SPEED * view.dt;
        // Where the paddle can actually stand when the ball gets here: its own
        // position plus a whole number of steps. A candidate off this lattice is
        // a place it cannot be, so an objective computed about it is a number
        // about a future that will not happen.
        let reachable = arrival.ticks as i32;
        let mut best: Option<Plan> = None;
        for index in 0..CANDIDATES {
            let spread = (index as f32 / (CANDIDATES - 1) as f32) * 2.0 - 1.0;
            let steps = (spread * reachable as f32).round();
            let stand = (view.mine + steps * step).clamp(-PADDLE_LIMIT, PADDLE_LIMIT);
            // Constrain: it has to make contact, with the tip off the menu.
            let offset = (arrival.pos.y - stand) / reach;
            if offset.abs() > 1.0 - EDGE_MARGIN {
                continue;
            }
            // Optimise: run the opponent's own rule forward beside the ball's,
            // and score the landing against where that puts it. Against an
            // opponent that chases, "furthest from the middle" is close to the
            // worst objective available.
            let face = face_of(Vec2::new(paddle_home(Side::Left), stand), Side::Left);
            let speed = (arrival.vel.length() * SPEEDUP).min(MAX_BALL_SPEED);
            let away = rebound(arrival.pos.y, face, speed);
            let Some((landing, foe)) =
                land_against_opponent(arrival.pos, away, view.theirs, view.dt)
            else {
                continue;
            };
            let threat = (landing - foe).abs();
            if best.is_none_or(|other: Plan| threat > other.threat) {
                best = Some(Plan {
                    stand,
                    landing,
                    threat,
                });
            }
        }
        match best {
            Some(plan) => {
                self.report.planned += 1;
                self.report.threat_total += plan.threat;
                self.plan = Some(plan);
                self.struck = Some(plan);
                plan.stand
            }
            // Nothing survived both constraints: run at the ball.
            None => {
                self.plan = None;
                self.struck = None;
                arrival.pos.y
            }
        }
    }
}

/// Everything a decision reads, pulled out of the world once.
struct View {
    /// Where the ball is.
    ball_pos: Vec2,
    /// How it is travelling.
    ball_vel: Vec2,
    /// Our paddle's centre.
    mine: f32,
    /// The opponent's paddle's centre.
    theirs: f32,
    /// What the match is doing.
    stage: Stage,
    /// Returns our paddle has made this match.
    returns: u32,
    /// One tick, in seconds.
    dt: f32,
}

impl View {
    /// The world as a decision sees it, or `None` before `Startup` has run.
    fn of(world: &World) -> Option<View> {
        let round = world.find_resource::<Round>()?;
        let tally = world.find_resource::<Tally>()?;
        let (_, ball_transform, ball) = world.query::<(&Transform, &Ball)>().next()?;
        let mut mine = None;
        let mut theirs = None;
        for (_, transform, paddle) in world.query::<(&Transform, &Paddle)>() {
            match paddle.side {
                Side::Left => mine = Some(transform.pos.y),
                Side::Right => theirs = Some(transform.pos.y),
            }
        }
        Some(View {
            ball_pos: ball_transform.pos,
            ball_vel: ball.vel,
            mine: mine?,
            theirs: theirs?,
            stage: round.stage,
            returns: tally.returns[Side::Left.index()],
            dt: world.resource::<Time>().fixed_dt.as_f32(),
        })
    }
}

/// Where a rolled-forward ball reached a plane, and when.
struct Arrival {
    /// Ticks from now.
    ticks: u32,
    /// Where it was.
    pos: Vec2,
    /// How it was travelling.
    vel: Vec2,
}

/// Roll the ball forward with nothing in its way until it crosses `plane`.
///
/// Nothing in its way is right: a ball travelling towards one paddle cannot
/// reach the other, and this game's only other collider is the walls, which
/// `step_ball` handles on its own.
fn roll_to_plane(pos: Vec2, vel: Vec2, plane: f32, approach: f32, dt: f32) -> Option<Arrival> {
    let (mut pos, mut vel) = (pos, vel);
    for tick in 1..=ROLLOUT_CAP {
        let step = step_ball(pos, vel, &[], dt);
        let crossed = (plane - step.pos.x) * approach <= 0.0;
        if crossed {
            // Where along this tick's travel it actually crossed, so the aim is
            // about the contact point rather than about the tick after it.
            let span = step.pos.x - pos.x;
            let at = if span.abs() > f32::EPSILON {
                ((plane - pos.x) / span).clamp(0.0, 1.0)
            } else {
                1.0
            };
            return Some(Arrival {
                ticks: tick,
                pos: pos.lerp(step.pos, at),
                vel: step.vel,
            });
        }
        pos = step.pos;
        vel = step.vel;
    }
    None
}

/// Where a shot crosses the opponent's plane, and where the opponent will be.
///
/// The opponent's own rule, run forward beside the ball's, one tick at a time
/// and in the order the game runs them: the paddle moves, then the ball does.
fn land_against_opponent(pos: Vec2, vel: Vec2, foe: f32, dt: f32) -> Option<(f32, f32)> {
    let (mut pos, mut vel, mut foe) = (pos, vel, foe);
    let plane = face_of(Vec2::new(paddle_home(Side::Right), foe), Side::Right).plane;
    for _ in 0..ROLLOUT_CAP {
        foe = chase(
            foe,
            opponent_target(pos, vel, Side::Right),
            OPPONENT_SPEED,
            dt,
        );
        let step = step_ball(pos, vel, &[], dt);
        if (plane - step.pos.x) * 1.0 <= 0.0 {
            let span = step.pos.x - pos.x;
            let at = if span.abs() > f32::EPSILON {
                ((plane - pos.x) / span).clamp(0.0, 1.0)
            } else {
                1.0
            };
            return Some((pos.lerp(step.pos, at).y, foe));
        }
        pos = step.pos;
        vel = step.vel;
    }
    None
}

/// Which key moves the paddle from `at` towards `target`, or none if it is close
/// enough that a whole step would overshoot.
///
/// Stopping inside half a step is what keeps the paddle on the lattice the
/// candidates above were chosen from.
fn steer(at: f32, target: f32, dt: f32) -> Option<Key> {
    let step = PLAYER_SPEED * dt;
    let gap = target - at;
    if gap.abs() < step * 0.5 {
        return None;
    }
    // Y is down, so S is the larger number.
    if gap > 0.0 {
        Some(Key::S)
    } else {
        Some(Key::W)
    }
}
