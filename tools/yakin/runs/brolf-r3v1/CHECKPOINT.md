# CHECKPOINT — brolf-r3v1

task: brolf-r3v1
variant: V1
stage: done
tick: released
resumes: 0
gates-green-at: e943e29
updated: 2026-10-07 21:52 PDT

## Done so far
- claimed
- inline DESIGN.md written (V1)
- games/brolf-r3v1 builds and plays; tools/verify brolf_r3v1 passes (81 checks); fast gate clean

- mutation round: 22/23 first pass (B23 helmet escaped), contract check added, B23+B24 2/2 — 24/24 overall
- FINDINGS.md G-070..G-073; build-web + serve-web --check pass

- full gate green at e943e29 (tools/test 1492 passed); PR https://github.com/JHorace/jidousha/pull/139 opened

## Exact next step
None — done. The PR awaits review; never merge from a tick.

## Deviations
- cups are single-use and open one at a time (CUP_OPENS 0/12/24/36/48 s) instead of once-per-golfer: first build had a champion in 7 s
- shots carry seeded dispersion (8 deg / 12% at full power) and the landing area is drawn as a ring around the landing marker
- a clubbed golfer is guarded from clubs for 120 ticks after the stun (GUARD_TICKS): first build stun-locked golfers forever
- timers run down for every golfer before anyone acts (stun lasts 180 ticks whoever dealt it)
- the aiming check reads the first aim with the zone ring on the course (the r36 ring lies off the course)
