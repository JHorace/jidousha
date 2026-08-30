//! Which finger is in which slot, between the tick it lands and the tick its
//! end is reported.
//!
//! Key types: `TouchTracker`.
//! Depends on: its parent for the touch vocabulary. Must never depend on: the
//! platform crate — a finger arrives as an opaque `FingerId` and everything
//! interesting happens here, above the seam (input.md §6).
//! INVARIANT: slot assignment, the four-touch bound and the tap-inside-one-
//! frame rule are contracts, so they live where a test with no touchscreen can
//! reach them — the same argument that put the edge rules in `SnapshotBuilder`.

use jidousha_core::math::Vec2;

use super::{FingerId, MAX_TOUCHES, Touch, TouchId, TouchList, TouchPhase};

/// One finger the builder is following, between the tick it lands and the tick
/// its end is reported.
#[derive(Clone, Copy, Debug)]
struct Tracked {
    /// The platform's name for it, which is how later events find this slot.
    finger: FingerId,
    /// Where it was when it was last heard from.
    screen: Vec2,
    /// The phase the next snapshot will report.
    phase: TouchPhase,
    /// An end that arrived before its `Began` had been reported, owed to the
    /// tick after. A tap inside one frame is two edges and a snapshot's touch
    /// entry has room for one, so the second waits rather than being lost —
    /// the same rule the keyboard's `pressed`/`released` pair encodes a
    /// different way (input.md §2, §3a).
    owed_end: Option<TouchPhase>,
}

impl Tracked {
    /// Whether this touch's end has already been decided.
    fn is_ending(&self) -> bool {
        self.owed_end.is_some() || self.phase.is_final()
    }
}

/// Which finger is in which slot, and what each owes the next snapshot.
///
/// Lives here rather than in the platform crate for the reason every other
/// input rule does: slot assignment, the four-touch bound and the tap-inside-
/// one-frame rule are contracts, and behind the winit seam none of them could
/// be tested on wasm CI or on a machine with no touchscreen (input.md §6).
pub(crate) struct TouchTracker {
    slots: [Option<Tracked>; MAX_TOUCHES],
}

/// Every slot, as the ids they are.
///
/// The one place the array's indices and the engine's `TouchId`s meet, written
/// out so that nothing here ever converts a `usize` into a `u8` and has to say
/// what it would do if that failed. Its length *is* [`MAX_TOUCHES`], so a
/// change to the bound that forgot this list would not compile.
const SLOTS: [TouchId; MAX_TOUCHES] = [TouchId(0), TouchId(1), TouchId(2), TouchId(3)];

impl TouchTracker {
    /// A tracker with no fingers down.
    pub(crate) const fn new() -> Self {
        Self {
            slots: [None; MAX_TOUCHES],
        }
    }

    /// Take a finger's landing, and say which slot it got.
    ///
    /// `None` when it is dropped: every slot is occupied (the documented
    /// four-touch bound), or this finger is already down, which a platform
    /// should not report twice and which would otherwise consume a second
    /// slot for one finger.
    pub(crate) fn begin(&mut self, finger: FingerId, screen: Vec2) -> Option<TouchId> {
        if self.find(finger).is_some() {
            return None;
        }
        let slot = SLOTS.into_iter().find(|slot| self.at(*slot).is_none())?;
        self.slots[usize::from(slot.0)] = Some(Tracked {
            finger,
            screen,
            phase: TouchPhase::Began,
            owed_end: None,
        });
        Some(slot)
    }

    /// Take a finger's move. `None` if it is not one this tracker is following.
    pub(crate) fn moved(&mut self, finger: FingerId, screen: Vec2) -> Option<TouchId> {
        let slot = self.find(finger)?;
        let tracked = self.slots[usize::from(slot.0)].as_mut()?;
        if tracked.is_ending() {
            // A move after the lift, or after a cancel. The touch is over; its
            // last position is the one it ended at.
            return None;
        }
        tracked.screen = screen;
        if tracked.phase != TouchPhase::Began {
            // A finger that lands and moves in the same frame reports `Began`
            // at the position it reached — one entry, and the phase that
            // matters is the one a game keys a press off.
            tracked.phase = TouchPhase::Moved;
        }
        Some(slot)
    }

    /// Take a finger's lift or cancellation. `None` if it is not one this
    /// tracker is following, or if its end is already decided.
    pub(crate) fn end(
        &mut self,
        finger: FingerId,
        phase: TouchPhase,
        screen: Vec2,
    ) -> Option<TouchId> {
        let slot = self.find(finger)?;
        let tracked = self.slots[usize::from(slot.0)].as_mut()?;
        if tracked.is_ending() {
            return None;
        }
        tracked.screen = screen;
        if tracked.phase == TouchPhase::Began {
            tracked.owed_end = Some(phase);
        } else {
            tracked.phase = phase;
        }
        Some(slot)
    }

    /// End every touch as cancelled, for a window that lost focus.
    ///
    /// Cancelled and not ended: the fingers may still be on the glass, and the
    /// engine is not claiming they were lifted — it is saying it stopped being
    /// told (input.md §4).
    pub(crate) fn cancel_all(&mut self) {
        for slot in &mut self.slots {
            if let Some(tracked) = slot
                && !tracked.is_ending()
            {
                if tracked.phase == TouchPhase::Began {
                    tracked.owed_end = Some(TouchPhase::Cancelled);
                } else {
                    tracked.phase = TouchPhase::Cancelled;
                }
            }
        }
    }

    /// This frame's touches, with the edges they are carrying.
    pub(crate) fn edges(&self) -> TouchList {
        let mut list = TouchList::new();
        for (slot, tracked) in self.occupied() {
            list.push(Touch {
                id: slot,
                phase: tracked.phase,
                screen: tracked.screen,
            });
        }
        list
    }

    /// The touches still down, with no edges on them — for a catch-up tick.
    ///
    /// Every touch the tracker is following is reported as `Moved`, including
    /// one whose end is owed: it is still on the list until that end is
    /// reported, and a catch-up tick never reports an end (input.md §2).
    pub(crate) fn state(&self) -> TouchList {
        let mut list = TouchList::new();
        for (slot, tracked) in self.occupied() {
            list.push(Touch {
                id: slot,
                phase: TouchPhase::Moved,
                screen: tracked.screen,
            });
        }
        list
    }

    /// Spend this frame's edges: what was reported does not report again.
    ///
    /// A touch that reported a final phase leaves; one that owes an end
    /// reports it next tick; every other one settles into `Moved`.
    pub(crate) fn spend(&mut self) {
        for slot in &mut self.slots {
            let Some(tracked) = slot else { continue };
            if tracked.phase.is_final() {
                *slot = None;
            } else if let Some(end) = tracked.owed_end.take() {
                tracked.phase = end;
            } else {
                tracked.phase = TouchPhase::Moved;
            }
        }
    }

    /// What is in a slot, if anything.
    fn at(&self, slot: TouchId) -> Option<&Tracked> {
        self.slots[usize::from(slot.0)].as_ref()
    }

    /// The slot a finger is in, if it is down.
    fn find(&self, finger: FingerId) -> Option<TouchId> {
        SLOTS.into_iter().find(|slot| {
            self.at(*slot)
                .is_some_and(|tracked| tracked.finger == finger)
        })
    }

    /// Every occupied slot, in slot order — which is what makes the touch list
    /// canonical without anything having to sort it.
    fn occupied(&self) -> impl Iterator<Item = (TouchId, &Tracked)> {
        SLOTS
            .into_iter()
            .filter_map(|slot| Some((slot, self.at(slot)?)))
    }
}
