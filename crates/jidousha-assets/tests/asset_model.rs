//! The A0 exit criterion: the real store and a naive reference model agree
//! under thousands of random load/commit/unload sequences (assets.md §8).
//!
//! A failure prints the seed and the shortest failing prefix of the sequence,
//! so any mismatch is reproducible by re-running that one seed.

mod support;

use jidousha_assets::{AssetStatus, Assets};
use support::{CATALOG, Handle, Op, Reference, generate, source};

/// How many independent sequences to run, and how long each one is.
const SEQUENCES: u64 = 2000;
const SEQUENCE_LENGTH: usize = 120;

/// Both stores, driven in lockstep, compared after every operation.
struct Pair {
    assets: Assets,
    model: Reference,
    /// Handles the game still holds, paired with the model's key for each.
    live: Vec<(Handle, usize)>,
    tick: u64,
    seen: Seen,
}

/// What a run actually exercised. A sequence that never resolved anything would
/// compare `Loading` to `Loading` two thousand times and pass.
#[derive(Debug, Default, PartialEq, Eq)]
struct Seen {
    ready: usize,
    failed: usize,
    /// Unloads of assets whose bytes had not arrived yet — the case where a
    /// completion has to be dropped rather than delivered.
    unloaded_in_flight: usize,
    /// Handles whose slot index had been used by an earlier, unloaded handle.
    reused_slots: usize,
    /// Readings of a `Ready` texture that had texels — the store decoded a
    /// file's bytes at the commit (FINDINGS G-006).
    decoded: usize,
}

impl Pair {
    fn new() -> Self {
        Self {
            assets: Assets::new(source()),
            model: Reference::new(),
            live: Vec::new(),
            tick: 0,
            seen: Seen::default(),
        }
    }

    /// Apply one operation to both stores, returning the first disagreement.
    fn apply(&mut self, op: Op, retired: &mut Vec<String>) -> Result<(), String> {
        match op {
            Op::Load { index } => {
                let entry = CATALOG[index];
                let handle = if entry.texture {
                    Handle::Texture(self.assets.load_texture(entry.path))
                } else {
                    Handle::Bytes(self.assets.load_bytes(entry.path))
                };
                let printed = handle.debug();
                if retired.contains(&printed) {
                    return Err(format!(
                        "load handed back {printed}, which an earlier unloaded handle already \
                         printed — a stale handle would now name a live asset"
                    ));
                }
                if retired.iter().any(|old| slot_of(old) == slot_of(&printed)) {
                    self.seen.reused_slots += 1;
                }
                let key = self.model.load(entry);
                self.live.push((handle, key));
            }
            Op::Commit { advance } => {
                self.tick += advance;
                let got: Vec<String> = self
                    .assets
                    .commit(self.tick)
                    .into_iter()
                    .map(|failure| failure.path)
                    .collect();
                let want = self.model.commit(self.tick);
                if got != want {
                    return Err(format!(
                        "commit({}) reported failures {got:?}, model expected {want:?}",
                        self.tick
                    ));
                }
            }
            Op::Unload { target } => {
                if self.live.is_empty() {
                    return Ok(());
                }
                let (handle, key) = self.live.remove(target % self.live.len());
                if self.model.status(key) == AssetStatus::Loading {
                    self.seen.unloaded_in_flight += 1;
                }
                retired.push(handle.debug());
                handle.unload(&mut self.assets);
                self.model.unload(key);
            }
        }
        self.compare()
    }

