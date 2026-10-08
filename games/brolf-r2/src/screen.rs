//! The screen: the always-visible status panel, the aim preview, the result
//! overlay, and the three Draw systems.
//!
//! Every row of text is chrome in the UI kit's `Panel`, laid out in a 960x540
//! design space that rides the camera, so the floors judge one list and the
//! frame check finds the same list on the recorded frame. Rows have fixed
//! slots: an absent row is simply not emitted, so no two rows can collide.

use jidousha::prelude::*;
use jidousha::ui::{Cell, Floors, Icon, Mapping, Panel, TextRun};

use crate::TURF;
use crate::layers;
use crate::rules::{
    BALL_RADIUS, CUP_RADIUS, Circle, Fate, GOLFER_RADIUS, GRACE_TICKS, Item, Landing, SLOTS,
    course, effects, landing_of, phase_line, phase_of, polar, ring_segments, shot_speed_for, spec,
    walk_ticks, zone_at,
};
use crate::sim::{Match, Snapshot, read_snapshot};

/// The design space chrome is laid out in.
pub const DESIGN: Vec2 = Vec2::new(960.0, 540.0);

/// The floors: 12-unit text, chrome inside the design rect, map labels on the course.
pub const FLOORS: Floors = Floors {
    min_text: 12.0,
    chrome: Rect {
        min: Vec2::ZERO,
        max: DESIGN,
    },
    world: Rect {
        min: crate::rules::COURSE_MIN,
        max: crate::rules::COURSE_MAX,
    },
};

/// The game draws no icons; this is the kit's art vocabulary, empty.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Art {}

impl Icon for Art {
    fn size_at(self, _scale: f32) -> Vec2 {
        match self {}
    }
}

pub const DIM: Color = Color::rgb(0.62, 0.66, 0.60);
pub const INK: Color = Color::rgb(0.93, 0.93, 0.88);
pub const GOLD: Color = Color::rgb(0.95, 0.78, 0.30);
pub const WARN: Color = Color::rgb(0.95, 0.40, 0.30);
pub const OK: Color = Color::rgb(0.45, 0.90, 0.55);
pub const ORANGE: Color = Color::rgb(1.0, 0.55, 0.15);
pub const LANDING_IN: Color = OK;
pub const LANDING_OUT: Color = WARN;
pub const LINE: Color = Color::rgba(1.0, 1.0, 1.0, 0.35);
pub const CUP: Color = Color::rgb(0.05, 0.07, 0.06);
pub const CUP_RIM: Color = Color::rgb(0.9, 0.9, 0.85);
pub const PAD_SHUT: Color = DIM;
pub const ZONE_NOW: Color = Color::rgba(1.0, 1.0, 1.0, 0.9);
pub const ZONE_NEXT: Color = Color::rgba(0.45, 0.70, 1.0, 0.8);
pub const OVERLAY_DIM: Color = Color::rgba(0.0, 0.0, 0.0, 0.75);
pub const SPIKES_TINT: Color = Color::rgb(0.35, 0.85, 0.95);
pub const HEAVY_TINT: Color = Color::rgb(0.70, 0.70, 0.72);

/// The seats' names and colours: the player white, the NPCs orange, blue, violet.
pub const NAMES: [&str; 4] = ["YOU", "N1", "N2", "N3"];
pub const SEAT_COLORS: [Color; 4] = [
    Color::rgb(0.96, 0.96, 0.96),
    Color::rgb(0.95, 0.55, 0.20),
    Color::rgb(0.35, 0.65, 1.0),
    Color::rgb(0.80, 0.40, 0.90),
];

/// The colour a pickup of `item` is drawn in.
pub fn item_tint(item: Item) -> Color {
    match item {
        Item::Driver => GOLD,
        Item::Spikes => SPIKES_TINT,
        Item::Heavy => HEAVY_TINT,
        Item::Helmet => OK,
    }
}

