//! What the asset store promises: immediate handles, frozen statuses between
//! commits, and the difference between a failure and a mistake (assets.md
//! §1, §4, §6).

use jidousha_assets::{AssetStatus, Assets, MemorySource};
use jidousha_core::{Resource, World};

fn source_with(paths: &[&str]) -> MemorySource {
    let mut source = MemorySource::new();
    for path in paths {
        source.insert(path, format!("bytes of {path}").into_bytes());
    }
    source
}

#[test]
fn a_load_hands_back_a_usable_handle_immediately() {
    let mut assets = Assets::new(source_with(&["player.png"]));
    let player = assets.load_texture("player.png");
    // No commit yet: the handle exists, the bytes do not.
    assert_eq!(assets.status(player), AssetStatus::Loading);
    assert_eq!(assets.path_of(player), "player.png");
    assert_eq!(assets.bytes_of(player), None);
}

#[test]
fn a_status_changes_only_at_a_commit() {
    let mut assets = Assets::new(source_with(&["a.png"]));
    let handle = assets.load_texture("a.png");

    // Reading repeatedly between commits gives one answer, however many times
    // it is asked — the contract the whole determinism story rests on.
    for _ in 0..5 {
        assert_eq!(assets.status(handle), AssetStatus::Loading);
    }
    assets.commit(1);
    for _ in 0..5 {
        assert_eq!(assets.status(handle), AssetStatus::Ready);
    }
}

#[test]
fn a_scripted_load_becomes_ready_on_its_tick_and_not_before() {
    let mut source = source_with(&["late.png"]);
    source.complete_at("late.png", 30);
    let mut assets = Assets::new(source);
    let handle = assets.load_texture("late.png");

    for tick in 1..30 {
        assets.commit(tick);
        assert_eq!(assets.status(handle), AssetStatus::Loading, "tick {tick}");
    }
    assets.commit(30);
    assert_eq!(assets.status(handle), AssetStatus::Ready);
}

#[test]
fn ready_assets_carry_their_bytes() {
    let mut assets = Assets::new(source_with(&["data.bin"]));
    let handle = assets.load_bytes("data.bin");
    assets.commit(1);
    assert_eq!(assets.bytes_of(handle), Some(&b"bytes of data.bin"[..]));
}

#[test]
fn a_missing_asset_fails_rather_than_loading_forever() {
    let mut assets = Assets::new(MemorySource::new());
    let handle = assets.load_texture("nowhere.png");
    let failures = assets.commit(1);
    assert_eq!(assets.status(handle), AssetStatus::Failed);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].path, "nowhere.png");
}

#[test]
fn a_failure_is_reported_once_and_not_every_commit() {
    let mut assets = Assets::new(MemorySource::new());
    let handle = assets.load_texture("nowhere.png");
    assert_eq!(assets.commit(1).len(), 1);
    for tick in 2..6 {
        assert!(
            assets.commit(tick).is_empty(),
            "the failure was already reported"
        );
    }
    assert_eq!(assets.status(handle), AssetStatus::Failed);
}

#[test]
fn a_failure_message_names_the_path_and_the_line_that_asked() {
    let mut assets = Assets::new(MemorySource::new());
    let _ = assets.load_texture("sprites/Player.png");
    let failures = assets.commit(1);
    let message = failures[0].message();
    assert!(message.starts_with("[jidousha] asset failed:"), "{message}");
    assert!(message.contains("sprites/Player.png"), "{message}");
    assert!(message.contains("asset_ops.rs"), "{message}");
    assert!(message.contains("likely cause:"), "{message}");
    assert!(message.contains("fix:"), "{message}");
}

#[test]
fn all_ready_is_false_while_anything_is_in_flight() {
    let mut source = source_with(&["a.png", "b.png"]);
    source.complete_at("b.png", 5);
    let mut assets = Assets::new(source);
    let _ = assets.load_texture("a.png");
    let _ = assets.load_texture("b.png");

    assets.commit(1);
    assert!(!assets.all_ready(), "b.png is still coming");
    assets.commit(5);
    assert!(assets.all_ready());
}

