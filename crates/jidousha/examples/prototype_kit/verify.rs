//! The §5 loop: script input, run headless, assert, print the transcript.
//!
//! This is what `tools/verify prototype_kit` runs, and it is the engine's whole
//! thesis in one function — an agent drives the game, asks what the world did
//! and what was drawn, and never opens a window (input.md §5, renderer.md §9).
//!
//! It runs the *same* systems and the same config the window does. What differs
//! is only what a person would otherwise supply: the input comes from a script,
//! and the art from a store with scripted arrival ticks, so the run is the same
//! on every machine and on every day.
//!
//! # One thing here is not the shape to copy
//!
//! **A game gets its frames from `FrameRecorder`.** Two calls — `FrameRecorder::new`
//! with the camera's viewport, then `recorder.draw(&mut sim)` once a tick, which
//! hands back the `FrameRecord` every assertion reads. `recorder.font_texture()`
//! answers "which texture is the font on", and `frame.plan` is what a capture
//! path replays to get a PNG. That is the whole of it, and it is what the testing
//! document prescribes.
//!
//! **This file does not do that**, and the fifteen lines it spends instead —
//! `sim.draw()`, its own `TextureTable` from `create_builtin_textures`,
//! `plan_frame`, `backend.render`, and a throwaway `NullBackend` in
//! `textures_font_id` below to work out where the font landed — are **not** part
//! of writing a `--verify` mode. They are here because this example is doing a
//! second job that a game does not have: `play` takes a `&mut dyn RenderBackend`,
//! so the identical session runs through a null backend *and* through a real GPU,
//! and the run can assert that the world did the same thing both times. Driving
//! the backend by hand is what buys that comparison, and `FrameRecorder` records
//! into a null backend only.
//!
//! So: read this file for the *checks* — what to assert about a world and about
//! what was drawn, and how to report a failure. Do not read it for how to get a
//! frame. E0 run 6 read both and had to work out which half was advice.
//!
//! DELIBERATE: the divergence is kept rather than fixed, and it is named here at
//! the top rather than only where it happens (see ADR-0026). Making this file use
//! the recorder would delete the two-backend comparison, which is the one thing in
//! the repository that checks a session is backend-agnostic; splitting it into two
//! examples would duplicate a whole game to say one thing twice. What was wrong
//! was that the explanation lived two hundred lines down, in the doc comment of a
//! private helper, where a reader who had already copied the shape would meet it
//! (e0-findings.md F-073).

use crate::checks::{Checks, fail, greater, near, sizes_covering};
use crate::{Paddle, config, register};
use jidousha::prelude::*;
use jidousha::testing::{
    BackendTextureId, FONT_TEXTURE, FramePlan, InputScript, MemorySource, NullBackend,
    RenderBackend, create_builtin_textures, decode_png, plan_frame, upload_ready_textures,
};
use std::process::ExitCode;

/// How long the scripted session runs.
///
/// Long enough for the script below to push the paddle into *both* ends of
/// its clamp — a shorter run would still pass every assertion here, and the
/// clamp would be asserted only in the sense of never having been reached.
const TICKS: u64 = 130;

/// The tick the art is scripted to arrive on.
///
/// Partway through, so the run spends time on both sides of it and the
/// placeholder is part of what gets verified.
const ART_ARRIVES: u64 = 30;

/// How far the paddle may travel from the centre, matching its component.
const LIMIT: f32 = 7.0;

/// The camera the headless run uses, so the transcript is the same everywhere.
const HEADLESS_VIEWPORT: PhysicalSize = PhysicalSize::new(1280, 720);

/// The world height the game's camera is set to (`main.rs`'s `VIEW_HEIGHT`).
const HEADLESS_VIEW_HEIGHT: f32 = crate::VIEW_HEIGHT;

/// The score's text size, from `draw_the_readout`.
const SCORE_SIZE: f32 = 1.6;

/// Where the ball is after `TICKS` ticks, in world units.
///
/// A number, checked in. The ball's X is a sine of simulated time and its Y
/// never moves, so after a fixed number of fixed-length ticks it is in exactly
/// one place — and that is the whole determinism claim (core.md §7, ADR-0009)
/// reduced to something a verification can compare against.
const BALL_X_AT_END: f32 = -2.3294;
const BALL_Y: f32 = -4.0;

