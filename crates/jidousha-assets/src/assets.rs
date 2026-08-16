//! The asset store: what is loading, what is ready, and the one moment either
//! can change.
//!
//! Key types: `Assets`, `AssetStatus`, `AssetFailure`.
//! Depends on: `handle`, `source`, `jidousha-core` (for `Resource` and the §9
//! message format). Must never depend on: any I/O.
//! INVARIANT: statuses change **only** inside [`Assets::commit`]. Load timing is
//! environmental — disk speed, network, cache — and if simulation could observe
//! it at arbitrary moments the same game would diverge between machines. One
//! commit point per frame is what makes readiness part of the recorded timeline
//! instead (assets.md §4 CONTRACT).

use std::collections::BTreeMap;

use jidousha_core::{Resource, TextureId, message};

use crate::handle::{AssetHandle, AssetId, AssetKind, BytesHandle, IdAllocator, TextureHandle};
use crate::payload::{AssetError, Payload, TextureData};
use crate::source::{ByteSource, RequestId};

/// Where an asset is in its life.
///
/// `Loading → Ready` or `Loading → Failed`, and nothing else. A handle that has
/// been unloaded has no status at all: using it is a contract violation, not a
/// state (assets.md §1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetStatus {
    /// Asked for; not here yet. Draw it anyway — the renderer shows a
    /// placeholder (ADR-0011).
    Loading,
    /// Here, and usable.
    Ready,
    /// It will not arrive. The renderer keeps showing the placeholder, and the
    /// failure is reported once (assets.md §6).
    Failed,
}

/// One asset that will not arrive, reported once at the commit that resolved it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetFailure {
    /// The path as the game asked for it.
    pub path: String,
    /// Which kind of load this was.
    pub kind: AssetKind,
    /// Where the game asked, so the message points at the game's line rather
    /// than the loader's.
    pub requested_at: String,
    /// What went wrong, from the source.
    pub error: AssetError,
}

impl AssetFailure {
    /// The failure in the engine's message format (core.md §9).
    ///
    /// Each failure class says something specific — a case mismatch names the
    /// file that is actually there, an oversized image names its size and the
    /// limit (assets.md §6). The formatting lives on [`AssetError`] so a source
    /// can produce the same sentences without a store to put them in.
    #[must_use]
    pub fn message(&self) -> String {
        self.error
            .message(&self.path, self.kind, &self.requested_at)
    }
}

/// One asset that resolved at a commit, one way or the other.
///
/// What a recorder writes down: which request, and whether it arrived. Not the
/// payload — a recording is a timeline, not an archive of everybody's art, and
/// the payload is the one part simulation is forbidden to observe anyway
/// (assets.md §4, input.md §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resolution {
    /// Which request this answers.
    pub request: RequestId,
    /// Whether it arrived. `false` is a load that failed.
    pub arrived: bool,
}

/// One texture's texels, on their way to the GPU.
///
/// Handed over by [`Assets::take_uploads`] exactly once, and **moved** rather
/// than lent: the renderer is the only thing allowed to read texels
/// (renderer.md §3), so once it has them the store has no reader left to serve
/// and keeps only the status and the path (ADR-0016).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureUpload {
    /// The texture in the renderer's vocabulary, matching what a [`Quad`]
    /// carries.
    ///
    /// [`Quad`]: jidousha_core::Quad
    pub id: TextureId,
    /// The texels, size included.
    pub data: TextureData,
}

/// One slot's bookkeeping.
#[derive(Debug)]
struct Entry {
    path: String,
    status: AssetStatus,
    request: RequestId,
    requested_at: String,
    /// What arrived, once it did.
    data: Option<Payload>,
}

/// One table per asset kind.
struct Table {
    allocator: IdAllocator,
    entries: Vec<Option<Entry>>,
}

impl Table {
    fn new() -> Self {
        Self {
            allocator: IdAllocator::new(),
            entries: Vec::new(),
        }
    }

    fn insert(&mut self, entry: Entry) -> AssetId {
        let id = self.allocator.create();
        if id.index() >= self.entries.len() {
            self.entries.resize_with(id.index() + 1, || None);
        }
        self.entries[id.index()] = Some(entry);
        id
    }
}