/// The design rect fitted inside the view, centred — the chrome rides the camera.
pub struct UiMap {
    pub origin: Vec2,
    pub scale: f32,
}

impl UiMap {
    pub fn for_camera(camera: &Camera) -> Self {
        let view = camera.visible_bounds();
        let scale = (view.size().x / DESIGN.x).min(view.size().y / DESIGN.y);
        Self {
            origin: view.center() - DESIGN * (scale * 0.5),
            scale,
        }
    }
}

impl Mapping for UiMap {
    fn to_world(&self, ui: Vec2) -> Vec2 {
        self.origin + ui * self.scale
    }
    fn scale(&self) -> f32 {
        self.scale
    }
}

/// Size-12 chrome text in `color`.
pub fn small(color: Color) -> TextStyle {
    TextStyle {
        size: 12.0,
        color,
        depth: Depth::layer(layers::CHROME),
        ..TextStyle::default()
    }
}

/// The status column's left edge and width, in design units.
pub const PANEL_X: f32 = 690.0;
pub const PANEL_W: f32 = 260.0;

/// Row `i` of the status column.
fn row(i: usize) -> Cell {
    Cell::new(Vec2::new(PANEL_X, 52.0 + 16.0 * i as f32), PANEL_W)
}

/// `ticks` as seconds with one decimal.
fn secs(ticks: u64) -> String {
    format!("{:.1}s", ticks as f32 / 60.0)
}

/// `tick` as a match clock, `m:ss`.
fn clock(tick: u64) -> String {
    let seconds = tick / 60;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// What the aim line shows: the ball it starts from, where the strike would
/// stop at the current charge (a full one before charging begins), and
/// whether the zone will still hold that spot when the golfer gets there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Preview {
    pub from: Vec2,
    /// Whose ball it is.
    pub owner: usize,
    pub landing: Landing,
    pub in_zone_at_arrival: bool,
}

/// The aim preview for `seat`: `Some` exactly when a ball is in strike reach.
pub fn aim_preview(snap: &Snapshot, seat: usize) -> Option<Preview> {
    let me = snap.golfer(seat);
    let owner = snap.reach(seat).ball?;
    let from = snap.ball(owner).pos;
    let charge = if me.charge == 0 { 60 } else { me.charge };
    let speed = shot_speed_for(
        &effects(&me.held),
        &effects(&snap.golfer(owner).held),
        owner == seat,
        charge,
    );
    let landing = landing_of(from, polar(1.0, me.aim), speed, course(), snap.dt);
    let walk = walk_ticks(
        landing.at.distance(me.pos),
        effects(&me.held).walk_scale,
        snap.dt,
    );
    let arrival = zone_at(&snap.course, snap.tick + landing.ticks + walk);
    Some(Preview {
        from,
        owner,
        landing,
        in_zone_at_arrival: arrival.contains(landing.at),
    })
}

/// One NPC's row: its state and what it banks.
fn seat_line(snap: &Snapshot, seat: usize) -> String {
    let golfer = snap.golfer(seat);
    let name = NAMES[seat];
    let bank = snap.banked(seat);
    match golfer.fate {
        Fate::Playing if golfer.out_ticks > 0 => {
            format!("{name} OUT {}", secs(GRACE_TICKS - golfer.out_ticks))
        }
        Fate::Playing => format!("{name} playing {bank}"),
        Fate::Stunned { until } => {
            format!("{name} stunned {}", secs(until.saturating_sub(snap.tick)))
        }
        Fate::Extracted { .. } => format!("{name} extracted {bank}"),
        Fate::Survived { .. } => format!("{name} survived {bank}"),
        Fate::Eliminated { .. } => format!("{name} eliminated"),
    }
}

