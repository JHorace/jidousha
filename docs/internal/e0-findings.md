# E0 findings — what building a game with this engine actually cost

Status: **one run, all fourteen findings fixed, awaiting run 2.** The harness is
`docs/internal/e0-prompt.md`; the milestone is implementation-plan.md §3. The bar
is two consecutive runs with no new `engine` or `docs` findings, so run 1 being
fully answered is the start of the measurement, not the end of it.

E0 is the project's definition of working: a fresh Claude Code session, given
only `docs/api/jidousha-api.md` and `crates/jidousha/examples/`, builds a
playable Pong. This file is where its frictions get root-caused.

---

## 1. The rule

**Every friction is an engine bug or a docs gap until proven otherwise.**

The tempting reading of an E0 failure is "the prompt was unclear" or "the agent
made a mistake". Sometimes that is true. It is the *last* explanation to reach
for, because it is the one that requires nothing to change, and a milestone
whose failures cost nothing is a milestone that measures nothing.

Each finding is classified as exactly one of:

| Class | Meaning | What it costs |
|---|---|---|
| `engine` | The engine is missing something, or does something surprising. | A code change, and usually an ADR if the surprise was deliberate. |
| `docs` | The API document does not say something a game author needs. | A change to the facade's doc comments or `tools/api-doc/` prose, then `tools/gen-api-doc`. |
| `author` | The run made an ordinary mistake the document does cover. | Nothing — but the finding stays, because three `author` findings on the same topic is a `docs` finding wearing a hat. |

A finding classified `author` needs a quote from `docs/api/jidousha-api.md`
showing where the answer already was. Without that quote it is not an `author`
finding.

## 2. The bar

E0 passes when **two consecutive runs produce no new `engine` or `docs`
findings**. Not "no findings" — an `author` finding is allowed in a passing run,
and the second clean run is what distinguishes a fixed engine from a lucky one.

A run whose transcript shows a read under `crates/*/src/`, `docs/internal/` or
`docs/adr/` is void and does not count towards the two. Void runs are logged
below anyway: a restriction that is hard to honor is itself a finding about the
prompt.

## 3. Run log

| Run | Date | Outcome | New `engine` | New `docs` | New `author` | Notes |
|---|---|---|---|---|---|---|
| 1 | 2026-08-16 | Pong shipped; **not** a pass | 5 | 9 | 1 | Game compiled first try, `--verify` green, human playtest good. The document did not survive it. Raw notes: `E0-NOTES.md`. All 14 fixed; §6. |

Run 1 produced a working, fun Pong and a document-shaped hole underneath it. The
game is not the measurement — `E0-NOTES.md` is — and it says the run could not
have written a single call from `docs/api/jidousha-api.md` alone.

**The run was valid.** Its transcript shows no read under `crates/*/src/`,
`docs/internal/` or `docs/adr/`; the restriction held, including at the points
where honoring it cost the run a feature (see F-002).

## 4. Findings

Fifteen findings from run 1. F-001 is the parent of most of the `docs` set: six
of them are one bug — the Reference has no signatures — observed from six
different angles. They are kept separate anyway, because each one names a
distinct thing a game author went looking for and did not find, and a fix that
satisfies F-001 but leaves any of them unanswered has not finished.

### F-001 — "The API document is a table of contents, not an API"

Class: docs · Run: 1 · Fixed in: `4f9c10f`

**What the run did.** Wrote ~450 lines of Pong against
`docs/api/jidousha-api.md` and `crates/jidousha/examples/`, as the prompt
requires.

**What happened.** From `E0-NOTES.md`:

> Its "Reference" section is ~90 bullet points of the form `**Rect** — An
> axis-aligned rectangle, in whatever space its user is working in`. There are
> no signatures, no struct fields, no enum variants, no argument orders, no
> defaults. It tells you that a thing exists and roughly what it is *for*. It
> never tells you how to call it.
>
> **If `examples/` had not been in scope, I could not have written this game from
> the document.** Not "it would have been harder" — I could not have made the
> first `ctx.rect` call, because nothing states its argument order.

Every call in the game came from reading other examples: `World::query_mut`,
`World::component_mut`, `Rect::from_center_size`, `TextStyle::width_of`,
`Camera::visible_bounds`, `Depth::layer`, `Time::fixed_dt`, `Rng::next_f32`,
`InputScript::hold`, `Input::just_pressed`, `Vec2::normalize_or_zero`.

