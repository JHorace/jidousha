//! Test scaffolding shared by both halves of the driver.
//!
//! Key types: `SharedBackend`, `Seen`, `Frames`.
//! Its own file because `mod.rs` and `frame.rs` both need it and neither owns
//! it. Compiled only under `cfg(test)`.

use jidousha_assets::TextureData;
use jidousha_core::{Draw, DrawCtx, GameConfig, Resource, Seconds, Update, World, build};
use jidousha_input::{Input, Key};
use jidousha_render_core::{
    BackendTextureId, FramePlan, NullBackend, PhysicalSize, RawImage, RenderBackend, RenderError,
    TextureDesc, create_builtin_textures,
};

use super::Driver;

/// A backend the test still holds a handle to after giving it away.
///
/// `Driver` owns its backend, so a test that wants to ask what the backend
/// was told needs a shared record. This forwards everything to a
/// `NullBackend` behind a mutex and hands the same one back — which is what
/// makes the frame path below checkable without a window or a GPU.
#[derive(Clone)]
pub(crate) struct SharedBackend(std::sync::Arc<std::sync::Mutex<NullBackend>>);

impl SharedBackend {
    pub(crate) fn new() -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(
            NullBackend::new(),
        )))
    }

    pub(crate) fn read<T>(&self, question: impl FnOnce(&NullBackend) -> T) -> T {
        let Ok(backend) = self.0.lock() else {
            panic!("a test thread panicked while holding the backend");
        };
        question(&backend)
    }

    pub(crate) fn write<T>(&mut self, action: impl FnOnce(&mut NullBackend) -> T) -> T {
        let Ok(mut backend) = self.0.lock() else {
            panic!("a test thread panicked while holding the backend");
        };
        action(&mut backend)
    }
}

impl RenderBackend for SharedBackend {
    fn create_texture(&mut self, desc: &TextureDesc, texels: &[u8]) -> BackendTextureId {
        self.write(|backend| backend.create_texture(desc, texels))
    }
    fn destroy_texture(&mut self, id: BackendTextureId) {
        self.write(|backend| backend.destroy_texture(id));
    }
    fn resize_surface(&mut self, size: PhysicalSize) {
        self.write(|backend| backend.resize_surface(size));
    }
    fn render(&mut self, plan: &FramePlan) -> Result<(), RenderError> {
        self.write(|backend| backend.render(plan))
    }
    fn capture(&mut self) -> Result<RawImage, RenderError> {
        self.write(NullBackend::capture)
    }
}

/// A driver with a backend installed, as `resumed` would install a real one.
pub(crate) fn driver_with_a_backend() -> (Driver, SharedBackend) {
    let mut driver = driver();
    let mut backend = SharedBackend::new();
    driver.textures = Some(create_builtin_textures(&mut backend));
    driver.backend = Some(Box::new(backend.clone()));
    (driver, backend)
}

/// The smallest texture there is, for tests about timing rather than pixels.
pub(crate) fn one_texel() -> TextureData {
    TextureData {
        width: 1,
        height: 1,
        rgba: vec![255, 255, 255, 255],
    }
}

/// What `Input` said, once per tick, in tick order.
#[derive(Debug, Default)]
pub(crate) struct Seen {
    pub(crate) pressed: Vec<bool>,
    pub(crate) held: Vec<bool>,
    pub(crate) released: Vec<bool>,
}
impl Resource for Seen {}

/// How many times the Draw phase ran.
///
/// An atomic because Draw systems cannot write the world (ADR-0008), and
/// resources are `Send + Sync`. This is the interior mutability core's
/// world-shape check is defense-in-depth against — used here on purpose,
/// by a test, to count something the world's *shape* does not record.
#[derive(Debug, Default)]
pub(crate) struct Frames(std::sync::atomic::AtomicU32);
impl Resource for Frames {}

pub(crate) fn count_the_frame(ctx: &mut DrawCtx) {
    ctx.world
        .resource::<Frames>()
        .0
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn frames_drawn(driver: &Driver) -> u32 {
    driver
        .simulation
        .world()
        .resource::<Frames>()
        .0
        .load(std::sync::atomic::Ordering::Relaxed)
}

pub(crate) fn watch_the_key(world: &mut World) {
    let (pressed, held, released) = {
        let input = world.resource::<Input>();
        (
            input.just_pressed(Key::A),
            input.held(Key::A),
            input.just_released(Key::A),
        )
    };
    let seen = world.resource_mut::<Seen>();
    seen.pressed.push(pressed);
    seen.held.push(held);
    seen.released.push(released);
}

/// A driver with the watcher registered and nothing else.
pub(crate) fn driver() -> Driver {
    let config = GameConfig::default();
    let simulation = build(config, |app| {
        app.add_system(Update, watch_the_key);
        app.add_system(Draw, count_the_frame);
    });
    let mut driver = Driver::new(config, simulation);
    driver
        .simulation
        .world_mut()
        .insert_resource(Seen::default());
    driver
        .simulation
        .world_mut()
        .insert_resource(Frames::default());
    driver
}

/// Long enough for exactly `ticks` ticks at the default timestep.
///
/// With a nudge, because `fixed_dt` is `1.0 / 60.0` rounded to f32 and
/// `ticks / 60.0` is not: three ticks' worth of the former is a hair more
/// than the latter, and without the nudge this asks for three and gets two.
pub(crate) fn frames_worth(ticks: u32) -> Seconds {
    Seconds(ticks as f32 / 60.0 + 1e-4)
}
