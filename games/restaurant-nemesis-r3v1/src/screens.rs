//! The three screens — ledger, service, end — each a `jidousha::ui::Panel`
//! built by a pure function of the `Game`, in a 960x540 design space mapped
//! one-to-one onto the world.
//!
//! The words come from the rule functions: an order row's "if unmet" line is
//! `outcome(.., Skip)`, the nemesis card is `progress` and `visit_result`, the
//! ledger's forecasts are `forecast`. The draw, the floors and the frame check
//! all read the same panel.

use jidousha::prelude::*;
use jidousha::ui::{Cell, Icon, Mapping, Panel, TextRun, clipped};

use crate::content::{DINERS, THEMES};
use crate::rules::{
    self, CAPACITY, Consequence, DAYS, RENT, ROUNDS, TAKEDOWN_COST, TAKEDOWN_CUT, TEMP_COST,
};
use crate::sim::{Game, Order, PER_DAY, Phase, Serve, Who};

/// The design space every screen is laid out in.
pub const DESIGN: Vec2 = Vec2::new(960.0, 540.0);

/// This game draws no pictures: an art vocabulary with no roles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Art {}

impl Icon for Art {
    fn size_at(self, _scale: f32) -> Vec2 {
        match self {}
    }
}

/// Design units are world units: the camera looks at exactly the design
/// rect.
pub struct OneToOne;

impl Mapping for OneToOne {
    fn to_world(&self, ui: Vec2) -> Vec2 {
        ui
    }
    fn scale(&self) -> f32 {
        1.0
    }
}

/// Bands.
pub mod layers {
    /// Row backgrounds.
    pub const BACK: i16 = 0;
    /// Text.
    pub const TEXT: i16 = 1;
}

/// Palette.
pub mod palette {
    use jidousha::prelude::Color;
    /// The floor of the restaurant.
    pub const FLOOR: Color = Color::rgb(0.10, 0.08, 0.09);
    /// Plain text.
    pub const TEXT: Color = Color::rgb(0.93, 0.92, 0.88);
    /// Quieter text.
    pub const DIM: Color = Color::rgb(0.62, 0.62, 0.6);
    /// A warning: a spawn, a defeat, a loss.
    pub const ALARM: Color = Color::rgb(1.0, 0.45, 0.35);
    /// Good news.
    pub const GOOD: Color = Color::rgb(0.5, 0.95, 0.55);
    /// The picked row's bar.
    pub const PICKED: Color = Color::rgb(1.0, 0.8, 0.3);
    /// A card's outline.
    pub const CARD: Color = Color::rgb(0.45, 0.42, 0.45);
}

/// Text sizes.
pub const ROW: f32 = 14.0;
/// The card and feed size, which is the floor.
pub const SMALL: f32 = 12.0;

fn style(size: f32, color: Color) -> TextStyle {
    TextStyle {
        size,
        color,
        depth: Depth::layer(layers::TEXT),
        ..TextStyle::default()
    }
}

/// How many order rows fit above the feed; a longer queue pages.
pub const VISIBLE_ROWS: usize = 7;

/// Which queue rows are on screen: a window of `VISIBLE_ROWS` that keeps the
/// selected row in view.
pub fn visible_rows(game: &Game) -> std::ops::Range<usize> {
    let rows = game.queue.len();
    let start = game
        .selected
        .saturating_sub(VISIBLE_ROWS - 1)
        .min(rows.saturating_sub(VISIBLE_ROWS));
    start..(start + VISIBLE_ROWS).min(rows)
}

/// The queue's column: where rows go and how wide they may be.
const QUEUE: Cell = Cell {
    at: Vec2::new(16.0, 44.0),
    width: 612.0,
};
/// One order row's height.
pub const ROW_PITCH: f32 = 3.0 * (SMALL + 2.0) + 6.0;
/// The card column.
const CARD: Cell = Cell {
    at: Vec2::new(636.0, 44.0),
    width: 316.0,
};
/// The feed's top.
const FEED_TOP: f32 = 410.0;

/// Who an order is, in words.
pub fn who_line(game: &Game, order: &Order) -> String {
    match order.who {
        Who::Diner(name) => DINERS[name].to_owned(),
        Who::Follower(leader) => format!("fan of {}", game.nemeses[leader].title),
        Who::Nemesis(index) => format!("{} [NEMESIS]", game.nemeses[index].title),
    }
}