**Root cause.** Not a judgement call that went the wrong way — a spec that was
never implemented. `public-api.md` §4 defines the Reference as "the §2
inventory, grouped as above, one entry per item: **signature, one-liner, tiny
example**". `tools/gen-api-doc` emits the one-liner only. Its module docstring
records the narrowing as intentional — "Only the first sentence is taken: the
reference is an index, not a manual" — but that is the generator disagreeing
with its own design document, and implementation-plan.md §2 puts subsystem
design docs above implementation. The design doc wins; the generator is the
thing that is wrong.

Nothing forced the shape. The committed document is 16,461 chars ≈ 4,115
estimated tokens against the 25,000-token budget the generator itself
enforces — six times the headroom needed.

The mechanism is `doc_summaries()`, which scans for
`^\s*pub (?:struct|enum|trait|fn|const|type|mod)\s+(\w+)` and keeps only
`first_sentence()` of the doc block above it. It never reads a declaration, so
no field, variant, argument or return type has ever been able to reach the page.

**Fix.** Teach `tools/gen-api-doc` to extract declarations as well as summaries:
struct fields with types, enum variants, trait and inherent-impl method
signatures, free-function signatures, associated consts, and `Default` values.
Render each entry as a Rust block. Measured cost of full extraction is ≈12.2k
tokens, comfortably inside the budget, so no curation is required to fit.

The fix must also make thinness *loud*. The reference's failure mode is not
being wrong, it is being thin — and a thin entry is indistinguishable from a
complete one to the agent reading it. Every exported item must yield at least a
declaration or the generator fails, per CLAUDE.md rule 3.

**Still open: the third of §4 that is not signatures.** §4 asks for "signature,
one-liner, **tiny example**". The commit above landed the first two. Per-item
examples do not exist, and the finding is marked fixed anyway because what the
run reported — that it could not make a call from this document — is answered by
an argument list.

Deferred rather than dropped, and the reasoning is the milestone's own: run 2 is
the instrument that says whether signatures alone were enough, and spending the
budget before it reports is guessing at the answer it exists to give. The
document sits at ~13.8k tokens of 25,000; the ~39 doctests already in the crates
would cost roughly 5k more, so the budget is not what is deciding this.

If it is taken up, the mechanism is to harvest those doctests rather than write
anything: the example a game copies is then the example CI compiles, which is
the same argument that embeds `quickstart.rs` verbatim. One obstacle to plan
for — most of them open with `use jidousha_core::{...}`, which is forbidden
vocabulary, so the harvest has to drop `use` lines. That is not a loss: the
document's first sentence is already "everything here is reachable from one
import", and a doctest's import line is never a line a game would write.

**This note is the point.** A spec and its implementation disagreeing quietly is
what produced F-001 in the first place — `gen-api-doc` rationalised the gap in
its own docstring and nothing else recorded it, for the whole life of the
project. A gap that is written down is a decision; the same gap unwritten is the
bug again, one third the size.

### F-002 — "`Key` has no listed variants"

Class: docs · Run: 1 · Fixed in: `4f9c10f`

**What the run did.** Wanted arrow keys for a second control scheme, and
`Escape` to quit, and a digit or `P` to pause.

**What happened.**

> I guessed `Key::ArrowUp` / `Key::ArrowDown` from the shape of `Key::Space` and
> it compiled first try — that was luck, not inference. `ArrowUp` vs `Up` vs
> `Arrow(Up)` were all equally plausible.
>
> I also wanted `Key::Escape` (to quit) and `Key::Digit1`/`Key::P` (pause) and
> gave up on all of them rather than play compile-error roulette.

**Root cause.** All 83 variants exist, including every one the run abandoned —
`Escape`, `Digit1`, `P`, and the four arrows. The document lists the type and
not one variant, and the run correctly refused to guess repeatedly at a surface
it could not see.

There is a second, sharper cause specific to `Key`: it is generated by a
`macro_rules! keys` invocation, so its body is `$( $name, )*` and a plain
enum-body scan finds nothing. That is why this entry would stay empty even after
F-001's general fix, and it is why the fix needs a loud completeness gate rather
than a best-effort extractor.

The run notes that `input_echo.rs` *prints* held keys at runtime, "which is no
help to someone writing code in a container with no display" — the one existing
route to the list is unavailable in exactly the environment E0 runs in.

