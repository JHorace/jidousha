# Frame pacing on the web — an open investigation

Status: **diagnosed, mechanism identified; one measurement outstanding.** This
note holds one observed defect, the ranked hypotheses for it, the instrument
built to tell them apart, two rounds of owner readings, and the verdict they
carry (§5). The cause is `DMABUF_WEBGL` blocked by Mozilla's own blocklist,
forcing a per-pixel copy of every frame (§5.4); what is not yet established is
that this is the *dominant* cost rather than one of several. **This note closes
when §5.5's one reading lands** — not before, and it says so rather than
guessing.

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
~1.46 means a near-fullscreen canvas is several megapixels of *device* pixels,
every one of them going through that copy. This is not a browser being slow in
general; it is a browser paying a per-pixel tax on an unusually large number of
pixels. A 4K screen is where this defect goes from "measurable" to "12fps".

**Confidence.** This is a mechanism the machine's own feature log states as
fact — `DMABUF_WEBGL: blocked` is a reading, not an inference. What remains
inferred is that this blocked row is *the* dominant cost rather than one of
several, and §5.5 has the one measurement that would settle it quantitatively.
Treat it as: cause identified, magnitude unconfirmed.

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
(games/giri/UI.md §6), input mapping and the simulation are all untouched. On a
path whose cost is a per-pixel copy this is close to the whole cost, and §5.3's
1.46 device pixel ratio on a 4K panel is why it has so much to give back. It is
opt-in: no deployed build's default behavior changed, and a default cap on
device pixel ratio would be a decision needing an ADR rather than a quiet edit.

### 5.5 Still open — one measurement

Step 1 reported (§5.3). Step 2 is retired as moot (§5.4). What is left is step 3,
and it has been promoted: it is no longer only a validation of the mitigation,
it is the **quantitative test of the mechanism**, because a per-pixel copy makes
a prediction nothing else here does.

**With the defect present, in Firefox, load `?frametime=1&renderscale=0.5` and
report the overlay.** If the dominant cost is a per-pixel copy, a quarter of the
pixels should move the median frame time by close to a factor of four — from
~83ms toward ~21ms, bounded below by whatever fixed per-frame cost is left. The
overlay's own `scale` line reports the backing store it actually got, so the
reading is self-describing.

- **Roughly 4× better** ⇒ the mechanism in §5.4 is confirmed in magnitude as
  well as in kind, and this note closes.
- **Barely better** ⇒ the blocked `DMABUF_WEBGL` row is real but is not the
  dominant cost, and something else on that path is. The note stays open and
  §5.4's confidence paragraph is what gets revised.

Two optional readings, if they are cheap and only if they are:

- **A second window size**, with the overlay median at each and the device-pixel
  counts from its `scale` line. Two points give the scaling exponent, and the
  mechanism predicts linear in pixel area. This is the window-shrink experiment
  §5.1 records, made into a number.
- **Un-blocklisting `DMABUF_WEBGL`**, if this build offers a way to override
  gfxInfo for that entry, would be the direct test. Mozilla bug **1924578** is
  the entry to read first — it says who the blocklist is protecting and from
  what, and "the crash it was added to prevent" is a real possible answer to
  "why not just turn it on".

Whoever reads that reply writes it into §5.4, marks the magnitude confirmed or
revises it, and closes this note.

> **Verdict:** _a presentation-path defect in Firefox — ~12fps on hardware that
> does ~238fps in Chrome, cost scaling with pixel area. Hypothesis 2 is dead;
> hypothesis 1 is dead for compositing and unanswerable from a renderer string
> that stock Firefox spoofs; interpolation was never going to fix it. The
> cross-GPU mechanism is refuted — WebGL and compositing are both on the RTX
> 5090. The mechanism is `DMABUF_WEBGL` blocked by Mozilla's blocklist
> (bug 1924578), forcing a per-pixel copy of every frame, multiplied by a 4K
> panel at a 1.46 device pixel ratio. Cause identified; magnitude unconfirmed,
> and §5.5 is the one reading that settles it._
