//! A shot: where it is aimed, where it scatters to, how long it flies, and a strike.
//!
//! Key types: `Flight`. Key functions: `aim_point`, `scatter`, `flight_ticks`,
//! `strike_landing`.
//! Depends on: the prelude and the course rect from `main`. Never depended on outside the game.
//! INVARIANT: a ball lands where it lands and stops; there is no roll.

use jidousha::prelude::*;

use crate::COURSE_HALF;

/// The longest shot, in world units, before equipment.
pub const MAX_SHOT: f32 = 7.0;
/// How fast a ball flies, in world units per second.
pub const BALL_SPEED: f32 = 12.0;
/// How far a shot may scatter, as a fraction of its length.
pub const SHOT_SPREAD: f32 = 0.12;
/// How near a golfer must stand to their resting ball to hit it.
pub const REACH_BALL: f32 = 1.2;
/// How far a strike on a ball sends it, before equipment.
pub const STRIKE_DISTANCE: f32 = 4.0;

/// A ball in the air: where it left, where it will rest, and when.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flight {
    /// Where it left from.
    pub from: Vec2,
    /// Where it will rest.
    pub to: Vec2,
    /// The tick it left on.
    pub start: u64,
    /// How many ticks it flies for.
    pub ticks: u64,
}

impl Flight {
    /// Where the ball is at `tick`.
    pub fn at(&self, tick: u64) -> Vec2 {
        if tick >= self.start + self.ticks {
            return self.to;
        }
        let t = tick.saturating_sub(self.start) as f32 / self.ticks as f32;
        self.from.lerp(self.to, t)
    }

    /// Whether the ball has landed by `tick`.
    pub fn done(&self, tick: u64) -> bool {
        tick >= self.start + self.ticks
    }
}

/// `point` pulled inside the course.
pub fn into_course(point: Vec2) -> Vec2 {
    point.clamp(-COURSE_HALF, COURSE_HALF)
}

/// Where a shot toward `want` is aimed: clamped to `reach`, then into the course.
pub fn aim_point(ball: Vec2, want: Vec2, reach: f32) -> Vec2 {
    into_course(ball + (want - ball).clamp_length_max(reach))
}

/// How many ticks a ball takes to fly `distance`; at least one.
pub fn flight_ticks(distance: f32) -> u64 {
    ((distance / BALL_SPEED * 60.0).ceil() as u64).max(1)
}

/// Where a shot aimed at `aim` from `ball` actually lands: scattered by up to
/// `SHOT_SPREAD` of its length, the two draws taken in a fixed order.
pub fn scatter(aim: Vec2, ball: Vec2, rng: &mut Rng) -> Vec2 {
    let length = rng.next_f32() * SHOT_SPREAD * (aim - ball).length();
    let angle = rng.next_f32() * Radians::TAU.as_f32();
    into_course(aim + rotate(Vec2::X * length, Radians(angle)))
}

/// Where a ball struck from `striker` lands: straight away from the striker.
pub fn strike_landing(striker: Vec2, ball: Vec2, distance: f32) -> Vec2 {
    let away = (ball - striker).normalize_or_zero();
    let direction = if away == Vec2::ZERO { Vec2::X } else { away };
    into_course(ball + direction * distance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_strike_from_on_top_of_the_ball_goes_plus_x() {
        let landing = strike_landing(Vec2::new(1.0, 1.0), Vec2::new(1.0, 1.0), 4.0);
        assert_eq!(landing, Vec2::new(5.0, 1.0));
    }

    #[test]
    fn a_flight_is_a_straight_line_that_ends_at_its_target() {
        let flight = Flight {
            from: Vec2::ZERO,
            to: Vec2::new(4.0, 0.0),
            start: 10,
            ticks: 20,
        };
        assert_eq!(flight.at(10), Vec2::ZERO);
        assert_eq!(flight.at(20), Vec2::new(2.0, 0.0));
        assert_eq!(flight.at(30), Vec2::new(4.0, 0.0));
        assert_eq!(flight.at(99), Vec2::new(4.0, 0.0));
        assert_eq!(flight_ticks(4.0), 20);
    }

    #[test]
    fn a_shot_is_clamped_to_reach_and_scatter_stays_within_its_spread() {
        let ball = Vec2::new(-5.0, 0.0);
        let aim = aim_point(ball, Vec2::new(10.0, 0.0), 7.0);
        assert_eq!(aim, Vec2::new(2.0, 0.0));
        let mut rng = Rng::from_seed(5);
        for _ in 0..200 {
            let to = scatter(aim, ball, &mut rng);
            assert!((to - aim).length() <= SHOT_SPREAD * 7.0 + 1e-3);
        }
    }
}