**Fix.** Render the full variant list, read from the `keys!` invocation, keeping
that list's own grouping and comments (letters, digit row, arrows, editing keys,
modifiers, function keys, punctuation). No run compression — `A–Z` is how a
variant goes missing without anyone noticing. Fail the generator if a public
enum yields no variants, so the next macro-generated enum cannot repeat this
silently.

### F-003 — `Rect::contains` and `Rect::overlaps` exist, and are invisible

Class: docs · Run: 1 · Fixed in: `4f9c10f`

**What the run did.** Went looking for a rectangle overlap test before writing
its own, and filed the result under "Things I expected to exist and could not
find":

> `Rect` has `from_center_size`, `min`, `max`, `center()`, `size()`. It has no
> `intersects`, no `contains`, no `overlaps`. "Do these two rectangles overlap"
> is the very first thing every 2D game needs after it can draw.

**What happened.** It wrote the arithmetic by hand.

**Root cause.** **The methods exist.** `crates/jidousha-core/src/visual.rs:142`
and `:151` define `Rect::contains(self, point: Vec2) -> bool` and
`Rect::overlaps(self, other: Rect) -> bool`, both half-open, with tests. The
engine was already right and the document could not say so, because a bullet
naming a type cannot name its methods.

**This is the finding that measures what F-001 costs**, and the reason it is
classified `docs` rather than `engine` is the whole point: the run's own
conclusion — that the absence was "arguably correct for a v1 that does not want
to own collision" — was a reasonable inference from the evidence available, and
it was wrong. A document that hides a feature is worse than one that lacks it,
because it makes the reader confidently reimplement it.

**Fix.** F-001. Once the entry carries `impl Rect { … }`, an agent grepping for
"overlap" finds `overlaps` on the first try. A content test asserting the
reference contains `pub fn overlaps(self, other: Rect) -> bool` is the
regression guard.

### F-004 — `GameConfig`'s fields are unlisted and `fixed_dt`'s value is never stated

Class: docs · Run: 1 · Fixed in: `4f9c10f, fbd22e9`

**What the run did.** Wanted to set the window's initial size, because the
game's field is a fixed 34×19 world units. And needed the tick rate to express
`SERVE_DELAY`, a pause measured in ticks.

**What happened.**

> I could not tell whether such a field exists. I left `..GameConfig::default()`
> and wrote a comment admitting a narrow window will crop the field. That is a
> gameplay decision made by ignorance.

and

