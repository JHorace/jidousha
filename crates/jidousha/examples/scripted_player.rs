//! A player driven by a script instead of by hands: the loop this engine exists
//! for (input.md §5).
//!
//! An agent writes the input, runs the ticks, and asserts on world state. No
//! window is opened, nothing is timed, and the run is identical on every
//! machine — which is what makes "did my change break the jump?" a question a
//! test can answer.
//!
//! Both ways of driving one are here, because they answer different questions:
//!
//! - **`InputScript`** is the session written down in advance. It is a pure
//!   function of the tick, so a test can seek, replay and bisect it. Reach for
//!   it whenever the input does not depend on what the game does back.
//! - **`SnapshotBuilder`** is a controller deciding as it goes: look at the
//!   world, then record a press or a release. Reach for it when the input has to
//!   *respond* — a blind script never returns a ball, so it can prove the
//!   controls work and still say nothing about whether the game is playable.
//!
//! A third part, for a game whose input is a **mouse**: `pointer_at` takes
//! screen pixels and a game states its targets in world units, so the click has
//! to be converted — and converted through the camera the frame is drawn with,
//! viewport included. `click_a_world_target` is that worked end to end, with
//! the result read back off a recorded frame's transcript — and then played a
//! second time with a **finger**, which is the same game with nothing changed
//! (input.md §3a).
//!
//! Run it: `cargo run -p jidousha --example scripted_player`

use jidousha::prelude::*;
// Driving input is a testing facility, not something a shipped game does.
use jidousha::testing::{
    FingerId, FrameRecorder, InputEvent, InputScript, InputSnapshot, SnapshotBuilder,
};

/// Where the player is, in world units.
#[derive(Clone, Copy, Debug)]
struct Position(Vec2);
impl Component for Position {}

/// How fast they are going, in units per second.
#[derive(Clone, Copy, Debug)]
struct Velocity(Vec2);
impl Component for Velocity {}

/// What the player has done, so the assertions have something to check.
#[derive(Clone, Copy, Debug)]
struct Tally {
    jumps: u32,
    shots: u32,
}
impl Component for Tally {}

/// Ground level, in world units. Y is down (ADR-0010), so "up" is negative.
const GROUND: f32 = 0.0;
const RUN_SPEED: f32 = 4.0;
const JUMP_SPEED: f32 = -12.0;
const GRAVITY: f32 = 30.0;

/// How long the run is. Longer than the script, on purpose — see `main`.
const TICKS: u64 = 130;

fn spawn_the_player(world: &mut World) {
    let player = world.spawn();
    world.insert(player, Position(Vec2::new(0.0, GROUND)));
    world.insert(player, Velocity(Vec2::ZERO));
    world.insert(player, Tally { jumps: 0, shots: 0 });
}

/// Read the input once, then write the world: the read-pass/write-pass shape
/// every system with a resource and a query takes (core.md §5, ADR-0013).
fn control_the_player(world: &mut World) {
    let (run, jump, shoot) = {
        let input = world.resource::<Input>();
        let mut run = 0.0;
        if input.held(Key::A) {
            run -= RUN_SPEED;
        }
        if input.held(Key::D) {
            run += RUN_SPEED;
        }
        (
            run,
            input.just_pressed(Key::Space),
            input.pointer().just_pressed(PointerButton::Primary),
        )
    };

    for (_, position, velocity, tally) in
        world.query_mut::<(&Position, &mut Velocity, &mut Tally)>()
    {
        velocity.0.x = run;
        // Jumping only works with your feet on the ground, which is what makes
        // the edge semantics matter: a held Space must not lift you twice.
        if jump && position.0.y >= GROUND {
            velocity.0.y = JUMP_SPEED;
            tally.jumps += 1;
        }
        if shoot {
            tally.shots += 1;
        }
    }
}

fn move_the_player(world: &mut World) {
    let step = world.resource::<Time>().fixed_dt.as_f32();
    for (_, position, velocity) in world.query_mut::<(&mut Position, &mut Velocity)>() {
        velocity.0.y += GRAVITY * step;
        position.0 += velocity.0 * step;
        if position.0.y > GROUND {
            position.0.y = GROUND;
            velocity.0.y = 0.0;
        }
    }
}

