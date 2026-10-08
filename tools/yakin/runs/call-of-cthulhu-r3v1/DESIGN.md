# DESIGN — call-of-cthulhu-r3v1 (V1, inline)

The design is `games/call-of-cthulhu-r3v1/DESIGN.md`: the beings, the call
structure, the day loop and the sanity arithmetic.

Files (all inside the Fence): `games/call-of-cthulhu-r3v1/{Cargo.toml,
DESIGN.md, FINDINGS.md, mutants/r1.txt, src/*.rs}`, `Cargo.lock` (this
crate's entry), this run folder.

Order: pure rules + content (`rules.rs`, `lore.rs`), the state machine
(`play.rs`), the screen (`screen.rs`) and systems (`main.rs`), then
`verify.rs` (decision-row checks, three players, layout floors), the
mutation round, the capture, the gates.

Done-when checks: DESIGN.md names the four things (read it);
`tools/verify call_of_cthulhu_r3v1` → `pass` with a `row 1` and a `row 2`
check; scholar wins and mute loses in that run; `git diff --stat
origin/main...`; `tools/build-web` + `tools/serve-web --check`; the PR.
