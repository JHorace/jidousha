//! Snapshots as bytes: the format recordings are made of.
//!
//! Key types: `DecodeError`.
//! Depends on: `key`, `pointer`, `touch`, `snapshot`. Must never depend on:
//! `serde` or any derive-based serializer — see ADR-0014.
//! INVARIANT: byte-stable across platforms and builds. Every integer is
//! little-endian and fixed-width, every float is written as its IEEE bits, and
//! every list is canonical. Two machines encoding equal snapshots produce
//! identical bytes, which is what lets a recording move between them
//! (input.md §5).
//! INVARIANT: decoding is strict. Anything that would not re-encode to the
//! bytes it came from is refused, so `decode(encode(x)) == x` and
//! `encode(decode(b)) == b` both hold — for the version this build writes.
//! A **version 1** snapshot, written before touch existed, still decodes: to
//! the same value it always meant, with no touches. Re-encoding it produces
//! version 2 bytes, because the encoder has one output and it is the current
//! format. That is the price of "old recordings keep replaying" and it is the
//! whole of it (input.md §5, ADR-0043).

use core::fmt;

use jidousha_core::math::Vec2;
use jidousha_core::message;

use crate::key::Key;
use crate::pointer::{PointerButton, PointerId, PointerState};
use crate::snapshot::InputSnapshot;
use crate::touch::{MAX_TOUCHES, Touch, TouchId, TouchList, TouchPhase};

/// Marks the bytes as ours, so a wrong file fails as a wrong file.
const MAGIC: [u8; 4] = *b"JDIN";

/// The format version. Bump when the layout changes; a decoder refuses what it
/// does not know rather than misreading it.
///
/// **2 adds the touch list** and changes nothing before it, which is what makes
/// reading version 1 a matter of stopping early rather than of a second parser
/// (ADR-0043). New snapshots are written at 2 and an older engine refuses them
/// by number — a recording made here replays here, said out loud rather than
/// discovered.
const VERSION: u16 = 2;

/// The oldest version this build reads. Everything from here to [`VERSION`] is
/// the same bytes with fewer fields on the end.
const OLDEST_VERSION: u16 = 1;

/// The first version that carries a touch list.
const FIRST_TOUCH_VERSION: u16 = 2;