/// An order's demands, in words.
pub fn wants(order: &Order) -> String {
    THEMES
        .iter()
        .filter(|theme| order.demands[theme.index()] > 0)
        .map(|theme| format!("{} {}", theme.name(), order.demands[theme.index()]))
        .collect::<Vec<_>>()
        .join("  ")
}

/// What a failure would set off, in words.
pub fn consequence_words(game: &Game, consequence: Consequence) -> String {
    match consequence {
        Consequence::None => "no lasting harm".to_owned(),
        Consequence::Spawn => "SPAWNS A NEMESIS".to_owned(),
        Consequence::Repost { nemesis } => format!(
            "+{} fans to {}",
            rules::CAP_REPOST,
            game.nemeses[nemesis].title
        ),
        Consequence::Feeds {
            nemesis, followers, ..
        } => format!("{:+} fans to {}", followers, game.nemeses[nemesis].title),
    }
}

/// The three lines of one order row: who and the serve; demands and what the
/// serve earns; and what happens if it goes unmet.
pub fn order_lines(game: &Game, index: usize) -> [String; 3] {
    let order = game.queue[index];
    let picked = if index == game.selected { ">" } else { " " };
    let first = format!(
        "{picked}{}. {:<8} {}",
        index + 1,
        order.serve.name(),
        who_line(game, &order)
    );
    let served = rules::outcome(game, &order, order.serve);
    let result = if served.satisfied {
        format!("SATISFIED {:+}$", served.money)
    } else if order.serve == Serve::Skip {
        "unmet".to_owned()
    } else {
        format!("falls short {:+}$", served.money)
    };
    let second = format!("  wants {} -> {result}", wants(&order));
    let unmet = rules::outcome(game, &order, Serve::Skip);
    let third = match unmet.failure {
        Some((theme, severity)) => format!(
            "  if unmet: sev {severity} {}, {:+}$, {}",
            theme.name(),
            unmet.money,
            consequence_words(game, unmet.consequence)
        ),
        None => "  if unmet: nothing (no demands)".to_owned(),
    };
    [first, second, third]
}

/// The nemesis card for nemesis `index` visiting with `order`.
pub fn card_lines(game: &Game, index: usize, order: &Order) -> Vec<String> {
    let them = &game.nemeses[index];
    let reading = rules::progress(game, index);
    let mut lines = vec![
        format!("NEMESIS: {}", them.title),
        format!(
            "theme {}, sensitivity +{}",
            them.theme.name(),
            rules::SENSITIVITY
        ),
        format!(
            "tier {}, {} followers",
            rules::tier(them.followers),
            them.followers
        ),
        "defeat paths:".to_owned(),
        format!("1 tally {}/{} visits", reading.tally.0, reading.tally.1),
        format!(
            "2 overwhelm: quality {} vs need {}",
            reading.overwhelm.1, reading.overwhelm.0
        ),
        format!(
            "3 followers {}, quits under {}",
            reading.following.0, reading.following.1
        ),
        "this visit:".to_owned(),
    ];
    for serve in [Serve::Skip, Serve::Standard, Serve::Careful, Serve::AllOut] {
        let result = rules::visit_result(game, index, order, serve);
        let words = match (result.defeats, result.satisfied) {
            (Some(path), _) => format!("DEFEATED, {}", rules::path_name(path)),
            (None, true) => format!("tally {}/{}", reading.tally.0 + 1, reading.tally.1),
            (None, false) => format!("fails, +{} fans", rules::NEMESIS_FAIL),
        };
        lines.push(format!("{} ({}): {words}", serve.name(), serve.cost()));
    }
    lines
}

