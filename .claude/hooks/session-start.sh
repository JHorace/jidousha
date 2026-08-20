#!/bin/bash
# Give a remote session the two things it needs to look at its own game: a software
# Vulkan rasterizer, and a virtual display a windowed example can actually open on.
#
# e0-findings.md F-054 / F-065: five consecutive E0 runs built a game on a machine
# with no display and no GPU, so no run has ever seen its own work. Every claim
# about how a game *looks* was inference from a transcript, rescued afterwards by a
# person playing it. The engine was never the gap — `WgpuBackend::offscreen`
# renders headless and `tools/verify` captures a PNG "if the machine has a GPU".
# What was missing was a machine where that condition is ever true.
#
# lavapipe, in `mesa-vulkan-drivers`, is Mesa's CPU rasterizer. It is deterministic,
# which is what makes a checked-in golden reference reproducible at all, and it is
# the same package `.github/workflows/ci.yml` has installed on the Linux runner the
# whole time. This is that one line, moved to where the authoring happens.
#
# A failure here is not fatal and must not be silent (CLAUDE.md): the golden tier
# already reports "no adapter" and passes, and `tools/doctor` prints a gpu line, so
# a session that lands without a driver degrades to exactly the state the first five
# E0 runs had — and says so rather than leaving it to be inferred.
set -uo pipefail

# A local checkout has whatever the developer's machine has. This closes a gap in
# the container, so it has no business touching anyone's workstation.
if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

readonly LAVAPIPE_ICD=/usr/share/vulkan/icd.d/lvp_icd.json

# e0-findings.md F-111: with lavapipe in place, `--verify` and the capture work and
# the *playtest* still does not. `cargo run --example pong` under `xvfb-run` panics
# inside `xkbcommon-dl`, because winit's X11 backend dlopens `libxkbcommon-x11.so`
# and the image ships only `libxkbcommon.so.0`. That is E0's after-the-run step 2 —
# "play it" — unreachable from the container that wrote the game, which is why run 8
# shipped a Pong nobody had seen in a window. One library closes it.
readonly XKB_X11=/usr/lib/x86_64-linux-gnu/libxkbcommon-x11.so.0

# The container is cached after this completes, so a warm start does nothing.
if [ -e "$LAVAPIPE_ICD" ] && [ -e "$XKB_X11" ]; then
  echo "[session-start] lavapipe and the X11 keyboard library are already present"
  exit 0
fi

SUDO=""
if [ "$(id -u)" -ne 0 ]; then
  SUDO="sudo"
fi

# xvfb is the display, xdotool sends the keys and x11-apps supplies `xwd` to read
# the pixels back — which together make the playtest a thing this container can do
# rather than only a thing it can compile.
install_the_display_packages() {
  $SUDO apt-get install -y --no-install-recommends \
    mesa-vulkan-drivers libxkbcommon-x11-0 xvfb xdotool x11-apps
}

echo "[session-start] installing lavapipe and Xvfb's keyboard library, so a frame can be"
echo "                captured and a windowed example can be opened and played"
if ! install_the_display_packages >/dev/null 2>&1; then
  # A cold or stale package index. `update` warns about third-party PPAs this
  # project does not use and still exits 0, so its noise stays out of the log.
  $SUDO apt-get update >/dev/null 2>&1 || true
  install_the_display_packages >/dev/null 2>&1 || true
fi

if [ -e "$LAVAPIPE_ICD" ] && [ -e "$XKB_X11" ]; then
  echo "[session-start] lavapipe installed — golden-image tests will run rather than skip,"
  echo "                and 'tools/verify <example>' will write a PNG you can open"
  echo "[session-start] libxkbcommon-x11 installed — a windowed example runs under"
  echo "                'xvfb-run -a -s \"-screen 0 1280x720x24\" cargo run --example <name>'."
  echo "                Xvfb has no window manager, so nothing sets the input focus and every"
  echo "                key goes to the root window: 'xdotool windowfocus --sync \$(xdotool"
  echo "                search --name <name> | tail -1)' once, and the keyboard reaches the game."
  exit 0
fi

# Four parts, the same shape the engine's own messages use.
{
  echo "[session-start] the display packages could not all be installed"
  echo "  what this costs: without lavapipe, golden-image tests skip and say so and tools/verify"
  echo "    reports 'capture: skipped, no GPU on this machine' (e0-findings.md F-054); without"
  echo "    libxkbcommon-x11 a windowed example panics under xvfb-run and cannot be played"
  echo "    (F-111). Nothing fails — both degrade to the state earlier E0 runs had."
  echo "  likely cause: the package index or the archive was unreachable from this container."
  echo "  fix: run 'tools/doctor' for the gpu line, then 'apt-get install -y"
  echo "    --no-install-recommends mesa-vulkan-drivers libxkbcommon-x11-0 xvfb"
  echo "    xdotool x11-apps' by hand."
} >&2
exit 0
