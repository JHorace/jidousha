//! Everything drawn. The strings come from `text.rs`; the rings, the aim and
//! the cue are drawn from the same rule functions the sim applies.

use jidousha::prelude::*;

use crate::rules::{self, COURSE, Contact, PAD_RADIUS};
use crate::sim::{CUP_OPENS, Match};
use crate::text::{grace_left, hint_line, result_lines, status_lines};
use crate::zone::{Circle, next_zone, zone_at};
use crate::{HALF_H, HALF_W, layers};

/// The palette, named once: checks find things by these tints.
pub mod palette {
    use jidousha::prelude::Color;
    /// Off the course.
    pub const ROUGH: Color = Color::rgb(0.05, 0.11, 0.07);
    /// The course.
    pub const FAIRWAY: Color = Color::rgb(0.10, 0.25, 0.13);
    /// The zone as it is now.
    pub const ZONE_RING: Color = Color::rgb(0.95, 0.95, 0.95);
    /// The zone it is heading for.
    pub const NEXT_RING: Color = Color::rgb(1.0, 0.72, 0.2);
    /// The aim line.
    pub const AIM: Color = Color::rgb(0.55, 0.95, 1.0);
    /// Where the shot stops.
    pub const LANDING: Color = Color::rgb(0.3, 1.0, 0.9);
    /// How far a full shot could go.
    pub const REACH_RING: Color = Color::rgba(0.55, 0.95, 1.0, 0.35);
    /// The contact cue.
    pub const CUE: Color = Color::rgb(1.0, 0.35, 0.2);
    /// A cup.
    pub const CUP: Color = Color::rgb(0.02, 0.03, 0.02);
    /// A pad, open.
    pub const PAD_OPEN: Color = Color::rgb(0.3, 1.0, 0.4);
    /// A pad, not yet or no longer open.
    pub const PAD_CLOSED: Color = Color::rgba(0.6, 0.6, 0.6, 0.5);
    /// A pickup.
    pub const PICKUP: Color = Color::rgb(0.85, 0.75, 1.0);
    /// The grace countdown.
    pub const OUT: Color = Color::rgb(1.0, 0.25, 0.25);
    /// A stun.
    pub const STUN: Color = Color::rgb(1.0, 0.95, 0.3);
    /// HUD text.
    pub const TEXT: Color = Color::rgb(0.95, 0.97, 0.95);
    /// The bands behind HUD text and the result card.
    pub const PANEL: Color = Color::rgba(0.0, 0.0, 0.0, 0.75);
    /// One colour per golfer, the player's first.
    pub const GOLFERS: [Color; 6] = [
        Color::rgb(0.3, 0.9, 1.0),
        Color::rgb(1.0, 0.45, 0.3),
        Color::rgb(1.0, 0.5, 0.8),
        Color::rgb(0.7, 0.55, 1.0),
        Color::rgb(1.0, 0.85, 0.3),
        Color::rgb(0.6, 1.0, 0.35),
    ];
}

/// What each golfer is called on screen, the player's first.
pub const NAMES: [&str; 6] = ["YOU", "RED", "PINK", "VIOLET", "GOLD", "LIME"];

/// How many straight pieces a ring is drawn with.
pub const RING_SEGMENTS: usize = 64;

/// HUD text size, in world units.
pub const HUD_SIZE: f32 = 0.9;

/// The top of the status bar's first line.
pub const STATUS_TOP: f32 = -HALF_H + 0.35;

/// The top of the hint line, under the course.
pub const HINT_TOP: f32 = COURSE.max.y + 0.55;

/// The pieces a ring is drawn with: every one whose ends are both on the
/// course, and only every other one when `dashed`.
pub fn ring_segments(circle: Circle, dashed: bool) -> Vec<(Vec2, Vec2)> {
    let mut pieces = Vec::new();
    let point = |index: usize| {
        let angle = Radians(index as f32 / RING_SEGMENTS as f32 * core::f32::consts::TAU);
        let (sin, cos) = sin_cos(angle);
        circle.center + Vec2::new(cos, sin) * circle.radius
    };
    for index in 0..RING_SEGMENTS {
        if dashed && index % 2 == 1 {
            continue;
        }
        let (from, to) = (point(index), point(index + 1));
        if COURSE.contains_rect(Rect::from_center_size(from, Vec2::ZERO))
            && COURSE.contains_rect(Rect::from_center_size(to, Vec2::ZERO))
        {
            pieces.push((from, to));
        }
    }
    pieces
}

