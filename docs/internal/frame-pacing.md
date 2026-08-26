# Frame pacing on the web — an open investigation

Status: **defect characterised, mechanism still open; one reading outstanding.**
This note holds one observed defect, the ranked hypotheses for it, the
instrument built to tell them apart, three rounds of owner readings, and what
they have and have not settled (§5). Settled: what the defect *is*, that all
three §3 hypotheses are wrong, and — from two experiments that vary different
things — that **the cost tracks window size, not canvas resolution** (§5.5). Not
settled: the mechanism. Two candidates have been proposed and both refuted, and
the surviving three are in §5.5. **This note closes when §5.6's one reading
lands** — not before, and it says so rather than guessing.

Inherits: the frame-clock contract (ADR-0005), the loop and its catch-up bound
(core.md §7), the playtest page shell (web-publish.md §2), no retained render
state (renderer.md §2).

---

## 1. The observation

Owner, **2026-08-23**, on the deployed builds:

> Pong's ball visibly **jumps forward** instead of moving smoothly in **Firefox
> on Linux**. **Chrome on the same Linux machine is smooth.** The display is
> 120Hz (VRR-capable, though browsers on Linux do not drive pages at variable
> refresh).

Two halves matter equally. 120Hz is an integer multiple of the 60Hz tick, so
uninterpolated rendering shows each simulated state exactly twice at even
pacing — which is what "Chrome is fine" means, and it is the control. Firefox
being jumpy **on the same display and the same page** therefore says its frames
are not pacing the ticks evenly; it does not say why.

(The overlay later revised the "120Hz" in that report: Chrome presented at
~238.1 Hz, so the display was in a ~240Hz mode on the day of the readings — §5.1.
It changes nothing about the reasoning above, which only needs the refresh to be
a multiple of the tick, and it is left in the quote because that is what was
reported before anything was measured.)

## 2. Why a jump rather than a stutter

The engine draws committed state once per rendered frame and runs whole ticks in
between (core.md §7). Frames that arrive unevenly produce, from the accumulator,
a sequence like 1, 1, 0, 2, 1, 1, 0, 2 — and a 2-tick frame moves the ball twice
as far as its neighbours in the same amount of screen time. The eye reads that
as a jump forward, not as a dropped frame, which is exactly the word the owner
used and is worth taking as evidence rather than as loose phrasing.

Note what is *not* wrong: the simulation. Ticks are the same length and the same
count either way; what varies is how many of them land between two presented
frames. This is a presentation defect, and §4 is what was done about it before
the readings landed. §5 is what the readings turned out to say — which is that
the appearance was the smaller half of the problem.

## 3. The hypotheses, ranked, and what each looks like on the instrument

Each of these produces a distinguishable signature on the `?frametime=1` overlay
(web-publish.md §2). That is what the overlay was built to show.

1. **Software rendering.** Firefox on Linux falls back to software WebRender and
   software WebGL more readily than Chrome does. A frame the CPU cannot finish
   in time is a long frame, and a long frame is a multi-tick catch-up.
   *Signature:* the overlay's renderer line names llvmpipe / lavapipe /
   SwiftShader, its software warning is up, the delta histogram has a long tail
   past 20ms, and the ticks-per-frame counts show a real share of 2 and 3+.
2. **Timer quantization.** Firefox clamps `performance.now()` to about a
   millisecond by default; Chrome's resolution is ~5µs. A 1ms quantum against a
   16.67ms cadence drifts, and a drifting delta feeds the accumulator an
   occasional 0-tick or 2-tick frame with nothing actually slow behind it.
   *Signature:* the clock line reads "whole milliseconds — coarse", the
   histogram piles onto whole milliseconds and stays tight around the refresh
   period, the renderer is hardware and the warning is absent, and 0-tick and
   2-tick frames appear in roughly equal, small numbers.
3. **rAF / compositor scheduling jitter.** Neither of the above: hardware
   renderer, a clock line reading "sub-millisecond — fine", and the deltas
   themselves uneven.
   *Signature:* a histogram with real spread and no other line explaining it.

