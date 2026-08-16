//! The byte-source seam, and the in-memory source tests and `verify` run on.
//!
//! Key types: `ByteSource`, `RequestId`, `Completion`, `MemorySource`.
//! Depends on: nothing outside this crate. Must never depend on: the
//! filesystem, `fetch`, or `wasm-bindgen` — I/O lives in the platform crates,
//! exactly as rendering backends do (assets.md §5, ADR-0003's discipline).
//! INVARIANT: a source hands back completions only when polled, and a poll
//! happens only inside `Assets::commit`. Nothing about load timing can reach
//! simulation between commits (assets.md §4 CONTRACT).

use std::collections::BTreeMap;

use crate::handle::AssetKind;
use crate::payload::{AssetError, Payload, TextureData};

/// Identifies one outstanding request, from asking to arriving.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(pub(crate) u64);

impl RequestId {
    /// A request id from a raw counter.
    ///
    /// For sources outside this crate — the platform crates implement
    /// [`ByteSource`] and have to mint their own. Ids need only be unique
    /// within one source, which is why a plain counter is enough.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }
}

/// A request that has finished, one way or the other.
#[derive(Clone, Debug)]
pub struct Completion {
    /// The request this answers.
    pub request: RequestId,
    /// What arrived, or why it did not.
    ///
    /// CONTRACT: the payload matches the [`AssetKind`] the request asked for.
    /// A source that fetched a texture returns decoded texels, not the file it
    /// read — decoding belongs to whoever has the bytes and a thread to spare
    /// (assets.md §5).
    pub result: Result<Payload, AssetError>,
}

/// Where bytes come from.
///
/// The native source reads files on a loader thread; the web source calls
/// `fetch`; [`MemorySource`] hands back what a test put there. All three are
/// asked the same two questions, and none of them ever blocks.
///
/// DELIBERATE: poll-based, with no `async fn` anywhere — see ADR-0011. A future
/// agent proposing "modernize this to async" should read that ADR first: the
/// asynchrony is real, and a game loop already expresses it by being a loop.
/// The `Send + Sync` bound is inherited, not chosen: `Assets` is a world
/// resource, and resources are `Send + Sync` so the engine keeps its
/// parallel-scheduler headroom (core.md §3). A native loader holding an
/// `mpsc::Receiver` — which is `Send` but not `Sync` — wraps it in a `Mutex`;
/// the store is only ever touched from one thread, so the lock is never
/// contended.
pub trait ByteSource: Send + Sync + 'static {
    /// Begin fetching `path`. Never blocks, never fails here — a path that
    /// cannot be read fails later, as a completion.
    ///
    /// `kind` says what to return: raw bytes, or decoded texels. A source that
    /// can decode off the frame should, and one that cannot decodes wherever it
    /// is able to — the requirement is only that the payload matches
    /// (assets.md §3, §5).
    fn request(&mut self, path: &str, kind: AssetKind) -> RequestId;

    /// Everything that finished by `tick`, in a deterministic order.
    ///
    /// CONTRACT: called only from `Assets::commit`. A source must return each
    /// completion exactly once, and must order a single poll's completions by
    /// request id — two runs that complete the same requests at the same tick
    /// must see them in the same order, or replay diverges.
    fn drain_completed(&mut self, tick: u64) -> Vec<Completion>;

    /// Requests asked for but not yet returned by `drain_completed`.
    ///
    /// Used by `all_ready` to answer "is anything still in flight" without
    /// waiting on anything.
    fn outstanding(&self) -> usize;
}

/// A source whose bytes are already in memory and whose *timing* is scripted.
///
/// This is the workhorse for tests and for `tools/verify`: it makes "the
/// texture becomes ready at tick 30" something a test can state, so loading
/// behaviour — placeholders, gates, the frame a sprite pops in — is testable
/// without a disk (assets.md §5, §7).
///
/// ```
/// use jidousha_assets::{Assets, AssetStatus, MemorySource};
///
/// let mut source = MemorySource::new();
/// source.insert("player.png", b"fake png".to_vec());
/// source.complete_at("player.png", 3);
///
/// let mut assets = Assets::new(source);
/// let player = assets.load_texture("player.png");
///
/// assets.commit(1);
/// assert_eq!(assets.status(player), AssetStatus::Loading);
///
/// assets.commit(3);
/// assert_eq!(assets.status(player), AssetStatus::Ready);
/// ```
pub struct MemorySource {
    /// Path → what it holds, or an error to report instead.
    content: BTreeMap<String, Result<Payload, AssetError>>,
    /// Path → the tick its request completes on. Absent means "immediately",
    /// i.e. at the first commit after the request.
    schedule: BTreeMap<String, u64>,
    /// Requests made and not yet drained, in request order.
    pending: Vec<Pending>,
    next_request: u64,
}

#[derive(Debug)]
struct Pending {
    request: RequestId,
    path: String,
}

