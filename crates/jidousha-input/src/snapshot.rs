//! The one value all input arrives in, and the resource games read it through.
//!
//! Key types: `InputSnapshot`, `Input`.
//! Depends on: `key`, `pointer`, `jidousha-core` (for `Resource`).
//! INVARIANT: a snapshot is the complete input truth for one tick and is plain
//! data — no handles, no callbacks, nothing that means anything only while the
//! platform is alive. Record it, replay it, and simulation cannot tell the
//! difference (core.md §7, input.md §1).
//! INVARIANT: the key and button lists are canonical — sorted, no duplicates —
//! so two snapshots meaning the same input *are* equal. Order within a tick is
//! not observable to simulation (every query is by key), so canonicalizing
//! loses nothing and keeps replay comparisons honest.

use jidousha_core::Resource;
use jidousha_core::math::Vec2;

use crate::key::Key;
use crate::pointer::{PointerId, PointerState};

/// Everything the player did during one Update tick.
///
/// Built by the platform layer once per frame ([`SnapshotBuilder`]), or written
/// directly by a test ([`InputScript`]). Simulation never sees anything else:
/// no events, no callbacks, no mid-tick polling. That single choke point is
/// what makes recording and replay a matter of storing these values in order
/// (input.md §1).
///
/// [`SnapshotBuilder`]: crate::SnapshotBuilder
/// [`InputScript`]: crate::InputScript
#[derive(Clone, Debug, PartialEq)]
pub struct InputSnapshot {
    /// Keys down this tick. Canonical: sorted, no duplicates.
    pub(crate) held: Vec<Key>,
    /// Keys that went down this tick.
    pub(crate) pressed: Vec<Key>,
    /// Keys that came up this tick.
    pub(crate) released: Vec<Key>,
    /// Every pointer. INVARIANT: index 0 is always the primary.
    pub(crate) pointers: Vec<PointerState>,
    /// Whether the window had focus.
    pub(crate) window_focused: bool,
}

impl InputSnapshot {
    /// A tick in which the player did nothing.
    ///
    /// The primary pointer exists and sits at the origin, because
    /// [`Input::pointer`] promises to always have one to return.
    ///
    /// DELIBERATE: no `Default` impl, despite `clippy::new_without_default`
    /// (see ADR-0012) — one way to do everything, and `new` is that way.
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            held: Vec::new(),
            pressed: Vec::new(),
            released: Vec::new(),
            pointers: vec![PointerState::new(PointerId::PRIMARY)],
            window_focused: true,
        }
    }

    // DELIBERATE: there is no `without_edges` here, deriving a catch-up tick
    // from a snapshot you already hold. Catch-up derivation exists once, on
    // `SnapshotBuilder::catch_up_snapshot`, because only the live platform path
    // ever needs it: a recording stores one snapshot per *tick* (input.md §5),
    // so replay plays back the catch-up ticks it recorded rather than deriving
    // them again. A second spelling would be a second way to do one thing
    // (conventions §1), and the two could drift.
    //
    // DELIBERATE: there is no `with_keys(&[Key])` either, building a populated
    // snapshot in one call (see ADR-0019). A closed-loop check — one that has
    // to see the game before deciding what to press — records `InputEvent`s
    // into a `SnapshotBuilder`, which is the same path a real keyboard takes
    // and the only place the edge rules live. A constructor here would have to
    // answer the edge question a second time.

    /// Keys down this tick, sorted.
    #[must_use]
    pub fn held_keys(&self) -> &[Key] {
        &self.held
    }

    /// Keys that went down this tick, sorted.
    #[must_use]
    pub fn pressed_keys(&self) -> &[Key] {
        &self.pressed
    }

    /// Keys that came up this tick, sorted.
    #[must_use]
    pub fn released_keys(&self) -> &[Key] {
        &self.released
    }

    /// Every pointer this tick. Index 0 is the primary.
    #[must_use]
    pub fn pointers(&self) -> &[PointerState] {
        &self.pointers
    }

    /// Whether the window had focus this tick.
    #[must_use]
    pub fn window_focused(&self) -> bool {
        self.window_focused
    }
}

/// What the player did this tick, held as a world resource.
///
/// The driver replaces it before every Update tick, so a windowed game always
/// has one — but a headless run has none until a test inserts one, and no
/// resource exists before the first tick in either. Reach for it with
/// `world.find_resource::<Input>()` and return early when it is absent; that is
/// what the Quickstart does and why.
///
/// ```
/// # use jidousha_input::{Input, InputSnapshot, Key};
/// # use jidousha_core::World;
/// # let mut world = World::new();
/// world.insert_resource(Input::new(InputSnapshot::new()));
///
/// let input = world.resource::<Input>();
/// assert!(!input.held(Key::Space));
/// ```
///
/// CONTRACT: read-only for games. There is no method here that changes what the
/// player did — the driver replaces the whole resource each tick, and a system
/// that could edit input could edit the recording, which would make replay a
/// story rather than a guarantee (input.md §1).
pub struct Input {
    snapshot: InputSnapshot,
}

impl Resource for Input {}

impl Input {
    /// The input resource for one tick.
    #[must_use]
    pub fn new(snapshot: InputSnapshot) -> Self {
        Self { snapshot }
    }

    /// Whether `key` is down this tick.
    ///
    /// True on the tick a key is tapped, even though it is released on that
    /// same tick: a press edge without a held bit would make
    /// `just_pressed(k) && !held(k)` possible, which no game expects.
    #[must_use]
    pub fn held(&self, key: Key) -> bool {
        self.snapshot.held.contains(&key)
    }

