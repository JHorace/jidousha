//! The court, and the two decisions a check will want to ask about directly.
//!
//! Everything here is a `const` or a free function of its arguments. Nothing in
//! this file touches a `World`, which is the point: `docs/api/jidousha-api.md`
//! asks for the opponent's decision and the collision response to be callable
//! rather than buried in an `Update` body, because a check that wants to know
//! *where the ball will be* has no way to fork a running simulation and roll it
//! forward. It can only call a function. So these are functions.
//!
//! The layout is stated as constants derived from [`WINDOW`], which is the
//! prototype answer and is an answer about one aspect: at 16:9 the court is
//! 32x18 world units, and a player who drags the window narrower moves the side
//! walls in without any check being able to see it.

use jidousha::prelude::*;

// --- the court ---------------------------------------------------------

/// The window `run` opens at, and the one shape every extent below is stated in.
pub(crate) const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);

/// Half the world height the camera spans — the one number the layout picks.
pub(crate) const HALF_H: f32 = 9.0;

/// And half the width, which is the height times the shape of the window.
///
/// Derived rather than typed: `HALF_H * (16.0 / 9.0)` would be two facts about
/// one window, and changing [`WINDOW`] would leave the ratio silently stale.
pub(crate) const HALF_W: f32 = HALF_H * WINDOW.aspect();

/// How much empty space the camera leaves around the court, in world units.
///
/// The court is what the ball bounces in; the camera shows a little more of it
/// than that. Without this the border marking — which is a `ctx.line`, and
/// `ctx.line` centres its thickness on the segment — hangs half its width off
/// the screen, which is what the first verify run reported. Insetting the border
/// instead would fix the assertion and leave the closest quad 0.07 units from
/// the edge, which is the cliff the testing document describes: passing, and
/// starting to fail the day anything moves.
pub(crate) const MARGIN: f32 = 0.6;

/// How many world units the camera spans vertically.
pub(crate) const VIEW_HEIGHT: f32 = (HALF_H + MARGIN) * 2.0;

/// What the court is cleared to.
pub(crate) const COURT: Color = Color::rgb(0.04, 0.05, 0.07);

// --- the ball ----------------------------------------------------------

/// Half the ball's side, in world units. The ball is a square, as Pong's is.
///
/// Square rather than round because the collision arithmetic below is written
/// against an axis-aligned box, and drawing a disc over a box collider is a
/// discrepancy a picture would show and no assertion would.
pub(crate) const BALL_HALF: f32 = 0.28;

/// How fast a serve leaves the centre, in world units per second.
pub(crate) const BALL_SPEED_START: f32 = 22.0;

/// How much faster the ball gets with every paddle hit.
pub(crate) const BALL_SPEED_GAIN: f32 = 2.0;

/// The ceiling on ball speed, in world units per second.
///
/// **This number is not free: it is bounded by [`PADDLE_HALF_X`].** The engine
/// tests collisions only at tick boundaries, and although [`paddle_contact`]
/// sweeps the crossing itself, the wall clamp below does not — so a tick's
/// travel must stay inside the thinnest thing the ball must not pass through.
/// At 1/60s this is 0.70 units a tick against a paddle 1.0 unit thick. This
/// game *did* play too slowly — rallies of fifty touches, and neither side able
/// to score — and the paddle went from 0.64 thick to 1.0 **first**, which is
/// what let the speed go from 26 to 42. Raising the speed alone would have put
/// the ball through the paddle.
/// `checks::the_ball_cannot_outrun_the_thinnest_collider` is that inequality,
/// asserted against the `fixed_dt` the engine actually hands us.
pub(crate) const BALL_SPEED_MAX: f32 = 42.0;

/// The steepest a paddle can send the ball, measured from the horizontal.
pub(crate) const MAX_BOUNCE: Radians = Radians::from_degrees(58.0);

/// How far off the horizontal a serve may wander.
pub(crate) const SERVE_SPREAD: Radians = Radians::from_degrees(22.0);

/// How far the ball's *centre* may get from the middle before the wall stops it.
pub(crate) const BALL_Y_LIMIT: f32 = HALF_H - BALL_HALF;

/// How far the ball's *centre* travels before the point is over.
///
/// The court edge less the ball's own half, so the ball is never drawn hanging
/// off the side of the screen — which is where the first verify run found it,
/// as a quad reaching x 16.0011 against a camera ending at 16.0.
pub(crate) const GOAL_X: f32 = HALF_W - BALL_HALF;

// --- the paddles -------------------------------------------------------

/// Half a paddle's thickness. See [`BALL_SPEED_MAX`]: this is a speed limit.
pub(crate) const PADDLE_HALF_X: f32 = 0.5;

/// Half a paddle's height.
pub(crate) const PADDLE_HALF_Y: f32 = 1.6;

/// How far from the middle a paddle's centre line sits.
pub(crate) const PADDLE_X: f32 = 14.2;

/// How far a paddle's centre may get from the middle before the wall stops it.
pub(crate) const PADDLE_Y_LIMIT: f32 = HALF_H - PADDLE_HALF_Y;

