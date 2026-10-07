//! What each stage puts on the screen, as data: a `Panel` of rows, the rectangles that
//! answer a click, and the bars. The draw system, the click reader and the checks all
//! read these same functions, so the thing drawn, the thing clicked and the thing judged
//! cannot be three different lists.
//!
//! Layout is in a 960x540 design space, drawn through a mapping that fits it in the
//! camera (`main.rs`). This is the prototype answer: one aspect, letterboxed on others.

use jidousha::prelude::*;
use jidousha::ui::{Floors, Icon, Panel, TextRun, clipped, wrap};

use crate::game::{Choice, Ending, Game, Stage};
use crate::lore::{Being, FACTS, FACTS_PER_BEING};
use crate::palette;
use crate::rules::{
    ACTIONS, DAYS, Kind, START_SANITY, answer_outcome, describe_action, exchange_cost, option_kinds,
};

/// The design space's width.
pub const DESIGN_W: f32 = 960.0;
/// The design space's height.
pub const DESIGN_H: f32 = 540.0;
/// The smallest text on any screen, in design units.
pub const MIN_TEXT: f32 = 12.0;
/// Body text.
pub const BODY: f32 = 14.0;
/// Headings.
pub const HEAD: f32 = 18.0;
/// The side margin.
const MARGIN: f32 = 24.0;

/// The floors every screen is judged against.
pub const FLOORS: Floors = Floors {
    min_text: MIN_TEXT,
    chrome: Rect {
        min: Vec2::new(0.0, 0.0),
        max: Vec2::new(DESIGN_W, DESIGN_H),
    },
    world: Rect {
        min: Vec2::new(0.0, 0.0),
        max: Vec2::new(DESIGN_W, DESIGN_H),
    },
};

/// This game has no pictures; the kit wants a vocabulary for them anyway, and this is an
/// empty one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NoArt {}

impl Icon for NoArt {
    fn size_at(self, _scale: f32) -> Vec2 {
        match self {}
    }
}

/// The stage's panel type.
pub type Screen = Panel<NoArt>;

fn style(size: f32, color: Color) -> TextStyle {
    TextStyle {
        size,
        color,
        depth: Depth::layer(1),
        ..TextStyle::default()
    }
}

/// `text` wrapped to what fits across `width` in `style`.
fn wrapped(style: &TextStyle, text: &str, width: f32) -> String {
    wrap(text, style.columns_in(width))
}

/// Where morning option `i` sits: two columns of four cells.
pub fn morning_cell(i: usize) -> Rect {
    let (col, row) = (i / 4, i % 4);
    Rect::from_min_size(
        Vec2::new(MARGIN + col as f32 * 468.0, 150.0 + row as f32 * 84.0),
        Vec2::new(444.0, 78.0),
    )
}

/// The left column of the call screen's width.
const CALL_W: f32 = 536.0;
/// Where call answer `i` (0-2) and the hang-up (3) sit.
pub fn call_row(i: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(MARGIN, 192.0 + i as f32 * 66.0),
        Vec2::new(CALL_W, 62.0),
    )
}

/// Where the right-hand status block starts on the call screen.
const STATUS_X: f32 = 596.0;
const STATUS_Y: f32 = 318.0;
/// How wide the right-hand column is.
const SIDE_W: f32 = 340.0;

/// The temper as a word.
pub fn temper_word(being: Being, temper: i32) -> &'static str {
    if temper <= 0 {
        "calm"
    } else if temper >= being.wrath_at() {
        "IN WRATH"
    } else {
        "irked"
    }
}

/// What the click targets are on this stage: a rectangle and the choice it makes.
pub fn controls(game: &Game) -> Vec<(Rect, Choice)> {
    match &game.stage {
        Stage::Morning => (0..ACTIONS.len())
            .map(|i| (morning_cell(i), Choice::Option(i)))
            .collect(),
        Stage::Call => (0..4).map(|i| (call_row(i), Choice::Option(i))).collect(),
        Stage::Over(_) => Vec::new(),
    }
}

/// The filled rectangles: the sanity bar, and on a call the temper pips.
pub fn bars(game: &Game) -> Vec<(Rect, Color)> {
    let mut out = Vec::new();
    let bar = Rect::from_min_size(Vec2::new(MARGIN, 36.0), Vec2::new(300.0, 8.0));
    out.push((bar, palette::EMPTY));
    let filled = (game.sanity.max(0) as f32 / START_SANITY as f32).min(1.0);
    out.push((
        Rect::from_min_size(bar.min, Vec2::new(bar.size().x * filled, bar.size().y)),
        palette::SANITY,
    ));
    if let (Stage::Call, Some(call)) = (&game.stage, &game.call) {
        let wrath = call.plan.being.wrath_at();
        for pip in 0..wrath {
            let color = if pip < call.temper {
                palette::WARN
            } else {
                palette::EMPTY
            };
            out.push((
                Rect::from_min_size(
                    Vec2::new(STATUS_X + 200.0 + pip as f32 * 18.0, STATUS_Y + 2.0),
                    Vec2::new(12.0, 12.0),
                ),
                color,
            ));
        }
    }
    out
}

