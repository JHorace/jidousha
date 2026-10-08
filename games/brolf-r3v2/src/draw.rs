//! The `Draw` phase: the course and the zone, the play, the HUD.
//!
//! Every ring goes through `clip_to_course`, golfers are clamped and balls
//! reflect, so nothing is drawn at a radius that can leave the camera
//! (DESIGN.md, Systems: draw). The zone is two rings rather than a disc: a disc
//! of radius 34 cannot be drawn inside a camera 36 units tall.

use jidousha::prelude::*;

use crate::hud::{LEGEND, aiming, hud_lines, result_lines};
use crate::model::*;
use crate::rules::*;
use crate::world::snap;

/// Draw bands, low to high.
pub mod layers {
    /// The course.
    pub const FIELD: i16 = -1;
    /// Rings, lines and marks on the course.
    pub const MARK: i16 = 0;
    /// Golfers, balls, pickups.
    pub const PLAY: i16 = 1;
    /// The HUD and the banner.
    pub const UI: i16 = 2;
}

/// How thick a ring's segments are.
pub const RING_THICKNESS: f32 = 0.15;
/// The HUD's first row, its left edge and its size.
pub const HUD_AT: Vec2 = Vec2::new(-31.0, -17.5);
/// HUD text size.
pub const HUD_SIZE: f32 = 1.0;
/// The legend's row and size.
pub const LEGEND_Y: f32 = 16.6;
/// The legend's text size.
pub const LEGEND_SIZE: f32 = 0.8;
/// The banner's rows.
pub const BANNER_Y: [f32; 3] = [-3.2, -1.2, 0.8];
/// The banner's text size.
pub const BANNER_SIZE: f32 = 1.6;
/// The banner's backing.
pub const BANNER_BACK_RECT: Rect = Rect {
    min: Vec2::new(-20.0, -4.4),
    max: Vec2::new(20.0, 3.6),
};

/// A ring of `segments` lines around `center`, clipped to the course.
fn ring(ctx: &mut DrawCtx, center: Vec2, radius: f32, segments: u32, color: Color, depth: Depth) {
    if radius <= 0.0 {
        return;
    }
    let point = |k: u32| {
        let turn = Radians(Radians::TAU.as_f32() * k as f32 / segments as f32);
        center + direction(turn) * radius
    };
    for k in 0..segments {
        if let Some((a, b)) = clip_to_course(point(k), point(k + 1)) {
            ctx.line(a, b, RING_THICKNESS, color, depth);
        }
    }
}

/// A text style on a band.
fn style(size: f32, color: Color, layer: i16, z: f32) -> TextStyle {
    TextStyle {
        face: Face::BUILT_IN,
        size,
        color,
        depth: Depth { layer, z },
    }
}

/// The course, the zone now and when I get to my ball, and the gate.
pub fn draw_course(ctx: &mut DrawCtx) {
    let view = snap(&ctx.world);
    ctx.rect(COURSE, palette::COURSE_GREEN, Depth::layer(layers::FIELD));
    let mark = Depth::layer(layers::MARK);
    ring(
        ctx,
        CUP,
        zone_at(view.now).radius,
        64,
        palette::ZONE_RING,
        mark,
    );
    if let Some(me) = view.golfer(0)
        && let Some(aim) = aiming(&view, me)
    {
        ring(ctx, CUP, aim.zone.radius, 64, palette::NEXT_RING, mark);
    }
    ctx.circle(CUP, CUP_RADIUS, Color::BLACK, mark);
    let gate = gate();
    let corners = [
        gate.min,
        Vec2::new(gate.max.x, gate.min.y),
        gate.max,
        Vec2::new(gate.min.x, gate.max.y),
    ];
    for k in 0..4 {
        ctx.line(
            corners[k],
            corners[(k + 1) % 4],
            0.15,
            palette::PICKUP,
            mark,
        );
    }
    let label = style(0.8, palette::PICKUP, layers::MARK, 1.0);
    let at = Vec2::new(
        GATE_CENTER.x - label.width_of("GATE") * 0.5,
        GATE_CENTER.y - 0.4,
    );
    ctx.text(at, "GATE", label);
}

