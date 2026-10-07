//! Every sentence the game draws, as free functions over the read-only world.
//!
//! Key types: `Look`. Key functions: `look`, `status_line_1`, `status_line_2`,
//! `cue_line`, `pickup_line`, `result_lines`, `aiming`, `aim_landing`.
//! Depends on: `contact`, `items`, `outcome`, `shots`, `systems`, `world`, `zone`.
//! Never depended on outside the game.
//! INVARIANT: the verify run and the draw systems read the same strings, and every
//! literal is printable ASCII, because the built-in font draws anything else as a box.

use jidousha::prelude::*;

use crate::contact::{Contact, PlayerSnap, contact_target, snap_players};
use crate::outcome::{
    EXTRACT_TICKS, Outcome, PAD_RADIUS, PadState, Placing, kept_on, kept_text, standing,
};
use crate::shots::{MAX_SHOT, REACH_BALL, aim_point, flight_ticks};
use crate::systems::WALK_SPEED;
use crate::world::{Course, MatchState, Pickup, Player, Pointer};
use crate::zone::{GRACE_TICKS, landing_safe, next_stop, shrink_begins, zone_at};

/// How far from a pickup its label starts to read what it does.
pub const LABEL_RADIUS: f32 = 2.5;

/// Everything the strings read, taken once from the world.
pub struct Look {
    /// The clock.
    pub tick: u64,
    /// Every golfer in play.
    pub snaps: Vec<PlayerSnap>,
    /// The human, if still in play.
    pub me: Option<PlayerSnap>,
    /// The human's body component, if still in play.
    pub player: Option<Player>,
    /// The course.
    pub course: Course,
    /// The pointer in world units.
    pub pointer: Vec2,
    /// Whether the match is on.
    pub state: MatchState,
}

/// The one reader.
pub fn look(view: &WorldView<'_>) -> Look {
    let snaps = snap_players(view);
    Look {
        tick: view.resource::<Time>().tick,
        me: snaps.iter().find(|snap| snap.index == 0).copied(),
        player: view
            .query::<&Player>()
            .find(|(_, player)| player.index == 0)
            .map(|(_, player)| *player),
        snaps,
        course: *view.resource::<Course>(),
        pointer: view.find_resource::<Pointer>().map_or(Vec2::ZERO, |p| p.0),
        state: *view.resource::<MatchState>(),
    }
}

/// How a golfer is named on screen.
pub fn tag_of(index: usize) -> String {
    if index == 0 {
        "YOU".to_owned()
    } else {
        format!("P{index}")
    }
}

/// Seconds, rounded up, for a count of ticks.
fn seconds_up(ticks: u64) -> u64 {
    ticks.div_ceil(60)
}

/// Whether the human can aim: alive, free, and standing at their resting ball.
pub fn aiming(look: &Look) -> bool {
    let Some(me) = &look.me else { return false };
    matches!(look.state, MatchState::Playing)
        && !me.dazed(look.tick)
        && !me.extracting
        && me.ball_resting
        && (me.ball - me.pos).length() <= REACH_BALL
}

/// Where the human's shot toward the pointer would land, how long it is, and whether
/// that spot is inside the zone by the time the ball has landed and the golfer has
/// walked there.
pub fn aim_landing(look: &Look) -> Option<(Vec2, f32, bool)> {
    let me = look.me.as_ref()?;
    let reach = MAX_SHOT * me.effects.shot_reach;
    let aim = aim_point(me.ball, look.pointer, reach);
    let length = (aim - me.ball).length();
    let walk = (aim - me.pos).length() / (WALK_SPEED * me.effects.walk);
    let arrive = look.tick + flight_ticks(length) + (walk * 60.0).ceil() as u64;
    Some((
        aim,
        length,
        landing_safe(&look.course.schedule, aim, arrive),
    ))
}

/// The top band's first line: the zone, the grace countdown, shots, daze.
pub fn status_line_1(view: &WorldView<'_>) -> String {
    let look = look(view);
    let (Some(me), Some(player)) = (&look.me, &look.player) else {
        return String::new();
    };
    let zone = zone_at(&look.course.schedule, look.tick);
    let mut line = format!("ZONE r{:.1}", zone.radius);
    match next_stop(&look.course.schedule, look.tick) {
        Some((stop, next)) => {
            let begins = shrink_begins(stop);
            if look.tick < begins {
                line += &format!(
                    " -> r{:.1} in {}s",
                    next.radius,
                    seconds_up(begins - look.tick)
                );
            } else {
                line += &format!(
                    " -> r{:.1} closing {}s",
                    next.radius,
                    seconds_up(stop - look.tick)
                );
            }
        }
        None => line += " final",
    }
    if player.exposure > 0 {
        let left = GRACE_TICKS.saturating_sub(player.exposure) as f32 / 60.0;
        line += &format!("   OUT {left:.1}s");
    }
    line += &format!("   SHOTS {}", player.shots);
    if me.dazed(look.tick) {
        line += &format!(
            "   DAZED {:.1}s",
            (me.dazed_until - look.tick) as f32 / 60.0
        );
    }
    line
}

