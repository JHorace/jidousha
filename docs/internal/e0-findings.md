# E0 findings — what building a game with this engine actually cost

Status: **six runs, seventy-nine findings, awaiting run 7.** The
harness is `docs/internal/e0-prompt.md`; the milestone is implementation-plan.md
§3. The bar is two consecutive runs with no new `engine` or `docs` findings.
Run 6 answered run 5 and then found eleven more of its own, so the count of
consecutive clean runs is still zero. Run 1 found five `engine` findings, run 2
three, run 3 none, run 4 three (all decided: ADRs 0021–0023), run 5 one, declined
(ADR-0024), and **run 6 one, accepted and fixed** — `Radians::from_degrees` is a
`const fn`, so an angle constant can be written in degrees (F-069). Nine of run
6's eleven are sentences the document does not carry, which is the fifth
consecutive run whose findings are mostly that shape.

**Run 6 is the first run that could look at its own game, and the first to be
wrong about nothing.** Every claim in `docs/e0/run-6.md` was checked against the
source during triage and every one held — which has not happened before, and is
worth recording as a property of the *log* rather than of the engine: the run
marked which of its findings were measured and which were inferred from document
text it could not check, and the inferences were sound.

**Its headline finding is a sentence that was false**, not one that was missing:
*Testing your game* told a game author the clear colour could not be asserted, and
it is one line. The run believed it, wrote no check, and that was the one fault of
seventeen its mutation round missed (F-068). This is the second such finding in
two runs — F-055 was the first — and both were invisible to every gate this
pipeline has.

**The other cost worth carrying up is that the controller warning cut the wrong
way.** Run 6 hit the exact symptom four previous runs hit, with a controller that
was correct and a game that was not (F-074). What resolved it in one step was the
contract check F-056 asks for, which run 6 had written. **So F-056's fourth
attempt worked, and the lever run 4 named — a worked controller in a game unlike
Pong — stays unspent.**

**Run 4's triage is §4a, run 5's is §4b and run 6's is §4c**, each the whole run
on one page with the class, the cross-run corroboration and the settling ADR or
`DELIBERATE:` tag for every finding, plus a plain statement of what the triage
could not settle.

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
| `environment` | The harness could not run the thing it was measuring. | An escalation, per CLAUDE.md's never-agent-fixable list. |

**`environment` is new in run 4** and it exists because four consecutive runs hit
one friction that is none of the other three: no display and no GPU driver, so no
run has ever seen its own game or rendered a pixel (F-054). Calling that an
`engine` finding would point the fix at code that is already correct, and calling
it an `author` finding would be absurd. A taxonomy with no slot for "the
measurement could not be taken" is a taxonomy that files that as nobody's problem,
which is how it survived three runs unrecorded.

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
| 4 | 2026-08-18 | Pong shipped; **not** a pass | 3 | 11 | 1 (+1 `environment`) | Compiled clean, `--verify` green. One full debug cycle lost to `ctx.circle`, six tuning runs lost to its own verify controller. Three `engine` findings, decided in ADRs 0021–0023 (two applied, one declined with the boundary documented), plus one environment escalation (F-054). Raw notes: `docs/e0/run-4.md`. Triage: §4a. §6. |
| 5 | 2026-08-19 | Pong shipped; **not** a pass | 1 | 7 | 2 (+1 `environment`) | Compiled clean, `--verify` green, 1,263 frames recorded. Two cycles lost to a controller that optimised onto the boundary of what its paddle could reach (F-056) — the fourth run to be sent into its game's constants by its own driver, and the first that had *read* the warning. The one `engine` finding is **declined**: ADR-0024 says draw order was always observable and a `Depth` on `DrawnQuad` would not have caught the bug it was wanted for. Raw notes: `docs/e0/run-5.md`. Triage: §4b. §6. |
| 6 | 2026-08-19 | Pong shipped; **not** a pass | 1 | 9 | 1 (+1 `environment`) | Compiled clean, `--verify` green, 2,013 ticks, and **the first run to see a frame of its own game** — the capture path landed and the PNG looks like Pong. Blocked once, on its own game design: an opponent unbeatable by arithmetic (F-074, the other side of F-064). The one `engine` finding is **accepted and fixed** — `Radians::from_degrees` is now a `const fn` (F-069). Its headline is a document sentence that was *false* rather than missing (F-068), the second run running. Every claim in its log held under checking. Raw notes: `docs/e0/run-6.md`. Triage: §4c. §6. |

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

**Run 5 was valid, and its game was played.** The maintainer checked the
transcript for reads under the restricted paths and found none, and ran the game
in a window and in a browser — both after-the-run steps 1 and 2, taken before the
decks were cleared for run 6. Worth recording that the order matters: clearing the
decks deletes `crates/jidousha/examples/pong/`, so a playtest deferred past that
point is a playtest that cannot happen. Run 5's own §5 lists five things it wanted
to look up in `src/` and did not, which is the same shape of evidence run 3
offered, and its `--verify` transcript is the artifact for everything else.

## 4. Findings

Fifteen findings from run 1 (F-001–F-015), fourteen from run 2 (F-016–F-029),
nine from run 3 (F-030–F-038), sixteen from run 4 (F-039–F-054, triaged together
in §4a) and eleven from run 5 (F-055–F-065, §4b). F-066 and F-067 come from **no
run at all** — a maintainer session between runs 5 and 6, closing the last item
F-054 named — and §2's counter does not move for them; see the note above them. F-001 is the parent of most of run 1's
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


---

## 4a. Run 4 triage — the whole run on one page

Sixteen findings, in the order run 4's cost ranks them. **Class** is §1's;
**settled by** names the ADR or `DELIBERATE:` tag that already answers the
complaint, where one does — six of these are things the engine gets right on
purpose and could not say so.

| # | Finding | Class | Also found by | Settled by | Verdict |
|---|---|---|---|---|---|
| F-039 | `ctx.circle` is sixteen quads | docs | **run 3, wrongly** | `DELIBERATE:` at `shapes.rs`'s `CIRCLE_SEGMENTS` | doc fix landed; ADR-0020 records the choice |
| F-040 | `frames()` + `draw()` do not compose | docs | first | — | doc fix landed; **ADR-0023 accepted** — `draw` returns an owned frame |
| F-041 | no sweep, no `Rect::inflate` | engine | **runs 1, 3** | **ADR-0022** | **declined by decision**; the boundary is now documented |
| F-042 | `visible_bounds` returns a tuple | engine | first | **nothing — checked** | **ADR-0021 accepted**; returns `Rect`, plus `Rect::contains_rect` |
| F-043 | no vertical text metric | docs | first | — | doc fix landed; `height_of` declined |
| F-044 | unprintable chars invisible to assertions | docs | run 3 (other half) | fallback box is deliberate (F-030) | doc fix landed; `debug_assert` **declined**, reasons recorded |
| F-045 | `sin_cos` has two spellings | docs | first | — | doc fix landed; the *document* taught the wrong one |
| F-046 | `--verify` never mentioned | docs | first | — | doc fix landed, plus the verdict-line protocol |
| F-047 | a mediocre controller makes you retune | docs | **runs 1, 2, 3** | F-037 predicted this run | doc fix landed; worked example declined again |
| F-048 | `Time::alpha` has no consumer | docs | first | renderer.md §2 (no retained state) | doc fix landed |
| F-049 | `Depth::layer`'s numbering is imitation | docs | first | — | doc fix landed |
| F-050 | `Phase` looks taken and is not | docs | **3rd of F-017's class** | curation invariant, facade `INVARIANT` | doc fix landed; **not** exported |
| F-051 | `Seconds` has no multiplication | author | first | `as_f32`'s own entry | one sentence; the absence is correct |
| F-052 | no sound | engine, out of scope | first | ADR-0001 | nothing to fix; post-v1 list |
| F-053 | on screen is not in the right place | docs | all four, as a habit | — | doc fix landed |
| F-054 | four runs, no display, no pixel ever rendered | environment | **runs 1, 2, 3** | CLAUDE.md's escalation rule | **escalated, unresolved** — but CI already installs the rasterizer, so the fix is one apt line copied to the E0 image |

**The three proposals were accepted and are applied.** ADR-0020 through 0023 are
all `accepted`; the surface changes landed with the examples that prove them.

- **ADR-0021** — `Camera::visible_bounds` returns `Rect`, and `Rect` gains
  `contains_rect`, closed on all four sides where `contains` is half-open.
  `testing.md`'s off-screen check went six lines to three; `pong/verify.rs`'s
  `assert_on_screen` lost its four-comparison body and its tuple parameter.
- **ADR-0022** — accepted *as recommended*, which means the sweep and
  `Rect::inflate` are **declined**. Nothing was added; what changed is that
  Concepts now names the absence as a v1 boundary and gives the eight-line shape
  to write instead, which is the treatment `App::quit` gets and which run 4 called
  "the right way to document an absence".
- **ADR-0023** — `FrameRecorder::draw` returns an owned `FrameRecord`; `clear()`
  declined, so the frame history stays whole. `pong/verify.rs` lost an entire
  second `FrameRecorder` that existed only to work around the borrow.

`tools/verify pong` reports the same 2,598 ticks, the same 5–0 in 43.3s and the
same 101-quad final frame as before the changes, so none of this touched the game.

**What this triage still cannot settle, stated plainly.** Two things.

1. **F-054 I cannot resolve because this container has no display or GPU
   either.** So run 4's claims about how the game looks are still unchecked by
   anybody, and this note does not pretend otherwise.
2. **F-047's prediction is unfalsifiable until run 5.** Prose has now had two
   attempts at the controller trap and the second one failed differently from the
   first. Whether the third sentence works is a measurement, not an argument.

**Two things that were not findings and are corrections to this file.** Run 3's
§5.2 recorded a false answer about `ctx.circle` and it stood through a maintainer
pass — see F-039. And `pong` was left out of `WINDOWED_EXAMPLES` and
`VERIFIABLE_EXAMPLES` when run 4's game landed, so `tools/test` failed on the
windowed run for reasons nothing to do with the code; `e0-prompt.md` step 6 makes
re-registering it the maintainer's step and it was missed. Both are fixed in this
commit.

### F-039 — `ctx.circle` is sixteen quads, and the only worked assertion teaches the shape that cannot see it

Class: docs · Run: 4 · Fixed in: this commit · Settled by: `DELIBERATE:` at
`shapes.rs`'s `CIRCLE_SEGMENTS` (renderer.md §2, §9)

**The most expensive finding of the run**, by the run's own ranking, and the one
with the sharpest cross-run evidence in this file.

**What the run did.** Asserted that its ball was drawn, by copying the only
worked example of the question — `prototype_kit/verify.rs` checks a paddle by
looking for a quad *the size of the paddle* at the paddle's position.

**What happened.** It fails, because nothing the size of the ball is drawn
anywhere:

> what covers the ball's centre is sixteen wedges of 0.450×0.172, 0.416×0.318,
> 0.318×0.416, 0.172×0.450 and so on. I only found out by making the assertion
> dump what it had actually found, which is a full debug cycle spent on an
> undocumented implementation detail of the one primitive a ball is made of.

**The answer, since the run could not go and get it.** `CIRCLE_SEGMENTS` is
**32**, fixed, and each quad is the centre plus three rim points — two segments
per quad — so a circle is **sixteen quads at any radius**. The count carries a
`DELIBERATE:` tag: a radius-dependent count would make a transcript and every
golden image change when a circle grows by a pixel, and identical submissions
producing identical output is what the whole verification story rests on. So the
run's open question — "is that count fixed or does it scale with radius" — has
an answer, it is deliberate, and it was written down at the site four crates away
from anything a game may read.

Two facts follow that a check can lean on, and both were true of the run's own
`disc_drawn` without the run knowing why: every wedge is inscribed, so **nothing
a circle draws reaches outside `2r × 2r`**; and all sixteen share the centre as a
corner while the extreme rim points fall exactly on the axes, so **the union of
the quads covering the centre is exactly `2r × 2r`**. `FrameRecord::covering`
counts a point on an edge or corner as inside, which is what makes asking about
the centre return all sixteen.

**Run 3 asked the same question and wrote down the wrong answer.** This is the
part only a maintainer can see, and it is why this outranks its neighbours. Run 3
§5.2 lists "what `ctx.circle` expands into" among the three things it wanted to
grep for, and then closes it:

> I answered it from the transcript instead (the ball is one quad, exactly
> `2r × 2r`), which is a better answer than reading the code would have been,
> because it is the observed behaviour rather than the implementation.

Its game drew its ball with `ctx.circle` (`47a711e:pong/draw.rs:81`), so its
transcript carried sixteen wedges. **"The ball is one quad" is false**, it was
recorded as resolved, the maintainer's follow-up did not catch it, and nothing
failed — because run 3 never wrote a "a quad of this size is here" assertion for
its ball, only the off-screen bounds check, which the inscribed-wedge property
happens to make safe. So two consecutive runs went looking for this: one lost a
debug cycle and got it right, one lost nothing and got it wrong. A document
silent about behaviour does not only cost time; it produces confident false
findings, and this file recorded one for a whole run.

**Root cause, and why the sentence had nowhere to go.** `Submit::circle`'s doc
body already said "made of a fixed number of straight edges, so the same circle
always produces the same vertices" — and the generator carries **first sentences
only**, truncated at 68 characters for a member line, so the body reached no
reader. The summary was "Fill a circle." The one place the fact could have
reached a game author was the four words the reference prints, and they said
nothing.

**Fix — option (a), documented, not queryable.** ADR-0020 records the decision
and the rejected alternative.

- `circle`'s member summary is now "Fill a circle, as a fan of sixteen quads
  rather than as one", which is 60 of its 68 characters and is the fact.
- Concepts gains a paragraph on quad counts per verb — one for `rect` and
  `line`, sixteen for `circle`, one per character for `text` — with the budget
  consequence, because "a circle costs sixteen rectangles" is the sentence that
  makes a frame's quad count predictable. The run's own measurement:
  `101` quads in its last frame, `16` of them the ball.
- *Testing your game* carries the **worked disc assertion**, generalised from
  the run's `disc_drawn`: union the bounds of the quads covering the centre that
  fit inside `2r × 2r`, and check the union. Written out rather than cited,
  because `pong/verify.rs` is deleted before the next run starts (F-019).
- `DrawnQuad::contains` now says edges count, which is the second thing the run
  wanted to look up and did not.

**The game's workaround does not get simpler, and that is the honest cost of
option (a).** `disc_drawn` stays exactly as written; what changed is that it is
now the documented idiom rather than something an author had to invent. Making it
shorter is ADR-0020's rejected alternative and needs authority above this task.

### F-040 — The document's two recorder snippets are a borrow error together

Class: docs · Run: 4 · Fixed in: this commit · The recorder's *shape* was the
separate question and it is answered: **ADR-0023, accepted**

Classified `docs` and not `engine` under §1's one-class rule, because the run
needed no new API to get its check written — it needed the document not to teach a
composition that does not compile. What the shape would buy is real and is
ADR-0023's argument, not this finding's.

**The worst class of doc bug: a pattern that does not compile as taught.**

**What the run did.** Followed *Testing your game* twice. Once for "record every
tick, then inspect the last frame":

```rust
let frame = recorder.frames().last().expect("600 frames were drawn");
```

and once, a page later, for F-032's fix — "check the screens your run never
reaches" — which is another `recorder.draw(&mut sim)`.

**What happened.**

> Doing both is a borrow error — `frames()` holds the recorder immutably for as
> long as the frame reference lives, and `draw()` wants it mutably. I ended up
> doing *both* workarounds: `.cloned()` for the match's last frame, and a second
> `FrameRecorder` for the staged screens.

The second `FrameRecorder` is worse than it looks: it also moves what
`transcript()` prints, and the run walked into that first — it printed a
synthetic staged screen instead of the real last frame of the match, which is
the one artifact a run with no display has.

**Root cause.** Both snippets are correct in isolation and F-032's fix was
written into a file that already contained the other one, a page apart, with
nothing between them. The signatures make the conflict unavoidable —
`frames(&self) -> &[FrameRecord]` and `draw(&mut self) -> &FrameRecord` — and the
run needed exactly the composition the document recommends. Nobody composed them
because nobody wrote the two together; the file is prose and each paragraph was
reviewed against the one before it.

**Related, and the same design question.** The recorder retains every frame,
2,598 of them in this run to look at one, and has no `clear()` — though
`NullBackend`, the lower-level path the recorder was built to replace (F-010),
has one.

**Fix, docs half.**

- The first snippet now ends `.clone()`, with a paragraph naming the borrow rule,
  saying `draw`'s return value has it too, and saying to do it as a matter of
  course *because* the recommended shape needs both halves in one function.
- The same paragraph says to read `font_texture()` out before the loop, which is
  the third thing with this rule, and states the retention plainly: every frame,
  oldest first, no way to forget them.

**Fix, engine half: made. ADR-0023 accepted.** `FrameRecorder::draw` returns an
owned `FrameRecord`, so the two paragraphs compose without a workaround. `clear()`
was declined and retention stays whole — the frame history is what a failing
assertion reads backwards, and a check that could throw away the tick before the
one that broke would be throwing away the tick the failure message wants.

**The measurable result is that `pong/verify.rs` lost a whole `FrameRecorder`.**
The second one existed only so the staged screens could be drawn while a reference
into `frames()` was alive; they now go through the same recorder as the match, and
the three-line comment apologising for the clone is gone with it. The run's other
workaround — cloning the match's last frame out of `frames()` — became the loop
simply keeping the frame `draw` handed it.
`a_recorded_frame_outlives_the_next_draw` is the regression guard, written as the
two paragraphs of `testing.md` that used not to compile together.

