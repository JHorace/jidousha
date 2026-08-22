//! Every `Vec2` operation a game reaches for, in one file.
//!
//! `Vec2` comes from `glam` and this repository does not own its documentation,
//! so the reference cannot generate an entry for it — but it is in almost every
//! line of a game, and "documented there" points at a crate whose docs are not
//! necessarily to hand. This file is the entry instead: it is embedded in the
//! API document verbatim, and cargo compiles it, so nothing listed here can
//! stop existing without the build saying so.
//!
//! **What cargo cannot check is the other direction.** An operation `glam` has
//! and this file omits is invisible — and E0 run 6 hit exactly that, wanting
//! `lerp` for a swept contact point, finding it unlisted, and writing
//! `from + (to - from) * t` rather than trust that the omission meant anything.
//! So: this is the vocabulary, kept complete on purpose and by hand, and a gap
//! in it is a bug to report rather than an answer. `glam` has more — component
//! comparisons, rounding, reflection, `Vec3` and matrix types — and `cargo doc
//! -p glam --open` is where the rest of it is.
//!
//! Nothing here is a special jidousha operation. It is the vocabulary a
//! position, a velocity and a size are written in.
//!
//! Run it: `cargo run -p jidousha --example vec2_tour`

use jidousha::prelude::*;

/// A position that can be worked out at compile time — `new` is a `const fn`.
const CORNER: Vec2 = Vec2::new(-HALF_W, -HALF_H);

/// The window the game asks `run` for, and the shape every extent is stated in.
const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);

/// Half the world height the camera spans — the one number a layout picks.
const HALF_H: f32 = 9.0;

/// And half the width, which is the height times the shape of the window.
///
/// `PhysicalSize::new` and `PhysicalSize::aspect` are both `const fn`, so a
/// layout stated in constants derives this rather than typing a ratio. The
/// alternative is `HALF_H * (16.0 / 9.0)`, which is two facts about one window:
/// change `WINDOW` and the ratio is silently stale, and only a runtime
/// assertion against `Camera::visible_bounds()` would ever say so.
const HALF_W: f32 = HALF_H * WINDOW.aspect();

/// An angle a game states once, in the units a person can check.
///
/// `Radians::from_degrees` is a `const fn`, so a bounce limit, a cone of vision
/// or a turn rate is a `const` written as a number you can picture. The two
/// alternatives are both worse: `Radians(1.0471976)` is rejected by clippy as an
/// approximation of `FRAC_PI_3`, and `Radians(core::f32::consts::FRAC_PI_3)`
/// stops being writable the moment the angle is not a tidy fraction of pi.
const MAX_BOUNCE: Radians = Radians::from_degrees(60.0);

