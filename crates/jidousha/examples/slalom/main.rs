//! Slalom: a glider descending through drifting gates, and the controller that
//! flies it.
//!
//! A/D or the arrow keys steer. The glider falls at a fixed rate, so the course
//! *is* the clock: every gate arrives whether you are ready or not. Clear the
//! gap and score; clip a post or miss the gap and you do not.
//!
//! Run it:   `cargo run -p jidousha --example slalom`
//! Check it: `cargo run -p jidousha --example slalom -- --verify`
//!
//! # Why this example exists, which is not the same as why the game exists
//!
//! This is the worked controller `docs/api/jidousha-testing.md` points at. That
//! document argues at length about how to drive a game you cannot look at, and
//! the argument is genre-neutral while every worked instance of it was a paddle
//! returning a ball. A reader could not tell which sentences were *the lesson*
//! and which were *Pong*, and six runs of the acceptance exercise mis-read it in
//! the same place.
//!
//! So this game has no opponent, no bounce and no rally. What it has is the same
//! four-step shape, and the point is that the shape survives the change of
//! genre:
//!
//! 1. **Predict** where the target will be when you get there — [`gate_center_at`]
//!    is a plain function for exactly this reason, so the controller can ask.
//! 2. **Constrain** to what the glider can actually reach, with a margin, so the
//!    plan is never "be exactly on the post".
//! 3. **Optimise** inside what survives, against the whole remaining course
//!    rather than against the next gate alone.
//! 4. **Enumerate** the positions the glider can occupy, because a key moves it
//!    a whole step at a time and a plan for the space between two steps is a
//!    plan for a place it cannot be.
//!
//! `controller.rs` is that, in sixty lines of decision and a lot of comment.

use std::process::ExitCode;

use jidousha::prelude::*;

mod checks;
mod controller;
mod verify;

// --- the course --------------------------------------------------------
//
// Every number is in world units. The camera follows the glider down, so `y`
// grows without bound and the layout is stated relative to the glider rather
// than to a fixed court.

/// How many world units the camera spans vertically.
pub(crate) const VIEW_HEIGHT: f32 = 20.0;

/// How far from the centre line the course may wander, either side.
///
/// The walls are drawn here and the glider is clamped to it.
pub(crate) const COURSE_HALF_WIDTH: f32 = 16.0;

/// How fast the glider descends, in world units per second.
///
/// Constant, and not under the player's control. That is the whole design: the
/// only decision in this game is *where to be*, and the time to decide it is
/// fixed by the course rather than by the player's patience.
pub(crate) const DESCENT_SPEED: f32 = 11.0;

/// How fast the glider moves sideways under a held key, in units per second.
pub(crate) const GLIDE_SPEED: f32 = 16.0;

/// How far apart the gates are, down the course.
pub(crate) const GATE_SPACING: f32 = 9.0;

/// Half the width of a gate's gap.
pub(crate) const GATE_HALF_GAP: f32 = 2.3;

/// How wide a gate post is drawn, and how wide it counts as.
pub(crate) const POST_WIDTH: f32 = 0.55;

/// Half a post's height, which is also the only margin the cull needs.
pub(crate) const POST_HALF_HEIGHT: f32 = 0.7;

/// Half the glider's width — what has to fit inside the gap.
pub(crate) const GLIDER_HALF_WIDTH: f32 = 0.7;

/// How many gates a course has.
pub(crate) const GATES: u32 = 24;

/// Where the first gate sits, below the glider's start.
pub(crate) const FIRST_GATE_Y: f32 = 14.0;

// --- how a gate drifts -------------------------------------------------

/// How far a gate's centre swings either side of the centre line.
pub(crate) const DRIFT_AMPLITUDE: f32 = 13.0;