/// Why a snapshot could not be read.
///
/// Environmental, not a contract violation: these bytes came from outside the
/// program — a file, a network, an older engine — so this is a `Result` rather
/// than a panic (input.md §7, core.md §9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// The bytes do not begin with the format's magic number.
    NotASnapshot,
    /// A version this build does not know how to read.
    UnsupportedVersion {
        /// The version the bytes claim.
        found: u16,
    },
    /// A touch phase code this build has never heard of.
    UnknownTouchPhase {
        /// The code found.
        code: u8,
    },
    /// More touches than the format has room for, or a slot outside it.
    MalformedTouches,
    /// The bytes ran out before the value did.
    Truncated {
        /// How many bytes were needed at that point.
        needed: usize,
        /// How many remained.
        available: usize,
    },
    /// Bytes remained after a complete snapshot.
    TrailingBytes {
        /// How many.
        count: usize,
    },
    /// A key code this build has never heard of.
    UnknownKey {
        /// The code found.
        code: u16,
    },
    /// A button code this build has never heard of.
    UnknownButton {
        /// The code found.
        code: u8,
    },
    /// A list that was not sorted, or held a duplicate.
    NotCanonical {
        /// Which list.
        list: &'static str,
    },
    /// A float that is NaN or infinite.
    NotFinite {
        /// Which field.
        field: &'static str,
    },
    /// A snapshot with no primary pointer, or with its pointers out of order.
    MalformedPointers,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (what, specifics, cause): (String, String, &str) = match self {
            DecodeError::NotASnapshot => (
                "not an input snapshot".to_owned(),
                "the bytes do not start with the format's magic number".to_owned(),
                "the file is not a recording, or the stream is misaligned",
            ),
            DecodeError::UnsupportedVersion { found } => (
                format!("input snapshot version {found} cannot be read"),
                format!("this build reads versions {OLDEST_VERSION} to {VERSION}"),
                "the recording was made by a newer engine",
            ),
            DecodeError::UnknownTouchPhase { code } => (
                format!("unknown touch phase code {code}"),
                "the code is not in this build's TouchPhase enum".to_owned(),
                "the recording was made by a newer engine, or the bytes are corrupt",
            ),
            DecodeError::MalformedTouches => (
                "the touch list is malformed".to_owned(),
                format!("a snapshot carries at most {MAX_TOUCHES} touches, in slot order"),
                "the bytes were written by something other than this encoder",
            ),
            DecodeError::Truncated { needed, available } => (
                "input snapshot ends early".to_owned(),
                format!("needed {needed} more bytes, {available} left"),
                "the recording was cut off — a crash mid-write, or a partial copy",
            ),
            DecodeError::TrailingBytes { count } => (
                "input snapshot has bytes left over".to_owned(),
                format!("{count} bytes follow a complete snapshot"),
                "two snapshots were concatenated, or the length was miscomputed",
            ),
            DecodeError::UnknownKey { code } => (
                format!("unknown key code {code}"),
                "the code is not in this build's Key enum".to_owned(),
                "the recording was made by a newer engine that knows more keys",
            ),
            DecodeError::UnknownButton { code } => (
                format!("unknown pointer button code {code}"),
                "the code is not in this build's PointerButton enum".to_owned(),
                "the recording was made by a newer engine, or the bytes are corrupt",
            ),
            DecodeError::NotCanonical { list } => (
                format!("the {list} list is not canonical"),
                "snapshot lists are sorted with no duplicates".to_owned(),
                "the bytes were written by something other than this encoder",
            ),
            DecodeError::NotFinite { field } => (
                format!("{field} is not a finite number"),
                "NaN and infinity cannot appear in a snapshot".to_owned(),
                "the bytes are corrupt — NaN does not equal itself, so it could never replay",
            ),
            DecodeError::MalformedPointers => (
                "the pointer list is malformed".to_owned(),
                "a snapshot carries the primary pointer first, then any others in id order"
                    .to_owned(),
                "the bytes were written by something other than this encoder",
            ),
        };
        formatter.write_str(&message(
            &what,
            &specifics,
            cause,
            "re-record the session; a recording this build cannot read cannot be replayed by it",
        ))
    }
}

impl core::error::Error for DecodeError {}

impl InputSnapshot {
    /// The snapshot as bytes.
    ///
    /// DELIBERATE: hand-written, not derived from `serde` — see ADR-0014. The
    /// format is ours to keep stable, and it is forty lines.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());

        write_keys(&mut out, &self.held);
        write_keys(&mut out, &self.pressed);
        write_keys(&mut out, &self.released);
        out.push(u8::from(self.window_focused));

        let count = u32::try_from(self.pointers.len()).unwrap_or(u32::MAX);
        out.extend_from_slice(&count.to_le_bytes());
        for pointer in &self.pointers {
            out.extend_from_slice(&pointer.id.code().to_le_bytes());
            out.extend_from_slice(&pointer.screen.x.to_bits().to_le_bytes());
            out.extend_from_slice(&pointer.screen.y.to_bits().to_le_bytes());
            out.extend_from_slice(&pointer.scroll.to_bits().to_le_bytes());
            write_buttons(&mut out, &pointer.held);
            write_buttons(&mut out, &pointer.pressed);
            write_buttons(&mut out, &pointer.released);
        }