#[test]
fn all_ready_counts_a_failure_as_resolved() {
    // A game gating on all_ready must not wait forever for a file that will
    // never arrive.
    let mut assets = Assets::new(MemorySource::new());
    let _ = assets.load_texture("nowhere.png");
    assets.commit(1);
    assert!(assets.all_ready());
}

#[test]
fn an_empty_store_is_ready() {
    let assets = Assets::new(MemorySource::new());
    assert!(assets.all_ready());
}

#[test]
fn the_two_kinds_of_handle_do_not_collide() {
    let mut assets = Assets::new(source_with(&["a.png", "a.bin"]));
    let texture = assets.load_texture("a.png");
    let bytes = assets.load_bytes("a.bin");
    assets.commit(1);
    assert_eq!(assets.path_of(texture), "a.png");
    assert_eq!(assets.path_of(bytes), "a.bin");
}

#[test]
fn an_unloaded_slot_is_reused_by_the_next_load() {
    let mut assets = Assets::new(source_with(&["a.png", "b.png"]));
    let first = assets.load_texture("a.png");
    assets.unload(first);
    let second = assets.load_texture("b.png");
    assert_eq!(assets.path_of(second), "b.png");
    assert_ne!(
        format!("{first:?}"),
        format!("{second:?}"),
        "new generation"
    );
}

#[test]
fn unloading_something_still_in_flight_drops_its_bytes() {
    let mut source = source_with(&["a.png"]);
    source.complete_at("a.png", 10);
    let mut assets = Assets::new(source);
    let handle = assets.load_texture("a.png");
    assets.unload(handle);

    // The bytes arrive at tick 10 with nowhere to go; the commit must not
    // resurrect the slot or land them in whatever took its place.
    let reused = assets.load_texture("a.png");
    let failures = assets.commit(10);
    assert!(failures.is_empty());
    assert_eq!(assets.status(reused), AssetStatus::Ready);
}

#[test]
#[should_panic(expected = "asset handle used after unload")]
fn reading_the_status_of_an_unloaded_handle_panics() {
    let mut assets = Assets::new(source_with(&["a.png"]));
    let handle = assets.load_texture("a.png");
    assets.unload(handle);
    let _ = assets.status(handle);
}

#[test]
#[should_panic(expected = "asset handle used after unload")]
fn unloading_twice_panics() {
    let mut assets = Assets::new(source_with(&["a.png"]));
    let handle = assets.load_texture("a.png");
    assets.unload(handle);
    assets.unload(handle);
}

#[test]
#[should_panic(expected = "asset commit went backwards")]
fn committing_an_earlier_tick_panics() {
    let mut assets = Assets::new(MemorySource::new());
    assets.commit(10);
    assets.commit(9);
}

#[test]
fn committing_the_same_tick_twice_is_allowed_and_changes_nothing() {
    // A driver that commits, then commits again before ticking, must not see
    // different answers the second time.
    let mut assets = Assets::new(source_with(&["a.png"]));
    let handle = assets.load_texture("a.png");
    assets.commit(4);
    let second = assets.commit(4);
    assert!(second.is_empty());
    assert_eq!(assets.status(handle), AssetStatus::Ready);
}

#[test]
fn assets_live_in_the_world_like_any_other_resource() {
    fn assert_resource<T: Resource>() {}
    assert_resource::<Assets>();

    let mut world = World::new();
    world.insert_resource(Assets::new(source_with(&["a.png"])));
    let handle = world.resource_mut::<Assets>().load_texture("a.png");
    world.resource_mut::<Assets>().commit(1);
    assert_eq!(
        world.resource::<Assets>().status(handle),
        AssetStatus::Ready
    );
}
