# FINDINGS — brolf-r3v2

Entries in the shape of `docs/internal/e0-findings.md` (make-game §C). G-numbers continue from the largest
heading on `main` (G-069) after this tick's earlier game, `stack-card-game-r3v2` (G-070–G-072, PR #137).
G-068's fork risk applies: a concurrent run may take the same numbers.

Reading note: `docs/api/` (all five) and `crates/jidousha/examples/prototype_kit/`, read earlier in the same
tick for another game, plus `games/keifu/Cargo.toml` for the manifest. Nothing under `crates/*/src/`,
`docs/internal/` or `docs/adr/`. The design stage filed no findings.

### G-073 — the cup was open from tick 1, and PIKE's first DRIVE holes out on tick 83

Class: design misled · Session: brolf-r3v2, implement stage · Owner: the design stage (yakin V2)

**Doing:** building DESIGN.md as written and running its gates.

**Expected:** a match in which the zone, contact and extraction matter. The spec's scope line asks for NPCs
"good enough to make the zone, contact and extraction matter", and DESIGN.md says "the funnel and the hole
are the same point, so late play is contested by construction".

**Happened:** every match ended `Lost { by: 2 }` on tick 83, which made gates 1, 3 and 4 and the idle player's
`Eliminated` all impossible. A ball sinks when it passes within `CUP_RADIUS` of the cup slower than
`SINK_SPEED`. For a DRIVE aimed straight at the cup that holds at any start distance from 19.1 to 21.5
units, and PIKE's ball starts 21.0 from the cup. The design checked roll lengths (9.98, 3.04, 20.91) but not
which distances sink. **What I did:** the spec wins, so the cup now opens at tick 7200 (`CUP_OPENS`, when the
zone reaches 12). `roll_out` takes the tick of its first step, so the aim line still predicts the sim
exactly; HUD line 2 shows `cup 1.0u (opens 45s)`, and the contracts check that the cup stays shut before
it opens. A real deviation, listed in the PR. Cost: one diagnosis cycle. **Fix:** a design's pass over a
physics rule should ask which starting states end the game at once, not only how far a shot rolls.

### G-074 — the Hunter's rule locks its victim down for good

Class: design misled · Session: brolf-r3v2, implement stage · Owner: the design stage (yakin V2)

**Doing:** reading why the good player made 3 shots in a 7,200-tick match.

**Expected:** a club to be "a temporary disable, never a direct permanent kill" (the spec).

**Happened:** the Hunter's rule ("a rival golfer within 8 units → walk at them, contact when clubbable")
swings again every `SWING_COOLDOWN` (60 ticks). A club disables for `CLUB_STUN` (180) and resets an existing
disable, so the victim never gets a free tick. **What I did:** prey is now a rival that is not disabled
and holds loot a club would knock loose (looting is the incentive DESIGN.md gives aggression). The contact
rules are unchanged. The match run now asserts that no golfer stays disabled longer than one club (180
ticks) without a free tick; mutant X2 proves that check bites. **Fix:** a design that pairs a cooldown with
a disable should compare the two numbers.

### G-075 — the good player loses at the cup's opening (the game's own)

Class: the game's own · Session: brolf-r3v2, implement stage · Owner: the next session on this game

**Doing:** the three-player table on seed 7.

**Happened:** `good Lost { by: 1 } t=7200 | chaser Lost { by: 1 } t=7212 | idle Eliminated t=3634`. All four
golfers park their balls around the cup before it opens, and whoever is in a sinking band on tick 7200
wins. ROOK was, both times. The good player's numbers are healthy (55 of 58 shot windows taken, shots
resting 0.00 from where planned), so per `docs/api/jidousha-controllers.md` it is the game and not the
controller that decides this. **What I did:** nothing beyond G-073; the gates do not ask who wins.
**Next:** the endgame at the cup wants a design pass. For example, the cup could open on a ball that
comes to rest in it rather than on one passing over it, or shots could be refused within a radius of the
cup until it opens.