### F-041 — Nothing sweeps, and the vocabulary stops one question short

Class: engine, out of v1 by decision · Run: 4 · **Third sighting** (runs 1, 3, 4)
· Settled by: **ADR-0022, accepted — the primitives are declined**

**What the run did.** Read Concepts' fixed-timestep paragraph — F-034's fix,
which names tick-boundary tunnelling as "the first thing that bites a game with a
fast small ball" and says the fix is the game's — and then went looking for
something to write that fix with.

**What happened.** The collision vocabulary is `Rect::overlaps` and
`Rect::contains`, and neither answers a ball against a paddle. It wrote a
plane-crossing test by hand (`advance` in `main.rs`) and predicted that every
Pong written against this engine writes the same forty lines. It also wanted
`Rect::inflate`, because "the paddle expanded by the ball's radius" is
`PADDLE_SIZE.y * 0.5 + BALL_RADIUS` spelled out at the call sites.

**Corroborated twice, and it is now the second-most-corroborated finding in this
file after F-037.** Run 1 went looking for an overlap test, found none, and
noted: "I ended up needing a *swept* test anyway, which no engine helper would
have given me." Run 3 §1.4 and §2.1: "the one thing I kept reaching for was a
swept or continuous collision helper […] it is the single piece of vocabulary a
Pong needs that shapes-and-text does not cover." F-034 answered the *warning*
and left the *primitive*, and the third run through named that gap directly.

**What the run's forty lines actually are, which matters to the decision.**
`advance` is not a generic sweep. Reading it: the plane crossing is about eight
lines, and the other thirty are the *response* — reflect off the paddle by where
along its face contact landed, gain speed, cap it, advance the remainder of the
tick from the contact point, then resolve the walls. A `Rect::sweep` would absorb
the eight and leave the thirty, because the response is the game's model of Pong
and no engine can own it. So the run's "every Pong writes that same forty lines"
is true and the engine could remove about a fifth of them. That is a real number
and it is smaller than the finding sounds; ADR-0022 argues from it rather than
from the sentence.

`Rect::inflate` is smaller still. Two of the three sites the run counted are the
paddle (`x` and `y`); the other two are the field walls, which are a different
rectangle. Inflate would replace two scalar expressions with a `Rect` the call
sites then destructure — roughly break-even at this scale, and clearly positive
in a game with more than one collider shape.

**Decided: declined for v1, and the boundary is documented.** ADR-0022 is accepted
as recommended, so no `Rect::sweep`, no segment-versus-rect helper and no
`Rect::inflate`. The argument is the eight-versus-thirty split above: a primitive
that answered "where did they first touch" and refused "and what happens now"
would be the start of a collision subsystem, which ADR-0001 scopes out — and the
bugs all three runs actually shipped (run 1's bounce plane 1.5 units behind the
paddle, run 4's sign error) were in the thirty lines, not the eight.

**What changed is the document, not the API.** Concepts' fixed-timestep paragraph
now says the absence is a v1 boundary rather than something the reader has missed,
and gives the eight-line shape to write instead. That is the treatment `App::quit`
gets, and run 4 is the evidence it works: it called that "the right way to document
an absence" and said the quit boundary cost it nothing, in the same log where an
*undocumented* absence (sound, F-052) was the one thing it felt as a loss.

**`pong` is unchanged**, deliberately: `advance` in `main.rs` stays exactly as run
4 wrote it. This is the one finding where the E0 rule that a fixed finding should
simplify the game does not apply, because the decision is that the code is the
game's job.

### F-042 — `Camera::visible_bounds` returns `(Vec2, Vec2)` where `Rect` is that pair

Class: engine · Run: 4 · Fixed in: this commit · **ADR-0021, accepted** · No ADR
or `DELIBERATE:` tag fixed the tuple — checked, which is what made it changeable

**What the run did.** Wrote the off-screen assertion F-029 added, which is the
document's own recommended check.

**What happened.** Six lines of hand-written comparisons, twice:

> `Rect { min, max }` is documented as "min: top-left, max: bottom-right", which
> is precisely the pair this returns. The consequence is visible in the
> document's own recommended assertion: six lines of hand-written `>=`/`<=`
> comparisons that would be one call on a `Rect`.

**Checked, as the brief asked: nothing settles this deliberately.** `camera.rs`
carries `DELIBERATE:` on `Camera::default` (ADR-0012) and nothing on
`visible_bounds`; no ADR mentions it; `renderer.md` does not raise it; and there
is no crate-boundary reason, because `Rect` lives in core and the camera is
downstream of core and already imports from it. The tuple is not a decision that
was made. It is a signature nobody revisited, and F-001's fix put it in front of
a reader for the first time.

**The cost is measurable and it lands in the one check the document pushes
hardest.** `testing.md`'s off-screen snippet is six lines *because* of this
signature; with a `Rect` it is `assert!(view.contains_rect(bounds))`-shaped, and
every game that writes the highest-value assertion in the document writes the six
lines instead. Run 2 wrote them, run 3 wrote them, run 4 wrote them twice and
factored them into a helper.

**Fixed. `visible_bounds` returns `Rect`, and `Rect` gains `contains_rect`.** Both
halves were needed and the second is the load-bearing one: returning a `Rect` and
leaving the comparison hand-written would have saved one destructuring line, and
the six lines *are* the comparison. `testing.md`'s check is three lines now, and
`pong/verify.rs`'s `assert_on_screen` is one call over a `Rect` parameter instead
of four comparisons over a tuple.

**`contains_rect` is closed on all four sides where `contains` is half-open**, and
that asymmetry is the cost the fix accepts. A quad flush against the camera's edge
is on screen, so an off-screen check written with the half-open rule reports a
false failure; `contains` is half-open because it partitions space between
adjacent rectangles, which is a different question. Both doc comments name the
other and say why, and
`a_box_flush_against_the_edge_is_still_inside_the_box_around_it` pins it — without
that test the fix would have traded one silent trap for another.

**Migrated in the same pass**, with no deprecation, because ADR-0012 forbids
shipping both forms: `pong/verify.rs`, `prototype_kit/main.rs`, `input_echo.rs`,
`testing.md`'s snippet, and the camera's own `world_to_screen` and
`screen_to_world` — both of which destructured the tuple in order to throw half of
it away.

### F-043 — Text has no vertical metric, and `size` was documented in a way that hid it

Class: docs · Run: 4 · Fixed in: this commit

**What the run did.** Placed a score, a hint and two banners, and needed to know
how much vertical room a line takes.

**What happened.** It measured it off the output.

> `TextStyle::width_of` is exact and the document is right to push it. There is no
> `height_of`, and `size` is "the height of one line, in world units —
> *including the gap below it*", so how much of that a glyph actually occupies is
> unstated. I placed everything by its top edge and then read the draw transcript
> to find out where the glyph quads landed — they span exactly `size` top to
> bottom.

**The run's measurement is right.** `glyph_quad` builds a quad of
`(size * 7 / 9, size)` from the pen position, and `layout` puts the pen at the
cell's **top-left** and moves it down by exactly `size` per `\n`. So a line
occupies `at.y ..= at.y + size`, an N-line block is `N * size`, and consecutive
lines tile exactly.

**Root cause, and the old wording was actively misleading.** A cell is 9 texels
tall holding 7 texels of ink, so the clear border is one texel **above and
below** — not "the gap below it". A reader doing layout from that sentence
believes the glyph sits at the top of its `size` and that some unknown remainder
hangs beneath, which is exactly the uncertainty the run reported. The precise
statement was available in the source and the summary said the imprecise half.

**Fix.**

- `size`'s field line is now "One line's height in world units — a glyph quad,
  top to bottom", which states the metric in the 63 characters the reference
  prints. `TextStyle`'s body carries the ink inset (the middle seven ninths, one
  ninth clear above and below) for a reader of the source.
- Concepts states it in the quad-count paragraph, where it sits beside "one quad
  per character": each quad exactly `size` tall and `size * 7 / 9` wide, laid out
  from its top-left corner, so an N-line block occupies `N * size`.

