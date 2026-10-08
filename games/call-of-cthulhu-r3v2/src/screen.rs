//! Every screen as one `Panel`, built by one pure function from the game.
//!
//! The draw system draws exactly this panel, the floors judge it, and the
//! transcript is its strings — so the thing drawn, the thing judged and the
//! thing asserted are one list.

use crate::beings::BeingId;
use crate::flow::{
    Game, Screen, call_options, effect_line, morning_label, morning_options, option_text,
};
use crate::rules::{AnswerKind, DAYS, START_SANITY, TEMPER_LIMIT, WRATH, answer_outcome};
use jidousha::prelude::*;
use jidousha::ui::*;

/// The game has no pictures; the panel's icon role is empty.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Art {}

impl Icon for Art {
    fn size_at(self, _scale: f32) -> Vec2 {
        match self {}
    }
}

/// The design space, which is also the world: the camera shows exactly it.
pub const DESIGN: Vec2 = Vec2::new(960.0, 540.0);
/// The left edge every row starts at.
pub const LEFT: f32 = 24.0;
/// How wide a row may run.
pub const WIDTH: f32 = 912.0;
/// Where the sanity bar's track sits.
pub const BAR_AT: Vec2 = Vec2::new(200.0, 50.0);
/// The bar's full width.
pub const BAR_W: f32 = 400.0;
/// The bar's height.
pub const BAR_H: f32 = 10.0;
/// Where the footer hint starts.
pub const HINT_AT: Vec2 = Vec2::new(24.0, 496.0);
/// Below this, the bar turns ember.
pub const LOW_SANITY: i32 = 25;

/// The floors every screen is judged against.
pub const FLOORS: Floors = Floors {
    min_text: 12.0,
    chrome: Rect {
        min: Vec2::ZERO,
        max: DESIGN,
    },
    world: Rect {
        min: Vec2::ZERO,
        max: DESIGN,
    },
};

/// Draw bands.
pub mod layers {
    /// The sanity bar.
    pub const BAR: i16 = -1;
    /// Every row of text.
    pub const TEXT: i16 = 1;
}

/// The palette.
pub mod palette {
    use jidousha::prelude::Color;
    pub const NIGHT: Color = Color::rgb(0.05, 0.04, 0.09);
    pub const INK: Color = Color::rgb(0.86, 0.83, 0.74);
    pub const DIM: Color = Color::rgb(0.52, 0.50, 0.58);
    pub const GOLD: Color = Color::rgb(0.90, 0.72, 0.30);
    pub const EMBER: Color = Color::rgb(0.85, 0.32, 0.22);
    pub const SEA: Color = Color::rgb(0.35, 0.75, 0.70);
}

/// Design space is world space: the camera is centred on the design rect.
pub struct Identity;

impl Mapping for Identity {
    fn to_world(&self, ui: Vec2) -> Vec2 {
        ui
    }
    fn scale(&self) -> f32 {
        1.0
    }
}

fn style(size: f32, color: Color) -> TextStyle {
    TextStyle {
        size,
        color,
        depth: Depth::layer(layers::TEXT),
        ..TextStyle::default()
    }
}

/// The title.
pub fn title_style() -> TextStyle {
    style(20.0, palette::SEA)
}
/// Body text.
pub fn body() -> TextStyle {
    style(14.0, palette::INK)
}
/// Effect lines and hints.
pub fn small() -> TextStyle {
    style(12.0, palette::DIM)
}
/// Lore, and the lore reply.
pub fn gold() -> TextStyle {
    style(14.0, palette::GOLD)
}
/// Costs and tempers.
pub fn ember() -> TextStyle {
    style(14.0, palette::EMBER)
}

/// Extra space between body rows (14 + 4 = 18).
const BODY_GAP: f32 = 4.0;
/// Extra space between small rows (12 + 4 = 16).
const SMALL_GAP: f32 = 4.0;

/// A row of body-sized prose, wrapped to the row width.
fn para(panel: &mut Panel<Art>, at: Vec2, text: &str, style: TextStyle, gap: f32) -> f32 {
    let columns = style.columns_in(WIDTH - (at.x - LEFT));
    panel.block(at, &wrap(text, columns), style, gap)
}

