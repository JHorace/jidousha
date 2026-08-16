//! Everything the input system knows, on screen (input.md §8, I1).
//!
//! Press keys, move the mouse, click, scroll. This draws what arrived: the keys
//! currently down, the last few edges with the tick they landed on, the
//! pointer in both screen and world coordinates, its buttons, and how much the
//! wheel has turned. It is the translation table's proof, and the thing to look
//! at when a key does not do what a game expects.
//!
//! Things worth trying, because each one is a rule the engine promises:
//!
//! - **Hold a key.** One press edge, then nothing while it stays down — the
//!   operating system's auto-repeat is dropped at the seam (input.md §2).
//! - **Alt-tab away while holding it.** Every held key releases, so a character
//!   does not keep running while you read your email (input.md §4).
//! - **Press a key your keyboard has and this engine does not** — a numpad key,
//!   a media key. Nothing happens, which is a documented boundary rather than a
//!   failure.
//!
//! Run it: `cargo run -p jidousha --example input_echo`
//! On the web: `tools/serve-web input_echo`
//!
//! DELIBERATE: built but not run by `tools/test`, like the other windowed
//! examples — it opens a window and waits for a person (tooling.md).

use jidousha::prelude::*;

/// How many world units the window spans vertically.
const VIEW_HEIGHT: f32 = 20.0;

/// How many past edges to keep on screen.
///
/// An edge lasts one tick, which at sixty a second is a flicker. Keeping the
/// last few is what turns "did that register?" into something a person can
/// actually read.
const LOG_LINES: usize = 8;

/// What the last few ticks of input looked like.
///
/// Accumulated in Update rather than in Draw, because Draw cannot write the
/// world (ADR-0008) — and because this is state the game has, not a picture.
#[derive(Debug, Default)]
struct Echo {
    /// Keys down right now, in the engine's canonical order.
    held: Vec<String>,
    /// Recent edges, oldest first, each with the tick it landed on.
    log: Vec<String>,
    /// Every line the wheel has turned since the program started.
    scroll_total: f32,
    /// Where the pointer is, in pixels from the window's top-left.
    screen: Vec2,
    /// Which pointer buttons are down.
    buttons: Vec<String>,
    /// Whether the window has focus. Not input, but observable by simulation
    /// and therefore recorded (input.md §4).
    focused: bool,
}
impl Resource for Echo {}

fn main() -> Result<(), RunError> {
    run(
        GameConfig {
            title: "jidousha — input echo",
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, set_the_scene);
            app.add_system(Update, watch_the_input);
            app.add_system(Draw, draw_the_readout);
            app.add_system(Draw, draw_the_pointer);
        },
    )
}

fn set_the_scene(world: &mut World) {
    world.insert_resource(Camera {
        clear_color: Color::rgb(0.06, 0.07, 0.10),
        height: VIEW_HEIGHT,
        ..Camera::default()
    });
    world.insert_resource(Echo::default());
    println!("window open — press keys, move the mouse, scroll. close it to quit");
}

/// Copy this tick's input into something the Draw systems can read.
fn watch_the_input(world: &mut World) {
    let tick = world.resource::<Time>().tick;
    let Some(input) = world.find_resource::<Input>() else {
        // The first frame can run a tick before the driver has set any input,
        // and a game that assumed otherwise would panic on startup.
        return;
    };

    // `Input` answers about one key at a time — `held(Key::W)` — which is what
    // a game asks. Wanting the *whole list* is unusual enough that it lives on
    // the snapshot instead, and a readout like this one is the case for it.
    let snapshot = input.snapshot();
    let held: Vec<String> = snapshot
        .held_keys()
        .iter()
        .map(|key| key.to_string())
        .collect();
    let pointer = input.pointer();
    let buttons: Vec<String> = pointer
        .held_buttons()
        .iter()
        .map(|button| button.to_string())
        .collect();
    let screen = pointer.screen;
    let scroll = pointer.scroll;
    let focused = input.window_focused();

    // Every edge this tick, in one list, so the log reads in the order things
    // happened rather than keyboard-then-mouse.
    let mut edges: Vec<String> = Vec::new();
    for key in snapshot.pressed_keys() {
        edges.push(format!("{tick:>5}  down  {key}"));
    }
    for key in snapshot.released_keys() {
        edges.push(format!("{tick:>5}  up    {key}"));
    }
    for button in pointer.pressed_buttons() {
        edges.push(format!("{tick:>5}  click {button}"));
    }
    for button in pointer.released_buttons() {
        edges.push(format!("{tick:>5}  let   {button}"));
    }
    if scroll != 0.0 {
        edges.push(format!("{tick:>5}  wheel {scroll:+.2} lines"));
    }

    let echo = world.resource_mut::<Echo>();
    echo.held = held;
    echo.buttons = buttons;
    echo.screen = screen;
    echo.scroll_total += scroll;
    echo.focused = focused;
    echo.log.extend(edges);
    if echo.log.len() > LOG_LINES {
        let excess = echo.log.len() - LOG_LINES;
        echo.log.drain(..excess);
    }
}

