//! A screen built as data and judged twice — against the floors it states and
//! against the frame it was drawn into — with a chip, a feed and a row of
//! meters over a toy camp. The UI kit (`jidousha::ui`), whole, in one headless
//! run.
//!
//! Run it: `cargo run -p jidousha --example ui_kit`

use jidousha::prelude::*;
use jidousha::testing::FrameRecorder;
use jidousha::ui::{
    Attention, Breach, Cell, Chip, ClassSpec, FeedEntry, Floors, Icon, IconRun, Mapping, MeterSpec,
    Mode, Panel, Pause, TextRun, centered, class_faults, clipped, count, faces, feed, find_class,
    frame_text_floor, glyph_run, inside, judge_frame, judge_panel, meter_faults, reason_line,
    toggle, wrap,
};

// --- the game's own vocabulary ----------------------------------------------

/// The art library's roles. The kit never names a texture; this does.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Art {
    Coin,
    Flame,
    Face,
}

impl Icon for Art {
    fn size_at(self, scale: f32) -> Vec2 {
        Vec2::splat(16.0 * scale)
    }
}

/// What can happen in the camp.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Departed,
    Refused,
}

/// The class table: id, chip colour, chip picture, and the mode it opens on.
const CLASSES: &[ClassSpec<Class, Art>] = &[
    ClassSpec {
        class: Class::Departed,
        id: "departed",
        color: DIM,
        icon: Art::Face,
        default_mode: Mode::Ignore,
    },
    ClassSpec {
        class: Class::Refused,
        id: "refused",
        color: EMBER,
        icon: Art::Flame,
        default_mode: Mode::PauseAndFocus,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TraitId {
    Caring,
    Restless,
}

/// A trait's explanation, read off its row at the moment it is shown.
fn explain(id: TraitId) -> String {
    match id {
        TraitId::Caring => "caring - moved by somebody else's trouble".to_owned(),
        TraitId::Restless => "restless - wanting to be elsewhere".to_owned(),
    }
}

/// One thing that happened: when, what, to whom, and in what words.
struct Event {
    minute: u64,
    class: Class,
    who: usize,
    note: &'static str,
}

/// The world, as one resource: four people, their purses, and what happened.
struct Camp {
    names: Vec<&'static str>,
    wallets: Vec<i64>,
    traits: Vec<TraitId>,
    log: Vec<Event>,
    attention: Attention<Class, Art>,
    paused: Option<Pause<Class>>,
}
impl Resource for Camp {}

const DUE: i64 = 4;

/// The question a meter asks of one person, over this camp.
type Ask = fn(&Camp, usize) -> Option<String>;

fn short(camp: &Camp, who: usize) -> Option<String> {
    (camp.wallets[who] < DUE).then(|| format!("holds {}g of the {DUE}g due", camp.wallets[who]))
}

fn flush(camp: &Camp, who: usize) -> Option<String> {
    (camp.wallets[who] >= DUE).then(|| format!("holds {}g", camp.wallets[who]))
}

const METERS: &[MeterSpec<Art, Ask>] = &[
    MeterSpec {
        id: "short",
        label: "short",
        icon: Art::Coin,
        asks: short,
    },
    MeterSpec {
        id: "flush",
        label: "flush",
        icon: Art::Flame,
        asks: flush,
    },
];

/// The UI's state — none of it simulation state, and every field one value.
#[derive(Clone, Debug, Default)]
struct Flow {
    /// Which trait chip's explanation is showing, if one is.
    explained: Chip<TraitId>,
    /// The selected person — the one selection this screen has.
    selected: Option<usize>,
    /// Which meter chip has been drilled into, if one has.
    drilled: Chip<usize>,
}
impl Resource for Flow {}

// --- the layout, in a 960x540 design space ----------------------------------

const DESIGN: Vec2 = Vec2::new(960.0, 540.0);
const DIM: Color = Color::rgb(0.55, 0.52, 0.63);
const INK: Color = Color::rgb(0.91, 0.87, 0.77);
const GOLD: Color = Color::rgb(0.88, 0.70, 0.29);
const EMBER: Color = Color::rgb(0.83, 0.33, 0.23);
const TEXT_BAND: i16 = 1;
const PIECE_BAND: i16 = -1;
const MAP_BAND: i16 = -5;

const FLOORS: Floors = Floors {
    min_text: 12.0,
    chrome: Rect {
        min: Vec2::ZERO,
        max: DESIGN,
    },
    world: Rect {
        min: Vec2::ZERO,
        max: Vec2::new(4000.0, 4000.0),
    },
};

fn small(color: Color) -> TextStyle {
    TextStyle {
        size: 12.0,
        color,
        depth: Depth::layer(TEXT_BAND),
        ..TextStyle::default()
    }
}

fn meter_chip(index: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(10.0 + index as f32 * 110.0, 40.0),
        Vec2::new(100.0, 32.0),
    )
}

fn face_row(row: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(10.0, 90.0 + row as f32 * 36.0),
        Vec2::new(300.0, 32.0),
    )
}

