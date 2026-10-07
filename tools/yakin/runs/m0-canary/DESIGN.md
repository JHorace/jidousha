# DESIGN — m0-canary (inline, V1)

Build: only `tools/yakin/canary.md`. Touches no code. Order: record toolchain,
push CHECKPOINT, cold timed `tools/test` in background, read
`target/verify/report.json`, write canary.md, open PR.
Checks: canary.md numbers come from commands run this tick; `git diff --stat
origin/main...` lists only canary.md and the run folder.