        // Version 2's addition, and it is on the end because that is what
        // "additive" costs: everything above decodes identically at either
        // version, so reading an old snapshot is reading this file and
        // stopping here (ADR-0043).
        let touches = self.touches.as_slice();
        // A `u8` count for a list that cannot exceed four. The bound is the
        // contract; spending four bytes to say "one" would be pretending it
        // is not.
        out.push(u8::try_from(touches.len()).unwrap_or(u8::MAX));
        for touch in touches {
            out.push(touch.id.slot());
            out.push(touch.phase.code());
            out.extend_from_slice(&touch.screen.x.to_bits().to_le_bytes());
            out.extend_from_slice(&touch.screen.y.to_bits().to_le_bytes());
        }
        out
    }

    /// Read a snapshot back.
    ///
    /// # Errors
    ///
    /// If the bytes are not a snapshot this build can read: wrong format, a
    /// newer version, truncated, corrupt, or holding a value the engine would
    /// never have written.
    pub fn try_decode(bytes: &[u8]) -> Result<InputSnapshot, DecodeError> {
        let mut reader = Reader { bytes, at: 0 };

        if reader.take(MAGIC.len())? != MAGIC {
            return Err(DecodeError::NotASnapshot);
        }
        let version = reader.u16()?;
        if !(OLDEST_VERSION..=VERSION).contains(&version) {
            return Err(DecodeError::UnsupportedVersion { found: version });
        }

        let held = reader.keys("held")?;
        let pressed = reader.keys("pressed")?;
        let released = reader.keys("released")?;
        let window_focused = match reader.u8()? {
            0 => false,
            1 => true,
            // Anything else would re-encode as 0 or 1 and break the round trip.
            _ => return Err(DecodeError::NotCanonical { list: "focus flag" }),
        };

        let count = reader.u32()?;
        let mut pointers = Vec::new();
        for _ in 0..count {
            let id = PointerId::from_code(reader.u32()?);
            let x = reader.f32("pointer x")?;
            let y = reader.f32("pointer y")?;
            let scroll = reader.f32("scroll")?;
            let mut pointer = PointerState::new(id);
            pointer.screen = Vec2::new(x, y);
            pointer.scroll = scroll;
            pointer.held = reader.buttons("pointer held")?;
            pointer.pressed = reader.buttons("pointer pressed")?;
            pointer.released = reader.buttons("pointer released")?;
            pointers.push(pointer);
        }
        if pointers.first().map(|pointer| pointer.id) != Some(PointerId::PRIMARY)
            || !pointers.windows(2).all(|pair| pair[0].id < pair[1].id)
        {
            return Err(DecodeError::MalformedPointers);
        }

        // A version-1 snapshot ends where version 2 starts its touch list, and
        // means what it always meant: nobody was touching anything.
        let touches = if version >= FIRST_TOUCH_VERSION {
            reader.touches()?
        } else {
            TouchList::new()
        };

        let left = reader.bytes.len() - reader.at;
        if left > 0 {
            return Err(DecodeError::TrailingBytes { count: left });
        }
        Ok(InputSnapshot {
            held,
            pressed,
            released,
            pointers,
            touches,
            window_focused,
        })
    }
}

fn write_keys(out: &mut Vec<u8>, keys: &[Key]) {
    let count = u32::try_from(keys.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&count.to_le_bytes());
    for key in keys {
        out.extend_from_slice(&key.code().to_le_bytes());
    }
}

fn write_buttons(out: &mut Vec<u8>, buttons: &[PointerButton]) {
    let count = u8::try_from(buttons.len()).unwrap_or(u8::MAX);
    out.push(count);
    for button in buttons {
        out.push(button.code());
    }
}

