//! The Draw systems: the course, the zone, the golfers, the aim preview, the bands, the result.
//!
//! Key functions: `draw_course`, `draw_zone`, `draw_players`, `draw_aim`, `draw_bands`,
//! `draw_result`.
//! Depends on: `contact`, `items`, `outcome`, `shots`, `text`, `world`, `zone`.
//! Never depended on outside the game.
//! INVARIANT: every ring is dots clipped to the course rect, so the opening zone, which
//! is bigger than the screen, draws only where it crosses the course.

use jidousha::prelude::*;

use crate::contact::{Contact, contact_target};
use crate::items::Item;
use crate::outcome::{HOLE_OPENS, HOLE_RADIUS, PAD_RADIUS, PadState};
use crate::shots::{MAX_SHOT, SHOT_SPREAD};
use crate::text::{
    aim_landing, aiming, cue_line, look, pickup_line, result_lines, status_line_1, status_line_2,
    tag_of,
};
use crate::world::{Ball, MatchState, Pickup, Player};
use crate::zone::{next_stop, zone_at};
use crate::{COURSE_HALF, COURT, HALF_W, LINE_Y, TEXT_SIZE, layers};

/// The course's grass.
pub const COURSE_FILL: Color = Color::rgb(0.14, 0.28, 0.16);
/// The current zone's ring.
pub const ZONE_NOW: Color = Color::rgb(0.3, 0.6, 1.0);
/// The next zone's ring.
pub const ZONE_NEXT: Color = Color::WHITE;
/// The ring round whatever a swing would hit.
pub const TARGET_RING: Color = Color::rgb(1.0, 0.55, 0.1);
/// The preview's landing disc when the spot will be safe.
pub const LAND_SAFE: Color = Color::rgba(0.2, 1.0, 0.3, 0.35);
/// The preview's landing disc when it will not.
pub const LAND_UNSAFE: Color = Color::rgba(1.0, 0.25, 0.2, 0.35);
/// The ring of the longest shot.
const REACH_RING: Color = Color::rgba(0.85, 0.85, 0.85, 0.6);
/// The size of a zone ring's dots, in world units.
pub const ZONE_DOT: f32 = 0.18;
/// How many dots the current zone's ring is drawn with.
pub const NOW_DOTS: usize = 48;
/// How many dots the next zone's ring is drawn with.
pub const NEXT_DOTS: usize = 36;

/// A golfer's colour: the human white, each rival their own.
pub fn hue(index: usize) -> Color {
    match index {
        0 => Color::WHITE,
        1 => Color::rgb(1.0, 0.85, 0.2),
        2 => Color::rgb(1.0, 0.3, 0.3),
        3 => Color::rgb(0.75, 0.4, 1.0),
        4 => Color::rgb(0.3, 1.0, 0.55),
        _ => Color::rgb(1.0, 0.55, 0.8),
    }
}

/// A piece of equipment's colour.
fn item_color(item: Item) -> Color {
    match item {
        Item::HeavyBall => Color::rgb(0.6, 0.6, 0.75),
        Item::LightBall => Color::rgb(0.95, 0.95, 0.6),
        Item::Helmet => Color::rgb(0.2, 0.8, 0.8),
        Item::BigClub => Color::rgb(0.7, 0.45, 0.2),
    }
}

/// The course as a rect.
pub fn course_rect() -> Rect {
    Rect::from_center_size(Vec2::ZERO, COURSE_HALF * 2.0)
}

/// A ring of `count` square dots of side `side` round `center`, skipping any whose
/// rect is not wholly inside the course.
fn ring(
    ctx: &mut DrawCtx,
    center: Vec2,
    radius: f32,
    count: usize,
    side: f32,
    color: Color,
    depth: Depth,
) {
    let course = course_rect();
    for k in 0..count {
        let angle = Radians::TAU.as_f32() * k as f32 / count as f32;
        let at = center + rotate(Vec2::X * radius, Radians(angle));
        let rect = Rect::from_center_size(at, Vec2::splat(side));
        if course.contains_rect(rect) {
            ctx.rect(rect, color, depth);
        }
    }
}

