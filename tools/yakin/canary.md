# yakin canary — 2026-10-07

Tick: branch claude/yakin-m0-canary · resumes 0 · started 2026-10-07 01:09:18 PDT · finished 2026-10-07 01:22:44 PDT

## Toolchain
rustc 1.94.1 (e408947bf 2026-03-25)
cargo 1.94.1 (29ea6fb6a 2026-03-24)
Python 3.11.17
git version 2.43.0
Wed 2026-10-07 01:09 PDT
4
/dev/vda        252G  9.8G   30G  26% /
ENV_OK

## tools/test (cold)
exit: 0 · wall: 742s · report status: pass
counts: 1487 passed · 0 failed · 0 ignored
cold build (the `build` phase): 78.32s

| phase | status | seconds |
|---|---|---|
| tool-selftest | ok | 12.71 |
| build | ok | 78.32 |
| test | ok | 25.86 |
| doc-test | ok | 5.32 |
| compile-fail | ok | 0.5 |
| check-assets | ok | 0.13 |
| check-game-deps | ok | 0.3 |
| check-api-coverage | ok | 0.51 |
| check-api-prose | ok | 0.53 |
| api-doc | ok | 0.63 |
| example:headless_sim | ok | 1.48 |
| example:homing | ok | 1.02 |
| example-build:input_echo | ok | 3.21 |
| example:load_from_disk | ok | 1.49 |
| example:loading_gate | ok | 1.61 |
| example-verify:pong | ok | 4.89 |
| example-verify:prototype_kit | ok | 4.2 |
| example-build:quickstart | ok | 3.01 |
| example:scripted_player | ok | 1.65 |
| example-verify:slalom | ok | 3.84 |
| example:spawn_and_reap | ok | 1.6 |
| example-build:sprites | ok | 3.21 |
| example-verify:text | ok | 4.84 |
| example:ui_kit | ok | 2.45 |
| example:vec2_tour | ok | 0.61 |
| example-build:window_clear | ok | 3.01 |
| example-build:window_blank | ok | 2.96 |
| example:what_was_drawn | ok | 4.1 |
| game-verify:keifu | ok | 127.23 |
| game-verify:ninjo | ok | 439.83 |

## Verdict
A cold full `tools/test` took 742s (~12 min) against a 2h tick, with a 78.32s cold build, so it fits comfortably; the setup script needs no pre-warming. (Note: the first rustc call of the tick also installed the pinned 1.94.1 toolchain, which is not counted in the wall time.)