impl MemorySource {
    /// A source with nothing in it. Unknown paths fail, which is what a missing
    /// file does.
    ///
    /// DELIBERATE: no `Default` impl, despite `clippy::new_without_default`
    /// (see ADR-0012) — one way to do everything, and `new` is that way.
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            content: BTreeMap::new(),
            schedule: BTreeMap::new(),
            pending: Vec::new(),
            next_request: 0,
        }
    }

    /// Put bytes at `path`.
    ///
    /// Handed back as-is, whatever kind is asked for. Scripting a store is
    /// about *timing*, not about round-tripping a file format, so a test that
    /// loads this as a texture gets these bytes rather than a decode error —
    /// use [`insert_texture`](MemorySource::insert_texture) when the texels
    /// themselves matter.
    pub fn insert(&mut self, path: &str, bytes: Vec<u8>) {
        self.content
            .insert(path.to_owned(), Ok(Payload::Bytes(bytes)));
    }

    /// Put a decoded image at `path`.
    ///
    /// What the native loader produces, without needing a real PNG in the
    /// test: a size and some texels are the part a renderer test cares about.
    pub fn insert_texture(&mut self, path: &str, texture: TextureData) {
        self.content
            .insert(path.to_owned(), Ok(Payload::Texture(texture)));
    }

    /// Make `path` fail, as a missing or unreadable file would.
    pub fn fail(&mut self, path: &str, error: AssetError) {
        self.content.insert(path.to_owned(), Err(error));
    }

    /// Hold `path`'s completion until `tick`.
    ///
    /// Without this, a request completes at the first commit after it is made.
    pub fn complete_at(&mut self, path: &str, tick: u64) {
        self.schedule.insert(path.to_owned(), tick);
    }
}

impl ByteSource for MemorySource {
    fn request(&mut self, path: &str, _kind: AssetKind) -> RequestId {
        let request = RequestId(self.next_request);
        self.next_request += 1;
        self.pending.push(Pending {
            request,
            path: path.to_owned(),
        });
        request
    }

    fn drain_completed(&mut self, tick: u64) -> Vec<Completion> {
        // Partitioned rather than filtered in place, so the remaining requests
        // keep their order: request order is completion order for anything
        // scheduled at the same tick.
        let mut completed = Vec::new();
        let mut still_pending = Vec::new();
        for entry in core::mem::take(&mut self.pending) {
            let due = self.schedule.get(&entry.path).copied().unwrap_or(0);
            if due > tick {
                still_pending.push(entry);
                continue;
            }
            let result = match self.content.get(&entry.path) {
                Some(content) => content.clone(),
                None => Err(AssetError::NotFound),
            };
            completed.push(Completion {
                request: entry.request,
                result,
            });
        }
        self.pending = still_pending;
        completed
    }

    fn outstanding(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_completes_at_the_first_commit_by_default() {
        let mut source = MemorySource::new();
        source.insert("a.png", vec![1, 2, 3]);
        let request = source.request("a.png", AssetKind::Bytes);
        let completed = source.drain_completed(0);
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].request, request);
        assert_eq!(completed[0].result, Ok(Payload::Bytes(vec![1, 2, 3])));
    }

    #[test]
    fn a_scheduled_request_waits_for_its_tick() {
        let mut source = MemorySource::new();
        source.insert("a.png", vec![1]);
        source.complete_at("a.png", 5);
        source.request("a.png", AssetKind::Bytes);
        assert!(source.drain_completed(4).is_empty());
        assert_eq!(source.drain_completed(5).len(), 1);
    }

    #[test]
    fn a_completion_is_handed_back_exactly_once() {
        let mut source = MemorySource::new();
        source.insert("a.png", vec![1]);
        source.request("a.png", AssetKind::Bytes);
        assert_eq!(source.drain_completed(0).len(), 1);
        assert!(source.drain_completed(0).is_empty());
    }

    #[test]
    fn an_unknown_path_fails_rather_than_hanging() {
        let mut source = MemorySource::new();
        let request = source.request("missing.png", AssetKind::Bytes);
        let completed = source.drain_completed(0);
        assert_eq!(completed[0].request, request);
        assert!(completed[0].result.is_err());
    }

    #[test]
    fn a_scripted_failure_reports_its_reason() {
        let mut source = MemorySource::new();
        source.fail(
            "bad.png",
            AssetError::Decode {
                detail: "bad chunk at byte 12".to_owned(),
            },
        );
        source.request("bad.png", AssetKind::Bytes);
        let completed = source.drain_completed(0);
        assert_eq!(
            completed[0].result.as_ref().err(),
            Some(&AssetError::Decode {
                detail: "bad chunk at byte 12".to_owned(),
            })
        );
    }

    #[test]
    fn completions_come_back_in_request_order() {
        let mut source = MemorySource::new();
        for path in ["a", "b", "c"] {
            source.insert(path, vec![0]);
        }
        let requests: Vec<RequestId> = ["c", "a", "b"]
            .map(|path| source.request(path, AssetKind::Bytes))
            .into();
        let completed = source.drain_completed(0);
        let order: Vec<RequestId> = completed.iter().map(|entry| entry.request).collect();
        assert_eq!(order, requests, "request order, not path order");
    }

    #[test]
    fn outstanding_counts_what_has_not_arrived() {
        let mut source = MemorySource::new();
        source.insert("a", vec![0]);
        source.complete_at("a", 9);
        source.request("a", AssetKind::Bytes);
        assert_eq!(source.outstanding(), 1);
        source.drain_completed(9);
        assert_eq!(source.outstanding(), 0);
    }
}
