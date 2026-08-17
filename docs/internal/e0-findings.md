# E0 findings — what building a game with this engine actually cost

Status: **three runs, thirty-eight findings fixed and one with nothing to fix, awaiting run 4.** The
harness is `docs/internal/e0-prompt.md`; the milestone is implementation-plan.md
§3. The bar is two consecutive runs with no new `engine` or `docs` findings.
Run 3 answered run 2 and then found nine more of its own, so the count of
consecutive clean runs is still zero. Run 1 found five `engine` findings and
run 2 three; **run 3 found none.** Eight of its nine are sentences the document
does not carry, the ninth is an `author` finding with nothing to fix, and the
engine's behaviour was right every time the run went looking.

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
| 1 | 2026-08-16 | Pong shipped; **not** a pass | 5 | 9 | 1 | Game compiled first try, `--verify` green, human playtest good. The document did not survive it. Raw notes: `docs/e0/run-1.md`. All 14 fixed; §6. |
| 2 | 2026-08-17 | Pong shipped; **not** a pass | 3 | 10 | 1 | Every run-1 finding held up. The reference is now callable; the gap moved to "which of these is a resource". Raw notes: `docs/e0/run-2.md`. §6. |
| 3 | 2026-08-17 | Pong shipped; **not** a pass | 0 | 8 | 1 | Zero compile errors on the first `cargo check`; "the API document was enough". No `engine` finding. What is left is what the document does not *say* about behaviour that is already right. Raw notes: `docs/e0/run-3.md`. §6. |

Run 1 produced a working, fun Pong and a document-shaped hole underneath it. The
game is not the measurement — `docs/e0/run-1.md` is — and it says the run could
not have written a single call from `docs/api/jidousha-api.md` alone.

**The run was valid.** Its transcript shows no read under `crates/*/src/`,
`docs/internal/` or `docs/adr/`; the restriction held, including at the points
where honoring it cost the run a feature (see F-002).

Run 2 wrote a second Pong from nothing against the fixed document and **closed
every run-1 finding by using it**: argument orders, `Key`'s variant list,
`GameConfig`'s fields, the stated 1/60 timestep, `Rect::overlaps` and
`FrameRecorder::font_texture()` were each read from the reference and each used.
The game compiled on the first attempt with two warnings, both the author's own.
So F-001's real claim — that the document could not be written from — is
answered, and the fourteen findings under it stay closed.

**It is not a pass**, because it produced fourteen new findings of its own. They
are a different kind: not "the document does not say how to call this" but "the
document does not say that this is a *resource*", plus one generator bug that
had been silently deleting a whole `impl` block from the reference. See F-016.

**Run 2 was valid with one caveat, which is F-020**: the prompt told it to write
into run 1's notes file, so it read run 1's findings before writing a line of
Pong. Three facts — the timestep, `Key::ArrowUp`, `ctx.rect`'s argument order —
were known to it in advance. Its own notes say so, and say where the document
told it each of them independently, so the conclusions stand; the measurement is
weaker than it looks and run 3's will not be.

Run 3 is the first clean measurement: its own file, created by it, no other
run's notes opened. It wrote ~1,300 lines of Pong with **zero compile errors on
the first `cargo check`** and reported no point at which it felt blocked. Its
headline is "the API document was enough", and the fourteen run-2 findings stay
closed.

**It is not a pass**, because it produced nine findings of its own — but they
are a third kind again, and the shift is the interesting result. Run 1's were
"the document does not say how to call this". Run 2's were "the document does
not say that this is a resource". Run 3's are **"the document does not say what
this does"**: not a missing signature or a missing noun, but a missing sentence
about behaviour the engine already gets right. The font draws a loud fallback
box for an unknown character and always has; nothing told the author, and no
assertion available to a game could tell the difference (F-030). Collision is
tested at tick boundaries; true, unstated, and the first thing that bites a ball
(F-034). The advance is exactly 7/9 of `size`; exact, derivable, never written
down (F-031).

**Run 3 was valid.** Its transcript shows no read under `crates/*/src/`,
`docs/internal/`, `docs/adr/` or `docs/e0/`, and §5 of its notes lists the three
things it wanted to grep for and did not — the font's coverage among them, which
is F-030 and which it shipped a documented workaround for rather than an answer.

## 4. Findings

Fifteen findings from run 1 (F-001–F-015), fourteen from run 2 (F-016–F-029)
and nine from run 3 (F-030–F-038). F-001 is the parent of most of run 1's
`docs` set: six of them
are one bug — the Reference has no signatures — observed from six different
angles. They are kept separate anyway, because each one names a distinct thing a
game author went looking for and did not find, and a fix that satisfies F-001
but leaves any of them unanswered has not finished.

Run 2's set has the same shape around a different parent. F-021 — the document
never says which types are resources — is most of it, and F-016 is the one that
would have blocked a new author outright.

### F-001 — "The API document is a table of contents, not an API"

Class: docs · Run: 1 · Fixed in: `4f9c10f`

**What the run did.** Wrote ~450 lines of Pong against
`docs/api/jidousha-api.md` and `crates/jidousha/examples/`, as the prompt
requires.

**What happened.** From `docs/e0/run-1.md`:

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

**Correction to the report.** Issue #23 and `docs/e0/run-1.md` both conclude that the
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

### F-016 — `World`'s entire resource API is missing from the reference

Class: engine · Run: 2 · Fixed in: this commit

**What the run did.** Read `World`'s entry to find out how a game reaches the
score, the round state and the camera.

**What happened.** From `docs/e0/run-2.md`:

> `World`'s impl block lists seventeen methods: `spawn`, `despawn`, `insert`,
> `remove`, `query`, `component_mut`, `commands`, and so on. Not one of them is
> about resources. But the Quickstart — in the same document, above the
> reference — calls `world.insert_resource(Score::default())`,
> `world.find_resource::<Input>()` and `world.resource_mut::<Rng>()`.

The run recovered the set by grepping `examples/` for `resource`, and guarded
with `find_resource` everywhere a miss seemed possible because nothing said
whether `resource::<T>()` panics.

