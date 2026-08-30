# Frame pacing — the web investigation (parked), and how native paces frames

Status: **two halves, and they are not the same kind of thing.**

- **§1–§5, the web.** A closed investigation, **parked by the owner,
  2026-08-26 — not this engine's defect.** Do not reopen it here; §5.7 holds the
  four triggers that would, and a separate session owns them.
- **§6, native.** Live design documentation: how a windowed native run is paced,
  what the audit of 2026-08-30 found and fixed, and the `JIDOUSHA_FRAMETIME`
  overlay at **level 1**. This is where a native pacing question is answered. It
  shares §2's vocabulary and §4's instrument and shares none of §5's verdict —
  the web defect is a browser's, and §6's was the engine's.
- **§7, the performance panel.** The same switch at **level 2**, on both
  targets: where a frame's milliseconds go, what the process and the GPU cost,
  and what the engine is holding. §6's disciplines are law there — off unless
  asked, drawn on a copy, never printed, sampled off the frame rather than per
  frame — and §7 adds nothing that relaxes them.

**The web half, in one paragraph.** Four rounds of readings (§5) settled what the
defect is, killed all three §3 hypotheses, and refuted two proposed mechanisms.
One survives every reading: **a
per-frame operation on the WebGL canvas sized by its displayed size rather than
its backing store**, existing at all because Firefox blocklists the zero-copy
WebGL path on this driver (§5.6). Ordinary pages at the same window size are
fine, so it is WebGL-specific and not the compositor.

**Nothing in this engine is at fault there and no engine change was made or is
indicated for it.** What the engine owed this defect was the ability to diagnose it from
a URL on a machine nobody here can see, and §4's instrument plus this branch's
two page-side changes delivered that. **The investigation is parked; the
instrument is live.** §5.7 has the owner's reasoning, the `n = 1` caveat that
would break it, the four triggers that reopen this, and the two readings and one
seam design preserved so a future round resumes rather than restarts.

**Read this note for**: how a native run's frames are paced and how to turn the
readout on (§6 — start here for anything native); what a slow frame is *spent
on*, and how to tell CPU-bound from GPU-bound from display-paced (§7); the shape of a browser
presentation defect and how the `?frametime=1` overlay tells one from a pacing
bug (§2, §3, §5.6); why the renderer string cannot be trusted in Firefox even on
a stock profile (§5.2, §5.3); and what `?renderscale=` is actually for (§5.5).

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

---

## 6. Native — how frames are paced, and the audit that fixed it

Status: **live.** §1–§5 above are a parked investigation into somebody else's
browser. This section is the engine's own frame loop on native, and unlike that
one it found a defect here and changed code. Nothing below reopens §5.

### 6.1 The observation

Owner, **2026-08-30**:

> A native `ninjo` build revs my PC — in a way a paused pixel game must not.

The half that makes it a defect rather than a preference is **paused**. `ninjo`
pauses by having the simulation perform a pause; the picture then barely changes
from frame to frame, and a machine audibly working for a still image is a machine
doing work nobody asked for.

### 6.2 What paces a native frame — the three places, audited

A windowed run's rate is decided in three places and only three. Each was read
rather than assumed.