/// The header every stage opens with: title, day, sanity as a number.
fn header(panel: &mut Screen, game: &Game, what: &str) {
    let title = format!(
        "CALL OF CTHULHU    Day {} of {DAYS} - {what}",
        game.day.min(DAYS)
    );
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 10.0),
        title,
        style(HEAD, palette::GOLD),
    ));
    let sanity = format!("Sanity {} / {START_SANITY}", game.sanity.max(0));
    let sty = style(
        BODY,
        if game.sanity <= 10 {
            palette::WARN
        } else {
            palette::TEXT
        },
    );
    panel.text(TextRun::new(Vec2::new(MARGIN + 316.0, 34.0), sanity, sty));
}

/// The panel for the game as it stands.
pub fn panel(game: &Game) -> Screen {
    match &game.stage {
        Stage::Morning => morning(game),
        Stage::Call => call(game),
        Stage::Over(ending) => over(game, *ending),
    }
}

/// Everything known, as "Dagon 2/5" counts.
pub fn known_line(game: &Game) -> String {
    let mut parts: Vec<String> = Being::ALL
        .iter()
        .map(|b| {
            format!(
                "{} {}/{FACTS_PER_BEING}",
                b.name(),
                game.known_facts(*b).len()
            )
        })
        .collect();
    parts.push(format!(
        "Composure {}/{}",
        game.composure,
        crate::rules::COMPOSURE_MAX
    ));
    format!("You know: {}", parts.join("   "))
}

fn morning(game: &Game) -> Screen {
    let mut panel = Screen::default();
    header(&mut panel, game, "Morning");
    let note = style(BODY, palette::NOTE);
    let at = Vec2::new(MARGIN, 58.0);
    panel.block(
        at,
        &wrapped(&note, &game.note, DESIGN_W - 2.0 * MARGIN),
        note,
        3.0,
    );
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 96.0),
        known_line(game),
        style(BODY, palette::TEXT),
    ));
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 124.0),
        "Spend the morning on one thing. Each says what it does to tonight.",
        style(BODY, palette::GOLD),
    ));
    for (i, action) in ACTIONS.iter().enumerate() {
        let cell = morning_cell(i);
        let effect = describe_action(game, *action);
        let head_color = if effect.available {
            palette::TEXT
        } else {
            palette::NOTE
        };
        panel.text(TextRun::new(
            cell.min,
            format!("[{}] {}", i + 1, effect.label),
            style(BODY, head_color),
        ));
        let small = style(MIN_TEXT, palette::NOTE);
        panel.block(
            cell.min + Vec2::new(0.0, 18.0),
            &wrapped(&small, &effect.text, cell.size().x),
            small,
            1.0,
        );
    }
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 506.0),
        "Press 1-8, or click an option.",
        style(MIN_TEXT, palette::NOTE),
    ));
    panel
}

fn call(game: &Game) -> Screen {
    let mut panel = Screen::default();
    let Some(call) = &game.call else {
        return panel;
    };
    let being = call.plan.being;
    header(
        &mut panel,
        game,
        &format!("Evening, call {} of {}", call.slot + 1, game.night.len()),
    );
    let body = style(BODY, palette::NOTE);
    panel.block(
        Vec2::new(MARGIN, 56.0),
        &wrapped(&body, &format!("{}: {}", being.name(), call.reply), CALL_W),
        body,
        2.0,
    );
    let Some(fact) = call.fact() else {
        return panel;
    };
    let head = style(HEAD, palette::TEXT);
    panel.block(
        Vec2::new(MARGIN, 134.0),
        &wrapped(&head, fact.question, CALL_W),
        head,
        2.0,
    );
    let knows = game.known[being.index()][call.fact_index()];
    let kinds = option_kinds(game.seed, game.day, call.slot, call.exchange);
    for (i, kind) in kinds.iter().enumerate() {
        let text = match kind {
            Kind::Right => fact.right,
            Kind::Guess => fact.guess,
            _ => fact.insult,
        };
        let row = call_row(i);
        panel.text(TextRun::new(
            row.min,
            format!("[{}] {}", i + 1, text),
            style(BODY, palette::TEXT),
        ));
        let hint = if knows {
            hint_for(game, call.plan.being, call.temper, *kind)
        } else {
            "? You have no lore on this. It could be right, a guess, or an insult.".to_owned()
        };
        let small = style(
            MIN_TEXT,
            if knows && *kind == Kind::Right {
                palette::GOOD
            } else {
                palette::NOTE
            },
        );
        panel.block(
            row.min + Vec2::new(0.0, 18.0),
            &wrapped(&small, &hint, CALL_W),
            small,
            1.0,
        );
    }
    let row = call_row(3);
    panel.text(TextRun::new(
        row.min,
        "[4] Hang up",
        style(BODY, palette::TEXT),
    ));
    let small = style(MIN_TEXT, palette::NOTE);
    let hang = hint_for(game, being, call.temper, Kind::HangUp);
    panel.block(
        row.min + Vec2::new(0.0, 18.0),
        &wrapped(&small, &hang, CALL_W),
        small,
        1.0,
    );
    side(&mut panel, game, call);
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 506.0),
        "Press 1-4, or click an answer.",
        style(MIN_TEXT, palette::NOTE),
    ));
    panel
}