**Root cause.** Not a missing doc comment — all five methods are documented, at
length, with `# Panics` sections. `tools/gen-api-doc` was dropping the whole
block. It scanned sources in path order and attached an `impl` block's members
to a type it had already seen, so `crates/jidousha-core/src/resource.rs`, which
sorts before `world.rs`, looked `World` up, found nothing, and discarded six
signatures without a word. `crates/jidousha-input/src/codec.rs` lost
`InputSnapshot::encode`/`try_decode` the same way.

**The asymmetry was the clue and the run spotted it**: `WorldView` documents its
`resource`/`find_resource` pair correctly, because `WorldView` is declared and
implemented in one file.

**Fix.** The generator reads every source **twice** — declarations first, then
`impl` blocks — so path order cannot decide what reaches the page, and members
are ordered so the declaring file's block comes first whatever the path order
was. A unit test scans two fragments in the wrong order and asserts the members
survive; a content test asserts the six resource methods are in the committed
document. The census line `tools/gen-api-doc` prints went from 251 signatures to
259, which is the number that would have made this visible in review.

**An audit went with it**, because a bug that deletes an `impl` block silently
could have deleted others: exactly two exported types were affected, `World` and
`InputSnapshot`, and both are now in the document. F-017 is the separate class
the audit turned up.

**Also answered, in the doc comments rather than by the fix**: `remove_resource`
exists; `resource::<T>()` and `resource_mut::<T>()` panic with a `message(…)`
naming the type and telling you to insert it during setup. Both summaries now
say so on the signature line, as do `component`/`component_mut`, which had the
same shape.

### F-017 — A type named in a signature and defined nowhere in the document

Class: docs · Run: 2 · Fixed in: this commit

**What the run did.** Read `HeadlessSim::draw()`, documented as returning
`&Submissions`, and went looking for `Submissions`.

**What happened.**

> `Submissions` appears nowhere else: not in the reference, not in the testing
> section, not in Concepts. `prototype_kit` calls `.quads()` on it and that is
> the only evidence it has methods. […] a return type named in a signature and
> then never defined is the one kind of gap that has no workaround if you happen
> to need it.

**Root cause.** The facade is a curation, and `Submissions` was curated out
while the method returning it stayed in. The reference is generated from the
facade's `pub use` lists, so a type can be *named* by a signature it does not
export.

**Fix.** `Submissions` is exported (App and lifecycle) with a
`check-api-coverage` `EXEMPT` entry saying how a game reaches it —
`sim.draw().quads()`, never by name. `DecodeError` gets the same treatment in
`jidousha::testing`: it was already named twice, by `InputSnapshot::try_decode`
and inside `RecordingError::Snapshot`, with no entry of its own.

**The class is wider than the two, and the rest is recorded rather than fixed.**
Scanning the rendered signatures for type positions with no `####` entry also
turns up `ByteSource` (what `asset_source` returns, deliberately opaque),
`AssetHandle` and `AssetKind`, `Phase` and `IntoSystem` (the `add_system`
bounds), `Query`/`ReadOnlyQuery`/`QueryIter`/`QueryIterMut`, `CommandKind`, and
four testing-only types. The query traits are answered by prose instead
(F-023); the bounds and the opaque source are arguably right to stay unnamed.
**A generator gate for this class is not built**, because each remaining entry
needs a decision — export it, or exempt it with a reason — and a gate landed
without those decisions would just be a wall of exemptions. It is the obvious
next piece of work on the generator and it is written down here so it is not
rediscovered.

### F-018 — `Vec2` is out of scope of a document that says nothing is out of scope

Class: docs · Run: 2 · Fixed in: this commit

**What happened.** The math module's entry ended:

> Also in `math`, re-exported from `glam` and documented there.

against a document that opens "If something you want is not here, it is not part
of v1".

> `Vec2` is in almost every line of this game — `length`, `abs`, `splat`, `min`,
> arithmetic operators, `const fn new` in a `const` item — and it is part of v1,
> and it is not here. […] This cost me nothing, because I happen to know glam.
> That is luck, not the document working.

**Root cause.** The generator has nothing to generate from for a foreign type,
and its own comment argued against a hand-copied list on the grounds that it is
"the one thing that could go stale here without CI noticing". That reasoning was
right about the list and wrong about the conclusion.

**Fix.** The entry is an **example** instead of a list:
`crates/jidousha/examples/vec2_tour.rs` is every `Vec2` operation a game reaches
for, written as assertions, embedded verbatim in the document the way the
Quickstart is. Cargo compiles it and `tools/test` runs it, so it cannot drift
from what the type offers. `TOURS` in `tools/gen-api-doc` is the mapping.

### F-019 — An engine example documents itself by reference to a game file

Class: docs · Run: 2 · Fixed in: `c4582fc`, corrected in this commit

**What happened.** `prototype_kit/verify.rs` carried:

> **A game does not do this.** `FrameRecorder::font_texture()` answers the
> question directly — see `pong/verify.rs`

> So an engine example documents itself by reference to a file in a *game* that
> an exercise like this one is expected to produce. It happened to stay true —
> I did use `font_texture()` — but it was true by luck, and I had deleted the
> file it names before I read the comment.

**Root cause.** A citation across a boundary that exists precisely so the two
sides can change independently. `pong/` is the artefact under measurement; every
E0 run is free to delete and rewrite it.

**Fix.** The six-line shape is written out in the comment, with a `DELIBERATE:`
tag saying why it is inlined rather than cited.

**The first version of that fix was worse than the bug**, and is corrected here.
It replaced the pointer at `pong/verify.rs` with a pointer at *this file* —
"(e0-findings.md F-019)", in an example, which is on the run's **allowed** list.
That is F-005's mistake one directory over: a citation of a document the reader
may not open is worse than silence, and this one named the root-caused ledger of
everything previous authors could not find. It also described "the E0 exercise"
to the person inside it. Both are gone; the `DELIBERATE:` tag now explains
itself without naming anything the reader cannot reach. Two `(ADR-0019)`
citations added to `examples/scripted_player.rs` at the same time went with it.
Caught by grep before run 3, not by a run.

