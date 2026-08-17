//! Everything this game puts on screen.
//!
//! Three Draw systems, in the order they are registered: the field behind
//! everything, the things the game is about, and the readout over the top. A
//! Draw system takes a `DrawCtx` and cannot change the world — the type says
//! so — so all three read and none of them decide anything.
//!
//! Ordering comes from `Depth`, not from the order things were submitted, which
//! is why the bands are named in `layers` rather than written as numbers here.

use jidousha::prelude::*;

/// How tall the banner across the middle is, and where its top edge sits.
/// Named because the check looks for a glyph inside that band: a screen that
/// lost its headline still passes a bounds check, since nothing drawn is
/// trivially inside.
pub(crate) const BANNER_SIZE: f32 = 1.6;
pub(crate) const BANNER_TOP: f32 = -BANNER_SIZE;

use crate::{
    BALL_RADIUS, Ball, Control, FIELD, PADDLE_SIZE, Paddle, Round, SCORE_SIZE, SCORE_TOP,
    Scoreboard, Side, WINNING_SCORE, layers,
};

/// The border, the halfway line, and the two goal lines.
pub(crate) fn draw_the_field(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::FIELD);
    let edge = Color::rgba(1.0, 1.0, 1.0, 0.22);
    let corners = [
        Vec2::new(-FIELD.x, -FIELD.y),
        Vec2::new(FIELD.x, -FIELD.y),
        Vec2::new(FIELD.x, FIELD.y),
        Vec2::new(-FIELD.x, FIELD.y),
    ];
    for index in 0..4 {
        ctx.line(corners[index], corners[(index + 1) % 4], 0.12, edge, depth);
    }

    // The halfway line, as dashes. Twelve of them, so the gaps are even
    // whatever the field height is.
    let dashes = 12;
    let span = FIELD.y * 2.0 / f32::from(dashes as u16);
    for index in 0..dashes {
        let top = -FIELD.y + span * index as f32 + span * 0.2;
        ctx.line(
            Vec2::new(0.0, top),
            Vec2::new(0.0, top + span * 0.6),
            0.12,
            Color::rgba(1.0, 1.0, 1.0, 0.12),
            depth,
        );
    }
}

/// The paddles and the ball, from where the world says they are.
pub(crate) fn draw_the_play(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::PLAY);
    let paddles: Vec<(Vec2, Control)> = ctx
        .world
        .query::<(&Transform, &Paddle)>()
        .map(|(_, transform, paddle)| (transform.pos, paddle.played_by))
        .collect();
    for (at, played_by) in paddles {
        let color = match played_by {
            Control::Keyboard => Color::rgb(0.45, 1.0, 0.75),
            Control::Machine => Color::rgb(1.0, 0.55, 0.45),
        };
        ctx.rect(Rect::from_center_size(at, PADDLE_SIZE), color, depth);
    }

    // Nothing to look at between points, so the ball is only drawn in play.
    if ctx.world.resource::<Scoreboard>().round != Round::Rallying {
        return;
    }
    let ball = ctx
        .world
        .query::<(&Transform, With<Ball>)>()
        .map(|(_, transform, _)| transform.pos)
        .next();
    if let Some(at) = ball {
        ctx.circle(at, BALL_RADIUS, Color::WHITE, depth);
    }
}

/// The score above the field, a banner in the middle when nothing is in play,
/// and the controls along the bottom.
pub(crate) fn draw_the_readout(ctx: &mut DrawCtx) {
    let board = ctx.world.resource::<Scoreboard>();

    let score = TextStyle {
        size: SCORE_SIZE,
        color: Color::WHITE,
        depth: Depth::layer(layers::UI),
    };
    let text = board.text();
    ctx.text(
        Vec2::new(-score.width_of(&text) * 0.5, SCORE_TOP),
        &text,
        score,
    );

    // Every string here is plain ASCII. The only font the engine has is the one
    // `prototype_kit` prints its printable range from, and that range is 0x20 to
    // 0x7e — a nicer-looking "—" or "·" still submits a quad, so the layout
    // checks all pass and there is nothing to say whether it drew a dash or a
    // blank. Not worth finding out from a screenshot.
    let banner = TextStyle {
        size: BANNER_SIZE,
        color: Color::rgb(1.0, 0.95, 0.6),
        depth: Depth::layer(layers::UI),
    };
    let headline = match board.round {
        Round::Serving { .. } => Some("get ready"),
        Round::Rallying => None,
        Round::Over { winner } => Some(match winner {
            Side::Left => "you win",
            Side::Right => "the machine wins",
        }),
    };
    if let Some(headline) = headline {
        ctx.text(
            Vec2::new(-banner.width_of(headline) * 0.5, -banner.size),
            headline,
            banner,
        );
    }

    let small = TextStyle {
        size: 0.8,
        color: Color::rgba(0.7, 0.85, 1.0, 0.85),
        depth: Depth::layer(layers::UI),
    };
    // Drawn as its own centred line rather than tacked onto the headline: one
    // long string is the shape that runs off both edges of the screen at once.
    if matches!(board.round, Round::Over { .. }) {
        let text = "space to play again";
        ctx.text(
            Vec2::new(-small.width_of(text) * 0.5, small.size * 0.5),
            text,
            small,
        );
    }

    let text = format!("w / s to move - first to {WINNING_SCORE}");
    ctx.text(
        Vec2::new(-small.width_of(&text) * 0.5, FIELD.y + 0.7),
        &text,
        small,
    );
}