/// How far along its swing each gate starts, relative to the one above it.
///
/// This is what makes consecutive gates ask for different positions, and it is
/// bounded by how far the glider can travel between them.
///
/// INVARIANT: the course is only completable while the largest gap between two
/// consecutive gates' centres *at the moments the glider reaches them* is no
/// more than `GLIDE_SPEED * GATE_SPACING / DESCENT_SPEED` plus the slack in the
/// gap. `checks::the_course_is_completable` is that inequality, asserted rather
/// than believed — the arithmetic that decides whether *any* controller can
/// clear this course, and the first thing to reach for when a run reports the
/// game unplayable.
pub(crate) const DRIFT_PER_GATE: Radians = Radians::from_degrees(244.0);

/// How fast a gate swings, in angle per second.
///
/// **This is the constant that makes the game a game, and the number that
/// matters is how it compares to `GLIDE_SPEED`.** At this rate a gate's centre
/// crosses the course at up to 27 units a second and the glider only manages
/// 11, so a pilot that steers at where a gate *is* can never catch it — it is
/// always heading at a place the gate has left. The only way through is to work
/// out where the gate will be when the glider arrives and be standing there.
///
/// Set it low enough that the glider outruns the gate and the game evaporates:
/// chasing becomes a winning strategy, prediction buys nothing, and the course
/// has no decision left in it. That is not a hypothetical — it is what this
/// example did at 31 degrees a second, and `checks::the_gap_between_pilots_is_a_game`
/// is the check that caught it.
pub(crate) const DRIFT_PER_SECOND: Radians = Radians::from_degrees(120.0);

/// Where gate `index`'s centre sits at `seconds`, in world X.
///
/// **A plain function, deliberately, and this is the load-bearing decision in
/// the whole example.** The controller has to know where a gate will be when
/// the glider gets there, not where it is now. It can only ask that question if
/// asking is *possible* — if the answer lives inside the body of a system, the
/// controller's only route to it is to run the whole game forward and look,
/// which it cannot do while it is the thing deciding what the game does next.
///
/// So the rule this example is here to demonstrate: **anything a controller has
/// to predict must be a pure function of the world rather than a branch inside
/// the system that acts on it.** It costs nothing while you are writing the game
/// and it is expensive to retrofit.
#[must_use]
pub(crate) fn gate_center_at(index: u32, phase: f32, seconds: f32) -> f32 {
    // Deterministic and closed-form, so a check can ask about gate 400 at any
    // moment without simulating anything. `sin_cos` is the engine's, never
    // `f32::sin`: those are the deterministic ones (`docs/api/`, Conventions).
    let degrees = index as f32 * DRIFT_PER_GATE.to_degrees()
        + seconds * DRIFT_PER_SECOND.to_degrees()
        + phase;
    let (sin, _cos) = sin_cos(Radians::from_degrees(degrees % 360.0));
    sin * DRIFT_AMPLITUDE
}

/// How far down the course gate `index` sits.
#[must_use]
pub(crate) fn gate_depth(index: u32) -> f32 {
    FIRST_GATE_Y + index as f32 * GATE_SPACING
}

/// How many ticks the glider takes to fall from `from` to `to`.
///
/// Rounded down, because a gate is judged on the tick the glider is past its
/// plane and a fraction of a tick buys nothing.
#[must_use]
pub(crate) fn ticks_to_fall(from: f32, to: f32, fixed_dt: f32) -> u32 {
    if to <= from {
        return 0;
    }
    ((to - from) / (DESCENT_SPEED * fixed_dt)) as u32
}

/// Whether a glider centred at `x` clears gate `index`.
///
/// The whole glider has to fit: its half-width against the gap's half-width,
/// which is why a plan that puts it exactly on the gap's edge is a plan to
/// clip a post the first time anything rounds the wrong way.
#[must_use]
pub(crate) fn clears(x: f32, gate_x: f32) -> bool {
    (x - gate_x).abs() + GLIDER_HALF_WIDTH <= GATE_HALF_GAP
}

