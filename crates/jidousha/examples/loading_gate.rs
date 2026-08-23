//! The loading gate: ask for everything at startup, play when it has resolved.
//!
//! Shows the three things A0 exists for — loads that return immediately, a
//! single commit point per frame, and readiness that moves only at a numbered
//! tick. The gate itself is optional; the note at the bottom explains why most
//! games should not write one.
//!
//! Run it: `cargo run -p jidousha --example loading_gate`
//!
//! DELIBERATE: this example drives the commit point by hand. The frame loop
//! arrives with the platform crate (M5) and will call `commit` before the
//! frame's first Update tick; until it exists, the driver is whoever wrote the
//! loop (assets.md §4).
//!
//! Note for A3: the paths below live in a `MemorySource`, not on disk, so
//! `tools/check-assets` must not treat them as broken asset references
//! (assets.md §7).

use jidousha::prelude::*;
use jidousha::testing::TextureData;

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
    // Texels rather than a file's bytes, because this example is about *when* a
    // load resolves and not about what is in it. A store handed a real PNG's
    // bytes with `insert` works the same way — the store decodes them when the
    // texture request resolves, and bytes that are not a picture resolve
    // `Failed` exactly as the missing `banner.png` does below.
    source.insert_texture(
        "hero.png",
        TextureData {
            width: 2,
            height: 2,
            rgba: vec![255; 16],
        },
    );
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

    // Which tick the gate opened on, so the assertions below can say *when*
    // rather than only *whether*. A run only tests the states it reaches, and a
    // gate that was never shut reaches the same final state as one that worked:
    // asserting `Stage::Playing` at the end passes for a game that started
    // there. Recorded rather than derived, because that is the whole claim.
    let mut opened_at = None;

    for tick in 1..=6 {
        // The commit point: the one moment statuses may change. Everything the
        // tick below does sees one consistent picture of what is ready.
        for failure in sim.world_mut().resource_mut::<Assets>().commit(tick) {
            // Reported once, at the commit that resolved it — not every frame.
            println!("{}", failure.message());
        }
        sim.tick();
        if opened_at.is_none() && *sim.world().resource::<Stage>() == Stage::Playing {
            opened_at = Some(tick);
        }

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
    // And it was shut until everything resolved. `hero.png` completes at 4 and
    // the missing `banner.png` resolves at the same commit, so the gate opens
    // on the tick after the last commit that could change a status — never
    // before, and never not at all.
    assert_eq!(
        opened_at,
        Some(4),
        "the gate opened on tick {opened_at:?}; hero.png completes at tick 4 and the gate \
         must stay shut until then and open once it has"
    );

    // What most games should do instead: nothing. Draw the hero from tick one
    // and let the renderer show a placeholder until the texels arrive — no
    // gate, no stage enum, no waiting. The gate above is for the cases that
    // genuinely cannot start early, like a level whose layout is still loading.
    println!("gate opened, and most games would not have needed one");
}