These are not exclusive — 1 and 2 can both be true — and the point of the
instrument is that it does not have to be argued about.

## 4. What was done about it before knowing the answer

Two things, because both are right under every hypothesis. §5 grades them: the
instrument is what produced the verdict, and the interpolation is correct and
insufficient — it was never going to fix a browser presenting at 12fps.

- **The instrument.** `?frametime=1` on any deployed build (web-publish.md §2).
  Page-side only: it never calls into the wasm module, so nothing it measures
  can reach a tick.
- **Render interpolation, in the games rather than in the engine.**
  `examples/pong` and `examples/prototype_kit` keep a previous position and
  submit `previous.lerp(current, Time::alpha)` from Draw — the idiom the
  documentation has described since M3 and nothing had ever used
  (e0-findings.md **F-048**). This is the actual fix for the symptom under
  hypotheses 2 and 3, and it substantially softens hypothesis 1: with
  interpolation, a 2-tick frame is drawn at the position the elapsed time
  actually calls for rather than a tick ahead of the last one. **ADR-0041** is
  the one engine change it needed, and it is a value only presentation reads.

What was **not** done: a tighter clamp on ticks-per-frame. One already exists
(`MAX_FRAME = 0.25s`, applied in `FrameClock::frame` and again in
`Simulation::advance` — core.md §7), and tightening it would trade a jump for
simulation time silently falling behind. Interpolation fixes the appearance
without lying about the clock; the clamp's job is the spiral of death, and it is
doing it.

## 5. The verdict — a presentation-path defect in Firefox

Read by the owner on their own machine, **2026-08-24**, on build `8bf8b2c`, from
the `?frametime=1` overlay §4 built for exactly this. Both browsers, same
machine, same display, same page.

### 5.1 The readings

| | **Chrome** (control) | **Firefox** (defect) |
|---|---|---|
| presented | ~238.1 Hz | **~12.0 Hz** |
| median frame | 4.20ms | **83.30ms** |
| mean frame | 4.18ms | 82.15ms |
| spread | 2.60 – 11.20ms over 240 frames | 16.06 – 665.88ms over 155 frames |
| frames ≥ 50ms | — | **94%** |
| ticks/frame | 0:74% 1:26% 2:0% | **3+:94%** |
| clock | sub-millisecond | sub-millisecond |
| renderer string | ANGLE (NVIDIA RTX 5090), unspoofed | "NVIDIA GeForce GTX 980, or similar" |
| webgpu | available | absent (`navigator.gpu` null — §5.3) |

`about:support` → Compositing in Firefox reads **WebRender (hardware)**.

The machine has **hybrid graphics**: a discrete NVIDIA RTX 5090 and an
integrated AMD. Which of them Firefox's WebGL actually runs on was *not*
captured this round and is not knowable from the page. It was captured the round
after, and the answer changed the verdict — **§5.3**.

One more reading, from the owner rather than from the overlay, and it is the one
that points at the mechanism: **shrinking the Firefox window sharply improves
the median frame time. The per-frame cost scales with pixel area.**

### 5.2 What the hypotheses did

**Hypothesis 2 — timer quantization: ruled out.** The clock line reads
"sub-millisecond — fine" in *both* browsers. Firefox's `performance.now()` was
not clamped here, so there is no quantum to drift against and nothing for the
accumulator to answer with a spurious 0-tick or 2-tick frame. The signature §3
predicted — deltas piling onto whole milliseconds, tight around the refresh
period — is absent, and the deltas are neither tight nor whole.

**Hypothesis 1 — software rendering: ruled out for compositing, and
unfalsifiable as stated.** Compositing is hardware WebRender by Firefox's own
account, so the literal claim is false for the compositor. For WebGL the
hypothesis cannot be tested the way §3 proposed to test it, because **Firefox
spoofs the renderer string**: `WEBGL_debug_renderer_info` reported "NVIDIA
GeForce GTX 980, or similar" — an anti-fingerprinting bucket — on a machine
whose GPU is an RTX 5090. That is not a hardware/software answer at all. It is
a *category* the browser picked, and no reading of it can distinguish the two.