    /// Compare everything observable: every handle the game still holds, and
    /// the one-line gate that summarises all of them.
    fn compare(&mut self) -> Result<(), String> {
        for (handle, key) in &self.live {
            let (handle, key) = (*handle, *key);

            let got = handle.status(&self.assets);
            let want = self.model.status(key);
            if got != want {
                return Err(format!(
                    "status({}) is {got:?}, model says {want:?}",
                    handle.debug()
                ));
            }
            match got {
                AssetStatus::Ready => self.seen.ready += 1,
                AssetStatus::Failed => self.seen.failed += 1,
                AssetStatus::Loading => {}
            }

            let got = handle.path_of(&self.assets);
            let want = self.model.path(key);
            if got != want {
                return Err(format!(
                    "path_of({}) is {got:?}, model says {want:?}",
                    handle.debug()
                ));
            }

            let got = handle.bytes_of(&self.assets);
            let want = self.model.bytes(key);
            if got != want {
                return Err(format!(
                    "bytes_of({}) is {got:?}, model says {want:?}",
                    handle.debug()
                ));
            }

            // The G-006 property, checked on every handle after every
            // operation: a `Ready` texture has texels. The catalogue's pictures
            // are inserted as PNG *files*, so the only way for this to hold is
            // for the store to have decoded them at the commit.
            let got = handle.texels(&self.assets);
            let want = self.model.texels(key);
            if got != want {
                return Err(format!(
                    "texels({}) is {got:?}, model says {want:?}",
                    handle.debug()
                ));
            }
            if got.is_some() {
                self.seen.decoded += 1;
            }
        }

        if self.assets.all_ready() != self.model.all_ready() {
            return Err(format!(
                "all_ready is {}, model says {}",
                self.assets.all_ready(),
                self.model.all_ready()
            ));
        }
        Ok(())
    }
}

/// A printed handle without its generation: `TextureHandle(3 v2)` →
/// `TextureHandle(3`. Two handles sharing this share a slot, and the kind is
/// part of it because each kind has its own table.
fn slot_of(debug: &str) -> &str {
    debug.split_once(' ').map_or(debug, |(slot, _)| slot)
}

/// Run a sequence, returning what it exercised.
fn run(ops: &[Op]) -> Result<Seen, String> {
    let mut pair = Pair::new();
    let mut retired = Vec::new();
    for (step, op) in ops.iter().enumerate() {
        pair.apply(*op, &mut retired)
            .map_err(|mismatch| format!("step {step} ({op:?}): {mismatch}"))?;
    }
    Ok(pair.seen)
}

/// The shortest prefix that still fails — a poor agent's shrinker, and enough
/// to turn a 120-operation sequence into something readable.
fn shortest_failing_prefix(ops: &[Op]) -> &[Op] {
    for length in 1..=ops.len() {
        if run(&ops[..length]).is_err() {
            return &ops[..length];
        }
    }
    ops
}

#[test]
fn the_store_matches_the_reference_model_under_random_operation_sequences() {
    for seed in 0..SEQUENCES {
        let ops = generate(seed, SEQUENCE_LENGTH);
        if let Err(mismatch) = run(&ops) {
            let prefix = shortest_failing_prefix(&ops);
            panic!(
                "seed {seed}: {mismatch}\nshortest failing prefix ({} ops):\n{prefix:#?}",
                prefix.len()
            );
        }
    }
}

/// Guards the generator, not the store: each of these is a state the comparison
/// above is worthless without, and none of them is guaranteed by construction.
#[test]
fn the_generated_sequences_reach_every_interesting_state() {
    let mut total = Seen::default();
    for seed in 0..SEQUENCES {
        let Ok(seen) = run(&generate(seed, SEQUENCE_LENGTH)) else {
            // The comparison test above reports the mismatch properly; this one
            // only cares about coverage.
            continue;
        };
        total.ready += seen.ready;
        total.failed += seen.failed;
        total.unloaded_in_flight += seen.unloaded_in_flight;
        total.reused_slots += seen.reused_slots;
        total.decoded += seen.decoded;
    }
    assert!(total.ready > 0, "no asset ever became Ready: {total:?}");
    assert!(total.failed > 0, "no asset ever Failed: {total:?}");
    assert!(
        total.unloaded_in_flight > 0,
        "nothing was ever unloaded while still in flight: {total:?}"
    );
    assert!(
        total.reused_slots > 0,
        "no unloaded slot was ever reused: {total:?}"
    );
    assert!(
        total.decoded > 0,
        "no texture was ever decoded from a file's bytes: {total:?}"
    );
}