/// How much room to spare a glider at `x` has in gate `index`'s gap.
///
/// Negative means it clipped. This is the number the controller optimises and
/// the number the check reads, which is not a coincidence: a controller that
/// cannot report how well it did cannot be told apart from one that got lucky.
#[must_use]
pub(crate) fn clearance(x: f32, gate_x: f32) -> f32 {
    GATE_HALF_GAP - GLIDER_HALF_WIDTH - (x - gate_x).abs()
}

// --- colours -----------------------------------------------------------

/// The sky. Dark enough for a white glider to read against.
pub(crate) const SKY: Color = Color::rgb(0.06, 0.07, 0.12);
/// The course walls.
pub(crate) const WALL: Color = Color::rgba(1.0, 1.0, 1.0, 0.4);
/// A gate not yet reached.
pub(crate) const GATE_AHEAD: Color = Color::rgb(0.45, 0.80, 1.0);
/// A gate the glider cleared.
pub(crate) const GATE_CLEARED: Color = Color::rgb(0.45, 1.0, 0.65);
/// A gate the glider missed.
pub(crate) const GATE_MISSED: Color = Color::rgb(1.0, 0.45, 0.42);
/// The glider.
pub(crate) const GLIDER: Color = Color::rgb(1.0, 0.95, 0.72);

/// Draw-order bands. Named once here rather than spelled as numbers at each
/// call site, which is what `docs/api/` asks for.
pub(crate) mod layers {
    /// The walls and the depth markings.
    pub const COURSE: i16 = 0;
    /// Gates.
    pub const GATES: i16 = 1;
    /// The glider.
    pub const GLIDER: i16 = 2;
    /// Score and hints.
    pub const UI: i16 = 3;
}

// --- state -------------------------------------------------------------

/// The glider. There is exactly one, but it is an entity rather than a resource
/// because it has a `Transform` and gets drawn.
#[derive(Clone, Copy)]
pub(crate) struct Glider;
impl Component for Glider {}

/// How a gate turned out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Not reached yet.
    Ahead,
    /// The glider fitted through the gap.
    Cleared,
    /// It did not.
    Missed,
}

/// The course, and how the run is going.
pub(crate) struct Course {
    /// Which swing the gates start on. Seeded, so a course is replayable.
    pub phase: f32,
    /// One entry per gate, in order.
    pub outcomes: Vec<Outcome>,
    /// The next gate the glider has not yet fallen past.
    pub next_gate: u32,
    /// Each cleared gate, and how much room to spare it was taken with.
    pub clearances: Vec<(u32, f32)>,
}

impl Resource for Course {}

impl Course {
    /// A fresh course at `phase`.
    #[must_use]
    pub(crate) fn new(phase: f32) -> Self {
        Self {
            phase,
            outcomes: vec![Outcome::Ahead; GATES as usize],
            next_gate: 0,
            clearances: Vec::new(),
        }
    }

    /// How many gates the glider has cleared.
    #[must_use]
    pub(crate) fn cleared(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|o| **o == Outcome::Cleared)
            .count()
    }

    /// Whether every gate has been judged.
    #[must_use]
    pub(crate) fn finished(&self) -> bool {
        self.next_gate >= GATES
    }
}

