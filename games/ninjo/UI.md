# ninjo — UI specification

Home: `games/ninjo/UI.md`. Owns the game's presentation: the map screen, its
chrome, the signifiers, and the mechanical readability rules. `DESIGN.md`
owns the rules of the world and `GDD.md` the rules of the game; where they
meet, the design documents win.

This file inherits giri's `games/giri/UI.md` wholesale — its principles
(§1), its floors (§7), its screenshot process (§8), its tuning-drawer rules
(§9a, §12) and its interim-UI standing law: **ugly is acceptable;
unreadable is a regression against shipped assertions.** What follows is
only what the fork changes or adds. Where a section is not mentioned here,
giri's text stands.

## 1. The one screen, and the two spaces

giri had three screens; the substrate has **one** — the map — with three
drawers over it (the feed, the auto-pause config, the tuning drawer) and,
over the map itself, the attention surfaces §3a describes. Everything drawn lives in one of two
spaces:

- **World space**: the terrain tiles, the location markers and their
  labels, the party tokens. Pans and zooms with the engine `Camera`.
- **UI space**: a 960x540 reference rect — giri's design rect, kept — that
  `camera::UiMap` fits uniformly inside whatever the camera shows, centred.
  The top bar, the speed chips, the drawers, the breakdown band and **the
  selected character's own name** live here,
  so the chrome is a constant size *on screen* at any zoom, and every floor
  stays stated in reference pixels. That last one is why the name of the
  person you are looking at survives a zoom that takes every other map word
  away (§4).

The mapping is giri's scaling contract (its UI.md §6) restated over a
camera that moves: aspect preserved, letterboxed, symmetric.
`floors::uimap_contract` asserts it at four viewports and three zooms.

## 2. Signifier vocabulary — the fork's changes

giri's colour roles stand. The changed and new rows:

| Signifier | Meaning | Notes |
|---|---|---|
| heart icon | **the town — home base**, and the `idle` meter chip | reassigned from regard, which the substrate does not have |
| an event class's colour + icon | **what kind of thing happened** — one chip per class on every feed row | the pair is the row's two channels; both come off `attention::CLASSES` and nothing else names them |
| the watchtower icon | the `away` meter chip | interim: no "out on the road" role exists in the curated set, and a thing that watches a road is the nearest one |
| gold, on a feed row | **the entry an auto-pause fired on** | the same fact as the reason line above it, from `Lens::pause` |
| a gold ring on a map figure | **the selected character — the one selection there is** | exactly one is ever drawn, on the figure at their doorstep or on their token on the road (§3b) |
| a portrait, on the map | **a character, standing wherever they are** | one per person and exactly one, at marker weight (32 world units), at their doorstep or on their road (§3, §4); named underneath only at a doorstep |
| dungeon icons (cave/crypt/tower/vault) | one quest site each | unchanged in art, now map markers; **a marker opens that site's board and issues no order** (§3c) |
| an aptitude chip on a job row | **what kind of work the job is** — the task type (`CAST.md` §2) | the icon is the aptitude row whose id *is* the task's, so the chip on a job and the chip on a person are the same picture of the same word |
| `open` in regard-green on a job row | **the row can be ordered from** | the only state a job row takes an order in; a claimed row is dim and a done row fainter still, and both name whose it is |
| `fit N` on a job row | **what the selected character brings to that work** | `traits::competence_at`, the scorer's own aptitude term; gold when it is more than nothing, faint at zero. Absent when nobody is selected |
| a verdict in regard-green or ember on a job row | **what the selected character would do about a posting here, and why** | `would take it` / `reluctant` / `would refuse`, from `answers::read` — the scorer's own answer, read-only. It stands where the row's state would be, because an open row's state is the word it replaces |
| `- 20g +` and `TO <name>` in the board's footer | **what the next tap offers, and whom it offers it to** | the board's own two controls; the wage opens at the standing rate for the site's first open row |
| `WITHDRAW` on a ledger row | **the one way to take a posting down** | instant, and heard the same way a posting is: a posting withdrawn before it was heard is one nobody answers |
| `STAND` / `STANDS` on a rates row | **a standing open posting for that kind of work**, at the rate beside it | gold while one stands — the closest thing to policy the player has by hand |
| `?` at the end of a job row or a feed entry | **the arithmetic behind the verdict beside it** | the only control on either surface that explains rather than acts; gold while its own sum is the one on the band (§3e) |
| a term line in the breakdown band | **one part of a decision, and what produced it** | `+6 aptitude - fighter`: the value, what the scorer calls the term, and the trait row or the fact behind it — the row's name read off the vocabulary now, never a sentence written per trait |
| a name in gold under a figure | **the selected character**, named in chrome | the one map word no zoom takes away (§4); every other name on the map is parchment and world-space |
| coin icon | the treasury | beside the gold number in the top bar |
| gold | the active speed chip, the selected character wherever they appear, selection | still not a general accent |
| terrain colours | the six terrain kinds | one colour per kind, `theme.rs`; the fill *is* the grid data |