/// Every asset the game has asked for.
///
/// Held as a world resource, so any system can load without new plumbing:
///
/// ```
/// # use jidousha_assets::{Assets, MemorySource};
/// # use jidousha_core::World;
/// # let mut world = World::new();
/// # let mut source = MemorySource::new();
/// # source.insert("player.png", vec![0]);
/// world.insert_resource(Assets::new(source));
///
/// let player = world.resource_mut::<Assets>().load_texture("player.png");
/// # let _ = player;
/// ```
///
/// Loads never block and never fail at the call site (ADR-0011): `load_texture`
/// hands back a usable handle immediately, and whether the bytes ever arrive is
/// a question for later — usually one the game never has to ask, because the
/// renderer draws a placeholder meanwhile.
pub struct Assets {
    textures: Table,
    bytes: Table,
    source: Box<dyn ByteSource + Send + Sync>,
    /// Request → where it landed, so a completion finds its slot.
    routes: BTreeMap<RequestId, (AssetKind, AssetId)>,
    failures: Vec<AssetFailure>,
    /// What the most recent commit resolved, in the order it applied them.
    ///
    /// Kept for one commit only: a recorder reads it immediately after
    /// committing, and holding it longer would invite reading a stale tick's
    /// answer as if it were this one's.
    resolved: Vec<Resolution>,
    /// Textures that turned `Ready` and have not been handed to a renderer yet,
    /// in the order they committed.
    ///
    /// A queue rather than an immediate hand-off because there may be no
    /// renderer to hand to: a window arrives a few frames after the program
    /// starts, and a headless run never has one at all. Waiting here costs
    /// nothing — the texels were already in the store — and means art that
    /// finished loading before the GPU did still reaches it (renderer.md §5).
    pending_uploads: Vec<AssetId>,
    /// Commits so far, purely so `commit` can reject going backwards.
    last_commit: Option<u64>,
}

impl Resource for Assets {}

impl Assets {
    /// A store that pulls bytes from `source`.
    ///
    /// The platform crates supply the real sources; [`MemorySource`](crate::MemorySource)
    /// is the one for tests and `tools/verify`.
    #[must_use]
    pub fn new(source: impl ByteSource) -> Self {
        Self {
            textures: Table::new(),
            bytes: Table::new(),
            source: Box::new(source),
            routes: BTreeMap::new(),
            failures: Vec::new(),
            resolved: Vec::new(),
            pending_uploads: Vec::new(),
            last_commit: None,
        }
    }

    /// Ask for an image. Returns immediately, always.
    ///
    /// The handle is usable at once; the texels arrive later, or not at all.
    #[track_caller]
    pub fn load_texture(&mut self, path: &str) -> TextureHandle {
        TextureHandle(self.load(AssetKind::Texture, path))
    }

    /// Ask for raw bytes — anything the engine does not decode itself.
    #[track_caller]
    pub fn load_bytes(&mut self, path: &str) -> BytesHandle {
        BytesHandle(self.load(AssetKind::Bytes, path))
    }

    /// Where `handle` is in its life.
    ///
    /// CONTRACT: the answer changes only at a [`commit`](Assets::commit), so two
    /// reads within one tick always agree, and a replay sees the same
    /// transitions at the same ticks as the original run.
    ///
    /// # Panics
    ///
    /// If `handle` was unloaded — a contract violation, distinct from `Failed`.
    /// A missing file is a fact about the world; using a handle you threw away
    /// is a bug in the game (assets.md §1).
    #[must_use]
    pub fn status<H: AssetHandle>(&self, handle: H) -> AssetStatus {
        self.entry(handle).status
    }

    /// The bytes behind `handle`, if it is `Ready` and holds bytes.
    ///
    /// `None` for a texture: what a texture holds is texels and a size, not a
    /// file — see [`texture_of`](Assets::texture_of).
    ///
    /// # Panics
    ///
    /// If `handle` was unloaded.
    #[must_use]
    pub fn bytes_of<H: AssetHandle>(&self, handle: H) -> Option<&[u8]> {
        match self.entry(handle).data.as_ref() {
            Some(Payload::Bytes(bytes)) => Some(bytes),
            _ => None,
        }
    }

