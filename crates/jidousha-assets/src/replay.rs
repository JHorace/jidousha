//! Replaying a session's asset timing, whatever the disk feels like today.
//!
//! Key types: `ReplaySource`.
//! Depends on: `source`, `handle`. Must never depend on: `jidousha-input` —
//! the recording lives there and this takes a plain schedule of numbers, which
//! is what lets the two crates stay unaware of each other (input.md §5).
//! INVARIANT: a completion is released on the tick the schedule says, and on no
//! other. That is the whole job: load timing is environmental, the recorded
//! timeline is not, and replay has to follow the second one.

use std::collections::BTreeMap;

use crate::handle::AssetKind;
use crate::source::{ByteSource, Completion, RequestId};

/// A source that releases another source's completions on recorded ticks.
///
/// Wraps a real source rather than inventing payloads, because the payload is
/// not in the recording and should not be: a recording is a timeline, not an
/// archive of everybody's art. What it replaces is *when*, which is the only
/// part simulation can observe (assets.md §4).
///
/// ```
/// use jidousha_assets::{Assets, AssetStatus, MemorySource, ReplaySource};
///
/// let mut source = MemorySource::new();
/// source.insert("hero.png", b"art".to_vec());
///
/// // The session being replayed saw request 0 arrive on tick 5, however fast
/// // the disk was on the day.
/// let mut assets = Assets::new(ReplaySource::new(source, [(0, 5)]));
/// let hero = assets.load_bytes("hero.png");
///
/// assets.commit(4);
/// assert_eq!(assets.status(hero), AssetStatus::Loading);
/// assets.commit(5);
/// assert_eq!(assets.status(hero), AssetStatus::Ready);
/// ```
pub struct ReplaySource<S> {
    inner: S,
    /// Request → the tick it resolved on in the recorded session.
    schedule: BTreeMap<RequestId, u64>,
    /// Completions the inner source produced before their recorded tick.
    early: Vec<Completion>,
}

impl<S: ByteSource> ReplaySource<S> {
    /// A source that plays `inner` back on `schedule`'s ticks.
    ///
    /// `schedule` is `(request id, tick)` pairs, which is what a `Recording`'s
    /// `AssetReady` entries carry. A request the schedule does not mention is
    /// released as soon as it arrives — a replay of a session that never asked
    /// for it has nothing to say about when it should land.
    #[must_use]
    pub fn new(inner: S, schedule: impl IntoIterator<Item = (u64, u64)>) -> Self {
        Self {
            inner,
            schedule: schedule
                .into_iter()
                .map(|(request, tick)| (RequestId::from_bits(request), tick))
                .collect(),
            early: Vec::new(),
        }
    }

    /// Completions that have arrived but are not due yet.
    ///
    /// A replay that ends with any of these replayed a shorter session than it
    /// recorded — worth being able to ask about, since the alternative is a
    /// test that quietly asserts nothing. It is not what keeps a loading gate
    /// shut: an undelivered completion leaves its asset `Loading`, and
    /// `all_ready` reads the assets.
    #[must_use]
    pub fn unreleased(&self) -> usize {
        self.early.len()
    }
}

impl<S: ByteSource> ByteSource for ReplaySource<S> {
    fn request(&mut self, path: &str, kind: AssetKind) -> RequestId {
        self.inner.request(path, kind)
    }

