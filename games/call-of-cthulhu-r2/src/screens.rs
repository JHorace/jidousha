//! Every screen as data: one `Panel` per phase, and the rows a tap can choose.
//!
//! Three readers take the same panel — the draw system, the floors and the
//! frame check — so what is drawn, what is judged and what is found on the
//! recorded frame are one list. The tappable rows come out of `choices`, which
//! the draw and the hit-test both call, so the row a tap lands on is the row
//! that was drawn there.
//!
//! The design space is the world: the camera shows 960x540 world units from
//! the origin, so a design unit is a world unit and the mapping is flat.

use jidousha::prelude::*;
use jidousha::ui::{Cell, Floors, Icon, Mapping, Panel, TextRun, wrap};

use crate::play::{CallState, Game, Hangup, Phase, callers_line, ending_lines};
use crate::rules::{self, MorningAction};

/// The window, and the design rect the screens are laid out in.
pub const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);
pub const DESIGN_H: f32 = 540.0;
pub const DESIGN_W: f32 = DESIGN_H * WINDOW.aspect();
/// The left and right margin every row keeps.
pub const MARGIN: f32 = 40.0;

/// The game draws no pictures; the kit wants a picture vocabulary anyway.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Art {}
impl Icon for Art {
    fn size_at(self, _scale: f32) -> Vec2 {
        match self {}
    }
}

/// A design unit is a world unit.
pub struct Flat;
impl Mapping for Flat {
    fn to_world(&self, ui: Vec2) -> Vec2 {
        ui
    }
    fn scale(&self) -> f32 {
        1.0
    }
}

/// Draw bands.
pub mod layers {
    /// The sanity bar.
    pub const BAR: i16 = 0;
    /// Every row of text.
    pub const TEXT: i16 = 1;
}

/// The readability floors, in design units.
pub const FLOORS: Floors = Floors {
    min_text: 14.0,
    chrome: Rect {
        min: Vec2::ZERO,
        max: Vec2::new(DESIGN_W, DESIGN_H),
    },
    world: Rect {
        min: Vec2::ZERO,
        max: Vec2::new(DESIGN_W, DESIGN_H),
    },
};

/// The colours.
pub mod palette {
    use jidousha::prelude::Color;
    /// The background: the dark of a room at night.
    pub const NIGHT: Color = Color::rgb(0.03, 0.05, 0.06);
    pub const INK: Color = Color::rgb(0.86, 0.9, 0.84);
    pub const FAINT: Color = Color::rgb(0.55, 0.6, 0.58);
    pub const VOICE: Color = Color::rgb(0.55, 0.95, 0.75);
    pub const LORE: Color = Color::rgb(0.95, 0.85, 0.45);
    pub const DANGER: Color = Color::rgb(1.0, 0.45, 0.4);
    pub const BAR_EMPTY: Color = Color::rgb(0.18, 0.2, 0.22);
    pub const BAR_FULL: Color = Color::rgb(0.35, 0.75, 0.6);
}

fn style(size: f32, color: Color) -> TextStyle {
    TextStyle {
        size,
        color,
        depth: Depth::layer(layers::TEXT),
        ..TextStyle::default()
    }
}

pub fn title() -> TextStyle {
    style(26.0, palette::INK)
}
pub fn body() -> TextStyle {
    style(17.0, palette::INK)
}
pub fn small() -> TextStyle {
    style(15.0, palette::FAINT)
}

/// The sanity bar: where it is, and how much of it is full.
pub fn sanity_bar(sanity: i32) -> (Rect, Rect) {
    let whole = Rect::from_min_size(
        Vec2::new(MARGIN, 62.0),
        Vec2::new(DESIGN_W - 2.0 * MARGIN, 10.0),
    );
    let share = (sanity.max(0) as f32 / rules::START_SANITY as f32).min(1.0);
    let full = Rect::from_min_size(whole.min, Vec2::new(whole.size().x * share, 10.0));
    (whole, full)
}

/// What a tappable row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Morning(usize),
    Answer(usize),
    GoOn,
}

/// The tappable rows on this screen: where, and what a tap there does.
pub fn choices(game: &Game) -> Vec<(Rect, Pick)> {
    match &game.phase {
        Phase::Morning => rules::morning_options(&game.run)
            .iter()
            .enumerate()
            .map(|(index, _)| (morning_row(index), Pick::Morning(index)))
            .collect(),
        Phase::Call(_) => (0..3)
            .map(|index| (answer_row(index), Pick::Answer(index)))
            .collect(),
        Phase::Hangup(_) | Phase::Ended(_) => vec![(
            Rect::from_min_size(Vec2::ZERO, Vec2::new(DESIGN_W, DESIGN_H)),
            Pick::GoOn,
        )],
    }
}