/// The top band's second line: what is held, what extracting keeps, how close each end is.
pub fn status_line_2(view: &WorldView<'_>) -> String {
    let look = look(view);
    let Some(me) = &look.me else {
        return String::new();
    };
    let pad = look.course.pad_state(look.tick);
    let st = standing(&look.snaps, 0, look.course.hole, look.tick, pad);
    let pad_text = match st.pad {
        PadState::Closed { opens_at } => format!("opens in {}s", seconds_up(opens_at - look.tick)),
        PadState::Open {
            closes_at: Some(closes),
        } => format!("open {}s", seconds_up(closes.saturating_sub(look.tick))),
        PadState::Open { closes_at: None } => "open".to_owned(),
        PadState::Gone => "gone".to_owned(),
    };
    let hole_text = if st.hole_open {
        "open".to_owned()
    } else {
        format!(
            "opens in {}s",
            seconds_up(crate::outcome::HOLE_OPENS - look.tick)
        )
    };
    format!(
        "HOLD {}  EXTRACT {} keeps {}  HOLE {:.1} {}  RIVALS {}  PIN #{}",
        kept_text(&me.kit),
        pad_text,
        kept_text(&kept_on(Outcome::Extracted, &me.kit)),
        st.hole_distance,
        hole_text,
        st.rivals_left,
        st.pin_rank,
    )
}

/// The bottom band's first line: what the next input does.
pub fn cue_line(view: &WorldView<'_>) -> String {
    let look = look(view);
    let Some(me) = &look.me else {
        return String::new();
    };
    if !matches!(look.state, MatchState::Playing) {
        return String::new();
    }
    if me.dazed(look.tick) {
        return "DAZED".to_owned();
    }
    if let Some(player) = &look.player
        && let Some(since) = player.extracting_since
    {
        let left = (EXTRACT_TICKS - (look.tick - since).min(EXTRACT_TICKS)) as f32 / 60.0;
        return format!("EXTRACTING {left:.1}s");
    }
    match contact_target(me, &look.snaps, look.tick) {
        Some(Contact::Club {
            who,
            daze_ticks,
            drops,
        }) => {
            return format!(
                "SPACE: club {} (dazes {:.1}s, drops {})",
                tag_of(who),
                daze_ticks as f32 / 60.0,
                drops.map_or("nothing", |item| item.name())
            );
        }
        Some(Contact::Strike { whose, lands_at }) => {
            let theirs = look.snaps.iter().find(|snap| snap.index == whose);
            let from_hole = theirs.map_or(0.0, |s| (s.ball - look.course.hole).length());
            let to_hole = (lands_at - look.course.hole).length();
            let direction = if to_hole > from_hole {
                "away from"
            } else {
                "toward"
            };
            let distance = theirs.map_or(0.0, |s| (lands_at - s.ball).length());
            return format!(
                "SPACE: strike {} ball (knocks it {distance:.1} {direction} the hole)",
                tag_of(whose)
            );
        }
        None => {}
    }
    let pad = look.course.pad_state(look.tick);
    let on_pad = |p: Vec2| (p - look.course.pad).length() <= PAD_RADIUS;
    if matches!(pad, PadState::Open { .. }) && on_pad(me.pos) && on_pad(me.ball) {
        return format!(
            "E: extract (keeps {})",
            kept_text(&kept_on(Outcome::Extracted, &me.kit))
        );
    }
    if aiming(&look)
        && let Some((_, length, safe)) = aim_landing(&look)
    {
        let verdict = if safe {
            "lands in zone"
        } else {
            "lands OUTSIDE the zone"
        };
        return format!("CLICK: shoot {length:.1} -> {verdict}");
    }
    format!(
        "walk to your ball ({:.1} away)",
        (me.ball - me.pos).length()
    )
}

/// The bottom band's second line: what the nearest pickup does, and what it replaces.
pub fn pickup_line(view: &WorldView<'_>) -> String {
    let look = look(view);
    let Some(me) = &look.me else {
        return String::new();
    };
    let nearest = view
        .query::<(&Transform, &Pickup)>()
        .map(|(_, transform, pickup)| ((transform.pos - me.pos).length(), pickup.0))
        .filter(|(distance, _)| *distance <= LABEL_RADIUS)
        .min_by(|a, b| a.0.total_cmp(&b.0));
    let Some((_, item)) = nearest else {
        return String::new();
    };
    let mut line = format!("TAKE {}: {}", item.name(), item.describe());
    if let Some(held) = me.kit.in_slot(item.slot()) {
        line += &format!(" (replaces {})", held.name());
    }
    line
}

/// The result screen's three lines.
pub fn result_lines(view: &WorldView<'_>) -> [String; 3] {
    let MatchState::Over {
        outcome,
        kept,
        placing,
    } = *view.resource::<MatchState>()
    else {
        return [String::new(), String::new(), String::new()];
    };
    let headline = match outcome {
        Outcome::Extracted => "EXTRACTED",
        Outcome::HoledOut => "HOLED OUT",
        Outcome::LastStanding => "LAST STANDING",
        Outcome::ClosestToPin => "CLOSEST TO PIN",
        Outcome::Eliminated => "ELIMINATED",
        Outcome::Lost { .. } => "LOST",
    };
    let place = match placing {
        Placing::Won => "placed 1 of 6".to_owned(),
        Placing::Out { place } => format!("placed {place} of 6"),
        Placing::Left { still_in } => format!("left with {still_in} still in"),
        Placing::Lost { to } => format!("P{to} holed out first"),
    };
    [
        headline.to_owned(),
        format!("kept: {}", kept_text(&kept)),
        place,
    ]
}
