//! The table's dimensions and the game's tuning, in one place.
//!
//! Everything here is in world units and units-per-second, never pixels and
//! never per-tick: `GameConfig::fixed_dt` is what turns a rate into a step, and
//! a constant with the timestep already baked into it silently means something
//! else the day that changes.
//!
//! The verification reads these same constants, so a check cannot keep passing
//! against a table that has been resized underneath it.

use jidousha::prelude::*;

/// How many world units the camera spans vertically.
///
/// At the default 1280x720 window that makes the table 35.56 units across.
pub(crate) const VIEW_HEIGHT: f32 = 20.0;

/// Half the height of the playable strip: the ball bounces off `±WALL_Y`.
pub(crate) const WALL_Y: f32 = 9.0;

/// How far out the goal lines are. Comfortably behind the paddles, so a ball
/// that beats one is visibly past it before the point is given.
pub(crate) const GOAL_X: f32 = 17.0;

/// Where a paddle's centre sits, either side of the middle.
pub(crate) const PADDLE_X: f32 = 15.4;

/// How big a paddle is, in world units.
pub(crate) const PADDLE_SIZE: Vec2 = Vec2::new(0.7, 4.0);

/// How far a paddle's centre may travel from the middle, so it stays inside
/// the walls rather than hanging through them.
pub(crate) const PADDLE_TRAVEL: f32 = WALL_Y - PADDLE_SIZE.y * 0.5;

/// The ball's radius.
pub(crate) const BALL_RADIUS: f32 = 0.42;

/// The player's paddle speed, in world units per second.
pub(crate) const PLAYER_SPEED: f32 = 26.0;

/// The machine's. Slower than the player's on purpose: this is the whole of
/// the difficulty setting, and a paddle that could always arrive in time is
/// not an opponent.
pub(crate) const MACHINE_SPEED: f32 = 20.0;

/// How far off its target the machine will sit rather than chase, so it
/// settles instead of shivering one step either side.
pub(crate) const MACHINE_DEAD_BAND: f32 = 0.35;

/// How often the machine paddle looks at the ball, in ticks.
///
/// This is the difficulty knob, and it is a reaction time rather than a speed
/// on purpose. A machine paddle that reads the ball every tick is not beatable
/// by any speed that still looks like it is trying: the ball takes most of a
/// second to cross a table only fourteen units tall, so eighteen units a second
/// covers everything, and dropping it far enough to miss makes it visibly
/// asleep. Reading every twelfth tick instead leaves it moving at a believable
/// pace and lagging behind a fast ball by more than its own length — so what
/// beats it is hitting hard and steep, which is what beats a person too.
///
/// Ticks rather than seconds because the tick is the canonical timeline: a
/// fifth of a second at the default timestep.
pub(crate) const MACHINE_REACTION: u64 = 12;

/// How fast a serve leaves the middle.
pub(crate) const SERVE_SPEED: f32 = 21.0;

/// How much faster the ball gets with each paddle it beats.
pub(crate) const SPEED_GAIN: f32 = 1.15;

/// The ball's ceiling.
///
/// This is the number that keeps the swept paddle test honest and the wall
/// bounce exact: at the default 60 Hz timestep a tick of travel is 0.55 world
/// units, comfortably less than the paddle is thick.
pub(crate) const MAX_BALL_SPEED: f32 = 33.0;

/// The widest angle off the horizontal a paddle can send the ball, reached at
/// the very tip. Dead centre sends it straight back.
pub(crate) const MAX_BOUNCE: Radians = Radians(0.95);

/// How far either way a serve may be aimed.
pub(crate) const SERVE_SPREAD: Radians = Radians(0.45);

/// The pause between a point and the next serve, in ticks.
///
/// Ticks rather than seconds because the tick is the canonical timeline: three
/// quarters of a second at the default timestep.
pub(crate) const SERVE_PAUSE: u32 = 45;

/// Points needed to win a match.
pub(crate) const WINNING_SCORE: u32 = 5;

/// Draw bands. Naming them once is what stops `layer: 2` appearing in a dozen
/// places; the engine sorts by these numbers and has no opinion about them.
pub(crate) mod layers {
    /// The table, its border and its centre line.
    pub(crate) const TABLE: i16 = -1;
    /// Paddles and ball.
    pub(crate) const PLAY: i16 = 0;
    /// Score, banners and the hint line.
    pub(crate) const UI: i16 = 1;
}

/// The line printed to the terminal and drawn along the bottom of the table.
///
/// Plain ASCII on purpose: the font covers space through `~` and draws anything
/// else as a box of exactly the same size, so a stray em dash or curly quote
/// would pass every assertion this game makes about what was drawn.
pub(crate) const HINT: &str = "W/S move the left paddle - SPACE serves - first to 5 wins";
