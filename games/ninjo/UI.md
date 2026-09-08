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
  The top bar, the speed chips, the party strip and both drawers live here,
  so the chrome is a constant size *on screen* at any zoom, and every floor
  stays stated in reference pixels.

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
| a portrait, on the party strip | **whose chip this is** | the member's own face, so a person on the map and a chip on the strip are the same picture of the same person |
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
  under them at a doorstep and not on the road: a name has a tent under it to
  belong to, and a walking figure's status is the party strip's line. They
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
- **Party strip** (visible whenever no drawer is): **one chip per person**
  since wave 1.1, in two rows of five — portrait, name, and a one-line
  status (`at home` / `-> Watchtower` / `at Watchtower` / `-> Hana's` /
  `with Hana` / `<- home`). A party is a one-person band, so the strip and
  the roster are the same ten names; the portrait is the member's own, so a
  face on the road and a figure at a doorstep are the same person. **A chip is
  one of the four doors onto the one selection** (§3b): click one to select
  that person, then tap an open job on a site's board to dispatch. There is no
  idle gate on the *selection* — selecting is looking at somebody, and it is
  the order that refuses. A refused order bounces: a toast under the bar, and
  the same sentence in the notices. A
  drawer hides the strip rather than being drawn over it — a row of text
  under a scrim is still a row lying across a control.
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
- **Never two at once.** Opening any drawer shuts the others and closes both
  over-the-map panels (`Flow::close_everything`) — which, since the panel *is*
  the selection, means opening a drawer puts the selection down. A click that
  is not one of the open drawer's own controls shuts it. Under an open drawer the map's
  own chrome — the banner, the toast, the meters — draws nothing at all: a
  row nobody can read lying across a control somebody can click is exactly
  what the floors forbid.

## 3b. Selection, and the dispatch that reads it

**There is one selection.** One index (`Flow::selected`) over the ten people,
which is the same index over the ten parties — a party is a one-person band in
registry order, so a party *is* a character and the two lists are one list
twice. Every select path writes that field; everything that highlights reads
it. There is no second selection to keep in step, and keeping two in step was
never the fix.

**Four doors, one act.** Selecting a person — by their **map sprite**, their
**party-strip chip**, their **roster row's name**, or their **face in a faces
list** — does the same thing from every surface:

- they become *the* selected character;
- the character panel opens on them, and on nobody else;
- exactly **one** selection ring is drawn, on them — at their doorstep while
  they are home, on their token while they are on the road, never both. It
  computes no position of its own: `screens::selection_ring` asks
  `where_drawn`, the same function the figure is drawn at and the click is
  tested against, so a ring cannot come to sit beside the person it marks;
- their strip chip lights. The strip's highlight **is** the one selection, not
  a second one, which is what makes the strip a glanceable status row rather
  than a second selector.

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
chip-select one character, sprite-select another, count the rings on the
photographed frame — and `dispatch_reads_the_selection` and
`selection_moves_nothing` are the other two halves. The count is a shipped
literal: **one**.

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
- **A row's anatomy**: name and pot and duration on the first line; the
  **task-type chip** (icon + word), the **state** and the **fit** on the
  second. State is `open`, `<name> has it`, or `<name> did it` — open in
  regard-green, a taken row dim, a finished one fainter still.
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

## 4. Readability floors — what binds here

giri's §7 floors bind: text ≥ 12 reference pixels; clickable targets ≥
32x32 (chips, handles, party chips, tuner controls — and site markers,
whose 32-world-unit rects meet the floor at the reference zoom where one
world unit is one reference pixel); no interactive overlap; no text across
a control it does not label; stat numbers carry their icon (the treasury's
coin); ASCII everywhere.

The floors bind **every** surface §3a, §3b, §3c and §3d add: a feed row, a face
row, a config radio, a meter chip, a roster row, a **job row**, the board's
wage steppers and its `TO` toggle, a **ledger row's withdraw**, a **rate's
steppers and its STAND**, a trait chip anywhere it appears, and the character
panel's and the board's closes are all at or above the 32x32 target floor, none of them overlaps another control that
shares its screen, and every row of text is inside the surface that holds it.
`floors::controls_for` is the one function that says which controls share a
screen, so the overlap floor is asked about the right set — and since the job
board and the faces list share the left of the screen and are never open
together, **the base screen is two sets**, `targets()` and `board_targets()`,
and `layout_floors` judges both.

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
always have. Whether the settlement is too crowded to read is a judgement
about the map and it is the owner's (`FINDINGS.md` G-022).

**Map labels are bound at the default zoom, and that is what fixes the zoom.**
The chrome rides `UiMap` and is a constant size on screen however the camera
moves; a name under a figure is drawn in *world* units and shrinks as the
camera pulls back. `floors::map_legibility` states it: at the default camera a
name reads at exactly the twelve-pixel floor, and **one notch of the wheel out
puts it at 10.7**, under the floor — so the wave-1.2 rider to open the default
zoom out one level was refused by the floor, and the default stays at 540. The
crowding that rider was aimed at is real and is not a zoom problem
(`FINDINGS.md` G-022).

**The off-screen floor is restated for a camera that roams.** giri asserted
every quad inside the design rect; a pan/zoom map legitimately draws
partially-visible tiles at the view's edge. The fork's floors: every
*chrome* row and icon stays inside the UI rect; map-space content is culled
— nothing is submitted that does not overlap the view (to a label's width
of slack, since culling is per run, not per glyph); and zoomed in, the
tile count actually submitted drops. `verify.rs::culling_probe` and the
frame judges hold all three.

## 5. Screenshot process

Seventeen PNGs per verify run. Reference-only, because they are pictures of what
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
one character picked on the strip and another picked on the map, with one ring
and one lit chip on the second of them (§3b) — and, since the double-drawn
cast, **the ring on a token**: somebody picked while they are out, marked on
the figure walking the road with their own doorstep standing empty, which is
the selection's other state and had no picture at all while the map drew
everybody twice — and, since wave 1.2, **a
refusal mid-pause** (a posting made to somebody who says no, the world stopped
by `ask-declined`, and the reason on the banner) and **the postings ledger**
with one posting still recruiting, one refused, and the standing rates beside
them. At both the reference surface and
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