1. **The swap chain's present mode**, chosen when the surface is configured
   (`jidousha-render-wgpu`'s `init::configure`). This is the one that can make
   the loop wait for the display.
2. **winit's control flow**, set by the driver every iteration
   (`driver/mod.rs::about_to_wait`). `Poll` runs the next iteration at once;
   `WaitUntil` sleeps the thread.
3. **Whether anything busy-spins.** Nothing does, before or after: no loop in
   this engine waits by re-reading a clock. The clock is read exactly twice a
   frame — `FrameClock::frame` for the duration handed to the accumulator, and
   `FrameClock::since_frame` for how much of the cap is left — and the waiting
   is `ControlFlow::wait_duration`, which is winit sleeping the thread.

**What (1) was doing, and it is the defect.** The surface was configured from
`wgpu::Surface::get_default_config`, which takes `caps.present_modes.first()` —
whatever the backend happens to list first, which is a different answer on every
platform:

| wgpu backend | first mode listed | waits for the display? |
|---|---|---|
| Vulkan (Linux, Android; Windows when chosen) | **whatever order the driver's `vkGetPhysicalDeviceSurfacePresentModesKHR` returns** | not reliably — measured `Immediate` on this project's Linux stack (§6.5) |
| DX12 (the Windows default) | **`Mailbox`** (`present_modes = vec![Mailbox, Fifo, …]`) | **no** — wgpu maps it to `SyncInterval = 0` |
| Metal | `Fifo` | yes |
| GL (and so every web build) | `Fifo` | yes |

So on the two platforms the owner's `ninjo` build can be running on, the swap
chain did not wait — and with (2) set to `Poll` and a redraw asked for on every
iteration, nothing else did either. **The loop drew as fast as the machine could
draw, paused or not.** That is the revving.

Note what is *not* implicated, exactly as in §2: the simulation. The fixed
timestep, the accumulator, the speed-invariance contract and `MAX_FRAME`'s 0.25s
catch-up clamp are untouched by everything below (core.md §7, ADR-0005). This is
presentation, and a cap on the simulation would be a speed change rather than a
pacing one.

### 6.3 What it does now

- **Vsync is asked for by name.** `init::configure` sets
  `WANTED_PRESENT_MODE = wgpu::PresentMode::Fifo` whenever the surface offers it,
  which is everywhere in practice: Vulkan requires FIFO of every conformant
  implementation and wgpu's other three backends all list it. The display is then
  the pace, so a 144Hz monitor still gets 144 frames a second — the fix bounds
  waste, not frame rate.
- **A fallback cap, for the surface that will not.** `driver/pacing.rs` holds
  `FALLBACK_CAP_HZ = 60.0`, and it is applied only when the backend reports a
  present mode that never waits. Sixty because that is `GameConfig::fixed_dt`'s
  default tick rate and therefore the rate at which this engine's picture
  actually changes; a cap under the tick rate would make every frame run two
  ticks, which is §2's jump. A test asserts that relationship rather than
  trusting the comment.
- **The cap is a sleep.** `ControlFlow::wait_duration(cap − time this frame
  already spent)`, so the loop sleeps the *remainder* of the period rather than a
  whole period on top of the work, and winit still wakes it immediately on an
  event — input latency is unchanged.
- **The seam that carries the fact.** `RenderBackend::presentation` returns
  `Presentation` — `Offscreen`, `Vsync`, `Mailbox`, `Immediate` — and
  `Presentation::needs_a_cap()` is the whole decision. `Offscreen` answers *no*
  on purpose: it is what a backend says while its device is still coming, and a
  startup polling for a GPU must not be slowed down. This is a report, never a
  request: nothing above the seam may set a present mode (renderer.md §7).

**The web path is untouched.** Its GL surface lists `Fifo` first and was already
getting it, so it reports `Vsync`, takes the `Poll` arm, and runs exactly the
loop it ran before. §4's interpolation and the clamp stand.

### 6.4 The native overlay — `JIDOUSHA_FRAMETIME`

**The switch, exactly:** set the environment variable `JIDOUSHA_FRAMETIME` to
anything other than `0` or `false`.

```
JIDOUSHA_FRAMETIME=1 cargo run --release -p ninjo
```

Off unless set — including unset, `0`, and `false` — which mirrors the web
overlay's `?frametime=1` down to the shorthand it accepts. **`=2` is the same
switch one level up**, and everything in this section is unchanged by it: level
2 is this panel plus the performance sections §7 documents, never this panel
rearranged. A value nobody planned for (`=3`, `=banana`) is level 1, because a
diagnostic switch must not be the reason a game will not start; the panel's
first line names the level it actually reached, which is where a typo is
noticed. An **environment
variable** rather than a `GameConfig` field or a flag, because the person who
wants it is the person *running* a build somebody else shipped them: a config
field needs a rebuild, and a flag would have to be plumbed through every game's
own argument parsing, which `--verify` already owns (input.md §5).

It draws a panel in the top-left corner of the window, over everything, in the
engine's built-in font — so it needs no asset and works on a game's first frame:

```
jidousha frame pacing: JIDOUSHA_FRAMETIME=1
present   ~59.5 fps - median 16.81ms, mean 17.50ms
spread    16.73ms .. 64.89ms over 240 frames
pacing    immediate - no vsync on this surface, so the loop is capped at 60 fps
ticks/fr  0:1 (0%)  1:441 (96%)  2:15 (3%)  3+:1 (0%)
frame deltas
   16-17ms ####################  206 (86%)
   17-18ms #                       3 (1%)
   …
```

The same readings as the web panel, in the same one-millisecond histogram buckets
so the two can be held side by side (web-publish.md §2) — and two of them are
better here:

- **`pacing`** is the line this overlay exists on native to print, and the web
  has no equivalent: it is the present mode the surface was **actually configured
  with**, plus the cap when one is being applied. "Is anything bounding this
  frame rate, and what" is answerable off a screenshot.
- **`ticks/fr`** is read off `Simulation::advance`'s return rather than modelled.
  The page cannot see inside the wasm module and has to re-run the accumulator
  over its own deltas (§4); a native run just asks.

Three things it deliberately does not do:

- **it never prints.** No log line, no stderr — a diagnostic that spammed a
  terminal would be a second thing to turn off;
- **it is drawn after the Draw phase has closed**, onto a copy of the
  submissions the world never sees. So a game's transcript, a recorded replay and
  a `--verify` run are byte-identical with the overlay on and off, which is what
  makes it safe to leave in a shipped build;
- **it is printable ASCII only.** The built-in font is ASCII 32–126 and draws a
  visible fallback box for anything else (renderer.md §6), so the em dashes the
  web panel uses would come out as boxes here. A test keeps that true.

### 6.5 The readings

Taken on this project's headless Linux container — Xvfb, `lavapipe` (a CPU
rasterizer), a 1280×720 window running `examples/window_clear`, a scene that is
static by construction. CPU is `utime + stime` out of `/proc`, over ten seconds
after a five-second settle. **State the machine class, because it is not the
owner's**: no GPU, and no display with a refresh rate.

