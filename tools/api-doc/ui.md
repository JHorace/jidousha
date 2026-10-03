A game's chrome — the bar across the top, the panel a tap opens, the feed of
what happened, the chip whose explanation unfolds — is where a game stops
being shapes and starts being read. The kit is what a game that has reached
that point imports. It came out of one game that built every part of it
three or more times, and it carries the one lesson that game paid for five
times over: **a surface's state is one value, never a flag beside an
`Option`.** Everything below is either that lesson as a type, or the floors
that catch its absence.

### A screen is data first, and drawn second

Every screen hands back a `Panel`: every row of text and every icon it puts
on the frame, with its position. Three readers take the same panel — the draw
system turns it into quads, the floors judge what was meant, and the frame
check finds every row on the recorded frame — so the thing drawn, the thing
judged and the thing photographed cannot be three different lists.

A `TextRun` is a position, a string and the `TextStyle` it is drawn in; the
style carries the size, the colour, the face and the band. An `IconRun` is a
position, one of *your* picture roles, a whole-number scale, a tint and a
band. The kit never names a texture: your art enum implements `Icon`, which
says only how big a role is at a scale, and your art library turns an
`IconRun` into a `Sprite` at draw time.

```rust
# #[derive(Clone, Copy, Debug, PartialEq)]
# enum Art { Coin }
# impl Icon for Art {
#     fn size_at(self, scale: f32) -> Vec2 { Vec2::splat(16.0 * scale) }
# }
# fn wallet() -> i64 { 40 }
let small = TextStyle { size: 12.0, color: Color::WHITE, depth: Depth::layer(1), ..TextStyle::default() };
let mut sheet: Panel<Art> = Panel::default();
sheet.icon(IconRun::new(Vec2::new(10.0, 10.0), Art::Coin, 2.0, -1));
sheet.text(TextRun::new(Vec2::new(46.0, 20.0), format!("{}g in hand", wallet()), small));
// A paragraph is stored as the rows it is, so a glyph count per row stays exact.
let next = sheet.block(Vec2::new(10.0, 50.0), "two\nrows", small, 2.0);
assert_eq!(sheet.runs.len(), 3);
```

Chrome is laid out in a **design space** of your own — 960x540 reference
pixels is the shape the kit was extracted from — and rides the camera: a
`Mapping` says where a design point lands in the world and how many world
units one design unit is, and `Panel::draw` places every chrome row and icon
through it, scaled with it, while `world_runs` and `world_icons` are drawn
where they are and culled to the camera's view. Lay the design rect out once,
fit it inside whatever the camera shows, and the floors stay stated in design
units at every zoom.

```rust
# #[derive(Clone, Copy, Debug, PartialEq)]
# enum Art { Coin }
# impl Icon for Art {
#     fn size_at(self, scale: f32) -> Vec2 { Vec2::splat(16.0 * scale) }
# }
# fn screen() -> Panel<Art> { Panel::default() }
# struct Gallery;
# impl Gallery { fn sprite(&self, _art: Art, _scale: f32, _layer: i16, _tint: Color) -> Sprite { unreachable!() } }
# fn gallery() -> Gallery { Gallery }
/// The design rect fitted inside the view, centred: uniform, letterboxed.
struct UiMap { origin: Vec2, scale: f32 }
impl UiMap {
    fn for_camera(camera: &Camera) -> Self {
        let view = camera.visible_bounds();
        let scale = (view.size().x / 960.0).min(view.size().y / 540.0);
        Self { origin: view.center() - Vec2::new(960.0, 540.0) * (scale * 0.5), scale }
    }
}
impl Mapping for UiMap {
    fn to_world(&self, ui: Vec2) -> Vec2 { self.origin + ui * self.scale }
    fn scale(&self) -> f32 { self.scale }
}

fn draw_chrome(ctx: &mut DrawCtx) {
    let map = UiMap::for_camera(ctx.world.resource::<Camera>());
    let art = gallery();
    screen().draw(ctx, &map, |icon, scale| art.sprite(icon.art, scale, icon.layer, icon.tint));
}
```

A drawer is built out of ordinary rows and **lifted** onto the overlay's band
at the end — `panel.lifted(layer)` — in one place rather than per row. Two
spellings of one thing is how a band gets missed on the row added last.

### One value, never a flag beside it

The game the kit came from grew five surfaces that each held one fact twice:
an `Option` saying which thing was open and a `bool` beside it saying whether
one was, kept in step by a comment and broken by the next wave. The remedy
was the same every time — collapse the pair into one value and let a `match`
over it do what a rule was doing — and the kit carries that remedy as types,
because a type cannot be left out of a clearing list.

