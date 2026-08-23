# Frame pacing on the web — an open investigation

Status: **instrument built, verdict outstanding.** This note holds one observed
defect, the ranked hypotheses for it, the instrument built to tell them apart,
and the reading that will close it. It closes when the owner's two screenshots
land in §5 — not before, and this note says so rather than guessing.

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

## 2. Why a jump rather than a stutter

The engine draws committed state once per rendered frame and runs whole ticks in
between (core.md §7). Frames that arrive unevenly produce, from the accumulator,
a sequence like 1, 1, 0, 2, 1, 1, 0, 2 — and a 2-tick frame moves the ball twice
as far as its neighbours in the same amount of screen time. The eye reads that
as a jump forward, not as a dropped frame, which is exactly the word the owner
used and is worth taking as evidence rather than as loose phrasing.

Note what is *not* wrong: the simulation. Ticks are the same length and the same
count either way; what varies is how many of them land between two presented
frames. This is a presentation defect with a presentation fix, and §4 is the fix.

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
   *Signature:* the clock line reads ~1ms, the histogram piles onto whole
   milliseconds and stays tight around the refresh period, the renderer is
   hardware and the warning is absent, and 0-tick and 2-tick frames appear in
   roughly equal, small numbers.
3. **rAF / compositor scheduling jitter.** Neither of the above: hardware
   renderer, fine clock, and the deltas themselves are uneven.
   *Signature:* a histogram with real spread and no other line explaining it.

These are not exclusive — 1 and 2 can both be true — and the point of the
instrument is that it does not have to be argued about.

## 4. What was done about it before knowing the answer

Two things, because both are right under every hypothesis.

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

## 5. The verdict — outstanding

The two screenshots that settle §3, from the owner's own machine:

1. Open Pong with **`?frametime=1`** in **Firefox**. Screenshot the overlay.
2. Open the same URL with **`?frametime=1`** in **Chrome**. Screenshot the
   overlay.
3. In Firefox, **`about:support` → Graphics → Compositing** — it reads either
   hardware WebRender or software. Note which.

Read together with §3, those decide it. Whoever fills this in writes the
readings out, names which hypothesis they support, and moves this note's status
from *outstanding* to a finding with a fix or a closed non-issue.

> **Verdict:** _outstanding — no screenshot has been taken. Nobody in this
> session can see the owner's monitor, and a diagnosis written without those
> readings would be a guess wearing a finding's clothes._