| build | `pacing` line | presented | CPU |
|---|---|---|---|
| **before** — surface as wgpu defaulted it, `ControlFlow::Poll` every iteration | `immediate` | ~181.7 fps | **150% of one core** |
| **the cap alone** — same `immediate` surface, `FALLBACK_CAP_HZ` applied | `immediate — … capped at 60 fps` | ~59.5 fps | **34% of one core** |
| **after** — vsync requested, cap available | `vsync` | ~188.3 fps | 148% of one core |

Three things this says, and one it does not:

1. **The defect reproduced.** wgpu's default configuration chose **`Immediate`**
   on this machine's Vulkan surface — the row §6.2 predicted from wgpu-hal's
   source, measured. A native build was presenting with no wait of any kind.
2. **The cap does what it is for.** On that same uncapped surface, applying
   `FALLBACK_CAP_HZ` took the loop from 182fps and 150% of a core to 59.5fps and
   **34%** — a 4.4× reduction in CPU for a static scene, with `ticks/fr 1:96%`
   confirming the simulation ran exactly as before.
3. **Vsync is being asked for and granted.** The `pacing` line moves from
   `immediate` to `vsync`, which is the fix.
4. **What it does not show: the vsync saving.** The `after` row is still ~188fps
   and ~148%, because **Xvfb has no refresh to wait for** — Mesa's X11 FIFO
   presents immediately when the display server never blocks. On a real display
   FIFO blocks in the swap-chain acquire and the loop's rate becomes the refresh
   rate; this container cannot demonstrate that, and saying so is more useful
   than a number that would not mean what it looked like. **The owner's own
   before/after, with the overlay on, is the acceptance test** — row 2 is the
   closest analogue available here, and it is the same mechanism.