**The wider class is recorded rather than fixed.** `crates/jidousha/examples/`
still carries fifteen citations of `ADR-00NN` and `core.md §N` — in `homing.rs`,
`sprites.rs`, `input_echo.rs`, `window_clear.rs`, `headless_sim.rs` and
`prototype_kit/` — every one of them naming a document the run may not open.
`docs/api/` has had a gate for this since F-005 (`scrub_internal_references`
plus the `FORBIDDEN` list, checked on the generated text); `examples/`, the
*other* allowed source, has never had one. The severity is lower — those point
at design rationale, not at an answer key — and stripping them costs a human
reader real context, so it is a judgement call rather than a defect: either
scrub them and lose the rationale, or keep them and accept that an E0 author
reads "see ADR-0009" and cannot. A `check-api-coverage`-style gate would at
least stop the set growing. Not blocking run 3.

### F-020 — The harness leaves the previous run's work where the next run will read it

Class: author · Run: 2 · Fixed in: `c4582fc` (notes), this commit (game)

**What happened.** The run reported it itself, unprompted, under "Contamination,
stated plainly":

> Run 1's notes are in this file, and this file is the one I was told to write
> into, so I read them before writing a line of Pong. That means I knew — before
> opening the API document — that the timestep is 1/60, that `Key::ArrowUp`
> exists, and that `ctx.rect` takes `(rect, color, depth)`.

**Root cause.** `e0-prompt.md` named a single file at the repository root,
`E0-NOTES.md`, and run 2 was pointed at the file run 1 had already filled.

**Why the conclusions still stand.** All three facts are now genuinely in the
document, and the run says where the document told it each one. But "run 2
guessed at nothing" is weaker evidence when the run was handed three of the
answers, and the difference is not recoverable after the fact.

**The same root cause has a second, larger instance, and the run reported that
one too**, in the same breath and without being asked:

> A fresh Pong, written from nothing: the previous run's `pong/` was deleted
> without being opened.

That was the author's own judgement. **The prompt never told them to**, and it
could not have: `crates/jidousha/examples/` is on the *allowed* list, and run
1's finished, working, verified Pong was sitting in it. A complete worked
solution to the exact task, inside the one directory the author is pointed at,
which a run reaching for a worked example would find first. Run 2 chose not to
open it. Nothing but that choice was protecting the measurement.

**Fix, both halves.** `E0-NOTES.md` is split into `docs/e0/run-1.md` and
`docs/e0/run-2.md`; the prompt says to write `docs/e0/run-N.md`, to choose the
lowest unused N, and lists `docs/e0/` among the directories it may not read.
And *Before starting a run* now deletes the previous run's `pong/` on the
attempt branch, together with its two `tools/test` registrations, so the next
author cannot read what is not there. The game stays in the default branch's
history for diffing, and step 6 puts the registrations back when the new game
lands.

Classified `author` because nothing about the engine or its document caused
it — the harness did. **It is the only class of finding that invalidates
evidence rather than costing time**, which is why both halves are closed before
run 3 rather than after it.

### F-021 — The document never says which types are resources

Class: docs · Run: 2 · Fixed in: this commit

**What the run did.** Wanted to set the camera, and had `Camera` documented as a
struct with four fields and six methods.

**What happened.**

> Nothing anywhere says how a game *sets* one. The answer is
> `world.insert_resource(Camera { .. })` in a `Startup` system, which I got from
> `window_clear.rs`. Nothing says whether a default camera exists if you never
> insert one, either — the `Default` line implies one could, but "the engine
> installs it for you" and "you must install it" are very different, and only
> one of them is true.

`Time` had the same problem. `Rng` said "held as a world resource" in its
summary and so was the only one of the three that did not.

**Root cause.** "Is a resource" is a fact about a type that lives in
`impl Resource for Camera {}` — a line the generator does not read and would not
know what to do with — and no doc comment carried it. The three types
disagreeing was chance: whoever wrote `Rng`'s summary happened to mention it.

**Fix.** Two halves. Every engine-provided resource's summary line now says it
is one and who installs it — `Time`, `Rng`, `Input`, `Camera`, `Assets` — so the
fact reaches the reference entry. And Concepts gains a **resources** section
with a table of the five: who inserts each, and whether it can be absent. The
two that can be absent, `Input` and `Assets`, are the two `find_resource` exists
for, which is the question the run guarded against without being able to check.

### F-022 — The documented `main` throws away the good error message

Class: docs · Run: 2 · Fixed in: this commit

**What happened.** With no display, the game printed:

```
Error: NoDisplay { detail: "os error at /root/.cargo/registry/src/index.crates.io-…/winit-0.30.13/src/platform_impl/linux/mod.rs:765: neither WAYLAND_DISPLAY nor WAYLAND_SOCKET nor DISPLAY is set." }
```

> The *content* is excellent, and worth saying plainly against run 1's
> postscript, which found the equivalent message pointing squarely the wrong
> way: this one is accurate, it names the real cause, and it is the right
> variant. […] But that is the `Debug` form.

**Root cause.** `fn main() -> Result<(), RunError>` is what the Quickstart shows
and every example copies, and Rust prints `Debug` for a `Result`-returning
`main`. `RunError` implements `Display` and its `Display` is the engine's
`message(what, specifics, likely_cause, fix)` house style — the whole of which
the documented shape discards, in favour of a struct dump carrying a vendored
dependency path and a line number from inside it.

**Fix.** The Quickstart returns `ExitCode` and matches, printing `Display` on
the error path, with a comment saying why; `input_echo`, `sprites` and
`prototype_kit` follow. `RunError` moves to `EXEMPT` in `check-api-coverage`,
since a game now matches it out of `run` without writing the type. The
`NoDisplay` fix text also loses its `(core.md §8)` citation, which the Quickstart
change would otherwise have started printing at a reader forbidden to open it —
the F-005 mistake, one layer down.

**`pong/` is deliberately left on the old shape.** It is the artefact under
measurement and changing it invalidates the comparison with run 1.