/// Where in `track` the largest value first appears.
fn peak_at(track: &[f32], pick: fn(f32, f32) -> bool) -> usize {
    let mut best = 0;
    for (index, value) in track.iter().enumerate() {
        if pick(*value, track[best]) {
            best = index;
        }
    }
    best
}

/// The art, arriving on a scripted tick rather than whenever a disk says.
///
/// The real PNG, baked into the binary. A verify run reads no files at all —
/// it is the same on a machine that checked the repository out somewhere
/// else, and the bytes are the ones the window would have shown, so a
/// picture that stopped decoding fails here rather than passing on a stub.
fn store() -> Assets {
    let Ok(hero) = decode_png(include_bytes!("../../../../assets/sprites/hero.png")) else {
        fail(
            "the example's own art no longer decodes",
            "assets/sprites/hero.png is baked into this binary and read by nothing else",
        );
    };
    let mut source = MemorySource::new();
    source.insert_texture("sprites/hero.png", hero);
    source.complete_at("sprites/hero.png", ART_ARRIVES);
    Assets::new(source)
}

/// Hold S until the paddle jams against the bottom, then W until it jams
/// against the top.
///
/// Both holds deliberately last longer than the travel they have available,
/// so the clamp is *exercised* rather than merely not violated.
fn script() -> InputScript {
    InputScript::new().hold(Key::S, 5..45).hold(Key::W, 50..130)
}

/// What one scripted run of the game did.
///
/// Returned rather than asserted on inside the loop, so the *same* loop can be
/// played through two different backends and the results compared — which is
/// how this file checks renderer.md §1's contract that everything above the
/// seam is backend-agnostic.
pub(super) struct Run {
    /// The paddle's Y after each tick.
    pub(super) paddle_track: Vec<f32>,
    /// Where the paddle ended up.
    pub(super) paddle_pos: Vec2,
    /// Where the ball ended up.
    pub(super) ball_pos: Vec2,
    /// How many frames drew the checkered placeholder.
    pub(super) placeholder_frames: u32,
    /// How many frames were submitted.
    pub(super) frames: usize,
    /// The camera the last frame was drawn with.
    ///
    /// The game's own, with the viewport this run chose stamped on — which is
    /// the pair that has to agree before any assertion about *where* a quad is
    /// means anything. Carried out rather than rebuilt here, so a check reads
    /// the camera the frame was actually planned from.
    pub(super) camera: Camera,
}

/// Play the scripted session through `backend`, drawing every tick.
///
/// `viewport` is the camera's, which decides the frame's aspect ratio and
/// nothing else — the world is the same whatever it is set to, which is the
/// point of the comparison the caller makes.
pub(super) fn play(backend: &mut dyn RenderBackend, viewport: PhysicalSize) -> Run {
    let mut sim = headless(config(), register);
    // Before Startup, which is what `set_the_scene` checks for.
    sim.world_mut().insert_resource(store());

    let script = script();
    let mut textures = create_builtin_textures(backend);
    let mut paddle_track = Vec::new();
    let mut paddle_pos = Vec2::ZERO;
    let mut ball_pos = Vec2::ZERO;
    let mut placeholder_frames = 0;
    let mut frames = 0;
    let mut last_camera = Camera::default();

    for tick in 1..=TICKS {
        let Some(assets) = sim.world_mut().find_resource_mut::<Assets>() else {
            fail(
                "the store vanished",
                "Startup installs one and nothing removes it",
            );
        };
        assets.commit(tick);
        upload_ready_textures(assets, backend, &mut textures);

        sim.world_mut()
            .insert_resource(Input::new(script.snapshot_at(tick)));
        sim.tick();

        let paddle = sim
            .world()
            .query::<(&Transform, &Paddle)>()
            .map(|(_, transform, _)| transform.pos)
            .next();
        match paddle {
            Some(pos) => {
                paddle_pos = pos;
                paddle_track.push(pos.y);
            }
            None => fail("the paddle is gone", "Startup spawns exactly one"),
        }
        match sim
            .world()
            .query::<(&Transform, &Sprite)>()
            .map(|(_, transform, _)| transform.pos)
            .next()
        {
            Some(pos) => ball_pos = pos,
            None => fail("the ball is gone", "Startup spawns exactly one sprite"),
        }

        // Draw every tick, so the transcript covers the frames before the
        // art arrives as well as the ones after.
        let camera = Camera {
            viewport,
            ..*sim.world().resource::<Camera>()
        };
        last_camera = camera;
        let quads = sim.draw().quads().to_vec();
        let plan = plan_frame(&camera, &quads, &textures);
        if plan
            .batches
            .iter()
            .any(|batch| batch.texture == textures.placeholder())
        {
            placeholder_frames += 1;
        }
        if let Err(error) = backend.render(&plan) {
            fail("a backend refused a frame", &error.to_string());
        }
        frames += 1;
    }

    Run {
        paddle_track,
        paddle_pos,
        ball_pos,
        placeholder_frames,
        frames,
        camera: last_camera,
    }
}