A note that follows from row 4 and is worth keeping: **`Fifo` is a request to
wait, and a display server with nothing to wait for grants it without waiting.**
The overlay prints the mode rather than only the rate for exactly this reason —
`vsync` at 188fps is a fact about the display server, not a contradiction.

### 6.6 Where this lives

| what | where |
|---|---|
| the present mode asked for | `crates/jidousha-render-wgpu/src/init.rs` — `WANTED_PRESENT_MODE` |
| the fact, above the seam | `crates/jidousha-render-core/src/backend.rs` — `Presentation`, `RenderBackend::presentation` |
| the cap and the schedule | `crates/jidousha-platform/src/driver/pacing.rs` — `FALLBACK_CAP_HZ` |
| the one place it becomes a `ControlFlow` | `driver/mod.rs::about_to_wait` |
| the switch, the window, the readout text | `crates/jidousha-platform/src/driver/overlay.rs` — `SWITCH` |
| the readout as quads | `crates/jidousha-render-core/src/overlay.rs` — `draw_readout` |
| off-by-default, in pixels | `crates/jidousha/tests/frame_overlay.rs`, which writes `target/verify/overlay-{off,on,perf}.png` |
| the level-2 sections | §7 below, and `driver/overlay/{phases,process,memory,snapshot}.rs` |

---

## 7. The performance panel — `JIDOUSHA_FRAMETIME=2`

Status: **live**, 2026-08-30. §6's overlay answers *how fast, and what is
pacing it*. This answers the question a slow frame raises next: **what is it
spent on, and is anything visibly growing** — the problems that are not obvious
from looking at the picture.

**The switch is §6's switch, one level up.** `JIDOUSHA_FRAMETIME=2` natively,
`?frametime=2` on a page. Level 1 is unchanged, character for character; level 2
is level 1 plus the sections below. Off unless set.

```
JIDOUSHA_FRAMETIME=2 cargo run --release -p ninjo
```

Everything §6.4 says the overlay deliberately does not do still holds, and the
sections below inherit all of it rather than being exempted from any of it:

- **it never prints.** No stderr, no log line, at any level;
- **it is drawn after the Draw phase has closed**, onto a copy of the
  submissions the world never sees — so a game's transcript, a recorded replay
  and a `--verify` run are byte-identical with the panel off, at level 1, and at
  level 2. A test asserts all three;
- **it is printable ASCII only**, for the reason §6.4 gives;
- **all sampling is draw-side.** Nothing here reads or writes simulation state;
  the two world counters are read through `World`'s ordinary read paths at draw
  time and nothing is written back;
- **operating-system counters are read at 1Hz and cached**, never per frame; the
  second is counted out of frame deltas the loop already produced, so no clock
  is read for it.

### 7.1 What it looks like

Taken on this project's headless container — Xvfb, `lavapipe`, `window_clear` —
which is why the numbers below are a software rasterizer's rather than a
machine's anybody would play on:

```
jidousha performance: JIDOUSHA_FRAMETIME=2
present   ~119.6 fps - median 8.36ms, mean 10.16ms
spread    5.77ms .. 45.07ms over 240 frames
pacing    vsync - the display sets the rate
ticks/fr  0:579 (44%)  1:626 (47%)  2:99 (7%)  3+:22 (2%)
frame deltas
     7-8ms #################      58 (24%)
     8-9ms ####################   71 (30%)
    9-10ms ###########            37 (15%)
    …
frame breakdown  ms: median  p95  max
  sim                             0.01    0.01    0.06
  draw                            0.00    0.00    0.00
  encode                          0.18    0.30    0.42
  present ###############         6.17    7.64   10.83
  sleep   #####                   1.93   21.43   37.37
  busy    2% of a 8.36ms frame - 8.17ms of it waiting
cpu       process 223% of one core
gpu       median 4.94ms, p95 6.50ms over 240 frames
memory    rss 125.1MB
  renderer 0.0MB textures, 0.5MB buffers
  world    0 entities, 0 components, 0 quads drawn
snapshot  press F9 to write this panel under target/
```

