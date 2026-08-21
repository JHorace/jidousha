# run-12 — Pong

Friction log, written as it happened. Author of a game, not of the engine;
read only `docs/api/*.md`, `crates/jidousha/examples/`, and the `make-game`
skill.

## Log

### F1 — the skill points at a file that does not exist
`make-game` step 5 says "`examples/pong/controller.rs` is it worked against an
opponent." There is no `crates/jidousha/examples/pong/` in the repo — that is
the directory I am being asked to *create*. So the one worked example of a
controller-vs-opponent is the thing I am supposed to produce. Noticed before
reading any document, while listing `examples/`. `examples/slalom/` exists and
is the single-player worked path.

### F2 — the NaN-safe comparison recipe does not cover one condition
`jidousha-api.md`'s `neg_cmp_op_on_partial_ord` passage is excellent and I
followed it in `paddle_contact` — three positively-named conditions, one `!` on
the conjunction — and it was clippy-clean first time. But `predict_crossing` has
exactly *one* condition to test (`distance > 0.0`, where NaN must answer "no"),
and there the recipe collapses back to `if !(distance > 0.0)`, which is the
literal shape the lint rejects:

```
error: the use of negated comparison operators on partially ordered types ...
   --> rules.rs:246:8
246 |     if !(distance > 0.0) {
```

The fix is to bind the condition to a name first (`let reachable = distance >
0.0; if !reachable`), which is what "lift the negation off the comparison"
means in the degenerate case — but the document only demonstrates it with `&&`
in the middle, so the one-conjunct case reads as if the lint would not fire.
One clippy round to find, one line to fix; would have been zero if the passage
said "bind it to a name" rather than "negate the whole conjunction".

### F3 — nothing says whether an example directory needs a `Cargo.toml` entry
`jidousha-api.md` says `cargo run --example <name>` picks up
`examples/<name>/main.rs` "with no `[[example]]` entry in any `Cargo.toml`".
That is stated, and it is true, but the error you get *before* `main.rs` exists
is `error: no example target named 'pong' ... a target with a similar name
exists: 'homing'`, which reads exactly like the missing-manifest-entry error.
I created `rules.rs` first and hit it. Not a document gap so much as a
misleading cargo message the document could pre-empt in one clause.

### F4 — the first `--verify` run found three real faults, and one diagnosis was misleading
Good news first: the check file worked as an instrument on its first run. It
reported, in one go: (a) the court border drawn off screen, (b) the chaser 0–0,
(c) the rollout winning only 3–0 in 5400 ticks. Collecting failures rather than
exiting on the first is worth every line the testing document says it is —
exiting first would have shown only (a) and hidden both game-design faults.

**(a) is on the documents.** A `ctx.line` of thickness `t` drawn *along* the
camera edge extends `t/2` past it. `jidousha-api.md` documents
`line(from, to, thickness, ...)` and says nothing about which side of the
segment the thickness goes — I assumed it was centred, which is right, but did
not carry that through to "so a border on the boundary overflows". The failure
message was excellent and named the exact quad
(`Vec2(-16.0, -9.07)..Vec2(16.0, -8.93)`), so it cost two minutes. `TextStyle`
gets a whole paragraph on its vertical metric for exactly this reason; `line`
gets no equivalent sentence about its thickness.

**(c) is a real gap in the controllers document's diagnosis table.** My three
numbers came back `met 31 of 31` (healthy), `shots landed 0.00 from where they
were planned` (healthy), `planned returns aimed to land 1.24 from the opponent`
(small). The table's row for that is "N healthy with Y and X both small: it
hits where it aimed and its aims are not threats, **so the objective is
wrong**." My objective is the document's own prescription for a predicting
opponent — run the opponent's rule forward and score the landing against where
that puts it — so the objective was not wrong. The real reading is that the
*game* admits no threat: the best shot available is 1.24 units against an
opponent whose paddle reaches 1.88, so no shot exists at all. The table cannot
distinguish "your objective is wrong" from "the game has no threat in it", and
the only thing that does is game-side arithmetic the document does not supply
for a predicting opponent — the closed form it gives is explicitly for one that
*chases*. I spent a round assuming my controller was at fault, which is the
document's own advice ("suspect the controller first") pointing the wrong way.

### F5 — `neg_cmp_op_on_partial_ord` bit twice, in the same one-conjunct shape
Second occurrence of F2, in `controller.rs::opponent_reach`. Writing the
NaN-safe guard the way the API document shows it is a reflex I now have, and it
is wrong every time there is only one condition. Two clippy rounds spent on one
sentence the document could carry.

### F6 — tuning by sweep works, and the game's numbers had to move a long way
The first playable-looking set of constants produced 0-0 matches with
seventy-touch rallies. The controllers document's remedy — "the opponent meets
the ball a fixed distance off its own centre" — I had already implemented before
running anything, and it was **not enough on its own**: the returns carried an
angle but the angle was not steep enough to outrun a 17-unit paddle, so both
sides still tracked. What actually moved it was the API document's coupling —
"a game that plays too slowly is not fixed by raising the speed; it is fixed by
thickening the thing the fast body must not miss, *then* raising the speed" —
paddle thickness 0.64 -> 1.0, then ball speed 26 -> 42. That paragraph earned
its place; I would not have found it by tuning.

Sweeping was two nested `sed` loops over the two opponent constants,
recompiling each time, reading the three verdict lines. The testing document
offers a `Tuning` resource for this and says a game with two numbers should not
take it — correct, the sed loop was fine, eight rows in about two minutes. What
the document does not say is that the sweep wants the **verdict lines** as its
objective function, and those come from the controllers document, which is a
different file. The two halves of "how do I tune this" are in two documents and
neither points at the other for this purpose.

Final row: `rollout 5-0, chaser 4-5, idle 0-5` — which is, exactly, the
signature `jidousha-controllers.md` opens with. That was reassuring enough to be
worth saying: the target was legible before I had a game that hit it.