§5.3 later established that this is **stock Firefox** — no
`privacy.resistFingerprinting`, no modified `gfx.*` or `webgl.*` preference, no
`user.js` — so the spoofing is the default rather than a hardened profile's
doing. It also answered the underlying question by a route the page does not
have: `about:support` reports WebGL running on the RTX 5090 outright, so the
literal hypothesis is false for WebGL too. The hypothesis was not merely
untestable from the string; it was *wrong*, and only a reading from outside the
page could say so.

**Hypothesis 3 — rAF/compositor scheduling jitter: not what this is.** Jitter
means uneven frames around a healthy median. This median is 83.30ms with 94% of
frames at or past 50ms: the frames are not jittering around the cadence, they
are uniformly nowhere near it.

### 5.3 The machine, from `about:support`

Step 1 of the protocol reported, **2026-08-26**, as a full `about:support` JSON
export rather than a screenshot — which is better, because it carries the
feature log, and the feature log is where the answer was.

**The GPUs, and which one is doing the work:**

| | |
|---|---|
| GPU #1 | **NVIDIA GeForce RTX 5090**, `0x10de:0x2b85`, driver `610.57.4.0` |
| GPU #2 | AMD integrated, `0x1002:0x164e` |
| `isGPU2Active` | **false** |
| `webgl1Renderer` / `webgl2Renderer` | **`NVIDIA Corporation -- NVIDIA GeForce RTX 5090/PCIe/SSE2`** |
| WebGL driver / WSI | `3.2.0 NVIDIA 610.57.04`, `EGL_VENDOR: NVIDIA` |

**The session:** Firefox 154.0 (Arch), KDE, `windowProtocol: wayland`,
`windowLayerManagerType: WebRender`, one window, accelerated.

**The display and the scaling**, which turn out to matter as much as the GPU:

- `Display0: 3840x2160@240Hz scales:1.450151 HDR`
- `graphicsDevicePixelRatios: [1.4634…]` — KDE fractional scaling
- `targetFrameRate: 60`

**The feature log** — the whole finding is three of these rows:

| feature | status | |
|---|---|---|
| `WEBRENDER` | available | hardware, as `about:support` → Compositing already said |
| `DMABUF` | available | |
| `DMABUF_SURFACE_EXPORT` | available | |
| **`DMABUF_WEBGL`** | **blocked** | `FEATURE_FAILURE_BUG_1924578`, "Blocklisted by gfxInfo" |
| **`WEBRENDER_COMPOSITOR`** | **blocklisted** | `FEATURE_FAILURE_WEBRENDER_COMPOSITOR_DISABLED` |
| `WEBGL` | available | |
| `WEBGPU` (`navigator.gpu`) | **null** | no adapter, default *or* fallback |