### 7.2 The readings, one at a time

Each entry: what it means, where it comes from, and where it is `n/a`.

**`frame breakdown` — the anchor.** Milliseconds per frame in each of five
buckets, as median, 95th percentile and worst over the same 240-frame window the
histogram above it uses. Three numbers rather than one because a phase that is
fine on every frame but one is a phase that *hitches*, and a median cannot say
so.

| bucket | what it is | where it comes from |
|---|---|---|
| `sim` | the frame's ticks, however many ran — including none | around `Simulation::advance` |
| `draw` | the Draw phase: the game's submissions, and the camera and face bookkeeping either side | around `Simulation::draw` |
| `encode` | turning submissions into a `FramePlan`: the asset commit, texture and text-atlas uploads, the overlay's own quads, `plan_frame` | the spans either side of the draw |
| `present` | `RenderBackend::render` — the backend's command encoding, the submit, **and the block waiting for the display** | around the seam call |
| `sleep` | everything else the frame contained: the pacer's `WaitUntil`, winit's dispatch, the operating system | **derived**: frame total minus the four measured |

Two things about that table are deliberate and worth stating plainly.

**`present` is one bucket, not two,** and the brief's "render-encode" and
"present-wait" are the two halves of it. The seam is where the driver's reach
stops: the backend acquires the surface, encodes, submits and blocks, and only
the backend is inside that. Splitting it would mean a clock below the backend
seam, and this engine has exactly one wall clock, in the platform crate
(`clock.rs`, ADR-0005) — a second one in `jidousha-render-wgpu` would buy a
column and cost the invariant that keeps determinism arguable. In practice the
split is not needed to read the panel: on any surface that waits, the wait is
nearly all of `present`, and the `encode` bucket beside it is the CPU-side cost
of building a frame. A 16.7ms `present` next to a 0.2ms `encode` is a loop the
display is pacing, and that is the reading.

**`sleep` is derived, never measured.** It is the frame's own duration minus the
four measured spans, so the five always add up to the frame and a mark that was
never taken shows up as sleep rather than as time that vanished. The duration
used is the elapsed time the *next* frame was given — that span runs from this
frame's start to the next one's, which is the only honest total the four spans
can be subtracted from. The consequence: the breakdown is one frame behind the
pacing readings above it, which at four repaints a second nobody can see.

**`busy` — the derived share.** `busy% = (frame − waits) / frame`, where the
waits are `present` and `sleep`: the two buckets in which this thread is not
running. **It is this thread's work, not the process's**, which is why the
example above reads `busy 2%` beside `cpu 223% of one core`: lavapipe rasterizes
on worker threads, so the main thread waits in `present` while the machine is
flat out. That disagreement is a reading, not a fault — and on a real GPU the
two converge.

**`cpu` — the process's own share of one core.** Native only, sampled at 1Hz and
cached in between. Linux reads `utime + stime` out of `/proc/self/stat` (the
comm field is parenthesised and may contain spaces, so the parse cuts at the
last `)` — everything after it is fixed-position); Windows reads
`GetProcessTimes`. Both are hand-rolled per platform: they are two system calls,
and `sysinfo` and its relatives bring a back-end tree to answer them
(agent-practices §5.8). **Of one core, not of the machine** — a number over 100
means more than a core's worth, which is the reading §6.5 turned on and which
dividing by the core count would have hidden. A run younger than two samples
reads `n/a`, because a percentage is a difference and one sample is not one. On
the web it reads `n/a` outright: a page has no process counters, and `busy`
above is the share that *is* answerable there.

