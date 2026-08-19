//! One session, two backends, the same world: renderer.md §1's contract as a
//! test rather than as a design intention.
//!
//! Everything above the backend seam is supposed to be backend-agnostic. The
//! type system gets most of the way there — a `Draw` system cannot touch the
//! world at all (ADR-0008) — but not all of it: `upload_ready_textures` takes
//! the `Assets` resource **mutably** and hands its texels to whichever backend
//! is present, so a backend that failed differently, or registered ids
//! differently, could leave a different world behind it. That is the one route
//! by which "which GPU drew it" could reach a simulation, and this is the check
//! that it does not.
//!
//! This lived in `examples/prototype_kit` until ADR-0028: it is a claim about
//! the engine, and a claim about the engine belongs in a test rather than in the
//! example every game author copies.
//!
//! **No adapter is not a failure.** Every runner this project has is headless
//! and some have no graphics stack at all, so the GPU half says it skipped and
//! the run stays green — the same rule the golden tier follows (renderer.md §9).
//! What is *not* allowed is skipping in silence, so it prints why.

use jidousha::prelude::*;
use jidousha_render_core::{
    NullBackend, RenderBackend, RenderError, create_builtin_textures, plan_frame,
    upload_ready_textures,
};
use jidousha_render_wgpu::WgpuBackend;

/// How long the session runs.
///
/// Long enough to cross the tick the art is scripted to arrive on, because the
/// upload path is the whole reason this test exists.
const TICKS: u64 = 12;

/// The tick the texture resolves on, partway through.
const ART_ARRIVES: u64 = 5;

/// How big the offscreen target is. Small: nothing here looks at pixels.
const SIZE: PhysicalSize = PhysicalSize::new(64, 64);

/// How many polls to give the GPU handshake before calling it absent.
///
/// The backend is poll-based by design and a test has no frame loop to do the
/// asking. A working adapter resolves in a handful of these.
const HANDSHAKE_POLLS: usize = 10_000;

/// Something for the sim to move, so the world has state worth comparing.
#[derive(Clone, Copy)]
struct Drifting;
impl Component for Drifting {}

/// The handle, kept so the assertions can ask what became of it.
struct Art(TextureHandle);
impl Resource for Art {}

fn set_the_scene(world: &mut World) {
    world.insert_resource(Camera {
        height: 10.0,
        ..Camera::default()
    });
    let handle = world.resource_mut::<Assets>().load_texture("art.png");
    world.insert_resource(Art(handle));

    let thing = world.spawn();
    world.insert(thing, Transform::at(Vec2::new(-3.0, 0.0)));
    world.insert(thing, Drifting);
    world.insert(
        thing,
        Sprite {
            size: Vec2::splat(2.0),
            ..Sprite::new(handle)
        },
    );
}

fn drift(world: &mut World) {
    let step = world.resource::<Time>().fixed_dt.as_f32();
    for (_, transform, _) in world.query_mut::<(&mut Transform, &Drifting)>() {
        transform.pos.x += step;
    }
}

fn draw_it(ctx: &mut DrawCtx) {
    draw_sprites(ctx);
}

/// The art, arriving on a scripted tick rather than whenever a disk says.
fn store() -> Assets {
    let mut source = MemorySource::new();
    source.insert_texture(
        "art.png",
        jidousha::testing::TextureData {
            width: 2,
            height: 2,
            rgba: vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
            ],
        },
    );
    source.complete_at("art.png", ART_ARRIVES);
    Assets::new(source)
}

/// Everything about the world this test is willing to call "the same".
///
/// Float **bits** rather than values, because "the same world" means identical
/// and not merely close — the whole determinism claim is bit-for-bit
/// (core.md §7).
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    positions: Vec<[u32; 2]>,
    art_status: String,
    quads_in_last_frame: usize,
}

/// Play the identical scripted session through `backend`.
fn play(backend: &mut dyn RenderBackend) -> Outcome {
    let mut sim = headless(
        GameConfig {
            title: "backend agnostic",
            seed: 11,
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, set_the_scene);
            app.add_system(Update, drift);
            app.add_system(Draw, draw_it);
        },
    );
    sim.world_mut().insert_resource(store());

    let mut textures = create_builtin_textures(backend);
    let mut quads_in_last_frame = 0;
    for tick in 1..=TICKS {
        let assets = sim.world_mut().resource_mut::<Assets>();
        assets.commit(tick);
        upload_ready_textures(assets, backend, &mut textures);

        sim.tick();

        let camera = *sim.world().resource::<Camera>();
        let submissions = sim.draw();
        let plan = plan_frame(&camera, submissions.quads(), &textures);
        quads_in_last_frame = submissions.quads().len();
        let Ok(()) = backend.render(&plan) else {
            panic!("a backend that is ready refused the plan it was handed on tick {tick}");
        };
    }

    let positions = sim
        .world()
        .query::<&Transform>()
        .map(|(_, transform)| [transform.pos.x.to_bits(), transform.pos.y.to_bits()])
        .collect();
    let handle = sim.world().resource::<Art>().0;
    let art_status = format!("{:?}", sim.world().resource::<Assets>().status(handle));
    Outcome {
        positions,
        art_status,
        quads_in_last_frame,
    }
}

/// A backend with a GPU behind it, or `None` on a machine that has none.
fn offscreen() -> Option<WgpuBackend> {
    let mut backend = WgpuBackend::offscreen(SIZE);
    for _ in 0..HANDSHAKE_POLLS {
        match backend.poll() {
            Ok(()) if backend.is_ready() => return Some(backend),
            Ok(()) => {}
            // No adapter is a fact about the machine. Every other handshake
            // error is a fault, and reporting one of those as "no GPU here"
            // files a real problem as a property of the hardware, for ever.
            Err(error @ RenderError::NoAdapter { .. }) => {
                println!("no GPU on this machine, so only the null half runs: {error}");
                return None;
            }
            Err(error) => {
                panic!("the GPU handshake failed, and not for want of an adapter: {error}")
            }
        }
    }
    println!("the GPU handshake never finished, so only the null half runs");
    None
}

#[test]
fn a_session_leaves_the_same_world_through_a_real_gpu_as_through_a_null_backend() {
    let through_null = play(&mut NullBackend::new());

    let Some(mut gpu) = offscreen() else {
        // Still worth having run: the null half proves the session is playable
        // and the assertions below it are the GPU's alone.
        return;
    };
    let through_gpu = play(&mut gpu);

    assert_eq!(
        through_gpu, through_null,
        "the same scripted session left two different worlds behind it depending on which \
         backend drew it; everything above the seam is backend-agnostic (renderer.md §1), and \
         the upload path is the only place that claim can break"
    );
}

#[test]
fn the_null_backend_sees_the_art_arrive_on_the_tick_it_was_scripted_for() {
    // The guard on the test above: if the art never resolved, both halves would
    // agree about a session in which nothing was uploaded, and the comparison
    // would be checking that two backends can both do nothing.
    let outcome = play(&mut NullBackend::new());
    assert_eq!(
        outcome.art_status, "Ready",
        "the scripted texture never became ready, so the upload path this test exists to \
         compare was never exercised"
    );
    assert!(
        outcome.quads_in_last_frame > 0,
        "nothing was drawn, so no plan reached either backend"
    );
}