    /// The decoded image behind `handle`, while the store still holds it.
    ///
    /// CONTRACT: **simulation must not read this** — nothing in a game's logic
    /// may depend on texture dimensions, or the same game behaves differently
    /// when the art is re-exported (renderer.md §3).
    ///
    /// `None` once [`take_uploads`](Assets::take_uploads) has handed the texels
    /// to a renderer, which is the normal end state in a windowed game: they
    /// live on the GPU from then on, and a second copy here would serve nobody
    /// (ADR-0016). A headless run has no renderer, so nothing takes them and
    /// this keeps answering — which is what the asset tests and `tools/verify`
    /// read.
    ///
    /// # Panics
    ///
    /// If `handle` was unloaded.
    #[must_use]
    pub fn texture_of(&self, handle: TextureHandle) -> Option<&TextureData> {
        match self.entry(handle).data.as_ref() {
            Some(Payload::Texture(texture)) => Some(texture),
            _ => None,
        }
    }

    /// Every texture that has become `Ready` since the last call, with texels.
    ///
    /// The renderer-facing half of the store: `jidousha-render-core` calls this
    /// once a frame and uploads what it gets (assets.md §5). Each texture is
    /// handed over exactly once, in commit order, so two runs that ready the
    /// same assets at the same ticks upload them in the same order.
    ///
    /// A texture unloaded between the commit that readied it and this call is
    /// dropped rather than handed over: the game said it was finished with it,
    /// and uploading it would put art on the GPU that nothing can draw.
    pub fn take_uploads(&mut self) -> Vec<TextureUpload> {
        let pending = core::mem::take(&mut self.pending_uploads);
        let mut uploads = Vec::with_capacity(pending.len());
        for id in pending {
            // The generation check is what stops a recycled slot answering in
            // place of the one that was unloaded.
            if !self.textures.allocator.is_live(id) {
                continue;
            }
            let Some(Some(entry)) = self.textures.entries.get_mut(id.index()) else {
                continue;
            };
            match entry.data.take() {
                Some(Payload::Texture(data)) => uploads.push(TextureUpload {
                    id: TextureHandle(id).texture_id(),
                    data,
                }),
                // Not a texture, or already taken. Neither should happen — only
                // texture completions are queued, and the queue is drained —
                // but putting it back is the honest response to being wrong.
                other => entry.data = other,
            }
        }
        uploads
    }

    /// The path `handle` was loaded from.
    ///
    /// # Panics
    ///
    /// If `handle` was unloaded.
    #[must_use]
    pub fn path_of<H: AssetHandle>(&self, handle: H) -> &str {
        &self.entry(handle).path
    }

    /// Whether every load asked for so far has resolved.
    ///
    /// The one-line loading gate for games that want one. `Failed` counts as
    /// resolved: it will not become anything else, and a game waiting for it
    /// would wait forever.
    #[must_use]
    pub fn all_ready(&self) -> bool {
        self.entries()
            .all(|entry| entry.status != AssetStatus::Loading)
    }

    /// Throw `handle` away, freeing what it held.
    ///
    /// # Panics
    ///
    /// If `handle` was already unloaded.
    pub fn unload<H: AssetHandle>(&mut self, handle: H) {
        self.expect_live(handle);
        let id = handle.id();
        let table = self.table_mut(handle.kind());
        if let Some(entry) = table.entries[id.index()].take() {
            // A load still in flight loses its route: the bytes will arrive and
            // be dropped, rather than landing in a slot someone else now owns.
            self.routes.remove(&entry.request);
        }
        self.table_mut(handle.kind()).allocator.destroy(id);
    }