### F-023 — Query shapes are shown, never stated

Class: docs · Run: 2 · Fixed in: this commit

**What happened.**

> `World::query<'w, Q: ReadOnlyQuery<'w>>` tells me there is a trait called
> `ReadOnlyQuery`. It does not tell me what may implement it. […] I do not know
> the maximum arity, or whether a 1-tuple works.

> I structured the whole game around 2-component queries […] so that I would
> never find out. It happens to be the better design, and I would defend it on
> the merits now. But I did not choose it on the merits; I chose it because I
> could not tell what would compile.

**Root cause.** `Query`'s doc comment *did* state the answer — "Implemented for
`&T`, `&mut T`, `With`, `Without`, and tuples of up to six of those" — in its
**second** sentence, and the reference carries first sentences only. `With`'s
doc comment carried a worked example showing tuple placement, and the reference
carries declarations, not examples. Both facts existed and neither reached the
page.

**Fix, and it is mostly in Concepts rather than in the reference**, because
`Query` and `ReadOnlyQuery` are named by `World::query`'s bound and are *not*
facade exports — so no amount of doc-comment rewriting puts them on the page
(they are on F-017's list for that reason). Concepts now states the parts, the
arity, that the one-tuple works, and that the iterator prepends `Entity`, and
shows all four shapes as loops, which is the form the answer is needed in. What
does reach the reference is `With` and `Without` saying they yield `()` in their
tuple position; their doc comments and the two traits' are corrected for a
rustdoc reader either way, and `Without` gains the worked example `With` already
had.

### F-024 — `Rect::overlaps` does not say whether a shared edge counts

Class: docs · Run: 2 · Fixed in: this commit

**What the run did.** Parked the ball flush against a paddle face after a
bounce, and could not tell whether the next tick would re-trigger the bounce.

> I made the question moot by *also* requiring the ball to be travelling toward
> the paddle — but I wrote that guard because I did not know the answer, not
> because I had reasoned it was needed. The `contains` entry says it counts the
> top-left edges and not the others; `overlaps` says nothing.

**Root cause.** `contains` documents its half-open convention and `overlaps`,
written beside it, did not — even though the implementation is strict on all
four sides and a test named `edge to edge is not overlap` already pinned it.

**Fix.** The summary says "touching edges do not count", and the body says what
that buys: the same test cannot fire twice on a body parked against the face it
just hit, which is exactly the case the run was guarding.

### F-025 — `TextStyle::width_of` on multi-line text is unstated, and silent when it overruns

Class: docs · Run: 2 · Fixed in: this commit

**What happened.** Two halves of one entry. The run avoided multi-line text
because nothing said what `width_of` returns for it:

> `prototype_kit` passes multi-line strings to `ctx.text`, so the first half
> evidently works; the second half decides whether centring a two-line banner is
> possible. I avoided it entirely and drew two separately-centred `ctx.text`
> calls instead.

And then shipped a single-line banner 43.5 world units wide onto a 35.6-unit
screen, clipped at both ends, with eight assertions passing.

**Root cause.** "Multi-line text reports its widest line" was the *fourth*
paragraph of the doc comment and the reference carries first sentences. The
silence is worse: `width_of` is exact, centring by it is the documented idiom,
and nothing anywhere warns that the result can be wider than the camera.

**Fix.** The summary states the widest-line rule; the body says `\n` starts a
line, that centring by it is silent, and points at `Camera::visible_bounds`.
F-029 is the assertion.

### F-026 — Nobody says who owns the camera's viewport in a headless run

Class: docs · Run: 2 · Fixed in: this commit

**What happened.**

> `FrameRecorder::new(viewport)` takes a `PhysicalSize`. The `Camera` resource
> *also* has a `viewport` field, defaulting to 1280×720. Does the recorder
> override the camera's, or are the two independent […]?

The run made the question moot by passing 1280×720 to both, and said why it
mattered: its most valuable assertion reads the rectangle from the `Camera`
resource and the quads from the recorder, so a disagreement would leave it
"quietly comparing against the wrong rectangle" and passing.

**Root cause.** The recorder does override — `FrameRecorder::draw` builds
`Camera { viewport: self.viewport, ..the game's camera }` — and nothing writes
that viewport back into the world, so the two really can disagree. Neither the
method's doc comment nor `testing.md` said so. This is F-012's shape one layer
out: the windowed driver stamps the viewport every frame precisely because a
stale one is silent, and the headless path has the same trap with no stamp.

**Fix.** `FrameRecorder::new`'s summary says it overrides, and its body spells
out the failing assertion and the two ways to avoid it. `testing.md` says the
same where a test will read it.

### F-027 — Nothing says how a game exits

Class: docs · Run: 2 · Fixed in: this commit

**What happened.**

> There is no `App::quit`, nothing on `World` or `Commands`, and `run` is
> documented as "Run a game in a window, **forever**". I read the whole
> reference looking for it rather than guessing […] but the document does not
> say that either, and `Key::Escape` being listed invites you to look.

**Root cause.** A genuine v1 boundary that had never been written down anywhere
— not in `public-api.md`'s inventory, not in an ADR, not in the document. The
run was right, and spent a full read of the reference confirming it.

**Fix.** Concepts says it plainly, including why `Key::Escape` is listed, and
`public-api.md` §3 records it as a stated v1 exclusion so the next reader of the
inventory finds a decision rather than an absence.

### F-028 — There is no way to build one tick of input

Class: engine · Run: 2 · Fixed in: this commit

**What the run did.** Wrote a closed-loop player — one that watches the ball and
decides what to press — because the scripted session proves the controls and the
drawing and says nothing about whether the game is *playable*.

**What happened.** With `InputSnapshot::new()` meaning "the player did nothing"
and every other method a reader, the only route to a populated snapshot was a
script built in advance, so the check built a throwaway one-tick script every
tick:

```rust
InputScript::new().hold(Key::S, tick..tick + 1).snapshot_at(tick)
```

> It works, it is deterministic, and it is faintly absurd. An
> `InputSnapshot::with_keys(&[Key])` is the missing word.

**Root cause.** The gap is real. It is also worse than the run knew: that idiom
puts a **press edge on every tick**, because every tick is the start of its own
range. A game keyed on `just_pressed` would see the key tapped sixty times a
second; run 2's Pong happened not to read `just_pressed` on the key it drove.

**Fix, and it is not the one suggested.** `SnapshotBuilder` and `InputEvent`
already exist, are already the engine's single home for the edge rules, and are
what the windowed driver itself uses — they were simply not exported to
`jidousha::testing`. They are now. `with_keys` would have been a second way to
make a snapshot and would have had to answer the edge question a second time;
ADR-0019 records the decision and `snapshot.rs` carries the `DELIBERATE:` tag
beside the one already refusing `without_edges` for the same reason.
`examples/scripted_player.rs` now runs both shapes side by side, and asserts
that a controller which never presses a key produces no edge for it — the check
the one-tick-script idiom fails.

### F-029 — "Testing your game" omits the two assertions that pay for themselves

Class: docs · Run: 2 · Fixed in: this commit

**What the run did.** Spent its most expensive hours on a game that was not fun,
and found its worst bug by reading a frame transcript rather than by any
assertion.

**What happened.** Two lessons, both stated by the run as generalising past its
game:

> An assertion that says only "this is wrong" is nearly useless to an author who
> cannot look at the thing. It has to report the numbers it judged. That took me
> one wasted cycle to learn.

Its first failure message printed only the score; rewritten to include the
longest rally and the top ball speed — 14 touches, 25.6 units/s — the diagnosis
was immediate.

> **"Nothing is drawn outside `Camera::visible_bounds()`" is the single highest
> value assertion a shapes-and-text game can write**, and it is mentioned
> nowhere. It is six lines.

**Root cause.** `testing.md` covered the vocabulary — the recorder, `covering`,
the transcript — and nothing about what to assert with it. The transcript is
described as "good enough to check a layout by eye", which is true and is a
manual step; the assertion is the automatic one and was not there.

**Fix.** Both are in `testing.md`, the off-screen check written out in full with
the message that names the offending quad, the camera's extent, and centred text
as the usual culprit. Run 2 negative-tested that assertion by lengthening the
banner again, which is the evidence it is worth six lines.

### F-030 — What the font carries, and what an unknown character draws, is unstated

Class: docs · Run: 3 · Fixed in: this commit

**What the run did.** Wrote `"w / s to move · first to 5"` before it occurred to
it to check whether the font had a `·`, then found it could not check.

> **A non-ASCII character still submits a quad.** The `·` in my hint produced a
> glyph quad in the transcript, at the same advance width as every other
> character. So every check I have — glyph counts, "was text drawn", the
> off-screen bounds check, `width_of` centring — passes identically whether that
> quad draws a middle dot, a blank, or garbage.

It retreated to ASCII everywhere and left a comment saying why. Its own summary:
"the one piece of the game I cannot verify at all".

**The answer, since the run could not go and get it.** The atlas holds the
ninety-five printable ASCII characters, space (`0x20`) through `~` (`0x7e`), one
five-by-seven glyph each in a seven-by-nine cell. `cell_index` maps everything
else — every code point below space and every one above tilde alike — to a
ninety-sixth cell holding a **fallback box**, which is drawn at the same advance
as any other glyph. So the `·` drew a visible box. Not a blank, not garbage, and
not silence.

**The engine is right and stays as it is.** The fallback is deliberate, is
tested by name (`a_character_the_font_does_not_have_draws_the_fallback_box`),
and follows the same reasoning as the missing-texture placeholder: a character
that drew nothing would make "my score is half there" a mystery instead of a
picture. **No ADR**, because there is no oddity to explain — the loud fallback
is the obvious behaviour and the codebase already carries the argument for it
beside the art. What was missing was only that a game author could not find out.

**Root cause.** `renderer.md` §6 states the range and the fallback in one line.
That file is one a game author may not open, and nothing restated it on the
public side; `prototype_kit` draws `0x20`–`0x7e` and calls it "the whole
printable range", which is evidence in an example rather than a statement in the
reference. The run's own diagnosis is the right one: the observable effect is
identical either way, so no assertion could have closed the gap that a sentence
closes.

**Fix.** `TextStyle`'s summary — the sentence the reference carries — now names
the range and says an out-of-range character draws a visible box rather than
being skipped.

### F-031 — The character advance is exact, derivable, and never written down

Class: docs · Run: 3 · Fixed in: this commit

**What the run did.** Laid out the score, the banner and the hint by guessing,
running, and reading the numbers back out of the transcript.

> I could not tell whether a 38-character banner at size 1.4 would fit in a
> 35.5-unit-wide camera without building it and looking at the transcript. […]
> From the transcript the answer turns out to be `7/9 × size`. That is a fact I
> extracted from output rather than one I was told.

**The run's number is correct**, checked against the source rather than copied:
a cell is `CELL_W = 7` by `CELL_H = 9` texels and `TextStyle::advance` is
`size * CELL_W / CELL_H`, so every character advances exactly `size * 7 / 9`.
`text_advances_by_one_cell_per_character` pins it at size 9 → 7.

**Root cause.** `width_of` is documented as exact, which is what makes centring
work, and exactness after the fact is a different service from arithmetic
beforehand. A monospace font's whole advantage is that layout is multiplication,
and the multiplier was the one number not stated.

**Fix.** The ratio is in `TextStyle`'s summary, so it rides into the reference
beside `width_of`'s signature; `width_of`'s own doc comment carries the
`N * 7 / 9 * size` form for a reader of the source. Deliberately *not* in
`width_of`'s member summary, which the generator truncates at 68 characters.

### F-032 — The screens a run never reaches are the screens nothing checks

Class: docs · Run: 3 · Fixed in: this commit

**The sharpest finding of the run, and it is a sharper version of F-029.** The
off-screen assertion F-029 added was written early, ran 5,400 times, and passed
the whole run.

> Then I noticed that my longest string — `"the machine wins — space to play
> again"` — is **only drawn when the machine wins**, and my verify controller is
> a perfect tracker that wins 5–0 every time. […] At an estimated 41 world units
> against a 35.5-unit camera, it would have run off both edges on the first
> match a real person lost.
>
> the danger is not "text is silently too wide", it is "**the screen that is too
> wide is the one your test never reaches**".

**Root cause.** F-029 put the right assertion in the document and said nothing
about its domain. An assertion over drawn frames judges only the frames a run
draws, and a controller good enough to finish a game is a controller that never
loses it — so the strings on the losing, timeout and paused screens are exactly
the ones no frame ever carried. This is not specific to text or to Pong; it is
what "assert on what was drawn" means when the run picks what gets drawn.

**Fix.** `testing.md`, immediately after the `width_of` paragraph: build the
unreached screens by hand — one tick so `Startup` has run, set the resource that
selects the screen, draw one frame, run the same check — with the three lines
written out. The run's own fix is the idiom, generalised; it is written out
rather than cited, because the file it lives in is deleted before the next run
starts (F-019).

### F-033 — The closed-loop snippet reads a world that is not there yet

Class: docs · Run: 3 · Fixed in: this commit

**What happened.** The run panicked on tick 1 of its first verify run, reading
`world.resource::<Scoreboard>()` at the top of its controller loop.

> the document told me, and I still did it. The reason is that the document
> frames the fact around *arranging a test's starting state* […] and the case
> that actually bites is different — it is the **closed-loop controller**, the
> exact shape the document recommends two pages later.

**Root cause, and why this is a `docs` finding and not an `author` one.** The
fact is stated in Concepts (F-007's fix) and it is stated in the register the
reader is in when *setting up* a sim. The `SnapshotBuilder` snippet in *Testing
your game* is the one worked example of the shape that trips on it, its very
first line is `let want = /* look at the world, then decide */`, and it does not
repeat the warning. Run 1 hit the same empty-world-on-tick-1 in a rally harness
before F-007 existed; run 3 hit it after, having read the sentence. Two runs
into the same hole through two different doors is the document's problem.

**Fix.** A paragraph under the snippet: `Startup` runs inside the first
`tick()`, so the controller's read happens once against an empty world —
`find_resource` rather than `resource`, and a query that yields nothing rather
than a `[0]` into an empty `Vec`.

**The panic message is not at fault and is worth recording as a thing that
worked.** The run: "excellent — it named the resource, said resources are
inserted explicitly, gave the likely cause and gave two fixes, one of which
(`find_resource`) was the correct one. Cost: about two minutes."

### F-034 — Nothing says collision is only tested at tick boundaries

Class: docs · Run: 3 · Fixed in: this commit

**What the run did.** Worried its way to the answer rather than reading it.

> nothing in the API sweeps, so a ball that moves further in one tick than a
> paddle is thick passes straight through it. […] A line under `Rect::overlaps`
> saying "collision is tested at tick boundaries; a mover faster than its target
> is thick will step through" would have saved me working it out.

Corroborated by run 1, which went looking for an overlap test, found none, and
noted: "I ended up needing a *swept* test anyway, which no engine helper would
have given me."

**Root cause.** `Rect::overlaps` answers a question about two rectangles, and
tunnelling is a question about a fixed timestep — the reference documents the
first and Concepts documented the timestep without ever drawing the consequence.
Both runs reached it, and both reached it by reasoning rather than by reading.

**Fix.** Concepts, in the fixed-timestep paragraph rather than on `Rect`,
because that is where the cause is: collisions are tested at tick boundaries,
nothing in v1 sweeps, and the fix is the game's — keep `speed * Time::fixed_dt`
under the thinnest thing it must not miss. The paragraph names `Rect::overlaps`
so the reader who searches for it lands here. **`Rect::overlaps`'s own summary
is unchanged**: it is already 66 of the 68 characters the generator gives a
member line, and a truncated warning is worse than none.

**Run 3's handling was better than the line it asked for**, and the fix says so:
it asserted the margin against the `Time::fixed_dt` the engine handed the game,
which catches a raised `GameConfig::fixed_dt` that a comment would not.

### F-035 — Two worked examples disagree on per-tick versus per-second, and one is the Quickstart

Class: docs · Run: 3 · Fixed in: this commit

**What the run did.** Made an engine convention up, correctly, and said so.

> `prototype_kit` writes its paddle speed as "world units per tick"
> (`speed: 0.25`) and does not touch `fixed_dt`. `scripted_player.rs` writes
> per-second constants and multiplies by `fixed_dt`. Both are in the examples
> directory, and they are opposite conventions. I went with per-second-and-
> multiply, because it survives a change to `GameConfig::fixed_dt`, but the
> document has no opinion and the two worked examples disagree.

**Wider than the run could see.** The Quickstart is the third example and it was
on the per-tick side — `const SPEED: f32 = 0.35;`, "how far the player moves in
one tick" — which makes the disagreement the *first* thing a copy-and-change
author inherits. Run 3 could read that, of course, but it had already reached
the right answer from the other two and did not go back.

**Root cause.** `conventions.md` rules on units in types, on angles and on
durations, and had nothing to say about rates. With no ruling, three examples
and no reason to agree, they did not, and "one way to do everything" was
enforced everywhere except in the thing every game has.

**Fix.** The ruling is in `conventions.md` under Time — speeds and rates are per
second and multiplied by `Time::fixed_dt` where they are applied; counting ticks
stays ticks, because the tick is the canonical timeline — and it reaches the API
document through the conventions digest. `quickstart.rs` and
`prototype_kit/main.rs` are converted; `prototype_kit`'s paddle is 15.0 units a
second where it was 0.25 a tick, which is the same speed at 1/60 and now stays
that speed if the timestep moves. The Quickstart gains one line and a
`Time::fixed_dt` read, which is the first place a game author sees the shape.

### F-036 — `Batch` is a type a worked example reads and the document does not define

Class: docs · Run: 3 · Fixed in: this commit · **Second instance of F-017's class**

**What the run did.** Read `prototype_kit/verify.rs`, saw `plan.batches`,
`batch.texture` and `batch.quad_count()`, and went looking.

> `Batch` has no entry in the API document at all — `FramePlan` names the field
> as `Vec<Batch>` and the type is never described. I did not need it (I used
> `FrameRecorder`, which is the shape the document recommends for a game), but
> if I had followed the worked example instead of the prose I would have been
> writing against an undocumented type.

**Root cause.** Exactly what F-017 predicted and left unbuilt: a type named by a
signature — here a public field — with no entry of its own, because the facade
is a curation and `Batch` was curated out while `FramePlan::batches` stayed in.
F-017 wrote down that a generator gate for this class was "the obvious next
piece of work" and that each remaining candidate needed a decision. This is the
decision for one of them, arrived at the way F-017 said the next one would be:
by a run reporting it.

**Fix.** `Batch` and `QuadVertex` are exported in `jidousha::testing` — the
second because exporting the first names it, and a fix that moves the hole one
type along is not a fix. `FrameRecorder` remains the road a game should take and
the comment beside the export says so.

**The gate is still not built**, and it is now overdue rather than obvious: two
runs have found this class by hand. The remaining candidates are the ones F-017
listed — `ByteSource`, `AssetHandle`, `AssetKind`, `Phase`, `IntoSystem`, the
query traits, `CommandKind` — and the work is a decision each, not a mechanism.

### F-037 — There is no worked controller that plays to *win*, and a tracker makes a game unwinnable

Class: docs · Run: 3 · Fixed in: this commit · **Found independently by all three runs**

**The most-corroborated finding in this file.** Every run has walked into it,
each without knowing the others had:

> **Run 1.** "a player that tracks the ball exactly meets it with the middle of
> the paddle every time, and a centre hit returns the ball dead flat, so two
> exact trackers rally forever at a fixed height. The degenerate equilibrium is
> an artifact of the perfect tracker, and it hid the real question."
>
> **Run 2.** "**First run: 0–0 after a hundred simulated seconds.** Not a crash
> — an *unloseable* rally. […] The bounce model has a fixed point at 'hit it in
> the middle' and two perfect trackers sit down in it."
>
> **Run 3.** "**0–0 after 90 seconds, one rally of 78 touches.** […] a closed-
> loop test controller that plays *safe* is not a playability test. A tracker
> that centres every return proves the controls work and simultaneously proves
> the game cannot be won, because it has made the game degenerate."

Run 3 spent two full tuning attempts on it and named the cost:
"`scripted_player.rs`'s closed-loop example chases a target, which is tracking;
there is no worked example of a controller that plays to *win*."

**Root cause.** ADR-0019 and `testing.md` distinguish a *script* from a
*controller* — "a blind script never returns a ball" — and stop there. The
second step, that a controller can be closed-loop and still measure nothing, has
never been written anywhere. Three runs found it by losing hours to it.

**Fix, and what was deliberately not done.** The lesson is in `testing.md` under
the `SnapshotBuilder` snippet, stated as the shape rather than as Pong: a
controller that plays safe measures its own caution; play to win, aim the return
away from the opponent, take the shot a person would take — with the driver that
brakes for every corner and the fighter that blocks everything as the same trap
in other clothes.

**The worked example is declined, and the reason is F-020.** A worked
closed-loop controller that plays a game to win is a worked *game* — the trap
only exists where a symmetric return angle has a fixed point, which is to say in
Pong or something isomorphic to it. `crates/jidousha/examples/` is on E0's
allowed list, so shipping one there would hand run 4 the answer to the exercise,
which is the precise failure F-020 exists to prevent and the reason each run's
game is deleted before the next starts. Prose that every run reads is the lever
available; if run 4 walks into this a fourth time, the next move is a worked
example of the *shape* in a game deliberately unlike Pong, not a Pong.

### F-038 — `Rng::below` is documented, and clippy pushes the other way

Class: author · Run: 3 · Fixed in: nothing to fix

**What the run did.** Wrote `next_u32() % 2 == 0` for a coin flip, had clippy
reject it as `manual_is_multiple_of`, and took the lint's suggested
`is_multiple_of(2)` before recognising the engine's own answer.

> The right answer was `Rng::below(2)`, which is documented and which I had read
> past. The lint pushed me toward `is_multiple_of(2)` — the engine's own "one
> way to do everything" answer was the better one and the lint does not know
> about it.

**Where the answer already was**, per §1's rule for an `author` finding —
`docs/api/jidousha-api.md`, `Rng`:

> `pub fn below(&mut self, limit: u32) -> u32;  // A value in `0..limit`, with
> every value equally likely`

**Nothing to fix.** The run diagnosed it correctly and unprompted, cost itself a
lint cycle, and reached the documented answer. A lint that does not know a
project's vocabulary is not a docs gap; the entry is one line above the method
the run reached for instead. Recorded because §1 says three `author` findings on
one topic is a `docs` finding wearing a hat, and this is the first on this one.

## 5. Notes on the run's procedure

Two things about run 1 that are not findings but would confuse a later reader.

**The game was registered with `tools/test` before the run passed.** Commit
`6626b9c` added `pong` to `VERIFIABLE_EXAMPLES` (`tools/test:118`).
`e0-prompt.md` step 6 made that the maintainer's step "on the run that
passes". Harmless — the game verifies green — but the registration is not
evidence of a pass, and the milestone is not ticked.

**Settled before run 3: register after every run, not only the passing one.**
Run 1 was right to register immediately — a game nobody verifies is a game that
rots between runs — and the old wording was simply written before anyone had run
E0. The registration now comes out with the game at the start of each attempt
and goes back in when the game lands, both maintainer steps, neither touching
the author. It has to come out with it: a registered example that does not exist
fails `test_the_windowed_list_names_examples_that_exist`, which exists to catch
exactly the stale name this would otherwise leave behind.

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

## 6. Where the runs stand

### What run 2 answered about run 1's fixes

- **The reference is callable.** F-001's open question was whether signatures
  were enough, and run 2 says yes for anything that is a function call: "I
  checked argument orders against it, not against examples, for the entire
  drawing vocabulary." The game compiled first time. The document went from
  ~4,100 tokens to ~13,800 doing it, and the run reported no trouble navigating
  the result, so the size worry in the old version of this section did not
  materialise.
- **Every specific fix got used.** `Key`'s variant list, `GameConfig`'s fields
  and default, the 1/60 timestep stated in ticks-per-second terms,
  `Rect::overlaps`, `TextStyle`'s own `depth` (ADR-0018) and
  `FrameRecorder::font_texture()` are each named in run 2's notes as having
  closed a specific run-1 finding. **These are the things not to regress.**
- **`FrameRecorder` was reached for**, which was the open question about F-010.
  The run used it throughout and called it "the right shape"; `covering(point)`
  is what made every drawing assertion a two-liner. The long form in
  `prototype_kit` was not copied — but the comment steering away from it pointed
  at a game file, which is F-019.
- **F-011 is still unverified.** No display and no adapter in run 2's container
  either. It needs the reporter's hardware.
- **The open third of `public-api.md` §4 — signatures without per-item
  examples — is answered for now.** Run 2 read declarations and called them
  correctly, and did not go to `examples/` to see calls being made. Where it did
  reach for an example it was for a *shape* rather than a signature: how a game
  sets a camera (F-021), which is a Concepts question, not a per-item one.

### What run 3 answered about run 2's fixes

Run 3 is the first run whose answers are uncontaminated (F-020), so this is the
first clean report card on anything.

- **"Which of these is a resource" is closed.** F-021's fix is named in run 3's
  short list of what the document does unusually well: "**The resource-
  availability table.** Which resources exist, who inserts them, and which three
  can be absent […] I used `find_resource` in exactly the right two places on
  the first try because of that table." The fix was in the right place.
- **The closed-loop route is found.** ADR-0019's `SnapshotBuilder` was reached
  for and worked first time; `InputScript` was read, understood, and
  deliberately not used. No one-tick scripts. Closed.
- **The off-screen assertion gets written.** F-029's check was written early and
  ran the whole session — and F-032 is the thing it could not catch, which is a
  new finding rather than a failed fix. The other half of F-029 landed harder
  than expected: "every failure in my verify run prints its numbers, and during
  the tuning fight those numbers were the *entire* diagnosis."
- **F-016's class is gone; F-017's is not.** No generator dropout was reported.
  `Batch` is a second instance of a type named by a signature with no entry
  (F-036), which is the second run to find that class by hand — the gate F-017
  deferred is now overdue rather than optional.
- **The engine's messages keep earning their length.** Both failures the run
  caused — the missing-resource panic and `RunError::NoDisplay` — were acted on
  without investigation. "I acted on both without investigating anything."
- **F-011 is still unverified by a run.** No display and no adapter in run 3's
  container either, for the third time. What is new is that the game was played
  by a person afterwards and reported fine, so the *game* is confirmed and the
  *finding* is not.

### What run 4 should be watched for

- **Whether the third kind of gap is closed, or just this instance of it.** Run
  3's findings are all "the document does not say what this does". F-030,
  F-031 and F-034 each add one sentence about behaviour that was already
  correct. If run 4 produces a fourth batch of the same shape about three
  different behaviours, the problem is not the sentences — it is that nothing in
  the pipeline asks "does the reference state what this *does*, not just what it
  *is*".
- **Whether F-037 needs a worked example after all.** Three runs have made a
  game unwinnable with a perfect tracker. The fix is prose, and the reason it is
  only prose is written down in F-037. A fourth run that walks into it is the
  evidence that prose is not enough, and the answer then is a worked controller
  in a game deliberately unlike Pong.
- **Whether the font sentence is enough.** F-030's fix is one clause in
  `TextStyle`'s summary. The test is whether run 4 uses a non-ASCII character
  *on purpose*, or avoids the question the way run 3 did — and if it uses one
  and is wrong about what it drew, the clause is in the wrong place.
- **Whether the unreached-screens check gets written.** F-032 is the sharpest
  thing run 3 found and the fix is three lines of prose in `testing.md`. A run 4
  whose banner overruns on a screen its controller never reaches is the same bug
  twice, and the third time this class has cost a run.
- **Whether the rates ruling holds.** F-035 converted the Quickstart and
  `prototype_kit`. Every example now says per-second; if run 4 writes per-tick
  constants anyway, the ruling is in `conventions.md` and not where a game
  author reads.
- **Whether anything in these fixes reads as an invitation to guess.** Same
  standard as before: a fix is only real if the next run does not have to infer
  the thing it fixed.

## 7. What this file feeds

The `make-game` skill (agent-practices §3) is written from E0's findings after
it passes. A friction that recurs across runs and cannot be designed away is
exactly what a skill is for — and one that *was* designed away must not appear
in the skill at all, because a skill that restates a fixed problem is how the
fix gets undone later.

On the evidence of run 1, most of this file must **not** reach the skill: F-001
through F-009 are all "the document did not say", and a skill that teaches an
agent to work around a thin reference is a skill that removes the pressure to
thicken it. Run 2 says the same about F-016 and F-021 — a skill listing the
engine's resources would be a skill papering over a document that does not.

**Run 3 adds two more of the same kind, and they are the strongest candidates in
the file.** F-032 — the screens a run never reaches are the screens nothing
checks — and F-037 — a controller that plays safe measures its own caution, not
the game — are both about writing a test for something you cannot look at, and
neither mentions this engine. F-037 in particular is the only finding here that
three independent runs reached on their own, which is the definition of a
friction that cannot be designed away. Both are in `testing.md` because run 4
needs them; both belong in the skill for the same reason F-029's pair does.

**Two of run 2's findings are skill material, and they are the two that are not
about this engine at all.** F-029's pair — that a failing assertion has to
report the numbers it judged, and that "nothing is drawn outside the camera" is
the first assertion a shapes-and-text game should write — generalise to any
game an agent writes without being able to look at it. They are in `testing.md`
because run 3 needs them; they belong in the skill because every run after that
does too.
