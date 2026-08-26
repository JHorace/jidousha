# Frame pacing on the web — a closed investigation, parked

Status: **parked by the owner, 2026-08-26 — not this engine's defect.** Four
rounds of readings (§5) settled what the defect is, killed all three §3
hypotheses, and refuted two proposed mechanisms. One survives every reading: **a
per-frame operation on the WebGL canvas sized by its displayed size rather than
its backing store**, existing at all because Firefox blocklists the zero-copy
WebGL path on this driver (§5.6). Ordinary pages at the same window size are
fine, so it is WebGL-specific and not the compositor.

**Nothing in this engine is at fault and no engine change was made or is
indicated.** What the engine owed this defect was the ability to diagnose it from
a URL on a machine nobody here can see, and §4's instrument plus this branch's
two page-side changes delivered that. **The investigation is parked; the
instrument is live.** §5.7 has the owner's reasoning, the `n = 1` caveat that
would break it, the four triggers that reopen this, and the two readings and one
seam design preserved so a future round resumes rather than restarts.

**Read this note for**: the shape of a browser presentation defect and how the
`?frametime=1` overlay tells one from a pacing bug (§2, §3, §5.6); why the
renderer string cannot be trusted in Firefox even on a stock profile (§5.2,
§5.3); and what `?renderscale=` is actually for (§5.5).

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

## 5. The verdict — a WebGL presentation defect in Firefox, not in the engine

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

> **Refined, round 4 (§5.6).** "Demoted" was half right. The blocklist row is
> not *a per-pixel copy of the drawing buffer* — that reading is dead — but it
> is why a copy exists at all, and the copy is sized by the canvas's displayed
> size rather than its source. So the row is back in the mechanism with its role
> corrected, not out of it.

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

### 5.6 Round 4 — WebGL-specific, and indifferent to the canvas's resolution

Owner, **2026-08-26**, answering §5.5's three asks in one reply:

> No, ordinary pages are fine. […] the overlay's scale confirms renderscale is
> working. I also tested renderscale=0.25, which still has a median frametime of
> ~80ms (no improvement).

Three readings, and each one closes something.

**1. The parameter was working.** The overlay's `scale` line confirmed the
backing store it actually got, which was the direct witness §5.5 asked for. The
null result stands as a real measurement rather than as a possible
misconfiguration, and §5.5 is not void.

**2. `?renderscale=0.25` is also ~80ms.** Not "a little better" — the same. At a
quarter linear scale the canvas is rendering **one sixteenth** of the pixels it
was, and the frame time did not move. The cost is therefore not weakly dependent
on the WebGL drawing buffer's size; it is **independent of it**. Anything
proportional to the number of pixels the game renders is out, at any coefficient.

**3. Ordinary pages are fine at the same window size.** This kills **candidate
1** outright: the compositor's own present path is not slow, because presenting a
window of this size on this machine is something Firefox does perfectly well all
day. Whatever is expensive happens *because there is a WebGL canvas on the page*.

#### What is left is one statement

Candidates 2 and 3 in §5.5 were the WebGL→compositor copy done at composited
size, and the upscale itself. Round 4 does not separate them — and they collapse
into one mechanism anyway, because physically they are the same event:

> **Firefox is doing a per-frame operation on the WebGL canvas whose cost is set
> by the canvas's *displayed* size, not by its backing store — and it does it
> only because the canvas is a WebGL canvas.**

Every reading in this note now falls out of that one sentence:

| reading | why it follows |
|---|---|
| ~12fps in Firefox, ~238fps in Chrome (§5.1) | Chrome's WebGL→compositor plumbing is not on this path |
| cost scales with window area (§5.1) | the canvas fills the page, so displayed size *is* window size |
| `renderscale` 0.5 and 0.25 change nothing (§5.5, here) | the operation is sized by the destination, not the source |
| ordinary pages fine at the same size (here) | they have no WebGL canvas, so the operation never happens |
| hardware WebRender, fine clock (§5.1, §5.3) | neither is involved |
| `DMABUF_WEBGL` blocked (§5.3) | **back in the frame** — it is why the handoff cannot be a zero-copy buffer share, and must be a copy at all |