> The Concepts section says the timestep is fixed and `Time::fixed_dt` is "the
> same number every tick". It never says what number. [...] to pick "about three
> quarters of a second" I had to assume 60 Hz. I got 60 Hz from a comment inside
> `scripted_player.rs` ("90 ticks at 4 units/second on a 60 Hz timestep: 6
> units"), i.e. from arithmetic in an example's assertion, not from the
> reference.

**Root cause.** Two distinct gaps behind one symptom.

The tick rate is `Seconds(1.0 / 60.0)`, set in `GameConfig::default()`
(`crates/jidousha-core/src/app.rs:56`) — and `fixed_dt` is a `GameConfig` field,
so a game may *change* it. The document says neither. The run did not merely
fail to learn the number; it had no way to learn the number was its own to pick.

The window size genuinely does not exist — see F-013. But the run could not
distinguish "no such field" from "a field I cannot see", and that ambiguity is
this finding: with the fields listed, the absence would have been a fact rather
than a guess.

**Fix.** F-001 renders `GameConfig`'s three fields and its `Default` body, which
puts `1.0 / 60.0` on the page as a value rather than a convention. `concepts.md`
states the tick rate in prose beside the fixed-timestep paragraph, because a
number a game expresses durations in should not have to be inferred from a
struct literal. The missing field itself is F-013.

### F-005 — `message`'s signature is unknown, and its entry cites a document the reader may not open

Class: docs · Run: 1 · Fixed in: `4f9c10f, f5a9910`

**What the run did.** Needed the engine's message format for its own `--verify`
failures.

**What happened.**

> The reference entry is `**message** — The failure in the engine's message
> format (core.md §9)`, pointing at a document I am not allowed to read. It is
> in the prelude, so it is clearly meant for games. I copied the four-argument
> shape (what / specifics / likely cause / fix) out of `prototype_kit/verify.rs`.
> I still do not know if there is a fifth optional thing or what the field names
> are.

**Root cause.** Two bugs in one bullet.

The signature is `pub fn message(what: &str, specifics: &str, likely_cause:
&str, fix: &str) -> String` (`crates/jidousha-core/src/error.rs:32`). Four
arguments, no fifth. F-001 again.

The citation is worse, and separate: `docs/api/` carries a pointer into
`docs/internal/`, which breaches the §4 CONTRACT that the game-facing document
"never mentions internal crates, the backend seam, archetype storage, or any
implementation vocabulary". The generator has a `forbidden_words` gate for
exactly this and it did not fire, because `FORBIDDEN` lists crate names and
seam types but no internal *document paths*. The Conventions digest leaks the
same way, carrying `docs/internal/renderer.md §5` into the same document.

The gate was not weak, it was aimed one inch to the left. That is the more
useful reading: an enforcement mechanism that covers a class incompletely reads
as covering it completely.

**Fix.** Add internal doc-path patterns to `FORBIDDEN`, reword the `///` at
`error.rs` and the leaking `conventions.md` sentences so the generated text
stands alone, and add a test that the check rejects a document containing
`core.md`. F-001 supplies the signature.

### F-006 — `Camera::viewport` is not in the document at all

Class: docs · Run: 1 · Fixed in: `4f9c10f`

**What the run did.** Needed `Camera { viewport, ..*world.resource::<Camera>() }`
per frame in its headless verification path.

**What happened.**

> Not mentioned in the document at all. [...] I only know that because
> `prototype_kit/verify.rs` does it. Who owns `viewport` in a windowed run — the
> driver? the game? — is not stated.

**Root cause.** F-001: `Camera`'s four fields are `center`, `height`,
`clear_color` and `viewport`, and a name-only bullet shows none of them.
Ownership is real information that no signature carries, and it is not written
down anywhere a game may read: the driver maintains `viewport` on resize, and a
headless caller sets it itself.

**Fix.** F-001 renders the fields and the `Default` body — which also tells a
reader the headless viewport is 1280×720, the other half of what the run was
missing. The ownership sentence belongs in `Camera`'s own doc comment, where it
reaches rustdoc and the reference together. Investigating this finding turned up
F-012, which is the engine half of the same confusion.

### F-007 — `Startup` is documented as running before the first tick; it runs inside it

Class: docs · Run: 1 · Fixed in: `fbd22e9`

**What the run did.** Built a rally harness that reads the ball's position
*before* each tick, to steer a paddle at it.

**What happened.** A panic on tick 1, indexing an empty `Vec`.

> The reference says `**Startup** — Runs once, before the first tick`. I read
> that as "before you call `tick()`", i.e. `headless(...)` returns a sim whose
> world is already populated. It does not. The world is empty until the first
> `sim.tick()` returns.

**Root cause.** The document is wrong, not merely thin — the only finding in
this set where the page states something untrue. `Simulation::tick`
(`crates/jidousha-core/src/simulation.rs:111-130`) calls `start()` first, so the
order inside a first `tick()` is Startup → `Time::advance()` → Update. "Before
the first tick" is true of the *phase order* and false of the *call*, and the
run took the reading that matters to someone writing a driver loop.

The text comes from the `///` on `pub struct Startup`
(`crates/jidousha-core/src/schedule.rs:91`), so this is a source fix, not a
generator fix. `concepts.md` hedges the same way with "`Startup` once at the
beginning".

This finding gets more dangerous after F-001, not less: enriching the reference
makes every doc comment on it more authoritative.

**Fix.** The run's own suggested wording, which is unambiguous: "Runs once, at
the start of the first tick." Applied at the definition and in `concepts.md`.

### F-008 — Alpha reads brighter than the number suggests, and the document implies otherwise

Class: docs · Run: 1 · Fixed in: `fbd22e9`

**What the run did.** Picked 0.16 alpha for field markings.

**What happened.** Nothing — the run got this right, and says why:

> Not something I hit, because `prototype_kit` warns about it in a comment
> (blending happens in linear light, so 0.06 white on dark reads as solid grey).
> [...] The API document's Color section says "sRGB-encoded... linearization
> happens inside the render backend, invisibly" — which is exactly the sentence
> that would lead someone to expect alpha to behave the way the number looks.
> The example knows better than the document.

**Root cause.** A near miss is still a finding: the run was saved by a comment
in an example it happened to read, not by the document. The Conventions digest
is included whole from `docs/conventions.md`, whose Color section says
linearization is "invisible" — accurate about encoding, and actively misleading
about blending, which happens in linear light and is therefore very visible in
exactly the case a prototype hits first (a low-alpha overlay on a dark
background).

**Fix.** Amend the Color section in `docs/conventions.md`, upstream of the
digest, to state the consequence `prototype_kit/main.rs:232-234` already states
in a comment. Fixing it in `conventions.md` rather than in the digest keeps one
copy.

### F-009 — Whether a game needs an `Assets` resource is never stated

Class: docs · Run: 1 · Fixed in: `fbd22e9`

**What the run did.** Wrote a game of pure shapes and text, touching no assets,
and could not tell whether that was supported.

**What happened.**

> I could not tell from the document whether `run()` or `headless()` requires an
> `Assets` resource to be present, so I wrote the game without one and braced
> for a panic. It was fine. Worth stating explicitly somewhere: **a game of pure
> shapes needs no asset story.** That is a genuine strength and the document
> buries it.

**Root cause.** Neither `run()` nor `headless()` inserts or requires `Assets`;
the driver guards with `find_resource_mut` and its doc comment
(`crates/jidousha-platform/src/driver/frame.rs:106`) already says "a game with
no `Assets` resource is a game with no assets, which is a perfectly ordinary
thing for a prototype to be". That sentence is in the source, where a game
author may not look.

The Concepts section spends a paragraph on assets and placeholders and never
says they are optional, so a reader infers a pipeline they have to opt out of.
This is the one finding where the document undersells a real design achievement
rather than omitting a fact.

**Fix.** One sentence in `concepts.md`'s assets paragraph.

### F-010 — The road to "assert on what was drawn" is far longer than the document implies

Class: engine · Run: 1 · Fixed in: `e38f60a`

**What the run did.** Wrote `--verify` assertions about what its game drew.

**What happened.** The document promises:

> *"To check what was drawn, render into `jidousha::testing::NullBackend`, which
> records every frame as structured data."*

The run's verdict: "That sentence undersells it by a lot." The actual ceremony
is `create_builtin_textures` → `sim.draw().quads().to_vec()` → `plan_frame` →
`backend.render(&plan)` → `last_frame()`, plus:

> to find out which `BackendTextureId` the font landed on at assertion time,
> building a **second throwaway `NullBackend` and a second texture table** in the
> same order and asking that one, because the real table is out of scope by then.
> `prototype_kit/verify.rs` has a 9-line doc comment apologising for this. I
> copied it verbatim, including the apology's logic, and I do not fully
> understand why the frame does not just carry the mapping.

**Root cause.** It does not carry the mapping by construction. `TextureTable`
(`plan.rs:88-92`) is one-way, with no reverse index and no public iterator;
`plan_frame` resolves to `BackendTextureId` at plan time (`plan.rs:185`); and
the table is long-lived driver state while a plan is per-frame. So `FramePlan`,
`FrameRecord` and `DrawnQuad` have all lost the `TextureId` before any assertion
runs, and reconstructing it means rebuilding the table in the same order.

Classified `engine` rather than `docs` because better prose describing this
ceremony would still leave the ceremony. The tell that it is a design problem
and not a documentation one: a *game example* has to name `RenderBackend` and
`FramePlan` — both on the generator's forbidden-vocabulary list — to assert that
it drew something.

A second, related gap under the same finding: `DrawnQuad` exposes `bounds()`,
`tint` and `texture`, so the only way to assert "the ball was drawn" is to
recognise it by being 0.8×0.8. The run copied that trick too and called it
"fragile the moment two things in a game are the same size", which is correct.

**Fix.** A helper in `jidousha::testing` that owns the backend, the table and the
plan and hands back a `FrameRecord` per tick — collapsing five lines and a
throwaway backend to one call, and removing the reason a game names the backend
seam at all. Optionally, carry the reverse mapping on the frame so
`FrameRecord` can answer "which quads sample the font?" directly. The test that
the fix worked is whether the apology comment in `pong/verify.rs` can be
deleted.

### F-011 — Adapter selection picks the wrong GPU, and the failure message misdiagnoses it

Class: engine · Run: 1 · Fixed in: `2669cfe` · Issue: [#23](https://github.com/JHorace/jidousha/issues/23)

**What the run did.** Nothing — the run never saw its own game. This came from
the repository owner running it on a real Linux desktop after the notes were
written.

**What happened.** On a machine with a discrete NVIDIA GPU (the compositor's)
and an integrated AMD one, *every* windowed example dies at surface setup with
`error 7: importing the supplied dmabufs failed`.
`VK_DRIVER_FILES=/usr/share/vulkan/icd.d/nvidia_icd.json` fixes it completely.

**Root cause.** wgpu selects the integrated GPU; the compositor is on the
discrete one; cross-vendor dmabuf import fails.

**Correction to the report.** Issue #23 and `E0-NOTES.md` both conclude that the
platform crate "is passing wgpu's default — `PowerPreference::None`, which
performs no adapter sorting whatsoever". That is not what the code does.
`crates/jidousha-render-wgpu/src/init.rs:129` passes
`PowerPreference::LowPower` **explicitly**, under a comment justifying it:

```rust
// Low power by default: a 2D prototype does not need the discrete
// GPU, and asking for it costs battery and switching latency.
power_preference: wgpu::PowerPreference::LowPower,
```

That is worse than the report describes. `None` performs no sorting and would
pick whatever enumeration order gives; `LowPower` *actively sorts the integrated
GPU to the front*. The reported symptom, the workaround and the suggested fix
are all correct; only the mechanism needs correcting — and it matters, because
"we forgot to choose" and "we chose, and the reasoning was wrong for multi-GPU
machines" call for different fixes. It also explains why `WGPU_POWER_PREF=high`
had no effect: an explicit value would override `from_env()` even if that were
wired in, and it is not.

The reasoning in that comment is not silly — it is right about battery on a
laptop and wrong about which GPU can import our buffers. The compositor's GPU is
the one that has to accept the frame.

**Secondary root cause, and the part a game author actually meets.** The Wayland
protocol error kills the connection, winit's `run_app` returns `Err`, and the
only mapping that catches it is `crates/jidousha-platform/src/lib.rs:134-138` →
`RunError::EventLoop`, which reports:

> likely cause: the display server went away mid-run, or the window system
> reported a fault
> fix: restart the program; if it repeats, report it with the message above

Both are false here. The run's judgement:

> For a project whose rule is that an error states what happened, its likely
> cause and its fix, this is the one message encountered in the whole exercise
> that got all three wrong — and it is the message a new user is most likely to
> hit first, because it fires before their game runs at all.

The real cause was printed by the Wayland client library one line earlier and
never reaches `detail`, which carries only winit's stringified OS error.

**Fix.** `PowerPreference::HighPerformance` on the windowed path, with the
rationale comment rewritten rather than left arguing for the removed behaviour.
The offscreen path has no surface to present to, so it may keep `LowPower` — with
a comment saying why the two differ, since an unexplained difference reads as an
oversight. `RunError::EventLoop`'s cause and fix text must stop asserting a
display-server fault it cannot know about; a distinct variant for
surface/adapter failure is the fuller answer.

**Not verifiable here.** This container has no display and no GPU adapter, so
the fix can be shown to compile and not to break anything, and cannot be shown
to work. Confirming it needs the reporter's hardware, and issue #23 should be
updated with the correction above and a re-test request.

Two observations from the report that belong in the document rather than the
code, and are not separate findings only because they have no separate fix:

- A game author cannot diagnose this and should not try — adapter selection is
  four crates from anything `DrawCtx` exposes, behind two isolation rules.
- `window_clear` earns its place in the repository on this alone: a windowed
  example small enough to prove the failure is not yours.

### F-012 — `Camera::viewport` is silently never set when a game inserts no `Camera`

Class: engine · Run: 1 · Fixed in: `b577ec1`

**What the run did.** Nothing. **E0 did not find this** — it surfaced while
root-causing F-006, because Pong inserts a `Camera` and so never took the broken
path.

**What happened.** `Driver::resize`
(`crates/jidousha-platform/src/driver/frame.rs:147-154`) writes
`camera.viewport` through `find_resource_mut::<Camera>()`, and `Driver::frame`
(`frame.rs:72-76`) falls back to `Camera::default()` when the resource is
absent. Nothing in `run()` or `Driver::new` ever inserts a `Camera`.

So a game that inserts none draws through a hard-coded 1280×720 viewport
forever, at an aspect ratio unrelated to its actual window, and every resize
event is silently discarded.

**`quickstart.rs` is such a game** — the flagship example, embedded verbatim
into `docs/api/`, which the document's own first line invites every author to
copy and start changing.

**Root cause.** A no-op fallback on a path where the correct behaviour is
knowable: `unwrap_or_default()` on a resource that the driver could simply
ensure exists. It reads as defensive and is the exact shape CLAUDE.md rule 3
forbids — "No silent failure. No no-op fallbacks." Nothing fails, nothing warns,
and the symptom (a slightly wrong aspect ratio) is one a new author would
attribute to their own camera height.

This is included as an E0 finding despite not coming from the run because it was
found *by* the run's process — root-causing F-006 required tracing viewport
ownership, and the question "who sets this?" had the answer "on this path,
nobody".

**Fix.** Have the driver insert a default `Camera` at startup so the resource
always exists and resize always lands. `quickstart.rs` then exercises the
correct path, which matters more than usual because it is the one example every
game starts as a copy of.

### F-013 — A game cannot set its window's initial size

Class: engine · Run: 1 · Fixed in: `555ab42`

**What the run did.** Wanted a 16:9-ish window for a field that is a fixed 34×19
world units, and settled for a comment admitting a narrow window will crop it —
"a gameplay decision made by ignorance" (see F-004).

**What happened.** There is no such field. `GameConfig` carries `title`, `seed`
and `fixed_dt` (`crates/jidousha-core/src/app.rs:36-45`), and
`Driver::window_attributes` (`driver/mod.rs:203-205`) sets only `.with_title()`,
so the OS picks the size.

**Root cause.** A promised field that never landed. `public-api.md` §2 has listed
`GameConfig { title, seed, fixed_dt, asset_root, window_size, camera_height }`
in the inventory from the start, and the struct's own doc comment
(`app.rs:33-35`) says "Fields for subsystems that do not exist yet — asset root,
window size, camera height — arrive with those subsystems". The windowing
subsystem arrived at M5 and its field did not come with it.

That the deferral was recorded in two places and still lapsed is the interesting
part: a note saying "this arrives later" has no mechanism attached, and
milestone M5's checklist did not carry it.

**Fix.** Add `window_size` to `GameConfig` and wire it into
`window_attributes`. Because games write `..GameConfig::default()`, adding a
field disturbs nothing already written — which is the property the struct was
designed for and the reason this is cheap now. `camera_height` remains unlanded
and stays recorded here rather than being quietly dropped.

### F-014 — `ctx.text` puts depth somewhere different from every other draw verb

Class: engine · Run: 1 · Fixed in: `19b2dc9` (ADR-0018; the API is unchanged)

**What the run did.** Drew shapes and a score.

**What happened.**

```rust
ctx.rect(rect,        color, depth);
ctx.circle(at, radius, color, depth);
ctx.line(from, to, width, color, depth);
ctx.text(at, string, style);          // depth lives *inside* TextStyle
```

> Four verbs take depth as a trailing argument; the fifth hides it in a struct.
> For a codebase whose first rule is "one way to do everything", this is a
> wobble, and the document gives no signatures at all so you only find it by
> trying.

**Root cause.** Real, and deliberate, and undocumented — which is the part that
made it cost anything. `Submit`
(`crates/jidousha-render-core/src/submit.rs:33-66`) gives `rect`/`line`/`circle`
a trailing `Depth`; `TextStyle` carries `size`, `color` and `depth` together
because text needs a style object regardless, and its size and color have
nowhere else to live.

The asymmetry is defensible. Its absence from `docs/adr/` is not: an oddity that
survives review by being re-argued each time is exactly what the `DELIBERATE:`
convention exists to stop, and `public-api.md` §3 already documents a sibling
asymmetry (sprites carry depth as component data) without either being linked to
the other.

**Fix.** Keep the API; write the ADR. Splitting depth out of `TextStyle` would
make every `ctx.text` call pass two structs to buy a consistency that costs
more than it returns. Per CLAUDE.md a new deliberate oddity gets an ADR *and* a
`DELIBERATE:` tag at the site, so `submit.rs` gets the tag. F-001 makes the
asymmetry visible in the document, which is what stops the next author finding
it "by trying".

### F-015 — The paddle bounce plane, and the score accounting assertion

Class: author · Run: 1 · Fixed in: n/a (no change)

**What the run did.** Two self-inflicted bugs, both found and fixed by the run
itself: a "clever" symmetric bounce-plane formula that was correct for the left
paddle and put the right paddle's bounce plane 1.5 world units behind it, and an
assertion `peak(score.left) + peak(score.right) == tally.points` that breaks the
moment the game supports a rematch.

**What happened.** Both were fixed. The bounce-plane bug was found by hand-
checking arithmetic on paper, not by running anything — the game ran, the ball
came back, and the first six `--verify` assertions all passed.

**Root cause.** Ordinary game-authoring mistakes in game code. Neither is about
the engine or its document: the engine offers no bounce-plane helper and should
not, and the accounting assertion was the run's own invariant, wrongly stated.

**No supporting quote is offered**, and §1 requires one for an `author`
classification. This is recorded deliberately rather than reclassified: the rule
exists to stop maintainers filing engine bugs under `author`, and neither of
these has an engine or document fix to hide. If a later run hits the same class
of thing, that is the signal to revisit.

**Worth keeping anyway**, for what the run concluded from it:

> That round trip is the argument for `--verify` being about *game invariants*
> rather than "did it run without crashing" — and it is an argument the engine's
> own framing supports well. The tooling made the assertion easy to write once I
> knew what to assert.

That is the acceptance milestone working: the run found a bug nothing else
caught, wrote the assertion that catches it, confirmed the assertion fails on
the old code, and restored the fix.

## 5. Notes on the run's procedure

Two things about run 1 that are not findings but would confuse a later reader.

**The game was registered with `tools/test` before the run passed.** Commit
`6626b9c` added `pong` to `VERIFIABLE_EXAMPLES` (`tools/test:118`).
`e0-prompt.md` step 6 makes that the maintainer's step "on the run that
passes". Harmless — the game verifies green — but the registration is not
evidence of a pass, and the milestone is not ticked.

**The run could not see its own game.** No display and no GPU adapter in the
container, so "it runs in a window and is playable" was inferred, not observed,
until the owner played it. The substitute the run used is worth recording as a
thing that worked: reading `FrameRecord::transcript()` and checking every quad's
world-space extent by eye.

> That is a genuinely good substitute for a screenshot, and it is the reason I am
> reasonably confident about the layout despite never having looked at it.

The human playtest, when it happened, changed no constant: controls good,
opponent hard but fair at roughly a one-in-four win rate, first-to-five the right
match length.

## 6. Where run 2 stands

Every `engine` and `docs` finding from run 1 is fixed. What run 2 measures is
whether the fixes were the right ones — and the honest summary of what changed
under it is short:

- **The reference now carries signatures**, which is most of the list. ~4,100
  tokens became ~13,800 of a 25,000 budget, and the six `docs` findings that
  were one bug seen from six angles are answered together. Whether that is
  *enough* is exactly what a fresh run is for: F-001's real claim was that the
  document could not be written from, and only a run that writes from it can
  say.
- **Two things are guarded rather than merely fixed.** `completeness_failures`
  fails the generator when an exported item yields no declaration, and the
  forbidden-vocabulary gate now covers internal document paths. Both exist
  because the original failures were silent, and a silent failure fixed without
  a gate is a silent failure scheduled to return.
- **One fix cannot be verified here.** F-011 needs the reporter's hardware; this
  container has no display and no adapter. Run 2 will not exercise it either.

What run 2 should be watched for, beyond new findings:

- Whether the enriched reference is *navigable*, not just complete. It is now
  four times its old size, and a document an agent cannot skim is a different
  failure from one that omits things. If a run reports hunting through it, that
  is a finding about the format rather than the content.
- Whether `FrameRecorder` is actually reached for. It is in `testing.md` and in
  `pong/verify.rs`, but `prototype_kit` still shows the long form for a reason
  it now states. If a run copies the long form anyway, the reason is not
  visible enough.
- Whether anything in this file's fixes reads as an invitation to guess. The
  `Key` list, the tick rate, and `GameConfig`'s fields were all things the run
  correctly refused to guess at; the fix is only real if the next run does not
  have to.
- **Whether signatures without examples are enough** — the open third of §4, and
  the one question run 2 is best placed to answer. A run that reads
  `pub fn overlaps(self, other: Rect) -> bool` and calls it correctly says the
  deferral was right. A run that has the signature and still goes to `examples/`
  to see the call being made says it was not, and F-001 is not finished.

## 7. What this file feeds

The `make-game` skill (agent-practices §3) is written from E0's findings after
it passes. A friction that recurs across runs and cannot be designed away is
exactly what a skill is for — and one that *was* designed away must not appear
in the skill at all, because a skill that restates a fixed problem is how the
fix gets undone later.

On the evidence of run 1, most of this file must **not** reach the skill: F-001
through F-009 are all "the document did not say", and a skill that teaches an
agent to work around a thin reference is a skill that removes the pressure to
thicken it.
