//! Everything the game draws, and the layout constants it draws against.
//!
//! Drawing is submission, not painting: a `Draw` system hands the renderer
//! quads and cannot touch the world — the `DrawCtx` it is called with says so
//! in the type. Order comes from `Depth`, not from the order these run in.
//!
//! The constants below are `pub(crate)` because the verification asserts
//! against them. "On screen" is not "in the right place", and a check that
//! carried its own copy of the score's position would keep passing after the
//! layout moved underneath it.

use jidousha::prelude::*;

use crate::{BALL_RADIUS, GOAL_X, PADDLE_SIZE, WALL_Y, WINNING_SCORE};

/// A score wider than one digit would not fit the layout above. Checked here
/// rather than in the verification, because it is a fact about the constant
/// rather than about a run.
const _: () = assert!(WINNING_SCORE < 10, "the score is drawn as a single digit");
use crate::{Ball, HINT, Paddle, Play, Scoreboard, Side, layers};

/// How tall the two score digits are, in world units.
pub(crate) const SCORE_SIZE: f32 = 2.8;

/// How far either side of the middle each score digit is centred.
pub(crate) const SCORE_X: f32 = 5.0;

/// The top edge of the score glyphs. Y is down, so this is near the top of the
/// table and comfortably inside the upper wall.
pub(crate) const SCORE_TOP: f32 = -8.3;

/// The hint line's height, and the top edge it is drawn from — outside the
/// walls, in the strip between the table and the bottom of the camera.
pub(crate) const HINT_SIZE: f32 = 0.62;
pub(crate) const HINT_TOP: f32 = WALL_Y + 0.3;

/// The end-of-match banner: two centred lines, the first at `BANNER_TOP`.
pub(crate) const BANNER_SIZE: f32 = 1.5;
pub(crate) const BANNER_TOP: f32 = -1.6;
/// The gap between the banner's two lines, top edge to top edge.
pub(crate) const BANNER_LEADING: f32 = 2.2;

/// How thick the table's markings are drawn.
const MARKING: f32 = 0.22;

/// How long one dash of the centre line is, and the gap after it.
const DASH: f32 = 1.1;
const DASH_GAP: f32 = 0.7;

/// The rectangle the ball is allowed to be in: the walls and the goal lines.
pub(crate) fn table() -> Rect {
    Rect {
        min: Vec2::new(-GOAL_X, -WALL_Y),
        max: Vec2::new(GOAL_X, WALL_Y),
    }
}

/// The second line of the end-of-match banner, as its own constant so the
/// verification can check it is printable without knowing how it is built.
pub(crate) const BANNER_PROMPT: &str = "press SPACE for a new match";

/// The banner's first line for a given winner.
pub(crate) fn banner_verdict(board: &Scoreboard) -> Option<String> {
    let Play::Over { winner } = board.play else {
        return None;
    };
    Some(format!(
        "{} wins {} - {}",
        winner.name(),
        board.points(winner),
        board.points(winner.other()),
    ))
}

/// The table: two walls, two goal lines, a dashed middle, and the score behind
/// the play the way every Pong has drawn it.
pub(crate) fn the_table(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::TABLE);
    let table = table();
    let wall = Color::rgba(0.55, 0.75, 0.95, 0.55);
    let goal = Color::rgba(0.55, 0.75, 0.95, 0.22);

    for y in [table.min.y, table.max.y] {
        ctx.line(
            Vec2::new(table.min.x, y),
            Vec2::new(table.max.x, y),
            MARKING,
            wall,
            depth,
        );
    }
    for x in [table.min.x, table.max.x] {
        ctx.line(
            Vec2::new(x, table.min.y),
            Vec2::new(x, table.max.y),
            MARKING * 0.7,
            goal,
            depth,
        );
    }

    // The middle, as dashes. Walked from the top wall down in fixed steps so
    // the pattern is the same every frame rather than fitted to the space.
    let mut y = table.min.y + DASH_GAP;
    while y + DASH <= table.max.y {
        ctx.line(
            Vec2::new(0.0, y),
            Vec2::new(0.0, y + DASH),
            MARKING * 0.6,
            Color::rgba(0.55, 0.75, 0.95, 0.3),
            depth,
        );
        y += DASH + DASH_GAP;
    }

    // The score, dim and behind everything, so the ball passes in front of it.
    let board = ctx.world.resource::<Scoreboard>();
    let style = TextStyle {
        size: SCORE_SIZE,
        color: Color::rgba(0.75, 0.9, 1.0, 0.30),
        depth,
    };
    for side in [Side::Left, Side::Right] {
        let text = board.points(side).to_string();
        let x = match side {
            Side::Left => -SCORE_X,
            Side::Right => SCORE_X,
        };
        ctx.text(
            Vec2::new(x - style.width_of(&text) * 0.5, SCORE_TOP),
            &text,
            style,
        );
    }
}

/// The paddles and the ball.
pub(crate) fn the_play(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::PLAY);

    // Queried and drawn in one pass, with no `Vec` in between. `WorldView`'s
    // query hands back an iterator borrowing the *world*, not the `DrawCtx`, so
    // `ctx.rect` inside the loop is not a second borrow of the same thing — the
    // read-first/write-second shape a `&mut World` system needs does not apply
    // here. Worth stating because the two worked examples disagree: the
    // Quickstart iterates directly and `prototype_kit` collects into a `Vec`
    // first, and a game author reading both learns there is no rule.
    for (_, transform, paddle) in ctx.world.query::<(&Transform, &Paddle)>() {
        let color = match paddle.side {
            Side::Left => Color::rgb(0.45, 1.0, 0.75),
            Side::Right => Color::rgb(1.0, 0.62, 0.45),
        };
        ctx.rect(
            Rect::from_center_size(transform.pos, PADDLE_SIZE),
            color,
            depth,
        );
    }

    for (_, transform, _) in ctx.world.query::<(&Transform, &Ball)>() {
        ctx.circle(
            transform.pos,
            BALL_RADIUS,
            Color::rgb(1.0, 0.97, 0.85),
            depth,
        );
    }
}

/// The hint line, and the banner at the end of a match.
pub(crate) fn the_readout(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::UI);
    let hint = TextStyle {
        size: HINT_SIZE,
        color: Color::rgba(0.6, 0.8, 1.0, 0.75),
        depth,
    };
    ctx.text(Vec2::new(-hint.width_of(HINT) * 0.5, HINT_TOP), HINT, hint);

    let board = *ctx.world.resource::<Scoreboard>();
    let Some(verdict) = banner_verdict(&board) else {
        return;
    };
    let big = TextStyle {
        size: BANNER_SIZE,
        color: Color::rgb(1.0, 0.95, 0.7),
        depth,
    };
    let small = TextStyle {
        size: BANNER_SIZE * 0.6,
        color: Color::rgba(1.0, 0.95, 0.7, 0.8),
        depth,
    };
    // Two `text` calls rather than one string with a `\n` in it: `width_of`
    // measures a block's *widest* line, so centring a two-line block by it
    // centres the long line and leaves the short one hanging off to the left.
    ctx.text(
        Vec2::new(-big.width_of(&verdict) * 0.5, BANNER_TOP),
        &verdict,
        big,
    );
    ctx.text(
        Vec2::new(
            -small.width_of(BANNER_PROMPT) * 0.5,
            BANNER_TOP + BANNER_LEADING,
        ),
        BANNER_PROMPT,
        small,
    );
}