pub fn run() -> ExitCode {
    let mut checks = Checks::default();
    let mut backend = NullBackend::new();
    let transcript_run = play(&mut backend, HEADLESS_VIEWPORT);
    let Run {
        paddle_track,
        paddle_pos,
        ball_pos,
        placeholder_frames,
        frames,
        camera,
    } = &transcript_run;
    let (paddle_track, paddle_pos, ball_pos) = (paddle_track.clone(), *paddle_pos, *ball_pos);
    let (placeholder_frames, frames) = (*placeholder_frames, *frames);
    let camera = *camera;

    // --- what the world did ------------------------------------------
    // Y is down (ADR-0010), so the bottom of the screen is the larger number.
    let start = paddle_track[0];
    let bottom_at = peak_at(&paddle_track, greater);
    let top_at = peak_at(&paddle_track, |a, b| greater(b, a));
    let (bottom, top) = (paddle_track[bottom_at], paddle_track[top_at]);

    checks.require(
        near(bottom, LIMIT) && near(top, -LIMIT),
        "the paddle did not come to rest against both ends of its field",
        format!(
            "it reached {bottom:.3} and {top:.3}; the clamp is +/-{LIMIT:.1}, and the \
             script holds each key long enough to run past it"
        ),
    );
    // Down first, then up: S and W the right way round. Both extremes are
    // reached either way, so only the order tells a swap apart.
    checks.require(
        bottom_at < top_at,
        "S and W move the paddle the wrong way round",
        format!(
            "the script holds S first, but the paddle was at the top on tick \
             {top} before it was at the bottom on tick {bottom}",
            top = top_at + 1,
            bottom = bottom_at + 1,
        ),
    );
    checks.require(
        greater(bottom, start),
        "the paddle did not start between the two ends it reached",
        format!("it started at {start:.3}, which is not above {bottom:.3}"),
    );

    // --- what was drawn ----------------------------------------------
    checks.require(
        frames == TICKS as usize,
        "one frame per tick was expected",
        format!("{frames} frames for {TICKS} ticks"),
    );
    let Some(last) = backend.last_frame() else {
        fail("no frame was recorded", "the loop above draws every tick");
    };
    // The field markings, the paddles, the ball and the text: several
    // batches, because they do not all sample the same texture.
    checks.require(
        last.plan.batches.len() >= 3,
        "the last frame is too simple to be this game",
        format!(
            "{} batches; expected shapes, art and text",
            last.plan.batches.len()
        ),
    );
    // Exactly the frames before the art arrives, rather than "at least one" and
    // "not all of them". Both of the looser forms pass for a store that resolves
    // on tick 1, which is the state this example exists to show is survivable —
    // a requirement stated where it can hardly fail is a requirement about a
    // case that hardly happens.
    checks.require(
        u64::from(placeholder_frames) == ART_ARRIVES - 1,
        "the placeholder did not cover exactly the frames before the art arrived",
        format!(
            "{placeholder_frames} of {frames} frames drew it; the art is scripted to arrive \
             on tick {ART_ARRIVES}, so the {} frames before it must show the checkered \
             placeholder and none after it may (renderer.md §5)",
            ART_ARRIVES - 1
        ),
    );
    // And the paddle really is on screen where the world says it is — the
    // position is read back out of the world rather than written down here,
    // so this asks whether drawing agrees with simulation.
    //
    // "Something is drawn there" is not enough: the readout text wanders
    // across most of the field, so that question passes with the paddle
    // deleted. The quad has to be the *size* of a paddle.
    //
    // And its **centre** has to be the paddle's, which is the half of this
    // check that was missing. A paddle-sized quad covers its own centre even
    // when it is drawn a long way out of position — displacing this one by 45%
    // of its own height passed the size test and the whole verification, and
    // was found by breaking the game on purpose rather than by reading the code
    // (e0-findings.md F-058). Covering a point says a quad is nearby; only its
    // bounds say where it is.
    let paddle_shaped = last.covering(paddle_pos).into_iter().any(|quad| {
        let bounds = quad.bounds();
        near(bounds.max.x - bounds.min.x, crate::PADDLE_SIZE.x)
            && near(bounds.max.y - bounds.min.y, crate::PADDLE_SIZE.y)
            && near(bounds.center().x, paddle_pos.x)
            && near(bounds.center().y, paddle_pos.y)
    });
    checks.require(
        paddle_shaped,
        "no paddle-shaped quad was drawn where the paddle is",
        format!(
            "the world puts it at ({:.2}, {:.2}), {} by {}; what covers that point is {}",
            paddle_pos.x,
            paddle_pos.y,
            crate::PADDLE_SIZE.x,
            crate::PADDLE_SIZE.y,
            sizes_covering(last, paddle_pos)
        ),
    );

    // Text is on screen, and where the game puts it. The font atlas is a
    // texture like any other (renderer.md §6), so "was text drawn" is "did a
    // quad sample the font", and the score's own position is what says the
    // layout ran rather than something merely having been submitted.
    let font = textures_font_id(&last.plan);
    checks.require(
        font.is_some(),
        "nothing on screen sampled the font atlas",
        "the score, the readout and the character sample are all text, so a frame without a \
         font batch has lost all three (renderer.md §6)"
            .to_owned(),
    );
    let glyphs: usize = font.map_or(0, |font| {
        last.plan
            .batches
            .iter()
            .filter(|batch| batch.texture == font)
            .map(|batch| batch.quad_count())
            .sum()
    });
    // The score is drawn centred at the top; its middle character is a dash,
    // whose cell straddles this point. A layout that stopped centring, or a
    // camera that stopped agreeing with it, moves the text off this spot.
    let score_middle = Vec2::new(0.0, -HEADLESS_VIEW_HEIGHT / 2.0 + 1.0 + SCORE_SIZE / 2.0);
    // Asked as "what is in *front* here" rather than "is a glyph here at all".
    // The halfway line runs through this point too, on the field band, so this
    // one question answers both "did the layout run" and "is the UI band over
    // the field band" — and the second is only answerable because the two
    // overlap. `covering` is the depth sort read backwards, so its first entry
    // is what a player sees.
    let front_at_score = last.covering(score_middle).into_iter().next();
    let score_drawn =
        font.is_some_and(|font| front_at_score.is_some_and(|quad| quad.texture == font));
    checks.require(
        score_drawn,
        "the score is not the front-most thing where the game draws it",
        format!(
            "the front-most quad at ({:.2}, {:.2}) — the middle of a score centred by \
             TextStyle::width_of, where the halfway line also runs — is {:?} rather than a \
             glyph; {glyphs} glyphs were drawn in all",
            score_middle.x,
            score_middle.y,
            front_at_score.map(|quad| quad.tint),
        ),
    );

    // The ball is a sprite, and after a fixed number of ticks it is in a fixed
    // place — the whole determinism claim in one number (core.md §7). The
    // engine's own sin/cos is what puts it there (ADR-0009), so this is the
    // assertion that fails if the timestep, the seed of the clock, or the
    // trigonometry ever changes.
    checks.require(
        near(ball_pos.x, BALL_X_AT_END) && near(ball_pos.y, BALL_Y),
        "the ball is not where this many ticks should have put it",
        format!(
            "after {TICKS} ticks it is at ({:.4}, {:.4}); it was ({BALL_X_AT_END:.4}, \
             {BALL_Y:.4}) when this was written",
            ball_pos.x, ball_pos.y
        ),
    );
    let ball_drawn = last
        .covering(ball_pos)
        .into_iter()
        .any(|quad| font != Some(quad.texture) && quad.bounds().size().x > 2.0);
    checks.require(
        ball_drawn,
        "the ball sprite is not drawn where the world puts it",
        format!(
            "the world has it at ({:.2}, {:.2}); what covers that point is {}",
            ball_pos.x,
            ball_pos.y,
            sizes_covering(last, ball_pos)
        ),
    );

    // --- the bands, where the sort disagrees with the submission order ---
    //
    // A frame carries the order quads were drawn in, not the `Depth` that
    // produced it, so a band is only visible where it *changes* that order.
    // `register` submits the hitboxes before the art and the field after it for
    // exactly this reason: both pairs come back sorted the other way round, so
    // swapping two constants in `mod layers` moves them and a check can say so.
    // Where a game's submission order already agrees with its bands, no
    // assertion over drawn quads can see a layer at all — which is how a whole
    // layering goes untested while every other check passes.
    let quads = last.quads();
    let sprite_at = quads.iter().position(|quad| {
        let bounds = quad.bounds();
        font != Some(quad.texture)
            && greater(bounds.size().x, 2.0)
            && near(bounds.center().x, ball_pos.x)
            && near(bounds.center().y, ball_pos.y)
    });
    let field_at = quads.iter().position(|quad| quad.tint == crate::FIELD_LINE);
    let marker_at = quads.iter().position(|quad| quad.tint == crate::HITBOX_DOT);
    checks.require(
        sprite_at.is_some() && field_at.is_some() && marker_at.is_some(),
        "one of the three bands drew nothing in the last frame",
        format!(
            "as indices into the draw order: field {field_at:?}, art {sprite_at:?}, debug \
             marker {marker_at:?}; None means that band drew nothing where it was looked for"
        ),
    );
    if let (Some(sprite), Some(field), Some(marker)) = (sprite_at, field_at, marker_at) {
        checks.require(
            field < sprite,
            "the field is drawn over the art instead of behind it",
            format!(
                "the field marking is at index {field} in the draw order and the art at \
                 {sprite}; the game submits the field *after* the art, so only FIELD sorting \
                 under PLAY can put it first"
            ),
        );
        checks.require(
            marker > sprite,
            "the debug marker is drawn behind the art instead of over it",
            format!(
                "the debug marker is at index {marker} in the draw order and the art at \
                 {sprite}; the game submits the hitboxes *before* the art, so only DEBUG \
                 sorting over PLAY can put it last"
            ),
        );
    }

    // --- the two shapes whose size an "is something there" check cannot see ---
    //
    // "A quad the size of the thing is at the thing's position" is right for a
    // rectangle and wrong for a circle: `ctx.circle` submits sixteen wedges and
    // nothing the size of the disc is drawn anywhere. What is true is that all
    // sixteen share the centre as a corner and all sixteen fit inside the
    // circle's bounding box, so the union of the quads covering the centre —
    // filtered to that box, because the halfway line runs through the centre
    // too — is exactly `2r x 2r`.
    let centre = Vec2::ZERO;
    let box_of_it = Rect::from_center_size(centre, Vec2::splat(crate::CENTRE_RADIUS * 2.0));
    let mut disc: Option<Rect> = None;
    for quad in last.covering(centre) {
        let drawn = quad.bounds();
        // Written out rather than as `Rect::contains`, which is half-open and
        // would throw away the wedges reaching the far edge.
        let inside = greater(drawn.min.x, box_of_it.min.x - 0.001)
            && greater(drawn.min.y, box_of_it.min.y - 0.001)
            && greater(box_of_it.max.x + 0.001, drawn.max.x)
            && greater(box_of_it.max.y + 0.001, drawn.max.y);
        if !inside {
            continue;
        }
        disc = Some(match disc {
            None => drawn,
            Some(so_far) => Rect {
                min: so_far.min.min(drawn.min),
                max: so_far.max.max(drawn.max),
            },
        });
    }
    let disc_size = disc.map(|rect| rect.size()).unwrap_or(Vec2::ZERO);
    checks.require(
        near(disc_size.x, crate::CENTRE_RADIUS * 2.0)
            && near(disc_size.y, crate::CENTRE_RADIUS * 2.0),
        "the centre marking is not a disc of the size the game draws",
        format!(
            "the wedges covering ({:.2}, {:.2}) span {:.3}x{:.3}; a radius of \
             {:.2} is {:.2} square",
            centre.x,
            centre.y,
            disc_size.x,
            disc_size.y,
            crate::CENTRE_RADIUS,
            crate::CENTRE_RADIUS * 2.0,
        ),
    );
    // And a second check the constant cannot move with. The one above compares
    // what was drawn against the number that drew it, so it goes on passing
    // after somebody changes that number — which is not hypothetical: it was
    // the one fault of fourteen that escaped this file when it was written.
    // This one states the requirement instead: a centre marking has to read as
    // one, which means a useful fraction of the court and not the whole of it.
    let court_height = camera.visible_bounds().size().y;
    checks.require(
        greater(disc_size.y, court_height * 0.1) && greater(court_height * 0.5, disc_size.y),
        "the centre marking is not a readable fraction of the court",
        format!(
            "it is {:.2} across on a court {court_height:.2} tall, which is {:.0}% of it; a \
             centre marking wants between a tenth and a half",
            disc_size.y,
            disc_size.y / court_height * 100.0
        ),
    );

    // And the hitbox outline really is the art's *component* size rather than
    // the bounds of the rotated quad — the difference this example exists to
    // show. Four lines of thickness `t` laid on the box's edges span the box
    // plus `t` in each direction, which is a number stated by the two constants
    // rather than written down here.
    let mut outline: Option<Rect> = None;
    for quad in quads.iter().filter(|quad| quad.tint == crate::HITBOX_LINE) {
        let drawn = quad.bounds();
        outline = Some(match outline {
            None => drawn,
            Some(so_far) => Rect {
                min: so_far.min.min(drawn.min),
                max: so_far.max.max(drawn.max),
            },
        });
    }
    let outline_size = outline.map(|rect| rect.size()).unwrap_or(Vec2::ZERO);
    let want = crate::ART_SIZE + Vec2::splat(crate::HITBOX_THICKNESS);
    checks.require(
        near(outline_size.x, want.x) && near(outline_size.y, want.y),
        "the hitbox outline is not the art's own size",
        format!(
            "the outline spans {:.3}x{:.3}; the art is {:.2}x{:.2} and the lines are \
             {:.2} thick, so it should span {:.3}x{:.3}",
            outline_size.x,
            outline_size.y,
            crate::ART_SIZE.x,
            crate::ART_SIZE.y,
            crate::HITBOX_THICKNESS,
            want.x,
            want.y,
        ),
    );

    // --- nothing off screen ---------------------------------------------
    //
    // The highest-value check a game of shapes and text can write, and three
    // lines. `contains_rect` is closed on all four sides, because a quad flush
    // against the camera's edge is on screen; `Rect::contains` takes a point and
    // is half-open, which is a different question and the wrong rule here.
    let view = camera.visible_bounds();
    let off_screen: Vec<Rect> = quads
        .iter()
        .map(|quad| quad.bounds())
        .filter(|bounds| !view.contains_rect(*bounds))
        .collect();
    checks.require(
        off_screen.is_empty(),
        "something was drawn outside what the camera shows",
        format!(
            "{} of {} quads fall outside {view:?}; the first is {:?} — text centred by \
             TextStyle::width_of is the usual culprit",
            off_screen.len(),
            quads.len(),
            off_screen.first(),
        ),
    );

    // --- the background, which leaves no quad behind ----------------------
    //
    // Two checks rather than one. The first moves with the constant it compares
    // against and would keep passing if somebody changed that constant; the
    // second states the requirement the colour exists to meet, and does not.
    let cleared = last.plan.clear_color;
    checks.require(
        cleared == crate::COURT,
        "the court was cleared to a colour the game does not name",
        format!(
            "the frame cleared to {cleared:?}; the game's constant is {:?}",
            crate::COURT
        ),
    );
    let brightness = cleared.r.max(cleared.g).max(cleared.b);
    checks.require(
        greater(0.25, brightness) && greater(cleared.a, 0.99),
        "the court is not dark enough for the white field markings to read against it",
        format!(
            "its brightest channel is {brightness:.3} at alpha {:.2}",
            cleared.a
        ),
    );

    // --- the strings themselves -------------------------------------------
    //
    // No assertion over drawn quads can see a wrong *character*: the font draws
    // an unknown one as a box at exactly a letter's advance, so a stray em dash
    // or curly quote passes the glyph count, the centring and the bounds check
    // alike. The string is the only instrument there is, which is why `main.rs`
    // hands its readout back as one rather than formatting it inside the draw
    // system where nothing could reach it.
    let readout = crate::readout_text(TICKS, TICKS as f32 / 60.0, 0.0);
    for (name, text) in [
        ("the score", crate::SCORE_TEXT),
        ("the font sample", crate::FONT_SAMPLE),
        ("the readout", readout.as_str()),
    ] {
        let stray = text
            .chars()
            .find(|glyph| *glyph != '\n' && !(' '..='~').contains(glyph));
        checks.require(
            stray.is_none(),
            "a string the game draws has a character the font cannot draw",
            format!(
                "{name} contains {stray:?}, which draws as a box at exactly a letter's width \
                 — no assertion over what was drawn can tell the difference"
            ),
        );
    }

    let captured = crate::capture::capture_a_frame(&mut checks, &paddle_track);
    let verdict = checks.verdict();

    println!("verified prototype_kit over {TICKS} ticks");
    println!(
        "  paddle: {start:.2} -> {bottom:.2} (tick {}) -> {top:.2} (tick {}), clamped to \
         +/-{LIMIT:.1}",
        bottom_at + 1,
        top_at + 1,
    );
    println!("  frames: {frames}, {placeholder_frames} of them with the placeholder");
    println!(
        "  ball: ({:.3}, {:.3}) after {TICKS} ticks",
        ball_pos.x, ball_pos.y
    );
    println!(
        "  last frame: {} batches, {glyphs} glyphs",
        last.plan.batches.len()
    );
    println!("  capture: {captured}");
    print!("{}", last.transcript());
    verdict
}

