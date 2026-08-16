# ADR-0018: Text carries its depth in its style, not as a trailing argument

Status: accepted · 2026-08-16

## Context

The `Submit` trait gives a Draw system five verbs. Four of them take depth last:

```rust
fn rect(&mut self, rect: Rect, color: Color, depth: Depth);
fn circle(&mut self, center: Vec2, radius: f32, color: Color, depth: Depth);
fn line(&mut self, from: Vec2, to: Vec2, thickness: f32, color: Color, depth: Depth);
fn text(&mut self, at: Vec2, text: &str, style: TextStyle);   // depth is inside
```

`TextStyle` is `{ size, color, depth }`, so text's depth is a field rather than
an argument. E0 run 1 found this by trying it, and filed it against the first
rule in CLAUDE.md — *one way to do everything*:

> Four verbs take depth as a trailing argument; the fifth hides it in a struct.
> For a codebase whose first rule is "one way to do everything", this is a
> wobble, and the document gives no signatures at all so you only find it by
> trying.

That is a fair reading of the evidence available. Nothing in `docs/adr/` said
the shape was chosen, and an oddity that survives review by being re-argued each
time is exactly what the `DELIBERATE:` convention exists to prevent.

The cost was small — the run wrote the call correctly at the first attempt, from
an example — but the cost is not the point. An unexplained inconsistency in a
five-verb API is a standing invitation to "fix" it, and the next reader to
notice will be an agent with a strong drive to tidy.

## Decision

**Keep the asymmetry. `TextStyle` carries `depth`, and `ctx.text` takes no
trailing `Depth`.**

Text is the only verb whose appearance needs more than the arguments naming what
to draw. A rectangle is fully described by its bounds and a color; a line adds a
thickness; text needs a size *and* a color, and needs them in world units
against a font whose metrics only the engine knows — which is also why
`TextStyle::width_of` exists. So text has a style object whether or not depth
lives in it, and the question is not "struct or argument" but "one struct or a
struct and an argument".

Splitting depth out would make every call site pass two things that both
describe how the text looks:

```rust
ctx.text(at, "score", TextStyle { size: 1.0, color: Color::WHITE }, Depth::layer(2));
```

That is a worse call than the one we have, and it buys a consistency that only
reads as consistency in a list of four signatures written out together — which
is to say, in a document, not in a game. The `Depth` argument on the other four
verbs is doing the same job `TextStyle.depth` does; it is spelled differently
because those verbs have nowhere to put it.

This is the same distinction `public-api.md` §3 already draws for sprites, and
drawing it twice is the argument for it being real rather than convenient:

> `Depth { layer, z }` is the uniform depth argument for immediate primitives;
> sprites carry `layer` in `Sprite` and `z` in `Transform` because they're
> entity data. This asymmetry is DELIBERATE: components are the entity-driven
> path, `Depth` is the immediate path; merging them made both worse.

The rule underneath all three cases: **depth travels with whatever else
describes the thing's appearance.** For a sprite that is its components; for
text that is its style; for a bare shape there is nothing else, so it is an
argument. "One way to do everything" is about there being one way to set a
drawn thing's depth — and there is: you set it on the thing that says how the
drawing looks.

## Consequences

- `submit.rs` carries a `DELIBERATE:` tag at `text` pointing here, which is the
  part that stops this being rediscovered.
- The signatures are now in `docs/api/`, so the next reader meets the shape in
  the reference rather than by compiling. That is a change of ADR-0018's own
  making only in the sense that it happened in the same pass; the fix is F-001's.
- If a fourth kind of drawn thing arrives that needs a style object, it takes
  its depth in that object too. The rule generalises; the four bare verbs are
  the special case, not text.
- **Not a licence for further asymmetry.** This ADR justifies depth's location,
  not divergence generally. `color` appears in all five and must stay
  comparable; a verb that renamed it, reordered its arguments, or took its color
  in a struct for no reason beyond taste would be the wobble E0 thought it had
  found.

## Alternatives considered

**Add a trailing `Depth` to `ctx.text` and remove it from `TextStyle`.** The
consistent-looking option, and the one E0's note implies. Rejected: it makes
every call pass two appearance structs, and it makes `TextStyle` a thing you
cannot fully describe a piece of text with, which is what it is for.

**Give the other four verbs style objects too** (`RectStyle { color, depth }`).
Consistent in the other direction, and much worse: four new types, each with one
useful field, so that `ctx.rect` can take two arguments where it took three.
This is the shape of consistency that costs more than the inconsistency did.

**Leave it undecided and let the next E0 run say whether it costs anything.**
Tempting, since the measured cost so far is one paragraph in a findings file.
Rejected because the run *did* report it, and a finding reported and then not
answered is how a document teaches its readers that reporting is pointless. The
answer can be "this is deliberate, here is why" — but it has to be written down,
which is the whole mechanism.
