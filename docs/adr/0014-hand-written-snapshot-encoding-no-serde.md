# ADR-0014: Recording formats are hand-written byte encodings, not `serde`

Status: accepted · 2026-08-16

## Context

Input snapshots must be written down and read back: that is what a recording is
(input.md §5), and I0's exit criterion is that a snapshot survives the round
trip. Asset readiness will be recorded into the same stream (assets.md §4), and
the draw transcript is a third candidate later.

The reflex in Rust is `#[derive(Serialize, Deserialize)]` and a format crate.
The question is whether that reflex is right for data whose defining property is
that it must be byte-identical across machines and stable across engine
versions.

## Decision

Recorded formats are **hand-written**: an `encode` that appends fixed-width
little-endian fields to a `Vec<u8>`, and a `try_decode` that reads them back
with an explicit length check at every step. No `serde`, no derive, no format
crate, in `jidousha-input` or in any crate that writes a recording.

The rules the hand-written codecs follow:

- Every integer is fixed-width and little-endian; every float is written as its
  IEEE bit pattern. No varints, no native-endian anything.
- Every value carries a magic number and a version, and a decoder refuses a
  version it does not know rather than misreading it.
- Decoding is **strict**: anything that would not re-encode to the bytes it came
  from is an error. Unsorted lists, duplicate entries, trailing bytes, NaN, and
  unknown enum codes are all refused. Both round trips hold —
  `decode(encode(x)) == x` and `encode(decode(b)) == b`.
- Enum wire codes are assigned by hand and never change. A variant keeps its
  number forever; new variants take unused ones.

## Rationale

- **Byte-stability is the whole point, and it is ours to guarantee.** A
  recording made on Windows must replay on Linux and in a browser, byte for
  byte. That is a property of the format, and with `serde` it becomes a property
  of a format crate's version, its feature flags, and its choices about integer
  packing — none of which we control, and all of which can change in a patch
  release without breaking `serde`'s own compatibility promises.
- **A recording outlives the code that wrote it.** The bug-repro workflow
  (input.md §5) is worth having only if last month's recording still replays
  today. Hand-assigned wire codes and an explicit version byte make that a thing
  we decide; derived encodings make it a thing that follows field order, so
  reordering two struct fields silently invalidates every recording on disk.
- **Strictness is only available to us.** "Refuse a list that is not sorted"
  keeps equal snapshots encoding to equal bytes, which is what makes recordings
  comparable. A derived deserializer accepts anything the serializer could have
  produced and much that it could not.
- **Dependency budget** (practices §5.8): `serde` + `serde_derive` brings
  `syn`, `quote`, and `proc-macro2`, and a format crate on top. The engine's
  entire external tree is currently one crate. This buys nothing we need for a
  format that is a hundred lines.
- The cost is genuinely small: the snapshot codec is ~120 lines including its
  error type, and it is exercised by property tests over thousands of generated
  snapshots.

## Consequences

- Adding a field to a recorded type means editing its `encode` and `try_decode`
  together, and bumping the version. This is deliberate friction: the format is
  a compatibility surface, and it should be as visible to change as one.
- Every recorded type needs its own codec. If a third or fourth appears and the
  shape repeats, the answer is a small shared writer/reader helper in the
  engine — not a serialization framework.
- Game code that wants to save *its own* data is unaffected: a game may depend
  on whatever it likes. This ADR binds engine crates and the formats the engine
  defines.
- `DELIBERATE:` tags at the codec sites point here — a future agent proposing
  "replace this boilerplate with serde" must read this first.

## Alternatives rejected

- **`serde` + `bincode`/`postcard`**: the obvious choice, and it makes the
  cross-platform byte-stability CONTRACT depend on a third party's encoding
  decisions rather than on ours.
- **`serde` with a hand-written `Serializer`**: all of the dependency cost, none
  of the convenience, and still a derived field order.
- **JSON or another text format**: readable, and immediately fatal — f32 round
  trips through decimal text are exactly the place a replay diverges.
