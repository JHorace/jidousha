//! The three players the `--verify` run drives the left paddle with.
//!
//! `docs/api/jidousha-controllers.md` is the argument; this is it applied to a
//! paddle. Three players, because one cannot measure a game's difficulty:
//!
//! - [`Style::Rollout`] plans its shot and should win. It clears the mechanics.
//! - [`Style::Chaser`] steers at where the ball *is*, which is what a person
//!   does on their first try. **Only this one can say the game is playable**: a
//!   game the rollout wins and the chaser cannot score in at all is a game whose
//!   rallies have nowhere to go.
//! - [`Style::Idle`] presses nothing, and proves the game can be lost.
//!
//! Each reports the document's three numbers about *itself*, so "suspect the
//! controller first" is a suspicion one run settles.

use jidousha::prelude::*;
use jidousha::testing::{InputEvent, InputSnapshot, SnapshotBuilder};

use crate::rules::{
    self, BALL_SPEED_GAIN, BALL_SPEED_MAX, CONTACT_REACH, OPPONENT_REACTION_X, OPPONENT_SPEED,
    PADDLE_Y_LIMIT, Side,
};
use crate::{Ball, Paddle, Round, Screen};

/// How much of the paddle's reach is off the menu at either tip.
///
/// The optimum is always struck at the very tip, and the tip is the boundary of
/// the feasible set: half a tick of overshoot there is a clean miss rather than
/// a worse result. So the tip is not a candidate.
const EDGE_MARGIN: f32 = 0.25;

/// How many lattice steps either side of the paddle's current position are ever
/// worth enumerating — a bound on the search, not on the paddle.
const MAX_STEPS: i32 = 240;

/// Which of the three players this is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Style {
    /// Plans the shot: predicts, constrains, then optimises on the lattice.
    Rollout,
    /// Steers at the ball's current height. The first thing a person does.
    Chaser,
    /// Presses nothing.
    Idle,
}

impl Style {
    /// The name this player is reported under.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Style::Rollout => "rollout",
            Style::Chaser => "chaser",
            Style::Idle => "idle",
        }
    }
}

/// The three numbers a controller has to report about itself.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Report {
    /// How many times the ball came at this paddle.
    pub(crate) approaches: u32,
    /// How many of those it actually touched.
    pub(crate) met: u32,
    /// Total of every planned shot's distance from where the opponent would be.
    planned_total: f32,
    /// How many shots that total covers.
    planned_count: u32,
    /// Total of every |planned landing - actual landing|.
    aim_total: f32,
    /// How many shots that total covers.
    aim_count: u32,
}

impl Report {
    /// How threatening the shots it *chose* were believed to be. Clears its
    /// objective.
    pub(crate) fn planned_threat(&self) -> f32 {
        if self.planned_count == 0 {
            return 0.0;
        }
        self.planned_total / self.planned_count as f32
    }

    /// Whether the shots it plans are the shots it produces. Clears its aim, and
    /// it is the one nobody writes unprompted.
    pub(crate) fn aim_error(&self) -> f32 {
        if self.aim_count == 0 {
            return 0.0;
        }
        self.aim_total / self.aim_count as f32
    }

    /// The three numbers, on one line each, as the document asks.
    pub(crate) fn lines(&self, style: Style) -> String {
        format!(
            "    {}: met {} of {} approaches; planned returns aimed to land {:.2} from the \
             opponent; shots landed {:.2} from where they were planned to",
            style.name(),
            self.met,
            self.approaches,
            self.planned_threat(),
            self.aim_error(),
        )
    }
}

/// Everything one decision reads out of the world, in one place.
///
/// A struct rather than seven parameters because clippy counts them, and because
/// the read and the decision are then two things rather than one long signature.
struct Approach {
    /// Where the ball is now.
    ball_at: Vec2,
    /// Where it is going, in world units per second.
    velocity: Vec2,
    /// Where this controller's paddle is.
    current: f32,
    /// Where the opponent's paddle is.
    opponent_y: f32,
    /// How far a paddle moves in one tick.
    step: f32,
    /// How long one tick is, in seconds.
    dt: f32,
    /// How fast the ball is travelling now.
    ball_speed: f32,
}

/// What the controller decided to do about one approach.
#[derive(Clone, Copy, Debug)]
struct Plan {
    /// Where the shot was meant to land, at the opponent's face.
    landing: f32,
    /// How far that was from where the opponent was expected to be.
    threat: f32,
    /// The lattice position the paddle has to stand on to produce it.
    stand: f32,
}

/// One player, and the keyboard it presses.
pub(crate) struct Controller {
    style: Style,
    keyboard: SnapshotBuilder,
    holding: Option<Key>,
    plan: Option<Plan>,
    /// Whether the ball was coming at us on the previous tick.
    incoming: bool,
    report: Report,
}

