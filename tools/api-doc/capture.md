A `--verify` run asserts on what was *submitted*. This is the other half: render
one of those frames for real, write it out as a PNG, and look at it. It is the
only instrument in this whole surface that answers "does it look like the game",
and it is the one that has twice caught a fault every assertion was happy with —
a banner reading `YOU WINS 5 - 2`, and a second line drawn straight through both
paddles and well inside the camera.

Read this last, and only once your `--verify` mode runs and asserts.

**The picture is yours to take, and nothing takes it for you.** `tools/verify`
renders nothing: it reads one line out of what your `--verify` mode printed, and
a run that captures nothing passes, silently. So a game with no capture path
satisfies every check the other three documents ask for, and the only thing it
lacks is a way to be looked at.

**And the frame you already recorded is the one to draw.** You do not replay the
session and you do not restructure your game to hand it a renderer: `FrameRecord`
carries a `plan` — the finished frame, with the depth sort and the batching
already done — and `WgpuBackend::offscreen` will execute it. The whole path is
about thirty lines, and **`examples/prototype_kit/capture.rs` is those thirty
lines with the reasoning written at each step.** Read it rather than
reconstructing it from here: the one time this document carried the path as well,
the two copies drifted and the one here was the wrong one. It also covers the
harder case — a game that loads art, whose texture ids mean something only to a
backend that created its textures in the same order.

Four things about it belong here rather than there, because each is either a
contract with the tooling or a mistake that is silent when you make it:

- **`tools/verify` reads exactly one line.** It takes the first whose text starts
  with `capture:` and contains ` written to `, and puts what follows into the
  report. Word it differently and the run still passes while the report says no
  picture was taken.
- **Capture at the recorder's aspect ratio.** A capture of another shape
  stretches the picture while every assertion you wrote goes on passing, because
  none of them look at pixels. 480x270 for a 1280x720 recorder, and assert the
  ratio rather than remembering it; `CAPTURE_SIZE` in the example says why
  nothing downstream can recompute it for you.
- **Check that the texture ids still mean the same thing**, in one line, before
  you believe the PNG. A plan whose ids drifted renders the wrong texture into an
  image every other check in your `--verify` is happy with.
- **A machine with no GPU must still pass, and a broken one must not.**
  `RenderError::NoAdapter` is a fact about the machine — every runner is headless
  and some have no graphics stack at all — so say the capture was skipped, put
  that in the summary, keep the run green, and do not skip in silence either.
  Every *other* handshake error is a fault, and reporting one of those as "no GPU
  here" files a real problem as a property of the hardware, on every machine, for
  ever. Match on the variant; the example's poll loop is the shape.

**Then open the file and look at it.** A capture path that writes a PNG is worth
nothing on its own; the question is whether it writes *your game's* PNG, and a
path wired to the wrong frame or to a stale plan passes every check that does not
ask. So look — name what you see — then break the game on purpose and look again:
move a paddle, stop drawing the score, change the clear colour, and confirm the
picture follows.