A `Chip<Id>` is the tap-to-explain word: which id's explanation is showing, or
none. Tapping the lit id puts it out; tapping another lights that one instead;
the same chip on two surfaces is one chip, so it cannot be left lit for a card
that has gone. The explanation is never stored — `line` derives it now from
the row the id names, so a renamed row renames every explanation on screen.
`toggle` is the same move for any `Option` field that opens on a tap: a board,
a selection, a list.

<!-- asserted-by: a_chips_line_is_read_off_the_row_at_the_moment_it_is_shown, a_chip_tapped_on_two_surfaces_is_one_chip -->
```rust
# #[derive(Clone, Copy, Debug, PartialEq, Eq)]
# enum TraitId { Caring }
# fn explain(_id: TraitId) -> String { "somebody else's trouble".to_owned() }
# let tapped = TraitId::Caring;
# let small = TextStyle::default();
# #[derive(Clone, Copy, Debug, PartialEq)]
# enum NoArt {}
# impl Icon for NoArt { fn size_at(self, _scale: f32) -> Vec2 { match self {} } }
# let mut sheet: Panel<NoArt> = Panel::default();
#[derive(Default)]
struct Flow {
    /// Which trait chip's explanation is showing — a trait, not a place.
    explained: Chip<TraitId>,
    /// Which site's board is open, if one is.
    board: Option<usize>,
}
let mut flow = Flow::default();
flow.explained.toggle(tapped);                   // a chip, tapped
toggle(&mut flow.board, 3);                      // a marker, tapped
// At draw time: gold while showing, and the line read off the row now.
let tone = if flow.explained.showing(tapped) { Color::MAGENTA } else { Color::WHITE };
if let Some(line) = flow.explained.line(explain) {
    sheet.text(TextRun::new(Vec2::new(10.0, 90.0), line, small));
}
```

The third carrier is a floor rather than a type: `judge_panel` refuses a
frame with two overlays' content in it, counted by the title row each one
draws. Hold the open drawer as one `Option<Drawer>` and the state is
unrepresentable; the floor says it anyway, so the next surface to grow an
open-flag of its own fails a check instead of a screenshot.

### Measured text

`ctx.text` does not wrap, and `\n` is the only line break there is. So a
generated sentence is wrapped by the game, to a column count the style
measures — `wrap(text, style.columns_in(width))` — and a row that may be too
long is **clipped**: `clipped(&style, text, width)` cuts at a word and marks
the cut with three dots, so a row that ran out of room never reads as a
rendering fault. The font's advance ratio appears nowhere; a game that wrote
`width / size` drew its line off the side of the world the day the face
changed.

Two idioms fold those into one call each. A `Cell` is a place for one row and
the width it may have, so a row's layout is stated as an offset from the row
and the clip comes for free; `Panel::hint` is a wrapped band of prose at a
declared width — the sentence every surface ends in. And `centered` is the one
baseline rule for a button's label: centred across the control, with the top
the game states once per control shape.

```rust
# let small = TextStyle { size: 12.0, ..TextStyle::default() };
# let faint = small;
# let name_of = |_who: usize| "Bartholomew the Unready";
# #[derive(Clone, Copy, Debug, PartialEq)]
# enum NoArt {}
# impl Icon for NoArt { fn size_at(self, _scale: f32) -> Vec2 { match self {} } }
# let mut panel: Panel<NoArt> = Panel::default();
/// A roster row's cells, as offsets from the row's own top-left.
const NAME: Cell = Cell { at: Vec2::new(36.0, 2.0), width: 120.0 };
let row = Vec2::new(10.0, 100.0);
panel.text(NAME.at(row).run(name_of(0), small));
let close = Rect::from_min_size(Vec2::new(900.0, 10.0), Vec2::new(32.0, 32.0));
panel.text(TextRun::new(centered(close, &small, "X", close.min.y + 10.0), "X", small));
panel.hint(Vec2::new(10.0, 400.0), 300.0, "tap a row for who, and why", faint, 2.0);
```

### The feed, and the table that is its behaviour

What happened in the world is a log the game owns. The feed is a **view** of
it: `feed` hands back indices into the log, newest first, filtered by the
player's attention config and bounded by a cap, derived on every call — so
there is no state in which the feed and the log could disagree.