impl Controller {
    /// A player of this style, with nothing held and nothing measured yet.
    pub(crate) fn new(style: Style) -> Controller {
        Controller {
            style,
            keyboard: SnapshotBuilder::new(),
            holding: None,
            plan: None,
            incoming: false,
            report: Report::default(),
        }
    }

    /// What it has measured about itself.
    pub(crate) fn report(&self) -> Report {
        self.report
    }

    /// Look at the world and produce this tick's input.
    ///
    /// On the way into tick 1 there is nothing to look at — `Startup` runs
    /// *inside* that tick — so every read here is a `find_resource` or a query
    /// that may yield nothing, and the answer in that case is "press nothing".
    pub(crate) fn decide(&mut self, sim: &HeadlessSim) -> InputSnapshot {
        let want = match self.style {
            Style::Idle => None,
            _ => self.aim(sim).and_then(|(target, current, step)| {
                // Stop inside half a step: a paddle cannot stand between two
                // lattice points, so steering for a finer position than that is
                // steering for a place it cannot be.
                let error = target - current;
                if error.abs() < step * 0.5 {
                    None
                } else if error > 0.0 {
                    Some(Key::S)
                } else {
                    Some(Key::W)
                }
            }),
        };

        if want != self.holding {
            // Events, not states: that is what makes a key held for a hundred
            // ticks press exactly once.
            if let Some(old) = self.holding {
                self.keyboard.record(InputEvent::KeyReleased(old));
            }
            if let Some(new) = want {
                self.keyboard.record(InputEvent::KeyPressed(new));
            }
            self.holding = want;
        }
        self.keyboard.first_tick_snapshot()
    }

    /// Where this player wants its paddle, its paddle's current y, and one step.
    fn aim(&mut self, sim: &HeadlessSim) -> Option<(f32, f32, f32)> {
        let world = sim.world();
        let round = world.find_resource::<Round>()?;
        let (_, ball_transform, ball) = world.query::<(&Transform, &Ball)>().next()?;
        let (ball_at, velocity) = (ball_transform.pos, ball.velocity);
        let mut mine = None;
        let mut theirs = None;
        for (_, transform, paddle) in world.query::<(&Transform, &Paddle)>() {
            match paddle.side {
                Side::Left => mine = Some((transform.pos.y, paddle.speed)),
                Side::Right => theirs = Some(transform.pos.y),
            }
        }
        let ((current, speed), opponent_y) = (mine?, theirs?);
        let dt = world.resource::<Time>().fixed_dt.as_f32();
        let step = speed * dt;

        if round.screen != Screen::Rally {
            return Some((0.0, current, step));
        }
        let target = match self.style {
            Style::Idle => current,
            // What a person does on their first try, and the whole reason this
            // player exists: it returns the ball dead flat, so if the game is
            // only a game for a planner it will say so here.
            Style::Chaser => ball_at.y.clamp(-PADDLE_Y_LIMIT, PADDLE_Y_LIMIT),
            Style::Rollout => self.plan_the_shot(&Approach {
                ball_at,
                velocity,
                current,
                opponent_y,
                step,
                dt,
                ball_speed: ball.speed,
            }),
        };
        Some((target, current, step))
    }