`DMABUF_WEBGL: blocked` (`FEATURE_FAILURE_BUG_1924578`) is therefore rehabilitated
from §5.5's demotion, with its role corrected: it is not "a per-pixel copy of the
drawing buffer" — that was the version the 0.5 reading refuted — it is **why a
copy exists at all**, and the copy turns out to be sized by where the pixels are
going rather than by where they came from. That distinction is the whole
difference between `?renderscale=` helping and doing nothing, and it took the
null result to see it.

**One methodological caveat, recorded rather than buried.** "Ordinary pages are
fine" is a strong reading but not a perfect control: a text page that is not
scrolling does not repaint every frame, so it is not being asked to do per-frame
work at all. The tightened version is in §5.7 and takes ten seconds; the
conclusion is not expected to move, and it should be checked rather than assumed.

#### What this means for the engine: nothing to fix, and that is the finding

This is a browser defect on a specific configuration — Firefox on Wayland with
the NVIDIA proprietary driver, where Mozilla's own blocklist disables the
zero-copy WebGL path. It is not a bug in this engine, in the frame clock, in the
accumulator, or in the interpolation ADR-0041 added. **No engine change is
indicated, and none should be made.** What the engine owed this defect was the
ability to diagnose it from a URL on a remote machine, and that is what §4's
instrument and the two changes on this branch actually delivered:

- the **measured slow-presentation warning** fires on it correctly (5.2×) where
  the string-based one read like healthy hardware;
- **`?renderscale=`** turned out to be the experiment that localised the cost,
  which is a better outcome than the mitigation it was shipped as. §5.5 has that
  correction; nothing about it changes again here.

The remaining question is not "what do we change" but "is there a mitigation at
all". §5.7 is where that lived, and §5.7 is where it was parked.

### 5.7 Parked — the owner's call, and what would reopen it

**Owner, 2026-08-26:**

> I strongly suspect this is purely a linux/nvidia/firefox issue - so lets put
> this to bed for now. […] We may return to this issue in the future.

**That is the right call on the evidence, and this note stops here.** Not
because the mechanism is fully nailed — §5.6 is a mechanism that survives every
reading rather than one that has been confirmed quantitatively — but because the
next reading would refine a defect that is **not this engine's, not this
engine's to fix, and not reachable by anyone this project ships to except on one
stack**. Investigation costs are real and the marginal one here buys precision on
somebody else's bug.

What the call rests on, stated so a future reader can check it rather than
inherit it:

- it was observed on Firefox + Linux + Wayland + the NVIDIA proprietary driver,
  and the evidence for those four is **not equal** — worth separating, because
  trigger 1 below turns on it:
  - **Firefox: direct.** Chrome on the same machine, same display, same page is
    unaffected at ~238fps (§5.1).
  - **The NVIDIA proprietary driver: strong.** The row that starts the whole
    chain (`DMABUF_WEBGL`, `FEATURE_FAILURE_BUG_1924578`) is a gfxInfo blocklist
    entry, and those are scoped by driver (§5.3).
  - **Linux and Wayland: untested.** Neither was varied. They are part of the
    configuration this was seen on, not established as necessary to it — the
    same driver exists elsewhere, and the blocklist entry's real scope was never
    read out of Mozilla's list.

  So "purely a Linux/NVIDIA/Firefox issue" is the shape of the evidence rather
  than a conclusion drawn from it, which is the honest way to hold it while
  parked;
- **nothing in this engine is implicated.** Not the frame clock, not the
  accumulator, not ADR-0041's interpolation — §5.6 works that through, and the
  conclusion is that the engine's whole obligation here was to make the thing
  diagnosable from a URL, which §4's instrument and this branch's two changes
  did;
- a playtester who hits it is **not without recourse**: the overlay tells them
  what is happening in words, and "try a smaller window" is advice the readings
  support.

**The caveat that would break the call, named on purpose: n = 1.** One machine
has ever shown this. "Purely a Linux/NVIDIA/Firefox issue" is a well-supported
reading of one configuration, not a measured population. The scope claim is the
part most likely to be wrong, and §5.7's first reopening trigger is exactly that.

#### What reopens this

Any one of these, and the work resumes from §5.7 rather than from §1:

1. **A second machine.** A report of the same signature — a slow `present` line,
   a healthy `refresh` estimate, `?renderscale=` changing nothing — on a
   configuration that is *not* Linux + NVIDIA + Firefox. That falsifies the scope
   claim above and makes this a browser-compatibility problem rather than one
   person's driver.
2. **A playtester who cannot work around it.** "Try a smaller window" is
   acceptable for one person who owns the machine and knows why. It is not
   acceptable as the answer to a stranger's bug report, and the second such
   report is the trigger.
3. **Mozilla bug 1924578 resolving.** If the blocklist entry lifts, the defect
   should evaporate on this machine, and the overlay is already the instrument
   that would show it. Worth a re-read the next time this page is opened on that
   machine, at zero cost.
4. **The engine gaining a second web path.** WebGPU is `navigator.gpu: null` in
   this Firefox (§5.3) and the web build is WebGL2-only (renderer.md §8/§9). A
   WebGPU path would not go through the blocked WebGL row at all, so the day that
   path exists, this is a thing to measure on it.

#### The two readings, preserved rather than run

Kept so that a future round starts where this one stopped, and **explicitly not
asked for**:

1. **Quantify the window-shrink effect** — the overlay median at two or three
   window sizes, with the `scale` line's **device-pixel box** at each (that box
   is the displayed size, the variable that has actually moved anything). Roughly
   linear in that area confirms §5.6 quantitatively.
2. **The tightened control** — a non-WebGL page that repaints *every frame* (any
   CSS animation) at the same window size. §5.6's "ordinary pages are fine" came
   from a text page, which does not repaint unless it is scrolling; this closes
   that gap.

#### The seam that was designed and deliberately not built

§5.6's mechanism predicts that shrinking the canvas's **CSS box** would work
where shrinking its backing store did nothing: render at native resolution into a
canvas that occupies a fraction of the page, letterboxed. A *display* scale
rather than a render scale. It would be **page-side only, with no engine change
at all** — winit reads the canvas's device-pixel content box, so the surface, the
camera viewport and pointer mapping all follow from CSS without a line of Rust.

**It is not built, and that is a decision rather than an omission.** Two
mechanisms were proposed in this note and both were refuted (§5.4, §5.5); the
cost of the second was a seam shipped with a claim that then had to be corrected
in three files. A third seam goes in after the reading that predicts it. Reading
(1) above is that reading, it was not run, and so the seam stays a paragraph.
If this reopens, the design is here and it is an afternoon.

#### What this note is now

A **closed record with a live instrument**. The investigation is parked; the
things it produced are not:

| produced | state |
|---|---|
| `?frametime=1` overlay (§4) | shipped, on every deployed build |
| measured slow-presentation warning | shipped, fires on this defect at 5.2× |
| `?renderscale=` seam | shipped, opt-in — a mitigation where cost is per rendered pixel, a discriminator everywhere |
| the finding that Firefox spoofs the renderer string on a **stock** profile | recorded, web-publish.md §2 |
| `viewport` and pointer are *surface* space | recorded, renderer.md §4 and input.md §3 |
| the defect itself | not ours, characterised, parked |

> **Verdict:** _a WebGL presentation defect in Firefox on Linux/Wayland/NVIDIA —
> ~12fps where Chrome does ~238fps on the same machine, on hardware and a
> compositor that are both fine. All three §3 hypotheses are dead. Two proposed
> mechanisms were refuted (cross-GPU transfer, by `about:support`; a per-pixel
> copy of the drawing buffer, by `?renderscale=` at 0.5 **and** 0.25 changing
> nothing), and one survives every reading: **a per-frame operation on the WebGL
> canvas sized by its displayed size rather than its backing store, existing at
> all because `DMABUF_WEBGL` is blocklisted on this driver.** Ordinary pages at
> the same window size are fine, so it is WebGL-specific and not the compositor.
> **Nothing in this engine is at fault; no engine change was made or is
> indicated.** Parked by the owner, 2026-08-26, as a defect of one browser on one
> platform with one driver — which is the shape of the evidence from n = 1
> machine, not a measured scope: Firefox is established directly and the driver
> strongly, while Linux and Wayland were simply never varied. §5.7 separates
> those, and holds the four triggers that reopen it and the two readings that
> would resume it._