**`gpu` — milliseconds on the GPU, where the device will say.** `wgpu`'s
`TIMESTAMP_QUERY` around the frame's main pass, median and p95 over the window.
Asked for as an **optional** device feature — an intersection with what the
adapter already offers — so a device that has no timestamps is created exactly
as it was before and nothing about that run changes. Absent, it reads
`gpu n/a - this device offers no timestamp queries`; never a zero, which would
claim the GPU did the frame instantly on precisely the machines with no reading.
WebGL2 has no timestamps at all, so every web build takes the `n/a` path.
**Milliseconds, never a percentage**: GPU *utilization* needs vendor libraries
this engine does not have, and one invented from a frame time would look like an
answer without being one (renderer.md §12a).

**`memory` — three tiers, and they do not add up.** They answer different
questions and are only ever compared with themselves:

1. **process** — resident set size, native, 1Hz. Linux `VmRSS` from
   `/proc/self/status`; Windows `K32GetProcessMemoryInfo`'s working set. What
   the operating system charges this program;
2. **wasm linear memory** — on the web instead of the above: the module's page
   count times 64KiB. **Not `performance.memory`**, which is Chrome-only,
   reports the whole tab, and is quantised for fingerprinting reasons. Linear
   memory only grows, so this is a high-water mark;
3. **engine-tracked accounting** — every platform, and the **actionable** tier.
   `renderer` is the backend's own running totals, counted at create and destroy
   (renderer.md §12a): textures and atlases on one line, vertex and uniform
   buffers on the other, because the two grow for different reasons. `world` is
   the entity count, the component count across every store, and the quads the
   game submitted this frame — the overlay's own quads excluded, since they go
   on a copy the world never sees.

The third tier is the one that catches a problem before it is visible. A
resident set size that climbs is a fact with no address in it; a texture total
that climbs is art nobody unloaded, an entity count that climbs is a spawner
with no reaper, and a component count that climbs while entities hold steady is
something inserting onto the same entities over and over. Those show up here
long before RSS moves, because an allocator does not return pages promptly and a
GPU's memory is not in RSS at all.

**`snapshot` — one key, one file.** While the panel is up, **F9** writes the
current panel to `target/jidousha-perf-<unix seconds>-<n>.txt`, one file per
press, and the panel then names the file it wrote. It is the only file the
overlay ever writes and it writes nothing without that press. The contents are
exactly the panel's own text and nothing else — not JSON, not a table — so the
numbers in a pull request and the numbers in the screenshot beside it cannot
disagree. This is what feeds the before/after-numbers practice: press it,
change something, press it again, paste both.

The key is **observed, never consumed**: the press goes on to the game exactly
as it would with the panel off, so a transcript and a replay are unaffected and
a game that binds F9 to something keeps it. Auto-repeat never reaches it —
`translate::key_event` drops repeats — so leaning on the key writes one file.

**The asymmetry, stated:** there is **no snapshot key on the web**. A page has
nowhere to write a file to, and the browser's equivalent of "save the panel" is
a screenshot. The panel says so in place of the key.

### 7.3 The recipe: which bound is this?

Read three lines together — `busy`, `present`, `gpu` — and the answer falls out:

| reading | what it is | what to do about it |
|---|---|---|
| `present` high, `gpu` low | **display pacing.** The loop is waiting for the refresh and the GPU has headroom. Nothing is wrong. | Nothing. Confirm the `pacing` line says `vsync`; if it says `immediate`, the wait is the loop's own cap and §6.3 explains it. |
| `gpu` high, near the frame time | **GPU-bound.** The frame is waiting for work that is actually running. | Fewer pixels or fewer batches — `?renderscale=` on the web is the quickest test (web-publish.md §2). |
| `busy` high, `gpu` low | **CPU-bound.** The loop fills the frame with its own work. | The breakdown says which bucket: `sim` is the game's systems, `draw` is its Draw phase, `encode` is uploads and plan-building. |
| `busy` low, `present` low, `sleep` high | **capped, or idle.** Nothing is asking for the time. | Check the `pacing` line: a cap is `FALLBACK_CAP_HZ` doing its job (§6.3). |
| any of the above, with `world` or `renderer` climbing steadily | **growth**, whatever the frame time says today. | The counter names the store; a leak that has not slowed anything down yet is the cheapest one to fix. |

