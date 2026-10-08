//! Every screen, as a `Panel` built from the run — drawn, judged and
//! photographed from the one list.
//!
//! Laid out in a 960x540 design space that the camera shows whole (the
//! camera is 540 world units tall, centred on the design rect, so at the
//! window's 16:9 the mapping is the identity). The numbered rows a player can
//! choose are rectangles here too, and the click routing reads the same ones.

use jidousha::prelude::*;
use jidousha::ui::*;

use crate::lore::Being;
use crate::play::{Ending, Fate, Run, Screen};
use crate::rules::{self, DAYS, SANITY_START, answer_outcome, effect_of, text_at};

/// The window the game opens at, and the shape every layout number assumes.
pub(crate) const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);
/// The design rect's size, in design units.
pub(crate) const DESIGN: Vec2 = Vec2::new(960.0, 540.0);
/// The left margin every row starts at.
const LEFT: f32 = 32.0;
/// Where a numbered row's text starts, after its number.
const INDENT: f32 = 64.0;
/// The widest any row of prose may run.
const PROSE: f32 = 880.0;

/// The picture vocabulary: none — this game is text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Art {}

impl Icon for Art {
    fn size_at(self, _scale: f32) -> Vec2 {
        match self {}
    }
}

/// The game's colours.
pub(crate) mod palette {
    use jidousha::prelude::Color;
    /// The room the telephone is in.
    pub(crate) const NIGHT: Color = Color::rgb(0.03, 0.05, 0.06);
    pub(crate) const INK: Color = Color::rgb(0.86, 0.88, 0.80);
    pub(crate) const FAINT: Color = Color::rgb(0.55, 0.60, 0.55);
    pub(crate) const SICKLY: Color = Color::rgb(0.55, 0.85, 0.55);
    pub(crate) const BLOOD: Color = Color::rgb(0.95, 0.35, 0.30);
    pub(crate) const GOLD: Color = Color::rgb(0.95, 0.80, 0.40);
}

/// The draw band every row of chrome is on.
const UI_LAYER: i16 = 1;

fn style(size: f32, color: Color) -> TextStyle {
    TextStyle {
        size,
        color,
        depth: Depth::layer(UI_LAYER),
        ..TextStyle::default()
    }
}

/// The smallest text the game draws, and the floor `verify` holds it to.
pub(crate) const SMALL: f32 = 14.0;
const BODY: f32 = 16.0;
const LARGE: f32 = 20.0;
const TITLE: f32 = 24.0;

/// The design rect fitted inside what the camera shows, centred.
pub(crate) struct UiMap {
    pub(crate) origin: Vec2,
    pub(crate) scale: f32,
}

impl UiMap {
    pub(crate) fn for_camera(camera: &Camera) -> Self {
        let view = camera.visible_bounds();
        let scale = (view.size().x / DESIGN.x).min(view.size().y / DESIGN.y);
        Self {
            origin: view.center() - DESIGN * (scale * 0.5),
            scale,
        }
    }