/// The always-visible match status (decision row 4), carrying rows 1-3's
/// surfaces too: the zone and its countdown, the aim and where it lands, who
/// is in club reach, what a pickup does, and what extracting keeps.
pub fn status_panel(snap: &Snapshot, seat: usize) -> Panel<Art> {
    let mut panel = Panel::default();
    let mut put = |i: usize, text: &str, color: Color| {
        panel.text(row(i).run(text, small(color)));
    };
    let me = snap.golfer(seat);
    let (k, _) = phase_of(snap.tick);
    let zone_word = if k >= 4 {
        "zone closed".to_owned()
    } else {
        format!("zone {}/4", k + 1)
    };
    put(0, &format!("BROLF {} {zone_word}", clock(snap.tick)), INK);
    put(1, &phase_line(snap.tick), DIM);
    put(
        2,
        &format!("you: {} pts bank now {}", me.points, snap.banked(seat)),
        INK,
    );
    let slot = |i: usize| me.held.get(i).map_or("-", |item| spec(*item).name);
    put(3, &format!("held: {}, {}", slot(0), slot(1)), INK);
    match me.fate {
        Fate::Extracted { .. } => put(4, &format!("extracted {}", snap.banked(seat)), GOLD),
        Fate::Survived { .. } => put(4, &format!("survived {}", snap.banked(seat)), GOLD),
        Fate::Eliminated { .. } => put(4, "eliminated", WARN),
        _ if me.out_ticks > 0 => put(
            4,
            &format!("zone: OUT {}", secs(GRACE_TICKS - me.out_ticks)),
            WARN,
        ),
        _ => put(4, "zone: IN", OK),
    }
    if snap.pad_open {
        put(5, "pad: OPEN", GOLD);
    } else {
        put(
            5,
            &format!("pad: opens {}", clock(crate::rules::PAD_OPENS_TICK)),
            DIM,
        );
    }
    let alive: Vec<usize> = (0..4).filter(|s| snap.golfer(*s).fate.alive()).collect();
    let lead = alive
        .iter()
        .copied()
        .fold(None, |best: Option<usize>, s| match best {
            Some(b) if snap.banked(b) >= snap.banked(s) => Some(b),
            _ => Some(s),
        });
    let lead = lead.map_or("lead -".to_owned(), |s| {
        format!("lead {} {}", NAMES[s], snap.banked(s))
    });
    put(6, &format!("alive {} {lead}", alive.len()), DIM);
    if me.fate == Fate::Playing {
        let reach = snap.reach(seat);
        if let Some(preview) = aim_preview(snap, seat) {
            let degrees = me.aim.to_degrees().rem_euclid(360.0).round() as i32;
            let charge = me.charge * 100 / crate::rules::CHARGE_TICKS;
            put(8, &format!("aim: {degrees} deg charge {charge}%"), INK);
            let word = if preview.in_zone_at_arrival {
                "IN"
            } else {
                "OUT"
            };
            let color = if preview.in_zone_at_arrival { OK } else { WARN };
            let distance = preview.landing.at.distance(preview.from);
            put(9, &format!("reach {distance:.1} lands {word}"), color);
            if preview.owner == seat {
                put(10, "Space: strike your ball", INK);
            } else {
                put(
                    10,
                    &format!(
                        "Space: strike {}'s ball +{}",
                        NAMES[preview.owner],
                        crate::rules::SLEDGE_POINTS
                    ),
                    GOLD,
                );
            }
        }
        if me.cooldown > 0 {
            put(11, &format!("club ready in {}", secs(me.cooldown)), DIM);
        } else if let Some(target) = reach.club {
            let victim = snap.golfer(target);
            let stun = effects(&victim.held).stun_ticks;
            put(
                11,
                &format!("F: club {} (stun {})", NAMES[target], secs(stun)),
                ORANGE,
            );
            if let Some(item) = victim.held.last() {
                put(12, &format!("  drops {}", spec(*item).name), ORANGE);
            }
        }
        if let Some(index) = reach.pickup {
            let item = spec(snap.pickups[index].item);
            put(
                13,
                &format!("E: take {}, worth {}", item.name, item.worth),
                INK,
            );
            put(14, &format!("  {}", item.line), DIM);
            if me.held.len() >= SLOTS {
                put(15, &format!("  swaps out {}", spec(me.held[0]).name), WARN);
            }
        }
        let ball = snap.ball(seat);
        if snap.pad_open
            && ball.at_rest
            && snap.course.pad.contains(me.pos)
            && snap.course.pad.contains(ball.pos)
        {
            put(
                16,
                &format!("X: extract, keeps {}", snap.banked(seat)),
                GOLD,
            );
        }
    }
    for (i, npc) in (1..4).enumerate() {
        put(17 + i, &seat_line(snap, npc), SEAT_COLORS[npc]);
    }
    put(21, "WASD walk  arrows aim", DIM);
    put(22, "hold Space, release to hit", DIM);
    panel.text(TextRun::new(Vec2::new(15.0, 14.0), "seed 7", small(DIM)));
    panel
}

