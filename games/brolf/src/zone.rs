//! The shrinking safe zone: a six-stop schedule and the one membership-at-time function.
//!
//! Key types: `Zone`, `Schedule`. Key functions: `zone_at`, `next_stop`, `landing_safe`.
//! Depends on: the prelude only. Must never be depended on by: nothing outside the game.
//! INVARIANT: `zone_at` is the only thing that says where the zone is at a tick; the
//! drawn rings, the grace countdown, the pad's window and elimination all read it.

use jidousha::prelude::*;

/// How long a shrink lasts, in ticks.
pub const SHRINK_TICKS: u64 = 900;
/// How long a body or ball may stay outside the zone before elimination, in ticks.
pub const GRACE_TICKS: u64 = 300;
/// `(tick, radius)` of each stop; the last stop's centre is the hole.
pub const STOPS: [(u64, f32); 6] = [
    (0, 16.5),
    (2100, 8.0),
    (3900, 5.0),
    (5700, 3.0),
    (7500, 1.2),
    (9000, 1.2),
];
/// The most a stop's centre may stray from its straight line to the hole.
const JITTER: [f32; 5] = [0.0, 1.5, 0.75, 0.5, 0.0];

/// A circle on the course.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zone {
    /// Where its middle is.
    pub center: Vec2,
    /// How far it reaches.
    pub radius: f32,
}

impl Zone {
    /// Whether `p` is inside or on the edge.
    pub fn contains(&self, p: Vec2) -> bool {
        (p - self.center).length_squared() <= self.radius * self.radius
    }
}

/// Where the zone stops, seeded once per match.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Schedule {
    /// `(tick, centre, radius)`, in order.
    pub stops: [(u64, Vec2, f32); 6],
}

impl Schedule {
    /// The schedule for a course whose hole is at `hole`.
    pub fn seeded(rng: &mut Rng, hole: Vec2) -> Schedule {
        let mut stops = [(0, Vec2::ZERO, 0.0); 6];
        for (k, stop) in stops.iter_mut().enumerate() {
            let centre = if k >= 4 {
                hole
            } else {
                let jitter = JITTER[k];
                let offset = if jitter > 0.0 {
                    let angle = rng.next_f32() * Radians::TAU.as_f32();
                    let length = rng.next_f32() * jitter;
                    rotate(Vec2::X * length, Radians(angle))
                } else {
                    Vec2::ZERO
                };
                hole * (k as f32 / 4.0) + offset
            };
            *stop = (STOPS[k].0, centre, STOPS[k].1);
        }
        Schedule { stops }
    }
}

/// The zone at `tick`: constant through a hold, linear through a shrink.
pub fn zone_at(schedule: &Schedule, tick: u64) -> Zone {
    for k in 1..schedule.stops.len() {
        let (stop_tick, centre, radius) = schedule.stops[k];
        let begins = stop_tick.saturating_sub(SHRINK_TICKS);
        if tick < begins {
            let (_, c, r) = schedule.stops[k - 1];
            return Zone {
                center: c,
                radius: r,
            };
        }
        if tick < stop_tick {
            let (_, c0, r0) = schedule.stops[k - 1];
            let t = (tick - begins) as f32 / SHRINK_TICKS as f32;
            return Zone {
                center: c0.lerp(centre, t),
                radius: r0 + (radius - r0) * t,
            };
        }
    }
    let (_, c, r) = schedule.stops[schedule.stops.len() - 1];
    Zone {
        center: c,
        radius: r,
    }
}

/// The first stop whose tick is after `tick`, with its zone; `None` after the last.
pub fn next_stop(schedule: &Schedule, tick: u64) -> Option<(u64, Zone)> {
    schedule
        .stops
        .iter()
        .find(|(stop_tick, _, _)| *stop_tick > tick)
        .map(|(stop_tick, center, radius)| {
            (
                *stop_tick,
                Zone {
                    center: *center,
                    radius: *radius,
                },
            )
        })
}

/// Whether `landing` is inside the zone at `arrive_tick`.
pub fn landing_safe(schedule: &Schedule, landing: Vec2, arrive_tick: u64) -> bool {
    zone_at(schedule, arrive_tick).contains(landing)
}

/// The first tick at or after `opens` at which the zone leaves `pad`, searched once.
pub fn pad_closes_at(schedule: &Schedule, pad: Vec2, opens: u64) -> Option<u64> {
    let last = schedule.stops[schedule.stops.len() - 1].0;
    (opens..=last).find(|tick| !zone_at(schedule, *tick).contains(pad))
}

/// The tick a shrink toward `stop_tick` begins.
pub fn shrink_begins(stop_tick: u64) -> u64 {
    stop_tick.saturating_sub(SHRINK_TICKS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_zone_stop_lies_inside_the_one_before_it_for_forty_seeds() {
        for seed in 0..40 {
            let mut rng = Rng::from_seed(seed);
            let angle = rng.next_f32() * Radians::TAU.as_f32();
            let hole = rotate(Vec2::X * (1.5 + 1.5 * rng.next_f32()), Radians(angle));
            let schedule = Schedule::seeded(&mut rng, hole);
            for pair in schedule.stops.windows(2) {
                let (_, c0, r0) = pair[0];
                let (_, c1, r1) = pair[1];
                assert!(
                    (c1 - c0).length() + r1 <= r0 + 1e-3,
                    "seed {seed}: stop at {c1:?} r{r1} leaves the one at {c0:?} r{r0}"
                );
            }
            assert_eq!(
                schedule.stops[5].1, hole,
                "seed {seed}: the last stop is the hole"
            );
        }
    }

    #[test]
    fn zone_at_is_constant_through_a_hold_and_linear_through_a_shrink() {
        let mut rng = Rng::from_seed(3);
        let schedule = Schedule::seeded(&mut rng, Vec2::new(2.0, 1.0));
        assert_eq!(zone_at(&schedule, 0), zone_at(&schedule, 1200));
        assert_eq!(zone_at(&schedule, 2100), zone_at(&schedule, 3000));
        let mid = zone_at(&schedule, 1650);
        assert!(
            (mid.radius - 12.25).abs() < 1e-3,
            "halfway 16.5 -> 8.0, got {}",
            mid.radius
        );
        let quarter = zone_at(&schedule, 1425);
        assert!(
            (quarter.radius - 14.375).abs() < 1e-3,
            "got {}",
            quarter.radius
        );
        assert_eq!(zone_at(&schedule, 9000), zone_at(&schedule, 12000));
    }

    #[test]
    fn the_zones_edge_is_inside_the_zone() {
        let zone = Zone {
            center: Vec2::new(1.0, 1.0),
            radius: 2.0,
        };
        assert!(zone.contains(Vec2::new(3.0, 1.0)));
        assert!(!zone.contains(Vec2::new(3.01, 1.0)));
    }

    #[test]
    fn the_next_stop_is_the_first_one_still_ahead() {
        let mut rng = Rng::from_seed(3);
        let schedule = Schedule::seeded(&mut rng, Vec2::new(2.0, 1.0));
        assert_eq!(next_stop(&schedule, 0).map(|(t, _)| t), Some(2100));
        assert_eq!(next_stop(&schedule, 2100).map(|(t, _)| t), Some(3900));
        assert_eq!(next_stop(&schedule, 9000), None);
    }
}