**Terrain is flat colour tiles, deliberately interim.** DESIGN §3 imagines
"terrain kind → Kenney tile"; the owner's Kenney packs live on the owner's
machine and the repo's curated set has no terrain regions, so S1 ships one
named colour per kind (two channels: colour + the map's shape) and the
one-grid-two-readers discipline is held by verify asserting every drawn
tile's fill against the sim's grid. Curating real terrain tiles is a door,
not a queue — the import path (`art/`) rode along from giri.

## 3. The map screen

- **Top bar** (always visible): title, the clock readout `d1 06:40`
  (integer world-minutes, days from one), the four speed chips
  `PAUSE 1x 2x 4x` (active in gold; they do exactly what space and 1/2/3
  do), the treasury with its coin, and the five drawer handles — TUNE, ROSTER,
  LEDGER, FEED, MODES. **Seventy-two reference pixels wide since wave 1.2**,
  where four of eighty fitted and five did not; the row still starts right of
  the treasury.
- **The map**: terrain tiles culled to the camera; markers + labels + an
  open-job count per site (`2 quests` / `1 quest` / `dry`) — **the glance the
  board is read through**, and clicking the marker is how the board is
  reached (§3c); party tokens moving tile to tile, between-tile progress
  derived at draw time and never written back (ADR-0041; DESIGN §3). The
  selected character carries a ring and a lit chip — one of each, §3b.
  **Under an open job board the map's own words say nothing**, exactly as
  under a drawer: the board lies across markers and their labels at the
  reference camera, and the words it hides are the words it carries in full.
- **The cast, wherever they are** (wave 0b; one figure each since the
  double-drawn cast): every character is drawn exactly once on the map, at
  `screens::where_drawn` — at their home tile while they are there, on their
  party's moving token while they are out, **never both**. A **name** is drawn
  under them at a doorstep and not on the road — a name has a tent under it to
  belong to and stays where it was put, where a name pinned to a moving figure
  would come and go as the figure passed things; where somebody on the road is
  going is the roster's column and their own panel's line. **And a name is
  drawn only where the floor and the map leave room for it** (§4). They
  are **click targets since wave 0a**, and since the fix that is true on the
  road as well as at a door: clicking a figure selects that person and opens
  their panel (§3a), and the 32-world-unit figure meets the target floor at
  the reference zoom exactly as a site marker does. **A site marker takes the
  click first**, because a party working at a site stands on that site's
  marker and a figure that answered there would make the site unorderable
  while anybody worked it — the same refusal §3a makes for the character
  panel's body; a person standing on a site is still three doors away. What a
  person *has* — wallet, desperation and its source, traits — is on their
  panel and nowhere else.
- **The party strip is gone** (owner decision, 2026-09-08). It was one chip
  per person along the foot of the screen — portrait, name, and a one-line
  status — and by the time the double-drawn cast was fixed every job it held
  had somewhere better to live: **selection** to the map's figures, the
  roster's rows and a faces list (§3b's doors, three now instead of four);
  **dispatch** to the job rows (§3c); **who is out** to the meters band and
  its drill-down (§3a); **who is who** to the ROSTER drawer, which carries
  what the strip carried and four columns more. The band it held is the map's
  again, and the breakdown band (§3e) borrows it while somebody has asked a
  verdict why. It is the third S1 residue retired after the world it was built
  for stopped needing it (`FINDINGS.md` G-024).

  **That disposition list was checked for coverage and not for reach**
  (`FINDINGS.md` G-026). Selection did have three homes left, and one of them
  is the map — which is the surface the job board covers, and the board is the
  one surface that needs a selection to do its own job. Listing where a job
  moved is not the same as checking the new home is reachable in every mode
  the old one was. §3c's candidate picker is the fourth door, put back for
  that reason.
- **Pan/zoom**: arrows pan, `-`/`=` and the scroll wheel zoom; the camera
  clamps to the map and to a zoom range. All of it is input through the
  snapshot, none of it simulation state.
- **Feed drawer**: §3a. It replaced wave 0b's log drawer, which was a copy
  of the event list; the feed is a view of it.
- **Tuning drawer**: giri's §12 rules verbatim, at the game's thirty-four
  constants — three columns of twelve, with the stamp and the prose band in
  the last two hundred pixels to the right of them — and
  APPLY restarts the **scenario** (this game's boundary). The stamp ends
  `seed <n>`. The variant picker is gone with the variant machinery.
- **Trait chips are drawn on the character panel and on every roster row**,
  at the 16 units square the vocabulary specifies, in the icon each row
  carries. **A chip is a click target wherever it appears** (wave 1.1): the
  same tap on the same word opens the same one-line explanation, because the
  line is *derived from the row* — `traits::explain` reads the row's
  stranger-facing line and then the fields the row actually moves (`upkeep
  x3/2 / pulls toward any paid work`, `counts for fight tasks`, `bonds weigh
  x2`, `reacts to 4 public marks`). Nothing about a trait is written per
  trait, so a rename, a moved multiplier or a sixth want changes the
  explanation without anybody editing prose. The chip whose line is showing
  is drawn in gold.
- **The roster drawer** (wave 1.1, the ROSTER handle or the `r` key):
  **everyone in one list** — portrait and name, their chips, their purse,
  their desperation, and what they are doing *with the reason they are doing
  it*, all through the lens. The name selects that character (§3b), which
  shuts the drawer and opens their panel; a chip opens its explanation, on the
  row under the title. The row's own name box
  and its chips are separate targets, because a control inside a control is
  what the overlap floor refuses.

## 3a. The attention surfaces (wave 0a)

All of them are **interim UI under the standing law**, and all of them are
laid out in `layout.rs` and asserted in `floors.rs` like every other row.

- **The meters band**, under the top bar: one chip per registered aggregate
  (`meters::METERS`), each an icon, a label and a count. **A chip colourises
  only when its count is nonzero** — a zero is a chip you are allowed not to
  look at. Clicking one opens the **faces list**: a panel of portraits, names
  and the *reason* each is counted, never a bare number. Clicking a face
  selects that character (§3b) — the fourth door, and the same act as the
  other three.
- **The pause banner**, under the meters: one line, gold, present only while
  the world has stopped itself, saying the class, the place and what
  happened. The same sentence appears in the feed's header when the drawer is
  open (`attention::reason_line` — one source, two placements), and the
  banner is the closed-drawer half.
- **The feed drawer** (`FEED`): the sim's event log as a view, newest first,
  bounded by `feed_cap`. One row per entry, and the row's anatomy is
  **world timestamp · class chip · place tag · the sentence under them**. A
  row is a 920x32 click target and clicking it moves the camera to the
  event's place, leaves a pulse marker there for `pulse_tenths` tenths of a
  second, and shuts the drawer. A `IGNORED: HIDDEN/SHOWN` toggle reveals the
  classes the config swallows, dimmed, for auditing. Under the feed, a
  **notices** band: the last two things the *player* did (a speed change, a
  refused order, a restart) — kept apart from the feed on purpose, because
  none of them happened in the world.
- **The auto-pause config drawer** (`MODES`): one row per registered class,
  each with its chip and three radios — `ignore` / `log` / `pause`. The write
  goes into the simulation, and the footer says so.
- **The character panel**: portrait, name, trait chips, wallet, desperation
  and its source line, what they are doing, and where they live — every field
  read through `lens.rs`. **It is the selection's own surface**: it is open
  exactly while somebody is selected, on exactly that person, whichever door
  the selection came through (§3b). Its close button puts the selection down.
  **Its controls are a click target and its body is not**: the close button
  and the trait chips answer a click, and anything else that lands on it goes
  to the map underneath — a marker takes the order, a figure moves the
  selection, bare ground puts it down. The panel is up through the whole of a
  two-click dispatch and it lies across two of the four site markers at the
  reference camera, so a body that swallowed clicks would make those two sites
  unorderable.
- **Never two at once, and it is the type that says so** (the 2026-09-11
  session). There is **one drawer**: `Flow::drawer` is an `Option<Drawer>`
  over the five, the render is one `match` over it and the click routing is
  the same `match` over the same value, so "two drawers are open" is not a
  rule anybody keeps — it is a state this game cannot represent, and the
  drawer that is drawn is the drawer that answers a click by construction
  rather than by two lists agreeing. It was five independent flags absorbed
  under five `if`s, and the owner opened TUNE over an open ROSTER and got both
  (`FINDINGS.md` G-027). Opening any drawer **displaces** whatever was open
  and closes both over-the-map panels (`Flow::close_everything`) — which,
  since the panel *is* the selection, means opening a drawer puts the
  selection down; its comment claims exactly that and is now true of every
  drawer rather than of four of them.
- **The handles stand above the drawers.** The top bar is not covered, so a
  handle is live from inside another drawer: one tap displaces, and a tap on
  the open drawer's own handle puts it down. **The open drawer's handle is
  lit gold**, like every other control in this game that is showing what it
  opened, so which one of the five is up is readable from the bar rather than
  from the screen under it. Every handle and the `r` key go through
  `Flow::toggle_drawer`, and the five handles, their labels and each drawer's
  own head row are walked off `Drawer::ALL` — a sixth drawer is a variant of
  that enum and nothing else to remember.
- A click that is not one of the open drawer's own controls shuts it (the
  tuning drawer excepted, which its handle closes). Under an open drawer the
  map's own chrome — the banner, the toast, the meters — draws nothing at
  all: a row nobody can read lying across a control somebody can click is
  exactly what the floors forbid.