One caution the container taught: **`busy` and `cpu` measure different things**,
and a software rasterizer separates them completely (see `busy` above). When
they disagree, `busy` is this thread and `cpu` is every thread.

### 7.4 What it costs

An instrument that perturbs what it measures is the failure mode, so this is
measured rather than asserted. Two figures, both from tests that print them
(`driver/overlay/mod.rs` and `driver/frame.rs`), in a release build on this
project's container:

| | level 1 | level 2 |
|---|---|---|
| **the measuring** — rolling windows, the 1Hz sampler, the engine counters, the panel rebuilt four times a second | 1.84µs/frame | **3.28µs/frame** (+1.43µs) |
| **end to end**, through a backend that draws nothing | 66.7µs/frame | **172µs/frame** |

The second row is dominated by **drawing** the panel rather than by measuring
anything: the readout is a quad per character, so level 2's twenty-odd lines are
about four times level 1's five, and that difference goes through `plan_frame`
and the backend like any other quad. On a real GPU it is one more batch of a few
thousand vertices.

The measuring itself is under two microseconds a frame added — about a hundredth
of one percent of a 60Hz frame — and the disciplines that keep it there are
structural rather than incidental: the operating system is asked once a second,
the panel is composed four times a second, every engine counter is a running
total something already maintains, and a run at level 1 or off takes no phase
marks at all. Each of those has a test that fails if it stops being true.

### 7.5 Where this lives

| what | where |
|---|---|
| the switch, the levels, and what has been measured | `crates/jidousha-platform/src/driver/overlay/mod.rs` |
| every line the panel says | `…/driver/overlay/panel.rs` |
| the frame breakdown and its window | `…/driver/overlay/phases.rs` |
| the process counters, per platform | `…/driver/overlay/process.rs` |
| the three memory tiers | `…/driver/overlay/memory.rs` |
| the snapshot key and its file | `…/driver/overlay/snapshot.rs` |
| the phase marks, and the engine counters read at draw | `…/driver/frame.rs` |
| the key tap, observed and not consumed | `…/driver/mod.rs` — `snapshot_key` |
| the accounting and the GPU timer, below the seam | `crates/jidousha-render-wgpu/src/timing.rs`, `init.rs`, and renderer.md §12a |
| the seam that carries them | `crates/jidousha-render-core/src/backend.rs` — `BackendStats` |
| off / level 1 / level 2, in pixels | `crates/jidousha/tests/frame_overlay.rs` → `target/verify/overlay-{off,on,perf}.png` |
| the web page's half of the switch | `tools/web-template/index.html`, and web-publish.md §2 |

### 7.6 The web, and why there are two panels at level 2

`?frametime=1` is the page's panel and stays the page's: it measures
`requestAnimationFrame` from outside the wasm module, which is a better reading
of *presentation* than anything inside the module can take, and its CONTRACT is
that it never calls in (web-publish.md §2).

That contract is exactly why level 2 needs a second panel. Sim ticks, texture
uploads, entity counts and GPU timings are not visible from a page at all, and
the page may not ask. So at `?frametime=2` the **engine** draws the performance
sections into the canvas, top left, while the page's own panel keeps the pacing
readings, top right. Each shows what only it can see; neither calls the other;
the page's note says so, so nobody has to work out why there are two.

The engine's web panel therefore omits the pacing block — one line points at the
page's instead — and reads `cpu process n/a`, `gpu n/a` (WebGL2 has no
timestamps) and `memory wasm linear …MB`. The breakdown, the busy share and the
whole engine-tracked accounting tier are the same readings as on native, because
they are the module's own.
