//! The other A0 exit criterion: the same script produces the same per-tick
//! statuses, every time (assets.md §8).
//!
//! This is the asset half of the determinism contract. Load timing is the most
//! environmental thing an engine touches — disk speed, cache warmth, a network
//! — so readiness is only replayable if it is a function of the *script* and
//! nothing else. A transcript that varies between two runs of one script is the
//! first symptom, and this test is the one that would catch it.

mod support;

use std::fmt::Write as _;

use jidousha_assets::{AssetStatus, Assets, MemorySource};
use support::{CATALOG, Handle, Op, generate, source};

/// Every observable thing the store said, in order, as text a diff can read.
fn transcript(ops: &[Op]) -> String {
    let mut assets = Assets::new(source());
    let mut live: Vec<Handle> = Vec::new();
    let mut tick = 0;
    let mut out = String::new();

    for op in ops {
        match *op {
            Op::Load { index, as_texture } => {
                let entry = CATALOG[index];
                let handle = if as_texture {
                    Handle::Texture(assets.load_texture(entry.path))
                } else {
                    Handle::Bytes(assets.load_bytes(entry.path))
                };
                // The handle itself is part of the transcript: two runs that
                // recycled slots differently would diverge from here on.
                let _ = writeln!(out, "load {} -> {}", entry.path, handle.debug());
                live.push(handle);
            }
            Op::Commit { advance } => {
                tick += advance;
                let failures: Vec<String> = assets
                    .commit(tick)
                    .into_iter()
                    .map(|failure| failure.path)
                    .collect();
                let _ = writeln!(out, "commit {tick} failed={failures:?}");
            }
            Op::Unload { target } => {
                if live.is_empty() {
                    continue;
                }
                let handle = live.remove(target % live.len());
                let _ = writeln!(out, "unload {}", handle.debug());
                handle.unload(&mut assets);
            }
        }
        // The per-tick picture: every handle the game still holds, in the order
        // it asked for them.
        let statuses: Vec<String> = live
            .iter()
            .map(|handle| format!("{:?}", handle.status(&assets)))
            .collect();
        let _ = writeln!(
            out,
            "  tick {tick} {statuses:?} all_ready={}",
            assets.all_ready()
        );
    }
    out
}

#[test]
fn the_same_script_produces_the_same_per_tick_statuses() {
    for seed in 0..200 {
        let ops = generate(seed, 80);
        let first = transcript(&ops);
        let second = transcript(&ops);
        assert_eq!(first, second, "seed {seed} replayed differently");
    }
}

#[test]
fn a_different_script_produces_a_different_transcript() {
    // Teeth for the test above: comparing two constants would also pass it.
    let first = transcript(&generate(1, 80));
    let second = transcript(&generate(2, 80));
    assert_ne!(first, second);
}

#[test]
fn a_replayed_transcript_actually_contains_transitions() {
    // More teeth: a script whose assets never resolved would replay identically
    // and prove nothing.
    let text = transcript(&generate(7, 200));
    assert!(text.contains("Loading"), "{text}");
    assert!(text.contains("Ready"), "{text}");
    assert!(text.contains("Failed"), "{text}");
}

#[test]
fn a_scripted_load_swaps_in_at_its_tick_and_only_there() {
    // The golden transcript from assets.md §7: placeholder until the scripted
    // tick, real texture from then on, and a failure reported exactly once.
    let mut source = MemorySource::new();
    source.insert("hero.png", b"texels".to_vec());
    source.complete_at("hero.png", 3);
    let mut assets = Assets::new(source);

    let hero = assets.load_texture("hero.png");
    let ghost = assets.load_texture("ghost.png");

    let mut out = String::new();
    for tick in 1..=5 {
        let failures: Vec<String> = assets
            .commit(tick)
            .into_iter()
            .map(|failure| failure.path)
            .collect();
        let _ = writeln!(
            out,
            "{tick}: hero={:?} ghost={:?} failed={failures:?} all_ready={}",
            assets.status(hero),
            assets.status(ghost),
            assets.all_ready()
        );
    }

    assert_eq!(
        out,
        "\
1: hero=Loading ghost=Failed failed=[\"ghost.png\"] all_ready=false
2: hero=Loading ghost=Failed failed=[] all_ready=false
3: hero=Ready ghost=Failed failed=[] all_ready=true
4: hero=Ready ghost=Failed failed=[] all_ready=true
5: hero=Ready ghost=Failed failed=[] all_ready=true
"
    );
    assert_eq!(assets.bytes_of(hero), Some(&b"texels"[..]));
    assert_eq!(assets.status(ghost), AssetStatus::Failed);
}