The config is a table: one `ClassSpec` row per class of event, with the id a
stamp names it by, the colour and the picture its chip is drawn in, and the
`Mode` it opens on — ignore it, log it, or stop the world for it. Nothing
branches on a class. A screen asks the row what colour to draw, the scheduler
asks the `Attention` config what a class does to the clock, and a wave that
adds a class adds a row. **Hold the config in simulation state**, not on the
screen: a change to it is a recorded input, and a replay that did not carry it
would reproduce the orders and not the pauses. When a class stops the world
the game records a `Pause` — which entry, which class, which minute — and
`reason_line` is the one sentence the banner, the feed's header and a card's
note all print for it.

<!-- asserted-by: the_feed_is_the_log_filtered_by_the_config_newest_first, setting_one_mode_moves_one_mode_and_the_stamp_says_every_row -->
```rust
# #[derive(Clone, Copy, Debug, PartialEq, Eq)]
# enum Class { Departed, Refused }
# #[derive(Clone, Copy, Debug, PartialEq)]
# enum Art { Tower, Skull }
# struct Event { class: Class, place: &'static str, note: &'static str }
# let log = vec![Event { class: Class::Departed, place: "camp", note: "Bob left" }];
const CLASSES: &[ClassSpec<Class, Art>] = &[
    ClassSpec { class: Class::Departed, id: "departed", color: Color::WHITE, icon: Art::Tower, default_mode: Mode::Ignore },
    ClassSpec { class: Class::Refused, id: "refused", color: Color::MAGENTA, icon: Art::Skull, default_mode: Mode::PauseAndFocus },
];
let mut attention = Attention::opening(CLASSES);   // lives in the sim's world
attention.set(Class::Departed, Mode::Log);          // a recorded input
let rows: Vec<FeedEntry> = feed(log.iter().map(|event| event.class), &attention, false, 10);
for entry in &rows {
    let event = &log[entry.index];
    let spec = find_class(CLASSES, event.class).map_or("?", |spec| spec.id);
    // ... one row: the stamp, the chip in spec.color, the note, the place
}
let stopped = Pause { event: 0, class: Class::Refused, minute: 480 };
let banner = reason_line("refused", log[stopped.event].place, log[stopped.event].note);
assert!(banner.starts_with("paused: refused at camp"));
```

### Meters that open into faces

A meter is a count of people, and every count opens into the people it
counted, each with the reason they count — never a bare percentage. A
`MeterSpec` row is an id, a label, a picture and a question asked of one
person: `Some(reason)` when they count. `faces` is everybody on the roll the
question has a reason for; `count` is that list's length and nothing else, so
a chip the player cannot walk into does not exist.

The question is yours, and so is the claim that makes the chip trustworthy:
**the set a chip names is the set the simulation acts on.** Ask the chip, then
run the act — the upkeep burn, the deadline — and assert the people it touched
are the people the chip named, against the act and never against a second
reading of the same predicate. That is the test every chip ships with.

<!-- asserted-by: a_meters_set_is_the_set_the_simulation_acts_on, a_meters_count_is_the_length_of_the_list_it_opens_into -->
```rust
# #[derive(Clone, Copy, Debug, PartialEq)]
# enum Art { Coin }
# struct Camp { wallets: Vec<i64> }
# fn burn(_camp: &mut Camp) {}
# fn short(camp: &Camp, who: usize) -> Option<String> {
#     (camp.wallets[who] < 4).then(|| format!("holds {}g of the 4g due", camp.wallets[who]))
# }
/// The question a meter asks, over this game's own read-only view.
type Ask = fn(&Camp, usize) -> Option<String>;
const METERS: &[MeterSpec<Art, Ask>] = &[
    MeterSpec { id: "short", label: "short", icon: Art::Coin, asks: short },
];
let mut camp = Camp { wallets: vec![10, 2, 0] };
let roll = 0..camp.wallets.len();
let named = faces(roll.clone(), |who| (METERS[0].asks)(&camp, who));
assert_eq!(count(roll, |who| (METERS[0].asks)(&camp, who)), named.len());
burn(&mut camp);   // then assert: the people the burn went short on == named
```

### The floors

A floor is a readability rule stated as a number the game owns — `Floors` is
the smallest text, the design rect every piece of chrome lies inside, and the
world rect every map label lies inside — and asserted over the panel before
anything is drawn. `judge_panel` returns every breach: text below the floor,
chrome off the rect, a row lying across a control it is not the label of (you
pass the controls, named), two rows of chrome colliding **on one band** (two
bands are an overlay, and legitimate), two map labels colliding, an icon at a
fractional scale, a chrome icon off the rect, and two overlays in one frame.
Then `judge_frame` asks the other direction of a recorded frame — is every
row and icon the panel named actually there, through the mapping it was drawn
with — and `frame_text_floor` asks the frame itself for the smallest glyph.