/// Which backend texture the font atlas landed on, read off the frame.
///
/// The table is gone by the time the assertions run, and the atlas is not at a
/// fixed id — it is whatever `create_builtin_textures` assigned. So: rebuild a
/// table against a throwaway backend, in the same order, and ask it.
///
/// **A game does not do this.** `FrameRecorder::font_texture()` answers the
/// question directly, because the recorder still owns the table that knows:
///
/// ```ignore
/// let mut recorder = FrameRecorder::new(PhysicalSize::new(1280, 720));
/// // A plain id, borrowing nothing; read out once so the assertions stay short.
/// let font = recorder.font_texture();
/// let frame = recorder.draw(&mut sim);
/// let text_was_drawn = frame.quads().iter().any(|quad| quad.texture == font);
/// ```
///
/// This example keeps the long way round because `play` below runs against a
/// *real* backend too, to capture a PNG, and the recorder records into a null
/// backend only. That is the whole reason the ceremony survives here: a golden
/// image needs a GPU, and asserting on what was drawn does not.
///
/// DELIBERATE: the shape above is written out rather than cited (see ADR-0026).
/// It used to point at another example's file, which is a dependency an example
/// has no business having — that file is free to be rewritten or deleted, and
/// this comment would quietly start naming something that is not there.
fn textures_font_id(plan: &FramePlan) -> Option<BackendTextureId> {
    let mut scratch = NullBackend::new();
    let table = create_builtin_textures(&mut scratch);
    let font = table.resolve(FONT_TEXTURE);
    plan.batches
        .iter()
        .any(|batch| batch.texture == font)
        .then_some(font)
}