fn main() {
    // Making one. `ZERO`, `ONE`, `X` and `Y` are constants; `splat` repeats a
    // scalar; `new` takes the two components, X first.
    let position = Vec2::new(3.0, 4.0);
    let size = Vec2::splat(2.0);
    assert_eq!(Vec2::ZERO, Vec2::new(0.0, 0.0));
    assert_eq!(Vec2::ONE, Vec2::splat(1.0));
    assert_eq!(Vec2::X, Vec2::new(1.0, 0.0));
    assert_eq!(Vec2::Y, Vec2::new(0.0, 1.0));

    // Components are plain public fields, readable and writable.
    let mut moving = position;
    moving.x += 1.0;
    moving.y = 0.0;
    assert_eq!(moving, Vec2::new(4.0, 0.0));

    // Arithmetic: vector with vector, and vector with scalar. `+= -= *= /=` all
    // work too, which is what a velocity integration step is written with.
    assert_eq!(position + size, Vec2::new(5.0, 6.0));
    assert_eq!(position - size, Vec2::new(1.0, 2.0));
    assert_eq!(position * 2.0, Vec2::new(6.0, 8.0));
    assert_eq!(position / 2.0, Vec2::new(1.5, 2.0));
    assert_eq!(position * size, Vec2::new(6.0, 8.0), "component-wise");
    assert_eq!(-position, Vec2::new(-3.0, -4.0));

    // Length. `length_squared` compares distances without the square root,
    // which is what a "within range?" test should use.
    assert_eq!(position.length(), 5.0);
    assert_eq!(position.length_squared(), 25.0);
    assert_eq!(position.distance(Vec2::ZERO), 5.0);
    assert_eq!(position.distance_squared(Vec2::ZERO), 25.0);
    assert!((position.normalize().length() - 1.0).abs() < 1e-6);

    // `normalize` of a zero vector is NaN, and NaN spreads: a velocity that
    // reaches exactly zero for one tick poisons every position after it, and
    // nothing panics. `normalize_or_zero` is the one to reach for whenever the
    // vector can be still.
    assert!(Vec2::ZERO.normalize().is_nan());
    assert_eq!(Vec2::ZERO.normalize_or_zero(), Vec2::ZERO);

    // Component-wise shaping: the operations a clamp to a playfield is made of.
    assert_eq!(Vec2::new(-3.0, 4.0).abs(), Vec2::new(3.0, 4.0));
    assert_eq!(position.min(size), Vec2::new(2.0, 2.0));
    assert_eq!(position.max(size), Vec2::new(3.0, 4.0));
    assert_eq!(position.clamp(Vec2::ZERO, size), Vec2::new(2.0, 2.0));

    // Capping a *magnitude* without turning the vector: a speed limit, a
    // maximum push, a terminal velocity. `clamp` above is component-wise and is
    // a different operation — it clips a diagonal into a box corner and changes
    // the direction; these keep the direction and move only the length.
    let fast = Vec2::new(3.0, 4.0); // length 5
    assert_eq!(fast.clamp_length_max(2.5), Vec2::new(1.5, 2.0));
    assert_eq!(
        fast.clamp_length_max(10.0),
        fast,
        "under the cap, untouched"
    );
    assert_eq!(fast.clamp_length_min(10.0), Vec2::new(6.0, 8.0));
    assert_eq!(fast.clamp_length(1.0, 2.5), Vec2::new(1.5, 2.0));

    // The zero vector has no direction to keep, so the two that can *lengthen*
    // it divide by zero and hand back NaN, silently, exactly as `normalize`
    // does. `clamp_length_max` is safe on it — it only ever shortens.
    assert_eq!(Vec2::ZERO.clamp_length_max(5.0), Vec2::ZERO);
    assert!(Vec2::ZERO.clamp_length_min(5.0).is_nan());

    // Dot tells you whether two directions agree — positive means "the same
    // way", which is how a game asks whether a ball is heading at a paddle.
    assert_eq!(Vec2::X.dot(Vec2::X), 1.0);
    assert_eq!(Vec2::X.dot(Vec2::Y), 0.0);
    assert_eq!(Vec2::X.dot(-Vec2::X), -1.0);

    // `lerp` is the point a fraction of the way along — which is how a swept
    // collision turns "the crossing happened 0.4 of the way through this tick"
    // into the world position where it happened.
    let (from, to) = (Vec2::ZERO, Vec2::new(10.0, 20.0));
    assert_eq!(from.lerp(to, 0.25), Vec2::new(2.5, 5.0));

    // `signum` is the direction of each component, which is what a reflection
    // or a serve direction is written with. Note it answers 1.0 for zero.
    assert_eq!(Vec2::new(-3.0, 4.0).signum(), Vec2::new(-1.0, 1.0));

    // `perp` is a quarter turn anticlockwise on paper — (x, y) becomes (-y, x)
    // — and it does not go through trigonometry at all, so it is exact. The
    // normal of a wall, and the sideways of a heading.
    assert_eq!(Vec2::X.perp(), Vec2::new(0.0, 1.0));

    // `move_towards` steps at most a fixed distance at the target and stops
    // exactly on it, which is a chasing opponent in one line and does not
    // overshoot on the last tick the way `normalize() * speed` does.
    let chaser = Vec2::ZERO.move_towards(Vec2::new(3.0, 4.0), 2.5);
    assert_eq!(chaser, Vec2::new(1.5, 2.0));
    assert_eq!(
        Vec2::ZERO.move_towards(Vec2::new(3.0, 4.0), 100.0),
        Vec2::new(3.0, 4.0)
    );

    // There is no scalar `move_towards` — `f32` is not a `Vec2` operation. A
    // paddle chases in *one* axis, so the one line there is: hold the
    // component you are not steering, take the one you are.
    let paddle_y = Vec2::new(0.0, 2.0).move_towards(Vec2::new(0.0, 6.0), 2.5).y;
    assert!((paddle_y - 4.5).abs() < 1e-6);

    // Angles go through the engine's own `sin_cos`, never through `f32::sin`:
    // those are the deterministic ones, and determinism is what makes a replay
    // replay. It lives in `jidousha::math` and the prelude re-exports it, so
    // the glob above is the whole import — there is no second `use` to write.
    let (sin, cos) = sin_cos(Radians::from_degrees(90.0));
    assert!(sin > 0.999 && cos.abs() < 1e-6);
    let turned = rotate(Vec2::X, Radians::from_degrees(90.0));
    assert!((turned - Vec2::Y).length() < 1e-6);
    assert!(atan2(1.0, 0.0).as_f32() > 0.0);
    // `from_degrees`, `to_degrees` and `as_f32` are all `const fn`, which is
    // what makes the constant above compile.
    assert!((MAX_BOUNCE.to_degrees() - 60.0).abs() < 1e-3);

    // Two Vec2s make a Rect, which is what collision and layout are written in.
    let bounds = Rect::from_center_size(position, size);
    assert!(bounds.contains(position));
    assert_eq!(bounds.size(), size);
    assert_eq!(Rect::from_min_size(CORNER, size).min, CORNER);

    // And a whole court, in constants, from the window the game opens at. The
    // camera spans `HALF_H` either side of its centre and as wide as the
    // window's shape makes it, so this is the rectangle `visible_bounds()`
    // reports back at that size — computed at compile time, from one number.
    let court = Rect::from_center_size(Vec2::ZERO, Vec2::new(HALF_W, HALF_H) * 2.0);
    assert_eq!(court.min, CORNER);
    // Nine units of half-height at sixteen by nine is sixteen of half-width,
    // and nothing typed that number: `WINDOW` did.
    assert_eq!(court.min, Vec2::new(-16.0, -9.0));

    println!("verified: every Vec2 operation above holds");
}