/// The result overlay: the verdict and every golfer, best bank first.
pub fn result_panel(snap: &Snapshot) -> Panel<Art> {
    let mut panel = Panel::default();
    let title = TextStyle {
        size: 14.0,
        ..small(GOLD)
    };
    panel.text(TextRun::new(Vec2::new(300.0, 150.0), "RESULT", title));
    let winner = snap.over.and_then(|(_, winner)| winner);
    let verdict = match winner {
        Some(0) => format!("YOU win with {}", snap.banked(0)),
        Some(seat) => format!("{} wins with {}", NAMES[seat], snap.banked(seat)),
        None => "the zone took everyone".to_owned(),
    };
    panel.text(TextRun::new(Vec2::new(300.0, 174.0), verdict, small(INK)));
    let mut order: Vec<usize> = (0..4).collect();
    order.sort_by_key(|seat| (std::cmp::Reverse(snap.banked(*seat)), *seat));
    for (i, seat) in order.into_iter().enumerate() {
        let word = match snap.golfer(seat).fate {
            Fate::Extracted { .. } => "extracted",
            Fate::Survived { .. } => "survived",
            Fate::Eliminated { .. } => "eliminated",
            Fate::Playing | Fate::Stunned { .. } => "playing",
        };
        panel.text(TextRun::new(
            Vec2::new(300.0, 206.0 + 20.0 * i as f32),
            format!("{}  {word}  {}", NAMES[seat], snap.banked(seat)),
            small(SEAT_COLORS[seat]),
        ));
    }
    panel.lifted(layers::OVERLAY)
}

/// The whole screen for `snap`: the status, and the result once it is over —
/// the list the verify run reads and the floors judge.
pub fn whole(snap: &Snapshot) -> Panel<Art> {
    let mut panel = status_panel(snap, 0);
    if snap.over.is_some() {
        panel.absorb(result_panel(snap));
    }
    panel
}

/// The overlay's dimmer, in design units.
pub fn overlay_rect() -> Rect {
    Rect {
        min: Vec2::new(240.0, 120.0),
        max: Vec2::new(720.0, 420.0),
    }
}

/// A ring: a disc in `color` with a turf disc inside it.
fn ring(ctx: &mut DrawCtx, at: Vec2, outer: f32, color: Color, layer: i16, z: f32) {
    ctx.circle(at, outer, color, Depth { layer, z });
    ctx.circle(at, outer - 0.08, TURF, Depth { layer, z: z + 0.2 });
}