/// The rows every screen opens with: the title, the day and phase, sanity.
fn header(panel: &mut Panel<Art>, game: &Game, phase: &str) {
    panel.text(TextRun::new(
        Vec2::new(LEFT, 16.0),
        "CALL OF CTHULHU",
        title_style(),
    ));
    panel.text(TextRun::new(
        Vec2::new(600.0, 20.0),
        format!("DAY {} of {DAYS} - {phase}", game.day),
        body(),
    ));
    panel.text(TextRun::new(
        Vec2::new(LEFT, 48.0),
        sanity_row(game),
        body(),
    ));
}

/// The sanity row's text.
pub fn sanity_row(game: &Game) -> String {
    format!("sanity {}/{START_SANITY}", game.sanity.max(0))
}

/// The screen the game is on, as a panel.
pub fn screen(game: &Game) -> Panel<Art> {
    let mut panel = Panel::default();
    match game.screen {
        Screen::Morning => morning(&mut panel, game),
        Screen::Exchange => exchange(&mut panel, game),
        Screen::Dawn => dawn(&mut panel, game),
        Screen::End { won } => end(&mut panel, game, won),
    }
    panel
}

fn plan_part(game: &Game, being: BeingId, per: u32) -> String {
    format!(
        "{} ({per} sanity an exchange, temper {}/{TEMPER_LIMIT})",
        being.name(),
        game.tempers[being.index()]
    )
}

fn morning(panel: &mut Panel<Art>, game: &Game) {
    header(panel, game, "MORNING");
    let plan = game.plan();
    let parts: Vec<String> = plan
        .calls
        .iter()
        .map(|call| plan_part(game, call.being, call.per_exchange))
        .collect();
    let mut y = para(
        panel,
        Vec2::new(LEFT, 80.0),
        &format!("tonight: {}", parts.join(", then ")),
        ember(),
        BODY_GAP,
    );
    for being in BeingId::ALL {
        panel.text(TextRun::new(
            Vec2::new(LEFT, y),
            format!(
                "{} - temper {}/{TEMPER_LIMIT} - secrets known {}/3",
                being.name(),
                game.tempers[being.index()],
                game.secrets_known(being)
            ),
            body(),
        ));
        y += 14.0 + BODY_GAP;
    }
    y += 8.0;
    for (index, action) in morning_options(game).into_iter().enumerate() {
        y = para(
            panel,
            Vec2::new(LEFT, y),
            &format!("{}. {}", index + 1, morning_label(action)),
            body(),
            2.0,
        );
        y = para(
            panel,
            Vec2::new(LEFT + 24.0, y),
            &effect_line(game, action),
            small(),
            SMALL_GAP,
        );
    }
    panel.hint(HINT_AT, WIDTH, "press the number of a choice", small(), 2.0);
}