/// A face row's cells, as offsets from the row.
const FACE_NAME: Cell = Cell {
    at: Vec2::new(36.0, 2.0),
    width: 120.0,
};
const FACE_REASON: Cell = Cell {
    at: Vec2::new(36.0, 17.0),
    width: 260.0,
};

fn sheet_chip() -> Rect {
    Rect::from_min_size(Vec2::new(620.0, 60.0), Vec2::new(120.0, 32.0))
}

fn close_button() -> Rect {
    Rect::from_min_size(Vec2::new(900.0, 40.0), Vec2::new(40.0, 32.0))
}

/// Every rectangle a click does something in, with the name a message uses.
fn controls() -> Vec<(String, Rect)> {
    let mut out = vec![("the close".to_owned(), close_button())];
    for index in 0..METERS.len() {
        out.push((format!("meter chip {index}"), meter_chip(index)));
    }
    out.push(("the trait chip".to_owned(), sheet_chip()));
    out
}

/// The design rect fitted inside the view, centred — the chrome rides the camera.
struct UiMap {
    origin: Vec2,
    scale: f32,
}

impl UiMap {
    fn for_camera(camera: &Camera) -> Self {
        let view = camera.visible_bounds();
        let scale = (view.size().x / DESIGN.x).min(view.size().y / DESIGN.y);
        Self {
            origin: view.center() - DESIGN * (scale * 0.5),
            scale,
        }
    }
}

impl Mapping for UiMap {
    fn to_world(&self, ui: Vec2) -> Vec2 {
        self.origin + ui * self.scale
    }
    fn scale(&self) -> f32 {
        self.scale
    }
}

// --- the screen, as data -----------------------------------------------------

/// Everything the glance draws: the meter chips, the faces a drilled chip
/// opens, the banner when the world is stopped, the feed, the selected
/// person's sheet, and a map label.
fn glance(camp: &Camp, flow: &Flow) -> Panel<Art> {
    let mut panel = Panel::default();
    let roll = 0..camp.names.len();
    for (index, spec) in METERS.iter().enumerate() {
        let chip = meter_chip(index);
        let counted = count(roll.clone(), |who| (spec.asks)(camp, who));
        let tone = if flow.drilled.showing(index) {
            GOLD
        } else {
            DIM
        };
        let mut icon = IconRun::new(chip.min + Vec2::splat(8.0), spec.icon, 1.0, PIECE_BAND);
        icon.tint = tone;
        panel.icon(icon);
        panel.text(TextRun::new(
            chip.min + Vec2::new(30.0, 10.0),
            format!("{} {counted}", spec.label),
            small(tone),
        ));
    }
    if let Some(index) = flow.drilled.lit {
        let spec = &METERS[index];
        for (row, (who, reason)) in faces(roll.clone(), |who| (spec.asks)(camp, who))
            .into_iter()
            .enumerate()
        {
            let at = face_row(row).min;
            panel.icon(IconRun::new(at, Art::Face, 2.0, PIECE_BAND));
            panel.text(FACE_NAME.at(at).run(camp.names[who], small(INK)));
            panel.text(FACE_REASON.at(at).run(&reason, small(DIM)));
        }
    }
    if let Some(pause) = camp.paused {
        let event = &camp.log[pause.event];
        let class = find_class(CLASSES, pause.class).map_or("?", |spec| spec.id);
        let line = reason_line(class, "the camp", event.note);
        panel.text(TextRun::new(
            Vec2::new(10.0, 12.0),
            clipped(&small(GOLD), &line, 600.0),
            small(GOLD),
        ));
    }
    let rows: Vec<FeedEntry> = feed(
        camp.log.iter().map(|event| event.class),
        &camp.attention,
        false,
        5,
    );
    for (row, entry) in rows.iter().enumerate() {
        let event = &camp.log[entry.index];
        let spec = find_class(CLASSES, event.class).map_or(&CLASSES[0], |spec| spec);
        let at = Vec2::new(340.0, 100.0 + row as f32 * 16.0);
        panel.text(TextRun::new(at, format!("{:>4}", event.minute), small(DIM)));
        panel.text(TextRun::new(
            at + Vec2::new(44.0, 0.0),
            spec.id,
            small(spec.color),
        ));
        panel.text(TextRun::new(
            at + Vec2::new(120.0, 0.0),
            clipped(&small(INK), event.note, 180.0),
            small(INK),
        ));
    }
    if let Some(who) = flow.selected {
        let chip = sheet_chip();
        let id = camp.traits[who];
        let tone = if flow.explained.showing(id) {
            GOLD
        } else {
            INK
        };
        let label = match id {
            TraitId::Caring => "caring",
            TraitId::Restless => "restless",
        };
        panel.text(TextRun::new(
            centered(chip, &small(tone), label, chip.min.y + 10.0),
            label,
            small(tone),
        ));
        if let Some(line) = flow.explained.line(explain) {
            panel.hint(Vec2::new(620.0, 100.0), 300.0, &line, small(GOLD), 2.0);
        }
    }
    let close = close_button();
    panel.text(TextRun::new(
        centered(close, &small(INK), "X", close.min.y + 10.0),
        "X",
        small(INK),
    ));
    let mut label = small(DIM);
    label.depth = Depth::layer(MAP_BAND);
    panel.world_text(TextRun::new(
        Vec2::new(900.0, 700.0),
        "the Deep Cave",
        label,
    ));
    panel
}