/// A cursor over the bytes that refuses to read past the end.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], DecodeError> {
        let available = self.bytes.len() - self.at;
        if available < count {
            return Err(DecodeError::Truncated {
                needed: count,
                available,
            });
        }
        let slice = &self.bytes[self.at..self.at + count];
        self.at += count;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, DecodeError> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Result<u32, DecodeError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn f32(&mut self, field: &'static str) -> Result<f32, DecodeError> {
        let value = f32::from_bits(self.u32()?);
        if !value.is_finite() {
            return Err(DecodeError::NotFinite { field });
        }
        Ok(value)
    }

    fn keys(&mut self, list: &'static str) -> Result<Vec<Key>, DecodeError> {
        let count = self.u32()?;
        let mut keys = Vec::new();
        for _ in 0..count {
            let code = self.u16()?;
            let Some(key) = Key::find_by_code(code) else {
                return Err(DecodeError::UnknownKey { code });
            };
            if keys.last().is_some_and(|last| *last >= key) {
                return Err(DecodeError::NotCanonical { list });
            }
            keys.push(key);
        }
        Ok(keys)
    }

    fn touches(&mut self) -> Result<TouchList, DecodeError> {
        let count = self.u8()?;
        if usize::from(count) > MAX_TOUCHES {
            return Err(DecodeError::MalformedTouches);
        }
        let mut touches = TouchList::new();
        let mut last: Option<TouchId> = None;
        for _ in 0..count {
            let slot = self.u8()?;
            let Some(id) = TouchId::find_by_slot(slot) else {
                return Err(DecodeError::MalformedTouches);
            };
            let code = self.u8()?;
            let Some(phase) = TouchPhase::find_by_code(code) else {
                return Err(DecodeError::UnknownTouchPhase { code });
            };
            // Slot order with no duplicates, for the same reason the key list
            // is sorted: it is what makes two equal snapshots equal bytes.
            if last.is_some_and(|last| last >= id) {
                return Err(DecodeError::NotCanonical { list: "touches" });
            }
            last = Some(id);
            let x = self.f32("touch x")?;
            let y = self.f32("touch y")?;
            if !touches.push(Touch {
                id,
                phase,
                screen: Vec2::new(x, y),
            }) {
                return Err(DecodeError::MalformedTouches);
            }
        }
        Ok(touches)
    }

    fn buttons(&mut self, list: &'static str) -> Result<Vec<PointerButton>, DecodeError> {
        let count = self.u8()?;
        let mut buttons = Vec::new();
        for _ in 0..count {
            let code = self.u8()?;
            let Some(button) = PointerButton::find_by_code(code) else {
                return Err(DecodeError::UnknownButton { code });
            };
            if buttons.last().is_some_and(|last| *last >= button) {
                return Err(DecodeError::NotCanonical { list });
            }
            buttons.push(button);
        }
        Ok(buttons)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::{InputEvent, SnapshotBuilder};
    use crate::touch::FingerId;

    /// Where a snapshot's touch count sits: the whole encoding, minus the
    /// list. Written as an offset from the end so that adding a field before
    /// it moves the tests rather than silently retargeting them.
    fn touch_count_at(bytes: &[u8], touches: usize) -> usize {
        // Each touch is a slot byte, a phase byte and two floats.
        bytes.len() - 1 - touches * 10
    }

    fn a_busy_snapshot() -> InputSnapshot {
        let mut builder = SnapshotBuilder::new();
        builder.record(InputEvent::KeyPressed(Key::W));
        builder.record(InputEvent::KeyPressed(Key::Space));
        builder.record(InputEvent::KeyReleased(Key::Space));
        builder.record(InputEvent::PointerMoved {
            id: PointerId::PRIMARY,
            screen: Vec2::new(400.5, -12.25),
        });
        builder.record(InputEvent::ButtonPressed {
            id: PointerId::PRIMARY,
            button: PointerButton::Primary,
        });
        builder.record(InputEvent::Scrolled {
            id: PointerId::PRIMARY,
            lines: -2.5,
        });
        builder.record(InputEvent::Touched {
            finger: FingerId::from_platform(11),
            phase: TouchPhase::Began,
            screen: Vec2::new(12.0, 34.5),
        });
        builder.record(InputEvent::Touched {
            finger: FingerId::from_platform(12),
            phase: TouchPhase::Began,
            screen: Vec2::new(600.0, 480.0),
        });
        builder.first_tick_snapshot()
    }

    #[test]
    fn a_snapshot_survives_the_round_trip_exactly() {
        let snapshot = a_busy_snapshot();
        let bytes = snapshot.encode();
        assert_eq!(InputSnapshot::try_decode(&bytes), Ok(snapshot));
    }

    #[test]
    fn the_bytes_survive_the_round_trip_too() {
        // Both directions, so no two byte strings can mean the same snapshot.
        let bytes = a_busy_snapshot().encode();
        let decoded = InputSnapshot::try_decode(&bytes).unwrap();
        assert_eq!(decoded.encode(), bytes);
    }

    #[test]
    fn an_empty_snapshot_round_trips() {
        let snapshot = InputSnapshot::new();
        let bytes = snapshot.encode();
        assert_eq!(InputSnapshot::try_decode(&bytes), Ok(snapshot));
    }

    #[test]
    fn foreign_bytes_are_refused() {
        assert_eq!(
            InputSnapshot::try_decode(b"not a recording at all"),
            Err(DecodeError::NotASnapshot)
        );
    }

    #[test]
    fn a_newer_version_is_refused_rather_than_guessed_at() {
        let mut bytes = InputSnapshot::new().encode();
        bytes[4] = 99;
        assert_eq!(
            InputSnapshot::try_decode(&bytes),
            Err(DecodeError::UnsupportedVersion { found: 99 })
        );
    }

    #[test]
    fn every_truncation_is_caught() {
        // A crashed recorder leaves a partial write; every prefix must fail
        // cleanly rather than decode into something plausible.
        let bytes = a_busy_snapshot().encode();
        for length in 0..bytes.len() {
            assert!(
                InputSnapshot::try_decode(&bytes[..length]).is_err(),
                "prefix of {length} bytes decoded"
            );
        }
    }

    #[test]
    fn trailing_bytes_are_caught() {
        let mut bytes = InputSnapshot::new().encode();
        bytes.push(0);
        assert_eq!(
            InputSnapshot::try_decode(&bytes),
            Err(DecodeError::TrailingBytes { count: 1 })
        );
    }

    #[test]
    fn an_unknown_key_code_is_named_in_the_error() {
        let mut snapshot = InputSnapshot::new();
        snapshot.held = vec![Key::A];
        let mut bytes = snapshot.encode();
        // The first key code sits after magic, version, and the list length.
        let at = MAGIC.len() + 2 + 4;
        bytes[at] = 250;
        bytes[at + 1] = 0;
        assert_eq!(
            InputSnapshot::try_decode(&bytes),
            Err(DecodeError::UnknownKey { code: 250 })
        );
    }

    #[test]
    fn an_unsorted_list_is_refused() {
        // Canonical order is what makes equal snapshots encode to equal bytes;
        // accepting a shuffled list would break the byte round trip.
        let mut snapshot = InputSnapshot::new();
        snapshot.held = vec![Key::B, Key::A];
        assert_eq!(
            InputSnapshot::try_decode(&snapshot.encode()),
            Err(DecodeError::NotCanonical { list: "held" })
        );
    }

    #[test]
    fn a_nan_position_is_refused() {
        let mut snapshot = InputSnapshot::new();
        snapshot.pointers[0].screen = Vec2::new(f32::NAN, 0.0);
        assert_eq!(
            InputSnapshot::try_decode(&snapshot.encode()),
            Err(DecodeError::NotFinite { field: "pointer x" })
        );
    }

    #[test]
    fn a_snapshot_without_its_primary_pointer_is_refused() {
        let mut snapshot = InputSnapshot::new();
        snapshot.pointers = vec![PointerState::new(PointerId::touch(0))];
        assert_eq!(
            InputSnapshot::try_decode(&snapshot.encode()),
            Err(DecodeError::MalformedPointers)
        );
    }

    #[test]
    fn a_snapshot_with_touches_survives_both_round_trips() {
        // The addition is only additive if it round-trips like everything else
        // that was already here.
        let snapshot = a_busy_snapshot();
        assert_eq!(snapshot.touches().len(), 2, "the fixture has fingers on it");
        let bytes = snapshot.encode();
        assert_eq!(InputSnapshot::try_decode(&bytes), Ok(snapshot));
        let Ok(decoded) = InputSnapshot::try_decode(&bytes) else {
            panic!("just encoded");
        };
        assert_eq!(decoded.encode(), bytes);
    }

    #[test]
    fn a_version_one_snapshot_still_decodes_and_means_no_touches() {
        // The compatibility promise, at the level of one snapshot: everything
        // before the touch list is byte-identical, so an old snapshot is this
        // one with the reading stopped early (ADR-0043). `tests/old_recordings.rs`
        // makes the same check against a file a pre-touch build actually wrote.
        let mut snapshot = a_busy_snapshot();
        let current = snapshot.encode();
        let mut old = current[..touch_count_at(&current, 2)].to_vec();
        old[4] = 1;
        old[5] = 0;

        snapshot.touches = TouchList::new();
        assert_eq!(InputSnapshot::try_decode(&old), Ok(snapshot));
    }

    #[test]
    fn a_version_one_snapshot_re_encodes_as_version_two() {
        // Stated rather than discovered: the byte round trip holds for what
        // this build writes, and reading an old snapshot is an upgrade. A
        // recording replayed through this engine and written back out is a
        // version 2 recording.
        let current = a_busy_snapshot().encode();
        let mut old = current[..touch_count_at(&current, 2)].to_vec();
        old[4] = 1;
        old[5] = 0;
        let Ok(decoded) = InputSnapshot::try_decode(&old) else {
            panic!("version 1 is readable");
        };
        let again = decoded.encode();
        assert_ne!(again, old);
        assert_eq!(&again[4..6], VERSION.to_le_bytes(), "written at version 2");
        assert_eq!(&again[..4], &old[..4], "and it is the same format");
    }

    #[test]
    fn a_touch_phase_code_this_build_does_not_know_is_refused() {
        let snapshot = a_busy_snapshot();
        let mut bytes = snapshot.encode();
        // The first touch's phase byte follows the count and its slot.
        let at = touch_count_at(&bytes, 2) + 2;
        bytes[at] = 99;
        assert_eq!(
            InputSnapshot::try_decode(&bytes),
            Err(DecodeError::UnknownTouchPhase { code: 99 })
        );
    }

    #[test]
    fn more_touches_than_the_format_holds_is_refused() {
        // The bound is the contract, so the decoder is where a file that does
        // not respect it stops — not a `Vec` that quietly grows.
        let snapshot = a_busy_snapshot();
        let mut bytes = snapshot.encode();
        let at = touch_count_at(&bytes, 2);
        bytes[at] = u8::try_from(MAX_TOUCHES + 1).unwrap_or(u8::MAX);
        assert_eq!(
            InputSnapshot::try_decode(&bytes),
            Err(DecodeError::MalformedTouches)
        );
    }

    #[test]
    fn a_touch_in_a_slot_the_format_does_not_have_is_refused() {
        let snapshot = a_busy_snapshot();
        let mut bytes = snapshot.encode();
        let at = touch_count_at(&bytes, 2) + 1;
        bytes[at] = 9;
        assert_eq!(
            InputSnapshot::try_decode(&bytes),
            Err(DecodeError::MalformedTouches)
        );
    }

    #[test]
    fn touches_out_of_slot_order_are_refused() {
        // Canonical order is what makes equal snapshots equal bytes, and the
        // touch list is canonical the same way the key list is.
        let snapshot = a_busy_snapshot();
        let mut bytes = snapshot.encode();
        let first = touch_count_at(&bytes, 2) + 1;
        bytes[first] = 1;
        bytes[first + 10] = 0;
        assert_eq!(
            InputSnapshot::try_decode(&bytes),
            Err(DecodeError::NotCanonical { list: "touches" })
        );
    }

    #[test]
    fn a_nan_touch_position_is_refused() {
        let Some(id) = TouchId::find_by_slot(0) else {
            panic!("the format has a slot 0");
        };
        let mut snapshot = InputSnapshot::new();
        snapshot.touches.push(Touch {
            id,
            phase: TouchPhase::Began,
            screen: Vec2::new(0.0, f32::INFINITY),
        });
        assert_eq!(
            InputSnapshot::try_decode(&snapshot.encode()),
            Err(DecodeError::NotFinite { field: "touch y" })
        );
    }

    #[test]
    fn a_decode_error_reads_like_every_other_engine_error() {
        let message = DecodeError::UnknownKey { code: 250 }.to_string();
        assert!(
            message.starts_with("[jidousha] unknown key code 250"),
            "{message}"
        );
        assert!(message.contains("likely cause:"), "{message}");
        assert!(message.contains("fix:"), "{message}");
    }
}