/// What an answer of `kind` would do, in words, from the one function the resolution uses.
pub fn hint_for(game: &Game, being: Being, temper: i32, kind: Kind) -> String {
    let o = answer_outcome(being, game.composure, temper, kind);
    let length = match o.remaining_delta {
        d if d < 0 => format!("Shortens the call by {}.", -d),
        0 => "The call gets no shorter.".to_owned(),
        d => format!("The call grows {d} longer."),
    };
    match kind {
        Kind::Right => format!("Right. {length} It calms. Costs {} sanity.", o.sanity_cost),
        Kind::Guess => format!("Wrong. {length} Costs {} sanity.", o.sanity_cost),
        Kind::Insult => format!(
            "An insult. {length} It grows angrier. Costs {} sanity.",
            o.sanity_cost
        ),
        Kind::HangUp => format!(
            "Ends the call now. Costs {} sanity, and {} calls first tomorrow.",
            o.sanity_cost,
            being.name()
        ),
    }
}

/// The right-hand column of a call: what you know of this being, and how the line stands.
fn side(panel: &mut Screen, game: &Game, call: &crate::game::Call) {
    let being = call.plan.being;
    let gold = style(BODY, palette::GOLD);
    panel.text(TextRun::new(
        Vec2::new(STATUS_X, 56.0),
        format!("WHAT YOU KNOW OF {}", being.name().to_uppercase()),
        gold,
    ));
    let small = style(MIN_TEXT, palette::TEXT);
    let known = game.known_facts(being);
    let mut y = 78.0;
    if known.is_empty() {
        panel.text(TextRun::new(
            Vec2::new(STATUS_X, y),
            "Nothing. Every answer is a guess.",
            style(MIN_TEXT, palette::NOTE),
        ));
    }
    for fact in known {
        let text = wrapped(
            &small,
            &format!("- {}", FACTS[being.index()][fact].lore),
            330.0,
        );
        y = panel.block(Vec2::new(STATUS_X, y), &text, small, 1.0) + 3.0;
    }
    let body = style(BODY, palette::TEXT);
    let cost = exchange_cost(being, game.composure, call.temper);
    let wrath = call.temper >= being.wrath_at();
    let lines = [
        (format!("Temper: {}", temper_word(being, call.temper)), body),
        (
            format!("About {} exchanges left.", call.remaining.max(1)),
            body,
        ),
        (
            format!("Every exchange costs {cost} sanity."),
            style(BODY, if wrath { palette::WARN } else { palette::TEXT }),
        ),
        (
            format!(
                "Composure {}/{}.",
                game.composure,
                crate::rules::COMPOSURE_MAX
            ),
            style(MIN_TEXT, palette::NOTE),
        ),
    ];
    for (i, (text, sty)) in lines.into_iter().enumerate() {
        panel.text(TextRun::new(
            Vec2::new(STATUS_X, STATUS_Y + i as f32 * 24.0),
            clipped(&sty, &text, SIDE_W),
            sty,
        ));
    }
}

fn over(game: &Game, ending: Ending) -> Screen {
    let mut panel = Screen::default();
    header(&mut panel, game, "The end");
    let (title, body) = match ending {
        Ending::Sound => (
            "THE LINE GOES QUIET",
            "Five nights. You are tired, and you are still yourself. The phone does not ring.",
        ),
        Ending::Frayed => (
            "THE LINE GOES QUIET, MOSTLY",
            "Five nights. You made it, but something of you stayed on the line.",
        ),
        Ending::Lost => (
            "YOU STAY ON THE LINE",
            "Your sanity runs out in the middle of a sentence. Something else finishes it.",
        ),
    };
    panel.text(TextRun::new(
        Vec2::new(MARGIN, 120.0),
        title,
        style(HEAD, palette::GOLD),
    ));
    let text = style(BODY, palette::TEXT);
    let at = Vec2::new(MARGIN, 160.0);
    let next = panel.block(at, &wrapped(&text, body, 880.0), text, 3.0);
    let known: usize = Being::ALL.iter().map(|b| game.known_facts(*b).len()).sum();
    panel.text(TextRun::new(
        Vec2::new(MARGIN, next + 16.0),
        format!(
            "Day {} - sanity {} - lore {known}/{}",
            game.day.min(DAYS),
            game.sanity.max(0),
            crate::lore::BEINGS * FACTS_PER_BEING
        ),
        style(BODY, palette::NOTE),
    ));
    panel.text(TextRun::new(
        Vec2::new(MARGIN, next + 48.0),
        "Press R to begin again.",
        style(BODY, palette::NOTE),
    ));
    panel
}