/// The ledger's lines for one spends choice.
pub fn ledger_lines(game: &Game) -> Vec<String> {
    let margin = game.money - game.spends.cost() - RENT;
    let mut lines = vec![
        format!("LEDGER - before day {}/{DAYS}", game.day),
        format!(
            "money ${} - spends ${} - rent ${RENT} = margin ${margin}",
            game.money,
            game.spends.cost()
        ),
    ];
    let chosen = rules::forecast(game, &game.spends);
    for (slot, forecast) in chosen.iter().enumerate() {
        let them = &game.nemeses[forecast.nemesis];
        let on = game.spends.takedowns.contains(&forecast.nemesis);
        lines.push(format!(
            "[{}] {}  {}  {} followers  takedown {}",
            slot + 1,
            them.title,
            them.theme.name(),
            them.followers,
            if on { "ON" } else { "off" }
        ));
        let mut other = game.spends.clone();
        if on {
            other.takedowns.retain(|&n| n != forecast.nemesis);
        } else {
            other.takedowns.push(forecast.nemesis);
        }
        let flipped = rules::forecast(game, &other);
        let alternative = flipped.iter().find(|f| f.nemesis == forecast.nemesis);
        lines.push(format!("    as chosen: {}", forecast_words(game, forecast)));
        if let Some(alternative) = alternative {
            lines.push(format!(
                "    {}: {}",
                if on {
                    "without takedown"
                } else {
                    "with takedown"
                },
                forecast_words(game, alternative)
            ));
        }
    }
    if chosen.is_empty() {
        lines.push("No nemeses. Yet.".to_owned());
    }
    lines.push(format!(
        "[1-3] takedown ${TAKEDOWN_COST}: -{TAKEDOWN_CUT} followers on that nemesis"
    ));
    lines.push(format!(
        "[T] temp cook ${TEMP_COST}: kitchen {} a round tomorrow  {}",
        CAPACITY + 1,
        if game.spends.temp { "ON" } else { "off" }
    ));
    lines.push("ENTER: open the day".to_owned());
    lines
}

/// One forecast, in words.
pub fn forecast_words(game: &Game, forecast: &rules::Forecast) -> String {
    if forecast.defeated {
        return format!("{} followers - GIVES UP tomorrow", forecast.followers);
    }
    let theme = game.nemeses[forecast.nemesis].theme;
    format!(
        "{} followers, tier {}, {} of {PER_DAY} follow, wanting {} {}",
        forecast.followers,
        forecast.tier,
        forecast.count,
        theme.name(),
        forecast.level
    )
}

fn top_bar(game: &Game, panel: &mut Panel<Art>) {
    let (capacity, used) = game.kitchen();
    let line = match game.phase {
        Phase::Service => format!(
            "DAY {}/{DAYS}  ROUND {}/{ROUNDS}  MONEY ${}  RENT ${RENT} tonight  KITCHEN {used}/{capacity}",
            game.day,
            game.round + 1,
            game.money
        ),
        _ => format!("DAY {}/{DAYS}  MONEY ${}", game.day, game.money),
    };
    panel.text(TextRun::new(
        Vec2::new(16.0, 12.0),
        line,
        style(ROW, palette::TEXT),
    ));
}

fn feed(game: &Game, panel: &mut Panel<Art>) {
    let small = style(SMALL, palette::DIM);
    let start = game.feed.len().saturating_sub(6);
    for (row, line) in game.feed[start..].iter().enumerate() {
        let text = clipped(&small, line, DESIGN.x - 32.0);
        panel.text(TextRun::new(
            Vec2::new(16.0, FEED_TOP + row as f32 * (SMALL + 2.0)),
            text,
            small,
        ));
    }
}

/// The service screen.
pub fn service_panel(game: &Game) -> Panel<Art> {
    let mut panel = Panel::default();
    top_bar(game, &mut panel);
    let window = visible_rows(game);
    for (slot, index) in window.clone().enumerate() {
        let top = QUEUE.at + Vec2::new(0.0, slot as f32 * ROW_PITCH);
        let lines = order_lines(game, index);
        let warn = lines[2].contains("SPAWNS");
        for (line, text) in lines.iter().enumerate() {
            let color = match line {
                0 => palette::TEXT,
                2 if warn => palette::ALARM,
                _ => palette::DIM,
            };
            let row = Cell::new(
                top + Vec2::new(0.0, line as f32 * (SMALL + 2.0)),
                QUEUE.width,
            );
            panel.text(row.run(text, style(SMALL, color)));
        }
    }
    let hidden = game.queue.len() - window.len();
    if hidden > 0 {
        let line = format!(
            "  ...rows {}-{} of {}: UP/DOWN to see the rest",
            window.start + 1,
            window.end,
            game.queue.len()
        );
        let at = QUEUE.at + Vec2::new(0.0, VISIBLE_ROWS as f32 * ROW_PITCH);
        panel.text(TextRun::new(at, line, style(SMALL, palette::GOOD)));
    }
    let side: Vec<String> = match game.queue.get(game.selected) {
        Some(order) => match order.who {
            Who::Nemesis(index) => card_lines(game, index, order),
            _ => nemeses_lines(game),
        },
        None => nemeses_lines(game),
    };
    for (row, line) in side.iter().enumerate() {
        let cell = Cell::new(
            CARD.at + Vec2::new(6.0, 8.0 + row as f32 * (SMALL + 3.0)),
            CARD.width - 12.0,
        );
        panel.text(cell.run(line, style(SMALL, palette::TEXT)));
    }
    feed(game, &mut panel);
    panel.text(TextRun::new(
        Vec2::new(16.0, DESIGN.y - SMALL - 6.0),
        "UP/DOWN pick  0 skip  1 standard  2 careful  3 all-out  ENTER cook the round",
        style(SMALL, palette::GOOD),
    ));
    panel
}