## 3b. Selection, and the dispatch that reads it

**There is one selection.** One index (`Flow::selected`) over the ten people,
which is the same index over the ten parties — a party is a one-person band in
registry order, so a party *is* a character and the two lists are one list
twice. Every select path writes that field; everything that highlights reads
it. There is no second selection to keep in step, and keeping two in step was
never the fix.

**Four doors, one act.** Selecting a person — by their **map sprite**, their
**roster row's name**, their **face in a faces list**, or their **row in a
job's candidate picker** (§3c) — does the same thing from every surface.
**The work list (§3f) is not a fifth**: it is opened *from* a selection and
names a job, not a person — the picker's mirror, running the other way. There
were four before the party strip retired too, and the one that went was the one
that did nothing the other three did not: **no path was lost with it**, which
is the test a retirement has to pass (§3). The picker is the one that came
back, and it came back because the three that were left all needed the player
to be able to reach the person: two of them are drawers that shut the board,
and the third is the map the board covers.

**The candidate picker is a writer of this field and not a selection of its
own.** It holds a *job* (`Flow::picking`, a `JobId` on the open board) and
there is nowhere in it to put a person; choosing a candidate sets
`Flow::selected` and returns, exactly as a faces row does. That is the whole defence: a
`picked_candidate` beside `selected` was the wave-1.1 bug (`FINDINGS.md`
G-017) and this was the third surface that could have reintroduced it.
Choosing **sets** rather than toggles, like a faces row and unlike a map
sprite — the picker's job is to name somebody for a job, and a tap that could
un-name the person it had just named would leave the footer reading `NOBODY`
at the moment the player went to post.

- they become *the* selected character;
- the character panel opens on them, and on nobody else;
- exactly **one** selection ring is drawn, on them — at their doorstep while
  they are home, on their token while they are on the road, never both. It
  computes no position of its own: `screens::selection_ring` asks
  `where_drawn`, the same function the figure is drawn at and the click is
  tested against, so a ring cannot come to sit beside the person it marks;
- **their name is drawn on the map in gold, in the chrome** — under their
  figure wherever it stands, at a constant size on screen, so the person you
  are looking at is named at any zoom and the label rule (§4) cannot drop
  them. It is the one map word that is chrome, and it is drawn *instead of*
  their world-space name rather than beside it: one person, one figure, one
  name. Like every other word the map says it goes quiet under a drawer or an
  open board, and like every other drawn thing it is not laid across a control
  it does not label — where neither under the figure nor over it is clear, it
  is not drawn, and the panel that is open on that same person carries the
  name in full.

Selecting somebody else moves the selection to them. Clicking the person who
is already selected puts them down; so does the panel's close button, a click
on bare ground, and opening any drawer. A selection that is put down closes the
panel with it — the panel is open exactly while somebody is selected.

**Nothing about it is simulation state.** The selection is presentation
derived from recorded clicks: two runs of one scenario that issue the same
clicks produce byte-identical transcripts whether or not anybody was ever
selected, and a replay carries the clicks, not the selection.

**Dispatch reads the same selection** and DESIGN §5's two clicks are unchanged
in number: select somebody, then tap an open **job row** on a site's board
(§3c). The marker is how the board is reached and it issues no order. If they
are idle the order is issued at the clock's minute; if they are out it bounces
with its reason (`sim::Refusal`), which is where the refusal belongs — the
*selection* of somebody who is out is allowed, because looking at a person is
not ordering them. A row tapped with nobody selected says so and does nothing;
dispatch never became one click. A successful order puts the selection down —
and the board with it, §3c.

`verify::one_selection` is the reproduction of the bug this rule replaced —
roster-select one character, sprite-select another, count the rings on the
photographed frame — and `dispatch_reads_the_selection` and
`selection_moves_nothing` are the other two halves. The count is a shipped
literal: **one**. All four doors are walked in the same battery, and the
picker's own half — that choosing writes this field and nothing else — is
`verify::the_picker_names_a_person`.

## 3c. The site panel — the job board, and the posting made from it

**A site marker opens a panel for that site, and asks nothing.** Before the
job board a marker showed a count and took an order, so with six jobs a site
the player could see how much work stood at a place and never what it was; the
owner's wave-1.1 playtest reported exactly that (`FINDINGS.md` G-019), and
"whom do I send, and why" is the decision the trait vocabulary exists to pose.
The panel is where that decision is made and given.

- **The panel** (`layout::board_panel`, left of the character panel and clear
  of it) carries the site's name and how much of it is open, and one row per
  authored job. It is not a drawer: it is up *with* the character panel
  through the whole of a dispatch, because the board says what the work is
  and the panel says who is being sent.
- **A row's anatomy**: name, pot, duration and the **fit** on the first line;
  the **task-type chip** (icon + word) and the **state** on the second. State
  is `open`, `<name> has it`, or `<name> did it` — open in
  regard-green, a taken row dim, a finished one fainter still. The fit moved
  up to the first line when the `who?` control took fifty-two pixels out of
  the row's width, and the reason it moved rather than the verdict shrinking
  is that the verdict's reason is the sentence wave 1.2 exists to print: the
  duration had left room on the first line, so the reason came out of the
  change **twenty-eight pixels wider** and the longest one the scorer can
  produce now fits it whole — on this row and on a candidate row, at the same
  number, so one sentence about one offer is cut in the same place on both.
