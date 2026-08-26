# Frame pacing on the web — an open investigation

Status: **diagnosed — a presentation-path defect in Firefox; one open item.**
This note holds one observed defect, the ranked hypotheses for it, the
instrument built to tell them apart, the owner's readings off that instrument,
and the verdict they carry (§5). What is left open is the *mechanism*: the
leading candidate is cross-GPU transfer on this hybrid machine, and §5.4 is the
three-step protocol on the owner's machine that confirms or replaces it. **This
note closes when §5.4 reports** — not before, and it says so rather than
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
| webgpu | available | not captured |

`about:support` → Compositing in Firefox reads **WebRender (hardware)**.

The machine has **hybrid graphics**: a discrete NVIDIA RTX 5090 and an
integrated AMD. Which of them Firefox's WebGL actually runs on was *not*
captured this round and is not knowable from the page — see §5.4.

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

**Hypothesis 3 — rAF/compositor scheduling jitter: not what this is.** Jitter
means uneven frames around a healthy median. This median is 83.30ms with 94% of
frames at or past 50ms: the frames are not jittering around the cadence, they
are uniformly nowhere near it.

### 5.3 The finding

**Firefox presents this canvas at ~12fps, with a per-frame cost that scales
with pixel area, on hardware that presents the identical page at ~238fps in
Chrome.** Nothing about the simulation, the clock, or the page differs between
the two; the display runs a ~240Hz mode and Chrome uses it.

That shape — hardware compositing, a fine clock, a uniformly slow frame, and a
cost proportional to the number of pixels — is a **presentation-path defect in
the copy/readback family**: something on the path between the WebGL drawing
buffer and the composited page is paying per pixel, per frame.

The **leading mechanism is cross-GPU transfer in the hybrid setup** — WebGL
rendering on one adapter and compositing on the other, with a full-framebuffer
copy across the bus every frame. It fits every reading, and the pixel-area
scaling is exactly what it predicts. **It is not confirmed.** §5.4 is the
protocol that would confirm it, and until those readings land this is the
leading candidate and is written down as one.

The ticks/frame column is worth naming as *not* a defect: 3+ ticks on 94% of
frames is the accumulator doing precisely its job (core.md §7). It is the
symptom's mechanism, not its cause.

**Interpolation (ADR-0041, §4) is functioning as designed and cannot help
here.** It places the drawn position where the elapsed time actually calls for,
which is the right thing to draw and is why the *shape* of the motion is honest.
But there is no pacing logic — none, in this engine or any other — that makes
12fps smooth. §4's second item was right under hypotheses 2 and 3 and it stays;
it is simply not the fix for this, because this is not a pacing problem. It is a
throughput problem wearing a pacing problem's symptoms.

**A finding about the instrument, too.** The overlay's software-rendering
warning is **string-based, and the string is spoofable — so the warning is
unreliable in Firefox**, the one browser it was most needed on. That is fixed
rather than noted: the overlay now also carries a **measured** slow-presentation
warning (web-publish.md §2), which compares the rolling median frame time with
the refresh period the browser's own quickest frames imply and says so, in the
box, past 2.5× and past 1/30s over 60 frames. Under these readings it fires on
Firefox at 5.2× and stays quiet on Chrome at 1.02×. The string check stays
beside it, and where the two disagree the measurement is the one to believe.

**And a mitigation a playtester can reach.** `?renderscale=0.5`
(web-publish.md §2) renders a quarter of the device pixels and lets the browser
upscale — presentation-only, so world space, the letterbox contract
(games/giri/UI.md §6), input mapping and the simulation are all untouched. On a
path that costs per pixel, that is most of the cost. It is opt-in: no deployed
build's default behavior changed, and a default cap on device pixel ratio would
be a decision needing an ADR rather than a quiet edit.

### 5.4 Still open — the owner's protocol

This note stays alive for exactly this, and closes when it reports. Three steps,
all on the owner's machine, all in Firefox:

1. **`about:support` → Graphics → GPU #1 / GPU #2 — which is active.** The
   page-visible renderer string is spoofed and this listing is not; it is the
   truthful source for what Firefox's WebGL is actually running on, and it was
   not captured last round.
2. **Relaunch Firefox on the discrete GPU** and re-read the overlay:
   ```
   __NV_PRIME_RENDER_OFFLOAD=1 __GLX_VENDOR_LIBRARY_NAME=nvidia firefox
   ```
   If the median frame collapses to about the refresh period, the hybrid
   cross-GPU mechanism in §5.3 is **confirmed on the record** and stops being a
   leading candidate.
3. **With the defect present, load `?frametime=1&renderscale=0.5`** and report
   the overlay. This is the validation that matters for the playtesters who
   cannot set environment variables: it says whether the mitigation works on the
   real defect rather than on a reasoned model of it. The overlay's own `scale`
   line reports the backing store it got, so the reading is self-describing.

Whoever reads those replies writes them into §5.1 and §5.3, marks the mechanism
confirmed or replaces it, and closes this note.

> **Verdict:** _a presentation-path defect in Firefox — ~12fps on hardware that
> does ~238fps in Chrome, cost scaling with pixel area. Hypothesis 2 is dead,
> hypothesis 1 is dead for compositing and unanswerable from the renderer string
> Firefox spoofs, and interpolation was never going to fix it. Cross-GPU
> transfer in the hybrid setup leads and is unconfirmed; §5.4 is what confirms
> it._