Each breach is a stable sentence and the numbers it judged. Keep them in your
own accumulator and print them together: an instrument that halts at the
first bad reading costs a cycle per fault.

```rust
# #[derive(Clone, Copy, Debug, PartialEq)]
# enum Art { Coin }
# impl Icon for Art {
#     fn size_at(self, scale: f32) -> Vec2 { Vec2::splat(16.0 * scale) }
# }
# fn screen() -> Panel<Art> { Panel::default() }
# fn controls() -> Vec<(String, Rect)> { Vec::new() }
# let mut problems: Vec<String> = Vec::new();
const FLOORS: Floors = Floors {
    min_text: 12.0,
    chrome: Rect { min: Vec2::ZERO, max: Vec2::new(960.0, 540.0) },
    world: Rect { min: Vec2::ZERO, max: Vec2::new(4000.0, 4000.0) },
};
const DRAWERS: &[(&str, &str)] = &[("FEED", "FEED - what happened"), ("ROSTER", "ROSTER - everyone")];
for breach in judge_panel(&screen(), &FLOORS, &controls(), DRAWERS) {
    problems.push(format!("the glance: {} — {}", breach.what, breach.detail));
}
```

**A floor you have not seen bite is a floor you have not got.** For every
floor, stage the screen it was written for — the two rows that collided, the
drawer drawn over another — judge it, and assert the breach *by name*: a
count any other fault could satisfy proves nothing. The kit's own tests do
this for each floor; your `--verify` does it again from your layout, because
what proves a floor still bites from its new home is seeing it bite there.

<!-- asserted-by: two_rows_of_chrome_on_one_band_bite_and_on_two_bands_do_not, two_overlays_in_one_frame_bite -->
```rust
# #[derive(Clone, Copy, Debug, PartialEq)]
# enum Art { Coin }
# impl Icon for Art {
#     fn size_at(self, scale: f32) -> Vec2 { Vec2::splat(16.0 * scale) }
# }
# const FLOORS: Floors = Floors {
#     min_text: 12.0,
#     chrome: Rect { min: Vec2::ZERO, max: Vec2::new(960.0, 540.0) },
#     world: Rect { min: Vec2::ZERO, max: Vec2::new(4000.0, 4000.0) },
# };
# let small = TextStyle { size: 12.0, depth: Depth::layer(1), ..TextStyle::default() };
let mut staged: Panel<Art> = Panel::default();
staged.text(TextRun::new(Vec2::new(10.0, 350.0), "seed 0", small));
staged.text(TextRun::new(Vec2::new(10.0, 352.0), "point at a constant", small));
let bites = judge_panel(&staged, &FLOORS, &[], &[])
    .iter()
    .any(|breach| breach.what == "two rows of chrome text overlap");
assert!(bites, "the overlap floor does not fail on the screen it was written for");
```

### Two patterns the kit does not type

**A sorted row list has one ordering function and two readers.** The draw and
the hit-test both call the same `fn rows(..) -> Vec<..>`, and the check asserts
equality **per row, at the row's own cell**: the name drawn in row three is the
name a click on row three resolves to. A `Vec` is the type; the discipline is
that nothing else orders the list, and the sort key is yours.

**Selection and inspection are one `Option`.** One `selected: Option<usize>`
over the roster; every door that selects writes that field — a sprite, a chip,
a roster row, a candidate list — and the inspection panel is open if and only
if it is `Some`. The ring on the map, the hit-test and the figure all read one
position function, which stays yours because placing a figure is a layout
rule of your map. A second index over the same roster is the defect this
pattern exists to refuse: two rings, and nobody can tell who is picked.

### What is not here, and what brings it in

The kit's parts got in by being produced three or more times in one game, and
two parts that nearly qualified were left out with their trigger recorded, so
a reader looking for them knows why there is no entry.

- **A one-drawer type** — an `Overlay` trait with `all()`, a handle rect and
  a title, and the one `Option` over it that the render, the click routing and
  the control set each `match` on. The shape exists in one game; it is
  extracted when a **second instance** exists, so the kit generalises from
  two rather than from one. Until then, hold your drawers as one `Option` and
  let the overlay floor above catch the day that stops being true.
- **A timer bar against a world-time deadline** — a `0..=1` share of a span,
  with an ember inside its last day. Two call sites, below the rule; it comes
  in with the next wave that adds a **deadline**.

The rule itself is `docs/conventions.md` §Extraction: three or more, proposed
by an audit's report, decided by the owner. A part a game keeps re-typing is
evidence; write it down where the next audit will read it.