- **Fit and travel belong to the one selection.** Each row shows the selected
  character's `traits::competence_at` for that task, and the header shows
  `sim::route_out` from wherever they stand — the same two functions the
  scorer and the dispatch use, never a second computation. **With nobody
  selected there is no fit column and no travel line at all**, because a fit
  for nobody is a number about nothing; the footer says what to do instead.
- **The row is the posting** (wave 1.2, and the whole of what changed). The
  same one-tap gesture that ordered somebody now **posts that job to them at
  the wage the footer is showing**, and they answer for themselves. The row
  says so before the tap: with somebody selected, each open row carries the
  scorer's own read of that offer — `would take it` / `reluctant` / `would
  refuse`, with the reason — from `answers::read`, which builds the candidate
  list a rescore would build and calls `autonomy::choose` on it. **A preview
  that could disagree with the decision is the failure this panel is most able
  to cause**, and one function is what refuses it. Tapping a claimed or
  finished row, or a row with nobody selected and nobody-to-anyone unset, or a
  job already posted, bounces in the established style — a toast under the bar
  and the same sentence in the notices — and changes nothing else. **Posting
  to somebody who is out is allowed**: asks travel, and it is heard when a
  messenger reaches them.
- **A row also carries a `who?`**, and it is what makes this board
  self-sufficient — see the picker below. On every **open** row, whether or
  not anybody is selected, and not while the `TO` toggle is offering to
  anyone: a bounty names nobody, so there is no candidate to pick and the tap
  says so instead of doing nothing. Like the `?`, it is a target of its own
  beside the row rather than a control inside one.
- **The board's own two controls** live in the footer band, not on the rows: a
  wage stepper (`- 20g +`) that opens at the standing rate, and a `TO` toggle
  that switches between the selection and anyone. They are the board's because
  a stepper inside a row would be a control inside a control, and because a
  row cut short to make room for one leaves no room for the verdict's reason —
  which is the sentence this wave exists to put on screen.
- **A made posting puts the board down with the selection.** The ask is made,
  the answer is the world's, and a board left open would lie across the
  markers the next one needs.
- **The board swallows clicks inside its own rectangle**, where the character
  panel's body falls through, and the difference is not an inconsistency. The
  panel is a passive detail view that must be up during a dispatch, so a body
  that took clicks would make the markers under it unorderable; the board *is*
  the dispatch surface, and a marker answering a click that landed between two
  rows would let a stray tap open a different site instead of ordering. The
  ways out of a board are its close, bare ground beyond it, any drawer, or the
  order itself.
- **The marker keeps its count** as the glance (§3): the count is what the
  board is read through, and it is the same number the board's header says in
  words. Under an open board the map's own words say nothing (§3).
- **The fit chip** in the header (`what is fit?`) says what the fit column
  means and what it does not mean yet — it sways whether somebody agrees, and
  every job succeeds until resolution lands (wave 1.4). It is derived from the
  wave it is in, like every other chip line.
- Six rows, which is what a site is authored with; `floors::layout_floors`
  asserts no site holds more, because a job with no row is a job nobody can be
  asked for now that the row is the posting.

**The candidate picker — the board naming its own person.**

**The defect was the requirement, not the occlusion.** Every drawer covers the
map — the feed, the roster, the ledger — and that is accepted. The board was
the one surface whose *purpose* required a map interaction: a posting had to be
aimed by selecting a character, and the three doors onto the selection left
after the party strip retired were two drawers that shut the board and the map
the board is lying across. So with a board open over somebody's sprite, that
somebody could not be posted to at all; and selecting first only works if you
already know what work stands at the site, which is what the board exists to
tell you (`FINDINGS.md` G-026). The board is not docked, inset, shrunk or
panned around. It names people instead.

- **A `who?` on each open job row opens that job's candidate list.** The
  **cast** — everyone, in one list, with the four things the choice turns on:
  the portrait and name, the **fit** for that job's task type, the **verdict**
  at the wage this row would offer (`would take it` / `reluctant` / `would
  refuse`, with the reason), and **where they are** with the **journey** from
  wherever that is. All four are the sim's own answers through the one function
  that answers each — `traits::competence_at`, `answers::read`,
  `Lens::whereabouts` over `Party::status`, `sim::route_out` — three of which
  the row and the header already call, once per candidate; the fourth is the
  sentence the roster's own activity column is built from.
- **Sorted by fit, descending; ties in roster order** (owner decision
  2026-09-09), which is a stable sort over the registry and so the same list
  twice for one frame. **The verdict is shown and never sorted on**: who is
  best at the work and who will agree to it are different questions, and the
  gap between them is the player's problem to solve. Nothing on the list is
  labelled best or recommended.
- **Everyone appears**, including the people who are out. Posting to somebody
  who is out is legal and travels, so leaving them off would hide a legal
  move; their state is what makes it plain what the player is doing.
- **It is a way of writing the one selection, not a selection of its own.**
  `Flow::picking` holds a *job*; choosing writes `Flow::selected` and closes
  the list, and the character panel, the map ring, the row verdicts and the
  footer's `TO <name>` all follow from that one field (§3b).
- **Posting stays on the job row.** The picker names a person; the row makes
  the posting, as wave 1.2 established. Choosing does not post. One way to
  post.
- **It draws instead of the board, not over it.** Ten rows of two lines do not
  fit the board's rectangle, and the space below the board is the breakdown
  band's — so the picker takes the whole left column and the board draws
  nothing at all while it is up. A row under a panel is text nobody can read
  lying across a control somebody can click, which is what §4's floors refuse.
  Its header carries the two things the covered board was still saying that
  matter: the **job by name** and the **wage these answers were read at**.
- **The picker and the breakdown band are never both up.** They are the same
  tap-deeper move on the same row and they want the same space, so opening the
  picker puts the band away — one rule in one place
  (`Flow::put_the_band_away`), like every other way out of it.
- **Fit is described in one voice because there is one chip.** The board's
  `what is fit?` chip stands in the same rectangle on both surfaces, toggles
  the same flag and prints the same sentence (`asks::fit_means`), dormancy
  clause and all. Two surfaces agreeing about fit is a thing somebody has to
  keep true; one chip is a thing nobody can break.
- **Closes on choosing, on the `who?` again, on its own X, or on the board
  going** — and the X is the board's own rectangle, because the close the
  player is looking at belongs to the topmost surface and a second X in a
  second place would be a second way to do one thing.
- **"The board going" includes the board being *replaced*.** A site marker
  outside the picker's own rectangle is still clickable, and a marker does not
  close a board — it opens a different one. So `Flow::picking` carries the
  whole `JobId` and not the slot: a slot alone would be a number read against
  whichever board is open, and the list would quietly become one of candidates
  for a different job than the `who?` that was tapped. The pair is checked on
  every tick (`Flow::put_the_picker_away`) and on every way out of a board
  (`verify::the_picker_names_a_person`).
- **Ten rows, which is the whole cast**; `floors::layout_floors` asserts the
  registry is not larger, because a candidate with no row is a person the board
  cannot name, which is the defect this surface exists to close.

**And the work list is this surface read from the other end** (§3f). The
picker holds a job and lists the people; the work list holds a person and
lists the jobs. Same decision — whom to post a job to — same three functions
answering it (`traits::competence_at`, `answers::read`, `Lens::travel`), same
one chip describing fit, same rule that neither of them posts. Two ways to
*arrive* at the board's row, one way to post from it.

**This is the assign-picker wave 1.5 was going to invent.** The petition-card
anatomy recorded at wave 0a names an "assign-picker with willingness hints" as
part of the card; this is that widget, arriving early because the job board
needed it first. **Wave 1.5 inherits it rather than building a second one** —
the willingness hint is the verdict column, and it is `answers::read` on both
surfaces or it is two answers to one question.

## 3d. The postings ledger, and the standing rates (wave 1.2)

**One drawer, two bands**, for the reason the feed drawer carries the notices
band: they are two readings of one subject, and a player deciding what to pay
is a player looking at what they have already promised. The LEDGER handle
opens it; with the asks module off it does not open at all and says so.

- **The ledger band** (left, eight rows, newest first) is **a view of
  `Sim::postings`**: every row is derived from the record on every draw, so
  there is no state here that could disagree with what the scorer weighs. A
  row is two lines — *status, who, what, wage, until* over *when it was made
  and who has heard or answered it, with the reason they gave* — and a
  standing posting carries a `WITHDRAW` button, which is the one way a posting
  comes down.
- **The rates band** (right, four rows) is a name, the rate, a `-` and a `+`,
  and a `STAND` button that posts that kind of work to anyone until withdrawn.
  Both writes go into the simulation, like the auto-pause radios: a rate
  stepped and a posting withdrawn are recorded inputs that change what the
  world does.
- **The job row is where a posting is made** (§3c); this drawer is where the
  player sees what they have asked for and what it costs. Nothing is posted
  from here except the four standing postings, and nothing else can withdraw.

## 3e. The breakdown — the scorer's arithmetic, one tap deeper

**A verdict a player cannot account for is the thing this surface fixes.** A
job row says `would refuse` and a reason; the reason is the loudest term of a
sum with five or six terms in it, and until this session the rest was computed
and thrown away. The owner's wave-1.2 playtest reported the consequence — "it
is difficult to determine what traits actually do" — and the answer is not
more words on the row but the sum itself, one tap deeper.

- **The verdict stays the headline.** Nothing about a row changes until the
  `?` at the end of it is tapped. Arithmetic shown by default would bury the
  three words the row exists to say.
- **The `?` is a target of its own**, at the end of the row and clear of it,
  because the row *is* the posting and a control inside a control is what the
  overlap floor refuses — the same separation the roster row makes between its
  name box and its chips (§3). It is drawn only where there is a sum to show:
  on an open row with somebody selected, and on a feed entry that recorded a
  decision. Tapped anywhere else it bounces and says why.
- **The band** (`layout::breakdown_panel`) is the space the party strip left,
  the width of the screen: five rows of two columns, ten cells — one more than
  the widest sum this game can produce, which `layout_floors` asserts against
  the scorer's own term count rather than against a memory of it. It took the
  width from the character panel, which gave up the sixty pixels of height it
  had briefly taken from the same band: a term line is a value, a word and an
  attribution, and the attribution is the half a narrow cell clips off.
  `layout_floors` walks every term line the vocabulary can produce and asserts
  the widest fits a cell, because a want covered by two of somebody's
  motivators names both rows and the third one authored would say so silently. Each cell is one term: **the value, what the scorer calls the term, and
  what produced it** (`+6 aptitude - fighter`). Then the total. Then, where the
  verdict is a refusal, **the candidate that beat it and by how much** —
  because a no is only accountable beside the yes it lost to.
- **Two askers, one band, one derivation.** A **job row** asks about an offer
  the player has not made yet: its sum comes out of the *same* `answers::Reading`
  that produced the verdict above it, so a band that disagreed with the row it
  explains is not a state this surface can reach. A **feed entry** asks about a
  decision already made: its sum is the `autonomy::Judged` the scorer decided
  from, recorded on the occurrence beside the reason it already carried
  (`Sim::remember`). Both are `autonomy::Reckoning` and one renderer draws
  them.
- **Recorded, not recomputed.** A sum re-derived ten minutes later would be a
  different sum — the board has been claimed, the wage stepped, the regard
  drifted — and a breakdown that disagrees with the decision it explains is
  worse than none. So the terms are kept where `Event::gold` is kept and for
  the same reason (§6's one-source rule applied to a decision). It is a record:
  nothing in the simulation reads it, no arithmetic depends on it, and a run
  that never opens a band is byte-identical to one that opens every one.
- **The attribution is derived from the row.** A term carries the trait rows'
  *ids*, and the name is read off the vocabulary at the moment the line is
  built, so a renamed row renames its line with nothing else edited. Where no
  row produced a term it carries the fact instead (`desperation 4`, `wage 12g`,
  `asked by name`). A hand-written string per trait is the failure mode this
  refuses, and `verify::attribution_is_derived` is what holds it.
- **The band belongs to the surface that opened it.** A job row's sum does not
  outlive its board or the selection; a feed entry's does not outlive the
  drawer. One rule in one place (`Flow::put_the_band_away`), because there are
  a dozen ways out of those surfaces and the band cannot be what every one of
  them remembers. Under the feed the **notices** band steps aside rather than
  being drawn beneath it.

## 3f. The work list — the board read from the person's side

**The same decision, approached from the person instead of the job.** The
board serves "whom do I post this job to"; this serves "what work is there for
*them*". The owner's 2026-09-11 playtest asked for it: finding work for one
character meant opening four boards in turn and remembering what each held.

- **The door is on the character panel**: a `work N` chip in the sheet's own
  header, beside the name it is about, carrying the number of jobs standing
  open anywhere. It is in the header because at its widest state — a trait's
  longest explanation open under the flowed rows — the sheet ends **twelve
  reference pixels** above the panel's foot, and a target is thirty-two. The
  count is the one thing about the list that belongs on the sheet whether or
  not the list is up.
- **The list stands in the left column**, in the board's own rectangle,
  drawing **instead of** the board exactly as the candidate picker does and
  for the same reason: the left of the screen is one column, the character
  panel has the other, and a surface drawn under another is a row nobody can
  read lying across a control somebody can click. `Flow::listing` holds the
  *person*, the way `Flow::picking` holds the *job*.
- **A row's anatomy** — task chip, the job's name, its pot, its duration and
  the **fit** on the first line; the **travel from wherever they stand** and
  the **verdict at that job's standing rate, with its reason**, on the second;
  and **which site it stands at**, which is the thing a list across every
  board has to say that a single board never did.
- **Every number is the sim's own answer**, from the three functions the board
  and the picker already call — `traits::competence_at`, `answers::read` (via
  `board::reading_at`, at the standing rate), `Lens::travel`. Nothing here
  recomputes anything, and `verify::the_work_list_navigates` asserts every
  row's three answers against those functions directly.
- **Sorted by fit, descending; ties in site then authored order** — the walk
  is sites in registry order and slots in authored order and the sort is
  stable, so a tie keeps what it arrived with and two readings of one frame
  are the same list. **The verdict is shown and never sorted on**, exactly as
  on the picker.
- **Ten rows, and the cap is declared.** Four sites of six jobs is
  twenty-four, the column holds ten, and the header says `10 of 24 open` so a
  list that stops is a list that said it stopped. The ten are the ten the sort
  puts first; there is no scrolling.
- **Tapping a row navigates. It does not post.** The board opens on that job's
  site with the character still selected, the list goes down, and the board's
  row posts as it has since wave 1.2 — one way to post, two ways to arrive at
  it. "In view" is the site's whole board: `layout_floors` asserts a row for
  every job a site is authored with.
- **Fit is described in one voice because there is one chip.** The board's
  `what is fit?` stands in the same rectangle here, toggles the same flag and
  prints the same sentence (`asks::fit_means`) — the third surface to show
  fit and still the first chip.
- **It goes away where its person or its column does.** The list is orphaned
  when the selection moves off it, when a board opens, and when a meter chip
  is drilled — one rule in one place (`Flow::put_the_list_away`), the same
  shape as the picker's and the band's.
- **With the asks module off there are no verdicts**, and the column says so
  rather than being blank: the module's degrades-to sentence, on this surface.

## 4. Readability floors — what binds here

giri's §7 floors bind: text ≥ 12 reference pixels; clickable targets ≥
32x32 (chips, handles, tuner controls — and site markers,
whose 32-world-unit rects meet the floor at the reference zoom where one
world unit is one reference pixel); no interactive overlap; no text across
a control it does not label; stat numbers carry their icon (the treasury's
coin); ASCII everywhere.

The floors bind **every** surface §3a, §3b, §3c, §3d, §3e and §3f add: a feed
row, a
face row, a config radio, a meter chip, a roster row, a **job row**, the
board's wage steppers and its `TO` toggle, a **ledger row's withdraw**, a
**rate's steppers and its STAND**, a **`?` and a `who?` on a job row** and a
**`?` on a feed entry**, a **candidate row**, a **work row** and the sheet's
**work chip**, a trait chip anywhere it appears,
and the character panel's and the board's
closes are all at or above the 32x32 target floor, none of them overlaps another control that
shares its screen, and every row of text is inside the surface that holds it.
`floors::controls_for` is the one function that says which controls share a
screen, so the overlap floor is asked about the right set — and since the job
board, the faces list, a job's candidate picker and a person's work list all
share the left of the screen and no two of them are ever open together, **the
base screen is four sets**, `targets()`, `board_targets()`,
`picker_targets()` and `worklist_targets()`, and `layout_floors` judges them.

**Two floors the 2026-09-11 session added, because the screen they were owed
against was wrong and nothing said so.**

- **No two rows of chrome on one band collide.** `judge_panel` asked chrome
  text against *controls* and map labels against *each other*, and never
  chrome against chrome — so the tuning drawer's stamp, which grows a row
  every other constant, walked into the prose band beside it and passed every
  check (`FINDINGS.md` G-028). **On one band**, because the layers are what
  make an overlay legitimate: the breakdown band is drawn over the feed
  drawer's footer with its own ground behind it and that is deliberate (§3e),
  while two rows on the same band are two rows drawn through each other.
- **At most one drawer's content in the frame.** With `Flow::drawer` one
  `Option<Drawer>` this is unrepresentable (§3), and the floor says it anyway,
  so the next surface to grow an open-flag of its own fails here rather than
  being found in a screenshot. It counts by each drawer's own head row
  (`Drawer::title`), which is the string the drawer prints and the floor
  reads — one string, so the two cannot drift into a floor that sees nothing.

Both are **demonstrated failing** on the screens they were written for
(`floors::floors_bite` stages the pre-fix right column and a frame carrying
two drawers, judges them into a throwaway `Checks`, and asserts each claim is
reported by name). A floor nobody has seen fail is a floor nobody knows is
connected.

**A band whose height is data is measured, not offset.** The tuning drawer's
right column and the character panel's lower rows both used typed offsets that
were true at the lengths of the day they were typed, and both had grown
through the row below (`FINDINGS.md` G-028, G-029). Both now flow: each block
starts where the one above it ended (`tuning::prose_top`, the character
panel's flowed rows), and `floors::tuner_right_column` asserts the drawer's
column still fits at `tuning::STAMP_HEADROOM` rows of growth — it fails while
there is still room, so the wave that adds the constant is told to re-lay the
column instead of finding out from a screenshot.

**A cell that clips is a floor here, not a nicety.** The picker's four columns
and the job row's verdict cell are asserted against the widest thing the game
can put in them — the widest verdict line the scorer produces over a played
world, the longest journey the pathfinder returns, the whole of a candidate's
whereabouts — because what a clip takes off a verdict is its reason and off a
journey is its number, and those are the halves a player decides on
(`verify::the_picker_names_a_person`).

**One person is one figure, and two people are two.** A character is drawn
exactly once, at `screens::where_drawn` — their own doorstep while they are
home, their party's interpolated position while they are on the road, never
both and never neither. A figure standing alone is drawn on its person's own
place with no offset at all; a figure is nudged only because somebody else is
standing close enough to read as them, and the nudge separates them by at
least `floors::FIGURES_APART` (sixteen units, half a figure). §6 names the two
floors that assert it.

Note what that does **not** say: that no two figures overlap. A figure is 32
world units and a tile is 16, so people at neighbouring doorsteps overlap and
always have — **a picture is never dropped**, only a word. Whether the
settlement is too crowded to read is a judgement about the map and it is the
owner's (`FINDINGS.md` G-022); what the label rule below took off it is the
northern row's names landing on the southern row's heads, which is what that
finding measured.

**The floor governs the map's words; it does not veto the camera.** The chrome
rides `UiMap` and is a constant size on screen however the camera moves; a name
under a figure is drawn in *world* units and shrinks as the camera pulls back.
`floors::map_legibility` states the two numbers: at the default camera a name
reads at exactly the twelve-pixel floor, and **one notch of the wheel out puts
it at 10.7**. Until the legibility session that arithmetic was used as a
refusal — the name was going to be drawn whatever the camera did, so the floor
forbade the zoom, and wave 1.2's rider to open the default out one level was
turned down. That was the wrong way round. **The label is what yields now:**

- a label is drawn only where it would clear the floor **and** land on nothing
  already drawn — no figure, no marker, no label accepted before it, and no
  chrome surface that is up (the character panel and its kin have a fill above
  the map's text band, so a word under one was never read, only hidden);
- the order is fixed — markers, then figures, then every site's name and count
  in registry order, then the cast in registry order — so **the same frame
  always drops the same labels**, and nothing about which survive depends on
  who asked or when;
- **below the floor none is drawn at all**, so pulling the camera back costs
  the names and never their legibility;
- **the selected character's name is chrome** (§3b) and is drawn instead of
  their world-space name, so it survives every zoom;
- **site markers and their counts follow the same rule** as a person's name.

So zooming out is permitted, and what it costs is legible: the names go and
the picture stays. `verify::map_labels_are_governed` asserts all of it, and
`screens/ninjo-zoomed-reference.png` is the picture. **Where the default
camera should sit is a play judgement and the owner's** — this rule is what
makes it a judgement they can make, and it deliberately leaves the default at
540 (`FINDINGS.md` G-022).

**The off-screen floor is restated for a camera that roams.** giri asserted
every quad inside the design rect; a pan/zoom map legitimately draws
partially-visible tiles at the view's edge. The fork's floors: every
*chrome* row and icon stays inside the UI rect; map-space content is culled
— nothing is submitted that does not overlap the view (to a label's width
of slack, since culling is per run, not per glyph); and zoomed in, the
tile count actually submitted drops. `verify.rs::culling_probe` and the
frame judges hold all three.

## 5. Screenshot process

Twenty-four PNGs per verify run. Reference-only, because they are pictures of what
is on screen rather than of how the chrome scales: **the settlement** at
world-minute 0 (the whole cast standing at their homes, named, before
anything is dispatched, which is wave 0b's own exit question), **the
auto-pause config** with a class set to pause, **a character's panel** with
the selection ring on their figure and a trait chip tapped, **the roster**
with a chip's explanation open on it, the **job board** in its three states —
read by a selected character (the fit column and the travel line up, on a
site whose rows are not all one kind of work), the moment after an order was
given from one of its rows, and refusing a row somebody already has — and
**the world living on its own** —
the map at a minute when nobody was told to go anywhere and half the band is
on the road because they decided to be, which is wave 1.1's own exit
question — **the selection reproduction**: the owner's own playtest steps,
one character picked on the roster and another picked on the map, with one
ring and one gold name on the second of them (§3b) — and, since the double-drawn
cast, **the ring on a token**: somebody picked while they are out, marked on
the figure walking the road with their own doorstep standing empty, which is
the selection's other state and had no picture at all while the map drew
everybody twice — and, since wave 1.2, **a
refusal mid-pause** (a posting made to somebody who says no, the world stopped
by `ask-declined`, and the reason on the banner) and **the postings ledger**
with one posting still recruiting, one refused, and the standing rates beside
them — and, since the legibility session, **three more**: a job row's
**breakdown open on a refusal**, with every term, what produced each, the total
and the job that beat it (§3e); **a decision's breakdown in the feed**, which is
"why did they go there" answered after the fact; and **the settlement one notch
of the wheel out**, where the map's words are gone, the figures are not, and
the selected character is still named because that name is chrome (§4). The
last of those is the picture the owner judges the default camera against.
And, since the candidate-picker session, **two more**: **a job's candidate
picker open with nobody selected** — the cast for one job, best fit first,
each of them named, fitted, answered and placed, which is the picture of a
board that can be aimed without reaching the map it is covering — and **the
board after choosing from it**, the footer reading `TO <name>`, that
character's panel up, and the row still the thing that posts. The first is
asserted to be a mixed-fit job whose list carries at least one refusal and at
least one person who is out, and the row it chooses from is asserted *not* to
be the best fit on the list, because "the player took the top row" is the one
reading that picture must not support.
And, since the 2026-09-11 session, **two more**: **the work list** open on a
selected character — every job standing open anywhere, sorted by fit, with the
character panel beside it saying whose list it is (§3f), asserted to carry a
spread of fits and at least one refusal, because a list of ten identical
yesses is a picture of a list and not of this decision — and **TUNE opened
over an open ROSTER**, the owner's exact path, showing one drawer and a right
column whose stamp and prose are clear of each other. The second is asserted
to carry exactly one drawer's head row.
At both the reference surface and
600x540 narrow: **the mid-travel map** (photographed with two parties on
visibly different routes) and **the feed mid-pause** (the reason line
showing, and the entry that stopped the world ringed in gold). Plus the
tuning drawer (reference only, pending state showing gold).

Committed copies live in `screens/`; the implementing agent opens and looks
at every one before declaring done.

**Text is the built-in bitmap face, deliberately.** The engine's TTF support
landed before wave 0a and was not adopted: the owner's verdict is that
proportional-heavy display faces are out for dense information, and the feed
is the densest surface this game has. Every floor and every glyph-count
assertion is stated against the five-by-seven face.

## 6. What binds a new surface

Every surface added after wave 0b owes the same three things, and the
verify run is where they are owed: every row of its content in the `Panel`
(so `floors.rs` can judge what was *meant* and `frames.rs` can find it on
the frame), every string ASCII (`library.rs` walks them), and every read of
the world through `lens.rs`. The last one is the easy one to skip and the
expensive one to retrofit — see that module's header for why.

**What is exempt from the first, and why — the whole list.** Only three
things on this screen are drawn past the `Panel`, and they are named here
rather than only at their code site:

- **the terrain**, because a 48x27 map is a thousand quads no panel should
  carry, and every drawn tile's fill is asserted against the sim's grid
  instead (§2);
- **the selection ring** and **the focus pulse**, because both are fills
  rather than content — neither carries a string, an icon role or a
  position of its own, and the ring's rectangle is `screens::where_drawn`,
  which the floors judge on the figure it rings.

**The party tokens were a fourth, undeclared, and that is where the
double-drawn cast hid** (`FINDINGS.md` G-023). This section said "every row
of its content in the `Panel`" without qualification; `screens.rs` took an
exception for the tokens on the grounds that their between-tile position is
derived at draw time, and wrote it in its own module header, where nobody
reading the rule would meet it. It was not even needed — interpolation is a
reading of the clock and a `Panel` icon takes a position like any other. The
cost of the undeclared exception was that the one thing on the map drawn
outside the `Panel` was the one thing drawn twice per person, on all sixteen
photographs, for two waves, and no floor could see it. **An exemption that
is not in this list is a defect, not an exemption.**

**The candidate picker adds no exemption either** (§3c). Its fill, its border
and its row ghosts are chrome fills like the board's; every row of text on it —
the header, each candidate's name, fit, whereabouts, verdict and journey, the
footer — is a `TextRun` in the `Panel`, and its portraits are `Panel` icons, so
`floors::judge_panel` judges what it says and `frames::judge_chrome` finds each
row on the frame. That is why `shots::judge_picker` can assert every
photographed candidate row is the person the list puts there.

**The breakdown band adds no exemption** (§3e). Its fill and its border are
chrome fills, like the character panel's; every row of text on it — the
heading, each term, the total, the line naming what beat a refusal — is a
`TextRun` in the `Panel`, so `floors::judge_panel` judges what it says and
`frames::judge_chrome` finds each row on the frame. That is why
`shots::judge_breakdowns` can assert the recorded terms of a decision appear
on the photograph of the band that explains it, term for term.

**And the map's figures are counted now.** Two floors, both directions:
`floors::judge_cast` reads the `Panel` — one figure per person, at
`where_drawn`, drawn exactly where its person stands unless somebody else is
standing there, and no two closer than `floors::FIGURES_APART` — on every
screen state the content floors judge. `floors::judge_figures` reads the
frame — the number of figure-weight pictures the screen named, at the
corners it named, and no others — on every photograph the run keeps.
`frames::judge_chrome` asks only whether what the screen said is on the
frame; the second is what asks whether anything else is.

## 7. Asset slots — the roles, and what fills them

giri's §9 forecast the slots and its §11 recorded the sizes they landed at.
The fork's table supersedes both, because the fork has a different set: no
card, no quest detail panel, a map that draws portraits as tokens, and — since
2026-09-02 — the founding cast's fifteen new roles (CAST.md §4, §9).

**The role is the contract, not the picture** (`src/sprites.rs`,
DESIGN §12's curation model). Every slot is a native texel size drawn at a
whole-number multiple of it; `Art::scale_across` takes the size a row wants and
panics if the two do not divide, and the readability floors assert the same
thing over every icon actually drawn (§1.4, §4).

| Slot | Roles | Texels | Drawn | Source |
|---|---|---|---|---|
| portraits | `portrait_{alex,bob,steve,tim,rin,goro,hana,ludo,ines,odd}` | 16x16 | 32 units (scale 2) on the map, the party chip and the character panel | Tiny Dungeon |
| quest icons | `quest_{cave,crypt,tower}` | 8x8 | 32 units (scale 4) as a map marker | Micro Roguelike |
| quest icon | `quest_vault` | 16x16 | 32 units (scale 2) as a map marker | Tiny Dungeon |
| stat and event icons | `icon_{flame,coin,skull,heart}` | 8x8 | 16 units (scale 2) as a chip; the heart also at 32 units (scale 4) as the town marker | Micro Roguelike |
| stat icon | `icon_eye` | 8x8 | 16 units (scale 2) | generated (`art/sprite_defs.py`) |
| aptitude chips | `icon_{fight,labor,scout,craft}` | 8x8 | 16 units (scale 2) as a trait chip | Micro Roguelike |
| motivator chips | `icon_{indebted,renown,caring,restless,maker}` | 8x8 | 16 units (scale 2) as a trait chip | Micro Roguelike |

Twenty-eight roles; `assets/CREDITS.md` carries one row per file and
`art/kenney-manifest.json` says which pack region fills which.

**The nine trait chips are roles ahead of their wearers.** They are in
`Art::ALL` and in `Gallery::load` so `tools/check-assets` and `library.rs`'s
art contract both know the names, and nothing draws them: wave 1.1 lands the
trait rows that carry them. The five personality chips keep the category icons
§3's trait-chip rule gave them (coin, heart, eye, flame, skull) — that
borrowing is unchanged, and the nine new icons do not touch it.

**The two chip families are told apart by weight, not by subject.** An
aptitude is line-work — a steel-and-timber implement with the panel showing
through it (a sword, a ladder, a lantern, a hammer); a motivator is one filled
warm mass that fills its cell (a satchel, a flag, a joint of meat, a chevron, a
bench). That is the cue a glance uses at 16 units, where the *subject* of an
8x8 picture is not yet legible and its weight already is. `art/sprite_defs.py`
draws the generated fallbacks to the same cue, so a withdrawn pack does not
change which family a chip reads as.

**The eye is still the one generated slot.** No eye glyph exists in any of the
packs (`art/kenney-manifest.json`'s `gaps`), and §2 fixes what the eye means, so
the slot keeps its violet icon rather than taking a substitute. The scout
lantern is the second gap recorded there: neither pack has a boot, a footprint,
a spyglass or a map, and the lantern is the nearest thing to "knows the way, or
finds it" that survives the 16-unit chip.

**No two portraits may read as one person at map scale.**
`library::portraits_are_tellable_apart` asserts it over every pair of portrait
roles, at native texel size on the ground colour: at least 15% of texels must
differ by more than 24 of 255 on some channel. Both numbers are shipped
literals rather than anything derived from the art, so a `chosen` edit that
picks a near-duplicate fails the verify run instead of arriving as two
identical figures standing at two home tiles. The landed ten clear it with room
(CAST.md §9 has the measurements).

**Picking is the owner's** (DESIGN §7), with one recorded exception: the
2026-09-02 cast-art session picked the fifteen against written criteria because
the owner was away from the machine with the packs, and the approval moved to
the PR by way of the committed picks sheet `art/picks/cast-2026-09.png`
(CAST.md §9). That is one session's dispensation and not a change to the model.
A veto is one line — edit `chosen` in the manifest — applied by any later
session with `art/extract.py` and `art/import_pack.py`.