fn ring(
    ctx: &mut DrawCtx,
    circle: Circle,
    dashed: bool,
    thickness: f32,
    color: Color,
    depth: Depth,
) {
    for (from, to) in ring_segments(circle, dashed) {
        ctx.line(from, to, thickness, color, depth);
    }
}

/// A label centred over `at`, kept on screen.
fn label(ctx: &mut DrawCtx, at: Vec2, text: &str, color: Color) {
    let style = TextStyle {
        size: 0.6,
        color,
        depth: Depth::layer(layers::GUIDES),
        ..TextStyle::default()
    };
    let width = style.width_of(text);
    let x = (at.x - width * 0.5).clamp(-HALF_W + 0.2, HALF_W - 0.2 - width);
    ctx.text(Vec2::new(x, at.y), text, style);
}

/// The course, its cups, pads and pickups, and the zone.
pub fn draw_the_course(ctx: &mut DrawCtx) {
    let game = ctx.world.resource::<Match>();
    ctx.rect(COURSE, palette::FAIRWAY, Depth::layer(layers::COURSE));
    let fixtures = Depth::layer(layers::FIXTURES);
    for (index, (cup, taken_by)) in game.cups.into_iter().zip(game.cup_taken_by).enumerate() {
        let opens = CUP_OPENS[index] * 60;
        if taken_by.is_none() && game.tick < opens {
            // Not yet: a grey cup and the time it opens.
            ctx.circle(cup, 0.45, palette::PAD_CLOSED, fixtures);
            let seconds = opens / 60;
            label(
                ctx,
                cup + Vec2::new(0.0, 0.6),
                &format!("{}:{:02}", seconds / 60, seconds % 60),
                palette::PAD_CLOSED,
            );
            continue;
        }
        if let Some(who) = taken_by {
            // Closed: filled in the colour of whoever took it, no flag.
            ctx.circle(
                cup,
                0.45,
                palette::GOLFERS[who].modulate(palette::FAIRWAY),
                fixtures,
            );
            continue;
        }
        ctx.circle(cup, 0.45, palette::CUP, fixtures);
        ctx.line(
            cup,
            cup + Vec2::new(0.0, -1.6),
            0.08,
            palette::TEXT,
            fixtures,
        );
        ctx.rect(
            Rect::from_min_size(cup + Vec2::new(0.0, -1.6), Vec2::new(0.7, 0.45)),
            palette::NEXT_RING.modulate(Color::rgb(1.0, 0.6, 0.6)),
            fixtures,
        );
    }
    for (index, pad) in game.pads.iter().enumerate() {
        let color = if game.pad_open(index) {
            palette::PAD_OPEN
        } else {
            palette::PAD_CLOSED
        };
        let box_of = Rect::from_center_size(*pad, Vec2::splat(PAD_RADIUS * 2.0));
        let corners = [
            box_of.min,
            Vec2::new(box_of.max.x, box_of.min.y),
            box_of.max,
            Vec2::new(box_of.min.x, box_of.max.y),
        ];
        for side in 0..4 {
            ctx.line(
                corners[side],
                corners[(side + 1) % 4],
                0.15,
                color,
                fixtures,
            );
        }
        label(ctx, *pad + Vec2::new(0.0, PAD_RADIUS + 0.1), "EXIT", color);
    }
    for pickup in game.pickups.iter().filter(|pickup| !pickup.taken) {
        ctx.rect(
            Rect::from_center_size(pickup.pos, Vec2::splat(0.7)),
            palette::PICKUP,
            fixtures,
        );
        label(
            ctx,
            pickup.pos + Vec2::new(0.0, 0.5),
            pickup.item.name(),
            palette::PICKUP,
        );
    }
    let zone = Depth::layer(layers::ZONE);
    ring(
        ctx,
        zone_at(&game.schedule, game.tick),
        false,
        0.18,
        palette::ZONE_RING,
        zone,
    );
    if let Some((next, _)) = next_zone(&game.schedule, game.tick) {
        ring(ctx, next, true, 0.14, palette::NEXT_RING, zone);
    }
}