/// The active nemeses, for the card column when no nemesis is picked.
pub fn nemeses_lines(game: &Game) -> Vec<String> {
    let mut lines = vec!["NEMESES".to_owned()];
    for index in game.active() {
        let them = &game.nemeses[index];
        lines.push(format!("{} ({})", them.title, them.theme.name()));
        lines.push(format!(
            "  tier {}, {} fans, tally {}/{}",
            rules::tier(them.followers),
            them.followers,
            them.tally,
            rules::TALLY_TO_WIN
        ));
    }
    if lines.len() == 1 {
        lines.push("none yet".to_owned());
    }
    lines
}

/// The ledger screen.
pub fn ledger_panel(game: &Game) -> Panel<Art> {
    let mut panel = Panel::default();
    top_bar(game, &mut panel);
    for (row, line) in ledger_lines(game).iter().enumerate() {
        let cell = Cell::new(
            Vec2::new(16.0, 44.0 + row as f32 * (ROW + 6.0)),
            DESIGN.x - 32.0,
        );
        panel.text(cell.run(line, style(ROW, palette::TEXT)));
    }
    feed(game, &mut panel);
    panel
}

/// The end screen.
pub fn end_panel(game: &Game, won: bool) -> Panel<Art> {
    let mut panel = Panel::default();
    let (title, color) = if won {
        ("THE RESTAURANT SURVIVES", palette::GOOD)
    } else {
        ("CLOSED FOR GOOD", palette::ALARM)
    };
    panel.text(TextRun::new(
        Vec2::new(16.0, 40.0),
        title,
        style(28.0, color),
    ));
    let beaten = game.nemeses.iter().filter(|n| !n.active()).count();
    let lines = [
        format!("day {} of {DAYS}, money ${}", game.day, game.money),
        format!("nemeses made {}, beaten {beaten}", game.nemeses.len()),
        "ENTER: a new run".to_owned(),
    ];
    for (row, line) in lines.iter().enumerate() {
        panel.text(TextRun::new(
            Vec2::new(16.0, 90.0 + row as f32 * (ROW + 6.0)),
            line.clone(),
            style(ROW, palette::TEXT),
        ));
    }
    feed(game, &mut panel);
    panel
}

/// The screen for where the run is.
pub fn screen(game: &Game) -> Panel<Art> {
    match game.phase {
        Phase::Ledger => ledger_panel(game),
        Phase::Service => service_panel(game),
        Phase::Over(won) => end_panel(game, won),
    }
}

/// Draw the current screen, with the selected row's backing and the card.
pub fn draw_the_screen(ctx: &mut DrawCtx) {
    let game = ctx.world.resource::<Game>();
    if game.phase == Phase::Service {
        // A bar beside the picked row, and an outline round the card.
        let slot = game.selected - visible_rows(game).start;
        let top = QUEUE.at + Vec2::new(-10.0, slot as f32 * ROW_PITCH - 2.0);
        ctx.rect(
            Rect::from_min_size(top, Vec2::new(4.0, ROW_PITCH - 6.0)),
            palette::PICKED,
            Depth::layer(layers::BACK),
        );
        let card = Rect::from_min_size(CARD.at, Vec2::new(CARD.width, FEED_TOP - CARD.at.y - 12.0));
        let corners = [
            card.min,
            Vec2::new(card.max.x, card.min.y),
            card.max,
            Vec2::new(card.min.x, card.max.y),
        ];
        for side in 0..4 {
            ctx.line(
                corners[side],
                corners[(side + 1) % 4],
                1.5,
                palette::CARD,
                Depth::layer(layers::BACK),
            );
        }
    }
    screen(game).draw(ctx, &OneToOne, |icon, _| match icon.art {});
}