/// The aim line, the reach cue, balls, golfers and pickups.
pub fn draw_play(ctx: &mut DrawCtx) {
    let view = snap(&ctx.world);
    let mark = Depth::layer(layers::MARK);
    if let Some(me) = view.golfer(0).copied() {
        if let (Some(aim), Some(ball)) = (aiming(&view, &me), view.ball(0)) {
            let safe = aim.roll.sunk || inside_zone(aim.zone, aim.roll.rest);
            let tone = if safe {
                palette::SAFE_DOT
            } else {
                palette::DANGER
            };
            ctx.line(ball.pos, aim.roll.rest, 0.1, tone, mark);
            ctx.circle(
                aim.roll.rest,
                0.4,
                tone,
                Depth {
                    layer: layers::MARK,
                    z: 1.0,
                },
            );
        }
        let fx = effects(&me.kit);
        if me.alive
            && view.result.is_none()
            && let Some(contact) = contact_target(&me, &view.golfers, &view.balls, fx)
        {
            ring(
                ctx,
                me.pos,
                CLUB_REACH * fx.reach_scale,
                32,
                palette::DANGER,
                mark,
            );
            let at = match contact {
                Contact::Club(idx) => view.golfer(idx).map(|g| g.pos),
                Contact::Strike(idx) => view.ball(idx).map(|b| b.pos),
            };
            if let Some(at) = at {
                ring(ctx, at, 1.1, 32, palette::DANGER, mark);
            }
        }
    }
    let zone = zone_at(view.now);
    for (at, item) in &view.pickups {
        ctx.rect(
            Rect::from_center_size(*at, Vec2::ONE),
            palette::PICKUP,
            Depth::layer(layers::PLAY),
        );
        let letter = style(0.8, Color::BLACK, layers::PLAY, 1.0);
        let corner = *at - Vec2::new(letter.width_of(item.letter()) * 0.5, 0.4);
        ctx.text(corner, item.letter(), letter);
    }
    for ball in &view.balls {
        let color = GOLFERS[usize::from(ball.owner)].2;
        ctx.circle(
            ball.pos,
            BALL_RADIUS,
            color,
            Depth {
                layer: layers::PLAY,
                z: 2.0,
            },
        );
    }
    for golfer in view.golfers.iter().filter(|g| g.alive) {
        let mut color = GOLFERS[usize::from(golfer.idx)].2;
        if golfer.stun_left > 0 {
            color.a = 0.5;
        }
        if !inside_zone(zone, golfer.pos) || golfer.outside_ticks > 0 {
            ctx.circle(
                golfer.pos,
                1.0,
                palette::DANGER,
                Depth {
                    layer: layers::PLAY,
                    z: 3.0,
                },
            );
        }
        ctx.circle(
            golfer.pos,
            GOLFER_RADIUS,
            color,
            Depth {
                layer: layers::PLAY,
                z: 4.0,
            },
        );
    }
}

/// The HUD, the legend, and the banner once the match is over.
pub fn draw_hud(ctx: &mut DrawCtx) {
    let view = snap(&ctx.world);
    let hud = style(HUD_SIZE, palette::HUD_TEXT, layers::UI, 0.0);
    for (row, line) in hud_lines(&ctx.world).iter().enumerate() {
        ctx.text(HUD_AT + Vec2::new(0.0, row as f32 * HUD_SIZE), line, hud);
    }
    let legend = style(LEGEND_SIZE, palette::HUD_TEXT, layers::UI, 0.0);
    ctx.text(
        Vec2::new(-legend.width_of(LEGEND) * 0.5, LEGEND_Y),
        LEGEND,
        legend,
    );
    let Some(outcome) = &view.result else { return };
    ctx.rect(
        BANNER_BACK_RECT,
        palette::BANNER_BACK,
        Depth {
            layer: layers::UI,
            z: 1.0,
        },
    );
    let banner = style(BANNER_SIZE, palette::HUD_TEXT, layers::UI, 2.0);
    for (line, y) in result_lines(outcome).iter().zip(BANNER_Y) {
        ctx.text(Vec2::new(-banner.width_of(line) * 0.5, y), line, banner);
    }
}