    /// Whether `key` went down this tick.
    #[must_use]
    pub fn just_pressed(&self, key: Key) -> bool {
        self.snapshot.pressed.contains(&key)
    }

    /// Whether `key` came up this tick.
    #[must_use]
    pub fn just_released(&self, key: Key) -> bool {
        self.snapshot.released.contains(&key)
    }

    /// The primary pointer — the mouse, or the first finger down.
    ///
    /// Always present: on a machine with no pointer at all it reports the
    /// origin and no buttons, which is what "nothing is happening" looks like.
    #[must_use]
    pub fn pointer(&self) -> &PointerState {
        match self.snapshot.pointers.first() {
            Some(pointer) => pointer,
            None => unreachable!("{}", MISSING_PRIMARY),
        }
    }

    /// Every pointer this tick.
    ///
    /// Length 1 until touch platforms land, and game code written against
    /// [`pointer`](Input::pointer) never has to change when they do.
    #[must_use]
    pub fn pointers(&self) -> &[PointerState] {
        &self.snapshot.pointers
    }

    /// Whether the window had focus this tick.
    ///
    /// Pause-on-unfocus is a legitimate gameplay concern, so this is readable —
    /// and recorded, because simulation can observe it (input.md §4).
    #[must_use]
    pub fn window_focused(&self) -> bool {
        self.snapshot.window_focused
    }

    /// The whole snapshot, for the recorder (I2) and for tests.
    #[must_use]
    pub fn snapshot(&self) -> &InputSnapshot {
        &self.snapshot
    }
}

/// Panic text for a snapshot that lost its primary pointer.
const MISSING_PRIMARY: &str = "[jidousha] engine bug: an input snapshot has no primary pointer\n  \
     likely cause: a snapshot was built without going through InputSnapshot::new\n  \
     fix: report this with the reproduction — game code cannot cause it";

/// Insert `key` into a canonical (sorted, deduplicated) list.
pub(crate) fn insert_sorted<T: Ord>(list: &mut Vec<T>, value: T) {
    if let Err(index) = list.binary_search(&value) {
        list.insert(index, value);
    }
}

/// Remove `value` from a canonical list, if it is there.
pub(crate) fn remove_sorted<T: Ord>(list: &mut Vec<T>, value: &T) {
    if let Ok(index) = list.binary_search(value) {
        list.remove(index);
    }
}

/// The primary pointer of a snapshot under construction, creating it if a
/// touch id arrived before the mouse ever moved.
pub(crate) fn pointer_mut(pointers: &mut Vec<PointerState>, id: PointerId) -> &mut PointerState {
    if let Some(index) = pointers.iter().position(|pointer| pointer.id == id) {
        return &mut pointers[index];
    }
    pointers.push(PointerState::new(id));
    // Pointers stay sorted by id so that iteration order — and therefore the
    // recording — never depends on which finger touched down first.
    pointers.sort_by_key(|pointer| pointer.id);
    let index = pointers
        .iter()
        .position(|pointer| pointer.id == id)
        .unwrap_or(0);
    &mut pointers[index]
}

/// A pointer position the engine refuses to record.
pub(crate) fn expect_finite(screen: Vec2, id: PointerId) {
    if screen.x.is_finite() && screen.y.is_finite() {
        return;
    }
    panic!(
        "{}",
        jidousha_core::message(
            &format!("pointer moved to a non-finite position: {screen:?}"),
            &format!("pointer: {id}"),
            "the platform layer computed a position from a zero window size, or from an \
             uninitialized value",
            "clamp or drop the event at the platform boundary; a recording containing NaN \
             cannot replay, because NaN does not equal itself",
        )
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_snapshot_still_has_a_primary_pointer() {
        let input = Input::new(InputSnapshot::new());
        assert_eq!(input.pointer().id, PointerId::PRIMARY);
        assert_eq!(input.pointers().len(), 1);
    }

    #[test]
    fn a_fresh_snapshot_reports_nothing_happening() {
        let input = Input::new(InputSnapshot::new());
        for &key in Key::ALL {
            assert!(!input.held(key));
            assert!(!input.just_pressed(key));
            assert!(!input.just_released(key));
        }
        assert!(input.window_focused(), "focus is the resting state");
    }

    #[test]
    fn sorted_insertion_keeps_the_list_canonical() {
        let mut list = Vec::new();
        for key in [Key::Z, Key::A, Key::M, Key::A] {
            insert_sorted(&mut list, key);
        }
        assert_eq!(list, vec![Key::A, Key::M, Key::Z]);
        remove_sorted(&mut list, &Key::M);
        assert_eq!(list, vec![Key::A, Key::Z]);
    }

    #[test]
    fn a_pointer_is_created_on_first_mention_and_kept_in_id_order() {
        let mut pointers = vec![PointerState::new(PointerId::PRIMARY)];
        pointer_mut(&mut pointers, PointerId::touch(4));
        pointer_mut(&mut pointers, PointerId::touch(1));
        let ids: Vec<PointerId> = pointers.iter().map(|pointer| pointer.id).collect();
        assert_eq!(
            ids,
            vec![PointerId::PRIMARY, PointerId::touch(1), PointerId::touch(4)],
            "id order, not arrival order"
        );
    }

    #[test]
    #[should_panic(expected = "non-finite position")]
    fn a_nan_pointer_position_is_refused() {
        expect_finite(Vec2::new(f32::NAN, 0.0), PointerId::PRIMARY);
    }
}