/// Where the ball's centre is at the moment its leading edge touches a paddle.
///
/// A magnitude: the left paddle's contact plane is at `-CONTACT_X`.
pub(crate) const CONTACT_X: f32 = PADDLE_X - PADDLE_HALF_X - BALL_HALF;

/// How far off a paddle's centre a contact can land and still count.
pub(crate) const CONTACT_REACH: f32 = PADDLE_HALF_Y + BALL_HALF;

/// How fast the player's paddle moves, in world units per second.
pub(crate) const PLAYER_SPEED: f32 = 17.0;

/// How fast the opponent's paddle moves, in world units per second.
///
/// Slower than the player on purpose, and — the number that actually decides
/// whether this is a game — slower than the ball's steepest vertical component
/// **at the speed a rally starts at**, not at its top speed:
/// `BALL_SPEED_START * sin(MAX_BOUNCE)` is 12.72 units a second, against 11.0
/// here. Checked at the slow end because that is where a rally spends most of
/// itself; at the top speed the margin is 2.0x and says nothing about the
/// rallies a player actually has.
/// `checks::a_steep_return_outruns_the_opponent` is that inequality.
///
/// This started at 12.0, which clears the slow end by six percent. The
/// controllers document is explicit that looking fair is not the test, so it
/// came down until the margin was one a rally could use.
pub(crate) const OPPONENT_SPEED: f32 = 11.0;

/// How far the ball must have come before the opponent starts tracking it.
///
/// The whole of the opponent's fallibility lives in this number together with
/// [`OPPONENT_SPEED`]: it predicts perfectly, so if it were allowed to start
/// predicting at the moment of the player's return it would never miss. Half a
/// court of warning at 11 units a second is about seven units of travel against
/// a fifteen-unit court, so a shot to the far corner beats it and a shot near
/// where it stands does not.
///
/// Swept together with [`OPPONENT_PLACEMENT`] rather than picked: at the gate
/// four units further back the chaser goes 1-5 and the rally doubles in length;
/// four units forward and the opponent returns half as many. The three verdict
/// lines are what chose this row.
pub(crate) const OPPONENT_REACTION_X: f32 = 0.0;

/// Where on its paddle the opponent tries to take the ball, as a fraction of
/// [`CONTACT_REACH`].
///
/// **This is the constant that decides whether the game has a rally in it**, and
/// it is the one to look at before any speed. An opponent that aims at the
/// ball's exact height returns every ball dead flat, straight back down the
/// middle, and against anybody who also tracks the ball the rally has nowhere to
/// go: this game's first sweep produced 0-0 matches with seventy-touch rallies
/// for exactly that reason. Meeting the ball a fixed distance off its own centre
/// puts an angle on every return it makes.
pub(crate) const OPPONENT_PLACEMENT: f32 = 0.9;

// --- the round ---------------------------------------------------------

/// How long the ball waits at the centre before a serve, in ticks.
///
/// Ticks rather than seconds because the tick is the canonical timeline; at the
/// default `fixed_dt` this is three quarters of a second.
pub(crate) const SERVE_TICKS: u32 = 45;

/// How many points win a game.
pub(crate) const WINNING_SCORE: u32 = 5;

// --- who is who --------------------------------------------------------

/// Which end of the court a paddle defends.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Side {
    /// The player's end, at negative x.
    Left,
    /// The opponent's end, at positive x.
    Right,
}

impl Side {
    /// Which way this side lies from the middle: -1 for left, +1 for right.
    pub(crate) const fn sign(self) -> f32 {
        match self {
            Side::Left => -1.0,
            Side::Right => 1.0,
        }
    }

    /// The other end of the court.
    pub(crate) const fn other(self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    /// Where this side's paddle centre line sits, in world units.
    pub(crate) fn paddle_x(self) -> f32 {
        self.sign() * PADDLE_X
    }

    /// Where the ball's centre is when it touches this side's paddle.
    pub(crate) fn contact_x(self) -> f32 {
        self.sign() * CONTACT_X
    }
}

// --- the decisions -----------------------------------------------------

/// Where and when the ball's leading edge crosses `side`'s paddle face.
///
/// The eight lines of swept arithmetic the engine deliberately does not carry:
/// the plane the leading edge touches, whether the ball was approaching it,
/// whether this tick's travel crossed it, and the fraction of the tick at which
/// it did. `from` and `to` are the ball's centre at the two ends of one tick.
///
/// The paddle is treated as **stationary at its post-move position** for the
/// whole tick. That is a choice — the paddle really did move during the tick —
/// and it is the one that is right about the case that matters, a paddle
/// closing on the ball. `main::register` puts both paddle systems ahead of the
/// ball's for exactly this reason, and `checks::the_paddles_move_before_the_ball`
/// holds the schedule to it.
///
/// Returns `None` when there is no contact, which includes every case where any
/// input has gone to NaN: each conjunct below is false for NaN, so the negated
/// conjunction is true and the answer is "no contact" rather than a contact at
/// a NaN fraction of the tick.
pub(crate) fn paddle_contact(from: Vec2, to: Vec2, side: Side, paddle_y: f32) -> Option<Contact> {
    let sign = side.sign();
    // Distance from the ball's leading edge to the paddle face, positive while
    // the ball is still in front of it. Measured in the direction of approach,
    // so the same arithmetic serves both ends of the court.
    let before = sign * (side.contact_x() - from.x);
    let travel = sign * (to.x - from.x);
    let after = before - travel;

    let approaching = travel > 0.0; // not standing still, not going the other way
    let in_front = before >= 0.0; // not already past the face
    let reached = after <= 0.0; // this tick's travel did not stop short
    if !(approaching && in_front && reached) {
        return None;
    }

    let fraction = before / travel;
    let at = from.lerp(to, fraction);
    if (at.y - paddle_y).abs() > CONTACT_REACH {
        return None;
    }
    Some(Contact { fraction, at })
}

/// Where along a tick the ball met a paddle, and where that was.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Contact {
    /// How far through the tick the contact happened, in `0.0..=1.0`.
    pub(crate) fraction: f32,
    /// The ball's centre at the moment of contact.
    pub(crate) at: Vec2,
}

