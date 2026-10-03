# ninjo — what `docs/api/` cost

The findings this build owes back, in the format `docs/internal/e0-findings.md`
uses (`make-game` step 9). G-numbers continue giri's sequence (its
`FINDINGS.md` ends at G-009); the fork inherits giri's open workarounds —
G-008's `?constants=` location reading rode along in `src/web.rs` unchanged
and is not re-counted here.

**Reading discipline:** this fork was written from `docs/api/` (all four),
`crates/jidousha/examples/`, and giri (a game, not the engine — at
`games/giri/` then, `attic/giri/` since wave 1.1). No
file under `crates/*/src/` was opened, and neither was `docs/internal/` nor
any ADR but 0038 and 0041 (both named by the handoff). Wave 0b held the same
line: `games/giri/` (the port source) and this crate, and nothing under
`crates/*/src/`.

**The selection-bugfix session (2026-09-06) read** `CLAUDE.md`, the
`make-game` skill, this game's `UI.md`, `DESIGN.md`, `GDD.md`, `FINDINGS.md`
and its whole `src/`, and nothing else: no file under `crates/*/src/`, no
`docs/internal/`, no ADR, and it asked `docs/api/` nothing — every question it
had was about this game's own UI state, and the two entries it files below are
about this repository rather than about the engine's documents.

**The double-drawn-cast session (2026-09-08) read** `CLAUDE.md`, the
`make-game` skill, this game's `UI.md`, `FINDINGS.md`, `screens/README.md` and
its whole `src/`, plus `docs/api/jidousha-api.md`'s `Vec2` tour (which of
`distance`/`length_squared` to reach for, and whether `normalize` on a zero
vector is safe — it is not, and the placement never normalizes) and
`docs/api/jidousha-testing.md`'s `DrawnQuad` entry. Nothing under
`crates/*/src/`, no `docs/internal/`, no ADR but 0041, which the handoff names.
Its one new entry is about this repository's own documents.

**The candidate-picker session (2026-09-10) read** `CLAUDE.md`, the
`make-game` skill, this game's `UI.md`, `FINDINGS.md`, `screens/README.md` and
its whole `src/`. No `docs/internal/`, no ADR, and it asked `docs/api/` one
thing — whether `Rect::overlaps` counts touching edges, since the new controls
had to be packed into the width the job row gave up. It does not
(`jidousha-api.md`'s `Rect` reference says so in the signature's own comment:
"touching edges do not count"), and the answer was accurate and easy to find.

**A reading-fence slip, recorded because the fence is the exercise**
(`make-game` §0.1): the session opened
`crates/jidousha-core/src/visual.rs` for that one line **before** looking in
`docs/api/`, where the answer was. Two lines of engine source were read and
nothing came of them that the documents did not also say — but "the document
did answer it and I looked in the wrong place first" is the failure the fence
exists to measure, so it is written down rather than left out. It is not a
document finding: nothing was missing and nothing misled.

**The drawers-and-work-list session (2026-09-11) read** `CLAUDE.md`, the
`make-game` skill, this game's `UI.md`, `FINDINGS.md` and its whole `src/`. No
file under `crates/*/src/`, no `docs/internal/`, and no ADR. **It asked
`docs/api/` nothing**, and that is a real answer rather than an empty section:
every question it had was about this game's own UI state and its own layout
arithmetic, and the one engine fact it needed — how wide a string is at a size
— it took from `theme::text(..).width_of`/`columns_in`, which the game has
called on every surface since S1. Its four entries below are all about *this
repository*, and the last of them names a class rather than a defect.

**Wave 1.3 (needs + settlement, 2026-09-13) read** `CLAUDE.md`, the
`make-game` skill, this game's own `GDD.md`, `DESIGN.md`, `UI.md`, `CAST.md`
and `FINDINGS.md`, and its whole `src/`. It opened no file under
`crates/*/src/`, no `docs/internal/`, and no ADR. **It asked `docs/api/`
nothing new** — the economy is arithmetic over the game's own data, its one new
surface is a `Panel` like every other, its two new inputs are clicks on
rectangles `layout.rs` already knew how to state, and its three new
occurrences ride the one scheduler S1 landed. Its four entries below are all
about *this game*.

**Wave 1.4 (resolution, 2026-10-02) read** `CLAUDE.md`, the `make-game`
skill, this game's own `GDD.md`, `DESIGN.md`, `UI.md`, `CAST.md` and
`FINDINGS.md`, and its whole `src/`. It opened no file under `crates/*/src/`,
no `docs/internal/`, and no ADR. **It asked `docs/api/` one question** — what
`Rng` offers, for a roll that has to be addressed by the occurrence rather than
drawn in call order — and the reference answered the signatures and not the
one property that mattered (G-045). Its other entries are about *this game*:
the finding it closes (G-035), the drawer ceiling it re-laid (G-034, closed),
a sentence that should have been derived and was not (G-046), the second
ceiling the same wave met (G-047), the number Steve's claim allows the share
to be (G-048), and the question it leaves open about risk (G-049).

**Wave 1.5 (petitions, 2026-10-02) read** `CLAUDE.md`, the `make-game` skill,
this game's own `GDD.md`, `UI.md`, `CAST.md` and `FINDINGS.md`, the sections of
`DESIGN.md` the clock and the scheduler live in, and the `src/` files the
petitions land on — the scheduler, the people, the needs, the asks and their
messenger, the scorer, the lens, the attention table, the flow, the layout, the
floors, the sweeps and the harness. It opened no file under `crates/*/src/`, no
`docs/internal/`, and no ADR. **It asked `docs/api/` nothing**: the module is
arithmetic over the game's own data on the one scheduler S1 landed, its two
surfaces are `Panel`s like every other, and its roll is G-045's mix, already
written. It also read the approved architecture mockup (the claude.ai artifact
"ninjo petitions mockup", via the artifact tool) and photographed it with the
headless browser `tools/serve-web` names, through Node's Playwright, because its
interesting states are behind a clock and two taps. Its entries below are all
about *this game*: what it closes (G-033, G-047, and G-034 a second time) and
what the petitions exposed (G-050 to G-053), plus one about its own harness
(G-054).

**Wave 1.6 (the injector and the scenario file, 2026-10-02) read**
`CLAUDE.md`, the `make-game` skill, this game's own `GDD.md`, `CAST.md`,
`UI.md` and `FINDINGS.md`, and the `src/` files the injector lands on — the
petitions and their record, the scheduler, the people, the flow and its
scenario load, the module registry, the constants and the drawer, the
attention table, the card, the economy and petitions batteries, the
conductor, the capture and the harness. It opened no `docs/internal/` and no
ADR. **It asked `docs/api/` one thing** — `Rng::below`'s signature, for the
director's draws (`jidousha-api.md`'s reference answered it, `u32` in and
out) — and its addressing is G-045's mix, already written. **A reading-fence
slip, recorded because the fence is the exercise** (`make-game` §0.1): looking
for how wide a clipped card line may run, the session ran a `grep` for
`fn columns_in` across `crates/*/src/` before `docs/api/`, which printed two
signature lines of `crates/jidousha-render-core/src/font/style.rs`. Nothing
came of them — the answer it used is the game's own `panels::clipped`, and
`jidousha-api.md` carries the same two signatures — so it is not a document
finding, but "looked in the engine first" is what the fence measures. Its
entries below are all about *this game* (G-056 to G-059).

**The wave-1 sanitation pass, session 1 — the UI exemplar audit
(2026-10-02) — read** `CLAUDE.md`, `docs/templates/SANITATION.md`,
`docs/agent-practices.md` §2.5, `docs/conventions.md` §Documents,
`docs/implementation-plan.md` §2, the `make-game` skill, this game's `UI.md`
whole and `FINDINGS.md` whole, `GDD.md` §8's wave-plan lines, and the UI side
of `src/` whole — the flow, the screens, the panels, the board and its picker,
the work list, the card and its clicks, the tuning drawer, the meters, the
ledger, the settlement panel, the floors, the frames, the layout's rectangles,
the attention table, the lens's header, and the harness that photographs them
(the sweep's conductor, `verify.rs`'s UI batteries, `shots.rs`, `pleashots.rs`,
`eventshots.rs`, `capture.rs`). It opened no file under `crates/*/src/`, no
`docs/internal/`, and no ADR; the two tool scripts it read (`tools/verify`,
`tools/test`) were read to know what a transcript contains. **It asked
`docs/api/` nothing.** The one document it could not find is the "process
doc §13" the handoff cites for the extraction rule, which is not in this
repository; the rule applied is the handoff's own statement of it (three or
more call sites), and that is a note rather than a finding. Its entries below
(G-060 to G-065) are all about *this game* and its harness, and the audit's
report — the promotion list, the hunt and what was considered and left alone —
is in the pull request.

## The wave-1 sanitation pass, session 1: the UI exemplar audit (2026-10-02) — **6 new findings**, all the game's own

The pass G-031 asked for. Report-first and transcript-identical: the report
was written before the first edit, the five suspected states were staged
through the real click harness by a throwaway test module that was deleted
before any change, and `tools/verify ninjo` and `tools/verify keifu` are
byte-identical before and after (the pull request carries both). Three of
the five fixes are one line each, which is the signature of the class: the
fix is never hard, only unfindable from inside one wave.

### G-060 — the class, fifth instance: the left column is five fields, and three bites were in it

