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

### F7 — the mutation round found four holes, and the testing document predicted all four
Seventeen one-line faults, first round: **13 caught, 4 missed, 0 failed to
compile**. The four escapes:

1. **`rebound` flipped in y.** Ball leaves *down* when struck high. Every check
   passed and the match was byte-identical in shape, because my controller
   *calls* `rules::rebound` to plan its shots — so a wrong rebound is
   consistently wrong on both sides of the court and in the check as well. This
   is precisely the document's "a check that reads the game's own answer back
   cannot see that answer change", and I had read that passage and still wrote
   it. Fixed with a contract check stating the requirement in literal numbers.
2. **Paddle 62% taller.** `each_paddle_is_drawn_where_it_stands` builds its
   expected rectangle from `PADDLE_HALF_Y`, so it moved with the constant. The
   `SCORE_TOP` failure the document says three of four runs make, in a different
   costume. Fixed with "a paddle covers less than a quarter of the court".
3. **`OPPONENT_PLACEMENT` to 0.0** — the exact degenerate groove the controllers
   document is about. My chaser check asked "did it score" and "did it win", and
   the groove answers 4–0 to both: it scored, it did not win. What actually
   distinguishes a groove is that the **match never finished** — 4–0 after 5400
   ticks with a 60-touch rally. Neither document names "the match must end" as
   the check, and it is the one that sees this.
4. **`in_front` in the sweep replaced by `true`.** My three contract cases were
   the document's three — through, past the end, leaving through the same face —
   and none of them covers a ball *already behind the paddle and still going
   away*, which is the only case `in_front` guards. The document lists its three
   cases as though they were the set; they are a set.

Second round after fixing: **17 of 17.** The round is worth every minute the
document claims. Making a missed search-and-replace a hard error caught nothing
this time but cost four lines, and a mutation that failed to compile would have
been scored as a catch without the check.

### F8 — the printable-string check could not reach the strings the game draws
The em-dash mutation was caught, but by the *glyph count*, not by
`every_drawn_string_is_printable`. That check reads a `BANNERS` array in
`verify.rs` — a second copy of the literals — so mutating the literal in
`main.rs` left the check inspecting the old text. The testing document says to
run the check "over every literal a game draws" and does not say how a check
reaches them; `prototype_kit` solves it by exposing `readout_text` as a function
so a check can ask the game for its exact string, and that solution is in the
example rather than in the document. Fixed the same way.

### F9 — I walked into the `git checkout` trap the testing document names, at step 7
The document is explicit: "commit **every** file the harness touches, not just
the one holding the checks: the revert eats an uncommitted fix in the *game*
file just as happily." I did commit before the mutation round, exactly as told.
Then step 7's closing procedure — "break the game on purpose and look again" —
had me delete a `ctx.text` call from `main.rs`, re-capture, and `git checkout --
main.rs`, which silently reverted an uncommitted `banner_for` refactor from F8.
It surfaced as `cannot find function banner_for` inside `tools/verify`, in the
report rather than the terminal.

So the commit-first rule is attached to step 6 in both the skill and the
document, and step 7 needs it just as much: the capture procedure is a mutation
round too, with the same revert. Neither says so.

The report file being ground truth was worth it here — the terminal printed
`FAIL — pong` and one line; `target/verify/pong.json` had the compile error.

### F10 — `HeadlessSim::draw()` is a dead end for a check, and the API document points at it
`jidousha-api.md`, in Concepts: "a check that wants a frame asks for one, with
`HeadlessSim::draw()`." I went looking for that and it returns `&Submissions`,
whose `quads()` yields `Quad` — and `Quad` has no `bounds()`, no `contains()`,
no `covering()`. Every instrument a check actually uses lives on `FrameRecord`,
which only `FrameRecorder::draw` produces, and the testing document never
mentions `HeadlessSim::draw` at all. So there are two ways to run the Draw
phase, the API document recommends the one a check cannot use, and the two
documents disagree by silence. I used `FrameRecorder` and never called
`HeadlessSim::draw`.

### F11 — small things I guessed at
- **`ctx.line`'s thickness is centred on the segment.** Nothing says which side
  it goes. `TextStyle` gets a whole paragraph on its vertical metric because
  every vertical number rests on it; `line` gets nothing, and a border is the
  first thing a Pong draws.
- **`Vec2::move_towards` has no scalar twin.** `vec2_tour` names it as "a
  chasing opponent in one line", and a paddle chases in *one axis*, so the one
  line becomes `move_towards(pos, Vec2::new(pos.x, target), step).y` or a
  hand-written scalar version. I wrote both, in different files, before
  noticing. `f32` is not a `Vec2` operation and the document says so about
  `signum`; it could say it here too.
- **Nothing states whether `Round`-style resources survive `insert_resource`
  during staging.** They do — `insert_resource` replaces — and the reference
  says "replacing any of the same type", so this one was answerable. Noting it
  only because staging is where I looked for it and Concepts is where it is.

### F12 — what the skill's order got right, and the one thing it cannot ask for
Three orderings paid for themselves and I would have got each wrong left to
myself:

- **Free functions before the first system.** `rules::paddle_contact`,
  `rebound`, `predict_crossing` and `opponent_target` were free functions from
  the first draft because the skill said so at step 2. The controller in
  `verify.rs` calls all four, and `opponent_reach` is a forward model built out
  of `opponent_target`. Retrofitting that would have been a rewrite of the main
  loop, exactly as advertised.
- **The controllers document before tuning any constant.** I had `OPPONENT_SPEED
  = 12.0` chosen by feel and it looked fine; the document's closed form said it
  cleared the slow end by six percent, and that is the number that decides
  whether the game exists. I would have shipped 12.0.
- **Clippy after the first hundred lines.** Two of the lints I hit
  (`neg_cmp_op_on_partial_ord`, `too_many_arguments`) wanted structural changes,
  not cosmetic ones.

**The one thing neither the skill nor the documents can give an unattended run
is the thirty-second bar.** "Play it when a display exists" assumes a person.
I drove the windowed build under Xvfb with `xdotool` and screenshotted it, which
proves it opens, takes keys and draws — and says nothing about whether it is
fun. The closest available proxy is the controllers document's three verdict
lines (`rollout 5-0, chaser 4-5, idle 0-5`), and that proxy is genuinely good:
it is what caught the game being a groove. Neither file says "this is your
substitute for playing it", and it should, because an agent will otherwise
either skip the bar or claim to have met it.

## Verdict

Pong plays. `rollout 5-0, chaser 4-5, idle 0-5`; 17 of 17 injected faults
caught on the second round; `tools/test` green at 755 passed;
`tools/serve-web pong --check` green; the PNG is a picture of Pong and follows
the game when the game is broken.

Nothing in this run was blocked, and I read no engine source. The two documents
gaps that cost the most were F2/F5 (the NaN comparison recipe, two clippy
rounds) and F4 (the controllers diagnosis table pointing at the controller when
the fault was in the game, one round).