/// The velocity the ball leaves a paddle with.
///
/// Pong's whole feel is this function: where on the paddle the ball lands
/// decides the angle, so the paddle is an aiming device rather than a wall.
/// A contact at the paddle's centre goes straight back; one at either tip
/// leaves at [`MAX_BOUNCE`].
pub(crate) fn rebound(contact_y: f32, paddle_y: f32, side: Side, speed: f32) -> Vec2 {
    let offset = ((contact_y - paddle_y) / CONTACT_REACH).clamp(-1.0, 1.0);
    let (sine, cosine) = sin_cos(Radians(MAX_BOUNCE.as_f32() * offset));
    // Away from the paddle in x; the sign of the offset in y, so a hit above
    // the paddle's centre sends the ball up. Y is down, so "up" is negative.
    Vec2::new(-side.sign() * cosine, sine) * speed
}

/// Where the ball's centre will be in y when it reaches `plane_x`, with bounces
/// off the top and bottom walls folded in.
///
/// `None` when it never gets there: the ball is standing still in x, or heading
/// the other way, or the arithmetic has gone to NaN.
///
/// This is the function that makes a controller possible — both the opponent's
/// and the one `--verify` plays with. Chasing the ball's *current* y is a losing
/// strategy in Pong; being where it is going is the game.
pub(crate) fn predict_crossing(from: Vec2, velocity: Vec2, plane_x: f32) -> Option<f32> {
    let distance = (plane_x - from.x) / velocity.x;
    // Bound rather than written `if !(distance > 0.0)`, which is the shape the
    // API document's NaN advice gives and which `neg_cmp_op_on_partial_ord`
    // rejects when there is only one conjunct to negate. The naming keeps the
    // NaN behaviour the document is actually after: a NaN distance fails
    // `> 0.0`, so `reachable` is false and the answer is "never gets there".
    let reachable = distance > 0.0;
    if !reachable {
        return None;
    }
    let unfolded = from.y + velocity.y * distance;
    Some(fold_into_court(unfolded))
}

/// Reflect a y coordinate back and forth between the walls until it is inside.
///
/// A triangle wave: the same shape a ball bouncing between two walls traces,
/// worked out in one step instead of one bounce at a time.
pub(crate) fn fold_into_court(y: f32) -> f32 {
    let span = 2.0 * BALL_Y_LIMIT;
    let shifted = (y + BALL_Y_LIMIT).rem_euclid(2.0 * span);
    let folded = if shifted <= span {
        shifted
    } else {
        2.0 * span - shifted
    };
    folded - BALL_Y_LIMIT
}

/// Where the opponent wants the centre of its paddle to be.
///
/// Three cases, in order:
///
/// 1. The ball is not coming — drift back to the middle, which is the best
///    place to be when you do not know where you will be needed.
/// 2. The ball is coming but has not passed [`OPPONENT_REACTION_X`] — hold.
///    This is where the opponent's beatability lives; see that constant.
/// 3. The ball is coming — predict where it crosses the paddle's face and stand
///    so that the contact lands [`OPPONENT_PLACEMENT`] of the way up the paddle
///    *away* from where the player is, because a hit off the paddle's centre
///    goes back at an angle and a hit on its centre does not.
pub(crate) fn opponent_target(ball: Vec2, velocity: Vec2, player_y: f32) -> f32 {
    if velocity.x <= 0.0 {
        return 0.0;
    }
    if ball.x < OPPONENT_REACTION_X {
        return 0.0;
    }
    let Some(crossing) = predict_crossing(ball, velocity, Side::Right.contact_x()) else {
        return 0.0;
    };
    // Send it away from the player: if the player is high on the court, take the
    // ball low on the paddle, which returns it low.
    let away = if player_y < 0.0 { 1.0 } else { -1.0 };
    let placed = crossing - away * OPPONENT_PLACEMENT * CONTACT_REACH;
    placed.clamp(-PADDLE_Y_LIMIT, PADDLE_Y_LIMIT)
}
