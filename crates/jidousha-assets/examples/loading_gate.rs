//! The loading gate: ask for everything at startup, play when it has resolved.
//!
//! Shows the three things A0 exists for — loads that return immediately, a
//! single commit point per frame, and readiness that moves only at a numbered
//! tick. The gate itself is optional; the note at the bottom explains why most
//! games should not write one.
//!
//! Run it: `cargo run -p jidousha-assets --example loading_gate`
//!
//! DELIBERATE: this example drives the commit point by hand. The frame loop
//! arrives with the platform crate (M5) and will call `commit` before the
//! frame's first Update tick; until it exists, the driver is whoever wrote the
//! loop (assets.md §4).
//!
//! Note for A3: the paths below live in a `MemorySource`, not on disk, so
//! `tools/check-assets` must not treat them as broken asset references
//! (assets.md §7).

use jidousha_assets::{AssetStatus, Assets, MemorySource, TextureHandle};
use jidousha_core::{Component, GameConfig, Resource, Startup, Time, Update, World, headless};

/// What the game is doing right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    /// Waiting for the art. The game is running: this is a stage, not a stall.
    Loading,
    /// Everything asked for has resolved, one way or the other.
    Playing,
}
impl Resource for Stage {}

/// The art this game needs, held so systems can ask about it later.
struct Art {
    hero: TextureHandle,
    banner: TextureHandle,
}
impl Resource for Art {}

/// A thing on screen, so there is something for the art to belong to.
#[derive(Clone, Copy, Debug)]
struct Hero;
impl Component for Hero {}

/// Ask for everything up front. Neither call blocks, and neither can fail here.
fn request_the_art(world: &mut World) {
    let assets = world.resource_mut::<Assets>();
    let hero = assets.load_texture("hero.png");
    let banner = assets.load_texture("banner.png");
    world.insert_resource(Art { hero, banner });

    let entity = world.spawn();
    world.insert(entity, Hero);
}

/// Open the gate on the first tick where nothing is still in flight.
fn open_the_gate_when_ready(world: &mut World) {
    if *world.resource::<Stage>() == Stage::Playing {
        return;
    }
    if !world.resource::<Assets>().all_ready() {
        return;
    }
    let tick = world.resource::<Time>().tick;
    println!("tick {tick}: everything resolved — play");
    world.insert_resource(Stage::Playing);
}

fn main() {
    // The source stands in for a disk. `banner.png` is missing, which is the
    // interesting case: the gate must open anyway, or the game hangs forever
    // waiting for a file that is never coming.
    let mut source = MemorySource::new();
    source.insert("hero.png", b"pretend these are texels".to_vec());
    source.complete_at("hero.png", 4);

    let mut sim = headless(
        GameConfig {
            title: "loading gate",
            ..GameConfig::default()
        },
        |app| {
            app.add_system(Startup, request_the_art);
            app.add_system(Update, open_the_gate_when_ready);
        },
    );
    sim.world_mut().insert_resource(Assets::new(source));
    sim.world_mut().insert_resource(Stage::Loading);

    for tick in 1..=6 {
        // The commit point: the one moment statuses may change. Everything the
        // tick below does sees one consistent picture of what is ready.
        for failure in sim.world_mut().resource_mut::<Assets>().commit(tick) {
            // Reported once, at the commit that resolved it — not every frame.
            println!("{}", failure.message());
        }
        sim.tick();

        let art = sim.world().resource::<Art>();
        let assets = sim.world().resource::<Assets>();
        println!(
            "tick {tick}: hero={:?} banner={:?} stage={:?}",
            assets.status(art.hero),
            assets.status(art.banner),
            sim.world().resource::<Stage>()
        );
    }

    let art = sim.world().resource::<Art>();
    let assets = sim.world().resource::<Assets>();
    assert_eq!(assets.status(art.hero), AssetStatus::Ready);
    assert_eq!(
        assets.status(art.banner),
        AssetStatus::Failed,
        "a missing file resolves as Failed — it does not load forever"
    );
    assert_eq!(*sim.world().resource::<Stage>(), Stage::Playing);

    // What most games should do instead: nothing. Draw the hero from tick one
    // and let the renderer show a placeholder until the texels arrive — no
    // gate, no stage enum, no waiting. The gate above is for the cases that
    // genuinely cannot start early, like a level whose layout is still loading.
    println!("gate opened, and most games would not have needed one");
}