    /// Apply everything that finished, and nothing else.
    ///
    /// CONTRACT: this is the **only** place statuses change. A driver calls it
    /// once per frame, before the frame's first Update tick, so every tick of
    /// that frame sees one consistent picture of what is ready (assets.md §4).
    ///
    /// Returns the failures resolved by this commit, each reported exactly once
    /// — the placeholder does the per-frame signalling from then on.
    ///
    /// # Panics
    ///
    /// If `tick` is earlier than the last commit's: readiness is part of the
    /// timeline, and a timeline that runs backwards is a bug in the driver.
    pub fn commit(&mut self, tick: u64) -> Vec<AssetFailure> {
        if let Some(last) = self.last_commit
            && tick < last
        {
            panic!(
                "{}",
                message(
                    &format!("asset commit went backwards: tick {tick} after tick {last}"),
                    "readiness is part of the recorded timeline, so commits move forward only",
                    "the driver called commit with a stale tick, or the clock was rewound",
                    "commit once per frame with the current tick; to replay from the start, \
                     build a fresh Assets",
                )
            );
        }
        self.last_commit = Some(tick);
        self.resolved.clear();

        for completion in self.source.drain_completed(tick) {
            // A route is absent when the handle was unloaded while its bytes
            // were still in flight. Dropping them is the whole point.
            let Some((kind, id)) = self.routes.remove(&completion.request) else {
                continue;
            };
            let table = match kind {
                AssetKind::Texture => &mut self.textures,
                AssetKind::Bytes => &mut self.bytes,
            };
            let Some(Some(entry)) = table.entries.get_mut(id.index()) else {
                continue;
            };
            self.resolved.push(Resolution {
                request: completion.request,
                arrived: completion.result.is_ok(),
            });
            match completion.result {
                Ok(payload) => {
                    entry.status = AssetStatus::Ready;
                    // Queued here rather than at `take_uploads` time, because
                    // this is the moment that is on the timeline: the upload
                    // order follows the commit order, which is recorded and
                    // replayable, rather than whatever a table walk happens to
                    // produce (assets.md §4).
                    if matches!(payload, Payload::Texture(_)) {
                        self.pending_uploads.push(id);
                    }
                    entry.data = Some(payload);
                }
                Err(error) => {
                    entry.status = AssetStatus::Failed;
                    self.failures.push(AssetFailure {
                        path: entry.path.clone(),
                        kind,
                        requested_at: entry.requested_at.clone(),
                        error,
                    });
                }
            }
        }
        core::mem::take(&mut self.failures)
    }

    /// What the most recent [`commit`](Assets::commit) resolved, in order.
    ///
    /// The recorder's half of the commit point: readiness is part of the
    /// recorded timeline (assets.md §4), and this is how it gets written down.
    /// Cleared and refilled by every commit, so it always describes the last
    /// one.
    #[must_use]
    pub fn resolved(&self) -> &[Resolution] {
        &self.resolved
    }

    #[track_caller]
    fn load(&mut self, kind: AssetKind, path: &str) -> AssetId {
        let request = self.source.request(path, kind);
        let entry = Entry {
            path: path.to_owned(),
            status: AssetStatus::Loading,
            request,
            // Recorded here so a failure points at the game's line rather than
            // the loader's (assets.md §6).
            requested_at: core::panic::Location::caller().to_string(),
            data: None,
        };
        let id = self.table_mut(kind).insert(entry);
        self.routes.insert(request, (kind, id));
        id
    }

    fn table(&self, kind: AssetKind) -> &Table {
        match kind {
            AssetKind::Texture => &self.textures,
            AssetKind::Bytes => &self.bytes,
        }
    }

    fn table_mut(&mut self, kind: AssetKind) -> &mut Table {
        match kind {
            AssetKind::Texture => &mut self.textures,
            AssetKind::Bytes => &mut self.bytes,
        }
    }

    fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.textures
            .entries
            .iter()
            .chain(&self.bytes.entries)
            .flatten()
    }

    fn entry<H: AssetHandle>(&self, handle: H) -> &Entry {
        self.expect_live(handle);
        let table = self.table(handle.kind());
        match table
            .entries
            .get(handle.id().index())
            .and_then(Option::as_ref)
        {
            Some(entry) => entry,
            None => unreachable!("{}", MISSING_ENTRY),
        }
    }

    fn expect_live<H: AssetHandle>(&self, handle: H) {
        let table = self.table(handle.kind());
        if table.allocator.is_live(handle.id()) {
            return;
        }
        let specifics = match table.allocator.slot_generation(handle.id()) {
            Some(generation) => format!("its slot now holds generation {generation}"),
            None => "its slot has never been used in this store".to_owned(),
        };
        panic!(
            "{}",
            message(
                &format!("asset handle used after unload: {handle:?}"),
                &specifics,
                "the handle was unloaded earlier, or it belongs to a different Assets store",
                "keep handles for as long as you draw with them; unload only what is finished \
                 with. A missing file reports Failed instead — this is not that",
            )
        );
    }
}

/// Panic text for a live handle whose entry is gone.
const MISSING_ENTRY: &str = "[jidousha] engine bug: a live asset handle has no entry\n  \
     likely cause: unload cleared the entry without freeing the slot\n  \
     fix: report this with the reproduction — game code cannot cause it";