fn draw_the_readout(ctx: &mut DrawCtx) {
    let echo = ctx.world.resource::<Echo>();
    let camera = ctx.world.resource::<Camera>();
    let (top_left, _) = camera.visible_bounds();
    let world = camera.screen_to_world(echo.screen);

    let heading = TextStyle {
        size: 0.9,
        color: Color::WHITE,
        depth: Depth::layer(1),
    };
    let body = TextStyle {
        size: 0.7,
        color: Color::rgba(0.65, 0.85, 1.0, 0.95),
        depth: Depth::layer(1),
    };

    let left = top_left.x + 1.0;
    let mut line = top_left.y + 1.0;

    ctx.text(Vec2::new(left, line), "input echo", heading);
    line += heading.size * 1.6;

    let held = if echo.held.is_empty() {
        "keys   (none)".to_owned()
    } else {
        format!("keys   {}", echo.held.join(" "))
    };
    let buttons = if echo.buttons.is_empty() {
        "buttons(none)".to_owned()
    } else {
        format!("buttons{}", echo.buttons.join(" "))
    };
    // Screen is what was recorded; world is derived from it through the camera,
    // which is the only sanctioned conversion (conventions, input.md §3).
    let facts = format!(
        "{held}\n\
         {buttons}\n\
         screen ({:.0}, {:.0})\n\
         world  ({:+.2}, {:+.2})\n\
         scroll {:+.2} lines total\n\
         focus  {}",
        echo.screen.x,
        echo.screen.y,
        world.x,
        world.y,
        echo.scroll_total,
        if echo.focused { "yes" } else { "no" },
    );
    ctx.text(Vec2::new(left, line), &facts, body);
    line += body.size * 7.0;

    ctx.text(Vec2::new(left, line), "recent edges", heading);
    line += heading.size * 1.6;
    let log = if echo.log.is_empty() {
        "  (nothing yet - press something)".to_owned()
    } else {
        echo.log.join("\n")
    };
    ctx.text(
        Vec2::new(left, line),
        &log,
        TextStyle {
            color: Color::rgba(1.0, 0.95, 0.7, 0.95),
            ..body
        },
    );
}

/// A crosshair where the pointer is, in world space.
///
/// The point of drawing it in *world* space rather than at the raw pixel: it
/// proves the camera conversion, and it is what a game actually needs when it
/// asks "what did the player click on".
fn draw_the_pointer(ctx: &mut DrawCtx) {
    let echo = ctx.world.resource::<Echo>();
    let camera = ctx.world.resource::<Camera>();
    let at = camera.screen_to_world(echo.screen);

    // Red while a button is down, so a click is visible at the cursor and not
    // only in the log.
    let color = if echo.buttons.is_empty() {
        Color::rgba(0.4, 1.0, 0.6, 0.9)
    } else {
        Color::rgb(1.0, 0.35, 0.3)
    };
    let depth = Depth::layer(2);
    let arm = 1.2;
    ctx.line(
        Vec2::new(at.x - arm, at.y),
        Vec2::new(at.x + arm, at.y),
        0.08,
        color,
        depth,
    );
    ctx.line(
        Vec2::new(at.x, at.y - arm),
        Vec2::new(at.x, at.y + arm),
        0.08,
        color,
        depth,
    );
    ctx.circle(at, 0.22, color, depth);

    // A ring that grows with the wheel, so scrolling has somewhere to show.
    let radius = 1.6 + echo.scroll_total * 0.1;
    if radius > 0.1 {
        ctx.rect(
            Rect::from_center_size(at, Vec2::splat(radius * 2.0)),
            Color::rgba(color.r, color.g, color.b, 0.10),
            Depth::layer(0),
        );
    }
}
