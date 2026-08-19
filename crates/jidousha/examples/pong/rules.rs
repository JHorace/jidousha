//! The arithmetic Pong is made of, as functions of their arguments and nothing
//! else.
//!
//! Everything here is pure: no world, no resources, no time. That is not
//! tidiness for its own sake — it is what lets `verify.rs` ask these functions
//! their contracts directly instead of hoping a played match reaches the case,
//! and what lets the scripted player push candidate shots through *the game's
//! own* bounce rather than through a second copy of it that can drift.
//!
//! The layout constants live here too, for the same reason: the check asserts
//! against them rather than against its own copy of the numbers.

use jidousha::prelude::*;

/// Half the playfield, in world units: the walls are at `+/-y`, the goal lines
/// at `+/-x`.
pub const FIELD_HALF: Vec2 = Vec2::new(16.0, 9.0);

/// How big a paddle is, in world units.
pub const PADDLE_SIZE: Vec2 = Vec2::new(0.8, 3.6);

/// How far from the centre a paddle stands, in world units.
pub const PADDLE_X: f32 = 14.0;

/// The ball's radius, in world units.
pub const BALL_RADIUS: f32 = 0.42;

/// How far a paddle's centre may travel from the middle before it is against a
/// wall.
pub const PADDLE_LIMIT: f32 = FIELD_HALF.y - PADDLE_SIZE.y * 0.5;

/// How far the ball's centre may travel from the middle before it is against a
/// wall.
pub const BALL_LIMIT: f32 = FIELD_HALF.y - BALL_RADIUS;

/// The steepest a paddle can send the ball, measured from the horizontal.
///
/// Struck at the very tip; struck dead centre the return is flat. This is the
/// whole of Pong's aiming model and the reason the game has any depth at all.
pub const MAX_BOUNCE: Radians = Radians(core::f32::consts::FRAC_PI_3);

/// How fast the ball leaves a serve, in world units per second.
pub const SERVE_SPEED: f32 = 26.0;

/// How much faster the ball gets with every paddle touch.
pub const SPEEDUP: f32 = 1.8;

/// The fastest the ball is ever allowed to go, in world units per second.
///
/// Chosen against the paddle's thickness: see `verify.rs`, which asserts the
/// margin against the `fixed_dt` the engine actually hands the game rather than
/// against the 1/60 this number was picked with. The swept contact below means
/// the game is correct even without the margin; the margin means the game is
/// also correct if `sweep_contact` is ever wrong.
pub const MAX_BALL_SPEED: f32 = 40.0;

/// How fast the player's paddle travels, in world units per second.
pub const PLAYER_SPEED: f32 = 26.0;

/// How fast the opponent's paddle travels, in world units per second.
///
/// Below the ball's steepest vertical speed on purpose: at `MAX_BOUNCE` a ball
/// at `SERVE_SPEED` is already climbing at 14.7 u/s, so a steep return outruns
/// the opponent vertically even though the opponent could out-travel it given
/// the time. That is the only way to score against something that never stops
/// chasing, and it is what makes aiming the point of the game.
pub const OPPONENT_SPEED: f32 = 15.0;

/// How close to the ball the opponent stops steering, in world units.
///
/// Without it the paddle chatters either side of the ball every tick, which
/// looks like a fault and costs it the reach it would otherwise have.
pub const OPPONENT_DEAD_ZONE: f32 = 0.25;

/// Which end of the court something belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    /// Where this side's paddle stands, in world units.
    pub fn paddle_x(self) -> f32 {
        match self {
            Side::Left => -PADDLE_X,
            Side::Right => PADDLE_X,
        }
    }

    /// Which way this side hits: `+1.0` for the left paddle, which sends the
    /// ball to the right.
    pub fn hits_toward(self) -> f32 {
        match self {
            Side::Left => 1.0,
            Side::Right => -1.0,
        }
    }

    /// The plane the ball's *centre* is on when it touches this paddle's face.
    pub fn contact_plane(self) -> f32 {
        self.paddle_x() + self.hits_toward() * (PADDLE_SIZE.x * 0.5 + BALL_RADIUS)
    }

    /// The x direction a ball travels to reach this side's goal line.
    pub fn goal_direction(self) -> f32 {
        match self {
            Side::Left => -1.0,
            Side::Right => 1.0,
        }
    }

    pub fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