/// Option `index` of the morning: its label row and its effect row.
pub fn morning_row(index: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(MARGIN, 140.0 + 44.0 * index as f32),
        Vec2::new(DESIGN_W - 2.0 * MARGIN, 40.0),
    )
}

/// Answer `index` of the call screen.
pub fn answer_row(index: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(MARGIN, 262.0 + 40.0 * index as f32),
        Vec2::new(DESIGN_W - 2.0 * MARGIN, 34.0),
    )
}

/// The screen for whatever phase the game is in.
pub fn screen(game: &Game) -> Panel<Art> {
    match &game.phase {
        Phase::Morning => morning(game),
        Phase::Call(call) => on_the_line(game, call),
        Phase::Hangup(hangup) => hung_up(game, hangup),
        Phase::Ended(ending) => {
            let (verdict, line) = ending_lines(*ending);
            let mut panel = Panel::default();
            panel.text(TextRun::new(Vec2::new(MARGIN, 160.0), verdict, title()));
            panel.text(TextRun::new(Vec2::new(MARGIN, 210.0), line, body()));
            panel.text(TextRun::new(
                Vec2::new(MARGIN, 250.0),
                format!("sanity {} of {}", game.run.sanity, rules::START_SANITY),
                body(),
            ));
            panel.text(TextRun::new(
                Vec2::new(MARGIN, 500.0),
                "Enter, Space or tap: a new run",
                small(),
            ));
            panel
        }
    }
}

/// The header every screen of a day shares: its title and the sanity figure.
fn header(panel: &mut Panel<Art>, heading: String, sanity: i32) {
    panel.text(TextRun::new(Vec2::new(MARGIN, 22.0), heading, title()));
    let figure = format!("SANITY {sanity}");
    let color = if sanity <= 25 {
        palette::DANGER
    } else {
        palette::INK
    };
    let style = style(26.0, color);
    let at = Vec2::new(DESIGN_W - MARGIN - style.width_of(&figure), 22.0);
    panel.text(TextRun::new(at, figure, style));
}

/// The morning: the day, tonight's callers, and every option with its effect.
pub fn morning(game: &Game) -> Panel<Art> {
    let run = &game.run;
    let mut panel = Panel::default();
    header(
        &mut panel,
        format!("DAY {} OF {} - MORNING", run.day, rules::DAYS),
        run.sanity,
    );
    let tempers: Vec<String> = crate::lore::Being::ALL
        .iter()
        .map(|being| format!("{} {}", rules::short_name(*being), run.temper_of(*being)))
        .collect();
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 84.0),
        format!(
            "composure {}   temper: {}",
            run.composure,
            tempers.join(", ")
        ),
        small(),
    ));
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 108.0),
        format!(
            "TONIGHT, AS THINGS STAND: {}",
            callers_line(&rules::tonight(run).callers())
        ),
        style(17.0, palette::VOICE),
    ));
    let label = Cell::new(Vec2::ZERO, DESIGN_W - 2.0 * MARGIN);
    let effect = Cell::new(Vec2::new(28.0, 20.0), DESIGN_W - 2.0 * MARGIN - 28.0);
    for (index, action) in rules::morning_options(run).iter().enumerate() {
        let row = morning_row(index).min;
        panel.text(label.at(row).run(
            &format!("{}  {}", index + 1, rules::action_label(*action)),
            body(),
        ));
        let tone = match action {
            MorningAction::Study(_) => palette::LORE,
            MorningAction::Disrupt(_) | MorningAction::Train => palette::FAINT,
        };
        panel.text(
            effect
                .at(row)
                .run(&rules::effect_line(run, *action), style(15.0, tone)),
        );
    }
    let lore: Vec<String> = crate::lore::Being::ALL
        .iter()
        .map(|being| {
            let known = run.known[being.index()]
                .iter()
                .filter(|known| **known)
                .count();
            format!(
                "{} {known}/{}",
                rules::short_name(*being),
                crate::lore::FACTS
            )
        })
        .collect();
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 478.0),
        format!("lore known: {}", lore.join(", ")),
        small(),
    ));
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 502.0),
        "1-7 or tap a row: spend the morning",
        small(),
    ));
    panel
}

