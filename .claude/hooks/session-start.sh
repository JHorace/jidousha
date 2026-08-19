#!/bin/bash
# Give a remote session a software Vulkan rasterizer, so a frame can be looked at.
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

# The container is cached after this completes, so a warm start does nothing.
if [ -e "$LAVAPIPE_ICD" ]; then
  echo "[session-start] lavapipe already present — frames can be rendered and captured"
  exit 0
fi

SUDO=""
if [ "$(id -u)" -ne 0 ]; then
  SUDO="sudo"
fi

install_the_rasterizer() {
  $SUDO apt-get install -y --no-install-recommends mesa-vulkan-drivers
}

echo "[session-start] installing mesa-vulkan-drivers (lavapipe), so a rendered frame can be captured"
if ! install_the_rasterizer >/dev/null 2>&1; then
  # A cold or stale package index. `update` warns about third-party PPAs this
  # project does not use and still exits 0, so its noise stays out of the log.
  $SUDO apt-get update >/dev/null 2>&1 || true
  install_the_rasterizer >/dev/null 2>&1 || true
fi

if [ -e "$LAVAPIPE_ICD" ]; then
  echo "[session-start] lavapipe installed — golden-image tests will run rather than skip,"
  echo "                and 'tools/verify <example>' will write a PNG you can open"
  exit 0
fi

# Four parts, the same shape the engine's own messages use.
{
  echo "[session-start] no software rasterizer could be installed"
  echo "  what this costs: golden-image tests will skip and say so, and tools/verify will"
  echo "    report 'capture: skipped, no GPU on this machine'. Nothing fails — this is the"
  echo "    state every E0 run before this one had (e0-findings.md F-054)."
  echo "  likely cause: the package index or the archive was unreachable from this container."
  echo "  fix: run 'tools/doctor' for the gpu line, then"
  echo "    'apt-get install -y --no-install-recommends mesa-vulkan-drivers' by hand."
} >&2
exit 0