    /// A world point, in design units.
    pub(crate) fn to_design(&self, world: Vec2) -> Vec2 {
        (world - self.origin) / self.scale
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

/// The game's camera: the design rect, whole.
pub(crate) fn camera() -> Camera {
    Camera {
        center: DESIGN * 0.5,
        height: DESIGN.y,
        clear_color: palette::NIGHT,
        viewport: WINDOW,
    }
}

/// One screen: what it draws, and the rows a tap can choose.
pub(crate) struct Layout {
    pub(crate) panel: Panel<Art>,
    /// The numbered rows, in number order, in design units.
    pub(crate) choices: Vec<Rect>,
}

/// The screen the run is on.
pub(crate) fn layout(run: &Run) -> Layout {
    let mut out = Layout {
        panel: Panel::default(),
        choices: Vec::new(),
    };
    match run.screen {
        Screen::Morning => morning(run, &mut out),
        Screen::Calling(_) => calling(run, &mut out),
        Screen::CallOver { call, ending, last } => {
            header(
                run,
                &mut out.panel,
                &format!(
                    "EVENING - day {} - call {} of {}",
                    run.state.day,
                    call.slot + 1,
                    run.night.calls.len()
                ),
            );
            let spec = call.being.spec();
            let (said, tone) = match ending {
                Ending::HungUp => (
                    format!("{} has said its piece. You hang up.", spec.name),
                    palette::SICKLY,
                ),
                Ending::Wrath => (
                    format!(
                        "WRATH. {} hangs up on you, and something follows you home (-{} sanity).",
                        spec.name,
                        rules::WRATH
                    ),
                    palette::BLOOD,
                ),
            };
            let mut y = out.panel.hint(
                Vec2::new(LEFT, 120.0),
                PROSE,
                &said,
                style(LARGE, tone),
                4.0,
            );
            y += 12.0;
            let cost = format!(
                "The call cost you {} sanity over {} exchanges (the last: {}).",
                call.cost, call.asked, last.sanity_cost
            );
            y = out.panel.hint(
                Vec2::new(LEFT, y),
                PROSE,
                &cost,
                style(BODY, palette::INK),
                4.0,
            );
            out.panel.text(TextRun::new(
                Vec2::new(LEFT, y + 40.0),
                "Space, Enter or a tap to go on.",
                style(BODY, palette::FAINT),
            ));
        }
        Screen::End(fate) => ending(run, fate, &mut out),
    }
    out
}

/// The title row and the sanity readout every screen starts with.
fn header(run: &Run, panel: &mut Panel<Art>, title: &str) {
    panel.text(TextRun::new(
        Vec2::new(LEFT, 16.0),
        title,
        style(TITLE, palette::GOLD),
    ));
    let sanity = sanity_line(run);
    let small = style(
        LARGE,
        if run.sanity <= 30 {
            palette::BLOOD
        } else {
            palette::INK
        },
    );
    panel.text(TextRun::new(
        Vec2::new(DESIGN.x - LEFT - small.width_of(&sanity), 18.0),
        sanity,
        small,
    ));
}

/// The sanity readout, as every screen prints it.
pub(crate) fn sanity_line(run: &Run) -> String {
    format!("sanity {}/{}", run.sanity, SANITY_START)
}

/// Tonight's callers, in order, as the morning names them.
pub(crate) fn callers_line(run: &Run) -> String {
    let names: Vec<&str> = run
        .night
        .callers()
        .into_iter()
        .map(|being| being.spec().name)
        .collect();
    format!("Tonight: {} will call.", names.join(", then "))
}

/// An option's effect on tonight, as the morning prints it under the option.
pub(crate) fn effect_line(run: &Run, row: usize) -> String {
    let Some(action) = run.options().get(row).copied() else {
        return String::new();
    };
    let changes = effect_of(&run.state, &rules::apply(&run.state, action));
    if changes.is_empty() {
        return "tonight: no change to the calls (the gain keeps for later nights)".to_owned();
    }
    let lines: Vec<String> = changes.iter().map(rules::Change::line).collect();
    format!("tonight: {}", lines.join("; "))
}

fn morning(run: &Run, out: &mut Layout) {
    header(
        run,
        &mut out.panel,
        &format!("MORNING - day {} of {DAYS}", run.state.day),
    );
    out.panel.text(TextRun::new(
        Vec2::new(LEFT, 56.0),
        callers_line(run),
        style(BODY, palette::INK),
    ));
    let known: Vec<String> = Being::ALL
        .iter()
        .map(|being| format!("{} {}/3", being.spec().name, run.state.known[being.index()]))
        .collect();
    let lore = format!(
        "lore: {}   composure {}",
        known.join("  "),
        run.state.composure
    );
    out.panel.text(TextRun::new(
        Vec2::new(LEFT, 80.0),
        lore,
        style(SMALL, palette::FAINT),
    ));
    let mut y = 112.0;
    for (row, action) in run.options().into_iter().enumerate() {
        let top = y;
        let name = format!("{}  {}", row + 1, action.name(&run.state));
        out.panel.text(TextRun::new(
            Vec2::new(LEFT, y),
            name,
            style(BODY, palette::INK),
        ));
        y = out.panel.hint(
            Vec2::new(INDENT, y + BODY + 2.0),
            PROSE - (INDENT - LEFT),
            &effect_line(run, row),
            style(SMALL, palette::SICKLY),
            2.0,
        );
        out.choices.push(Rect::from_min_size(
            Vec2::new(LEFT, top),
            Vec2::new(PROSE, y - top),
        ));
        y += 6.0;
    }
    out.panel.text(TextRun::new(
        Vec2::new(LEFT, DESIGN.y - 32.0),
        "Keys 1-6, or tap an option.",
        style(SMALL, palette::FAINT),
    ));
}

/// What the player knows of `being`, one entry per learned fact.
pub(crate) fn known_lore(run: &Run, being: Being) -> Vec<String> {
    let known = run.state.known[being.index()];
    being.spec().facts[..known]
        .iter()
        .map(|fact| format!("- {}: {}", fact.topic, fact.lore))
        .collect()
}

/// The hint beside an answer: its outcome if the lore is known, else unknown.
pub(crate) fn hint_line(run: &Run, row: usize) -> String {
    let Screen::Calling(call) = run.screen else {
        return String::new();
    };
    let id = run.night.calls[call.slot].question(call.asked);
    if !run.state.knows(id) {
        return "outcome unknown - you do not know this lore".to_owned();
    }
    let outcome = answer_outcome(id, row, call.anger, run.state.composure);
    let verdict = match (outcome.wrath, outcome.anger_change, outcome.line_change) {
        (true, _, _) => "WRATH",
        (false, 0, -2) => "ends sooner",
        (false, 0, _) => "drags on",
        _ => "angers it",
    };
    format!(
        "{verdict}: line {:+}, temper {:+}, sanity -{}",
        outcome.line_change, outcome.anger_change, outcome.sanity_cost
    )
}

/// The question on the line, as the call screen prints it.
pub(crate) fn question_line(run: &Run) -> String {
    let Screen::Calling(call) = run.screen else {
        return String::new();
    };
    let id = run.night.calls[call.slot].question(call.asked);
    format!("\"{}\"", id.question().ask)
}

/// The caller's state, as the call screen prints it.
pub(crate) fn temper_line(run: &Run) -> String {
    let Screen::Calling(call) = run.screen else {
        return String::new();
    };
    format!(
        "temper {}/{}   line {} to go   drain {} an exchange",
        call.anger,
        call.being.spec().temper,
        call.line,
        run.night.calls[call.slot].drain
    )
}

fn calling(run: &Run, out: &mut Layout) {
    let Screen::Calling(call) = run.screen else {
        return;
    };
    header(
        run,
        &mut out.panel,
        &format!(
            "EVENING - day {} - call {} of {}",
            run.state.day,
            call.slot + 1,
            run.night.calls.len()
        ),
    );
    let spec = call.being.spec();
    out.panel.text(TextRun::new(
        Vec2::new(LEFT, 52.0),
        format!("{}, {}, is on the line.", spec.name, spec.title),
        style(LARGE, palette::SICKLY),
    ));
    out.panel.text(TextRun::new(
        Vec2::new(LEFT, 78.0),
        temper_line(run),
        style(SMALL, palette::INK),
    ));
    let mut y = out.panel.hint(
        Vec2::new(LEFT, 106.0),
        PROSE,
        &question_line(run),
        style(LARGE, palette::GOLD),
        4.0,
    );
    y += 8.0;
    let id = run.night.calls[call.slot].question(call.asked);
    for row in 0..3 {
        let top = y;
        let text = format!("{}  {}", row + 1, text_at(id, row));
        out.panel.text(TextRun::new(
            Vec2::new(LEFT, y),
            text,
            style(BODY, palette::INK),
        ));
        let tone = if run.state.knows(id) {
            palette::SICKLY
        } else {
            palette::FAINT
        };
        out.panel.text(TextRun::new(
            Vec2::new(INDENT, y + BODY + 2.0),
            hint_line(run, row),
            style(SMALL, tone),
        ));
        y += BODY + 2.0 + SMALL;
        out.choices.push(Rect::from_min_size(
            Vec2::new(LEFT, top),
            Vec2::new(PROSE, y - top),
        ));
        y += 8.0;
    }
    y += 10.0;
    out.panel.text(TextRun::new(
        Vec2::new(LEFT, y),
        format!("What you know of {}:", spec.name),
        style(BODY, palette::INK),
    ));
    y += BODY + 4.0;
    let lore = known_lore(run, call.being);
    if lore.is_empty() {
        out.panel.text(TextRun::new(
            Vec2::new(LEFT, y),
            "nothing yet - study it some morning",
            style(SMALL, palette::FAINT),
        ));
    }
    for entry in &lore {
        y = out.panel.hint(
            Vec2::new(LEFT, y),
            PROSE,
            entry,
            style(SMALL, palette::FAINT),
            2.0,
        ) + 2.0;
    }
    out.panel.text(TextRun::new(
        Vec2::new(LEFT, DESIGN.y - 32.0),
        "Keys 1-3, or tap an answer.",
        style(SMALL, palette::FAINT),
    ));
}

/// The end screen's headline for `fate`.
pub(crate) fn fate_line(fate: Fate) -> &'static str {
    match fate {
        Fate::Won { .. } if fate.better() => "DAWN. You sleep without dreams.",
        Fate::Won { .. } => "DAWN. You survived the fifth night.",
        Fate::Lost { .. } => "THE RECEIVER IS STILL WARM.",
    }
}

fn ending(run: &Run, fate: Fate, out: &mut Layout) {
    header(run, &mut out.panel, "THE LAST CALL");
    let tone = if matches!(fate, Fate::Lost { .. }) {
        palette::BLOOD
    } else {
        palette::GOLD
    };
    out.panel.text(TextRun::new(
        Vec2::new(LEFT, 140.0),
        fate_line(fate),
        style(TITLE, tone),
    ));
    let story = match fate {
        Fate::Lost { day } => format!(
            "Your sanity ran out on the night of day {day}. Nobody answers the phone at your house now."
        ),
        Fate::Won { sanity } => format!(
            "Five nights, and {sanity} sanity left. The phone is silent until the stars come right."
        ),
    };
    let y = out.panel.hint(
        Vec2::new(LEFT, 190.0),
        PROSE,
        &story,
        style(BODY, palette::INK),
        4.0,
    );
    out.panel.text(TextRun::new(
        Vec2::new(LEFT, y + 40.0),
        "Space, Enter or a tap to begin again.",
        style(BODY, palette::FAINT),
    ));
}
