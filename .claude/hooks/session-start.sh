#!/bin/bash
# Give a remote session the three things it needs to look at its own game: a
# software Vulkan rasterizer, a virtual display a windowed example can open on,
# and the `wasm-bindgen` CLI that turns the web build into something a browser
# can load.
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

# e0-findings.md F-124: the other half of step 2. `tools/serve-web <example> --check`
# builds for wasm and drives a headless browser at it, and it is the only thing that
# runs the web target as a *program* rather than as the `cargo check` CI has gated
# since M0. Everything it needs was already here — the wasm32 target, and a Chromium
# at /opt/pw-browsers that the tool already knows how to find — except the CLI, so
# eight runs' worth of "no session has driven its game in a browser" was one missing
# binary. **Prebuilt, not `cargo install`**: the release tarball is a 1.3-second
# download against several minutes of compiling, and it is the same binary.
#
# The version must equal the `wasm-bindgen` crate in Cargo.lock exactly — a mismatch
# produces glue that fails at run time with a message about nothing in particular,
# which is why `tools/serve-web` refuses to guess. So it is read from the lock rather
# than written here: a version in two places is a version that drifts on the first
# `cargo update`.
readonly REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly WASM_BINDGEN=/usr/local/bin/wasm-bindgen
wanted_wasm_bindgen() {
  awk '/^name = "wasm-bindgen"$/{found=1; next}
       found && /^version = /{gsub(/[",]/, "", $3); print $3; exit}' \
    "$REPO_ROOT/Cargo.lock" 2>/dev/null
}
readonly WANTED_WASM_BINDGEN="$(wanted_wasm_bindgen)"

wasm_bindgen_is_current() {
  [ -z "$WANTED_WASM_BINDGEN" ] && return 0   # no lock to match: not this hook's problem
  [ -x "$WASM_BINDGEN" ] || return 1
  [ "$("$WASM_BINDGEN" --version 2>/dev/null)" = "wasm-bindgen $WANTED_WASM_BINDGEN" ]
}

# The container is cached after this completes, so a warm start does nothing.
if [ -e "$LAVAPIPE_ICD" ] && [ -e "$XKB_X11" ] && wasm_bindgen_is_current; then
  echo "[session-start] lavapipe, the X11 keyboard library and wasm-bindgen are already present"
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

# The published binary for the version the lock pins. musl, so it does not care
# what libc the image has.
install_wasm_bindgen() {
  [ -n "$WANTED_WASM_BINDGEN" ] || return 1
  local name="wasm-bindgen-${WANTED_WASM_BINDGEN}-x86_64-unknown-linux-musl"
  local url="https://github.com/rustwasm/wasm-bindgen/releases/download/${WANTED_WASM_BINDGEN}/${name}.tar.gz"
  local work
  work="$(mktemp -d)" || return 1
  curl -sSfL --retry 3 -o "$work/wb.tar.gz" "$url" \
    && tar xzf "$work/wb.tar.gz" -C "$work" \
    && $SUDO install -m 755 "$work/$name/wasm-bindgen" "$WASM_BINDGEN"
  local status=$?
  rm -rf "$work"
  return $status
}

echo "[session-start] installing lavapipe, Xvfb's keyboard library and wasm-bindgen, so a"
echo "                frame can be captured, a windowed example opened and played, and the"
echo "                web build loaded in a browser"
if [ ! -e "$LAVAPIPE_ICD" ] || [ ! -e "$XKB_X11" ]; then
  if ! install_the_display_packages >/dev/null 2>&1; then
    # A cold or stale package index. `update` warns about third-party PPAs this
    # project does not use and still exits 0, so its noise stays out of the log.
    $SUDO apt-get update >/dev/null 2>&1 || true
    install_the_display_packages >/dev/null 2>&1 || true
  fi
fi
wasm_bindgen_is_current || install_wasm_bindgen >/dev/null 2>&1 || true

if [ -e "$LAVAPIPE_ICD" ] && [ -e "$XKB_X11" ] && wasm_bindgen_is_current; then
  echo "[session-start] lavapipe installed — golden-image tests will run rather than skip,"
  echo "                and 'tools/verify <example>' will write a PNG you can open"
  echo "[session-start] libxkbcommon-x11 installed — a windowed example runs under"
  echo "                'xvfb-run -a -s \"-screen 0 1280x720x24\" cargo run --example <name>'."
  echo "                Xvfb has no window manager, so nothing sets the input focus and every"
  echo "                key goes to the root window: 'xdotool windowfocus --sync \$(xdotool"
  echo "                search --name <name> | tail -1)' once, and the keyboard reaches the game."
  echo "[session-start] wasm-bindgen $WANTED_WASM_BINDGEN installed — 'tools/serve-web <example>"
  echo "                --check' now builds for the web and drives the bundled Chromium at it,"
  echo "                which is the only check that runs the wasm target as a program."
  exit 0
fi

# Four parts, the same shape the engine's own messages use.
{
  echo "[session-start] the display packages could not all be installed"
  echo "  what this costs: without lavapipe, golden-image tests skip and say so and tools/verify"
  echo "    reports 'capture: skipped, no GPU on this machine' (e0-findings.md F-054); without"
  echo "    libxkbcommon-x11 a windowed example panics under xvfb-run and cannot be played"
  echo "    (F-111); without wasm-bindgen, 'tools/serve-web' refuses rather than guessing at a"
  echo "    version and the web target goes unrun (F-124). Nothing fails — each degrades to the"
  echo "    state earlier E0 runs had."
  echo "  likely cause: the package index, the archive or github.com was unreachable from this"
  echo "    container."
  echo "  fix: run 'tools/doctor' for the gpu line, then 'apt-get install -y"
  echo "    --no-install-recommends mesa-vulkan-drivers libxkbcommon-x11-0 xvfb"
  echo "    xdotool x11-apps' by hand, and install the wasm-bindgen version Cargo.lock pins"
  echo "    from its release page (or 'cargo install wasm-bindgen-cli --version <that>')."
} >&2
exit 0