/// The text style the tags and labels use.
fn style(size: f32, color: Color, depth: Depth) -> TextStyle {
    TextStyle {
        size,
        color,
        depth,
        ..TextStyle::default()
    }
}

/// The court, its edge, the hole and the pad.
pub fn draw_course(ctx: &mut DrawCtx) {
    let tick = ctx.world.resource::<Time>().tick;
    let course = *ctx.world.resource::<crate::world::Course>();
    ctx.rect(course_rect(), COURSE_FILL, Depth::layer(layers::COURSE));
    let edge = Depth {
        layer: layers::COURSE,
        z: 0.5,
    };
    let (lo, hi) = (-COURSE_HALF, COURSE_HALF);
    let corners = [lo, Vec2::new(hi.x, lo.y), hi, Vec2::new(lo.x, hi.y)];
    for k in 0..4 {
        ctx.line(
            corners[k],
            corners[(k + 1) % 4],
            0.06,
            Color::rgb(0.8, 0.8, 0.8),
            edge,
        );
    }
    let open = tick >= HOLE_OPENS;
    let hole_color = if open {
        Color::rgb(0.03, 0.03, 0.03)
    } else {
        Color::rgb(0.45, 0.45, 0.45)
    };
    ctx.circle(course.hole, HOLE_RADIUS, hole_color, edge);
    if open {
        let top = course.hole - Vec2::new(0.0, 0.9);
        ctx.line(course.hole, top, 0.05, Color::WHITE, edge);
        ctx.rect(
            Rect::from_min_size(top, Vec2::new(0.3, 0.2)),
            Color::rgb(1.0, 0.2, 0.2),
            edge,
        );
    }
    let pad_color = match course.pad_state(tick) {
        PadState::Closed { .. } => Color::rgb(0.5, 0.5, 0.5),
        PadState::Open { .. } => Color::rgb(1.0, 0.9, 0.2),
        PadState::Gone => Color::rgb(0.5, 0.1, 0.1),
    };
    ring(ctx, course.pad, PAD_RADIUS, 16, 0.15, pad_color, edge);
}

/// The current zone and the next one, as dotted rings.
pub fn draw_zone(ctx: &mut DrawCtx) {
    let tick = ctx.world.resource::<Time>().tick;
    let course = *ctx.world.resource::<crate::world::Course>();
    let now = zone_at(&course.schedule, tick);
    ring(
        ctx,
        now.center,
        now.radius,
        NOW_DOTS,
        ZONE_DOT,
        ZONE_NOW,
        Depth::layer(layers::ZONE),
    );
    if let Some((_, next)) = next_stop(&course.schedule, tick) {
        ring(
            ctx,
            next.center,
            next.radius,
            NEXT_DOTS,
            ZONE_DOT,
            ZONE_NEXT,
            Depth::layer(layers::ZONE),
        );
    }
}