fn main() -> ExitCode {
    if std::env::args().any(|arg| arg == "--verify") {
        return verify::run();
    }
    println!("A/D or the arrow keys steer. The glider falls on its own.");
    match run(GameConfig::default(), register) {
        Ok(()) => ExitCode::SUCCESS,
        // Display, not Debug: `RunError`'s four-part message, not a struct dump.
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Every system this game has, in the order they run.
///
/// **The order is the game's answer to a question the engine does not answer for
/// it**, and it is asserted rather than trusted: `steer` moves the glider and
/// `judge_the_gates` decides what it passed through, so the glider is judged at
/// its *post-move* position for the whole tick. `checks::the_schedule_is_the_one_we_chose`
/// holds the game to that with `HeadlessSim::schedule_debug`, which is the only
/// instrument that can see a swap of these two lines.
pub(crate) fn register(app: &mut App) {
    app.add_system(Startup, lay_out_the_course);
    app.add_system(Update, steer);
    app.add_system(Update, descend);
    app.add_system(Update, judge_the_gates);
    app.add_system(Update, follow_the_glider);
    app.add_system(Draw, draw_the_course);
    app.add_system(Draw, draw_the_glider);
    app.add_system(Draw, draw_the_hud);
}

fn lay_out_the_course(world: &mut World) {
    // Seeded, so the same config gives the same course on every machine.
    let phase = world.resource_mut::<Rng>().next_f32() * 360.0;
    world.insert_resource(Course::new(phase));
    world.insert_resource(Camera {
        center: Vec2::ZERO,
        height: VIEW_HEIGHT,
        clear_color: SKY,
        ..Camera::default()
    });
    let glider = world.spawn();
    world.insert(glider, Transform::at(Vec2::ZERO));
    world.insert(glider, Glider);
}

/// A/D or the arrows. One value per tick, so the game only asks what is true now.
fn steer(world: &mut World) {
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    let push = f32::from(input.held(Key::D) || input.held(Key::ArrowRight))
        - f32::from(input.held(Key::A) || input.held(Key::ArrowLeft));
    let step = push * GLIDE_SPEED * world.resource::<Time>().fixed_dt.as_f32();
    let limit = COURSE_HALF_WIDTH - GLIDER_HALF_WIDTH;
    for (_, transform, _) in world.query_mut::<(&mut Transform, &Glider)>() {
        transform.pos.x = (transform.pos.x + step).clamp(-limit, limit);
    }
}

/// Fall. Not a choice, which is what makes the course a clock.
fn descend(world: &mut World) {
    let step = DESCENT_SPEED * world.resource::<Time>().fixed_dt.as_f32();
    for (_, transform, _) in world.query_mut::<(&mut Transform, &Glider)>() {
        transform.pos.y += step;
    }
}

/// Keep the camera on the glider.
///
/// An `Update` system rather than a `Draw` one, because `Draw` cannot change the
/// world and the camera *is* world state. The lag this introduces is one frame
/// at most and is invisible; putting the layout arithmetic here rather than in
/// `Draw` is what lets every drawing system agree about where the view is.
fn follow_the_glider(world: &mut World) {
    let Some((_, glider, _)) = world.query::<(&Transform, With<Glider>)>().next() else {
        return;
    };
    let eye = glider.pos.y;
    world.resource_mut::<Camera>().center = Vec2::new(0.0, eye);
}

/// Judge every gate the glider has fallen past since the last tick.
///
/// A loop rather than an `if`, because a slow enough timestep could put two
/// gates inside one tick's travel and a gate that is never judged is a gate
/// that silently counts as cleared.
fn judge_the_gates(world: &mut World) {
    let Some((_, &position)) = world
        .query::<(&Transform, With<Glider>)>()
        .next()
        .map(|(entity, transform, _)| (entity, transform))
    else {
        return;
    };
    let at = position.pos;
    let seconds = world.resource::<Time>().elapsed.as_f32();
    let (phase, mut next) = {
        let course = world.resource::<Course>();
        (course.phase, course.next_gate)
    };
    let mut judged: Vec<(u32, Outcome, f32)> = Vec::new();
    while next < GATES && at.y >= gate_depth(next) {
        let gate_x = gate_center_at(next, phase, seconds);
        let room = clearance(at.x, gate_x);
        let outcome = if clears(at.x, gate_x) {
            Outcome::Cleared
        } else {
            Outcome::Missed
        };
        judged.push((next, outcome, room));
        next += 1;
    }
    if judged.is_empty() {
        return;
    }
    let course = world.resource_mut::<Course>();
    for (index, outcome, room) in judged {
        course.outcomes[index as usize] = outcome;
        if outcome == Outcome::Cleared {
            course.clearances.push((index, room));
        }
    }
    course.next_gate = next;
}

// --- drawing -----------------------------------------------------------
//
// The camera follows the glider, so everything is drawn in world coordinates
// and the camera's `center` is what scrolls. A Draw system reads the world's
// committed state and cannot change it.

/// Keep the camera on the glider, and draw the walls and every gate near it.
fn draw_the_course(ctx: &mut DrawCtx) {
    let Some(course) = ctx.world.find_resource::<Course>() else {
        return;
    };
    let Some(camera) = ctx.world.find_resource::<Camera>() else {
        return;
    };
    let seconds = ctx.world.resource::<Time>().elapsed.as_f32();
    let view = camera.visible_bounds();
    let (top, bottom) = (view.min.y, view.max.y);

    // The two walls, as tall as the view.
    for side in [-1.0_f32, 1.0] {
        ctx.rect(
            Rect::from_min_size(
                Vec2::new(side * COURSE_HALF_WIDTH - 0.15, top),
                Vec2::new(0.3, bottom - top),
            ),
            WALL,
            Depth::layer(layers::COURSE),
        );
    }

    for index in 0..GATES {
        let y = gate_depth(index);
        // Cull against the view the quads will be judged against, with the
        // post's own half-height as the only margin. A generous margin here is
        // a quad drawn off screen, which is the check every game should have.
        if y + POST_HALF_HEIGHT < top || y - POST_HALF_HEIGHT > bottom {
            continue;
        }
        let color = match course.outcomes[index as usize] {
            Outcome::Ahead => GATE_AHEAD,
            Outcome::Cleared => GATE_CLEARED,
            Outcome::Missed => GATE_MISSED,
        };
        let center = gate_center_at(index, course.phase, seconds);
        // Two posts, and the gap between them is the thing to fly through.
        for side in [-1.0_f32, 1.0] {
            let inner = center + side * GATE_HALF_GAP;
            let outer = inner + side * POST_WIDTH;
            ctx.rect(
                Rect::from_min_size(
                    Vec2::new(inner.min(outer), y - POST_HALF_HEIGHT),
                    Vec2::new(POST_WIDTH, POST_HALF_HEIGHT * 2.0),
                ),
                color,
                Depth::layer(layers::GATES),
            );
        }
    }
}

/// The glider: a small disc with a nose, so its heading reads at a glance.
fn draw_the_glider(ctx: &mut DrawCtx) {
    let Some((_, transform, _)) = ctx.world.query::<(&Transform, With<Glider>)>().next() else {
        return;
    };
    ctx.circle(
        transform.pos,
        GLIDER_HALF_WIDTH,
        GLIDER,
        Depth::layer(layers::GLIDER),
    );
}

/// The score and the hint, pinned to the camera rather than to the world.
fn draw_the_hud(ctx: &mut DrawCtx) {
    let Some(course) = ctx.world.find_resource::<Course>() else {
        return;
    };
    let Some(camera) = ctx.world.find_resource::<Camera>() else {
        return;
    };
    let view = camera.visible_bounds();
    let style = TextStyle {
        face: Face::BUILT_IN,
        size: 1.1,
        color: Color::rgba(0.85, 0.90, 1.0, 0.95),
        depth: Depth::layer(layers::UI),
    };
    let score = format!("{} / {}", course.cleared(), GATES);
    ctx.text(
        Vec2::new(-COURSE_HALF_WIDTH + 0.6, view.min.y + 0.8),
        &score,
        style,
    );
    let hint = if course.finished() {
        "course complete"
    } else {
        "A/D steer  the gates drift"
    };
    let hint_style = TextStyle {
        face: Face::BUILT_IN,
        size: 0.72,
        ..style
    };
    ctx.text(
        Vec2::new(-hint_style.width_of(hint) * 0.5, view.max.y - 1.7),
        hint,
        hint_style,
    );
}