fn main() {
    // The whole session, written down. Run right for a second and a half,
    // jumping twice on the way, and fire once at something on the right of the
    // screen.
    let script = InputScript::new()
        .hold(Key::D, 10..100)
        .press(Key::Space, 20)
        .press(Key::Space, 70)
        // A pixel picked out of the air, because nothing here has to be *hit*:
        // this click tests the edge, not the aim. A click that has to land on
        // something is `click_a_world_target`, below.
        .pointer_at(64, Vec2::new(640.0, 200.0))
        .click(PointerButton::Primary, 65);

    let mut sim = headless(
        GameConfig {
            title: "scripted player",
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, spawn_the_player);
            app.add_system(Update, control_the_player);
            app.add_system(Update, move_the_player);
        },
    );
    // Startup runs on the first tick, which needs the resource to already be
    // there — systems read input, they do not ask whether it exists.
    sim.world_mut()
        .insert_resource(Input::new(InputSnapshot::new()));

    // The script ends at tick 100; the game keeps running to 130. A script
    // says what the *player* does, not how long the world moves — the second
    // jump is still in the air when the last key is let go.
    for tick in 1..=TICKS {
        // The input choke point: one snapshot per tick, and simulation sees
        // nothing else (input.md §1).
        sim.world_mut()
            .insert_resource(Input::new(script.snapshot_at(tick)));
        sim.tick();
    }

    let (position, tally) = sim
        .world()
        .query::<(&Position, &Tally)>()
        .map(|(_, position, tally)| (position.0, *tally))
        .next()
        .unwrap_or((Vec2::ZERO, Tally { jumps: 0, shots: 0 }));

    println!(
        "after {TICKS} ticks (script ends at {}): x={:.2}, {} jumps, {} shots",
        script.last_tick(),
        position.x,
        tally.jumps,
        tally.shots
    );

    // Held for 90 ticks at 4 units/second on a 60 Hz timestep: 6 units right.
    assert!(
        (position.x - 6.0).abs() < 1e-4,
        "expected to run 6 units, got {}",
        position.x
    );
    assert_eq!(tally.jumps, 2, "two taps of Space, two jumps");
    assert_eq!(tally.shots, 1, "one click, one shot");
    assert!(
        (position.y - GROUND).abs() < 1e-4,
        "back on the ground by the end"
    );

    // The same script, run again, lands in exactly the same place — the point
    // of the whole exercise.
    let mut again = headless(
        GameConfig {
            title: "scripted player",
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, spawn_the_player);
            app.add_system(Update, control_the_player);
            app.add_system(Update, move_the_player);
        },
    );
    again
        .world_mut()
        .insert_resource(Input::new(InputSnapshot::new()));
    for tick in 1..=TICKS {
        again
            .world_mut()
            .insert_resource(Input::new(script.snapshot_at(tick)));
        again.tick();
    }
    let replayed = again
        .world()
        .query::<&Position>()
        .map(|(_, position)| [position.0.x.to_bits(), position.0.y.to_bits()])
        .next()
        .unwrap_or([0, 0]);
    let original = [position.x.to_bits(), position.y.to_bits()];
    assert_eq!(replayed, original, "same script, same run, bit for bit");
    println!("replayed the script: identical to the bit");

    chase_a_target();
    click_a_world_target();
}