/// Bodies, balls, pickups, their tags, and the ring round the swing's target.
pub fn draw_players(ctx: &mut DrawCtx) {
    let l = look(&ctx.world);
    let play = Depth::layer(layers::PLAY);
    let tag_depth = Depth {
        layer: layers::PLAY,
        z: 0.5,
    };
    let bounds = course_rect();
    for (_, transform, player) in ctx.world.query::<(&Transform, &Player)>() {
        let dazed = l.tick < player.dazed_until;
        let mut color = hue(player.index);
        if dazed {
            color.a = 0.5;
        }
        ctx.circle(transform.pos, 0.35, color, play);
        let tag = tag_of(player.index);
        let tag_style = style(0.4, hue(player.index), tag_depth);
        let width = tag_style.width_of(&tag);
        let at = Vec2::new(
            (transform.pos.x + 0.45).min(bounds.max.x - width),
            (transform.pos.y - 0.2).clamp(bounds.min.y, bounds.max.y - 0.75),
        );
        ctx.text(at, &tag, tag_style);
        if dazed {
            ctx.text(
                at + Vec2::new(0.0, 0.4),
                "DAZED",
                style(0.3, Color::rgb(1.0, 1.0, 1.0), tag_depth),
            );
        }
    }
    for (_, transform, ball) in ctx.world.query::<(&Transform, &Ball)>() {
        ctx.circle(transform.pos, 0.18, hue(ball.owner), play);
    }
    for (_, transform, pickup) in ctx.world.query::<(&Transform, &Pickup)>() {
        let side = 0.3;
        ctx.rect(
            Rect::from_center_size(transform.pos, Vec2::splat(side)),
            item_color(pickup.0),
            play,
        );
        let label = style(0.3, item_color(pickup.0), tag_depth);
        let width = label.width_of(pickup.0.tag());
        let at = Vec2::new(
            (transform.pos.x + 0.25).min(bounds.max.x - width),
            (transform.pos.y - 0.15).clamp(bounds.min.y, bounds.max.y - 0.3),
        );
        ctx.text(at, pickup.0.tag(), label);
    }
    if let Some(me) = &l.me
        && matches!(l.state, MatchState::Playing)
    {
        let target = match contact_target(me, &l.snaps, l.tick) {
            Some(Contact::Club { who, .. }) => {
                l.snaps.iter().find(|s| s.index == who).map(|s| s.pos)
            }
            Some(Contact::Strike { whose, .. }) => {
                l.snaps.iter().find(|s| s.index == whose).map(|s| s.ball)
            }
            None => None,
        };
        if let Some(at) = target {
            ring(
                ctx,
                at,
                0.5,
                12,
                0.1,
                TARGET_RING,
                Depth {
                    layer: layers::AIM,
                    z: 0.0,
                },
            );
        }
    }
}

/// The aim preview: reach ring, line, landing and its verdict.
pub fn draw_aim(ctx: &mut DrawCtx) {
    let l = look(&ctx.world);
    if !aiming(&l) {
        return;
    }
    let (Some(me), Some((aim, length, safe))) = (&l.me, aim_landing(&l)) else {
        return;
    };
    let depth = Depth::layer(layers::AIM);
    ring(
        ctx,
        me.ball,
        MAX_SHOT * me.effects.shot_reach,
        24,
        0.12,
        REACH_RING,
        depth,
    );
    ctx.line(me.ball, aim, 0.06, Color::WHITE, depth);
    ctx.circle(
        aim,
        0.15,
        Color::WHITE,
        Depth {
            layer: layers::AIM,
            z: 1.0,
        },
    );
    let tint = if safe { LAND_SAFE } else { LAND_UNSAFE };
    ctx.circle(aim, SHOT_SPREAD * length, tint, depth);
}

/// The four band lines.
pub fn draw_bands(ctx: &mut DrawCtx) {
    let lines = [
        status_line_1(&ctx.world),
        status_line_2(&ctx.world),
        cue_line(&ctx.world),
        pickup_line(&ctx.world),
    ];
    let style = style(TEXT_SIZE, Color::WHITE, Depth::layer(layers::UI));
    for (line, y) in lines.iter().zip(LINE_Y) {
        if !line.is_empty() {
            ctx.text(Vec2::new(-HALF_W + 0.2, y), line, style);
        }
    }
}

/// The result screen: three centred lines over a backing rect.
pub fn draw_result(ctx: &mut DrawCtx) {
    if !matches!(ctx.world.resource::<MatchState>(), MatchState::Over { .. }) {
        return;
    }
    let lines = result_lines(&ctx.world);
    let line_style = style(
        0.9,
        Color::WHITE,
        Depth {
            layer: layers::UI,
            z: 2.0,
        },
    );
    let widest = lines
        .iter()
        .map(|line| line_style.width_of(line))
        .fold(0.0, f32::max);
    let block = Vec2::new(widest, 0.9 * 3.0) + Vec2::splat(1.0);
    ctx.rect(
        Rect::from_center_size(Vec2::ZERO, block),
        COURT,
        Depth {
            layer: layers::UI,
            z: 1.0,
        },
    );
    for (k, line) in lines.iter().enumerate() {
        let x = -line_style.width_of(line) * 0.5;
        ctx.text(Vec2::new(x, -1.35 + 0.9 * k as f32), line, line_style);
    }
}