Class: **the class** (G-031's shape, one layer down from the drawers) · Game:
ninjo · Files: `games/ninjo/src/flow.rs`, `src/screens.rs`, `src/verify.rs`,
`UI.md` §3f, §3g · **The three bites closed here; the collapse open — an
owner call**

UI.md §3g states the fact once: the left of the screen is one surface at a
time — the job board, the candidate picker, the work list, the settlement
panel, the faces list. `Flow` states it five times: `board`, `picking`,
`listing`, `works`, `drilled`, kept in step by four hand-written clearings
(`open_the_works`, the marker click, the meter-chip click, the work chip) and
two tick rules. Each clearing was correct on the day it was written. The
settlement panel arrived in wave 1.3 and was added to the marker's list and
to its own, and not to the two that predate it:

- **A meter chip drilled over the settlement panel drew both.** The chip's
  click clears `board` and not `works`; the frame carried the faces list's
  title and the panel's, the faces rows took the click, `floors::controls_for`
  named the settlement set, and judging that frame against it reported seven
  problems (faces text across BUILD and the wage steppers). Reachable in two
  taps: the camp's marker, then any chip. G-027's shape exactly — the thing
  drawn, the thing clicked and the thing judged were three different lists.
- **The sheet's work chip did nothing with the settlement panel up.** Its
  clearing names the board, the picker, the drilled chip and the band — the
  siblings as of 2026-09-11 — and `put_the_list_away` then dropped the list on
  the same tick because `works` was true. The chip never lit, no list drew, no
  notice was written: a tap that did nothing and said nothing, which is the
  failure CLAUDE.md's third convention forbids.
- **Twelve map words were drawn under the work list.** `screens::content`'s
  `worded` silenced the map under the board and the works and not under the
  list, which takes the board's rectangle (§3f). Hidden by the list's fill,
  so no photograph showed them; present in the `Panel`, so `frames::judge_chrome`
  found every one on the frame and no floor said a word.

Expected: one value. Happened: five, and the three places a later surface
was not added. **What closed here** is the three bites — `works` goes down on
a drill and on the work chip, and `worded` names the list — with
`verify::the_left_column_is_one_surface` walking both paths through the real
handles and `map_labels_are_governed` asking the label rule of all three
surfaces that take the board's rectangle. **What is open** is the remedy
G-027 got: `Flow::column: Option<Column>` — `Board(site)`, `Picker(JobId)`,
`List(who)`, `Faces(chip)`, `Works` — one `match` in the render, the click
routing and `controls_for`, which also retires G-031's four tick-checked
pairs. It touches every reader of the five fields and it is the owner's call
(the pull request's Q3).

### G-061 — the game's own: the breakdown band carried a slot, and a marker that replaced the board left it explaining a row nobody tapped

Class: **the game's own** (the class, in the band) · Game: ninjo · Files:
`games/ninjo/src/flow.rs`, `src/panels.rs`, `src/board.rs`, `src/floors.rs`,
`UI.md` §3c · **Closed by this session**

`Breakdown::Job` held a slot. UI.md §3c says, of the picker, exactly why that
is not enough: a slot alone is a number read against whichever board happens
to be open, and a marker outside the surface's own rectangle is still
clickable. The band's orphaning rule asked `board.is_none()`, which a
*replacement* never satisfies.

Expected: the band goes with the board it explains. Happened, staged through
the harness: board A open, somebody selected, the `?` tapped on row 1, then
the Black Vault's marker — which lies under the character panel's body at the
reference camera, and the body falls through (§3a). The board became the
Vault's, the band stayed `Job(1)` and recomputed itself as `WHY - Bob would
take it the vault door`, and a gold `?` lit on the Vault's row 1, which
nobody had tapped. On a board whose row 1 is not open the band draws nothing
while its gold frame stays up and swallows clicks.

**The fix is the picker's shape**: `Breakdown::Job(JobId)`, and
`Flow::put_the_band_away` compares sites; `panels::read_breakdown` refuses a
board that is not the job's. `verify::the_band_belongs_to_its_surface`'s
marker way is what asserts it — once that way reached a marker (G-062).

### G-062 — the game's own: a battery's way out was a posting, and the check passed for the wrong reason

Class: **the game's own** (the harness; a check that passed for the wrong
reason) · Game: ninjo · Files: `games/ninjo/src/verify.rs`
(`the_band_belongs_to_its_surface`) · **Closed by this session**

The band battery walks four ways out of a board with the band up and asserts
the band is gone after each. Its fourth way, "another site's marker, which is
a board about something else", clicked the Deep Cave's marker — which at the
reference camera lies **inside the open board's rectangle**, and the board
swallows its own rectangle on purpose (§3c). The click landed on a job row and
made a posting; the posting put the selection and the board down; the band
went because of that; the check passed. The one way written to prove the band
survives a *replaced* board never replaced a board, which is why G-061 lived
under a green check.

Expected: a way out named "another site's marker" reaches a marker. Happened:
it reached a row. **Done on its authority:** nothing beyond trusting the green
— the probe that staged G-061 is what read the four events. The way now
clicks the Black Vault's marker (under the panel's body, which falls through),
carries the board it expects to find, and asserts no event was emitted, so it
cannot pass by posting again. The class is worth naming for the next battery:
a scripted click that lands somewhere other than the thing its label says is
a check about a different claim.

### G-063 — the game's own: the consequence chip was a flag beside the card

Class: **the game's own** (the class; the handoff's "boolean beside an
`Option` it shadows") · Game: ninjo · Files: `games/ninjo/src/flow.rs`,
`src/card.rs`, `src/plead.rs`, `src/floors.rs`, `UI.md` §3h · **Closed by
this session**

`Flow::consequence_open: bool` stood beside the card it was about — the
ledger's focused petition or the voiced one. The ledger reset it when a
*tapped* row changed; the overlay's chip, a resume by space or a speed chip,
and the ledger's fallback focus (`card::focused`, the first row) did not.

Expected: a chip open for one card and no other. Happened, staged at seed 10
through the real clicks: Bob's overlay, the chip tapped, `3` pressed; after
the resume the flag was still true, and Steve's overlay opened two world-days
later with its chip already gold and explained before anybody tapped it.

The fix is the type: `consequence_open: Option<usize>`, the petition whose
chip is open — the same shape as `explained: Option<TraitId>`, a trait and
not a place — so the overlay and the ledger each ask "is it mine" and the
ledger's row-change reset is retired as a rule nobody needs.
`verify::the_chip_belongs_to_its_card` stages both placements with the chip
open for the right card and for the wrong one.

### G-064 — the game's own: the board's offer and its TO toggle outlive the board

Class: **the game's own** (the class; a value feeding a recorded input) ·
Game: ninjo · Files: `games/ninjo/src/flow.rs`, `src/board.rs`, `UI.md` §3c ·
**Open — an owner call**

`Flow::offer` says of itself that it "goes down with the board", and UI.md
§3c says the wage stepper "opens at the standing rate". Only `make_posting`
and `close_everything` clear it; the board's X, bare ground and a marker
switch do not, and `post_open` survives even a posting.

Expected: the next board opens at the standing rate, offering to the
selection. Happened, staged: a board opened, the wage stepped to 24g, TO
toggled to anyone, the board closed with its X, another opened — its footer
read `24g` and `TO ANY` where the standing rate for its first open row is 16g.
What is shown is what is posted (`board_wage` and `make_posting` read the same
field), so the footer and the posting agree with each other and both disagree
with the documents.

**Not fixed here**, deliberately: the remedy by construction is the board as
one value — `Option<Board { site, offer: Option<i64>, to_anyone: bool }>` —
which touches every reader of `flow.board` and changes the wage a posting is
made at on a path no script walks. That is a decision about a recorded
input's source, and the sanitation fence hands it back. `board::wage_offered`'s
two ignored parameters and its "this row's wage" doc comment are the same
item's doc-truth half, left for that audit. UI.md §3c carries a note.

### G-065 — the game's own: three handles have a box and three do not

Class: **the game's own** (the class; a hand-list beside `Drawer::ALL`) ·
Game: ninjo · Files: `games/ninjo/src/screens.rs` (`draw_chrome`) · **Open
— its own small session**

`draw_chrome` draws a ghost ground and a border under `[feed_button,
tune_button, modes_button]` — the three handles that existed when the line
was written. UI.md §3 says every handle is walked off `Drawer::ALL` "and
nothing else to remember"; the labels and the click routing are, the boxes
are not. ROSTER (1.1), LEDGER (1.2) and PLEAS (1.5) are bare text on every
committed screenshot, and no floor sees a fill.

Expected: six boxes. Happened: three. **Not fixed here**: walking
`Drawer::ALL` is one loop, and it adds fifteen quads to every frame — the map
photograph's draw transcript is printed by the verify run, so the fix moves
the transcript and is not sanitation. It wants its own commit with the
regenerated PNGs. The drawer fills are the same second-list shape
(`feed_panel()` for four drawers, where `layout::roster_panel()` and
`ledger_panel()` are aliases of it today); a `Drawer::rect()` beside
`handle()` would end both.

### G-050 — the game's own: an idle camp reaches the desperation ceiling on its fifth night, petitions or not

Class: **the game's own** (a design fact the petition horizon exposed; the
owner's) · Game: ninjo · Files: `games/ninjo/src/petitioned.rs` (`sweeps`,
`CEILING_MINUTE`) · Open

The handoff asked the idle sweep re-judged at the petition horizon with the limp
floor standing — and named `broke` the floor's hard case: "assert nobody is at
the ceiling after it". **That cannot be asserted at this horizon, and petitions
are not why.** A voicing on day two reaches its cliff on day six to eight; by
then an idle camp has been short every night since its board ran dry on day two
(G-032), and with nothing in an idle world to lower desperation (G-033's other
end is a *met* petition, and an idle player's camp meets none — `IDLE_MET` is
`(0, 0)` over sixteen worlds), Steve reads the ceiling at minute **7200**, the
fifth night. The sweep asserts the diagnosis instead of the wish:
`first_ceiling` finds the same minute **with the petitions module switched off**
(`CEILING_MINUTE`, a shipped literal both ways), so the ceiling is the
shortfalls' and the floor's three-day form (`economy::judge_sweeps`, unchanged
and green with petitions on) is the horizon it holds over. What *is* asserted
at the petition horizon: no purse overdrawn by any consequence, every walk-out
home again when its days are up, and the conservation identity over the new
ports.

Expected: a limp floor that holds while consequences land. Happened: the floor
is gone before the first cliff falls, in every idle world. The levers are all
owner decisions and none of them is the petitions': an idle settlement that
can find work after day two (G-032), a relief rule that is not a petition (the
G-033 alternative this wave was asked *not* to invent), or a floor scoped to
the days before the board runs dry, which is what it is in practice.

### G-051 — the game's own: `look-after-them` asks for paying work and is judged on desperation, which work never lowers

Class: **the game's own** (content against the economy; the owner's) · Game:
ninjo · Files: `games/ninjo/src/petitions.rs` (T3's `Condition::OtherSettled`)
· Open

`CAST.md` §6's T3 says *"Get {other} paying work before {deadline}"* and is met
when `{other}`'s **desperation** is under the threshold at the deadline. Built
as written. But the only thing in this game that lowers desperation is a met
petition of the person's *own* — wages raise a purse and leave desperation
where it is. So the player can do exactly what Hana asks — Steve on steady
posted work, a full purse — and the card still fails at its cliff, and fires
`gives-away`, and (because Steve is still desperate) Hana raises it again at
her next check and gives away half her purse every four days. The attentive
sweep shows it: in its world zero Steve is paid all twelve days and Hana's T3
fails twice, handing him 50g and then 98g.

Expected: a condition the words describe. Happened: a condition only Steve's
own petition can meet. The two one-line fixes are both content decisions:
judge T3 on `{other}` finishing paid work (what the words say), or let a wage
that clears a person's upkeep relieve a step (a second relief rule — G-033's
alternative again). The wave did neither: implementing CAST as written was the
instruction.

### G-052 — the game's own: ARRANGE on `thin-days` opens an empty work list once the board is spent

Class: **the game's own** (G-032 met from the petition's side) · Game: ninjo
· Files: `games/ninjo/src/card.rs` (`arrange`) · Open

The decision-surface row holds — ARRANGE goes where the condition is acted on,
derived from the condition, and posts nothing — and where it goes is empty.
`thin-days` is raised by a third shortfall, which an idle-ish camp reaches on
day four; the authored board ran dry on day two, a shift is not postable, and
the works' three slots are filled by whoever's rescore lands first. The
photograph `ninjo-arranged-reference.png` is that state: Steve's work list,
`0 of 0 open`, with his `thin-days` running. The pipe's arranged twin
(`petitioned::pipe`) had to put a job back on the board to have something to
post. The fix is G-032's (work that outlasts the board), not the card's.

*Seen in the browser too, and earlier:* the web playtest at `?seed=10` pressed
ARRANGE on Ludo's `collectors-visit` at **d2 20:48** and got the same empty
list — `nothing stands open anywhere - the settlement's work is all claimed or
done` — with 1203g in the treasury. So on this seed, from day two, GIVE is the
only answer a money-shaped petition has; the owner's playtest question about
obligation is being asked of a player whose one non-money gesture leads
nowhere. Not `thin-days`-only: every ARRANGE whose destination is a work list.

### G-053 — the game's own: eight or nine of ten carry a petition by day four

Class: **the game's own** (a density the mockup did not have; 1.6's to
calibrate) · Game: ninjo · Files: `games/ninjo/src/constants.rs`
(`plea_odds`, `plea_hours`), `src/petitions.rs` (deadlines) · Open

The mockup voiced five petitions over twelve days, one per motivator, and said
"a ten-person camp voices a few per week". The build voices **thirteen to
seventeen** over twelve idle days, and because a deadline runs four to eight
days and most triggers stay true (everybody is short, somebody is always
desperate), the steady state is nearly one per person: at seed 10 nine are
asking at once by day eight. The ten-row ledger then shows only the running
ones (the photographed run had to open it before the ninth was voiced so the met
row was still on it). The rate is `plea_odds` at 15% a six-hour check; the
concurrency is the deadlines, which are content. GDD §8 says 1.6 lands its
density at twice the first guess — this is already past that, and the owner
should play it before 1.6 adds the director's.

### G-054 — the game's own: a picture of what a tap did inside a pause had no gate

Class: **the game's own** (the harness) · Game: ninjo · Files:
`games/ninjo/src/sweep.rs` (`Photo::step`) · Closed in this wave

A world stopped for a voicing holds its minute through every tap the player
makes inside it — LATER, the roster, a GIVE — so a photograph of "Bob's panel
after the gift" could be addressed by neither the minute, the tick (unknown in
advance) nor the pause (it was taken on the first paused tick, before any tap).
`Photo` gained a fourth gate, `step`: photograph once that many scripted
directives have been taken up and carried out. Every existing photo carries
`step: 0`, which is "any", so nothing else moved.

### G-055 — the game's own: a stop that landed while the ledger was open said nothing

Class: **the game's own** (a surface owing its pause line) · Game: ninjo ·
Files: `games/ninjo/src/card.rs` (`pleas_drawer`) · Closed in this wave

Found by playing the web build, not by the battery. LATER opens the ledger,
and a player who stays in it and resumes is there when the next cliff falls.
`petition-failed` pauses; an open drawer silences the map's banner (UI.md §3,
by design — a banner under a scrim is a row nobody can read); the feed has a
reason line and the ledger had none. So the world stopped with only the gold
PAUSE chip to say so, and nothing on screen said whose petition had failed.
The ledger's note line now carries `attention::reason_line` — the feed
header's sentence, from the same function — in gold while the world is
stopped, and `petitioned::a_stop_in_the_ledger_says_why` asserts it over the
staged camp stopped by its failure, which the content floors judge as a fifth
petitions state. The other drawers (ROSTER, LEDGER, TUNE) still stop silently
under a pause; that predates this wave and is the next surface's to decide.

### G-056 — the game's own: the collector D1 brings is often paid on the doorstep

Class: **the game's own** (a template's shape, as the handoff specified it) ·
Game: ninjo · Files: `games/ninjo/src/petitions.rs` (D1
`the-collector-comes`) · Open

D1 is "T1's body and consequence ... eligible: anyone `indebted` - wallet
regardless". T1's condition is the purse holding `{n}` (30g) at any point
before the deadline, and T1 was only ever raised on somebody whose purse was
*under* 30g. D1 reaches the indebted whatever they hold, so when Bob is
carrying 30g or more the petition is **met in the minute it is voiced** —
"Bob paid the collector off, for now ... nobody's doing but the world's" —
and, being met, it takes a step off his desperation (`plea_relief`). In the
attentive sweep at seed 0 this happens twice to Bob in two days. The collector
arriving and being paid is a coherent story, but a pressure event that relieves
pressure is the opposite of what an injector is for. Built as specified; the
choices are the owner's: D1's condition could be *paying* 30g (the purse
falling by `{n}` — a burn, which would make it a wallet event rather than a
regard one), its eligibility could keep T1's purse clause, or a met-on-voicing
petition could relieve nothing. No new consequence kind is needed for any of
them.

### G-057 — the game's own: the director is throttled by the cast's own petitions

Class: **the game's own** (a density, measured; the playtest's to judge) ·
Game: ninjo · Files: `games/ninjo/src/director.rs` (`reach`, the cap) · Open

The one-active-petition rule holds for the director as it does for everybody,
and G-053 is still true: by day four eight or nine of ten carry a petition of
their own. So an idle camp gives the director almost nobody to reach — over
sixteen idle worlds of twelve days it voices **one or two** director petitions
a world, and most of its firings pass ("nobody an event could reach", or one
already in play at `director_max` 1). An attentive camp, whose petitions get
met and so free their carriers, gets **three to eight**. The pressure is mild
by construction rather than by tuning — which is what the MVP gate asked for,
and is also why the attention differential *widened* (four petitions to six:
the director presses hardest on the camp that is being looked after). Whether
that reads as "the world moves without you" or as "the world moves mostly when
you move it" is the playtest's question. The levers are the three drawer rows;
the cast's own density (G-053) is the larger one.

### G-058 — the game's own: the pressure params are named for the drawer's cell

Class: **the game's own** (a deviation from the handoff's names) · Game:
ninjo · Files: `games/ninjo/src/constants.rs` · Closed in this wave

The handoff names the rows `director_calm_days`, `director_period_hours` and
`director_max_active`. The drawer's name cell is 136 units — fourteen glyphs at
the small size (`floors::tuner_has_room` asserts every name fits it), and the
longest of the three is twenty-one. They ship as **`calm_days`**,
**`director_hours`** and **`director_max`**: the same three numbers, the same
meanings (`Field::meaning` says each in words), names that fit. The drawer
holds fifty-three constants now (G-059 is what that cost).

### G-059 — the game's own: the tuning drawer's fourth column filled, and the stamp swapped places with the prose band

Class: **the game's own** (a layout ceiling, the third: G-028, G-034) · Game:
ninjo · Files: `games/ninjo/src/layout.rs` (`tuner_stamp`, `tuner_foot_for`,
`tuner_hint`), `src/tuning.rs` (`stamp_text`), `src/floors.rs`
(`tuner_right_column`) · Closed in this wave

The three pressure params took the fourth stepper column to eleven rows, and
the in-effect stamp under it — a heading, six named moved constants, a count
and the seed, now with the scenario beside it — no longer fit: at fifty-three
constants the column's foot holds two rows and the floor asks for the tallest
stamp under one more constant. Found by `floors::tuner_right_column`, as it
was written to be ("the floor fails while there is still room"), before any
screenshot did. **The re-lay**: the stamp went up into the header band (three
rows, 320 wide, between the presets and APPLY) and **packs** — the seed and
the scenario on its first row, the moved constants run along the other two as
`name value` pairs, never split, with `+N more` for the rest — and the prose
band, which is one short state at a time, came down to the column's foot,
where the room shrinks as the column grows. The floor asserts both: the stamp
packs at its tallest with no row wider than its band, and the prose band's
tallest state fits under one more constant. The column has room for three
more rows of stepper before this happens again; the next ceiling is the
drawer itself (fifty-six slots), and a fifth column does not fit the screen.

 does not say whether nearby seeds give independent streams

Class: **a document that was silent on the one thing asked** · Game: ninjo ·
Files: `games/ninjo/src/resolution.rs` (`address`, `roll`) · Owner:
`docs/api/jidousha-api.md`'s `Rng` entry · Open

Doing: the resolution roll has to be **addressed by the occurrence** — the
seed, the world-minute the work completes and the job — and never drawn from
the world's `Rng` in call order (G-016's rule; a draw in order would make the
roll depend on which occurrence fired first in a tick, and the speed-invariance
sweep would catch it). The natural spelling is a fresh generator per
occurrence: `Rng::from_seed(address).below(100)`.

Expected: the reference to say whether `from_seed` scrambles its argument — so
that seeds 41, 42 and 43 (the same job a minute apart) give unrelated first
draws — or that callers should hash first. Happened: the entry gives the
signature, "create a generator from a seed", and an example showing the same
seed replays; nothing about nearby seeds. Many small generators return a
first value that is a near-linear function of the seed.

**What was done on its authority**: nothing was assumed. The game mixes the
address itself with a splitmix-style finalizer before seeding
(`resolution::address`), and the staged distribution sweep (6144 rolls a fit,
`outcomes::tally`) is what shows the result is flat enough: 11% failure at a
strong fit against the curve's 11, 35% at a poor one against 35. If
`from_seed` already scrambles, the mix is redundant and harmless; if it does
not, the mix is load-bearing. The document should say which.

### G-046 — the game's own: the fit chip's honesty sentence was hand-written, so it could not retire itself

Class: **the game's own** (against the legibility session's derivation) ·
Game: ninjo · Files: `games/ninjo/src/asks.rs` (`fit_means`, removed),
`src/resolution.rs` (`fit_means`) · Closed in this wave

The handoff for this wave said: the fit chip's explanation ("every job succeeds
until resolution lands") must retire itself **because the data changed**, and if
it had to be hand-edited, that is a finding against the legibility session's
derivation. It had to be. The trait chips' dormancy clause *is* derived —
`traits::explain` reads `Consumer::Resolution.live(modules)` and the clause
vanished the moment `resolution` entered `modules::MODULES`, with nobody
touching a sentence (`traits::vocabulary` now asserts the flip both ways). But
`asks::fit_means` was a literal written by wave 1.2's clarity rider, carrying a
wave number in prose, and the legibility session that made the trait lines
derived did not reach it.

It is now `resolution::fit_means(tuning, modules)`: with the module off it says
the trait chips' own absence clause; with it on it walks every fit the
vocabulary can produce and prints the odds-word and the numbers off
`resolution::odds` — so moving `fail_base` moves the sentence, and the
went-well clause is `resolution::went_well_means`, read off the registry the
same way. Expected: one derivation for every honesty line. Happened: two
patterns, one of them a literal. Owner: this game (UI.md §3c).

### G-047 — the game's own: the attention config met the same ceiling the tuning drawer did

Class: **the game's own** (a layout ceiling, the G-034 class) · Game: ninjo ·
Files: `games/ninjo/src/layout.rs` (`MODES_ROWS`) · **Closed (wave 1.5)**

*Closed by wave 1.5, which brought the twentieth, twenty-first and
twenty-second classes:* the config drawer has its own rectangle now
(`layout::modes_panel`, to the foot of the screen, as the tuning drawer's is),
**twelve rows at a pitch of thirty-two** — the target floor exactly, rows that
touch and never overlap — so two columns hold twenty-four, and
`floors::modes_have_room` asks about the **next** class's radios and fails one
class early, which is the floor this entry asked for. The entry below is the
record of the ceiling.

Two new event classes took the table from seventeen to nineteen, and the config
drawer held two columns of nine. Nothing warned in advance — G-034 built a
"room for the next one" floor for the tuning drawer and not for this one — so
the first verify run after the classes landed failed with three radios off the
right edge of the screen. Re-laid at ten rows of thirty-six pixels (a radio is
thirty-two, so rows keep a gap), which ends the column at 464 above the footer
at 470: **twenty slots for nineteen classes**. The twenty-first class needs a
third column or narrower radios, and petitions (1.5) register a family of
classes. Expected: a floor that fails one class early. Happened: none; the
wave that adds the twenty-first class should add it (or this session's
successor should) before anything else.

### G-048 — the game's own: Steve's claim allows a share of three percent

Class: **the game's own** (a design fact the share exposed; the owner's to
price) · Game: ninjo · Files: `games/ninjo/src/constants.rs` (`share_pct`),
`src/economy.rs` · Open

The handoff fixed two things at once: a self-chosen job pays its worker a
share of the pot, and **Steve still goes short first in every idle world** —
"if the share rescues Steve, the constants are wrong, not the assertion: his
multiplier and purse are CAST content". Both hold only at a very small share,
and the arithmetic is short: Steve opens with **3g** against a first interval
of **7g** at minute 1440, and on day one he works two or three jobs back to
back (need opens his sum past the rest term, as it did before this wave). So
any share above about a gold a job rescues him.

The sweep, over sixty-four seeded worlds (`--probe` was a temporary tuning
mode this session used and removed):

| share | idle shortfalls | Steve first | attentive worst | margin |
|---|---|---|---|---|
| 0% | 14 | 64/64 | 3 | 11 |
| 3% (shipped) | 9..12 | **64/64** | 5 | 4 |
| 4% | 8..11 | 58/64 | 4 | 4 |
| 10% | 2..5 | 1/64 | 2 | 0 |
| 30% (first guess) | 1..3 | 1/64 | 2 | — |

Raising `upkeep_coin` alongside the share does not rescue the claim — every
cost scales and Bob, index 0, goes short at the same burn — and moving
`upkeep_hours` to six puts the first burn before anybody's first completion
and keeps Steve first at a 30% share, but presses somebody to the desperation
ceiling, which the limp floor forbids. **So the share ships at 3%**, every
assertion holds, and the share is honest but small: a gold or two a job. The
consequence the handoff asked to be said aloud — *the standing rate now
competes with the share* — is true in the arithmetic and weak in practice at
this number. What would make the share matter is one of: Steve's purse, the
upkeep cadence, or the claim scoped to day one. All three are owner
decisions; this session did not make any of them.

### G-049 — the game's own: the scorer does not price risk

Class: **the game's own** (an open design question) · Game: ninjo · Files:
`games/ninjo/src/autonomy.rs` (`pay_for`, `money`) · Open

The money term weighs what the job pays **if it is done** — the payout
function at `Tier::Done` — and no term weighs the odds. So a desperate
labourer weighs a fight job's share exactly as a fighter does, though it fails
him one time in three. That is what the handoff specified (the money term
reads the payout; the non-money terms were fenced), and it is what keeps
desperate people taking poor-fit work: 765 poor-fit jobs were taken across the
economy sweep's worlds, failing 35% of the time against a strong fit's 10%. Whether characters should *know* their
own odds — an expected-pay money term, or a risk term carried by a trait like
`craven` (`CAST.md` §3.3 parks it on "danger terms once fight tasks carry
danger") — is a scorer decision for the owner, and it would change who agrees
to a posting the board says is `risky`.

### G-032 — the game's own: the camp runs out of work before the band is whole

Class: **the game's own** (a design fact the staged start exposed) · Game:
ninjo · Files: `games/ninjo/src/autonomy.rs`, `src/settlement.rs` · Open, and
the owner's to price

The staged start seats the camp with four founders and lets the other six walk
in across days one to three (`CAST.md` §4). The settlement authors twenty-four
jobs across four sites and **the sites run dry** — DESIGN §10's open question,
answered the simpler way — so with the player idle the board is fully claimed
**by minute 2232**, which is the middle of day two. Hana, Ludo, Ines and Odd
arrive to a settlement with nothing left to do.

What this cost: wave 1.1's alive sweep asserted *everybody* takes paid work
with the player idle, which was true of a world where the whole cast stood in
the camp at minute zero. It is not true of this one, and the assertion had to
be re-scoped to "everybody who was in the camp while work stood open took
some", with the latecomers counted, named in the report, and asserted to be
**exactly** the people who arrived after the last row was claimed — so an
unexplained idler is still a failure rather than an excuse.

**The industry is the designed answer and an idle player never builds it.** In
the attentive sweep, where the scripted player builds the works on the first
day it can afford them, the settlement finishes forty-seven jobs against the
idle player's twenty-four and only one person is short at the end of day three
(`screens/ninjo-tenfold-reference.png` is that world). So the limp floor
exists exactly where GDD §5 says it does — *in the settlement*, not in the
camp — and what the wave leaves open is whether an idle settlement should be
able to reach it at all. That is a design question for the owner and not
something this session invented an answer to.

Expected: a board that outlasts the arrival column. Happened: it does not, and
the module that fixes it is behind a player decision. Owner: the owner's
(GDD §5, §10).

### G-033 — the game's own: nothing lowers desperation, so the escalation pipe has no other end

Class: **the game's own** (a design gap) · Game: ninjo · Files:
`games/ninjo/src/needs.rs`, `src/people.rs` · **Closed (wave 1.5)**

*Closed by wave 1.5, with the owner's answer (2026-10-02 mockup verdicts):*
**a met petition is the pipe's other end.** `petition-satisfied` lowers the
petitioner's desperation by `plea_relief` (a drawer step) and rewrites their
source line to the event — *"works off a debt that was his father's before it
was his - paid the collector off, for now (day 4)"* — and a declared
consequence raises it and rewrites it the other way (`broke` +2 and a rewrite,
`gives-away` +1, T5's `sours` +1). `petitioned::pipe` drives Tim from an empty
purse to `thin-days` both ways and asserts both transcripts; the photographed
run shows Bob's panel before and after (`ninjo-face-before/after-reference.png`).
What the closing does **not** do is reach an idle camp — an idle player meets
no petitions, so G-033's ratchet still runs there (G-050) — and it does not let
a wage relieve anybody (G-051). The entry below is the record of the gap.

GDD §5's needs row says "shortfall raises desperation (escalation pipe)" and
says nothing about what lowers it. Wave 1.3 built exactly that: a shortfall
presses by one step, held inside `people::DESPERATION_MAX`, and **no writer
anywhere lowers it**. Paying an interval in full does nothing; earning a wage
does nothing; the industry standing does nothing.

Over the three world-days the economy sweep runs, that is fine and the shipped
numbers are legible — the gradient over the cast reads exactly as `CAST.md`
§4's fiction does (Steve 8, Bob and Ludo 6, Rin and Goro 5, Alex and Tim
untouched at 2). Over a long game it is a ratchet: everybody who has ever been
broke converges on the ceiling and the scorer's opening term stops
distinguishing anybody. **The session did not invent a relief rule**, because
inventing one is a design decision and the handoff fences this wave at the
first rung of the pipe; wave 1.5's petitions are the next writer of
desperation and the natural place to decide it.

Expected: a pipe with two ends. Happened: one, by design, and the second is
unspecified. Owner: the owner (GDD §5's needs row, and wave 1.5).

### G-034 — the game's own: the tuning drawer is full

Class: **the game's own** (a layout ceiling) · Game: ninjo · Files:
`games/ninjo/src/layout.rs`, `src/tuning.rs`, `src/floors.rs` · **Closed (wave
1.4); reopened and closed again (wave 1.5)**

*Reopened by wave 1.5's five constants and closed by the stamp:* fifty stepper
rows push the fourth column to eight, and the pair-by-pair stamp under it was
twenty lines — 656 against a drawer that ends at 540. The floor said so the
first run, as it was built to. The stamp now names **what differs from the
shipped set** (`Tuning::moved`, `tuning::stamp_text`): `the shipped set` and
the seed when nothing is moved, `shipped, except` and up to six `name value`
lines otherwise, then a count of the rest. The shipped set is in the build, so
nothing is lost, and the whole set still rides every report and every link.
`floors::tuner_right_column` measures the tallest stamp (every constant moved)
at one more constant and one more row. The four constants' drawer names are
`plea_*` because `petition_regard` is fifteen glyphs and the name cell holds
fourteen.

*Closed by wave 1.4, which brought six constants:* the drawer has a **fourth
stepper column** (columns step 232, a row's own width, so four end at 956), the
stamp follows that column down under its last stepper
(`layout::tuner_stamp_for`), and the prose band moved up into the header beside
the presets (`layout::tuner_hint`). `floors::tuner_has_room` still asks about
the *next* constant — its stepper and the stamp it pushes down — and
`floors::tuner_right_column` asserts every state of the prose band fits its
three rows. The entry below is the record of the ceiling.

Three constants took the drawer from thirty-six to thirty-nine. Twelve stepper
rows at a pitch of thirty-four did not hold them, so the pitch dropped to
thirty-two — which is the target floor exactly, and the last row this geometry
has: `110 + 12 x 32 + 32` is 526 against a screen of 540. Three columns of
thirteen is **thirty-nine, and thirty-nine is what the game has**.

A fourth stepper column does not fit beside the stamp column at 960 reference
pixels: a row is 232 wide (a name, two buttons and the value between them) and
four of those plus a stamp column wide enough to read comes to more than the
screen. So the fortieth constant needs the **right column moved**, not
narrowed. `floors::tuner_has_room` is the floor that says so — it asserts the
next index's stepper is still inside the drawer, and it fails on the fortieth
rather than on a screenshot. The right column itself also reached its declared
headroom, and `tuning::APPLY_NOTE` gave up two rows of wording to keep it
inside the drawer.

Expected: room. Happened: none left, and the next wave is told rather than
finding out. Owner: this game's UI.

### G-035 — the game's own: a self-chosen job's money is felt by one personality, so the industry's wage lever moves nobody

Class: **the game's own** (a scorer/economy seam the needs wave exposed) ·
Game: ninjo · Files: `games/ninjo/src/autonomy.rs` (the pot term),
`src/sim.rs` (`work_done`) · **Closed (wave 1.4)**

*Closed by wave 1.4, both halves, through one function.* A self-chosen job now
pays its worker **a share of the pot** (`resolution::share_of`, the
`share_pct` drawer row; GDD §4.1's MINT line amended), and the scorer's money
term for any job reads **`resolution::payout`** — the function the completion
pays through — via `autonomy::pay_for`: a posting pulls by its wage, a
self-chosen job by its share, a shift by its wage, and never by a pot the
worker will not receive. The second half — two arithmetics — is one now:
`autonomy::money` feels any pay by pot affinity **and** desperation, and
`answers::terms` calls it for the wage. `economy::judge_the_wage` keeps its
unpressed ladder (still empty: with nobody pressed, money is affinity alone)
and gains a second over the camp at its authored desperation, which moves
people at 8g and 20g — a pinned literal, so the day it empties this entry
reopens. The share ships small for a reason of its own (G-048). The entry
below is the record of the seam.

GDD §4.1 is exact: a site's pot mints **into the treasury**, and what a worker
gets is a wage — a posting's (§3b) or an industry shift's. So a character who
decides for themselves to walk to the Deep Cave and haul mushrooms earns
**nothing at all**, and the forty gold goes to the player.

The scorer's pot term nonetheless weighs `quest.pot` at the carrier's own
`pot_affinity` — "the pot is 40g" is a *reason* a character gives for going
(`screens/ninjo-feed-reference.png` carries one). Before this wave nothing
turned on it. Now upkeep burns and the money matters, and the seam is visible:
somebody is drawn toward work by a number that is not theirs.

**And the seam has a second half, which is what this wave ran into.** There are
two money terms in this game and they are not the same arithmetic:

| where | what scales the money |
|---|---|
| `answers::terms` — a **posted** job | `pot_affinity` **+ desperation** |
| `autonomy::weigh` — a **self-chosen** job | `pot_affinity` alone |

The first reaches anybody who needs money; the second reaches only whoever
carries a pot affinity, and the neutral value is **zero** — so in this cast it
is `greedy`, and `greedy` is Bob alone.

An industry's slots are filled by self-choice (that is what "standing job
slots the scorer fills" means), so **the per-industry wage is felt by one
character in ten**, and never across a threshold: Bob's `indebted` want already
carries him over the idle floor before the wage says anything. The decision
table for this wave asks that "a wage step changes at least one worker's next
choice in the median seed", and at the shipped constants **it does not**.
`economy::judge_the_wage` is the instrument that says so — it asserts the lever
*reaches the decision function* (Bob's sum moves 8 → 12 across the panel's
range, shipped literals) and that the ladder moves **nobody's answer**, as an
empty shipped literal, so the day it starts to, the check fails and this entry
is what gets rewritten.

It is left alone on purpose — the handoff's fences are explicit that this wave
may not "change the posting rules, the scorer's terms, or the selection", and
both money terms are earlier waves'. The candidate fixes are all one-line and
all design decisions: give the neutral row a pot affinity of one; add
desperation to the self-chosen pot the way the posted wage has it; or give a
self-chosen job a default share so the money a character is drawn to is money
they get. The third also answers the first half of this entry.

Expected: the money a character is drawn to is money they get, and a wage the
player can move is a wage that moves somebody. Happened: neither, for a
self-chosen job — and the wave that made both matter is not the wave allowed to
fix either. Owner: the owner (GDD §4.1, §3b, and `autonomy::weigh`'s pot term).

**The legibility session (2026-09-09) read** `CLAUDE.md`, the `make-game`
skill, this game's `UI.md`, `GDD.md`, `FINDINGS.md` and its whole `src/`.
Nothing under `crates/*/src/`, no `docs/internal/`, and no ADR. It asked
`docs/api/` **one** thing it had not asked before — whether the input snapshot
carries a wheel event a scripted run can record, so a picture could be taken
one notch of the wheel out (it does: `InputEvent::Scrolled`, and the game's
conductor now has an `Act::Scroll` beside its clicks). The rest of the session
is arithmetic over the game's own data drawn into `Panel`s the floors already
judge. It closed **G-022** with its post-fix measurement and files **two new
entries**, both about *this game* and one of them about a handoff.

**The job-board session (2026-09-06) read** `CLAUDE.md`, the `make-game`
skill, this game's `UI.md`, `DESIGN.md`, `GDD.md`, `CAST.md`, `FINDINGS.md`
and its whole `src/`, plus one line of `docs/api/jidousha-testing.md` (the
`DrawnQuad` fields, checking whether a recorded quad carries the draw band it
was submitted on — it does not, which is why the answer below is a rule about
what the game draws rather than a smarter judge). Nothing under
`crates/*/src/`, no `docs/internal/`, no ADR. Its two entries are about this
game.

**Wave 1.1 read** `CLAUDE.md`, the `make-game` skill, this game's own
`GDD.md`, `DESIGN.md`, `UI.md`, `CAST.md` and `FINDINGS.md`, and its whole
`src/`. It opened no file under `crates/*/src/`, no `docs/internal/`, and no
ADR. It asked `docs/api/` nothing new — the scorer is arithmetic over the
game's own data and the two surfaces it added are `Panel`s like every other,
so its two entries below are about *this repository* rather than about the
engine's documents.

One entry from the S1 session. Nothing else was asked of the documents that
they did not answer: the Camera's pan/zoom, `visible_bounds`, the pointer's
scroll, `SnapshotBuilder`'s edge rules, `Time::alpha`'s per-tick value and
the capture path all worked as written.

### G-010 — the bounds check's stated form assumes a camera that does not move

Class: docs · Game: ninjo (as giri-rt) · Documents: `jidousha-testing.md` ("Assert that
nothing is drawn outside `Camera::visible_bounds()`") · Open

The testing document presents the bounds assertion — every quad
`contains_rect`-inside `visible_bounds()` — as "the highest-value check a
game of shapes and text can write", and for every game so far it was. A
game whose camera pans and zooms over a world larger than the screen cannot
pass it: a partially visible tile at the view's edge is *correct* rendering
and still fails `contains_rect`, and per-run text culling (a label is one
`ctx.text` call) means edge glyphs of a half-visible label land fully
outside. The check the situation actually wants is the inverse pair: nothing
submitted that does not *overlap* the view (culling is honest), and the
submitted count dropping when the view shrinks (culling is real).
ninjo ships that pair (`verify.rs::culling_probe`, UI.md §4); the
document could name the adaptation the first time a scrolling game reaches
it, because the naive reading is "skip the check", which drops real
coverage.

Expected: guidance on what the bounds check becomes for a camera that
roams. Happened: worked it out from the check's purpose; the workaround is
three assertions rather than one. Owner: `jidousha-testing.md`.


## Wave 1.2 (the asks module) — **2 new findings**, both the game's own

**Wave 1.2 read** `CLAUDE.md`, the `make-game` skill, this game's own
`GDD.md`, `DESIGN.md`, `UI.md`, `CAST.md` and `FINDINGS.md`, and its whole
`src/`. It opened no file under `crates/*/src/`, no `docs/internal/`, and no
ADR. **It asked `docs/api/` nothing new**, and that is a real answer rather
than an empty section: the module is arithmetic over the game's own data, its
three new surfaces are `Panel`s like every other, its two new inputs are
clicks on rectangles `layout.rs` already knew how to state, and its
occurrences ride the one scheduler the substrate landed in S1. The engine's
documents were not the cost of this wave; the two entries below are about
*this game*, and both are numbers the next playtest is owed.

### G-027 — the game's own, and FINDINGS-when-misled: two drawers were open at once, under a comment that said they could not be

Class: **the game's own** (a playtest defect) **and a doc that misled**
(a doc comment in this repository) · Game: ninjo · Files:
`games/ninjo/src/flow.rs`, `src/tuning.rs`, `src/screens.rs`, `src/floors.rs`,
`UI.md` §3 · **Closed by this session**

**What the owner reported (2026-09-11 playtest):** opening TUNE while ROSTER
was open drew both, one over the other.

Expected: the rule `UI.md` §3 states — never two at once. Happened: the tuning
drawer drew its thirty-six stepper rows straight over the roster's ten.

**What was there.** `Flow` carried five independent open-flags —
`feed_open`, `modes_open`, `roster_open`, `ledger_open` and `Tuner::open` —
and `screens::content` absorbed each under its own `if`, so any two of them
being true at once is a frame with two drawers in it. The TUNE handle was
handled in `tuning.rs` rather than in the handle loop in `flow.rs`, and it
cleared `feed_open` and `modes_open`: the two flags that existed when it was
written. `roster_open` and `ledger_open` arrived in wave 1.1 and wave 1.2, and
nothing pointed at the four lines that had to grow with them. The board and
the selection were left up under the tuning drawer too, by the same omission.

**The half that is about a document, and it is this repository's own.**
`Flow::close_everything` carries a comment that reads: *"one place, because 'a
drawer and a panel are never both up' is a claim the floors assert about pairs
of controls, and the way to keep it true is to have exactly one function that
opens anything."* That sentence is what made the defect invisible to a reader.
**What was done on its authority:** two sessions in a row (the roster, then the
ledger) added a drawer by adding a flag and a line to that function, read the
comment as the guarantee it claims to be, and never checked whether the tuning
drawer — the one drawer that does not go through the handle loop — was
actually calling it. It was not. The comment described an invariant the code
had stopped keeping, and it is more expensive than no comment because it is
what a reader checks *instead of* checking.

**The fix is the type, not the rule.** The five flags are one
`Option<Drawer>`; `Drawer` gains the tuning drawer as a variant; the render is
one `match` over that field and the click routing is the same `match` over the
same field, so "two drawers are open" is unrepresentable and the drawer that
is drawn is the drawer that answers a click by construction. Opening goes
through `Flow::open_drawer`, which calls `close_everything` first, so the
comment's claim is now true of every drawer rather than of four of them. The
handles, their labels and each drawer's own head row are walked off
`Drawer::ALL`.

**And the floor that would have caught it** (`UI.md` §4): at most one drawer's
content in a frame, counted by each drawer's own title row. It is
unrepresentable *and* asserted, because the next surface to grow an open-flag
should fail rather than overlap. `verify::one_drawer_at_a_time` walks all
twenty-five ordered pairs of drawers through the real handles and asks the
**frame** how many drawers it carries; `floors::floors_bite` stages a frame
carrying two drawers' content and asserts the floor reports it.

### G-028 — the game's own: a hand-placed offset met a band whose height is data, and no floor looked at text against text

Class: **the game's own** (a playtest defect, with a floor gap) · Game: ninjo
· Files: `games/ninjo/src/tuning.rs`, `src/layout.rs`, `src/floors.rs`,
`src/ui.rs`, `UI.md` §4 · **Closed by this session**

**What the owner reported (2026-09-11 playtest):** in the tuning drawer's
right column, the in-effect stamp and the explanatory prose were drawn over
one another — the screenshot reads `seed:0at a constant`.

Expected: a stamp and a note, one under the other. Happened: the stamp's last
row and the hint's first row occupied the same eighteen pixels.

**The arithmetic.** `Tuning::readout` is authored one line per pair of
constants and the stamp flowed down from y 124 at fourteen pixels a row; the
prose band's top was the constant `350` in `layout::tuner_hint`. At
thirty-four constants the stamp ended at 348 and the two cleared by two
pixels. Wave 1.2 added two more, the stamp reached 362, and the band did not
move — because it could not: nothing in it was derived from the thing above
it.

**Why no check saw it.** `floors::judge_panel` asserted chrome text against
*controls* ("nothing lies across a control it is not the label of") and map
labels against *each other*, and **never chrome text against chrome text**.
Every row was inside the UI rect, above the text floor, ASCII, and clear of
every control; the two rows were simply drawn through each other, which was
not a question anything asked.

**The fix is in two halves.** The layout is measured: `tuning::prose_top` is
one function of how many rows the stamp took, read by the drawer that lays the
column out and by the floor that asserts it fits, so the next constant *moves*
the band instead of colliding with it. And the hint and the note now share the
band — only one of them is ever what the player is asking for — which is what
makes the column fit at its tallest state with two rows of headroom, asserted
by `floors::tuner_right_column` at `tuning::STAMP_HEADROOM`. **The floor fails
while there is still room**, so the wave that adds the constant is told to
re-lay the column rather than finding out from a screenshot.

The other half is the general floor: **no two rows of chrome on one band
overlap**, over every state `content_floors` judges. *On one band*, because a
run on a higher layer has its own ground behind it and the breakdown band is
deliberately drawn over the feed drawer's footer (`UI.md` §3e); two rows on
the same band are two rows drawn through each other. `floors::floors_bite`
stages the pre-fix column and asserts the floor reports it, so the floor is
known to bite.

**One rider, found on the way.** `ui::wrap` ate the caller's own line breaks —
it split on whitespace — so wrapping the stamp would have joined thirty-six
numbers into one paragraph. It now wraps each of the caller's lines on its own
and keeps the breaks, which is the only contract under which "no line is wider
than the column" is true of every string rather than of every string without a
newline in it.

### G-029 — the game's own: the character panel had the same defect as the tuning drawer, and the new floor found it

Class: **the game's own** (found by a floor written for something else) ·
Game: ninjo · Files: `games/ninjo/src/panels.rs`, `src/layout.rs`,
`src/floors.rs` · **Closed by this session**

The character panel's lower rows — the source line, the activity line, the
home row and a tapped chip's explanation — sat at four typed offsets: 156,
198, 224, 242. Each of the first two wraps. At the shipped cast the activity
line wraps to two rows, which end at 226 — through the home row at 224.

Nobody reported it and no check saw it, for exactly G-028's reason: it is text
across text, and until this session that was not a question this game asked.
The floor written for the tuning drawer found it on the first run, on a
photographed state, which is the whole argument for writing floors as
questions about the class of defect rather than about the instance.

**The fix is the same fix**: the panel's lower half flows — each block starts
where the one above it ended — and the flow is budgeted at
`layout::sheet::LEAD_ROWS` with `floors::layout_floors` asserting the budget
and `floors::content_floors` rebuilding the real sheet over every judged state
and requiring every row of it to land inside the panel.

### G-030 — the game's own: the character panel's close button has had no X on it since it was built

Class: **the game's own** (found by looking at the pictures) · Game: ninjo ·
Files: `games/ninjo/src/screens.rs` · **Closed by this session; the floor gap
is open**

`screens::ghost` puts a control's ground on `layers::OVERLAY + 1`, which is
above `layers::TEXT`. Every other base-screen control is drawn with
`ghost_at(.., layers::CARD)` — the function exists for exactly this reason and
its own comment says so — but `layout::person_close()` used the plain `ghost`,
so the panel's `X` was submitted, recorded on the frame, and then painted out
by its own button. It is visible in `screens/ninjo-person-reference.png` in
every wave that has one.

**Why nothing caught it.** `frames::judge_chrome` asks whether every row the
screen said it would draw is *on the frame*; this row is. `judge_figures` asks
whether the frame carries anything the screen did not say; it does not. **No
check in this game asks whether a quad is on top of a row**, because the
`Panel` carries text and icons with their layers and the grounds are drawn
straight through `ctx` with no record of what they cover. Fixed by using the
band the function was written for; **a floor for "a control's ground does not
cover its own label" is not written**, and it needs the panel to carry its
rectangles the way it carries its rows. That is a sanitation-pass item, not
this session's.

### G-031 — the class: four defects in four waves, all of them two representations of one fact

Class: **the class** (a pattern across this game's own findings) · Game: ninjo
· Files: the four entries it names · **Open — the audit ran (2026-10-02, the
wave-1 sanitation pass) and found the fifth instance; closes with G-060's
collapse**

Four defects now, in four consecutive waves, with one shape:

| the defect | the two representations | what kept them in step | who found it |
|---|---|---|---|
| **the double selection** (G-017) | S1's dispatch pick and wave 0a's character selection, two indices over one roster | a convention that both be written together | the wave-1.1 playtest |
| **the double figure** (G-023) | wave 0b's `Panel` figures and wave 1.1's party tokens, two pictures of one person | a convention that the token be drawn only when away | the wave-1.1 playtest |
| **the board against the retired strip** (G-026) | the strip's disposition list and the surfaces that could actually reach a selection | a convention that the list be checked for coverage | the 2026-09-09 playtest |
| **the five drawers** (G-027) | five open-flags and the five `if`s that draw them | a comment claiming one function opens everything | the 2026-09-11 playtest |

**The shape is always the same.** Two representations of one fact, kept in
step by convention rather than by construction; the convention is written down
*somewhere*, usually in a doc comment near one of the two; a later wave adds
to one representation and not the other; and no check sees it, because every
check in this game is written against one of the two representations and
therefore agrees with itself.

**And the remedy has been the same three times running**: collapse the two
into one value and let a `match` over it do what a rule was doing. One
selection (`Flow::selected`), one figure (`screens::where_drawn`), one drawer
(`Flow::drawer`). Each collapse also made the *check* possible, because there
was finally one thing to ask about.

**What it is owed.** The wave-1 close's exemplar audit (`GDD.md` §8) should
walk this game for the remaining pairs rather than waiting for the fifth
playtest to find one. A start, from this session: `Flow::board` and
`Flow::picking` (kept in step by `put_the_picker_away`), `Flow::board` and
`Flow::listing` (by `put_the_list_away`), `Flow::selected` and
`Flow::breakdown` (by `put_the_band_away`), and `Flow::selected` and
`Flow::listing` (the same). All four are *checked every tick* rather than
merely conventional, which is the weaker version of the remedy and the reason
they have not bitten — but they are still two values where the surfaces they
describe are one column.

*The audit's answer (2026-10-02):* the column is the fifth instance — five
fields, four hand-written clearings, and three bites in the gap between them
(G-060), plus the band carrying a slot where the picker carries a job (G-061)
and the consequence chip's flag beside its card (G-063). The remedy proposed
is the same one, `Option<Column>`, and it is the owner's call.

### G-021 — the game's own: the standing rate is a coarse lever at the rate it ships at

Class: **the game's own** (a tuning fact, found by the check that exists to
find it) · Game: ninjo · Files: `games/ninjo/src/compliance.rs`,
`src/asks.rs`, `GDD.md` §9 · **Open — a question for the playtest**

The amendment's policy test asks that "raising fight pay by one drawer step
shifts at least one fighter's next choice". Expected: one tap of the
standing-rates panel moves somebody. Happened: at the shipped fight rate of
24g, **three** taps of the panel's 4g step are needed before anybody's *work*
changes, and the first person to move is Hana at 36g.

The arithmetic is not a bug and the check reports it rather than hiding it. A
wage's pull is `(pot_affinity + desperation) x wage / 10`, so 4g is worth one
or two points to most of the band, while the aptitude term that holds somebody
in their own trade is six. Walking the rate across the panel's whole range
shows the lever working exactly as designed — somebody's work changes at 12g,
16g, 36g and 56g, and five of the ten drift into fight work as it rises — so
what was wrong was the *test's* framing, not the mechanism.

**What was done:** the battery asserts the whole walk (the four turning
points, as shipped literals) **and** pins the distance from the shipped rate
to the first drift at three steps, so a moved weight moves a literal and the
mutation round notices. Nothing was tuned to make a one-step test pass: moving
the opening fight rate to sit just under a wall would have been tuning to the
instrument.

**What the owner is asked:** is a 4g step the right size for a policy control,
and is 24g the right price for fight work when 36g is what it takes to pull a
scout off her own trade? The answer is a content decision (`asks::RATES` and
`asks::RATE_STEP`), not a code one.

### G-022 — the game's own: the crowding at the settlement is the labels, not the zoom

Class: **the game's own** (a rider, answered by measuring it) · Game: ninjo ·
Files: `games/ninjo/src/floors.rs`, `src/camera.rs`, `UI.md` §4 ·
**Open — the next UI session's**

The wave's rider asked for the default camera to step out one level "so the
ten figures are not on top of each other", with the readability floors
re-asserted at that zoom and the note that if names stop being legible, "the
zoom is the thing that yields".

Expected: a wider view separates the figures. Happened: **zooming out cannot
separate them, because the overlap is scale-invariant** — the map is drawn in
world units and pulling the camera back shrinks the figures and the gaps
together. What it *does* change is legibility: a name is drawn at twelve world
units, which is exactly the twelve-reference-pixel floor at the default
camera, and one notch of the wheel out puts it at 10.7. So the floor refuses
the step, which is what the rider said should happen, and the default stays at
540.

The crowding the owner saw is real and has a different cause: the homes are
two rows of tents two tiles apart (`CAST.md` §4), a figure is 32 world units
tall (two tiles), and a name is drawn *under* its figure — so the northern
row's names land on the southern row's heads. The fix is a layout one — the
label above the figure for one row, or a name only for the selected character,
or three tiles between the rows — and each of those is a UI or content
decision this session did not have a mandate for.

**Reopened 2026-09-08, on its subject rather than its measurements.** This
finding measured the settlement's crowding and concluded the cause was label
placement against row spacing. The measurements are not wrong and none of them
changes. **Its subject was**: the map it measured was drawing twenty figures
for ten people (G-023), so "the ten figures are on top of each other" was a
report about a duplicate and not about the rows. The crowding question is to be
**re-judged by the owner** at ten figures before anybody moves a row or a
label, and the layout suggestions below — the label above the figure for one
row, a name only for the selected character, three tiles between the rows —
are not to be acted on until they are. The fix session deliberately did not
touch them.

**What was done (2026-09-08):** `floors::map_legibility` states the numbers as
a floor and the verify report prints them (`map labels 12.0px at the default
zoom, 10.7px one notch out`), so the next session inherits a measured fact
instead of a disagreement.

**Closed 2026-09-09 by the legibility session, and here is what it did and
what it then measured.** The reopening was right and the fix is neither of the
layout suggestions: **the floor was being used as a veto and it should have
been a governor.** A name was going to be drawn whatever the camera did, so the
twelve-pixel floor could only forbid the zoom; now the *label* yields — a
label is drawn only where it clears the floor and lands on nothing already
drawn (a figure, a marker, an earlier label, or a chrome surface that is up),
in one fixed order, and below the floor none is drawn at all. So:

- **the crowding is gone at the default camera without moving a row or a
  home.** Of the nineteen words the settlement can say, **twelve survive**;
  the seven that drop are the ones that were landing on the southern row's
  heads, which is exactly what this finding measured. `screens/ninjo-settlement-reference.png`
  is the after picture, and the seven that dropped are the *northern* row's
  names, so two of the ten figures now stand anonymous at the default zoom.
  **That is the residual, and it is the owner's to judge**: it is a real cost,
  it is the price of never drawing a name across a face, and a name only for
  the selected character (this finding's own second suggestion) is what the
  chrome label now gives on top of it.
- **zooming out is permitted**, which is what the wave-1.2 rider wanted and
  could not have: one notch out drops every map word and keeps every figure
  (`screens/ninjo-zoomed-reference.png`), and the selected character stays
  named because that name is chrome. **The default camera was deliberately not
  moved** — where it should sit is a play judgement, and the point of the rule
  is that it is now a judgement somebody can make by making it.
- `floors::map_legibility` keeps stating the two numbers, and
  `verify::map_labels_are_governed` asserts the rule itself: every drawn label
  clears the floor and touches nothing, none is drawn below it, the selected
  name survives both, and the same frame drops the same labels twice running.

**What the owner is asked for**, and the reason this closes rather than
reopens again: play it, zoom out one notch, and say whether the default should
move and whether two unnamed figures at the settlement is a price worth
paying. Both are content judgements now rather than measurements.

## The candidate-picker session (2026-09-10) — **1 new finding**, the game's own

The playtest finding the owner reported, and its process half — which is the
half that matters, because the same disposition list that missed this is the
one the wave-1 close's audit will be reading.

### G-026 — the game's own: the one surface whose purpose needs the map is the one that covers it

Class: **the game's own** (a playtest finding, with a process half) ·
Game: ninjo · Files: `games/ninjo/UI.md` §3, §3b, §3c, `src/board.rs`,
`src/flow.rs` · **Closed by this session; the process half is open**

**What the owner reported (2026-09-09, after the legibility session landed):**
with the party strip retired, an open site panel can cover character sprites,
and those characters then cannot be selected — so the posting gesture needs a
person you may be unable to reach. Selecting first works only if you already
know what work stands at the site, which is what the board exists to tell you.

Expected: that a board could be opened and posted from wherever it landed.
Happened: at the reference camera the board's own rectangle covers world
`x -80..496, y 58..386`, which is most of the settlement's doorstep row — and
the board swallows clicks inside its whole rectangle on purpose (UI.md §3c,
so a stray tap between two rows cannot open a different site instead of
ordering). Every figure under it is therefore unselectable while it is up.

**Why the framing matters more than the fix.** Every drawer in this game
covers the map and that is accepted. What made this one a defect is not the
occlusion but the **requirement**: the board is the only surface whose purpose
needs a map interaction, and it hides the map. So the remedy is not to dock,
inset, shrink or auto-pan around the panel, and not to bring the strip back in
any form — it is to make the board able to name a person. §3c's candidate
picker is that: a `who?` on each open job row, the cast for that job with fit,
verdict and whereabouts, and choosing writes the one selection.

**The process half, and it is the sharper one.** When the party strip retired,
its disposition list (UI.md §3) said selection had moved to "the map's figures,
the roster's rows and a faces list" — three doors, all true. Two of the three
are drawers, and a drawer shuts the board (`Flow::close_everything`); the
third is the map, and the map is exactly what the board covers. **So the
board, which needs a selection to do its own job, was left with no door onto
one that survives it being open.**

**Listing where a job moved is not the same as checking the new home is
reachable in every mode the old one was.** That is a different question from
the one G-024 asks — G-024 is "what is this surface still for?", and this is
"is the thing that replaced it reachable from everywhere the old one was?" A
retirement passes the first and fails the second exactly when the replacement
homes are surfaces that are mutually exclusive with the surface doing the
asking, which is a property nothing in this repository looks at. It belongs
beside the S1-residue entry: **the wave-1 close's exemplar audit** (`GDD.md`
§8) should walk both questions per retired surface, and a retirement's
disposition list should say, per job, *which surfaces the new home is
reachable from* and not only where it went.

**What this session did about it:** built the picker, made the board's own
`who?` and the picker's rows floors-bound controls, added the reachability
claim as a scripted check (`verify::the_picker_names_a_person`'s sixth part
opens a board over a character's own sprite and posts to that character with
no map click, asserting the transcript is byte-identical to the map-selected
route), and wrote the reach question into UI.md §3 beside the disposition list
it corrects.

**Two smaller things, recorded because they cost minutes rather than
decisions:**

- **The board's footer wage and a row's actual offer can be two numbers.**
  `board::board_wage` shows `flow.offer` or *the first open row's* standing
  rate, while `board::wage_offered` charges the *tapped row's* task rate — so
  on a site whose open rows are two task types the footer says one wage and a
  tap makes another. It predates this session (wave 1.2) and this session did
  not change it; the picker sidesteps it by printing the wage **its own job**
  would be offered at in its header, which is the number its verdicts were
  read at. Worth the owner's ruling: either the footer's stepper is per-task,
  or the board's wage is one number and the rate a row inherits is that one.
- **A sub-panel of a surface has to key off the surface's *identity*, not its
  presence.** The picker's state began as the job's slot, on the breakdown
  band's precedent (`Flow::breakdown` holds `Breakdown::Job(slot)` and is put
  away when the board is `None`). That is not enough for either of them: a
  site marker outside the open panel's rectangle is still clickable and a
  marker *replaces* a board rather than closing it, so a slot-only sub-panel
  survives into a board it was not opened on. The picker carries the whole
  `JobId` and its rule compares the pair; the band was left as it is, because
  changing it is a change to a surface this session was not sent to touch —
  **but the same reading applies to it**, and the way in is a marker that
  falls outside `board_panel()` (the Watchtower's and the Black Vault's do at
  the reference camera). Worth the next board-touching session's ten minutes.
- **`board_why` was a click target outside `floors::board_targets`** since the
  legibility session, so neither the 32x32 floor nor the overlap floor was
  asked about the `?` at the end of a job row. It passes both; this session
  added it to the set alongside the new `who?`, and the gap is worth noting
  because a control that is hit-tested in `flow.rs` and absent from
  `floors.rs` is a control no floor can see — the same shape as G-023's
  undeclared draw exemption, one file over.

## The legibility session (2026-09-09) — **2 new findings**, both the game's own

It closed G-022 above with the measurement that finding asked for. Its two new
entries are a pattern the owner has now paid for three times, and a handoff
that forbade in one section the thing it required in another.

### G-024 — the game's own: three surfaces have now outlived the world they were built for, and nobody was looking for the fourth

Class: **the game's own** (a pattern, not an incident) · Game: ninjo ·
Files: `games/ninjo/UI.md`, `src/flow.rs`, `src/screens.rs`, `GDD.md` §8 ·
**Open — the wave-1 close's**

Three sessions running have retired a piece of S1 presentation that was
correct when it was written and wrong by the time it was found, and each was
found by an owner playtest rather than by a check:

| the residue | what it was for | what made it wrong | found by |
|---|---|---|---|
| the **double selection** (G-017) | S1's dispatch pick, beside wave 0a's character selection | wave 1.1 made a party a character, so one roster had two indices over it | the wave-1.1 playtest |
| the **double figure** (G-023) | three party tokens stacked on a town tile | wave 0b gave every person their own figure, so the token was a second picture of the same person | the wave-1.1 playtest |
| the **party strip** (this session) | the only way to see and select a one-person band | wave 0a's meters, wave 1.1's roster and the map's own figures took its four jobs one at a time | the wave-1.2 playtest |

**The shape is the same every time**: a surface built for a world where it was
the *only* answer to a question, kept through the waves that gave that question
better answers, and retired only when somebody played the build and said it
felt cluttered. Nothing in this repository asks "what is this surface still
for?" — the floors ask whether a surface is legible, the content floors ask
whether it says what it means, and both pass with flying colours on a surface
nobody needs. **A check cannot find this**; a reading can.

**Where the rest should be swept for**: the wave-1 close's exemplar audit
(`GDD.md` §8). The audit walks the surfaces against what the built world
actually needs, and this is the class it should be walking for — the
candidates a reader should start from being every surface whose justification
in `UI.md` names a wave earlier than the one that last touched the question it
answers. Two are visible from here without judging them: the **notices band**
(the last two things the player did, in a drawer whose feed now carries a `?`
on every decision) and the **meters band's two chips** (`idle` and `away`,
which the roster's ten rows now state per person). Neither is wrong today;
both are surfaces whose question moved.

**What this session did about it:** retired the strip, listed its four jobs and
where each now lives (`UI.md` §3), and filed this. It did not touch the two
candidates above — naming them is the finding, judging them is the audit's.

**And the disposition list it wrote was itself the next finding** (G-026, the
day after): three of the strip's four jobs moved to surfaces that are mutually
exclusive with the one still asking for them, and the list recorded *where*
each job went without asking *from where the new home is reachable*. So the
audit this entry dispatches owes two questions per retired surface, not one.

### G-025 — the game's own: a handoff forbade in its fences what it required in its verification

Class: **the game's own** (a handoff defect, and the session deviated) ·
Game: ninjo · Files: the legibility session's handoff, `games/ninjo/src/sim.rs`
· **Open — the owner's to rule on**

The legibility handoff's fences say **"no sim state"** twice — in its opening
line and again in its "You may NOT" list. Its **Verify** section requires that
"the feed breakdown equals the `Judged` **recorded with that decision**", and
its work item 1 requires the same breakdown for "a choice already made" so
that "why did Ludo go there" is answerable *after the fact*.

Expected: to build both halves inside the fence. Happened: **they cannot both
be met.** The arithmetic behind a decision cannot be recovered later without
re-running the scorer against a world that has moved — the board has been
claimed, the wage stepped, the regard drifted — and a breakdown that disagrees
with the decision it explains is the failure mode the same handoff names first.
The job row's half needs no record at all (it is a preview of *now*); the
feed's half needs the terms kept at the moment they were weighed.

**What was done, and on whose authority:** the session kept them, on the
`Event::gold` precedent — a derived fact recorded on the occurrence because
recomputing it later would lie, which `sim.rs` already states in those words
for money. `Event::judged` is a record and not an input: nothing in the
simulation reads it, no arithmetic depends on it, no transcript prints it, and
the invariance sweep and every replay are byte-identical with it in place. It
is nonetheless **a change to a sim struct in a session told not to make one**,
and it is called out here and in the PR rather than shipped quietly.

**What the owner is asked to rule:** whether "no sim state" in a UI handoff
means "change no state the world decides from" (which this obeys) or "add no
field to a sim struct" (which this breaks). The second reading makes the
feed's half of a breakdown unbuildable by any session that is also forbidden
to compute a second answer, so the wording is worth settling before the next
UI handoff inherits it.

**Two smaller things about the same handoff**, recorded because they cost
minutes rather than decisions: it names `reason_for` as the function that
collapses a `Judged` to its loudest term — the function is `autonomy::words`,
and there is no `reason_for` in the crate; and it asks for the party strip's
jobs to be listed "in the PR", which is where they are, but the durable place
for them turned out to be `UI.md` §3, since a PR body is not somewhere anybody
reads twice (`make-game` §C says exactly that about findings).

## The job-board session (2026-09-06) — **2 new findings**, both the game's own

The documents were asked nothing they did not answer. `jidousha-testing.md`
was opened once, for `DrawnQuad`'s fields, and it was accurate; the panel /
floors / frames machinery this game already had absorbed a whole new surface
without a new engine question, which is the thing that made the session cheap.

### G-019 — the game's own: a site marker showed a count and took an order, and the work was invisible

Class: **the game's own** (a wave-1.1 gap the owner played into) · Game: ninjo ·
Files: `games/ninjo/src/flow.rs`, `src/board.rs`, `src/sim.rs`, `UI.md`,
`DESIGN.md` · **Fixed here**

Reported by the owner from the wave-1.1 playtest of the deployed build: a site
marker shows only a quest count, so there is no way to see the work or to
choose it.

Expected, from `CAST.md` §2 and the wave-1.1 board: the player picks a person
for a *kind of work*, because the whole trait vocabulary is built to make
"whom do I send, and why" a decision. Happened: dispatch was "this person to
this site", the site handed out its first open row, and the marker's `6
quests` was the only thing on screen about what stood there. Wave 1.1 had
grown the board from seven jobs to twenty-four and put a task type on every
one of them — which made the gap much larger than it had been, because a
count of six now stood in for six *different* pieces of work.

**What made it invisible to the checks:** every assertion about dispatch was
about the site. `expected_events` pinned arrival minutes, `judge_one_path`
compared movement classes, and the scorer's own battery weighed
`SeekWork { site }` — so a build in which the UI named one job and the sim
claimed another would have passed all of them. The fix carries its own
instrument: `autonomy::judge_named_claims` walks every spent row on every
board and requires that the departure which claimed it named it, which is a
check that could not have been written while a claim was a count.

**What the next session inherits:** the board and the faces list share the
left of the screen and are never open together, so the base screen is now two
control sets (`floors::targets` and `floors::board_targets`) and
`controls_for` picks between them. A third over-the-map surface has to pick a
side or take a third set.

### G-020 — the game's own: a recorded quad does not say which draw band it came from

Class: **the game's own**, though it starts at an engine boundary · Game: ninjo ·
Files: `games/ninjo/src/screens.rs`, `src/frames.rs` · **Worked around here,
and the workaround is a rule**

Found while adding the job board. `frames::judge_chrome` finds a row of chrome
on the recorded frame by counting glyph quads inside the row's own box; the
board's footer sits at the same world *y* as the Old Crypt's `4 quests` label
and overlaps it in *x*, so a glyph of the map label landed inside the footer's
box and the count came out one too high. The row was drawn correctly and the
panel covers the label on screen — the judge was counting a glyph the reader
cannot see.

Expected: a way to ask the frame for the chrome's own glyphs. Happened:
`DrawnQuad` carries `batch`, `texture`, `corners` and `tint`, and no depth or
layer (`jidousha-testing.md` is accurate about this; nothing misled). The
document is not wrong — it is a gap, and a small one, because the honest fix
turned out to be a rule about the game rather than a smarter judge.

**What we did on that basis:** extended UI.md §3's standing law — *under an
open drawer the map says nothing* — to the job board, which is a panel over
the map in exactly the same sense. While a board is open the map draws no
words at all; the board carries the site's name and its open count in full,
and the labels it hides are the ones it is standing on. That is a better rule
than the one it replaced anyway. It is filed here so the next surface over the
map meets the fact rather than the symptom, and so that a `layer` on
`DrawnQuad` — if the engine ever wants one — has a use case on record.

## Wave 0b (the people substrate) — **0 new findings**

Said explicitly, because `0 findings` is a real answer and an unsaid one
reads as a skipped step.

Wave 0b reached for **no engine API that S1 had not already established**.
The port was game logic — a registry, a vocabulary, three stores, an
arithmetic — and everything it touched of the engine's surface (`Resource`,
`headless`, `SnapshotBuilder`, `FrameRecorder`, the capture path,
`TextStyle::width_of`) was already load-bearing in this crate and answered
by the four documents when S1 asked. Nothing new was asked of them, so
nothing new can be reported about them; a finding invented to fill this
section would be worse than an empty one.

**G-010 stays open.** The bounds check's stated form still assumes a camera
that does not move, and this wave added map-space content — the cast's
figures and names — which is culled the same way the terrain is and would
fail the naive `contains_rect` reading for the same reason.

One thing worth recording that is *not* a documents finding, because it is
this game's decision and not the engine's: **`Sim::at_rest` had to stop
meaning "the queue is empty"** the moment an ambient occurrence started
rescheduling itself forever. The substrate's stopping condition was written
when every occurrence belonged to a party. Any wave that adds a recurring
ambient occurrence — needs ticking is the next one — meets the same fact, so
it is written down here as well as at the site.

## Wave 0a (the attention architecture) — **1 new finding**

Reading discipline held: `docs/api/` (all four), `crates/jidousha/examples/`,
`games/giri/` and this crate. Nothing under `crates/*/src/`, `docs/internal/`
or any ADR was opened.

### G-011 — whether the first-finger-to-pointer mirror applies to a scripted snapshot is not stated

Class: docs · Game: ninjo · Documents: `jidousha-api.md` ("a game written for
a mouse is already playable by touch"), `jidousha-testing.md`
(`InputEvent::Touched`, `SnapshotBuilder`) · Open

The API document states the mirror as a property of the engine — "the engine
puts the first finger down onto the primary pointer", so
`just_pressed(PointerButton::Primary)` is a tap — and the testing document
lists `InputEvent::Touched { finger, phase, screen }` among the events a
`SnapshotBuilder` records. Neither says whether the mirror is applied when a
*check* records a `Touched` event, or only by the platform layer on the way
in. That is the difference between a check that can verify the claim and a
check that cannot: if the mirror lived in the platform crate, a failing
assertion would mean "the harness does not mirror" rather than "the game's
hit-test is wrong", and there is no way to tell those apart from the
documents.

Expected: one sentence in the testing document saying that a recorded
`Touched` produces the mirrored pointer in the snapshot the game reads.
Happened: wrote the check and ran it to find out. It does mirror, and
`verify::touch_selects` now asserts a finger on a character's figure selects
them with no `PointerMoved` and no `ButtonPressed` in the snapshot — so the
answer is recorded here, and the document is the place it belongs.
Owner: `jidousha-testing.md`.

**G-010 stays open** for the third wave running: the bounds check's stated
form still assumes a camera that does not move, and wave 0a added more
map-space content (the selection ring, the focus pulse) culled the same way.

## The cast-art session (2026-09-02) — **3 new findings**

Reading discipline: `CLAUDE.md`, `.claude/skills/make-game/SKILL.md`,
`docs/internal/assets.md` (named by the handoff — the one document outside the
fence this session was told to read), and this crate whole. Nothing under
`crates/*/src/`, no ADR, and `docs/api/` was not opened, because the session
asked the engine nothing: it added fifteen rows to a table of file names.

All three are the *game's* tooling rather than the engine's, and all three are
about the same fact — `art/` was written for an owner sitting at the keyboard
with the packs, and this session was an agent standing in for one.

### G-012 — the import tool drops a staged role it has no grid for, silently

Class: tooling · Game: ninjo · Files: `games/ninjo/art/import_pack.py`
(`roles`, `plan`), `games/ninjo/art/sprite_defs.py` · **Fixed in this session
for ninjo; still open in `games/giri/art/`**

`import_pack.py` is the one door art comes in through, and it takes "the roles"
to be the names in `sprite_defs.LIBRARY` — the *generated-art* table. `plan()`
walks that list and looks for a file per role, so a role-named PNG staged for a
role the table does not carry is never looked at. Reproduced deliberately after
the fact, with `icon_maker` removed from `LIBRARY` and the fifteen staged:

```
[giri-art] 26 of 27 role(s) filled from target/ninjo-art/staged
exit 0
```

No mention of `icon_maker` anywhere in the output, and a zero exit. The count
reads exactly like a legitimate partial library, which is a documented and
normal state ("Roles the pack does not fill keep their current file"), so
nothing about the run says a file was ignored. This is the no-silent-failure
rule (CLAUDE.md) failing in the tool that enforces the curation model.

Expected: a role list that is the *game's* roles, and a refusal — or at least a
line — for a staged file that matches none of them. Happened: the session read
`plan()` before running it and added all fifteen roles to `sprite_defs.LIBRARY`
first, so the import worked; had it not, fifteen files would have stayed out of
`assets/` and the only symptom would have been `tools/check-assets` failing
later with fifteen missing files and no hint why.

The fix taken here is the honest half of it: every role now has a grid, which
also keeps `make_art.py --restore` meaningful — the way back if a pack is ever
withdrawn is not a way back for a role with no grid. The other half is the
tool's, and is not this session's to change in giri: a staged file matching no
role should be named and refused.

### G-013 — nothing in `art/` shows a candidate at the size it will be drawn

Class: tooling · Game: ninjo · Files: `games/ninjo/art/role_sheet.py`,
`games/ninjo/art/contact_sheet.py` · **Open**

`role_sheet.py` renders a role's shortlist at `--scale` (default 10) and
`contact_sheet.py` a whole pack at `--scale` (default 4). Both upscale the
*art*, so every sheet shows a candidate four to ten times larger than the game
draws it. For an owner at a screen that is fine — a person holds the slot's
real size in their head. An agent does not, and the sheets are the only thing
it sees.

What was done on the sheets' authority: the four aptitude icons were picked as
one family of steel implements — sword (`microrl:70`), pick (`71`), bow (`72`),
hammer (`74`) — off role sheets at scale 12, where all four are unambiguous and
the family cue is obvious. Then a throwaway script composed the same four at
scale 2 (the 16-unit chip, `attention::CHIP`) and magnified the *composed
sheet* rather than the art, which is the only way to see honest 16-pixel
pixels. At that size the pick and the hammer are the same picture — a dim
diagonal with a pale head — and the bow is a smear. Two of the four picks
changed (`71` → `91` ladder, `72` → `39` lantern), and the family cue changed
with them, from "steel implement" to "line-work against filled mass".

Expected: a mode that says "show me this shortlist at the size the slot is
drawn". Happened: wrote one by hand, twice, and would have shipped two
illegible chips without it. `art/picks_sheet.py` (added here) does it for the
landed set — every picture at its drawn size and again at 4x — but it reads
`assets/`, so it can only judge art that has already been imported. The
shortlist half is still missing: `role_sheet.py` wants a `--at-size N` that
composes at the slot's drawn scale and magnifies the sheet.

This is the entry the handoff asked for by name: what an agent-picker needed
that the manifest tooling, written for an owner at the keyboard, does not have.

### G-014 — the fork's art tooling still calls itself giri

Class: docs · Game: ninjo · Files: `games/ninjo/art/*.py` · **Partly fixed**

`art/` rode along whole in the fork (VARIANT.md), and every script still opens
`"""games/giri/art/<name>.py"""`, prints `[giri-art]`, and — until this session
— defaulted its output to `target/giri-art/` and told the reader to run
`cargo check -p giri`. The last of those is the expensive kind: `giri` is a real
crate that really builds, so following the instruction succeeds and checks the
wrong game. `import_pack.py` also wrote `(UI.md §9)` into `CREDITS.md`, which is
giri's asset-slot section; ninjo's is §7, and giri's §9 says something else.

Fixed here: the output paths, the `cargo check -p giri` line, and the `§9` that
went into a committed file. Left alone: the docstring headers and the
`[giri-art]` console prefix, which are wrong but cannot mislead an action — and
a rename of five files was not this session's to make.

Expected: a forked tool says which game it belongs to. Happened: the session
followed `extract.py`'s printed next-step verbatim and staged into
`target/giri-art/`, which is where a giri session would look for it.

### G-015 — the sweep's `games/giri/` paths in the engine's own comments

Class: docs · Game: ninjo · Files: `crates/*/src/`, `docs/api/` · Open

Wave 1.1 retired giri to `attic/giri/` (the handoff's last item). Every path in
`docs/internal/` that pointed at one of its documents was repointed in this
commit; the ones inside `crates/*/src/` doc comments were not, because engine
source is outside a game session's fence — and `docs/api/` carries them
because it is generated from those comments. The dangling references are:
`crates/jidousha-render-core/src/font/mod.rs` and `style.rs` (G-003),
`crates/jidousha-platform/src/driver/mod.rs`, `src/web/render_scale.rs` and
`src/lib.rs` (`games/giri/UI.md §6`, and `"games/giri/assets"` as the worked
example of a game's own asset root, which reaches `docs/api/jidousha-api.md`
and `tools/api-doc/concepts.md`).

Expected: a move that is mechanical in the game's own tree to be mechanical
everywhere. Happened: four engine files and one generated document now name a
directory that does not exist. None of them can mislead an *action* — they are
citations and an illustrative path — but the asset-root example is the sharper
half: it teaches `asset_source("games/giri/assets")` for "a game at
`games/giri/`", and the game it names is in the attic. A sanitation pass or an
engine session can repoint them at `games/ninjo/` and at `attic/giri/`; a game
session may not.

### G-016 — the game's own: an order cannot be addressed to a world-minute a fast clock never visits

Class: **the game's own**, recorded here because the next wave meets it ·
Game: ninjo · Open

Wave 1.1 retuned 1x to 24 world-minutes a real second (the owner's 0a
playtest). At 4x the clock then carries **1.6 world-minutes a tick**, which
has two consequences the substrate's shape did not anticipate and neither is
a bug:

1. **The clock skips minutes.** `floor(1.6t)` never lands on a minute whose
   residue mod 8 is 2, 5 or 7, so a scripted order addressed at such a minute
   fires at the next one the clock does visit — and at a *different* next one
   under each speed script, which breaks the invariance sweep at the
   conductor rather than in the world. The sweep's four orders were moved to
   minutes eight apart (8, 16, 24, 360) for this reason, and the reason is
   worth a sentence because the next content change will want to move them
   again.
2. **A multi-tick input sequence lands later than it starts.** An order is
   four ticks of clicking, which at 4x is six world-minutes. `When::Approaching`
   is the answer: the conductor simulates the clock forward the ticks the
   sequence takes and starts it early enough to *land* on the minute it names.

The design rule that survives both: **the invariance claim is about the
world's occurrences, not about the player's inputs**. Two speed scripts can
issue the same order at the same world-minute only where the clock visits
that minute at both speeds, and the sweep's scripts are authored to that.

**Amended by the job-board session (2026-09-06), and the rule is now a
check.** An order became three clicks (person, marker, job row), so its lead
went from three ticks to five and every one of the sweep's order minutes had
to be re-derived: they are now `sweep::ORDER_MINUTES` — 16, 32, 48 and 360,
multiples of eight sixteen apart, because six ticks of clicking at 4x is nine
world-minutes and 8 is where `floor(8k/5)` lands. The paragraph above said
"the next content change will want to move them again", and it did; so
`sweep::visited_minutes` now enumerates the minutes each speed's clock
actually reads and `sweep::orders_are_addressable` asserts every order minute
is in all three sets. **The entry stays open** because the rule still binds
the next change — but it now fails loudly instead of quietly, which is the
half of it that was missing.

### G-017 — the game's own: wave 1.1 made a party a character and left two selections over one roster

Class: **the game's own** (a wave-1.1 gap) · Game: ninjo ·
Files: `games/ninjo/src/flow.rs`, `src/screens.rs`, `UI.md` · **Fixed here**

Reported by the owner from a playtest of the deployed build: select a character
in the party strip, then click a *different* character's map sprite, and both
are ringed gold on the map.

Expected: one selection, because after wave 1.1 a party *is* a character — ten
parties, ten people, same order, `party.member == index`. Happened: S1's
dispatch pick (`Flow::selected`, an index over parties) and wave 0a's character
selection (`Flow::selected_person`, an index over people) survived wave 1.1 as
two independent fields over what had become one list, and each drew its own
ring — the token's at 36 units and the figure's at 38. Nothing was wrong with
either field on its own; what was wrong is that wave 1.1 unified the two lists
and did not unify the two states over them. Neither field's own tests could see
it: each asserted its own index, and no check counted rings.

The fix is a deletion, not a bridge: one field, written by every select path
and read by everything that highlights (UI.md §3b). The reproduction is
`verify::one_selection` — chip-select one, sprite-select another, count the
gold marker-sized quads on the photographed frame, expect the shipped literal
**one** — and `screens/ninjo-selection-reference.png` is that frame.

**What the next wave inherits:** the character panel's *body* is no longer a
click target (its close button and trait chips still are). It had to stop being
one: the panel now opens on any selection, so it is up for the whole of a
two-click dispatch, and at the reference camera it lies across the Watchtower's
and the Black Vault's markers — a swallowing body would have made two of the
four sites unorderable. `dispatch_reads_the_selection` orders to all four with
the panel open, so a later layout change cannot take that back quietly.

### G-018 — the game's own: an idle character is drawn twice, and the second one drifts further with every index

Class: **the game's own** (a wave-1.1 gap) · Game: ninjo ·
Files: `games/ninjo/src/screens.rs` ·
**Fixed 2026-09-08** — see G-023, which is the owner's report of it from the
deployed build and carries the fix, the floors and the doc-truth half

Found while working out which of a person's two portraits the one selection
ring belongs on, and left alone because this session's fence is the selection.

Expected: one picture of a person on the map. Happened: two. `content` draws a
**figure** at each at-home character's own tile (wave 0b's cast-at-home) and
`draw_map` draws a **party token** for every party, always — and after wave 1.1
those are the same ten people. The token carries S1's two-parties-on-one-tile
nudge, `(index * 4, index * -4)` world units, which was written when there were
three parties: at index 9 it is 36 units away, nearly two tiles, so the tenth
character's second portrait is not even overlapping the first.
`screens/ninjo-settlement-reference.png` shows all ten pairs at world-minute 0.

It is the same 1.1 gap as G-017 one layer down — two renderings of one roster
rather than two selections over it — and it is why `screens::selection_ring`
rings the *figure* when somebody is home and the *token* when they are on the
road: those are the two places a person is actually drawn. Fixing it is a
decision about what the map should show (a figure and no token while somebody
is idle, presumably, and the nudge retired or restated for ten) and it changes
five committed screenshots, which is an owner call rather than a bugfix
session's.

**G-010 stays open** for the fifth wave running, untouched here: this session
added map-space content (ten figures and their names) but no *new kind* of it,
and the culling pair it describes is unchanged.

### G-023 — the game's own: the map drew twenty figures for ten people, and the exception it hid behind was never written down

Class: **the game's own** (a wave-1.1 gap) and **a document that said less than
the code did** · Game: ninjo ·
Files: `games/ninjo/src/screens.rs`, `src/floors.rs`, `src/flow.rs`,
`src/layout.rs`, `src/shots.rs`, `src/verify.rs`, `UI.md` §3, §4, §6 ·
**Fixed here.** Closes G-018, which is the same defect found from the inside
two days earlier and left open as an owner call.

Reported by the owner from a playtest of the deployed build b48216a: character
sprites appear duplicated on the map, offset diagonally up-and-right; the whole
southern row of tents shows doubles, and in the northern row only Rin does.

Expected: one picture of a person. Happened: two, on every idle character.
`screens::content` drew a figure per person where `Lens::at_home`, at their
doorstep; `screens::draw_map` drew a sprite per party at `token_rect`. Since
wave 1.1 a party *is* a character and stands at that character's own doorstep,
so both paths drew the same person in the same place. `token_rect`'s per-party
nudge — `(index * 4, index * -4)` world units, S1 residue from three parties
stacked on the town tile — is what made the second copy visible and diagonal,
and it predicts the report exactly: index 0's duplicate hid perfectly, 1 to 3
offset 4 to 12 units and read as thickened sprites, index 4 (Rin) offset a
whole 16-unit tile and separated, and 5 to 9 offset 20 to 36 units per axis —
one to over two tiles. Rin is the fifth row of the northern row of tents and
the only one in it far enough out to read as two people, which is what the
owner saw.

**The second defect inside the first**, and the more expensive one: the nudge
put a token up to three tiles from the tile its party was actually standing on,
and the error grew with the roster. A figure that lies about where somebody is
lies to a click and to a camera focus too.

**Why nothing caught it, which is the half that is about a document.**
`UI.md` §6 said, without qualification, that every surface owes "every row of
its content in the `Panel`" — that is what makes `floors.rs` able to judge what
was meant and `frames.rs` able to find it on the frame. `screens.rs` took an
exception for the party tokens anyway, on the grounds that their between-tile
position is derived at draw time (ADR-0041), and recorded it **in its own
module header** — where nobody reading the rule in §6 would ever meet it. So:

- `floors.rs` judges the `Panel`, and the tokens were not in it;
- `frames::judge_chrome` asks only whether what the screen said is on the
  frame, never whether anything else is;
- the map-label overlap check covers `world_runs`, and labels were drawn once;
- `shots.rs`'s settlement check asserts *sim* truth — ten people, all home —
  which held perfectly.

The duplicate lived exactly in the gap the undeclared exception carved, on all
sixteen photographs, for two waves. **What was done on the document's
authority** was to leave the tokens outside the `Panel` when the figures moved
into it in wave 0b: §6 read as satisfied because the figures were content and
the tokens were "the drawing", and the sentence that would have said otherwise
was in a file the rule does not point at.

The exception was also not needed. Interpolation is a reading of the clock; a
`Panel` icon takes a position like any other, and `ui::draw` culls a world icon
to the camera exactly as the token loop did.

**The fix is one function.** `screens::where_drawn(lens, who, now)` is the
single answer to "where is this person drawn right now" — their doorstep while
`at_home`, their party's interpolated position otherwise, never both — and the
figure, the name under it, the selection ring and the map's hit-test all read
it and compute nothing of their own. The nudge became a **placement** rather
than a formula over an index: everybody takes the place they are standing if it
is clear and steps out a ring only because somebody else is standing close
enough to read as them, so a person alone is drawn on their own tile and two
people together are two figures.

**And the floor gap is closed, in the direction it was open.**
`floors::judge_cast` reads the `Panel` on every screen state the content floors
judge; `floors::judge_figures` reads the frame on every photograph, asking the
question `judge_chrome` never asked — whether the frame carries anything the
screen did not say it would draw. `verify::one_figure_each` is the
reproduction, and it failed before the fix on both photographed frames with the
owner's own numbers (`the settlement: 8 figure-sized quad(s) landed at corners
the screen never named ... the screen says 25 of them and the frame carries
35`). §6 now carries the whole list of what is exempt from the `Panel` and why,
and says that an exemption not in that list is a defect rather than an
exemption.

**What the next session inherits:** a site marker now takes a map click ahead
of a person's figure. It had to: a party working at a site stands on that
site's marker, and now that figures answer clicks on the road, a figure taking
it there would make the site unorderable while anybody worked it — the same
refusal `UI.md` §3a already makes for the character panel's body. A person
standing on a site is still selectable from the strip, the roster and a faces
list. And **G-022 is reopened on its subject**: it measured the settlement's
crowding against a map drawing twenty figures for ten people, so whether ten is
still crowded is the owner's to say before anybody moves a row or a label.

**G-010 stays open** for the sixth wave running, untouched by the
double-drawn-cast session: the party tokens moved from `ctx.sprite` into the
`Panel`'s world icons, which is a change of *which list* map-space content
lives in and not a new kind of it, and `ui::draw` culls a world icon on the
same bounds the token loop did. The culling pair G-010 describes is unchanged.