/// The other half: input decided from what the game is doing, tick by tick.
///
/// The player runs towards a target that is not where the script-writer could
/// have known it would be, and stops when it arrives. No script can express
/// that, because the answer on tick 40 depends on the world at tick 39.
///
/// `SnapshotBuilder` is the driver's own accumulator, so this exercises the real
/// edge rules: a key held across many ticks presses once, and letting go
/// releases once. Building a one-tick `InputScript` per tick would press on
/// every one of them.
fn chase_a_target() {
    /// Where the player is told to stop, in world units.
    const TARGET: f32 = 5.0;
    /// How close counts as arrived.
    const REACH: f32 = 0.1;

    let mut sim = headless(
        GameConfig {
            title: "closed-loop player",
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, spawn_the_player);
            app.add_system(Update, control_the_player);
            app.add_system(Update, move_the_player);
        },
    );
    sim.world_mut()
        .insert_resource(Input::new(InputSnapshot::new()));

    // A controller sends events, so it has to remember what it is already
    // holding. That is what a keyboard is, and it is why the edges come out
    // right.
    let mut keyboard = SnapshotBuilder::new();
    let mut holding_right = false;
    let mut arrived_at = None;

    for tick in 1..=TICKS {
        let here = sim
            .world()
            .query::<&Position>()
            .map(|(_, position)| position.0.x)
            .next()
            .unwrap_or(0.0);

        // The decision. This is the line a script cannot hold, because `here`
        // is not known until the tick before.
        let want_right = here < TARGET - REACH;
        if want_right != holding_right {
            keyboard.record(if want_right {
                InputEvent::KeyPressed(Key::D)
            } else {
                InputEvent::KeyReleased(Key::D)
            });
            holding_right = want_right;
        }
        if !want_right && arrived_at.is_none() {
            arrived_at = Some(tick);
        }

        sim.world_mut()
            .insert_resource(Input::new(keyboard.first_tick_snapshot()));
        sim.tick();
    }

    let finished = sim
        .world()
        .query::<&Position>()
        .map(|(_, position)| position.0.x)
        .next()
        .unwrap_or(0.0);
    let tally = sim
        .world()
        .query::<&Tally>()
        .map(|(_, tally)| *tally)
        .next()
        .unwrap_or(Tally { jumps: 0, shots: 0 });

    // Report the numbers the assertion judged, not just its verdict: nobody
    // writing a game this way can look at it, so the message is the instrument.
    println!(
        "closed loop: stopped at x={finished:.2} (target {TARGET}), arrived on tick {:?}, \
         {} jumps",
        arrived_at, tally.jumps
    );
    assert!(
        (finished - TARGET).abs() < 0.5,
        "expected to stop near {TARGET}, stopped at {finished:.3} after {TICKS} ticks"
    );
    assert!(
        arrived_at.is_some_and(|tick| tick < TICKS),
        "expected to arrive before the run ended; arrived_at={arrived_at:?}"
    );
    // Space was never recorded, so no edge can have been invented for it — the
    // check that a one-tick script per tick would have failed.
    assert_eq!(
        tally.jumps, 0,
        "a controller that never presses Space must produce no press edge for it"
    );
}