And a fact that changes what §5.2's hypothesis 1 means: **`privacy.
resistFingerprinting` is not set, and not one `gfx.*` or `webgl.*` preference is
modified.** `userJS.exists` is false. This is a stock Firefox. The renderer
string was therefore not spoofed *by a hardened profile* — it is spoofed by
default, on every Firefox, which is a much larger claim than §5.2 was in a
position to make.

### 5.4 The finding

**Firefox presents this canvas at ~12fps, with a per-frame cost that scales
with pixel area, on hardware that presents the identical page at ~238fps in
Chrome.** Nothing about the simulation, the clock, or the page differs between
the two; the display is 3840×2160 at 240Hz and Chrome uses it.

That shape — hardware compositing, a fine clock, a uniformly slow frame, and a
cost proportional to the number of pixels — is a **presentation-path defect in
the copy/readback family**: something between the WebGL drawing buffer and the
composited page is paying per pixel, per frame. §5.3 names it.

#### The mechanism the first round proposed is refuted

**Cross-GPU transfer in the hybrid setup is dead**, and §5.3 is what killed it.
`isGPU2Active` is false and **both WebGL versions report the RTX 5090**: WebGL
and compositing are already on the same discrete adapter. There is no bus to
copy across. It was a reasonable candidate from the overlay alone — hybrid
graphics were in the room — and the readings that could see the machine said no.

That also **retires step 2 of the protocol before it was run**: launching with
`__NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia` would ask for the
state Firefox is already in. Recorded rather than deleted, because "we did not
run it" and "running it could not have told us anything" are different things
and the second one is the true one.

#### The mechanism the feature log gives instead

**`DMABUF_WEBGL` is blocked by Mozilla's own blocklist** — `gfxInfo`,
`FEATURE_FAILURE_BUG_1924578` — on this driver, while plain `DMABUF` and
`DMABUF_SURFACE_EXPORT` are both available. That single row says the WebGL
drawing buffer **cannot be handed to WebRender as a shared buffer**. What is
left is the slow path: copy the rendered frame out of the WebGL context and back
in for compositing, **every frame, every pixel**.

That is a per-pixel, per-frame copy — which is precisely the shape §5.4's first
paragraph described, arrived at from the other direction. It explains every
reading without needing a second adapter:

- pixel-area scaling (the window-shrink experiment) — a copy costs what it copies;
- hardware compositing and a fine clock — both true, and neither relevant;
- Chrome unaffected — different WebGL→compositor plumbing, not subject to this
  blocklist entry;
- uniformly slow rather than jittery — a fixed cost paid on every frame.

`WEBRENDER_COMPOSITOR` being blocklisted compounds it: with no native-compositor
path, WebRender composites the whole window itself rather than handing surfaces
to KDE, so the copy has nowhere cheap to land.

**And the multiplier is the display.** 3840×2160 at a device pixel ratio of
~1.46 means a near-fullscreen window is several megapixels, every one of them
paying whatever the per-frame tax turns out to be. A 4K screen is where this
defect goes from "measurable" to "12fps". This paragraph survives round 3
unchanged — it was always about *how many pixels*, and round 3 only moved
*which* pixels: the window's, not the canvas's.

**Confidence.** This is a mechanism the machine's own feature log states as
fact — `DMABUF_WEBGL: blocked` is a reading, not an inference. What remains
inferred is that this blocked row is *the* dominant cost rather than one of
several, and §5.5 has the one measurement that would settle it quantitatively.
Treat it as: cause identified, magnitude unconfirmed.

> **Superseded, 2026-08-26.** That measurement was taken and the prediction
> failed: `?renderscale=0.5` changed the median frame time by nothing at all.
> The blocked row is real and is **not** the dominant cost. Read the rest of
> this subsection as the reasoning that was available before round 3, kept
> because the reading it was built on (§5.3) is still good and because two
> refuted mechanisms in a row is worth being able to see. **§5.5 is the current
> state.**

#### Two things that are not the defect

The **ticks/frame** column: 3+ ticks on 94% of frames is the accumulator doing
precisely its job (core.md §7). It is the symptom's mechanism, not its cause.

**Firefox's own `targetFrameRate` is 60** — on a 240Hz display. So Firefox was
never aiming at the display's cadence, and it is missing the target it *did*
aim at by a factor of five. Worth recording for two reasons: it is why the
overlay's estimate of the refresh period landed on ~62Hz rather than ~240Hz in
Firefox, and — since Firefox's own number is 60 — that estimate was **right
about what this browser could actually have achieved**. The tenth-percentile
method (web-publish.md §2) is corroborated by a number it never saw.

#### What was already done, graded

**Interpolation (ADR-0041, §4) is functioning as designed and cannot help
here.** It places the drawn position where the elapsed time actually calls for,
which is the right thing to draw and is why the *shape* of the motion is honest.
But no pacing logic — none, in this engine or any other — makes 12fps smooth.
§4's second item was right under hypotheses 2 and 3 and it stays; it is simply
not the fix for this, because this is not a pacing problem. It is a throughput
problem wearing a pacing problem's symptoms.

**A finding about the instrument.** The overlay's software-rendering warning is
string-based, and §5.3 establishes that the string is spoofed on a **stock**
Firefox — so that warning is unreliable on every default install of the browser
it was most needed on. Fixed rather than noted: the overlay now also carries a
**measured** slow-presentation warning (web-publish.md §2), comparing the
rolling median frame time against the refresh period the browser's own quickest
frames imply, past 2.5× and past 1/30s over 60 frames. On these readings it
fires on Firefox at 5.2× and stays quiet on Chrome at 1.02×. The string check
stays beside it, and where the two disagree the measurement is the one to
believe.

**And a mitigation a playtester can reach.** `?renderscale=0.5`
(web-publish.md §2) renders a quarter of the device pixels and lets the browser
upscale — presentation-only, so world space, the letterbox contract
(games/giri/UI.md §6), input mapping and the simulation are all untouched. It is
opt-in: no deployed build's default behavior changed, and a default cap on
device pixel ratio would be a decision needing an ADR rather than a quiet edit.

> **Corrected, 2026-08-26.** "A mitigation" is what this was landed as and it is
> not what it is *here* — round 3 measured no effect on this defect. It is a
> mitigation where a frame costs per rendered pixel, and on this browser it is
> instead the experiment that proved the cost is elsewhere. §5.5 has both, and
> the overlay now offers it as a test rather than as a fix.

### 5.5 Round 3 — the mitigation did nothing, and that is the finding

Owner, **2026-08-26**, on the PR-69 preview deploy (not production — the change
was not merged, and this is the build that carries it):

> Changing the renderscale doesn't meaningfully affect frame time. The median
> remains around 83ms.

§5.4 predicted ~4×. It got 1×. **The prediction failed, and the mechanism goes
with it.**

#### First, what this is not

Two boring explanations, both eliminated before touching the verdict:

- **The preview was serving the change.** CI run 32923303364 on head `f2a66d0`
  is green, deploy included, and `f2a66d0` carries the `?renderscale=` code from
  `b204ab9`.
- **The parameter works at this machine's device pixel ratio.** The one variable
  in §5.3 never reproduced was the fractional DPR — KDE at ~1.46, where a
  rounding or resize-feedback bug could plausibly hide. Driven locally at
  `--force-device-scale-factor=1.4634146341463414`: the backing store went
  **1174×667 → 587×334**, exactly 0.50×, with the device-pixel box unchanged at
  1174×667. No bug there.
- Still worth one line from the owner: the overlay's own **`scale`** reading from
  the failing run. It reports the backing store as measured, so it is the direct
  witness that the parameter took. Everything below assumes it did; if it reads
  `1.00x`, this section is void and the question is why.

#### What the null result rules out

The two experiments now vary two different things, and only one of them matters:

| experiment | canvas backing-store pixels | window / composited pixels | result |
|---|---|---|---|
| shrink the window (§5.1) | fewer | **fewer** | **much better** |
| `?renderscale=0.5` (here) | **fewer** | same | **no change** |

Read together: **the per-frame cost is not a function of the size of the WebGL
drawing buffer. It is a function of the size of the window.** The browser is
upscaling a quarter-sized buffer to the same window and paying the same price.

That **demotes `DMABUF_WEBGL`**. The blocklist row (§5.3) is still a fact, and a
forced copy of the WebGL buffer is still real — but it is not the dominant cost,
because a copy of a quarter as much data would have shown up. §5.4's confidence
paragraph said this was the branch to take if the prediction failed, and it is
the branch that happened.

#### What is still standing

Three candidates fit "cost proportional to window area, indifferent to canvas
resolution", and this note is **not** going to pick between them by reasoning:

1. **The compositor's own present path.** `WEBRENDER_COMPOSITOR` is blocklisted
   (§5.3), so WebRender composites the whole window itself and presents one
   window-sized framebuffer per frame — over Wayland, on the NVIDIA proprietary
   driver, whose EGL extension list is EGLStream-shaped. If that present is the
   slow step, nothing inside the window can matter.
2. **The WebGL→compositor copy, done at composited size.** `DMABUF_WEBGL`
   blocked still forces a copy; if it happens *after* the upscale, its cost is
   window-sized and `?renderscale=` cannot reach it. This keeps §5.4's row as
   the cause and only changes where it is paid.
3. **The upscale itself.** Blitting 587×334 to a window-sized surface may cost
   what drawing at window size costs, which would make the parameter unable to
   help here *by construction* rather than by pointing elsewhere.

(1) and (3) say the defect is not WebGL-specific. (2) says it is. That is a
sharp, cheap distinction, and §5.6 is how to make it.

#### What this does to the deliverables

**The measured slow-presentation warning is unaffected and was right.** It fires
on this browser at 5.2× and stays quiet on Chrome at 1.02×. Nothing here touches
it.

**`?renderscale=` survives, with its claim corrected.** It was landed as a
mitigation and documented as one; on this defect it mitigates nothing, and that
sentence is now wrong wherever it appears. What it actually is:

- a **mitigation** where a frame's cost is per rendered pixel — measured, on a
  CPU-rasterizing browser at this same fractional DPR, at a median of 50.00ms
  down to 33.40ms, and at DPR 1 turning a 16.50–33.40ms spread with 15% two-tick
  frames into a flat 16.50–16.80ms with 1%;
- a **discriminator**, which is the more valuable half and is only visible in
  hindsight: it moves the backing store and leaves the window alone, so a slow
  browser that shrugs at it has told you the cost is somewhere a smaller canvas
  cannot reach. That is exactly the reading this round produced, and it is worth
  more than the 4× would have been.

The overlay's warning now offers it as a **test** rather than as a fix, and says
what each answer means. Nothing else about the seam changes: it is still opt-in,
still presentation-only, still off by default.

**And the honest note about how this went.** Two mechanisms proposed, two
refuted — the first by a reading from outside the page (§5.3), the second by its
own prediction failing (here). The instrument was right every time; the
inferences drawn from it were not. That is the correct ratio for a note that
insisted on readings over guesses, and the guesses are labelled as such above so
that the third one is read the same way.

### 5.6 Still open — one page that is not this game

Do **not** run another `?renderscale=` variant; that variable is spent. The next
reading has to separate "this browser is slow at WebGL" from "this browser is
slow at presenting a window this size", and it takes one tab:

1. **Open a non-WebGL page at the same window size, on the same display**, and
   watch whether it is also janky — a long text page scrolled continuously is
   enough; anything with no `<canvas>` in it. Then shrink the window and watch
   again.
   - **Ordinary pages are also slow, and shrinking helps** ⇒ candidate 1: the
     defect is the compositor/present path and has nothing to do with WebGL or
     with this engine. That is the end of the engine's involvement, and this note
     closes as "not ours, recorded".
   - **Ordinary pages are fine and only the WebGL page is slow** ⇒ candidate 2
     or 3: the WebGL→compositor handoff, at composited size. `DMABUF_WEBGL` is
     back in the frame and Mozilla bug **1924578** is the thing to read.
2. **Optional, and only if step 1 says "WebGL-specific":** any other WebGL page
   at the same window size — a `?frametime=1` page from another site, or any
   WebGL demo. If those are slow too, it is the browser and not this engine's
   use of it, which is worth knowing before anything is changed here.
3. **Optional, cheap, and useful either way:** the overlay median at two window
   sizes with the `scale` line's device-pixel box at each. Window area is now the
   variable that *does* matter, so two points give its exponent.

Whoever reads that reply writes it into §5.5, picks between the candidates, and
closes this note — or hands it to Mozilla, which for candidate 1 is where it
belongs.

> **Verdict:** _a presentation-path defect in Firefox — ~12fps on hardware that
> does ~238fps in Chrome. Hypothesis 2 is dead; hypothesis 1 is dead for
> compositing and unanswerable from a renderer string that stock Firefox spoofs;
> interpolation was never going to fix it. Two mechanisms have been proposed and
> both refuted: cross-GPU transfer by `about:support` (WebGL is on the RTX
> 5090), and a per-pixel copy of the WebGL buffer by its own prediction failing
> — `?renderscale=0.5` changed nothing. What the two experiments together
> establish is sharper than either: **the cost tracks window size, not canvas
> resolution.** §5.6's one non-WebGL page decides whether this is the engine's
> problem at all._
