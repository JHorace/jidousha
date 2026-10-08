//! The safe zone: six nested circles and the schedule that shrinks one into
//! the next.
//!
//! `zone_at` is the one zone-membership-at-time function (DESIGN.md, decision
//! row 1). The ring drawn on screen, the next ring, the grace countdown and
//! elimination all read it, so the picture and the rule cannot disagree.

use jidousha::prelude::*;

/// Ticks per second, the engine's default fixed timestep.
pub const TICKS_PER_SECOND: u64 = 60;

/// The radius of each circle, widest first. The first covers the whole
/// course from its centre; the last is a point.
pub const RADII: [f32; 6] = [36.0, 24.0, 15.0, 9.0, 4.0, 0.0];

/// How long each circle holds before it starts to shrink, in seconds.
pub const HOLD_SECONDS: [u64; 5] = [20, 15, 15, 12, 10];

/// How long each shrink takes, in seconds.
pub const SHRINK_SECONDS: [u64; 5] = [12, 10, 10, 8, 8];

/// A circle on the course.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    /// Where it is centred.
    pub center: Vec2,
    /// How far it reaches.
    pub radius: f32,
}

impl Circle {
    /// Whether `point` is inside, edge included.
    pub fn contains(self, point: Vec2) -> bool {
        (point - self.center).length() <= self.radius
    }
}

/// The circles one match shrinks through, drawn from its seed.
#[derive(Clone, Debug)]
pub struct Schedule {
    /// The six circles, widest first.
    pub circles: [Circle; 6],
}

impl Schedule {
    /// Six nested circles: each next centre sits inside the current circle by
    /// at most the difference of the radii, so the next circle is inside the
    /// current one.
    pub fn draw(rng: &mut Rng) -> Self {
        let mut circles = [Circle {
            center: Vec2::ZERO,
            radius: RADII[0],
        }; 6];
        for index in 1..6 {
            let previous = circles[index - 1];
            // Not the full difference: a circle tangent to its parent from the
            // inside would leave nobody time to cross the course.
            let slack = (previous.radius - RADII[index]) * 0.6;
            let angle = Radians(rng.next_f32() * core::f32::consts::TAU);
            let (sin, cos) = sin_cos(angle);
            let distance = slack * rng.next_f32();
            let mut center = previous.center + Vec2::new(cos, sin) * distance;
            // Keep the playable circles over the course, which is wider than
            // it is tall.
            center.y = center.y.clamp(-8.0, 8.0);
            center.x = center.x.clamp(-20.0, 20.0);
            if (center - previous.center).length() > previous.radius - RADII[index] {
                center = previous.center;
            }
            circles[index] = Circle {
                center,
                radius: RADII[index],
            };
        }
        Self { circles }
    }
}

/// Where phase `phase` (0..5) starts to shrink, and where it stops, in ticks.
pub fn shrink_window(phase: usize) -> (u64, u64) {
    let mut start = 0;
    for index in 0..phase {
        start += (HOLD_SECONDS[index] + SHRINK_SECONDS[index]) * TICKS_PER_SECOND;
    }
    let begins = start + HOLD_SECONDS[phase] * TICKS_PER_SECOND;
    (begins, begins + SHRINK_SECONDS[phase] * TICKS_PER_SECOND)
}

/// The zone on `tick`. The one function everything about the zone reads.
pub fn zone_at(schedule: &Schedule, tick: u64) -> Circle {
    for phase in 0..5 {
        let (begins, ends) = shrink_window(phase);
        if tick < begins {
            return schedule.circles[phase];
        }
        if tick < ends {
            let t = (tick - begins) as f32 / (ends - begins) as f32;
            let from = schedule.circles[phase];
            let to = schedule.circles[phase + 1];
            return Circle {
                center: from.center.lerp(to.center, t),
                radius: from.radius + (to.radius - from.radius) * t,
            };
        }
    }
    schedule.circles[5]
}

/// Whether `pos` is safe on `tick`.
pub fn inside_at(schedule: &Schedule, pos: Vec2, tick: u64) -> bool {
    zone_at(schedule, tick).contains(pos)
}

/// What the zone is heading for, and on which tick it gets there: the end of
/// the shrink under way, or of the next one if the zone is holding. `None`
/// once the zone has closed.
pub fn next_zone(schedule: &Schedule, tick: u64) -> Option<(Circle, u64)> {
    for phase in 0..5 {
        let (_, ends) = shrink_window(phase);
        if tick < ends {
            return Some((zone_at(schedule, ends), ends));
        }
    }
    None
}

/// Whether the zone is shrinking on `tick`, and if not, the tick it starts.
pub fn shrinking_or_starts(tick: u64) -> Result<(), u64> {
    for phase in 0..5 {
        let (begins, ends) = shrink_window(phase);
        if tick < begins {
            return Err(begins);
        }
        if tick < ends {
            return Ok(());
        }
    }
    Ok(())
}