fn draw_screen(ctx: &mut DrawCtx) {
    let map = UiMap::for_camera(ctx.world.resource::<Camera>());
    let handle = ctx.world.resource::<Handle>().0;
    let panel = glance(ctx.world.resource::<Camp>(), ctx.world.resource::<Flow>());
    panel.draw(ctx, &map, |icon, scale| Sprite {
        size: icon.art.size_at(scale),
        anchor: Vec2::new(-0.5, -0.5),
        tint: icon.tint,
        layer: icon.layer,
        ..Sprite::new(handle)
    });
}

/// The one texture every role draws from here; a real game has a library.
#[derive(Clone, Copy)]
struct Handle(TextureHandle);
impl Resource for Handle {}

fn problems(what: &str, breaches: &[Breach]) -> Vec<String> {
    breaches
        .iter()
        .map(|breach| format!("{what}: {} — {}", breach.what, breach.detail))
        .collect()
}

fn main() {
    // --- the tables, before any world exists -------------------------------
    let faults = class_faults(CLASSES, Color::rgb(0.08, 0.07, 0.1));
    assert!(faults.is_empty(), "{faults:?}");
    let faults = meter_faults(METERS);
    assert!(faults.is_empty(), "{faults:?}");

    // --- a camp, stopped by a refusal -------------------------------------
    let mut camp = Camp {
        names: vec!["Alex", "Bob", "Rin", "Goro"],
        wallets: vec![10, 2, 0, 7],
        traits: vec![
            TraitId::Caring,
            TraitId::Restless,
            TraitId::Caring,
            TraitId::Restless,
        ],
        log: vec![
            Event {
                minute: 0,
                class: Class::Departed,
                who: 0,
                note: "Alex left for the cave",
            },
            Event {
                minute: 480,
                class: Class::Refused,
                who: 1,
                note: "Bob would rather not",
            },
        ],
        attention: Attention::opening(CLASSES),
        paused: None,
    };
    // The scheduler's rule: a class the config stops the world for records a
    // pause on the world itself.
    for (index, event) in camp.log.iter().enumerate() {
        if camp.attention.mode(event.class) == Mode::PauseAndFocus && camp.paused.is_none() {
            camp.paused = Some(Pause {
                event: index,
                class: event.class,
                minute: event.minute,
            });
        }
    }
    assert_eq!(camp.paused.map(|pause| pause.event), Some(1));
    assert_eq!(camp.log[1].who, 1);

    // The feed is a view: its rows are the log filtered by the config.
    let shown = feed(
        camp.log.iter().map(|event| event.class),
        &camp.attention,
        false,
        10,
    );
    assert_eq!(
        shown,
        vec![FeedEntry {
            index: 1,
            ignored: false
        }]
    );
    camp.attention.set(Class::Departed, Mode::Log);
    let shown = feed(
        camp.log.iter().map(|event| event.class),
        &camp.attention,
        false,
        10,
    );
    assert_eq!(shown.len(), 2, "logged now, so it is in the feed");
    assert_eq!(
        camp.attention.stamp(),
        "attention:departed=log,refused=pause"
    );

    // A chip's set is the set the simulation acts on: ask, then act, then compare.
    let roll = 0..camp.names.len();
    let named: Vec<usize> = faces(roll.clone(), |who| short(&camp, who))
        .into_iter()
        .map(|(who, _)| who)
        .collect();
    // The act: charge everybody who can pay, and leave short whoever cannot.
    let before = camp.wallets.clone();
    for wallet in &mut camp.wallets {
        if *wallet >= DUE {
            *wallet -= DUE;
        }
    }
    let acted: Vec<usize> = roll
        .clone()
        .filter(|who| camp.wallets[*who] == before[*who])
        .collect();
    assert_eq!(
        acted, named,
        "the chip named a different set from the one the burn skipped"
    );
    camp.wallets = before;

    // --- the player taps things: every field one value ---------------------
    let mut flow = Flow::default();
    toggle(&mut flow.selected, 1);
    flow.explained.toggle(TraitId::Restless);
    flow.drilled.toggle(0);
    assert!(flow.drilled.showing(0));
    flow.drilled.toggle(1);
    assert!(!flow.drilled.showing(0), "another chip tapped replaces it");
    flow.drilled.toggle(0);

    // --- the screen, judged before it is drawn -----------------------------
    let panel = glance(&camp, &flow);
    let judged = problems(
        "the glance",
        &judge_panel(&panel, &FLOORS, &controls(), &[]),
    );
    assert!(judged.is_empty(), "{judged:#?}");
    assert!(panel.all_strings().all(|text| text.is_ascii()));
    assert!(inside(FLOORS.chrome, panel.runs[0].bounds()));
    let wrapped = wrap(&explain(TraitId::Restless), small(GOLD).columns_in(300.0));
    assert_eq!(
        panel
            .runs
            .iter()
            .filter(|run| wrapped.lines().any(|l| l == run.text))
            .count(),
        wrapped.lines().count(),
        "the hint block is the wrapped line, row for row"
    );

    // --- and a staged violation, so the floor is seen to bite --------------
    let mut staged = panel.clone();
    staged.text(TextRun::new(Vec2::new(10.0, 60.0), "seed 0", small(INK)));
    staged.text(TextRun::new(
        Vec2::new(12.0, 62.0),
        "point at a constant",
        small(INK),
    ));
    assert!(
        judge_panel(&staged, &FLOORS, &[], &[])
            .iter()
            .any(|breach| breach.what == "two rows of chrome text overlap"),
        "the overlap floor does not fail on the screen it was written for"
    );

    // --- drawn through the mapping, and found on the frame -----------------
    let mut assets = Assets::new(MemorySource::new());
    let handle = Handle(assets.load_texture("roles.png"));
    let mut sim = headless(GameConfig::default(), |app| {
        app.add_system(Draw, draw_screen);
    });
    const VIEWPORT: PhysicalSize = PhysicalSize::new(1920, 1080);
    let camera = Camera {
        center: Vec2::new(600.0, 400.0),
        height: 1080.0,
        viewport: VIEWPORT,
        ..Camera::default()
    };
    sim.world_mut().insert_resource(camera);
    sim.world_mut().insert_resource(handle);
    sim.world_mut().insert_resource(camp);
    sim.world_mut().insert_resource(flow);
    let mut recorder = FrameRecorder::new(VIEWPORT);
    let frame = recorder.draw(&mut sim);
    let map = UiMap::for_camera(&camera);
    assert_eq!(
        map.scale(),
        2.0,
        "the design rect fits the view at twice its size"
    );
    let judged = problems(
        "the frame",
        &judge_frame(
            &panel,
            &frame,
            recorder.font_texture(),
            &map,
            camera.visible_bounds(),
        ),
    );
    assert!(judged.is_empty(), "{judged:#?}");
    // The frame floor is stated in world units: the chrome is drawn at twice
    // its design size here, and the map label at its own twelve.
    let judged = problems(
        "the frame",
        &frame_text_floor(&frame, recorder.font_texture(), 12.0),
    );
    assert!(judged.is_empty(), "{judged:#?}");
    assert!(
        !frame_text_floor(&frame, recorder.font_texture(), 13.0).is_empty(),
        "a floor above the map label's size is seen to bite on the frame"
    );
    let close = &panel.runs[panel.runs.len() - 1];
    let drawn = glyph_run(
        &frame,
        recorder.font_texture(),
        map.to_world(close.at),
        close.style.size * map.scale(),
        close.style.width_of(&close.text) * map.scale(),
    );
    assert_eq!(
        drawn, 1,
        "the close button's X is one glyph where the panel said"
    );

    println!(
        "ui_kit: {} rows and {} icons judged clean, found on a {}x{} frame of {} quads; the \
         overlap floor bites when staged",
        panel.runs.len(),
        panel.icons.len(),
        VIEWPORT.width,
        VIEWPORT.height,
        frame.quads().len()
    );
}
