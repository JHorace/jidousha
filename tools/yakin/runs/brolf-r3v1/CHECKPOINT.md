# CHECKPOINT — brolf-r3v1

task: brolf-r3v1
variant: V1
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 21:34 PDT

## Done so far
- claimed
- inline DESIGN.md written (V1)
- games/brolf-r3v1 builds and plays; tools/verify brolf_r3v1 passes (81 checks); fast gate clean

## Exact next step
Write games/brolf-r3v1/mutants/r3v1.txt and run `python3 tools/mutate brolf-r3v1 mutants/r3v1.txt`; then FINDINGS.md, build-web/serve-web --check, full gate, PR.

## Deviations
- cups are single-use and open one at a time (CUP_OPENS 0/12/24/36/48 s) instead of once-per-golfer: first build had a champion in 7 s
- shots carry seeded dispersion (8 deg / 12% at full power) and the landing area is drawn as a ring around the landing marker
- a clubbed golfer is guarded from clubs for 120 ticks after the stun (GUARD_TICKS): first build stun-locked golfers forever
- timers run down for every golfer before anyone acts (stun lasts 180 ticks whoever dealt it)
- the aiming check reads the first aim with the zone ring on the course (the r36 ring lies off the course)
