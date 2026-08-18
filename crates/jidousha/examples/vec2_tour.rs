//! Every `Vec2` operation a game reaches for, in one file.
//!
//! `Vec2` comes from `glam` and this repository does not own its documentation,
//! so the reference cannot generate an entry for it — but it is in almost every
//! line of a game, and "documented there" points at a crate whose docs are not
//! necessarily to hand. This file is the entry instead: it is embedded in the
//! API document verbatim, and cargo compiles it, so the list cannot drift away
//! from what the type actually offers.
//!
//! Nothing here is a special jidousha operation. It is the vocabulary a
//! position, a velocity and a size are written in.
//!
//! Run it: `cargo run -p jidousha --example vec2_tour`

use jidousha::prelude::*;

/// A position that can be worked out at compile time — `new` is a `const fn`.
const CORNER: Vec2 = Vec2::new(-16.0, -9.0);

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
    assert!((position.normalize().length() - 1.0).abs() < 1e-6);

    // Component-wise shaping: the operations a clamp to a playfield is made of.
    assert_eq!(Vec2::new(-3.0, 4.0).abs(), Vec2::new(3.0, 4.0));
    assert_eq!(position.min(size), Vec2::new(2.0, 2.0));
    assert_eq!(position.max(size), Vec2::new(3.0, 4.0));
    assert_eq!(position.clamp(Vec2::ZERO, size), Vec2::new(2.0, 2.0));

    // Dot tells you whether two directions agree — positive means "the same
    // way", which is how a game asks whether a ball is heading at a paddle.
    assert_eq!(Vec2::X.dot(Vec2::X), 1.0);
    assert_eq!(Vec2::X.dot(Vec2::Y), 0.0);
    assert_eq!(Vec2::X.dot(-Vec2::X), -1.0);

    // Angles go through the engine's own `sin_cos`, never through `f32::sin`:
    // those are the deterministic ones, and determinism is what makes a replay
    // replay. It lives in `jidousha::math` and the prelude re-exports it, so
    // the glob above is the whole import — there is no second `use` to write.
    let (sin, cos) = sin_cos(Radians::from_degrees(90.0));
    assert!(sin > 0.999 && cos.abs() < 1e-6);
    let turned = rotate(Vec2::X, Radians::from_degrees(90.0));
    assert!((turned - Vec2::Y).length() < 1e-6);
    assert!(atan2(1.0, 0.0).as_f32() > 0.0);

    // Two Vec2s make a Rect, which is what collision and layout are written in.
    let bounds = Rect::from_center_size(position, size);
    assert!(bounds.contains(position));
    assert_eq!(bounds.size(), size);
    assert_eq!(Rect::from_min_size(CORNER, size).min, CORNER);

    println!("verified: every Vec2 operation above holds");
}