/// Play: golfers, their balls, pickups, the reach cues and the aim line.
pub fn draw_play(ctx: &mut DrawCtx) {
    let Some(snap) = read_snapshot(&ctx.world) else {
        return;
    };
    let play = |z: f32| Depth {
        layer: layers::PLAY,
        z,
    };
    let reach = snap.reach(0);
    let player_playing = snap.golfer(0).fate == Fate::Playing;
    for golfer in &snap.golfers {
        if !golfer.fate.alive() {
            continue;
        }
        if player_playing && reach.club == Some(golfer.seat) {
            ring(ctx, golfer.pos, 0.52, ORANGE, layers::PLAY, 0.2);
        } else if golfer.out_ticks > 0 {
            ring(ctx, golfer.pos, 0.52, WARN, layers::PLAY, 0.2);
        }
        ctx.circle(
            golfer.pos,
            GOLFER_RADIUS,
            SEAT_COLORS[golfer.seat],
            play(1.0),
        );
    }
    for ball in &snap.balls {
        if !snap.golfer(ball.owner).fate.alive() {
            continue;
        }
        if player_playing && reach.ball == Some(ball.owner) && ball.owner != 0 {
            ctx.circle(ball.pos, 0.30, ORANGE, play(0.3));
        }
        ctx.circle(ball.pos, BALL_RADIUS, SEAT_COLORS[ball.owner], play(1.5));
    }
    for pickup in &snap.pickups {
        ctx.rect(
            Rect::from_center_size(pickup.pos, Vec2::splat(0.5)),
            item_tint(pickup.item),
            play(0.6),
        );
    }
    if player_playing && let Some(preview) = aim_preview(&snap, 0) {
        let tint = Color {
            a: 0.8,
            ..SEAT_COLORS[0]
        };
        ctx.line(preview.from, preview.landing.at, 0.06, tint, play(0.8));
        let disc = if preview.in_zone_at_arrival {
            LANDING_IN
        } else {
            LANDING_OUT
        };
        ctx.circle(preview.landing.at, 0.22, disc, play(0.9));
    }
}

/// `circle` as `n` line segments `thickness` thick.
fn draw_ring(ctx: &mut DrawCtx, circle: Circle, n: usize, thickness: f32, color: Color) {
    for (from, to) in ring_segments(circle, n) {
        ctx.line(from, to, thickness, color, Depth::layer(layers::FIELD));
    }
}

/// The course: fence, cups, pad, and the current and next zone rings.
///
/// Submitted after `draw_play` on purpose: only the FIELD band can put it
/// behind play, so the band is something a recorded frame can see.
pub fn draw_course(ctx: &mut DrawCtx) {
    let Some(snap) = read_snapshot(&ctx.world) else {
        return;
    };
    let fence = course();
    let corners = [
        fence.min,
        Vec2::new(fence.max.x, fence.min.y),
        fence.max,
        Vec2::new(fence.min.x, fence.max.y),
    ];
    for i in 0..4 {
        ctx.line(
            corners[i],
            corners[(i + 1) % 4],
            0.12,
            LINE,
            Depth::layer(layers::FIELD),
        );
    }
    for cup in snap.course.cups {
        let field = |z: f32| Depth {
            layer: layers::FIELD,
            z,
        };
        ctx.circle(cup, 0.56, CUP_RIM, field(0.1));
        ctx.circle(cup, CUP_RADIUS, CUP, field(0.2));
    }
    let pad = if snap.pad_open { GOLD } else { PAD_SHUT };
    draw_ring(ctx, snap.course.pad, 48, 0.10, pad);
    draw_ring(ctx, snap.zone, 64, 0.08, ZONE_NOW);
    if let Some(next) = snap.next {
        draw_ring(ctx, next, 64, 0.06, ZONE_NEXT);
    }
}

/// The chrome: the status panel, and the result overlay once the match is over.
pub fn draw_chrome(ctx: &mut DrawCtx) {
    let Some(snap) = read_snapshot(&ctx.world) else {
        return;
    };
    let map = UiMap::for_camera(ctx.world.resource::<Camera>());
    status_panel(&snap, 0).draw(ctx, &map, |icon, _| match icon.art {});
    if matches!(ctx.world.find_resource::<Match>(), Some(Match::Over { .. })) {
        let dim = overlay_rect();
        ctx.rect(
            Rect {
                min: map.to_world(dim.min),
                max: map.to_world(dim.max),
            },
            OVERLAY_DIM,
            Depth::layer(layers::OVERLAY),
        );
        result_panel(&snap).draw(ctx, &map, |icon, _| match icon.art {});
    }
}