**`height_of` is declined**, and the reason is in the fix's own wording: it would
return `size` for one line and `size * lines` for several — a method that
multiplies by a number the caller already has, which is a second way to do
something (CLAUDE.md's first Never). `width_of` earns its place because the
advance is a font metric only the engine knows; the height is not, once stated.
Saying so in the type's own sentence is the whole fix, and it is the answer to
"either document that or expose it".

### F-044 — An unprintable character is invisible to every assertion a game can write

Class: docs · Run: 4 · **Second sighting** (runs 3, 4) · Fixed in: this commit ·
Settled by: the fallback box is deliberate and tested by name
(`a_character_the_font_does_not_have_draws_the_fallback_box`); F-030 declined an
ADR because there is no oddity to explain

**What the run did.** Typed `"W / S — move"` in a hint line out of habit, and
caught it by re-reading.

**What happened.** Nothing — which is the finding.

> the bounds assertion cannot see it, because a box glyph is exactly the same
> size as a letter. A `debug_assert` in `ctx.text` on unprintable input would
> have caught it for free.

**F-030 is half-closed, and this is the sharp half.** F-030's fix put the range
and the fallback in `TextStyle`'s summary, and it worked as far as it goes: run 4
knew the font was "the ninety-five printable ASCII characters, space through
`~`", quotes that sentence, and attributes the catch to re-reading rather than to
guessing. §6 asked whether run 4 would "use a non-ASCII character *on purpose*, or
avoid the question the way run 3 did" — it did neither: it used one **by
accident**, which is the case neither the clause nor run 3 covered. Run 3 typed a
`·` and could not find out what it drew; run 4 typed an em dash, knew what it
would draw, and had no check that would have said so.

So the corroborated finding is not "what does the font carry" — that is answered.
It is: **the deliberate loud fallback is loud to an eye, and E0's premise is that
there is no eye.** Four runs, four containers with no display (F-054). A design
whose failure mode is "visible" has no failure mode at all here.

**Fix.** *Testing your game* now says the geometry is identical either way — glyph
counts, `width_of` centring and the off-screen bounds check all pass — so the
check has to look at the string rather than at the frame, with the one-line
assertion written out and the three characters that arrive uninvited named
(`—`, `’`, `·`).

**A `debug_assert` in `ctx.text` is declined, and the reasoning belongs on the
record because the brief asked for it.** Three reasons, in order of weight:

1. **It is not a silent failure**, which is the convention that would license a
   panic. The fallback box is the engine's answer to an unknown character and it
   is the same answer as the missing-texture placeholder: draw something loud
   rather than nothing. A `debug_assert` would be a *second* answer to one
   question, on the primitive whose first answer is deliberate and tested.
2. **It would fire on text a game did not author.** A player's name, a file path,
   a loaded string, anything a real game displays. The fallback exists precisely
   so those survive; asserting would turn "one character renders as a box" into a
   crash, in debug, which is where every `--verify` run lives.
3. **The available check is better placed.** The problem is a literal in the
   game's source, and the game knows which of its strings are literals. A
   one-line assertion over those strings catches it before a frame is drawn and
   costs the engine nothing.

What is *not* declined is the observation underneath: a visual-only failure mode
is a failure mode E0 cannot see, and that generalises past text. It is recorded
here and in F-054.

### F-045 — `sin_cos` has two spellings, and the API document itself teaches both

Class: docs · Run: 4 · Fixed in: this commit

**What the run did.** Read two worked examples before writing a line, and could
not tell which import was the rule.

> `prototype_kit/main.rs` opens with `use jidousha::math::sin_cos;` *alongside*
> `use jidousha::prelude::*`, which reads as "the prelude does not have it".
> `vec2_tour.rs` imports only the prelude and calls `sin_cos` […] For an engine
> whose first convention is "one way to do everything", two working spellings of
> the same import in two example files is exactly what a game author copies
> wrongly.

It used the prelude, which is right.

**Wider than the run could see, and this is the part that makes it a `docs`
finding rather than an example bug.** The API document **tells** the reader to
write the path-qualified spelling. `docs/conventions.md`'s Math section said
"use `jidousha::math::{sin_cos, atan2, ...}`", and that line rides into the
document through the conventions digest — so the reader who checks the rule is
told the module path, and the reader who reads `math`'s reference entry gets
`pub mod math` plus one line of prose that does not say the contents are
re-exported. `prototype_kit` was not diverging from the document. It was
following it. This is the same shape as F-035, where the disagreement between two
examples turned out to include the Quickstart, and it is a class no run can
diagnose: it requires knowing that `conventions.md` is an input to the file being
read.

**Fix.**

- `conventions.md`'s Math section now rules explicitly: the path-qualified
  spelling is the engine's, because engine-internal code has no facade to reach
  through, and the prelude is the game's. It reaches the document through the
  digest, which is where the wrong version came from.
- `math`'s module summary — the one sentence the reference prints — now says every
  name in it is re-exported by the prelude, so a game never writes the path.
- `prototype_kit/main.rs`'s redundant `use` is deleted. The two examples now
  agree, and they agree with the document.
- `vec2_tour.rs`'s comment naming the module gains a clause saying the prelude
  re-exports it, because that example is the one a reader trusts on `Vec2`.

**Why clippy never caught it.** A glob import and an explicit import of the same
item is legal and warning-free — the explicit one simply shadows the glob with
itself. There is no lint to turn on; the guard is the ruling.

### F-046 — The API document never mentions the `--verify` convention

Class: docs · Run: 4 · Fixed in: this commit

**What the run did.** Knew about `--verify` from its task and from
`prototype_kit/main.rs` sniffing `std::env::args()`.

**What happened.** Nothing, for this run. For a reader of the document alone:

> Its last line says `tools/verify <example>` "is the whole loop as one command",
> but nothing says that the loop is a mode the *example itself* has to implement,
> or that the switch is spelled `--verify`. […] A game author working from the
> document alone gets the whole "Testing your game" section — which is excellent
> — and no idea that there is a convention for wiring it to a command line.

**Root cause.** *Testing your game* was written outward from the vocabulary —
`headless`, `InputScript`, `SnapshotBuilder`, `FrameRecorder` — and its closing
line names the tool that runs the result. The convention that connects the two is
in `tools/verify`'s docstring, a file the document's reader has no reason to open
and E0's reader may not. Everything the section teaches is unreachable from a
command line without it.

**There is more of it than the run could see.** The wrapper looks for a verdict
line beginning with `verified `, and treats its absence as a *tooling* fault
rather than a failed check — the guard against an example that ignored the flag
and opened a window. An author who implements the mode without that prefix gets a
report saying the tooling broke. That protocol was documented nowhere a game
author reads.

**Fix.** *Testing your game*'s closing paragraph is replaced by the convention:
the mode is the game's, the flag is `--verify`, `main` branches on it before
calling `run`, the verdict line must begin with `verified `, indented lines under
it are the summary and are shown, everything after is kept as evidence. With the
`main` that does it, matching the Quickstart's `ExitCode` shape and its
`Display`-not-`Debug` error print (F-022). `tools/verify <example>` and the
by-hand `cargo run … -- --verify` are both named.

### F-047 — A mediocre controller does not report "unplayable". It reports a plausible wrong number, and you retune the game

Class: docs · Run: 4 · **Fourth sighting, and the corroboration §6 was waiting
for** · Fixed in: this commit

**F-037 predicted this run exactly, and named what to do if it happened.**

> **Whether F-037 needs a worked example after all.** Three runs have made a game
> unwinnable with a perfect tracker. The fix is prose […] A fourth run that walks
> into it is the evidence that prose is not enough, and the answer then is a
> worked controller in a game deliberately unlike Pong.

Run 4 walked into it. It also read the prose — it quotes the paragraph by name and
calls it "sharper than it reads" — so this is not a run that missed the warning.
It is a run that took the warning, wrote a controller that *aimed*, and was still
wrong.

**What happened.**

> I wrote the naive version first: predict the intercept, stand so the ball meets
> the paddle off-centre, aim away from wherever the opponent is standing. It won,
> so I believed it. The match took **79 seconds** […] and I spent six runs —
> twenty to forty seconds each — retuning `AI_SPEED`, `SPEED_GAIN` and
> `MAX_BALL_SPEED`, watching the summary line and guessing.
>
> None of the tuning was the problem. The controller was. Aiming "away from where
> the opponent is standing" is worthless against an opponent that drifts back to
> the middle between shots — by the time the ball arrives they are not there any
> more. Replacing it with "try every return this paddle can produce, work out
> where each would reach the far side, take the one that lands furthest from the
> middle" took the match from 79 s to 43 s **with the game unchanged**.

**Root cause, and why this is a new finding rather than F-037 recurring.** F-037's
prose describes the *degenerate* failure — the controller that centres every
return and reports 0–0. That is the loud version and the paragraph closes it: run
4 did not hit it. The version run 4 hit is quiet. The controller wins, so it looks
like it works; the number it prints is wrong by a factor approaching two; and the
only instrument available is that number, so the author tunes the game until the
number moves. **Six wasted runs on constants, on a game that needed no constant
changed.** F-037's paragraph tells you a timid controller under-reports. It does
not tell you that under-reporting sends you to edit the thing being measured, and
that is where the hours went.

Run 1 and run 2 both show the same second-order cost in a milder form — run 1's
three tuning passes, run 2's three — and in both cases the diagnosis was that the
*test player* was the fault, not the game. So the pattern is in all four runs and
only run 4 named the mechanism.

**Fix.** A paragraph in *Testing your game* immediately after F-037's, carrying
the run's numbers: 79 seconds to 43 with the game byte-identical, six tuning runs
spent first, and the two rules that follow — get the controller playing to win
before you believe any number it prints, and when a number looks wrong suspect the
controller first, because it is the newer and worse-tested of the two.

**The worked example is declined again, and for a stronger reason than F-037's.**
F-037 declined it because a worked controller that plays a game to win is a worked
*game*, and `crates/jidousha/examples/` is on E0's allowed list — shipping one
would hand the next run the answer (F-020). That still holds. What run 4 adds is
that the example would not have helped: run 4 had the prose, understood it, and
its controller was still the wrong one, because "aim away from the opponent" is a
*correct* reading of the advice that happens to fail against a returning opponent.
The gap is not the absence of a demonstration. It is that the advice named a
direction ("play to win") and not a test ("does your controller's model of the
opponent survive the opponent moving?"). The fix is the sharper sentence, and the
worked-example lever stays unspent.

**This is F-047's honest verdict and it is a prediction, so it is written down to
be checked:** if run 5 also mis-tunes a game because of its controller, prose has
had three attempts and the answer is the worked controller in a game deliberately
unlike Pong.

### F-048 — `Time::alpha` is defined precisely and nothing consumes it

Class: docs · Run: 4 · Fixed in: this commit

**What the run did.** Read the field, looked for what uses it, found nothing, and
ignored it — which is correct, and it filed the question under "things I wanted to
look up in the source".

> It is defined ("how far into the next tick the last rendered frame fell") and
> nothing in the API consumes it — there is no interpolation helper, and `Draw`
> reads the world's committed state. So a fast-moving ball judders at the fixed
> timestep and the field for fixing that exists but has nothing to plug into.

**The run's reading of the mechanism is exactly right.** `Simulation` computes
`alpha` from its accumulator and writes it into `Time` every step; nothing else in
the engine reads it. `Draw` sees committed state, so a game that submits
`transform.pos` unchanged steps at the tick rate however fast frames arrive. The
field's user is the **game**: keep last tick's value in a component of your own and
submit `previous.lerp(current, alpha)` from the Draw system. There is no lerp
helper and no engine-side notion of a previous transform, deliberately — that
would be retained render state, which renderer.md §2 rules out.

**Root cause.** The field's doc said what the number *is* and who may read it
("Draw-phase only") and never said what a game does with it or that the engine
does nothing with it. And its summary was 70 characters, so the reference printed
"How far into the next tick the last rendered frame fell, in…" — truncated before
the useful half. This is the fourth instance in run 4 of the same generator
constraint biting (F-039, F-043 and F-048), which is F-054's
observation about the pipeline.

**Fix.** The field's summary is now "A Draw-only interpolation fraction that
nothing in v1 consumes" — one sentence, 62 characters, and the fact the run
wanted. Its body says the engine draws no interpolation of its own, gives the
`previous.lerp(current, alpha)` shape, and says that ignoring it is the correct
move for a prototype. Concepts says the same beside the drawing paragraph, because
"why does my fast ball judder" is a Concepts question.

**"A field with no user yet" is not the verdict.** It has a user; the user is the
game, and the document did not name it. That distinction is the whole of the
finding: run 4 asked "is it for something the document should name, or is it a
field with no user" and the answer is the first one.

### F-049 — `Depth::layer`'s numbering is a convention propagating by imitation

Class: docs · Run: 4 · Fixed in: this commit

**What the run did.** Copied `prototype_kit`'s `mod layers` wholesale, minus the
`DEBUG` band, and said so.

> "Draw ordering" says `layer` is "the coarse tool (background/world/UI bands)"
> and stops. Every game will invent its own numbering. I copied `prototype_kit`'s
> `mod layers` wholesale because it is the only worked example of the idea, which
> means the convention is propagating by imitation rather than by being written
> down.

The diff is exact: `FIELD = -1`, `PLAY = 0`, `UI = 2`, in that order, with `DEBUG
= 1` dropped because this game has no hitbox overlay.

**The engine's behaviour is right and the example's comment is right.**
`prototype_kit`'s module says "this is the layering convention a real game would
put in its own module — naming the bands is what stops `z: 3.0` appearing in forty
places", which is precisely the intended reading: the numbers are the game's and
the engine sorts by them without an opinion. Nothing is wrong here except that
the *document* never says it, so a reader cannot tell whether they are copying a
convention they are allowed to change.

**Fix.** One clause in Concepts' drawing paragraph: `layer`'s numbers are yours,
the engine has no opinion about what they mean, name your bands once in a
`mod layers` of your own, and `examples/prototype_kit` is the worked version.
Naming the example is deliberate — it is not on the deletion list the way a game's
own files are (F-019, F-020), so a citation to it stays true between runs.

**Copying by imitation was the right move and stays available.** The fix does not
add an engine-side layer enum; that would be the engine taking an opinion it has
no business having, and it would break the first game whose bands are not
background/world/UI.

### F-050 — `Phase` looks taken and is not

Class: docs · Run: 4 · **Third instance of F-017's class** · Fixed in: this commit

**What the run did.** Named its screen-state enum `Stage`, believing `Phase` was
unavailable.

> The prelude exports a `Phase` trait. Mine is called `Stage`. Trivial, but the
> obvious name for a very common game-side concept is taken by an engine concept
> a game never names directly.

**The premise is false and the cost was still real.** `Phase` is not in the
prelude. It is not exported from the facade at all — neither at the crate root nor
in `prelude` — so `use jidousha::prelude::*;` imports no such name and a game's
`enum Phase` compiles. The run gave up a name that was free.

**Root cause — the gate F-017 deferred and F-036 called overdue.** `Phase` appears
in the document exactly where F-017 said this class appears: as a bound in a
rendered signature, `pub fn add_system<P, F>(&mut self, phase: P, system: F) where
P: Phase, F: IntoSystem<P>`, with no entry of its own, because the facade is a
curation and `Phase` was curated out while the method naming it stayed in. F-017
listed `Phase` and `IntoSystem` by name among the remaining candidates and said
each needed a decision. **This is the decision for those two, and it is the third
run to find this class by hand** — run 2 found `Submissions`, run 3 found `Batch`,
run 4 found `Phase`. Unlike the first two, this one is not "a type I could not look
up". It is "a type I wrongly believed I could not use", which is a strictly worse
outcome from the same hole: the reader who cannot find an entry may infer either
"opaque, ignore it" or "reserved, avoid it", and nothing tells them which.

**Fix, and it is the opposite of the previous two.** `Submissions` and `Batch` were
*exported* to close their holes. `Phase` and `IntoSystem` are **not** exported:
they are trait bounds a game satisfies by passing `Startup`, `Update` or `Draw`,
and exporting them would put two names in the prelude that no game writes, which
is the curation invariant working as intended. So the fix is a sentence instead —
Concepts' phases paragraph now says the three phase types are the whole set, that
`Phase` and `IntoSystem` are bounds in `add_system`'s signature, are not exported,
and are not names a game can collide with.

**The gate is still not built** and the remaining candidates are down to F-017's
list minus these two: `ByteSource`, `AssetHandle`, `AssetKind`, the query traits
(`Query`, `ReadOnlyQuery`, `QueryIter`, `QueryIterMut`, answered by Concepts'
query prose) and `CommandKind`. Three runs have now paid for this by hand and each
payment has been cheap; the argument for building the gate is that the fourth
payment was a *wrong belief* rather than a lookup, and a wall of exemptions with
reasons would have prevented it.

### F-051 — `Seconds` is a newtype you leave at the first multiplication

Class: author · Run: 4 · Fixed in: one sentence, because the reason was never
written down

**What the run did.** Wrote `as_f32()` in every system that moves anything, and
said the examples do too.

> Every integration step is `something * world.resource::<Time>().fixed_dt.as_f32()`.
> `Seconds` has `Add` and `Sub` and nothing that multiplies a rate, so
> `as_f32()` appears in every system that moves anything. The examples all do
> this too, so it is the intended shape — but "units live in types" ends at the
> first multiplication.

**Where the answer already was**, per §1's rule for an `author` finding —
`docs/api/jidousha-api.md`, `Seconds`:

> `pub fn as_f32(self) -> f32;  // The underlying value, for arithmetic the
> newtype does not cover`

The run diagnosed it correctly and unprompted, concluded the shape was intended,
and was right.

**And the absence is correct, which is the part worth recording.** A `Mul<f32> for
Seconds` would have to return `Seconds`, and `rate * dt` is a **distance**, not a
duration. There is no operator that types that correctly without a general unit
system, which ADR-0001's scope does not contain. So `as_f32` at the integration
step is not the newtype failing — it is the one place where leaving the newtype is
the dimensionally honest move, and the newtype has already done its job by making
seconds impossible to confuse with milliseconds on the way in.

**Fix: one sentence, not an operator.** `as_f32`'s doc comment now says why there
is no multiplication and that this is the expected call rather than a fallback.
Classified `author` because the document already pointed at `as_f32`; recorded
because §1 says three `author` findings on one topic is a `docs` finding wearing a
hat, and this is the first on this one.

### F-052 — Sound, and the absences a game author feels

Class: engine, out of v1 by scope · Run: 4 · Fixed in: nothing; recorded for the
roadmap

**What the run said**, in "things I expected to exist and could not find":

> **Sound.** There is none in the document, so presumably none in v1. Pong without
> the blip is noticeably less of a game, and this is the one absence I felt as an
> author rather than as a programmer.

**Verdict: correct, out of scope, and the framing is the finding.** ADR-0001 scopes
v1 and audio is not in it; there is no audio crate, no `Submit` verb, nothing
deferred-but-started. So there is nothing to fix and no ADR to write — a subsystem
that was never begun does not need a decision recorded, it needs to appear on the
roadmap, which implementation-plan §3 says is the conversation after v1.

It is logged here rather than only noted because of *how* the run flagged it. Every
other finding in four runs is a programmer's finding — a missing signature, an
unstated behaviour, a shape that does not compose. This is the only one filed as
"the game is worse". That is a different instrument and it is the one E0 exists to
be, so it should not vanish into "not v1". **First candidate on the post-v1 list,
with this sentence attached.**

**The quit boundary is the counter-example and belongs here for contrast.** The run
also wanted an Escape key and could not have one, and filed it "only because a Pong
wants one" while saying the document handles it correctly:

> The document is explicit that this is a v1 boundary rather than an omission,
> which is the right way to document an absence.

F-027's fix is therefore confirmed working by the run that hit the same wall run 2
hit. Two absences, identical size, opposite experiences — one documented as a
boundary and costing nothing, one undocumented and felt as a loss. That is the
argument for naming absences, and it is now measured rather than asserted.

### F-053 — On screen is not in the right place, and the transcript is the only instrument

Class: docs · Run: 4 · Fixed in: this commit

**What the run did.** Hung its hint line off `bottom_right.y - 1.3` — off the
camera — while the field's bottom wall is drawn at `FIELD_BOTTOM`, inside it. The
text sat on top of the wall.

> Nothing failed: it was on screen, so the bounds assertion was happy. The
> document claims the transcript is "good enough to check a layout by eye" and
> that is true — but it is also the *only* way, and "by eye" means reading a
> hundred lines of coordinates and holding the picture in your head.

It found the bug by reading 101 lines of transcript.

**Root cause.** F-029 gave the document its highest-value assertion and F-032
extended its domain to unreached screens; neither says what the check does **not**
cover. "Inside the camera" is a weak predicate — it catches the overrun that
motivated it and nothing about relative position. Every run has now used the
transcript as the instrument of last resort for layout (run 1 for the whole
layout, run 2 for the overrunning banner, run 3 for the advance width, run 4 for
this), and the document presents it as a nice-to-have beside the assertion rather
than as the thing that actually finds layout bugs.

**Fix.** A short paragraph after the unreached-screens snippet: "on screen" is not
"in the right place", assert quads against the game's own layout constants — a
field edge, a margin, the band the score lives in — and not only against the
camera, because otherwise the transcript is the only instrument and reading it
means holding a hundred lines of coordinates in your head.

**Deliberately not fixed with engine surface.** An overlap check over drawn quads
is a thing a game can write in three lines from `DrawnQuad::bounds` and
`Rect::overlaps`, and the engine cannot know which pairs of things are *meant* to
overlap — a score over a background is correct and a hint over a wall is not.
Naming the assertion is the whole available fix.

### F-054 — Four runs, four machines with no display: nothing in E0 has ever rendered a pixel

Class: environment, escalated · Run: 4 (and 1, 2, 3) · **Resolved after run 5** —
see the resolution note at the end of this entry

**What the run said.**

> I never saw the game. This machine has no display (`run` returns
> `RunError::NoDisplay`, with a genuinely good four-part message) and no Vulkan
> ICD, so `WgpuBackend::offscreen` has nothing to talk to either. I deliberately
> did **not** add the PNG capture that `prototype_kit` has, because here it could
> only ever print "skipped, no GPU on this machine" and I would be shipping a code
> path I had never executed.

**Confirmed, for the fourth time, and confirmed for the maintainer too.** This
session's container: `DISPLAY` and `WAYLAND_DISPLAY` both unset, no
`/usr/share/vulkan/icd.d`, no `/dev/dri`. So **the layout claims in run 4's log
remain unverified by anyone**, exactly as they stand, and this triage could not
check them either. Every run so far has been rescued by a human playing the game
afterwards — runs 1 and 3 explicitly — which is a person doing what the harness
cannot.

**The run's decision not to add the PNG step was right** and should be recorded as
right, because it is the second time an author has reasoned their way to it (run 3
§2.2 did the same). A capture path that always prints "skipped" is a code path
nobody has run, in a file whose whole purpose is to be evidence.

**Verdict: this is an environment gap, not an engine one, and it is not agent-
fixable.** The engine has the pieces — `WgpuBackend::offscreen` renders headless
and `tools/verify` already captures a PNG "if the machine has a GPU". The missing
thing is a machine where that condition is ever true. Per CLAUDE.md, missing
system deps and GPU/driver issues escalate rather than getting worked around, so
the fix is **a software Vulkan ICD in the E0 container image**.

**Correction, and it makes this cheaper and more embarrassing than first written.**
The repository already does exactly this, one directory away: `.github/workflows/ci.yml`
installs `mesa-vulkan-drivers` on the Linux runner, with a comment saying it is
"what turns a skipped tier into a running one — not a workaround for a failure",
and uploads `target/verify/*.png` as an artifact. **So CI renders pixels and has
for some time.** The gap is not that nobody knows how; it is that the E0 authoring
container was never given the package the CI container was. The escalation is
therefore one apt line, already written down in this repo, copied from the runner
image to the E0 image — not a design question at all.

**Two consequences that follow from the correction.**

- **`pong` still produces no picture even on CI.** It is in `VERIFIABLE_EXAMPLES`
  as of this commit, so `tools/verify pong` now runs on every push, on a runner
  that *has* the rasterizer — and captures nothing, because run 4 deliberately
  shipped no capture path (correctly: on its own machine that path could only ever
  print "skipped"). `prototype_kit` is the only example whose frame reaches the
  artifact. **Not added here**, deliberately: the author who ships a capture path
  should be an author who can execute it, so this belongs with the container fix
  rather than ahead of it. It is the first thing to do after that lands.
- **The finding's headline needs qualifying.** "Nothing in E0 has ever rendered a
  pixel" is true of every E0 *run* and false of the project — CI has been drawing
  and uploading frames the whole time. What four runs lacked was not a renderer
  that works; it was the ability to look at their own work while doing it, which is
  a harness property, not an engine one.

**What is deliberately *not* proposed.** A CPU rasteriser behind the backend seam,
so that `NullBackend` could produce an image. It would be a second renderer to
keep honest, it would need golden images of its own, and ADR-0003 puts one backend
behind the seam at a time. The transcript already is the deterministic
machine-readable frame; what is missing is a human-readable one, and a driver
supplies that without the engine growing a subsystem.

**Resolved after run 5, and the escalation was accepted.** `.claude/hooks/session-start.sh`
installs `mesa-vulkan-drivers` in a remote session, registered as a `SessionStart`
hook so every future E0 container gets it without a maintainer remembering — which
matters, because the one other thing this checklist asks a maintainer to remember
was missed twice (§4b). The hook is the CI runner's own apt line, moved to where
the authoring happens, and it is deliberately a no-op on a local checkout.

**Verified in this container rather than assumed**, which is the whole point of a
finding about not being able to look:

- `tools/doctor`'s gpu line went from "no vulkan drivers installed" to listing
  eight ICDs including `lvp_icd.json`, which is lavapipe.
- `tools/verify prototype_kit` now writes `capture: 480x270 written to
  target/verify/prototype_kit.png` where it used to say "skipped, no GPU on this
  machine" — **the first frame any E0-class session has rendered and looked at.**
- The golden tier runs rather than skips. Confirmed by mutation, not by the tests
  passing: swapping a wrong image in for `sprite_scene.png` fails
  `a_rendered_frame_matches_its_reference_image` and leaves
  `target/verify/golden/sprite_scene-actual.png` behind exactly as renderer.md §9
  promises, and the correct reference passes. A skipped golden test also passes, so
  the passing run alone would not have been evidence.
- Every branch of the hook was executed, including the cold install — the package
  was purged and the hook reinstalled it. The one path not exercised is the
  unreachable-archive branch, which needs a broken network to reach; it prints the
  four-part message and exits 0, degrading to the state described above.

**What this does not resolve.** Still no `DISPLAY`, so `run` still returns
`RunError::NoDisplay` and a windowed game still cannot be *played* here — the
after-the-run step 2 playtest remains a human's. What changed is that a run can now
see a still frame of its own work, which is what four runs of "I have never seen
this game" were actually asking for.

**And the first thing to do after this lands is now doable**: `pong` ships no
capture path, so `tools/verify pong` captures nothing even on a machine that can
render. Run 4 was right not to add a code path it could never execute — that
condition no longer holds, and the next E0 author will be the first who can write
that path and run it.

**The related question the brief raised — "is an example that cannot be seen by
the agent writing it a gap in `tools/verify`" — answers no.** `tools/verify` does
everything it can without a display: it runs the mode, parses the verdict, keeps
the transcript as evidence, and captures a picture when a picture is possible. The
gap is one layer down. Recording the distinction because the tool is the tempting
place to change and would be the wrong one.

## 4b. Run 5 triage — the whole run on one page

Eleven findings, in the order run 5's cost ranks them. **Class** is §1's;
**settled by** names the ADR or `DELIBERATE:` tag that already answers the
complaint, where one does.

| # | Finding | Class | Also found by | Settled by | Verdict |
|---|---|---|---|---|---|
| F-055 | `FrameRecorder::transcript` says "last frame", renders all of them | docs | first | **ADR-0023** (retention is deliberate) | doc fix landed; the *function* is right and its description was wrong |
| F-056 | "take the best shot" puts a controller on the boundary of feasibility | docs | **runs 1, 2, 3, 4** as F-037/F-047 | F-047 predicted this run and was wrong about how | doc fix landed — **fourth prose attempt**, and the first with a mechanism |
| F-057 | the two worked examples disagree about a Draw system's `Vec` | docs | 3rd of F-035/F-045's class | ADR-0013's read/write split | doc fix landed; `prototype_kit` stopped collecting |
| F-058 | a run only tests the states it reaches | docs | first | ADR-0022 (the sweep the run wrote) | doc fix landed, plus a hole found in `prototype_kit`'s own paddle check |
| F-059 | `DrawnQuad` carries no layer, so draw order is unassertable | engine | first | **nothing — checked** | **ADR-0024 accepted**: the premise is false, the field is **declined**, the vocabulary is now stated |
| F-060 | `width_of` cannot centre a multi-line block | docs | 2nd half of F-025 | ADR-0018 (depth in the style, unrelated) | doc fix landed |
| F-061 | the `--verify` skeleton and `prototype_kit` disagree about failure | docs | first | — | doc fix landed; **the example changed, not the document** |
| F-062 | nothing says whether the first Update sees tick 0 or 1 | docs | first | — | doc fix landed; the answer is 1 |
| F-063 | two spellings of "the player is present and idle" | author | first | **ADR-0019** | **declined**; one sentence saying which is which |
| F-064 | an opponent that reads the ball every tick is unbeatable | author, out of scope | first | — | nothing to fix; where the tuning cycles went |
| F-065 | five runs, no display: the game is still unseen by anybody | environment | **runs 1, 2, 3, 4** | **F-054**, unresolved | **escalated again**; the fix is still one `apt-get` line |

**One proposal, and it is a decline.** ADR-0024 is `accepted`, which means
`DrawnQuad` does not gain a `Depth`. It is the fourth consecutive run to produce
at most one engine finding, and the first whose engine finding turned out to rest
on a false premise: **draw order is observable and always has been.**
`FrameRecord::quads` is the plan's sorted sequence, so an index comparison is a
layering assertion, and `covering`'s front-to-back is that order reversed. What
the run could not do was read back the `layer` number — which, as ADR-0024 argues,
is the one thing that would not have caught the bug it wanted to catch, because a
`layer` read back only restates what the game submitted.

**What this triage still cannot settle, stated plainly.** Three things.

1. ~~**F-065, for the fifth time.**~~ **Taken, in the commit after this one.** At
   triage time this container had no display and no adapter, so nothing in E0 had
   ever rendered a pixel. The maintainer authorised the escalation immediately
   afterwards and `.claude/hooks/session-start.sh` now installs the rasterizer in
   every remote session: `tools/verify prototype_kit` writes a PNG, the golden tier
   runs, and F-054 carries the verification. Still no `DISPLAY`, so *playing* a
   windowed game remains a human step.
2. **F-056's fix is the fourth attempt at one paragraph, and whether it works is
   a measurement.** Run 4's watch list predicted in writing that a fifth run
   mis-tuning its game because its driver was wrong would mean prose had failed
   three times and the answer was a worked controller in a game unlike Pong.
   That is exactly what happened. The lever is deliberately **not** spent yet,
   and §6 says why and what would spend it.
3. **Nothing checks whether a summary is *true*.** Run 4's headline was that the
   generator keeps the first sentence and nothing asks whether it is the sentence
   that matters. F-055 is one step earlier and worse: the sentence the generator
   carried was simply false, in two places, and it had survived every review this
   repository has. No generator gate can catch that — a sentence that contradicts
   its own function is a review failure, not a tooling one. §6 says what follows.

**Two things that were not findings and are corrections to this file.**

**`pong` was left out of `WINDOWED_EXAMPLES` and `VERIFIABLE_EXAMPLES` again**,
so `tools/test` ran run 5's game as an ordinary example, watched it try to open a
window, and failed on `RunError::NoDisplay` for reasons nothing to do with the
code. This is the **second consecutive run** it has happened — run 4's triage
recorded the same miss and fixed it — which makes it a property of the procedure
rather than an accident. `e0-prompt.md` deliberately splits de-registration
(step 2, before the run) from re-registration (step 6, after it) into different
commits, so nothing structural connects them and the second half is remembered or
not. Both sets are corrected in this commit and the trap is now named in
tooling.md, where a maintainer chasing a red `example:pong` phase will meet it.

**`prototype_kit`'s paddle check had a hole, and mutation testing is what found
it.** "A paddle-sized quad covers this point" passes for a paddle drawn 45% of
its own height out of position, because a paddle covers its own centre wherever
it is drawn. Verified by breaking the game on purpose during this triage: exit 0,
no complaint, every other assertion green. The check now compares the quad's
bounds. See F-058, which is where the technique that found it is recorded.

### F-055 — Two methods called `transcript`, one description, and it fits the other one

Class: docs · Run: 5 · Fixed in: this commit · Settled by: **ADR-0023** (the
recorder keeps every frame on purpose)

**What the document said**, in both places it mentions the method:

> `pub fn transcript(&self) -> String;  // The last frame as stable, diffable text`

and, in *Testing your game*:

> `recorder.transcript()` renders the last frame as stable, diffable text —
> every quad's world-space extent, one per line.

**What it does.** Every frame the recorder holds, each headed `frame N:`. Run 5
recorded 1,263 frames and `print!("{}", recorder.transcript())` produced
**121,465 lines**.

**Why it stayed wrong, which is the interesting half.** The `--verify` convention
this file added in run 4 (F-046) says the verdict line and its indented summary
are shown and "everything after that is kept as evidence rather than reprinted,
which is where the transcript goes". So an author who follows both instructions
literally emits a hundred thousand lines per run and **never sees them**. The
document's two statements are individually plausible and jointly invisible. Run 5
caught it in about five minutes and only because the output looked wrong at a
glance; it says so itself — "it would have cost nothing and stayed wrong if I had
not looked."

**The function is right and the description was wrong**, which is the ruling this
finding needed and did not obviously have. ADR-0023 already decided that
retention is deliberate: the frame history is what a failing assertion reads
backwards, `clear()` is declined for the same reason, and a check that threw away
the tick before the one that broke has thrown away the interesting tick. Nothing
here reopens that. `FrameRecord::transcript` — on the frame `draw` hands back —
is the one-frame version and already existed two entries away, with the
near-identical summary "The frame as text: deterministic, diffable…", so the two
were a sentence apart and only one of them was about one frame.

**Fix.**

- `FrameRecorder::transcript`'s summary is now "Every recorded frame as text,
  oldest first — not only the last", and its body says what the history is for
  and points at `FrameRecord::transcript` for a screenshot.
- `FrameRecord::transcript`'s summary is now "This one frame as stable, diffable
  text — every quad, one per line", so the two adjacent reference entries cannot
  be read as the same thing.
- *Testing your game* carries the distinction as its own short paragraph, and the
  `--verify` convention names `frame.transcript()` where it used to say "the
  transcript".
- `the_recorders_transcript_carries_every_frame_and_a_records_carries_one` pins
  them apart so the descriptions cannot drift back together.

**What this says about the pipeline, and it is not what run 4 said.** Run 4's
headline was that the generator keeps a doc comment's first sentence and nothing
asks whether it is the sentence that matters. This is one step earlier: the first
sentence was *false*, and being false it was carried faithfully into the
reference and then paraphrased by hand into the prose, where it became false
twice. No gate over rendered summaries catches that. What catches it is a test
that asserts the sentence, which is why the guard above exists and why it names
both methods.

### F-056 — "Take the best shot available" resolves to "stand on the edge of your paddle"

Class: docs · Run: 5 · Fixed in: this commit · Also found by: **runs 1, 2, 3, 4**
(F-037, F-047) · Settled by: nothing — this is the fourth prose attempt

**The most valuable finding of the run**, by its own ranking and by this file's:
two full cycles, and the one item a document change could have prevented outright.

**What the document told it to do.** F-047's fix, written after run 4 lost six
tuning runs to a timid controller:

> replacing the aim with "try every return this paddle can produce, take the one
> that lands furthest from the middle" took the match to 43 seconds **with the
> game byte-identical**.

**What happened.** Run 5 implemented exactly that — thirteen sample contact
points, each pushed through the game's own `contact` function, scored by distance
from anywhere the machine could reach. It lost **0–5** and made six returns in a
minute.

**Why, and this is a fact about optimisation rather than about Pong.** The
sharpest return a paddle can produce is always the one struck at its very tip,
because that is where the bounce angle is widest. So "take the best shot"
resolves *every single time* to "stand so the ball hits your last millimetre".
**The optimum sits on the boundary of the feasible set**, and on that boundary
any error at all — a dead band, half a tick of overshoot — is a clean miss rather
than a worse return. Run 5's dead band was 0.45 world units and the margin at the
tip is zero.

This is the *same reported symptom* as F-037 and F-047 — "the game is unwinnable"
— produced by the opposite fault. F-047 is a controller too timid; this is one
too greedy. Both report a plausible wrong number with the same confidence, and
the previous fix's own example is what steers into the second one.

**Fix.** An addition, not a correction — the existing advice is not wrong, it is
unconstrained. *Testing your game* now says: score only the positions that (a)
really make contact, with margin — a fixed fraction of the paddle's half-length,
so the tip is not on the menu — and (b) can be reached before the ball arrives;
optimise inside what survives both; run at the ball when nothing does. Three
lines of set arithmetic in front of the search. `pong/controller.rs`'s `best_aim`
is the worked version at 78% of the half-length.

#### F-056a — and the warning against this had been read that morning

The paragraph F-047 added ends "when a number looks wrong, suspect the controller
first — it is the newer and worse-tested of the two". Run 5 did not:

> On the 0-5 result I went and changed `SERVE_SPEED`, `SPEED_GAIN` and
> `MACHINE_SPEED`, and added a whole new difficulty knob to the game
> (`MACHINE_VISION`, since deleted), before finding the fault in `best_aim`. The
> document called this in advance, in a paragraph I had read that morning […]
> and I still did it. Reading the warning is not the same as it working.

**That is the finding**, and the run is right to file it as friction rather than
as a personal failing: the measurement is what the document costs its reader, and
the answer is that this warning does not survive contact with a red result. Four
runs, four sightings, and the fourth had read the prose.

**So the fix is not more caution, it is an assertion.** *Testing your game* now
says to check the controller's own contract on the numbers it actually picked,
every tick: a controller that reports "my aim missed the ball on 94% of returns"
has diagnosed itself, where one that reports "the game is unwinnable" has
diagnosed your game. The first is a reading, the second is a conclusion — which
is F-029's rule, applied to the instrument rather than to the game. This is the
first attempt at this paragraph that gives the reader something to *run* instead
of something to remember, and §6 records what happens if it fails too.

### F-057 — The two worked examples disagree about whether a `Draw` system needs a `Vec`

Class: docs · Run: 5 · Fixed in: this commit · Settled by: ADR-0013's read/write
split, ADR-0008's Draw immutability

The cheapest fix on the list and fully verified, by run 5 and again here.

**What the two examples showed.** The Quickstart draws straight out of the query.
`prototype_kit`'s `draw_the_field` and `draw_the_hitboxes` both `.collect()` into
a `Vec` first. Run 5 copied `prototype_kit`, "because it is the bigger example and
I assumed the `Vec` was load-bearing", and wrote a comment explaining why it was
necessary.

**It is not.** `WorldView::query` returns `QueryIter<'w, Q>` — the lifetime is the
*world's*, not the `&self` borrow's — so the iterator holds no part of the
`DrawCtx` and the direct form compiles. Confirmed here by deleting the `Vec` from
both `prototype_kit` systems: `cargo check` clean, `tools/verify prototype_kit`
identical on every number it prints.

**Why it was a reasonable mistake, which is what makes it `docs`.** Concepts' own
"reading while writing: the two-pass pattern" paragraph is emphatic, and a
`DrawCtx` that `ctx.rect` borrows mutably looks exactly like the situation it
describes. Nothing said the rule belongs to `query_mut` and not to `query`. So the
larger example looked like the one that had met the problem, and the reader paid
two allocations a frame and wrote a comment asserting something false.

**This is the third run to find this shape** — two worked examples that disagree,
teaching that there is no rule. F-035 was per-tick versus per-second and included
the Quickstart; F-045 was `sin_cos`'s two import spellings and turned out to be
the *document* teaching the wrong one. Run 5 spotted the pattern itself and cited
F-045 by number from the conventions digest, without ever reading this file.

**Fix.**

- `prototype_kit` draws straight out of both queries, with a comment at the first
  one saying why the `Vec` is not needed and naming `homing.rs` as the example
  where it is.
- Concepts' two-pass paragraph gains its scope: it is a `query_mut` rule, a
  `Draw` system is not subject to it, and collecting first in a Draw system costs
  an allocation a frame and buys nothing.

### F-058 — A run only tests the states it reaches, and a margin is a state a correct game never reaches

Class: docs · Run: 5 · Fixed in: this commit · Settled by: **ADR-0022** (the
sweep the run had to write)

Not a complaint about the document — run 5 files it as "the most interesting
thing the run found about verification", and it is.

**What happened.** The run wrote the eight-line swept paddle test ADR-0022's
boundary paragraph asks for. It also capped the ball at 33 units/s, which at 60 Hz
is 0.55 units of travel against a paddle 0.7 thick — so the ball **cannot** tunnel,
and the sweep never does anything a naive position test would not. It found this
by mutation-testing its own verification:

> replacing the swept test with a position-only one passed the entire session.
> The check that the ball never left the table passed, the match still finished
> 5-0, every drawn-frame assertion held. The sweep is real safety and the run
> could not see it.

**Generalised, and this is the sentence:** everything the document teaches about
verification is about observing a *run*, and a run only exercises the states it
reaches. The safety margins a game is built on are exactly the states a correct
game never reaches. The speed ceiling is a tuning constant; the sweep is what
makes it a margin rather than the only thing between the game and a ball through
the back wall.

**Fix.** *Testing your game* gains "then check the contracts your run never
exercises", next to F-032's "check the screens your run never reaches" — which is
the same idea one level up, and now says so. The shape is to ask the function its
contract directly: one tick of travel eight units long across the paddle, plus the
two negative cases. `pong/verify.rs`'s `check_the_swept_test` is the worked
version and is the only check in that file not about a played match.

**And the technique that found it is now written down too**, because it found
something here as well. Run 5 broke its own game seventeen ways and caught all
seventeen — but **two only after tightening checks it had written carefully and
believed were thorough**, and it says it would not have found either by
inspection. The second was a paddle drawn half out of position passing a
"paddle-sized quad covers this point" check, because a paddle covers its own
centre wherever it is drawn — and it noted that `prototype_kit`'s paddle check
had the same hole.

**It did.** Verified during this triage by drawing `prototype_kit`'s paddle 45% of
its own height out of position: exit 0, no complaint, every other assertion green.
The check now compares the quad's *bounds* as well as its size, and catches the
same mutation. So the engine's own worked example was teaching a check with a hole
in it, found by a technique the document did not carry — which is what promotes
"mutate the game and check the run notices" from a nice habit to a paragraph in
*Testing your game*, with the paddle case as the worked example of why inspection
is not enough.

### F-059 — `DrawnQuad` carries no layer, and the conclusion drawn from that was wrong

Class: engine · Run: 5 · **Declined by decision: ADR-0024** · Fixed in: this
commit (the documentation half)

The only entry run 5 raised as an API change, and the one it named as the single
thing it would add to the engine.

**What the run did.** Followed the document's layer advice completely —
`layers::TABLE`, `PLAY`, `UI` in a `mod layers` of its own, score on `TABLE` so
the ball passes in front of it — and then tried to check it.

> `DrawnQuad` is `{ batch, texture, corners, tint }` […] Swap `layers::TABLE` for
> `layers::UI` on the score and the picture changes — the score paints over the
> ball — and every assertion in this game still passes.

It worked around it by not asserting on ordering at all, and recorded that the
information "exists before planning and is gone after it".

**The premise is false, and that is the finding.** Draw order is exactly what a
recorded frame shows. `plan_frame` sorts by `(layer, z, submission index)`, and
`FrameRecord::quads` hands back that sorted sequence — so a quad's index in it is
its place in the painter's order, and comparing two indices *is* a layering
assertion. `covering(point)` is the same order reversed, so `covering(p)[0]` is
what a player looking at `p` actually sees. The run's own example — is the score
behind the ball? — is a three-line check in either spelling. Confirmed here with a
frame that submits the high layer first: `quads()` returns it second.

**Why the field is still declined**, in ADR-0024 and in order of weight: a `layer`
read back is a tautology (it restates what the game submitted, and passes happily
for a `mod layers` whose constants are in the wrong order); it would be a second
and weaker way to ask a question the order already answers; and the depth is spent
by the time a plan exists, so restoring it means test-only payload on every vertex
or on `Batch`, which crosses the backend seam. The full case is in the ADR.

**This is ADR-0020's failure mode for the second time.** There, silence about
`ctx.circle`'s sixteen quads cost run 4 a debug cycle and made run 3 record a
confident falsehood. Here, silence about what "in draw order" means produced a run
that concluded in writing that the engine cannot see draw order and filed the
missing field as its one engine request. A document that does not say what
behaviour *is* does not merely cost time; it manufactures wrong findings, and this
is now twice.

**Fix.** `FrameRecord::quads` says the order is the depth sort and that an index
comparison is a layering assertion; `covering` says its first element is what the
player sees; *Testing your game* carries the assertion in both spellings and says
plainly that a frame does not carry the `Depth`, and why.
`a_frames_draw_order_is_the_depth_sort_not_the_submission_order` is the guard, and
it submits in the opposite order to the one it expects back so a frame that merely
echoed submission order would fail it.

### F-060 — `TextStyle::width_of` cannot centre a multi-line block, and the failure is invisible

Class: docs · Run: 5 · Fixed in: this commit · Also found by: run 2's F-025, the
other half

**The reference was accurate and the consequence was nowhere.** `width_of` returns
the widest line; `ctx.text` lays a block out from its top-left corner. So centring
a two-line block by subtracting half of `width_of` centres the *longest* line and
hangs every shorter line off to the left of the middle.

Run 5's end-of-match banner is two lines of very different lengths, so this would
have been a visibly crooked screen that passes the off-camera check, the glyph
check and the printable-ASCII check. It caught it by reasoning rather than by
seeing it, "which is luck" — and the run could not have seen it either way (F-065).

**This is a different failure from the one the document warns about.** F-025's fix
covers a banner running off the *edge*, and the bounds assertion's own message
says "text centred by `width_of` is the usual culprit". A crooked block is on
screen, at the right size, in the right band; the geometry is correct and the
picture is not. It belongs with F-044's unprintable character rather than with the
overrun: both are cases where every assertion a game can write over drawn quads
passes identically for the right layout and the wrong one.

**Fix.** `width_of`'s summary is now "In world units — its widest line only, so a
block centres crooked", which is the sixty-eight characters the reference prints,
and its body draws the consequence out. *Testing your game* carries the paragraph
next to the overrun warning, with the fix — one `ctx.text` call per line, each
centred by its own width.
`centering_a_block_by_its_width_leaves_the_short_line_left_of_centre` is the
guard.

### F-061 — The `--verify` skeleton and the worked example disagree about failure

Class: docs · Run: 5 · Fixed in: this commit — **and the example is what changed**

**The disagreement.** The document's skeleton is

```rust
verify::run();                 // ticks, asserts, prints "verified ..."
return ExitCode::SUCCESS;      // or FAILURE, if an assertion reported one
```

with the interesting half in a comment. `prototype_kit`'s `verify.rs` resolved it
the other way, with `fn fail(..) -> !` calling `process::exit(1)` on the first
problem.

**The document argues hard for the design its own example did not implement:**

> A failing assertion has to report the numbers it judged. […] the assertion is
> the only instrument there is, so a message that says only *this is wrong* costs
> a whole cycle to turn into a diagnosis.

An instrument that stops at the first bad reading costs a cycle per fault for
exactly that reason. Run 5 built the third thing — a `Checks` accumulator that
records every failure, prints them all in the engine's four-part shape and returns
`ExitCode::FAILURE` — and it paid immediately: one deliberate break reported six
problems and the precisely diagnostic one, "a ball that misses the paddle is
counted as a hit", was **fourth**. Under `prototype_kit`'s shape it would have
seen only "no one won the match", which is the conclusion rather than the fault.

**So the ruling is that the document was right and its example was not**, which is
the opposite of F-045's shape and worth saying because the reflex here is to fix
the document. `prototype_kit/verify.rs` now carries the same `Checks` accumulator,
returns an `ExitCode` from `run()`, and `main` returns it.

**`process::exit` survives, for a different job**, and the distinction is now
stated at the site: a paddle in the wrong place is one fault among several worth
reporting together, while a paddle that is *gone* leaves nothing after it to
measure. Only the second kind stops the run. The document's skeleton shows the
`ExitCode` return and the paragraph beside it says which is which.

Two of the converted messages also gained the numbers they judged rather than only
what they wanted — "what covers that point is 0.544x0.700, 0.500x4.000" instead of
"no paddle-shaped quad was drawn" alone, which is F-029's rule reaching the one
file in this repository that had been exempt from it.

### F-062 — Nothing says whether the first Update sees `tick == 0` or `1`

Class: docs · Run: 5 · Fixed in: this commit

An unanswered question in run 5's log rather than a wrong answer, and it is
recorded as one: the run gave its machine paddle a reaction time with
`tick.is_multiple_of(12)`, for which the answer does not matter, and says so.

> a game wanting "spawn the boss on tick 600" does, and the two candidate answers
> are one apart.

**What the document offered.** `tick: u64  // Update ticks since startup`, that
`Time::new` is "the clock at the start of a run, before the first tick", and that
`Startup` runs *inside* the first `tick()`. From which the run guessed 1, correctly
— but a guess is what it was, and the run marks it as such.

**The answer is 1.** `Simulation::tick` runs `Startup`, advances the clock, then
runs the `Update` phase, so every `Update` system reads a one-based counter.
`Time::new`'s zero is visible only to a driver holding a world between ticks,
which a game never is.

**Fix.** The field's own line — the sixty-eight characters the reference prints —
is now "Update ticks since startup, counting from 1 on the first Update", the
type's body says why and names the absolute-timing case, and Concepts says it in
the fixed-timestep paragraph where a game author meets `Time::tick` first.
`the_first_update_system_sees_tick_one` is the guard, and it also asserts the zero
before any tick, so the two halves cannot drift.

### F-063 — Two spellings of "the player is present and idle"

Class: author · Run: 5 · **Declined** · Settled by: **ADR-0019**

Run 5 needed an `Input` meaning "present and doing nothing" — not the same as
inserting no `Input` at all — to play a match with an idle player and prove the
game can be *lost*. It found two spellings and observed that the document blesses
both:

- `Input::new(InputSnapshot::new())` — "A tick in which the player did nothing",
  which is what `scripted_player.rs` uses.
- `Input::new(SnapshotBuilder::new().first_tick_snapshot())` — a builder with
  nothing recorded.

It used the second, because its controller already had a `SnapshotBuilder` and it
wanted the idle session to go through the identical path. The run files this as
"a mild scratch against one way to do everything rather than a real cost", and
that is the right weight.

**Declined, because they are not two ways to say one thing.** `InputSnapshot::new`
is the value "the player did nothing". `SnapshotBuilder::first_tick_snapshot` is
the builder's own per-tick call, which ADR-0019 made the single home for the edge
rules precisely so that a closed-loop controller does not hand-build snapshots —
and it happens to yield an idle snapshot when nothing has been recorded, the way
an empty accumulator yields an empty sum. Adding a rule against that would be a
rule against using the builder on the first tick, which is the one tick it must be
used on.

**What was actually missing is which to reach for**, and that is one sentence,
now in *Testing your game*: the value is `InputSnapshot::new()`; a controller that
already has a builder keeps using the builder. The declined thing is any change to
either API.

### F-064 — An opponent that reads the ball every tick is unbeatable, and the arithmetic is not obvious

Class: author, out of scope · Run: 5 · **Nothing to fix**

Recorded because it is where the run's tuning cycles went, and an honest account
of what the exercise cost has to include them.

Run 5's first machine paddle chased the ball's current `y` at 18.5 units/s against
a player at 26. It went 2–0 up with thirty-touch rallies and every knob reached for
made it worse. The arithmetic is why: the ball crosses a 30-unit table in about a
second and the paddle has only 14 units of travel to cover, so *any* speed above
about 14 units/s reaches everything, and dropping it far enough to miss makes the
paddle visibly asleep between points. The knob has to be a reaction *time* — run
5's reads the ball every twelfth tick and drives at what it last saw — and it took
three tries to work out that a constant of that **kind** was what was needed.

**Why nothing is fixed.** §1 requires an `author` finding to carry a quote from
`docs/api/jidousha-api.md` showing where the answer already was, and there is no
such quote: the document says nothing about designing an in-game opponent, because
it is an API reference and not a game-design manual. Nor is it a `docs` finding —
teaching opponent difficulty would be the reference growing a chapter about a
genre, and §7's argument against putting workarounds in the skill applies just as
well against putting game design in the reference. Run 5 classifies it the same
way itself: "a game-design finding rather than an API one".

So this is the second entry in this file for which the taxonomy has no clean slot
— F-052 (no sound) was the first, tagged "engine, out of scope" — and the honest
statement is that it is a real cost that is nobody's bug. It stays because three
cycles is three cycles, and because a fifth run hitting the same wall would change
that judgement.

### F-065 — Five runs, five machines with no display: the game is still unseen

Class: environment · Run: 5 · **Resolved** (with F-054, immediately after this
triage) · See: **F-054**

`cargo run -p jidousha --example pong` on run 5's container printed
`RunError::NoDisplay`, which run 5 calls "a genuinely excellent error message and
exactly the right four parts" — and which means **it never saw the game it built.**
Everything about how the game feels is inference from 1,263 recorded frames of
geometry read as numbers: whether 26 units/s is a nice paddle, whether the
machine's twelve-tick stutter looks like thought or like lag, whether 0.30 alpha
on the score reads as "behind" or as "smudge".

This is F-054 for the fifth consecutive run and it is not a new finding, so it
carries no new fix. What it adds is one more data point and one observation.

**The observation is that the document is written for a reader who cannot look,
and it shows.** Run 5 lists among the things that saved it: the warning that a low
alpha reads much brighter than the number suggests, and the warning that a wrong
character draws as a correctly-sized box no assertion can see. It took both — the
field markings are lower than felt right, every literal is checked against the
printable range. That is F-044's lesson working exactly as intended, and it is
only necessary because of this finding.

**The triage could not resolve it and the commit after it did.** At triage time
`tools/doctor` reported `ENV_OK` with `gpu: no vulkan drivers installed`, naming the
package it wanted; CI had installed `mesa-vulkan-drivers` the whole time. Per
CLAUDE.md's never-agent-fixable list, installing system packages is a human
decision — so the escalation stood until the maintainer took it, which they did
immediately afterwards. F-054 carries the resolution, the verification, and what it
does and does not buy.

**The playtest is a human step and was taken.** `e0-prompt.md`'s after-the-run
step 2 asks a person to run run 5's Pong in a window and in a browser; the
maintainer confirmed it, along with step 1's transcript check, before the decks
were cleared for run 6. `b094da6` is the precedent for recording it. **Run 5 is a
valid run**, and the fifth consecutive one whose game a person had to look at
because the harness could not.

### Findings from outside a run (F-066–)

**These do not count towards §2's two clean runs, in either direction.** They were
found by a maintainer session writing `pong`'s capture path — the thing F-054 said
was "the first thing to do after that lands" — and a maintainer reading the
document is not the measurement E0 is taking. They are numbered and filed here
anyway because they are exactly the shape the register exists for, and because a
run *will* hit both: F-066 is a sentence run 5 already read and correctly declined
to act on, and F-067 is the message any run without a GPU gets.

### F-066 — `tools/verify` captures no picture on its own, and the only sentence about it said it did

Class: docs · Run: none — found while writing `pong`'s capture path · Fixed in:
this commit

**What the document said**, as the last sentence of *Testing your game*, and the
only sentence anywhere on the subject:

> `tools/verify <example>` is then the whole loop as one command: it runs that
> mode under a timeout, parses the verdict, writes a report, and **captures a PNG
> if the machine has a GPU**.

It does not. `tools/verify` has no game, no renderer and no backend; it runs the
example, and its whole involvement with pictures is `parse_artifact`, which reads
one line of the example's output and lifts the path out of it. The PNG is captured
by the *example*, in code the example's author writes. `prototype_kit` has such a
path; `pong` did not, so `tools/verify pong` captured nothing on a machine that
could render — silently, and with a green verdict, which is F-054's second
consequence stated as a fact about the document rather than about the runner.

**This is F-055's shape** — a description that is false rather than absent — and
it is worse than F-055 in one way: a reader who acts on it does nothing, and
nothing is indistinguishable from having done the right thing. E0 run 5 read this
sentence and declined to write a capture path, on the correct ground that on its
machine the path could only ever print "skipped". The triage recorded that as
right. What nobody noticed is that had run 5 been on a machine *with* a driver,
the same sentence would have told it there was nothing to write.

**The engine had every piece and never joined them up.** The `jidousha::testing`
reference block already carries `FrameRecord { pub plan: FramePlan }`,
`RenderBackend::render(&plan)` and `capture()`, `WgpuBackend::offscreen`,
`create_builtin_textures`, `RawImage` and `encode_png` — every signature the path
needs, each with an entry of its own. The missing thing was the sentence that
connects them: **the plan on the frame you already recorded is replayable on a
second backend.** That is a fifteen-line capture path with no second play-through,
and it is now stated — in renderer.md §9 for maintainers, and in *Testing your
game* for authors, with the three silent traps (aspect, the `capture:` wording,
no-GPU-is-not-a-failure) and the rule that you have to open the file and mutate the
game.

**One wrinkle worth recording, because the next person to document this will hit
it.** `docs/api/` may not name the backend seam (public-api.md §4 CONTRACT), and
`gen-api-doc` enforces it on the generated text. The Reference's
`### Testing (jidousha::testing)` block is exempt — a maintainer carved it out
because "a golden image has to be drawn by something" — but the *Testing your
game* prose is not, and a capture snippet cannot be written without naming
`WgpuBackend` (which contains `wgpu`) and importing `RenderBackend` (whose methods
`render` and `capture` are trait methods, not inherent ones). So the prose states
the recipe in words, points at the reference block for the signatures and at
`examples/pong/capture.rs` for the code, and does not carry a snippet.

~~**Not proposed here:** widening the exemption to the whole testing section.~~
**Superseded by ADR-0025, two commits later, and by a better answer than the one
this paragraph was holding open.** The exemption was not widened — the *document*
was split. Testing is now `docs/api/jidousha-testing.md`, which may name a
renderer and exactly two other words, while the game document is checked entire
with no exemption at all. So the carve-out got **narrower**, not wider, and the
recipe is compiling code rather than prose and a pointer.

Worth keeping as a lesson about where this wrinkle came from: the carve-out was
sound for *reference entries* and quietly wrong for *prose*, and nothing said so
because no prose had needed it before. A rule that has only ever been exercised
by one kind of content is a rule whose scope is untested.

~~**The budget is now the constraint on this document.**~~ It was — at ~23,900
of 25,000 — and that pressure is what produced the measurement behind ADR-0025:
**46% of the document was about verifying a game rather than writing one.** The
game document now sits at ~13.3k of 25k and the testing document at ~11.6k of its
own 15k. The lesson generalises past this file: a full budget is usually evidence
of a missing seam rather than a number that wants raising.

### F-067 — "no graphics adapter" is reported as `Unsupported`, so the engine diagnoses the wrong thing

Class: engine · Run: none — found by executing the skip path on purpose ·
**Fixed**: `RenderError::NoAdapter`, in the commit after the one that filed this

**What happens.** On a machine with no adapter, `WgpuBackend::poll` reports
`RenderError::Unsupported { detail: "no graphics adapter: ..." }`
(`jidousha-render-wgpu/src/init.rs:210`). `Unsupported`'s `Display` is the
engine's four-part message with a *fixed* cause and fix:

```
[jidousha] the backend cannot render this frame
  no graphics adapter: No suitable graphics adapter found; ... vulkan
  drivers/libraries could not be loaded, ...
  likely cause: the frame asked for something outside the WebGL2 envelope
  fix: check the texture sizes and the batch count against the envelope
```

The detail is right and the diagnosis under it is wrong twice over. The frame
asked for nothing — there is no device to ask. And the fix sends the reader to
count texture sizes against the WebGL2 envelope (renderer.md §8), a subsystem with
no bearing on the problem, when the actual fix is `apt-get install
mesa-vulkan-drivers` — the very line `.claude/hooks/session-start.sh` and
`ci.yml` both run, and the one F-054 spent five runs escalating.

**Who reads it.** Every headless machine without the hook, which per F-054 is the
default condition of an E0 container and of any developer laptop that has not run
the hook. It is the single most common render failure this project produces, and
it is the one whose message is furthest from the truth. Practices §5.5 — errors
are documentation delivered at exactly the right moment — is the rule it breaks,
and it breaks it for the reader least able to check.

**Confirmed by execution, not by reading**: hiding the ICD with
`VK_DRIVER_FILES=/nonexistent WGPU_BACKEND=vulkan` and running
`tools/verify pong` prints exactly the message above, inside an otherwise green
run.

**Two ways to fix it**, recorded before either was taken: a fourth variant with
its own cause and fix, which is the honest modelling — "there is no device" is not
"the plan asked for too much" — or `Unsupported` carrying its cause and fix
per-site instead of per-variant, which is a smaller change and a weaker taxonomy.
The variant was recommended and, on the maintainer's instruction, taken.

**Fixed as `RenderError::NoAdapter { detail }`.** The message now names the
driver, names `mesa-vulkan-drivers` as the package that supplies a software
rasterizer, and says a run asserting on the draw transcript needs no adapter at
all and should report this as a skip (renderer.md §9). Confirmed by execution
rather than by reading: hiding the ICD and running `pong` prints the new message
inside a still-green run. Two tests, both mutation-checked by giving `NoAdapter`
its old text back and watching them fail —
`a_missing_adapter_is_not_reported_as_a_frame_the_backend_cannot_draw` pins the
message, and `no_two_render_errors_offer_the_same_diagnosis` stops the next
variant being added carrying a copy of its neighbour's advice, which is precisely
the mistake this was.

**The taxonomy asked for this and R1 did not deliver it**, which is the part worth
keeping. renderer.md §10's design bullet reads "No adapter/device at startup →
`Result` … (likely cause: missing drivers/headless env; fix: doctor hints)", and
`RenderError`'s own doc comment has always opened "Environmental: **no adapter**,
a lost device, a surface that vanished". The case was designed, described, and
then implemented into the variant next to it. Nothing caught the gap because a
wrong-but-well-formed four-part message passes every check the project has — the
same shape as F-055, one layer down.

**Two things this does *not* fix.**

- **`Unsupported` is still a grab-bag with a fixed diagnosis.** Its cause and fix
  name the WebGL2 envelope, which is right for exactly one of its remaining
  members (a device request that could not meet the limits) and wrong for the
  other six — the null backend having no pixels, a zero-sized capture target, a
  windowed backend refusing to read its surface back, `read_back` on the web, a
  surface taken twice, an adapter that cannot present. **This is the larger half
  of the finding and it is open.** It needs either more variants or a cause and
  fix carried per site, and that is a wider decision than the one taken here.
  Recorded in renderer.md §10 as well, so a reader who sees one message fixed does
  not conclude the taxonomy is sound.
- ~~**A `--verify` mode still cannot tell a missing adapter from a real
  fault.**~~ **Closed, on the maintainer's instruction, in the same series.**
  `RenderError` was *named* by every `RenderBackend` signature in the generated
  reference and defined nowhere in it — F-017's shape a third time — so an
  example could print the message and not match on it, and both capture paths
  treated every handshake error as "no GPU on this machine". It is now exported by
  `jidousha::testing` with a reference entry of its own, and `pong`'s capture path
  skips on `NoAdapter` and *fails* on anything else. **`prototype_kit`'s still
  conflates them**, along with the four-line message spill below; both are the
  same five-line change and are left rather than editing a verified example in
  passing.

  Worth stating why the export was the right call rather than completeness for
  its own sake: a naming without a definition is not merely untidy, it decides
  what a check can express. The engine had drawn the distinction one commit
  earlier and the only surface that needed it could not see it, so the fix was
  half-delivered until the type came with it.

**Both examples' capture paths quote this message**, which is how it was found: it
arrives inside a one-line `capture:` summary. `pong`'s flattens it onto one line
so the `--verify` summary block keeps one fact per line; `prototype_kit`'s does
not, and spills four lines of somebody else's paragraph into `tools/verify`'s PASS
output. Worth doing to `prototype_kit` too, and left alone rather than editing a
verified example in passing.


## 4c. Run 6 triage — the whole run on one page

Eleven findings, in the order run 6's cost ranks them. **Class** is §1's;
**settled by** names the ADR or `DELIBERATE:` tag that answers the complaint,
where one does.

| # | Finding | Class | Also found by | Settled by | Verdict |
|---|---|---|---|---|---|
| F-068 | the document said the clear colour was unassertable; it is one line | docs | first | **nothing — the sentence was false** | doc fix landed, plus a test that pins the capability |
| F-074 | the controller warning is calibrated only for a guilty controller | docs | the other side of **F-037/F-047/F-056** (runs 1–5) | F-056's contract check, which is what resolved it | doc fix landed — two small edits, deliberately not a fifth rewrite |
| F-069 | a `const` angle in degrees could not be written | **engine** | first | ADR-0009 governs `Radians` and is silent on `const` | **accepted and fixed**; convention now stated so the next newtype cannot miss it |
| F-073 | two ways to get a frame, and the worked example uses the other one | docs | first | **ADR-0026** (the divergence is kept and named at the top) | example header rewritten; the document says it from its side |
| F-077 | nothing says what a headless tick costs | docs | first | — | doc fix landed; **the only friction that changed a design decision** |
| F-071 | `Vec2::lerp` exists and the file calling itself *the* entry omitted it | docs | **F-018** again — the fix for it, measured | — | six operations added, and the file stopped overclaiming completeness |
| F-072 | a game in `examples/` inherits the engine's lints, unannounced | docs | first | — | doc fix landed; four lints named with their fixes |
| F-078 | is `FrameRecorder::draw` the same as the long way underneath? | docs | first | **ADR-0026** | doc fix landed; the answer is yes, exactly |
| F-076 | "one quad per character" does not say whether a space counts | docs | first | — | doc fix landed; it is a contract, and now a test |
| F-075 | mutation testing is recommended without "commit first" | docs | first | — | one clause, in the paragraph that recommends it |
| F-079 | six runs, no window: `--verify` is green and nobody has played it | environment | **runs 1–5** (F-054, F-065) | F-054, half-resolved | **escalated again**, and it is a smaller ask than it was |

Plus one **author** finding that is not new: run 6's opponent was unbeatable by
arithmetic, which is **F-064** a second time in two runs. See below.

**One engine proposal, and it is an acceptance.** F-069 is the first accepted
engine change since run 4's ADRs, and it is as small as an engine finding gets:
`Radians::from_degrees` becomes a `const fn`. It is worth noting *why* five runs
did not find it. Every previous run wrote its bounce angle as a literal or
computed it at runtime; run 6 is the first to want the angle as a named constant
in degrees, and the gap is invisible until somebody does. The rest of the
plain-data surface — `Color::rgb`, `Depth::layer`, `PhysicalSize::new`,
`TextureId::from_bits` — was `const fn` already, so this was a miss rather than a
decision, which is why it is fixed rather than argued and why no ADR is owed.

**F-064 is now at two sightings, and it is deliberately not being promoted.** Run
5 wrote an opponent that read the ball every tick and was unbeatable; run 6 wrote
one that predicted where the ball would cross and was unbeatable. Same class,
different mechanism, and both runs found it themselves. §1's rule says three
`author` findings on one topic is a `docs` finding wearing a hat, so this is one
short — and the reason to wait rather than write the paragraph now is that the two
runs' fixes disagree about what the lesson is. Run 5's answer was a reaction time;
run 6's was "chase where the ball *is*, not where it is going". A paragraph
written from two data points would have to pick one, and picking wrong here costs
what F-056 cost: a worked instruction that steers the next run into a different
ditch. **If run 7 writes an unbeatable opponent, this becomes a `docs` finding and
the paragraph gets written from three.**

**What this triage still cannot settle, stated plainly.** Three things.

1. **F-079, for the sixth run.** The `--verify` mode drives the identical systems
   through `headless`, the capture renders one of its recorded frames through the
   same `WgpuBackend` a window would use, and the PNG looks like Pong. What is
   still unexercised is window creation and the `winit`→`Input` plumbing, which is
   engine code rather than a run's. No run has played its own game. That is now a
   `DISPLAY`-shaped hole rather than a whole-graphics-stack one, which makes it a
   smaller ask than F-054 was — and still not one an agent may take.
2. **`tools/serve-web` has never run in an E0 container.** It needs
   `wasm-bindgen-cli` 0.2.127, matching the lockfile, which is not installed;
   installing a toolchain is on CLAUDE.md's never-agent-fixable list and run 6 was
   right not to. So the web target is CI-gated at `cargo check` and has never been
   *driven* by a run. Recorded under F-079 rather than given a number of its own,
   because it is the same escalation.
3. **Two false sentences in two runs, and the guard is still one test at a time.**
   F-055 and F-068 are the same failure: a document sentence that contradicts the
   code it describes, surviving every gate. Run 5's triage concluded that the
   answer is a test asserting the load-bearing sentence rather than a gate over
   summary quality, and F-068's fix is another such test. That is two, chosen by
   hand, after each was found the expensive way. Nobody has proposed a way to
   *enumerate* the sentences that need one, and this triage does not have one
   either. §6 carries it as the open question it is.

**One thing that was not a finding and is a correction to this file.** `pong` was
in both of `tools/test`'s example lists this time — the trap named in run 5's
triage did not fire, because the miss was caught within a commit of the game
landing rather than at the next run's start. Recording the non-event because two
consecutive runs had hit it and a third would have made it structural.

### Findings from run 6, and from triaging it (F-068–)

Run 6's raw notes are `docs/e0/run-6.md`, and that file is not edited: it is the
record of what one run cost, so a finding that turns out to be wrong is corrected
here rather than there. Where this triage disagrees with the log, this file is the
verdict and the log is the evidence.

### F-068 — The document said the clear colour was unassertable, and it is one line

Class: docs · Run: 6 · Fixed in: this commit · Settled by: nothing — the sentence
was simply false

**What the document said**, in the capture section, about breaking the game on
purpose and looking at the picture:

> Try the clear colour first, because nothing else in this document can see it.

**What is true.** `FrameRecord` has a `pub plan: FramePlan` and `FramePlan` has a
`pub clear_color: Color`. Both entries are in that document's own Reference, six
hundred lines below the sentence that denies them. The assertion is:

```rust
assert_eq!(frame.plan.clear_color, palette::COURT);
```

**Verified against the source before acting**: `camera.rs:49` declares the field,
`plan.rs:206` copies it out of the `Camera` into the plan, `null.rs:87` is the
record that carries the plan. The run's claim holds exactly as written.

**What it cost.** Run 6 believed the sentence and wrote no check. It then broke
its own game seventeen ways, one at a time; sixteen faults were caught and the one
that escaped was the clear colour — the one the document had told it not to
bother with. So this is the second finding in this file (after F-055) caused by a
sentence being *false* rather than *absent*, and the two failed in the same way:
individually plausible, and invisible to every gate the pipeline has.

**The second half of the finding is the more valuable one, and it generalises.**
The naive assertion — plan against the game's own `palette::COURT` — does not
survive `palette::COURT` being changed, because the check and the thing it checks
move together and the mutation walks through. Run 6 wrote that first, watched a
mutation pass it, and replaced it with a pair: the equality *and* a claim the
constant cannot move ("the court is dark enough for a white ball to read
against", spelled in numbers). Only the second catches the constant changing. That
shape — `assert_eq!(what_was_drawn, the_constant_that_drew_it)` — is everywhere a
game checks its own drawing, and nothing in the document had named it.

**Fix.**

- The false sentence is gone. *Testing your game* now says the clear colour is the
  one part of the picture that leaves no quad behind **and** is still assertable,
  gives the one-line form, and states what the capture answers that the assertion
  does not — and the reverse.
- The trap gets its own paragraph, with the pair written out, generalised past
  colour to any size, position or speed cap checked against the constant that
  produced it.
- `a_recorded_frames_plan_carries_the_cameras_clear_color` pins the capability the
  sentence denied, so the path from a recorded frame to the colour cannot quietly
  stop existing and make the old sentence true again.
- `examples/pong/verify.rs` carries both checks, which is where the wording came
  from.

**What this says about the pipeline.** F-055's conclusion was that no gate over
rendered summaries catches a false sentence, and the guard has to be a test that
asserts the claim. This is the same conclusion reached from the other end: the
sentence here was in hand-written prose rather than in a doc comment, so not even
the generator touched it. The guard is again a test, and it is again named after
the claim rather than after the function.


### F-069 — A `const` angle in degrees could not be written

Class: engine · Run: 6 · Fixed in: this commit · Settled by: nothing — ADR-0009
governs `Radians` and says nothing about `const`

**What run 6 hit.** `Radians::from_degrees` is documented as existing "for
humans", and a game that bounces something has an angle constant. It was not a
`const fn`, so `const MAX_BOUNCE: Radians = Radians::from_degrees(60.0);` did not
compile. The run wrote `Radians(1.0471976)`; clippy rejected it as
`approx_constant`. The spelling that compiled was
`Radians(core::f32::consts::FRAC_PI_3)` — used by nothing in either document, and
unwritable the moment the angle is fifty degrees rather than sixty.

**Verified before acting.** Confirmed against `math.rs`: `from_degrees` was
`pub fn`, one multiplication, no reason beyond nobody having tried. Confirmed the
`const fn` compiles on this toolchain — const float arithmetic has been stable
since 1.82 and the workspace pins 1.94.

**Checked against "one way to do everything", which is the question that matters
here.** Making an existing function `const` adds no second way to do anything: it
is the same function, callable in one more position. The check that would have
failed is the opposite one — a `Radians::DEGREES_60` constant, or a `deg!` macro,
either of which would be a second spelling of an existing call. Neither was
considered for long.

**Checked against the ADR that governs the type.** ADR-0009 decides that `Radians`
is a newtype over `f32`, that std trig is banned, and that the engine owns
deterministic `sin_cos`. `from_degrees` is one multiply by `PI / 180.0` and
touches none of that: it is IEEE multiplication, which the ADR itself names as
bit-exact everywhere, and const evaluation of it produces the same bits as the
runtime call. So nothing in ADR-0009 is reopened and no new ADR is owed — this is
a gap rather than a decision, which is why it is fixed rather than argued.

**Why it was not caught by the rest of the surface.** `Color::rgb`, `Depth::layer`,
`TextureId::from_bits` and `PhysicalSize::new` are all `const fn` already, so the
convention existed and `Radians` was simply the one that missed it. That is now
written down as a convention rather than left as a pattern, which is the part that
protects the *next* newtype.

**Fix.**

- `Radians::from_degrees`, `Radians::to_degrees` and `Radians::as_f32` are
  `const fn`. `Seconds::as_f32` too, for the same reason and by the same argument.
- `conventions.md` §Math states the rule, in the section the generated game
  document carries, with the two bad spellings named so a reader recognises the
  situation.
- `examples/vec2_tour.rs` — the file that presents itself as the entry for this
  vocabulary — carries `const MAX_BOUNCE: Radians = Radians::from_degrees(60.0);`
  and asserts on it.
- `an_angle_in_degrees_can_be_a_const` is the behavioural test, named after the
  thing the run could not do.


### F-070 — `e0-findings.md` was the one citation the scrubber could not see

Class: docs (tooling) · Run: found while triaging 6, not by it · Fixed in: this
commit

Not a run 6 finding: found by this triage, while adding a `conventions.md` bullet
whose rationale cited F-069. **It does not count towards §2's two clean runs, in
either direction.**

`CITATION_RE` strips `(ADR-0010)`, `(core.md §9)` and `(I2)` out of the generated
documents, because the E0 reader may not open any of those and a citation of a
document they cannot read is worse than silence (F-005). Its filename class was
`[a-z-]+\.md`, with no digits — so `(e0-findings.md F-045)` was not a citation as
far as the pattern was concerned. `FORBIDDEN` did not catch it either, because the
entry there is `docs/internal`, the directory, and the citation names the bare
file. Two had accumulated in `docs/api/jidousha-api.md`, both pointing a game
author at the one document in this repository that is *about* game authors failing.

**Fix.** The filename class takes digits, the citation form accepts a `F-045`
suffix as well as a `§9` one, and `e0-findings` joins `FORBIDDEN` so anything
written another way fails the build rather than shipping. Both existing citations
are gone from the generated text and both survive in `conventions.md`, which is
the point of scrubbing on the way out rather than rewording the source.


### F-071 — `Vec2::lerp` exists, and the file that calls itself the entry for `Vec2` did not list it

Class: docs · Run: 6 · Fixed in: this commit · Also found by: **F-018**, whose
fix this is · Settled by: nothing

**What run 6 hit.** `examples/vec2_tour.rs` opens by saying the reference cannot
generate an entry for `Vec2` and that "this file is the entry instead". It did not
list `lerp`. Swept collision — which Concepts explicitly sends a game author off
to write themselves (ADR-0022) — needs to turn "the crossing happened 0.4 of the
way through this tick" into a world position, which is one `lerp`. The run could
not tell whether the omission meant the method did not exist, so it wrote
`from + (to - from) * t` and moved on.

**Verified.** `lerp` exists — glam 0.33, `Vec2::lerp(self, rhs, s)` — along with
several other everyday operations the tour omitted. Checked by compiling them:
`distance_squared`, `normalize_or_zero`, `try_normalize`, `signum`, `perp`,
`perp_dot`, `move_towards`, `midpoint`, `project_onto`, `reflect`, `floor`,
`round`, `ceil`, `to_array`, `extend`, `element_sum`, `recip`, `rem_euclid`,
`copysign`, `cmpgt`/`select`, `mul_add`. So the run's finding holds, and the
tour's claim to be *the* entry did not.

**This is F-018's fix being measured, which is the frame to read it in.** Run 2
found `Vec2` documented as "re-exported from glam and documented there" against a
document that says nothing is out of scope; the answer was to make an example the
entry, because the generator has nothing to generate from for a foreign type. That
answer is still right — run 6 used the file, and its complaint is that the file
was incomplete rather than that it should not exist. But the file inherited a
reference's job without a reference's guarantee, and this is what that costs.

**The finding underneath the finding.** The tour also said "cargo compiles it, so
the list cannot drift away from what the type actually offers". That is only true
in one direction: cargo checks that everything listed exists, and can say nothing
about what is missing. The file was making a completeness claim its own guard
could not support, which is the same failure as F-068 and F-055 — a sentence that
is false rather than absent — in a third place.

**Fix.** Both halves, because either alone leaves the same question open.

- Six everyday operations added, each with the game situation it belongs to:
  `lerp` (the swept contact point, named as such), `distance_squared`,
  `normalize_or_zero` **and the NaN it avoids**, `signum`, `perp`,
  `move_towards`.
- The header now states the boundary instead of overclaiming: what cargo checks
  and what it cannot, that the vocabulary is curated by hand, that a gap is a bug
  to report rather than an answer, and where the rest of glam is.

**`normalize_or_zero` is the one worth calling out.** `Vec2::ZERO.normalize()` is
NaN, nothing panics, and NaN spreads through every position after it — a velocity
that reaches exactly zero for one tick is a game that silently stops existing.
No run has hit it yet; it is in the tour now because the tour is where a run
would look.


### F-072 — A game in `examples/` is held to the engine's lints, and neither document said so

Class: docs · Run: 6 · Fixed in: this commit · Settled by: nothing

**What run 6 hit.** `crates/jidousha/Cargo.toml` carries `[lints] workspace = true`,
which applies to example targets, so `cargo clippy --all-targets -- -D warnings`
judged its game by the maintainers' rules. Neither document mentions it. The run
met the rules at the "definition of done" step rather than while writing, and two
of its three clippy failures were in the `--verify` check rather than in the game.

**Verified, by writing a deliberately bad example and compiling it.** The four
that reach an example target are `missing_docs` (a *compile* error, before clippy
runs — the file needs a `//!` header), `unwrap_used` and `expect_used`,
`collapsible_if`, and `approx_constant`. Worth being exact about one thing the
existing comment in `clippy.toml` gets half right: it says examples "are not
covered by these flags", meaning the `allow-unwrap-in-tests` knobs, which is true
— but the `unwrap_used` **lint** does apply, and an example that wants `unwrap`
needs its own `allow`. No example currently does.

**Fix.** A Concepts paragraph, naming all four with the fix for each, and saying
to run clippy while writing rather than at the end. The `approx_constant` entry
points at F-069's `const fn`, because the two findings are the same five minutes
of one run's life.


### F-073 — Two ways to get a frame out of a headless game, and the worked example uses the other one

Class: docs · Run: 6 · Fixed in: this commit · Settled by: **ADR-0026** (the
divergence is kept and named at the top of the file)

**What run 6 hit.** `jidousha-testing.md` prescribes `FrameRecorder::new(viewport)`
then `recorder.draw(&mut sim)`. `examples/prototype_kit/verify.rs` — the file a
run browsing `examples/` takes the `--verify` shape from — instead calls
`sim.draw()`, builds its own `TextureTable`, calls `plan_frame` and
`backend.render`, and reconstructs the font's backend id through a throwaway
`NullBackend`. That is fifteen lines of ceremony against
`jidousha-api.md`'s own opening convention, "One way to do everything".

**Verified.** Both paths are real and both are public. `prototype_kit`'s exists
because `play` takes a `&mut dyn RenderBackend` and the run puts the identical
session through a `NullBackend` and a `WgpuBackend`, asserting the world came out
the same — the only check in the repository that a session is backend-agnostic.
`FrameRecorder` records into a null backend only, so it cannot buy that.

**The run's own account is the useful part**, because it is not "the example is
wrong":

> `prototype_kit` explains *why* it keeps the long way […] That is honest and I
> still lost time: the example is the thing you read to learn the shape, and the
> shape it teaches has fifteen lines of ceremony that the document says a game
> does not need.

So the reasoning existed and was correct, and was two hundred lines below the
code that raises the question, in the doc comment of a private helper — which a
reader meets *after* copying the shape. That is the finding: a `DELIBERATE:` tag
is a defense only where the surprise is, and the surprise was the file's whole
structure.

**Decision: keep it, name it. ADR-0026.** The two alternatives both cost more
than they buy — using the recorder deletes the two-backend comparison, and
splitting the example duplicates a whole game to say one thing twice.

**Fix.**

- `prototype_kit/verify.rs` opens with "One thing here is not the shape to copy":
  the recorder's two calls stated positively first, then what this file does
  instead, which lines are the difference, why they are here, and "read it for
  the checks, not for how to get a frame".
- *Testing your game* says the same from its side, in one paragraph, **naming no
  file** — E0 games are deleted before the next run (`e0-prompt.md` step 2), so a
  permanent document may not cite one, which is the same rule the existing
  `DELIBERATE:` on `textures_font_id` was written for. ADR-0026 settles that one
  too, so both tags now carry the ADR reference practices §1 requires.


### F-074 — The controller warning is calibrated in one direction, and run 6 landed in the other

Class: docs · Run: 6 · Fixed in: this commit · Also found by: the other side of
**F-037/F-047/F-056** (runs 1–5) · Settled by: nothing

**What run 6 hit.** *Testing your game* spends four paragraphs on "a controller
that plays it safe is not a playability test", ending "when a number looks wrong,
suspect the controller first". Run 6 hit the exact described symptom — a 37-touch
rally at 0–0 — and its controller was correct: 18 of 18 approaches met, playing to
win exactly as the section prescribes. Its **game** was the broken one. Its first
opponent predicted where the ball would cross and moved at 17.5 u/s; the run's own
arithmetic, in `docs/e0/run-6.md` §2, shows that opponent crossing the whole
17.2-unit court during the fastest shot the game could produce. Unbeatable by
geometry. No controller could ever have scored.

**This is the first run to land on the other side of that warning**, and it is
worth being precise about what that does and does not say. The warning is not
wrong: four consecutive runs were sent into their game's constants by a bad
driver, and F-056's fourth attempt at the paragraph is the response to that. What
run 6 shows is that the advice is *undischargeable* on its own. "Suspect the
controller" tells you where to look and never tells you when to stop looking, and
a correct controller and a broken one produce the same 0–0.

**What resolved it in one step is already in the document, two paragraphs later.**
F-056's fix asks the controller to assert its own contract on the numbers it
picked. Run 6 did that, and `met 18 of 18 approaches` is a controller reporting
itself healthy — which is what let the run stop suspecting its driver and go do
arithmetic instead. The run's own recommendation:

> the self-check is not a nice extra, it is what makes the warning actionable in
> both directions.

**Fix.** Two edits, both small, and deliberately not a rewrite of a paragraph that
is on its fourth attempt.

- The "suspect the controller first" sentence now ends with "and suspect it
  *once*", pointing at the contract check three paragraphs below and saying
  plainly that without it the advice cannot be discharged.
- The contract-check paragraph gains the other direction: run 6's symptom, its
  healthy controller, the broken game underneath, and the statement that one run
  of that check tells you which half of the program to open.

**This is evidence about F-056's fourth attempt, and it is good.** Run 4 predicted
in writing that if a fifth run mis-tuned its game because its driver was wrong,
prose had failed and the lever — a worked controller in a game unlike Pong —
should be spent. Run 6 did not do that. It wrote the contract check, its
`--verify` file contains an assertion about its own controller, and the check is
what stopped the search. **The lever stays unspent.** §6 carries this forward.


### F-075 — The document recommends breaking the game on purpose and not committing first

Class: docs · Run: 6 · Fixed in: this commit · Settled by: nothing

Mutation testing arrived in the document as F-058's fix, and run 6 calls it "the
most valuable thing either document recommends" — seventeen faults injected,
seventeen caught after the checks were tightened. The paragraph does not say to
commit first, and the natural revert, `git checkout -- <file>`, destroys every
uncommitted change in that file. It ate one of run 6's checks twice: the check
written to catch the fault being injected lived in the same file as the fault.

One clause, in the same paragraph, since it is a property of the technique the
document is recommending rather than general git advice: commit, mutate, revert,
repeat.


### F-076 — "One quad per character" does not say whether a space is a character

Class: docs · Run: 6 · Fixed in: this commit · Settled by: nothing

Run 6 wanted an exact glyph count on its hint line and could not tell from
"`ctx.text` submits one quad per character" whether a space counted. It wrote a
weaker assertion — combined bounds plus `width_of` — rather than guess, which cost
it nothing (that check is better anyway) and left the question open. It then
measured the answer out of its own run's numbers: a rally frame is 61 quads =
2 walls + 13 dashes + 2 paddles + 16 ball wedges + 2 score digits + **26** hint
glyphs, and `"W / S to move - first to 5"` is 26 characters including six spaces.

**Verified in the source, and it is a contract rather than an accident.** `layout`
skips `\n` and pushes a glyph for every other character; space is one of the
ninety-five printable ASCII characters the atlas covers, with a blank cell and the
same `size * 7 / 9` advance as any letter. The `TextStyle` doc comment already
says "monospace over the ninety-five printable ASCII characters, space through
`~`, every one of them advancing 7/9 of `size`" — so the fact was documented one
level down and the sentence a game author reads was the ambiguous one.

**Fix.** Concepts says "one quad per character, **spaces included**", names `\n`
as the only exception, gives the worked count, and says plainly that it is a
contract an exact assertion may be written against.
`a_space_is_a_glyph_and_a_newline_is_not` pins both halves.


### F-077 — Nothing says what a headless tick costs, and run 6 nearly designed around it

Class: docs · Run: 6 · Fixed in: this commit · Settled by: nothing

**The one friction in run 6 that changed a design decision.** Once its opponent
chased the ball rather than predicting it, there was no closed form for where that
opponent would be, so the run's controller had to roll the game forward tick by
tick — thirteen candidate shots, up to four hundred ticks each, per decision. It
expected that to be too slow. It is not: the whole `--verify` run, 2,013 ticks of
match plus two idle runs plus three staged screens plus a GPU capture, takes 2.3
seconds in a debug build. In the run's words, "I nearly designed around a cost
that is not there."

**Verified independently**: timed at 2.2 s on this machine, same debug build. The
number is worth carrying rather than a per-tick figure, because it is the number
a reader can reproduce with one command.

**Why the absence bites specifically here.** Every other cost question a game
author has is about a shipped game, where the answer is "look at a frame budget".
This one is about a *check*, where there is no frame budget at all and the only
reference point a reader has is the intuition that simulation is expensive. The
consequence is not a slow run — it is a controller that solves in closed form what
it could simulate, which then has to be kept in step with the game by hand, which
is F-056's failure mode wearing yet another hat.

**Fix.** A paragraph beside the `headless` snippet: a tick is the systems you
wrote and nothing else, the measured aggregate as an anchor, the permission stated
plainly ("simulate rather than solve"), and why the closed form is the worse of
the two.

### F-078 — Whether `FrameRecorder::draw` and the long way are the same underneath

Class: docs · Run: 6 · Fixed in: this commit · Settled by: **ADR-0026** (which
keeps the long way in one example)

Second on run 6's list of things it wanted to look up in the source and did not.
Having found two ways to get a frame (F-073), it wanted to know whether the
recorder was doing something the hand-driven path was not — which is the question
that decides whether the ceremony in `prototype_kit` is *buying* anything.

**The answer is no, and it is four lines of `record.rs`**: `draw` takes the game's
`Camera` with the recorder's viewport substituted, calls `sim.draw()`, calls
`plan_frame`, hands the plan to its recording backend and returns the kept frame.
Identical submissions, identical plan, identical arithmetic.

That is a fact a game author needs and cannot get: it is the difference between
"the short way is a convenience I might be trading something for" and "the short
way is the long way, done for me". *Testing your game* now says it in one clause,
where it distinguishes the two roads.


### F-079 — Six runs, and nobody has played the game

Class: environment · Run: 6 · Also found by: **runs 1–5** (F-054, F-065) ·
Fixed in: not fixed — escalated

**Half of F-054 is resolved and this is the other half.** The session hook
installs a software rasterizer, so run 6 is the first that could capture a frame:
`tools/verify pong` wrote a 640×360 PNG, the run looked at it, and it looks like
Pong. That is a genuine change in what E0 can measure and it is recorded as such.

What is still missing is a display. `cargo run -p jidousha --example pong` reports
`RunError::NoDisplay` — with a four-part message that names `headless` as the fix,
which the run singled out as good — so the windowed path has never been executed
by any run. What that leaves unexercised is precisely: window creation, and the
`winit`→`Input` plumbing between a real keyboard and a tick's `InputSnapshot`.
Both are engine code rather than a run's, and both are the part of "playable" that
`--verify` structurally cannot reach.

**Also unexercised: the web target.** `tools/serve-web pong --check` would drive
the game in a real browser. It needs `wasm-bindgen-cli` 0.2.127 — the lockfile's
version, checked — which is not installed in the container. Run 6 declined to
install a toolchain to route around it, which is exactly right per CLAUDE.md. So
wasm is gated at `cargo check --target wasm32-unknown-unknown` in CI and has never
been *run* by anybody.

**Escalated, and it is a smaller ask than F-054 was.** A `DISPLAY` in the session
(Xvfb, or a hosted browser for the wasm half) is the whole of it. Verified in this
triage: `tools/doctor` reports `ENV_OK` with `graphics: no DISPLAY/WAYLAND_DISPLAY
— headless`, and `wasm-bindgen` is absent from `PATH`.


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

### What run 4 answered about run 3's fixes

Run 4 is the second uncontaminated measurement, and it read no other run's log.

- **The third kind of gap is *not* closed — and §6 said what that means.** Run 3's
  findings were all "the document does not say what this *does*". Run 4 produced
  eleven more of the same shape about `ctx.circle`, `size`, `alpha`, `layer`, the
  `--verify` convention and the `sin_cos` spelling. The prediction below was
  written before run 4 ran:

  > If run 4 produces a fourth batch of the same shape about three different
  > behaviours, the problem is not the sentences — it is that nothing in the
  > pipeline asks "does the reference state what this *does*, not just what it
  > *is*".

  It did, about six. **So the sentences are not the problem and the pipeline is.**
  Four of run 4's eleven trace to one mechanism: `first_sentence` carries the first
  sentence only, truncated at 68 characters for a member line, and in every case
  the fact a game author needed was in the *body* of a doc comment that was
  already correct. `Submit::circle` said "made of a fixed number of straight
  edges" and the reference printed "Fill a circle." `Time::alpha` explained itself
  in four lines and the reference printed a truncated clause. The generator is not
  losing information by accident — it is doing what it was built to do — but
  nothing checks whether the sentence it keeps is the one that matters. That is
  the next piece of generator work and it now outranks F-017's export gate.
- **The specific run-3 fixes all landed and were all used.** F-030's font clause:
  quoted by name. F-031's `7/9` advance: used, and cross-checked against the
  transcript. F-032's unreached screens: **called "exactly right and I would not
  have thought of it"**, and it caught five screens the match never reached
  including one control that only exists there. F-033's empty-world paragraph: no
  panic on tick 1, for the first time in three runs that had one. F-034's
  tunnelling paragraph: read, taken, and asserted against the engine's own
  `fixed_dt` as the paragraph advises. F-035's rates ruling: per-second constants
  throughout, no per-tick anywhere. **These are the things not to regress.**
- **F-029's pair keeps paying, and one half of it paid the largest single dividend
  in four runs.** The run's F7 is unprompted praise for "report the numbers a
  condition looked at": a sign error that every structural assertion passed was
  caught by the one assertion that printed a quantity, and the run says the
  paragraph "paid for itself on the first failure" and is "easy to read as style
  advice". That is the second run to say so and it is the strongest evidence in
  this file for what belongs in the `make-game` skill.
- **F-037 is the one run-3 fix that did not hold**, and F-047 is why: the prose
  closed the loud failure and the quiet one cost this run six tuning runs.
- **F-010's fix is confirmed twice over.** `FrameRecorder` was used throughout, the
  apology comment is gone from the game, and `font_texture()` was read out before
  the loop. What the run found instead is that the recorder's *own* shape does not
  compose with itself (F-040) — a smaller problem than the one F-010 fixed, and
  found only because the fix put the recorder in the middle of everything.
- **F-011 is still unverified by a run, for the fourth time**, and F-054 promotes it
  from bad luck to a standing property of the harness.

### What run 5 answered about run 4's fixes

Run 5 is the third uncontaminated measurement, and it read no other run's log.

- **The controller trap cost a fifth run, exactly as predicted, and the prediction
  was right about the fact and wrong about the mechanism.** The watch list below
  said a run that mis-tuned its game because its driver was wrong would mean prose
  had failed three times. It did: run 5 changed three speed constants and added a
  difficulty knob before finding the fault in its own planner, having read the
  warning that morning. But the *shape* is new — F-047's fix worked, and its own
  worked phrase ("try every return this paddle can produce, take the one that
  lands furthest from the middle") is what steered the run onto the boundary of
  its paddle's feasible set. This is a controller too greedy where every previous
  sighting was one too timid, reporting the identical symptom. See F-056.
- **Nine of run 4's eleven doc fixes were used, and three by name.** F-039's circle
  paragraph: the worked union-of-wedges assertion is "copied almost verbatim" and
  the run says without it its ball check would have been false for every circle
  ever drawn. F-044's printable-ASCII check: taken, over every literal. F-046's
  `--verify` convention: implemented, including the `verified ` prefix — and it is
  what made F-055 invisible, which is a cost of the fix rather than a fault in it.
  F-043's `\n`-and-nothing-else metric, F-048's `alpha`, F-049's layer bands,
  F-053's "on screen is not in the right place": all used without comment, which is
  what a landed fix looks like.
- **All three of run 4's ADRs landed for a reader, and the sweep one landed
  hardest.** ADR-0021: the off-screen check is one `contains_rect` call, and the
  run used `contains` and `contains_rect` for the two different questions "on the
  strength of one paragraph" — so the trap ADR-0021 documents against was not
  walked into. ADR-0022: **"Told me what to write and why the helper is absent […]
  I wrote the eight lines without ever wondering whether I was missing an API."**
  Three runs inferred that boundary and the fourth read it, which is the outcome
  that ADR was written for. ADR-0023: the recorder was used throughout with no
  borrow complaint and no second recorder anywhere — a fix that lands silently,
  which is the only evidence available for that kind.
- **F-036's export gate is closed in practice.** `Batch` is defined in the
  reference now and run 5 read `DrawnQuad`'s fields off the document without
  remarking on it. No generator dropout was reported for the third run running.
- **The generator question needs restating, because F-055 is a worse case than the
  one run 4 diagnosed.** Run 4 concluded that nothing in the pipeline asks whether
  the summary it keeps is the sentence that matters. F-055 is one step earlier: the
  sentence was **false**, and being false it was carried faithfully into the
  reference and then paraphrased by hand into the prose, where it became false in
  two places at once. No gate over rendered summaries catches a true-shaped lie.
  What catches it is a test that asserts the sentence — which is what F-055's guard
  now is, and which is a cheaper and more general answer than the summary-quality
  gate run 4 proposed. **That gate is accordingly no longer the next piece of
  generator work**; asserting the load-bearing sentences is.
- **F-054 is unresolved for the fifth run and the fifth run's game is unplayed.**
  F-065 records it. The fix has been known and one line long since run 4.

### What run 6 answered about run 5's fixes

Run 6 is the fourth uncontaminated measurement, and it read no other run's log.

- **F-056's fourth attempt worked, and the prediction on the record is discharged
  in the run's favour.** Run 5's watch list said the specific thing to look for
  was "not whether the run *reads* the paragraph but whether its `--verify` file
  contains an assertion about its own controller", and that if run 6 changed a
  game constant before checking its own driver the lever should be spent. It did
  neither of the bad things: it wrote the contract check, its summary carries
  `controller: met 13/13 approaches (100%), aim landed 13/13 (100%)`, and it went
  into its game's constants **twice** — both times correctly, and both times
  *after* the check had said the driver was healthy. Its own words: "that check is
  worth the twenty lines it costs." **The lever stays unspent.** See F-074, which
  is the same paragraph failing in the opposite direction and is a much cheaper
  fix than the lever would have been.
- **"Constrain, then optimise" was enough, and the search did not have to be
  worked.** The second thing run 5's list asked. Run 6 wrote exactly the
  prescribed shape — "constrain first (only contact points well inside the paddle,
  only positions reachable in time), then optimise (push each survivor through the
  game's own bounce and take the one landing furthest from the opponent)" — from
  the prose, with no code to copy, and it worked first time. The three lines do not
  need writing out.
- **The ordering vocabulary was found and used.** ADR-0024 declined a `Depth` on
  `DrawnQuad` and documented the capability instead. Run 6 asserted on draw order
  — "the score is not painted behind the play" is in its mutation table, caught —
  and got there from `quads()`. No run 6 finding asks for a layer field. The
  decline holds.
- **The two `transcript` methods stayed apart.** F-055's guard was a test, and the
  thing to watch was whether run 6's `--verify` output was a sane number of lines.
  It is: the run prints one frame's transcript as evidence, and nothing in its log
  mentions the recorder's. A fix that lands silently.
- **The accumulating check was copied.** F-061 changed `prototype_kit` rather than
  the document, and run 6's `--verify` collects failures rather than exiting on the
  first — its log calls this out by name as one of the six things the documents got
  right, with a worked instance: "a single deliberate break produced runs reporting
  three faults at once, and in one of them the precisely diagnostic line was
  third." The example is where readers take their shape from, confirmed.
- **The second API document was found immediately, and the split reads as
  intended.** ADR-0025's open question was whether a run would notice a second file
  exists. Run 6 names both in the first paragraph of its log — "What I read:
  `docs/api/jidousha-api.md`, `docs/api/jidousha-testing.md`" — and its findings
  are correctly addressed to one document or the other throughout, including
  F-073, which is *about* the two documents disagreeing. **Discoverability was not
  underpaid.** The case for splitting again when the next subsystem lands is open.
- **Being able to see changed what the run found, and less than expected.** F-054's
  resolution was supposed to be a new kind of measurement. Run 6 shipped the
  capture path — the first run to write and execute one — from the corrected
  instructions rather than by inventing it from a reference block, which is what
  F-066's fix was for and is the answer to that question. But the *findings* are
  the same shape as five runs of blind ones: nine of eleven are sentences the
  document does not carry. The picture confirmed the game rather than revealing
  anything, and the one background fault it could have caught was caught by an
  assertion instead (F-068). Worth stating because the opposite was predicted here
  in writing.
- **F-054 is half-resolved and F-079 is the other half**, which is the sixth run
  with nobody having played the game.

### What run 7 should be watched for

- **Whether a false sentence is found for the third run running.** F-055 and F-068
  are the same failure — a document sentence that contradicts the code it
  describes — and the guard for each is a test asserting the claim, written after
  the fact. Two is a pattern; three would say the review process is the problem
  rather than any individual sentence. The thing to look for is not whether run 7
  reports one, but whether the one it reports is again in a *paragraph* rather than
  in a generated summary: both so far were prose a human wrote, and the generator
  carried them faithfully. If the third is too, the next move is a pass over
  *Testing your game* asking of each load-bearing claim "what test asserts this?"
  rather than any further tooling.
- **Whether F-074's edit cuts both ways.** F-056's paragraph has now been rewritten
  four times for a guilty controller and once, minimally, for an innocent one. A
  run 7 that hits a degenerate rally and spends a cycle suspecting its controller
  *after* its own contract check reported healthy means the pointer forward is in
  the wrong place, and the fix is to move the contract check above the warning
  rather than to add a fifth paragraph. A run that reads `met N of N` and goes
  straight to its game means the edit worked.
- **Whether an unbeatable opponent appears a third time.** F-064, twice now, both
  times self-diagnosed and both times costing the run its only blocked hour. §4c
  says why the paragraph is deliberately not written yet, and what a third sighting
  would license: writing it from three data points instead of two, so it can say
  what the shape of the mistake is rather than what one run's fix was.
- **Whether the `const` angle is used.** F-069 is a fix a run only benefits from if
  it reaches for the constant. A run 7 whose bounce limit is
  `Radians::from_degrees(...)` in a `const` says so directly; one that writes a
  radian literal again says the convention line in the game document is not where
  a reader meets the question, and the next home is the Concepts paragraph on
  angles.
- **Whether `vec2_tour` is now believed.** F-071's fix was half a completeness
  claim withdrawn and half six operations added. The failure mode to watch is a run
  that hand-writes something the file now lists — that would mean the tour is not
  being read as a reference at all, and the entry belongs in the generated document
  rather than in an example.
- **Whether the lint paragraph arrives before the lints do.** F-072's fix is one
  Concepts paragraph. It works if run 7's log does not mention clippy, or mentions
  it while writing rather than at the "definition of done" step. It has failed if a
  run again meets `-D warnings` for the first time at the end.
- **Whether `prototype_kit`'s header is read before its body.** ADR-0026 bet that a
  reader who starts at the top of a file leaves with the right shape. A run 7 that
  uses `FrameRecorder` and says nothing about the long way is the pass; one that
  copies the hand-driven path anyway means a header is not enough and the example
  has to be split after all.
- **Whether anything in these fixes reads as an invitation to guess.** Same
  standard as before: a fix is only real if the next run does not have to infer the
  thing it fixed.

### What run 6 was watched for

*Kept exactly as written before run 6 ran, because a prediction is only worth
anything if it is not edited afterwards. The verdicts are in "What run 6 answered"
above.*

- **Whether the controller paragraph works on its fourth attempt.** F-056's fix is
  the first that hands the reader something to *run* — assert the controller's own
  contract, on the numbers it picked, every tick — rather than something to
  remember. It is also the fourth attempt at one paragraph, against a failure four
  runs have now hit. **The lever named in run 4's list is deliberately not spent**:
  a worked controller in a game deliberately unlike Pong costs the exercise
  something (F-020), and this attempt differs in kind rather than in wording, so it
  earns one measurement. If run 6 changes a game constant before checking its own
  driver, spend it. That is a prediction on the record, and the specific thing to
  look for is not whether the run *reads* the paragraph but whether its `--verify`
  file contains an assertion about its own controller.
- **Whether "constrain, then optimise" is enough, or whether the search itself has
  to be worked.** F-056 states the fix in prose and points at no code. A run 6 that
  writes a greedy search anyway, or that constrains to a margin so wide it never
  aims, says the paragraph needs the three lines written out the way F-039's disc
  assertion is.
- **Whether the ordering vocabulary is found.** ADR-0024 declined a field and
  documented a capability instead, which is the same shape as ADR-0022's sweep. The
  test is the same too: does run 6 assert on draw order at all, and does it get
  there from `quads()`'s own entry rather than by asking for a `Depth`? A run that
  again concludes ordering is uncheckable means the sentence is in the wrong place,
  and the next move is the worked assertion in *Testing your game* rather than the
  field.
- **Whether the two `transcript` methods stay apart.** F-055 is the first finding
  in this file caused by a documented sentence being false rather than absent, and
  the guard is a test rather than a gate. The thing to watch is whether run 6's
  `--verify` output is a sane number of lines — if a run prints a hundred thousand
  again, the descriptions were not the problem and the shape is.
- **Whether the accumulating check gets written.** F-061 changed the example rather
  than the document, so a run 6 that copies `prototype_kit` now copies the right
  shape. If its `--verify` still exits on the first fault, the skeleton is where
  readers take their shape from and the paragraph beside it is not being read.
- **Whether anything a run needed was in a doc comment's *body*.** Run 4's fixes
  moved several facts into first sentences; F-060 moved another. The reference
  prints one line per member, so any fact that has to survive is a fact that has to
  fit in sixty-eight characters. A run 6 finding of the form "the reference told me
  half of it" is evidence that the member line is the wrong home and Concepts is
  the right one.
- **Whether run 6 can see its game — the first run that can.** F-054 is resolved:
  the session hook installs the rasterizer, so `tools/verify` captures a frame and
  the golden tier runs. Three things to watch, and they are new questions rather
  than the old one. Does run 6 *notice* it can capture, from `tools/verify`'s output
  alone? Does it ship the capture path for its own `pong`, which no run has been
  able to write and execute? **What that second question measures changed after
  this was written, on purpose:** F-066 found that the one sentence on the subject
  said `tools/verify` captured the picture itself, which is false and is what run 5
  read; *Testing your game* now says the picture is the example's to take, and how.
  So run 6 is being asked whether corrected instructions are enough, not whether it
  can invent the path from a reference block. And does being able to look change
  what it finds —
  five runs of findings are the findings of authors reading numbers, and a run that
  can see a still frame may report a different kind of friction entirely. **The
  window is still absent**, so "playable" remains a human's judgement.
- **Whether run 6 finds the second API document at all.** ADR-0025 split
  `docs/api/` in two by what the reader is doing, so the run writes its game from
  `jidousha-api.md` and its `--verify` mode from `jidousha-testing.md`. Nothing
  was added or taken away — the same prose, the same entries — but a run now has
  to notice a second file exists. Three pointers say so (the game document's
  header, the Reference group where the testing signatures used to be, and the
  section where the prose used to be) and `e0-prompt.md`'s may-read list names
  both. **A run that writes a `--verify` mode without ever opening
  `jidousha-testing.md` is a finding about the split, not about the run**, and
  the first evidence that discoverability was underpaid. The opposite result —
  a run that finds it immediately and says the game document felt focused — is
  the case for splitting again when the next subsystem lands.
- **Whether anything in these fixes reads as an invitation to guess.** Same standard
  as before: a fix is only real if the next run does not have to infer the thing it
  fixed.

### What run 5 was watched for

*Kept exactly as written before run 5 ran, because a prediction is only worth
anything if it is not edited afterwards. The verdicts are in "What run 5 answered"
above.*

- **Whether the generator asks the right question.** The finding above is the
  headline of this run. If run 5 reports another behaviour the reference states as
  a noun rather than a verb, the fix is not another sentence — it is a gate that
  reads each rendered summary and asks whether the item's own doc comment says
  something the summary dropped. Watch specifically for the 68-character member
  lines: three of run 4's were within two characters of the limit.
- **Whether the controller trap costs a fifth run.** F-047's fix is the third
  attempt at prose and it is a prediction on the record: if run 5 mis-tunes a game
  because its driver was wrong, prose has failed three times and the answer is the
  worked controller in a game deliberately unlike Pong. That is the last lever and
  spending it costs the exercise something (F-020), so it should not be spent
  early.
- **Whether the three decided ADRs actually land for a reader.** They are applied,
  not merely written, so run 5 is the measurement. Three specific things to look
  for. Does the off-screen assertion get written as one `contains_rect` call, or
  does the run hand-roll four comparisons anyway because it did not notice the
  method? Does it use `Rect::contains` for that check and get a false failure on a
  quad flush against the camera's edge — the trap ADR-0021 accepts and documents
  against? And does it read Concepts' declined-sweep paragraph and write the eight
  lines, or reach for a `Rect::sweep` that is not there and conclude, as three runs
  have, that nobody considered it? A boundary that has to be inferred a fourth time
  is a boundary in the wrong place.
- **Whether run 5 can see its game.** F-054 is an environment escalation, not a code
  change, and it is the only finding in this file whose fix would change what every
  future run can *observe* rather than what it knows. If run 5 runs on the same
  image, its log will say "I never saw the game" for the fifth time — and the fix is
  now known to be one `apt-get install mesa-vulkan-drivers`, because the CI runner
  has had exactly that line the whole time. This is the cheapest un-taken item in
  the file by a wide margin.
- **Whether `ctx.circle` gets used at all.** Run 3 used it and recorded a false
  fact; run 4 used it and lost a cycle. If run 5 draws a square ball, the
  documentation worked and nobody found out — so the thing to check is not whether
  the finding recurs but whether the run's ball is round.
- **Whether anything in these fixes reads as an invitation to guess.** Same standard
  as before: a fix is only real if the next run does not have to infer the thing it
  fixed.

### What run 4 was watched for

*Kept exactly as written before run 4 ran, because a prediction is only worth
anything if it is not edited afterwards. The verdicts are in "What run 4 answered"
above.*

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

**Run 4 adds one, promotes two, and disqualifies the rest.** The addition is
F-044's real lesson, stated generally: **a failure mode that is only visible is not
a failure mode when nobody can look.** The engine's loud fallback box, the magenta
placeholder and "the transcript is good enough to check a layout by eye" are all
built for an eye, and four runs have had none. That is skill material because it is
advice about writing a test, not about this API. The promotions are F-037/F-047 —
now four sightings, one of them a run that had read the prose and got it wrong
anyway — and F-029's "report the numbers it judged", which run 4 volunteered as
"the single most useful sentence in it" without being asked. Everything else run 4
found is a sentence the reference should carry, and by the argument above none of
those may reach the skill: eleven of sixteen are "the document did not say", and a
skill that taught an agent to work around a thin reference is a skill that removes
the pressure to thicken it.

**Run 5 adds two, and both are about writing a test rather than about this API.**
The first is F-058's general form: **a run only tests the states it reaches, so the
margins a correct game is built on are exactly what nothing exercises.** It
mentions no engine, it generalises past games entirely, and it is the same shape as
F-032 one level up — F-032 is the screens a run never reaches, this is the
contracts. The second is the technique that found it: **mutate the thing you built
and check your verification notices.** Run 5 broke its game seventeen ways, caught
all seventeen, and says two of them only after tightening checks it had written
carefully — and the same technique, applied during this triage, found a hole in
`prototype_kit`'s own paddle check that had survived every review since R3. A
habit that finds bugs in the engine's worked examples is a habit worth teaching.

F-056 is the harder call and it is **held back**, despite being the most expensive
finding of the run. "Constrain to what you can actually do, then optimise" is
general — it is not about this engine, and it would be true of a driving game or a
fighting game, which is exactly the test §7 applies. But it is also the fourth
prose attempt at a paragraph that is currently *in* the API document, and putting
it in the skill as well would mean two homes for one sentence and no way to tell
which one a future run read. It goes to the skill if and only if the document's
fourth attempt fails, and then it goes as the worked controller rather than as
prose (see §6).

Everything else run 5 found is a sentence the reference should carry, and by the
argument above none of those may reach the skill: seven of eleven are "the
document did not say", including the one that mattered most.

**Run 6 adds one, and it is the strongest candidate since F-032.** From F-068:
**a check that compares what was produced against the constant that produced it
moves with the thing it is checking, and catches nothing when that constant is
what changes.** Run 6 wrote `assert_eq!(plan.clear_color, palette::COURT)`,
watched a mutation of `palette::COURT` walk straight through it, and replaced it
with a pair — the equality, plus a claim about the game spelled in numbers the
constant cannot move. The equality still earns its place: it catches the camera
being set from the *wrong* constant. What it cannot do alone is survive the right
constant becoming wrong. That is a sentence about writing tests, it mentions no
engine, and it is true of a size, a position, a speed cap or a colour. It goes in
the skill.

**And a rider on run 5's mutation habit, from F-075**: commit before you break the
game, because the natural revert takes uncommitted work with it. That is not a
second lesson — it is the first sentence of the one run 5 contributed, and it
belongs in the same paragraph rather than in a list of its own.

**Nothing else run 6 found may reach the skill**, by the argument above: nine of
its eleven are sentences the reference should carry, including the one that cost
it a mutation round. F-074 is the interesting near-miss — "your controller's
self-check clears it as fast as it convicts it" generalises perfectly — but it is
one sentence of F-056's paragraph, which is held back for exactly the reason that
paragraph is: two homes for one lesson and no way to tell which a future run read.
It goes to the skill if and when F-056 does.

**Two of run 2's findings are skill material, and they are the two that are not
about this engine at all.** F-029's pair — that a failing assertion has to
report the numbers it judged, and that "nothing is drawn outside the camera" is
the first assertion a shapes-and-text game should write — generalise to any
game an agent writes without being able to look at it. They are in `testing.md`
because run 3 needs them; they belong in the skill because every run after that
does too.