/// The third shape: a **pointer** game, whose targets are world rectangles.
///
/// `InputScript::pointer_at` takes screen pixels. A game states where things
/// are in world units. So a scripted click is a conversion, and the conversion
/// is `Camera::world_to_screen` through a camera built **exactly** as the game
/// builds its own — `viewport` included.
///
/// That last word is the whole trap. Nothing stamps `Camera::viewport` under
/// `headless`: the windowed driver measures the window and writes it in every
/// frame, and there is no window here. A check that builds its camera
/// differently from the game's — a different height, a default viewport, a
/// centre it guessed — converts every click to the wrong pixel, and the run
/// fails with nothing selected and no clue why, because the input arrived
/// perfectly and simply landed somewhere else.
///
/// One camera, built once, used by the game *and* by the script, is what makes
/// that impossible. Then the click is asserted twice: on the world, and on the
/// recorded frame's transcript — the closest thing to looking at the screen.
fn click_a_world_target() {
    /// The viewport the frames are drawn at. The recorder's override and the
    /// camera below agree because they are the same constant.
    const VIEWPORT: PhysicalSize = PhysicalSize::new(1280, 720);
    /// How many world units the frame spans vertically.
    const VIEW_HEIGHT: f32 = 20.0;
    /// How long the run is.
    const TICKS: u64 = 30;

    /// Where the button is, in world units — the game's own statement of it,
    /// and the only place it is written down.
    fn button() -> Rect {
        Rect::from_center_size(Vec2::new(6.0, -4.0), Vec2::new(5.0, 2.0))
    }

    /// The camera the game installs, and the one a click is aimed through.
    ///
    /// A function rather than two copies: the check cannot drift from the game
    /// if there is only one of it.
    fn camera() -> Camera {
        Camera {
            center: Vec2::ZERO,
            height: VIEW_HEIGHT,
            clear_color: Color::rgb(0.05, 0.06, 0.09),
            viewport: VIEWPORT,
        }
    }

    /// Whether the button has been pressed, and how often it was missed.
    #[derive(Clone, Copy, Debug, Default)]
    struct Panel {
        pressed: bool,
        missed: u32,
    }
    impl Resource for Panel {}

    fn install_the_panel(world: &mut World) {
        world.insert_resource(camera());
        world.insert_resource(Panel::default());
    }

    /// The game's side of the same conversion, in the other direction.
    fn press_the_button(world: &mut World) {
        let Some(input) = world.find_resource::<Input>() else {
            return;
        };
        if !input.pointer().just_pressed(PointerButton::Primary) {
            return;
        }
        let screen = input.pointer().screen;
        let at = world.resource::<Camera>().screen_to_world(screen);
        let panel = world.resource_mut::<Panel>();
        if button().contains(at) {
            panel.pressed = true;
        } else {
            panel.missed += 1;
        }
    }

    /// Lit once pressed, so the frame says what the world says.
    fn draw_the_button(ctx: &mut DrawCtx) {
        let panel = *ctx.world.resource::<Panel>();
        let color = if panel.pressed {
            Color::rgb(0.2, 0.9, 0.4)
        } else {
            Color::rgb(0.35, 0.35, 0.4)
        };
        ctx.rect(button(), color, Depth::layer(0));
    }

    // The click, aimed in world units and converted once. Three ticks: move,
    // press, settle — the pointer has to be somewhere before the button that
    // reads it goes down.
    let aim = camera().world_to_screen(button().center());
    let script = InputScript::new()
        .pointer_at(10, aim)
        .click(PointerButton::Primary, 11);

    let mut sim = headless(
        GameConfig {
            title: "pointer target",
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, install_the_panel);
            app.add_system(Update, press_the_button);
            app.add_system(Draw, draw_the_button);
        },
    );
    sim.world_mut()
        .insert_resource(Input::new(InputSnapshot::new()));

    let mut recorder = FrameRecorder::new(VIEWPORT);
    let mut last = None;
    for tick in 1..=TICKS {
        sim.world_mut()
            .insert_resource(Input::new(script.snapshot_at(tick)));
        sim.tick();
        last = Some(recorder.draw(&mut sim));
    }
    let Some(frame) = last else {
        panic!("a frame was drawn on every one of the {TICKS} ticks");
    };

    let panel = *sim.world().resource::<Panel>();
    println!(
        "pointer: aimed at {aim:?} for world {:?}",
        button().center()
    );
    assert!(
        panel.pressed,
        "the click converted to {aim:?} and missed a button at {:?} — the \
         camera the script aimed through is not the one the game reads with",
        button()
    );
    assert_eq!(panel.missed, 0, "one click, aimed, and it landed");

    // And the same fact read off the frame, which is what a person would have
    // seen. `covering` answers "what is at this world position?"; the tint is
    // the button's lit colour, so the drawing agrees with the world.
    let under = frame.covering(button().center());
    assert_eq!(under.len(), 1, "exactly the button is under that point");
    assert!(
        (under[0].tint.g - 0.9).abs() < 1e-6,
        "the button drew unlit: {:?}",
        under[0].tint
    );
    // The transcript is the screenshot substitute: one line per quad, in world
    // units, stable enough to assert on and diff. The expected line is built
    // from `button()` rather than typed out, so it cannot drift from the game.
    let transcript = frame.transcript();
    let expected = format!(
        "quad ({:.3}, {:.3}) ({:.3}, {:.3})",
        button().min.x,
        button().min.y,
        button().max.x,
        button().max.y
    );
    assert!(
        transcript.contains(&expected),
        "expected `{expected}` — the button is not where the game says it is:\n{transcript}"
    );
    println!("pointer: the button is lit, and the frame agrees\n{transcript}");

    // ---- and now the same game, with a thumb -------------------------------
    //
    // Not one line of `press_the_button` changes. The engine mirrors the first
    // finger down onto the primary pointer — its position, and a `Primary`
    // press — so a game written for a mouse is playable on a phone
    // (input.md §3a). A check drives that the way a touchscreen does: touch
    // events through a `SnapshotBuilder`, which is where the mirror lives.
    let mut sim = headless(
        GameConfig {
            title: "touch target",
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, install_the_panel);
            app.add_system(Update, press_the_button);
        },
    );
    let mut builder = SnapshotBuilder::new();
    let thumb = FingerId::from_platform(0);
    for tick in 1..=TICKS {
        // Land on the button, drag a little, lift: what a tap actually is.
        match tick {
            10 => builder.record(InputEvent::Touched {
                finger: thumb,
                phase: TouchPhase::Began,
                screen: aim,
            }),
            12 => builder.record(InputEvent::Touched {
                finger: thumb,
                phase: TouchPhase::Moved,
                screen: aim + Vec2::new(2.0, 1.0),
            }),
            14 => builder.record(InputEvent::Touched {
                finger: thumb,
                phase: TouchPhase::Ended,
                screen: aim + Vec2::new(2.0, 1.0),
            }),
            _ => {}
        }
        sim.world_mut()
            .insert_resource(Input::new(builder.first_tick_snapshot()));
        sim.tick();
    }

    let panel = *sim.world().resource::<Panel>();
    assert!(
        panel.pressed,
        "the tap at {aim:?} did not reach a button at {:?} — the first finger \
         down is supposed to be the cursor",
        button()
    );
    assert_eq!(panel.missed, 0, "one tap, and it landed once");
    println!("touch: the same button, pressed with a finger and no code change");
}