    /// Predict, constrain, optimise — in that order, on the lattice.
    fn plan_the_shot(&mut self, look: &Approach) -> f32 {
        let &Approach {
            ball_at,
            velocity,
            current,
            opponent_y,
            step,
            dt,
            ball_speed,
        } = look;
        // Not coming: stand in the middle, which is the best place to be when
        // you do not know where you will be needed.
        if velocity.x >= 0.0 {
            return 0.0;
        }
        let plane = Side::Left.contact_x();
        let Some(crossing) = rules::predict_crossing(ball_at, velocity, plane) else {
            return 0.0;
        };
        // How many whole steps there is time for before the ball arrives. This
        // is the "can be reached" constraint, and it is the bound on the search
        // rather than a filter inside it.
        let travel = (plane - ball_at.x) / velocity.x;
        let steps = ((travel / dt) as i32).clamp(0, MAX_STEPS);

        let outgoing = (ball_speed + BALL_SPEED_GAIN).min(BALL_SPEED_MAX);
        let usable = CONTACT_REACH * (1.0 - EDGE_MARGIN);
        let from = Vec2::new(plane, crossing);

        let mut best: Option<(f32, Plan)> = None;
        for index in -steps..=steps {
            // The positions the paddle can actually stand on: a key moves it a
            // whole step at a time, so anything off this lattice is a place it
            // cannot be, and a plan about it is a plan about a future that will
            // not happen.
            let candidate = (current + index as f32 * step).clamp(-PADDLE_Y_LIMIT, PADDLE_Y_LIMIT);
            // Constrain first: contact has to be real, with margin, so the
            // paddle's tip — where every optimum sits and where half a step of
            // overshoot is a clean miss — is not on the menu.
            if (crossing - candidate).abs() > usable {
                continue;
            }
            let out = rules::rebound(crossing, candidate, Side::Left, outgoing);
            let Some(landing) = rules::predict_crossing(from, out, Side::Right.contact_x()) else {
                continue;
            };
            // Run the opponent's own rule forward beside the ball's and score
            // the landing against where that puts it. This opponent predicts
            // rather than chases, so "the return landing furthest from the
            // middle" is the wrong objective and there is no reduction of the
            // principle that fits it.
            let reached = opponent_reach(from, out, opponent_y, candidate);
            let threat = (landing - reached).abs();
            if best.is_none_or(|(score, _)| threat > score) {
                best = Some((
                    threat,
                    Plan {
                        landing,
                        threat,
                        stand: candidate,
                    },
                ));
            }
        }

        match best {
            Some((_, plan)) => {
                self.plan = Some(plan);
                plan.stand
            }
            // Nothing survived both constraints — the ball is going somewhere
            // this paddle cannot be. Run at it anyway; a miss by a little is
            // better than a miss by a lot, and the ball may yet be reachable
            // after the next wall bounce changes the arithmetic.
            None => {
                self.plan = None;
                crossing.clamp(-PADDLE_Y_LIMIT, PADDLE_Y_LIMIT)
            }
        }
    }

    /// After the tick: count the approach, and measure the shot against its plan.
    pub(crate) fn observe(&mut self, sim: &HeadlessSim) {
        let world = sim.world();
        let Some((_, transform, ball)) = world.query::<(&Transform, &Ball)>().next() else {
            return;
        };
        let (at, velocity) = (transform.pos, ball.velocity);
        let coming = velocity.x < 0.0;
        if coming && !self.incoming {
            self.report.approaches += 1;
        }
        // The tick the ball turned around next to our paddle is the tick we met
        // it. Nothing else can turn it around on this side of the court.
        if self.incoming && velocity.x > 0.0 && at.x < 0.0 {
            self.report.met += 1;
            if let Some(plan) = self.plan.take() {
                self.report.planned_total += plan.threat;
                self.report.planned_count += 1;
                if let Some(actual) = rules::predict_crossing(at, velocity, Side::Right.contact_x())
                {
                    self.report.aim_total += (actual - plan.landing).abs();
                    self.report.aim_count += 1;
                }
            }
        }
        self.incoming = coming;
    }
}

/// Where the opponent will have got to by the time the ball reaches its face.
///
/// A forward model of the game's own rule rather than a copy of it: it calls
/// [`rules::opponent_target`], so an opponent that changes its mind changes this
/// with it. Two legs, because the rule has two: drifting to the middle until the
/// ball passes [`OPPONENT_REACTION_X`], then closing on its chosen contact point.
fn opponent_reach(from: Vec2, velocity: Vec2, opponent_y: f32, player_y: f32) -> f32 {
    let face = Side::Right.contact_x();
    let to_face = (face - from.x) / velocity.x;
    // Named rather than written `if !(to_face > 0.0)`: with only one condition
    // there is no conjunction to hang the `!` on, and clippy rejects a negated
    // comparison. NaN fails `> 0.0`, so it lands here as "not coming".
    let coming = to_face > 0.0;
    if !coming {
        return opponent_y;
    }
    let to_gate = ((OPPONENT_REACTION_X - from.x) / velocity.x).clamp(0.0, to_face);

    // A point on the same trajectory at the moment the opponent starts looking.
    // Only `x` matters to the rule's gate, and `predict_crossing` gives the same
    // answer from every point on one trajectory — the fold happens once, at the
    // far end — so this carries the algebra without needing the ball's real
    // bounced height at that instant.
    let at_gate = Vec2::new(
        OPPONENT_REACTION_X.max(from.x),
        from.y + velocity.y * to_gate,
    );
    let drifted = move_towards(opponent_y, 0.0, OPPONENT_SPEED * to_gate);
    let target = rules::opponent_target(at_gate, velocity, player_y);
    move_towards(drifted, target, OPPONENT_SPEED * (to_face - to_gate))
        .clamp(-PADDLE_Y_LIMIT, PADDLE_Y_LIMIT)
}

/// Step `from` at most `limit` towards `to`, stopping exactly on it.
///
/// The scalar `Vec2::move_towards`, which glam has and a scalar does not.
fn move_towards(from: f32, to: f32, limit: f32) -> f32 {
    let delta = to - from;
    if delta.abs() <= limit {
        return to;
    }
    from + delta.signum() * limit
}