fn exchange(panel: &mut Panel<Art>, game: &Game) {
    header(panel, game, "NIGHT");
    let (Some(being), Some(night)) = (game.caller(), game.night.as_ref()) else {
        return;
    };
    let data = being.being();
    let temper = game.tempers[being.index()];
    let wrong = answer_outcome(being, temper, game.meditated, AnswerKind::Wrong);
    let lore = answer_outcome(being, temper, game.meditated, AnswerKind::Lore);
    let anger = answer_outcome(being, temper, game.meditated, AnswerKind::Anger);
    let hang = answer_outcome(being, temper, game.meditated, AnswerKind::HangUp);

    let mut y = para(
        panel,
        Vec2::new(LEFT, 80.0),
        &format!(
            "call {} of {} - {}, {}",
            night.call + 1,
            night.plan.calls.len(),
            data.name,
            data.epithet
        ),
        body(),
        BODY_GAP,
    );
    y = para(
        panel,
        Vec2::new(LEFT, y),
        &format!(
            "each exchange: -{} sanity | temper {temper}/{TEMPER_LIMIT} | patience {}",
            wrong.sanity_cost, night.patience
        ),
        ember(),
        BODY_GAP,
    );
    if !game.last_line.is_empty() {
        y = para(panel, Vec2::new(LEFT, y), &game.last_line, body(), BODY_GAP);
    }
    if night.exchange == 0 {
        y = para(panel, Vec2::new(LEFT, y), data.opening, body(), BODY_GAP);
    }
    y += 4.0;
    y = para(
        panel,
        Vec2::new(LEFT, y),
        &format!("what you know of {}:", data.name),
        body(),
        BODY_GAP,
    );
    let mut any = false;
    for (q, question) in data.questions.iter().enumerate() {
        if game.known[being.index()][q] {
            any = true;
            y = para(
                panel,
                Vec2::new(LEFT + 16.0, y),
                &format!("- {}", question.fact),
                gold(),
                BODY_GAP,
            );
        }
    }
    if !any {
        y = para(
            panel,
            Vec2::new(LEFT + 16.0, y),
            "- nothing",
            small(),
            BODY_GAP,
        );
    }
    y += 4.0;
    let question = &data.questions[game.question_index()];
    y = para(
        panel,
        Vec2::new(LEFT, y),
        &format!("it says: \"{}\"", question.ask),
        body(),
        BODY_GAP,
    );
    for (index, (kind, text)) in call_options(game).into_iter().enumerate() {
        let tone = if kind == AnswerKind::Lore {
            gold()
        } else {
            body()
        };
        y = para(
            panel,
            Vec2::new(LEFT + 16.0, y),
            &option_text(index, kind, text),
            tone,
            BODY_GAP,
        );
    }
    let _ = y;
    panel.hint(
        HINT_AT,
        WIDTH,
        &format!(
            "* a reply from its lore costs {} sanity. any other costs {}; one that offends it costs {} and +{} temper. hanging up adds +{} temper. at temper {TEMPER_LIMIT} it screams: -{WRATH} sanity and the call ends.",
            lore.sanity_cost,
            wrong.sanity_cost,
            anger.sanity_cost,
            anger.temper_delta,
            hang.temper_delta
        ),
        small(),
        2.0,
    );
}

fn dawn(panel: &mut Panel<Art>, game: &Game) {
    header(panel, game, "DAWN");
    let mut y = para(
        panel,
        Vec2::new(LEFT, 80.0),
        &format!("NIGHT {} is over.", game.day),
        body(),
        BODY_GAP,
    );
    if let Some(night) = game.night.as_ref() {
        for (being, exchanges, spent) in &night.summary {
            y = para(
                panel,
                Vec2::new(LEFT + 16.0, y),
                &format!("{}: {exchanges} exchanges, -{spent} sanity", being.name()),
                body(),
                BODY_GAP,
            );
        }
    }
    let _ = para(
        panel,
        Vec2::new(LEFT, y + 4.0),
        &sanity_row(game),
        body(),
        BODY_GAP,
    );
    panel.hint(HINT_AT, WIDTH, "press Enter", small(), 2.0);
}

fn end(panel: &mut Panel<Art>, game: &Game, won: bool) {
    header(panel, game, "END");
    if won {
        let y = para(
            panel,
            Vec2::new(LEFT, 80.0),
            &format!("DAWN OF DAY {}. You pull the cord from the wall.", DAYS + 1),
            gold(),
            BODY_GAP,
        );
        let _ = para(
            panel,
            Vec2::new(LEFT, y),
            &format!("sanity {}/{START_SANITY} remains.", game.sanity.max(0)),
            body(),
            BODY_GAP,
        );
    } else {
        let y = para(
            panel,
            Vec2::new(LEFT, 80.0),
            "Your sanity is gone.",
            ember(),
            BODY_GAP,
        );
        let talker = game.caller().map_or("Something", BeingId::name);
        let _ = para(
            panel,
            Vec2::new(LEFT, y),
            &format!("{talker} is still talking."),
            body(),
            BODY_GAP,
        );
    }
    panel.hint(HINT_AT, WIDTH, "press Enter to begin again", small(), 2.0);
}
