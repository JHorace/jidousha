# CHECKPOINT — keifu-x-inheritance

task: keifu-x-inheritance
variant: V2
stage: implement
tick: live
resumes: 0
gates-green-at: none
updated: 2026-10-07 09:14 PDT

## Done so far
- branch claimed (32c3d1d); mainline Keifu's spec, source and docs/api read (heartbeat 1743f0e)
- DESIGN.md written and released: the delta design (family/outsiders, six traits, the black mark, one inheritance function), the three decision rows elaborated with their panels, commit inputs, one-functions and gates G0..G15

## Exact next step
(S0 landed; verify passes on the pure copy.) Implement stage, WORKER.md. First act, its own commit (DESIGN.md S0): `cp -r games/keifu games/keifu-x-inheritance`, rename the package to keifu-x-inheritance, the window title, the verify verdict line, every capture file name (keifu-*.png -> keifu-x-inheritance-*.png), and the asset root string games/keifu/assets -> games/keifu-x-inheritance/assets; then `python3 tools/verify keifu-x-inheritance` must pass on that commit before any variant rule is built. Then S1..S5 in DESIGN.md's order.

## Deviations
- none