    fn drain_completed(&mut self, tick: u64) -> Vec<Completion> {
        // Anything the inner source has finished joins the holding pen first,
        // so a completion that arrived early on this run waits for its tick.
        self.early.extend(self.inner.drain_completed(tick));

        let mut due = Vec::new();
        let mut waiting = Vec::new();
        for completion in core::mem::take(&mut self.early) {
            let ready = match self.schedule.get(&completion.request) {
                Some(recorded) => *recorded <= tick,
                // Not in the recording: nothing to wait for.
                None => true,
            };
            if ready {
                due.push(completion);
            } else {
                waiting.push(completion);
            }
        }
        self.early = waiting;

        // The same CONTRACT every source keeps (assets.md §5): one poll's
        // completions come back in request order. Holding some back and
        // releasing others makes the arrival order arbitrary, so this sort is
        // doing real work here rather than restating what already held.
        due.sort_by_key(|completion| completion.request);
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::{AssetStatus, Assets};
    use crate::source::MemorySource;

    fn source_with(paths: &[&str]) -> MemorySource {
        let mut source = MemorySource::new();
        for path in paths {
            source.insert(path, vec![1]);
        }
        source
    }

    #[test]
    fn a_completion_waits_for_the_tick_the_recording_gives_it() {
        // The disk is fast today and was slow on the day. Replay follows the
        // recording, because that is the timeline simulation observed.
        let mut assets = Assets::new(ReplaySource::new(source_with(&["a"]), [(0, 7)]));
        let handle = assets.load_bytes("a");
        for tick in 0..7 {
            assets.commit(tick);
            assert_eq!(assets.status(handle), AssetStatus::Loading, "tick {tick}");
        }
        assets.commit(7);
        assert_eq!(assets.status(handle), AssetStatus::Ready);
    }

    #[test]
    fn a_request_the_recording_never_saw_is_released_at_once() {
        // A replay of a session that did not ask for this has nothing to say
        // about when it should land, and holding it forever would hang.
        let mut assets = Assets::new(ReplaySource::new(source_with(&["a"]), []));
        let handle = assets.load_bytes("a");
        assets.commit(0);
        assert_eq!(assets.status(handle), AssetStatus::Ready);
    }

    #[test]
    fn a_held_completion_keeps_the_loading_gate_shut() {
        // `all_ready` is the one-line gate a game writes (assets.md §1), and a
        // replay that opened it several ticks before the recorded session did
        // would change what every loading screen in the game does.
        let mut assets = Assets::new(ReplaySource::new(source_with(&["a"]), [(0, 5)]));
        assets.load_bytes("a");
        assets.commit(0);
        assert!(!assets.all_ready(), "it has arrived but is not due");
        assets.commit(5);
        assert!(assets.all_ready());
    }

    #[test]
    fn completions_released_together_come_back_in_request_order() {
        // The CONTRACT every source keeps (assets.md §5), and the one place it
        // is not free: the holding pen fills in *arrival* order, so releasing
        // from it hands back whatever order the inner source happened to
        // finish in. Here `b` arrives before `a` and both come due at once, so
        // an unsorted release would return them as (1, 0) — which two runs on
        // two machines have no reason to agree on.
        let mut source = MemorySource::new();
        for path in ["a", "b"] {
            source.insert(path, vec![0]);
        }
        source.complete_at("b", 1);
        source.complete_at("a", 2);
        let mut replay = ReplaySource::new(source, [(0, 9), (1, 9)]);
        for path in ["a", "b"] {
            replay.request(path, AssetKind::Bytes);
        }
        // b lands first, a second; neither is due, so both wait in that order.
        assert!(replay.drain_completed(1).is_empty());
        assert!(replay.drain_completed(2).is_empty());
        assert_eq!(replay.unreleased(), 2, "both are waiting, b ahead of a");

        let released: Vec<u64> = replay
            .drain_completed(9)
            .iter()
            .map(|completion| completion.request.bits())
            .collect();
        assert_eq!(released, vec![0, 1], "request order, not arrival order");
    }

    #[test]
    fn a_replay_that_stops_early_can_say_what_it_never_released() {
        let mut replay = ReplaySource::new(source_with(&["a"]), [(0, 100)]);
        replay.request("a", AssetKind::Bytes);
        replay.drain_completed(1);
        assert_eq!(replay.unreleased(), 1);
        replay.drain_completed(100);
        assert_eq!(replay.unreleased(), 0);
    }

    #[test]
    fn a_failure_replays_on_its_recorded_tick_too() {
        // A load that failed is as much a part of the timeline as one that
        // arrived — a game can see the difference (assets.md §4).
        let mut assets = Assets::new(ReplaySource::new(MemorySource::new(), [(0, 4)]));
        let missing = assets.load_bytes("nowhere");
        assets.commit(3);
        assert_eq!(assets.status(missing), AssetStatus::Loading);
        assets.commit(4);
        assert_eq!(assets.status(missing), AssetStatus::Failed);
    }
}