/// Balls, golfers, and what is wrong with them.
pub fn draw_the_play(ctx: &mut DrawCtx) {
    let game = ctx.world.resource::<Match>();
    let play = Depth::layer(layers::PLAY);
    for (who, golfer) in game.golfers.iter().enumerate() {
        if golfer.ending.is_some() {
            continue;
        }
        let color = palette::GOLFERS[who];
        ctx.circle(golfer.ball.pos, 0.42, color, play);
        ctx.circle(
            golfer.ball.pos,
            0.28,
            palette::TEXT,
            Depth {
                layer: layers::PLAY,
                z: 1.0,
            },
        );
        ctx.circle(
            golfer.pos,
            0.6,
            color,
            Depth {
                layer: layers::PLAY,
                z: 2.0,
            },
        );
        let mut above = golfer.pos + Vec2::new(0.0, -1.5);
        label(ctx, above, NAMES[who], color);
        if golfer.out_ticks > 0 {
            above.y -= 0.65;
            label(
                ctx,
                above,
                &format!("OUT {:.1}s", grace_left(golfer.out_ticks)),
                palette::OUT,
            );
        }
        if golfer.stun > 0 {
            above.y -= 0.65;
            label(
                ctx,
                above,
                &format!("STUN {:.1}", golfer.stun as f32 / 60.0),
                palette::STUN,
            );
        }
    }
}

/// The player's aim while addressing the ball, and the contact cue.
pub fn draw_the_guides(ctx: &mut DrawCtx) {
    let game = ctx.world.resource::<Match>();
    if game.over() {
        return;
    }
    let me = &game.golfers[0];
    let guides = Depth::layer(layers::GUIDES);
    if game.addressing(0) && rules::free_to_act(me) {
        let landing = rules::landing(me.ball.pos, me.aim, me.power, me.item);
        ctx.line(me.ball.pos, landing, 0.12, palette::AIM, guides);
        ctx.circle(landing, 0.32, palette::LANDING, guides);
        let area = rules::spread_radius(me.power, me.item);
        if area > 0.4 {
            ring(
                ctx,
                Circle {
                    center: landing,
                    radius: area,
                },
                false,
                0.06,
                palette::LANDING,
                guides,
            );
        }
        let reach = rules::REACH * rules::effect(me.item).reach_scale;
        ring(
            ctx,
            Circle {
                center: me.ball.pos,
                radius: reach,
            },
            true,
            0.06,
            palette::REACH_RING,
            guides,
        );
    }
    if let Some(contact) = rules::contact_for(game, 0) {
        let at = match contact {
            Contact::Club { target, .. } => game.golfers[target].pos,
            Contact::Strike { owner, .. } => game.golfers[owner].ball.pos,
        };
        ring(
            ctx,
            Circle {
                center: at,
                radius: 1.0,
            },
            false,
            0.12,
            palette::CUE,
            guides,
        );
    }
}

/// The status bar, the hint line and, at the end, the result card.
pub fn draw_the_hud(ctx: &mut DrawCtx) {
    let game = ctx.world.resource::<Match>();
    let hud = Depth::layer(layers::HUD);
    let style = TextStyle {
        size: HUD_SIZE,
        color: palette::TEXT,
        depth: Depth {
            layer: layers::HUD,
            z: 1.0,
        },
        ..TextStyle::default()
    };
    ctx.rect(
        Rect {
            min: Vec2::new(-HALF_W, -HALF_H),
            max: Vec2::new(HALF_W, COURSE.min.y - 0.1),
        },
        palette::PANEL,
        hud,
    );
    for (row, line) in status_lines(game).iter().enumerate() {
        ctx.text(
            Vec2::new(-HALF_W + 0.5, STATUS_TOP + row as f32 * HUD_SIZE * 1.15),
            line,
            style,
        );
    }
    let hint = hint_line(game);
    ctx.text(
        Vec2::new(-style.width_of(&hint) * 0.5, HINT_TOP),
        &hint,
        TextStyle {
            color: palette::NEXT_RING,
            ..style
        },
    );
    let lines = result_lines(game);
    if lines.is_empty() {
        return;
    }
    let card = Rect::from_center_size(Vec2::ZERO, Vec2::new(40.0, 16.0));
    ctx.rect(card, palette::PANEL, Depth::layer(layers::RESULT));
    let big = TextStyle {
        size: 1.0,
        color: palette::TEXT,
        depth: Depth {
            layer: layers::RESULT,
            z: 1.0,
        },
        ..TextStyle::default()
    };
    for (row, line) in lines.iter().enumerate() {
        ctx.text(
            Vec2::new(card.min.x + 1.5, card.min.y + 1.0 + row as f32 * 1.15),
            line,
            big,
        );
    }
}
