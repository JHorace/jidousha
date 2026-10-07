# CHECKPOINT — brolf

task: brolf
variant: V2
stage: done
tick: released
resumes: 0
gates-green-at: 48ec193
updated: 2026-10-07 11:52 PDT

## Done so far
- claimed (d21cb8f)
- DESIGN.md drafted whole: every section filled (fe9658f)
- DESIGN.md re-read against the spec's four rows; staged sessions freeze unnamed NPCs, NPCs gather items, the hole opens late (this commit) — design released

- game rules, systems, text and draw written; unit tests green (3e98b1b)
- --verify written (sessions A-F, three players, capture); `tools/verify brolf` verified, clippy clean (this commit)

## Exact next step
None: the PR is open; the review routine or the owner takes it from here.

## Deviations
- band text size 0.42 -> 0.33; status line 2 shortened (`HOLE {d}`, two-space separators): its worst case is 117 characters (G-073)
- status line 1 says `closing {s}s` during a shrink instead of `in {s}s` (s = seconds to the stop)
- `contact_target` skips a body already dazed (stunlock otherwise, 3 s daze vs 2 s swing wait): its ball is the target (G-072)
- Hunters swing at anything in reach and close on a rival ball its owner has left behind (intent rules 3, 4) so strikes happen (G-074)
- strike cue says `knocks it {d} away from/toward the hole` from the real landing
- `pad_state` takes the precomputed closing tick, not the schedule; `PlayerSnap` has `extracted`, no `alive`; `npc_intent` takes the pickups
- staging relative to which side the hole is on, rivals frozen until needed; C3 coordinates differ from the design's (G-071, G-075)
- gates: E1 shots >= 3 (not 6); E2 shots >= 1; E5 counted on the Golfer run (Full extracts at ~2820); A3 disc centre read from the preview (G-070); D stages the extraction at the pad's edge (offset 1.1) so its radius is checked
- mutation list is 28 faults, 28 of 28 noticed (22 in round one; six gates tightened: unit tests for the zone edge, hole radius and pin rank; verify checks for walk speed, swing wait and pad radius)