/// Where along a tick's travel a ball first meets a paddle's face, as a
/// fraction of the tick, or `None` if it does not.
///
/// CONTRACT: `from` and `to` are the ball's centre at the two ends of one
/// tick's travel, `approach` is `-1.0` for a ball that must be moving in `-x`
/// to count and `+1.0` for `+x`, and `half_span` is how far from `paddle_y`
/// the ball's centre may be and still touch. Answers `None` for all three ways
/// of not touching: travel in the wrong direction, travel that does not reach
/// the plane within this tick, and travel that crosses the plane past the end
/// of the paddle.
///
/// This is swept rather than a position test at `to`, so it is correct however
/// far the ball moves in a tick — the fixed timestep only ever tests at tick
/// boundaries, and a fast ball would otherwise step clean through.
pub fn sweep_contact(
    from: Vec2,
    to: Vec2,
    plane_x: f32,
    approach: f32,
    paddle_y: f32,
    half_span: f32,
) -> Option<f32> {
    let travel = to.x - from.x;
    // Moving the way this face is struck from, and actually moving.
    if travel * approach <= 0.0 {
        return None;
    }
    // Starting on the side the face is struck from. A ball already behind the
    // plane is leaving, not arriving, and must not be caught on the way out.
    if (from.x - plane_x) * approach > 0.0 {
        return None;
    }
    let t = (plane_x - from.x) / travel;
    if !(0.0..=1.0).contains(&t) {
        return None;
    }
    let y = from.y + (to.y - from.y) * t;
    ((y - paddle_y).abs() <= half_span).then_some(t)
}

/// How far up or down a paddle the ball struck, as `-1.0` at the top edge
/// through `0.0` at the middle to `+1.0` at the bottom.
///
/// Y is down, so `+1.0` is the bottom of the screen.
pub fn contact_offset(ball_y: f32, paddle_y: f32) -> f32 {
    ((ball_y - paddle_y) / (PADDLE_SIZE.y * 0.5)).clamp(-1.0, 1.0)
}

/// The velocity a paddle sends the ball away with.
///
/// `offset` is `contact_offset`'s answer and `toward` is the side's
/// `hits_toward`. The engine's own `sin_cos`, so the same rally replays the
/// same way on every machine.
pub fn bounce_velocity(offset: f32, speed: f32, toward: f32) -> Vec2 {
    let (sine, cosine) = sin_cos(Radians(offset.clamp(-1.0, 1.0) * MAX_BOUNCE.as_f32()));
    Vec2::new(toward * speed * cosine, speed * sine)
}

/// Reflect `y` back and forth between `-limit` and `+limit` until it lands
/// inside, the way a ball bouncing between two walls does.
pub fn fold_between_walls(y: f32, limit: f32) -> f32 {
    let span = 2.0 * limit;
    let mut folded = (y + limit).rem_euclid(2.0 * span);
    if folded > span {
        folded = 2.0 * span - folded;
    }
    folded - limit
}

/// Bounce a ball off the top and bottom walls, if it has gone through one.
///
/// A reflection rather than a clamp: a clamp lets a steep ball crawl along the
/// wall for a few ticks instead of coming off it.
pub fn reflect_walls(pos: Vec2, vel: Vec2) -> (Vec2, Vec2) {
    let (mut pos, mut vel) = (pos, vel);
    if pos.y < -BALL_LIMIT {
        pos.y = -2.0 * BALL_LIMIT - pos.y;
        vel.y = -vel.y;
    } else if pos.y > BALL_LIMIT {
        pos.y = 2.0 * BALL_LIMIT - pos.y;
        vel.y = -vel.y;
    }
    (pos, vel)
}

/// One tick of a ball meeting nothing but the walls.
pub fn advance_ball(pos: Vec2, vel: Vec2, dt: f32) -> (Vec2, Vec2) {
    reflect_walls(pos + vel * dt, vel)
}

/// One tick of a paddle doing what it was told.
pub fn paddle_step(paddle_y: f32, intent: f32, speed: f32, dt: f32) -> f32 {
    (paddle_y + intent * speed * dt).clamp(-PADDLE_LIMIT, PADDLE_LIMIT)
}

/// One tick of the opponent.
///
/// It chases where the ball *is*, not where the ball is going, and that is the
/// whole of the difficulty. An opponent that predicts the crossing cannot lose
/// at any speed this court allows: the flattest, fastest shot it can face still
/// takes most of a second to arrive, and a paddle moving even at half the
/// player's speed covers the whole court in that time. So it is not made
/// beatable by being made slower; it is beatable because it lags behind a steep
/// ball and because a bounce off a wall sends it the wrong way first.
///
/// Shared with the check rather than copied into it: the controller in
/// `verify.rs` rolls candidate shots through *this* function, so it cannot
/// grade the game against a second, drifting copy of the opponent.
pub fn opponent_step(paddle_y: f32, ball_y: f32, dt: f32) -> f32 {
    let gap = ball_y - paddle_y;
    let intent = if gap.abs() < OPPONENT_DEAD_ZONE {
        0.0
    } else {
        gap.signum()
    };
    paddle_step(paddle_y, intent, OPPONENT_SPEED, dt)
}