/// A call: who, how it is going, the question, the answers, and what you know.
pub fn on_the_line(game: &Game, call: &CallState) -> Panel<Art> {
    let run = &game.run;
    let lore = call.being.lore();
    let mut panel = Panel::default();
    header(
        &mut panel,
        format!(
            "NIGHT {} - CALL {} OF {}",
            run.day,
            call.slot + 1,
            game.night.calls.len()
        ),
        run.sanity,
    );
    let drain = rules::drain(call.being, run.temper_of(call.being), run.composure);
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 84.0),
        format!("{} - of {}", lore.name, lore.cult),
        style(17.0, palette::VOICE),
    ));
    let temper = run.temper_of(call.being);
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 108.0),
        format!(
            "temper {temper}   drain {drain} per exchange   wants {} more",
            call.interest
        ),
        style(
            15.0,
            if temper > 0 {
                palette::DANGER
            } else {
                palette::FAINT
            },
        ),
    ));
    let said = call.last.as_deref().unwrap_or(lore.greeting);
    panel.text(Cell::new(Vec2::new(MARGIN, 136.0), DESIGN_W - 2.0 * MARGIN).run(said, small()));
    let Some(asking) = game.asking() else {
        return panel;
    };
    let fact = &lore.facts[asking.fact];
    let voice = style(22.0, palette::VOICE);
    let columns = voice.columns_in(DESIGN_W - 2.0 * MARGIN);
    panel.block(
        Vec2::new(MARGIN, 172.0),
        &wrap(fact.question, columns),
        voice,
        4.0,
    );
    let known = run.knows(call.being, asking.fact);
    let answer = Cell::new(Vec2::ZERO, DESIGN_W - 2.0 * MARGIN);
    let hint = Cell::new(Vec2::new(28.0, 18.0), DESIGN_W - 2.0 * MARGIN - 28.0);
    for index in 0..3 {
        let row = answer_row(index).min;
        let kind = asking.order[index];
        let text = fact.answers[crate::lore::AnswerKind::ALL
            .iter()
            .position(|each| *each == kind)
            .unwrap_or(0)];
        panel.text(
            answer
                .at(row)
                .run(&format!("{}  {text}", index + 1), body()),
        );
        if let Some((_, outcome)) = game.preview(index) {
            let (line, tone) = if known {
                (hint_line(outcome), palette::LORE)
            } else {
                ("?".to_owned(), palette::FAINT)
            };
            panel.text(hint.at(row).run(&line, style(15.0, tone)));
        }
    }
    let heading = format!("WHAT YOU KNOW OF {}:", lore.name.to_uppercase());
    panel.text(TextRun::new(Vec2::new(MARGIN, 388.0), heading, small()));
    let known_facts: Vec<&str> = (0..crate::lore::FACTS)
        .filter(|each| run.knows(call.being, *each))
        .map(|each| lore.facts[each].known)
        .collect();
    let fact_cell = Cell::new(Vec2::ZERO, DESIGN_W - 2.0 * MARGIN - 16.0);
    if known_facts.is_empty() {
        panel.text(
            fact_cell
                .at(Vec2::new(MARGIN + 16.0, 410.0))
                .run("nothing yet", small()),
        );
    }
    for (row, known) in known_facts.iter().enumerate() {
        let at = Vec2::new(MARGIN + 16.0, 410.0 + 20.0 * row as f32);
        panel.text(fact_cell.at(at).run(known, style(15.0, palette::LORE)));
    }
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 502.0),
        format!(
            "1-3 or tap: answer.   silence costs {drain} every {} s",
            rules::SILENCE_TICKS / 60
        ),
        small(),
    ));
    panel
}

/// The hint an answer carries when its question is about a fact you know —
/// read off `rules::answer_outcome`, the function the exchange applies.
pub fn hint_line(outcome: rules::Outcome) -> String {
    let anger = if outcome.temper_change > 0 {
        ", angers it"
    } else {
        ""
    };
    format!(
        "{:+} to how long it wants to talk, -{} sanity{anger}",
        outcome.interest_change, outcome.sanity_cost
    )
}

/// Between calls: what the last one cost, and what comes next.
pub fn hung_up(game: &Game, hangup: &Hangup) -> Panel<Art> {
    let mut panel = Panel::default();
    header(&mut panel, "THE LINE GOES DEAD".to_owned(), game.run.sanity);
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 120.0),
        format!(
            "{} hung up after {} exchanges.",
            hangup.being.lore().name,
            hangup.exchanges
        ),
        body(),
    ));
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 150.0),
        format!("That call cost {} sanity.", hangup.spent),
        body(),
    ));
    panel.text(
        Cell::new(Vec2::new(MARGIN, 180.0), DESIGN_W - 2.0 * MARGIN).run(&hangup.last, small()),
    );
    let next = match game.night.calls.get(hangup.slot + 1) {
        Some(call) => format!("The phone rings again: {}.", call.being.lore().name),
        None if game.run.day >= rules::DAYS => "The last night is over.".to_owned(),
        None => format!("Dawn. Day {} is coming.", game.run.day + 1),
    };
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 230.0),
        next,
        style(17.0, palette::VOICE),
    ));
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 502.0),
        "Enter, Space or tap: go on",
        small(),
    ));
    panel
}
