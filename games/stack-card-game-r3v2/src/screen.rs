//! The table: one `Panel` built from the `Flow`, drawn by two Draw systems and
//! judged by the floors and the frame check from that same panel.
//!
//! Chrome is laid out in the kit's 960x540 design space, and the camera shows
//! exactly that rect (`set_the_table`), so one design unit is one world unit and
//! the mapping is the identity, `Flat`. The stack panel prints every item
//! top-first with its place in the resolution order and what it will do, both
//! read off `rules::forecast` — the same function the resolution applies.

use jidousha::prelude::*;
use jidousha::ui::*;

use crate::Flow;
use crate::duel::{Action, Duel, Outcome, Phase};
use crate::rules::{Side, forecast};

/// The design rect's size; the camera shows exactly this.
pub const DESIGN: Vec2 = Vec2::new(960.0, 540.0);

/// Draw bands, low to high.
pub mod layers {
    /// The panels' backgrounds.
    pub const TABLE: i16 = 0;
    /// Every row of the live table.
    pub const CHROME: i16 = 1;
    /// The result banner's backing.
    pub const BANNER_BACK: i16 = 2;
    /// The result banner's rows.
    pub const BANNER: i16 = 3;
}

/// The colours.
pub mod palette {
    use jidousha::prelude::Color;
    /// What the frame clears to.
    pub const TABLE: Color = Color::rgb(0.05, 0.06, 0.10);
    /// The panels' backgrounds.
    pub const PANEL: Color = Color::rgb(0.10, 0.12, 0.18);
    /// Ordinary text.
    pub const INK: Color = Color::WHITE;
    /// What cannot be acted on.
    pub const DIM: Color = Color::rgb(0.45, 0.47, 0.55);
    /// Your items and seat.
    pub const YOURS: Color = Color::rgb(0.45, 0.90, 1.0);
    /// The Rival's items and seat.
    pub const THEIRS: Color = Color::rgb(1.0, 0.50, 0.45);
    /// Legal targets, and the hint while aiming.
    pub const MARK: Color = Color::rgb(1.0, 0.85, 0.25);
}

/// This game's picture roles: none. Every shape is a rect and every word is text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Art {}

impl Icon for Art {
    fn size_at(self, _scale: f32) -> Vec2 {
        match self {}
    }
}

/// The identity mapping: the camera shows the design rect, so a design point is
/// a world point. (The UI doc's `judge_frame` example names a `Flat` it never
/// defines — FINDINGS.md G-1.)
pub struct Flat;

impl Mapping for Flat {
    fn to_world(&self, ui: Vec2) -> Vec2 {
        ui
    }
    fn scale(&self) -> f32 {
        1.0
    }
}

/// The floors: 12-unit text, everything inside the design rect.
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

/// A row's text size.
pub const ROW: f32 = 14.0;
/// A title's text size.
pub const TITLE: f32 = 18.0;
/// The result banner's text size.
pub const BANNER: f32 = 32.0;
/// Rows of a list sit this far apart.
pub const LEADING: f32 = 20.0;

/// The stack panel's background.
pub const STACK_PANEL: Rect = Rect {
    min: Vec2::new(20.0, 64.0),
    max: Vec2::new(570.0, 396.0),
};
/// The hand panel's background.
pub const HAND_PANEL: Rect = Rect {
    min: Vec2::new(590.0, 64.0),
    max: Vec2::new(940.0, 396.0),
};
/// The result banner's backing.
pub const RESULT_BACK: Rect = Rect {
    min: Vec2::new(200.0, 196.0),
    max: Vec2::new(760.0, 332.0),
};
/// Where the first stack row and the first hand row start, as row origins.
pub const STACK_ROWS: Vec2 = Vec2::new(20.0, 96.0);
/// The hand's first row origin.
pub const HAND_ROWS: Vec2 = Vec2::new(590.0, 96.0);
/// The priority line.
pub const PRIORITY_AT: Vec2 = Vec2::new(28.0, 408.0);
/// The first of the three log rows.
pub const LOG_AT: Vec2 = Vec2::new(28.0, 432.0);
/// The log's leading.
pub const LOG_LEADING: f32 = 18.0;
/// The hint row.
pub const HINT_AT: Vec2 = Vec2::new(28.0, 500.0);

/// The stack row's cells, as offsets from the row.
pub mod cells {
    use jidousha::prelude::Vec2;
    use jidousha::ui::Cell;
    /// ` `, `+` legal, `>` the cursor.
    pub const MARK: Cell = Cell {
        at: Vec2::new(8.0, 0.0),
        width: 14.0,
    };
    /// Its place in the resolution order, `x` voided, `-` not reached.
    pub const ORDER: Cell = Cell {
        at: Vec2::new(24.0, 0.0),
        width: 24.0,
    };
    /// `#n`.
    pub const ID: Cell = Cell {
        at: Vec2::new(50.0, 0.0),
        width: 36.0,
    };
    /// The card and its owner.
    pub const CARD: Cell = Cell {
        at: Vec2::new(90.0, 0.0),
        width: 160.0,
    };
    /// `-> #n`, for a stack card.
    pub const TARGET: Cell = Cell {
        at: Vec2::new(254.0, 0.0),
        width: 66.0,
    };
    /// What it will do.
    pub const SAYS: Cell = Cell {
        at: Vec2::new(324.0, 0.0),
        width: 220.0,
    };
    /// The hand's slot number.
    pub const SLOT: Cell = Cell {
        at: Vec2::new(8.0, 0.0),
        width: 20.0,
    };
    /// The hand's card name.
    pub const NAME: Cell = Cell {
        at: Vec2::new(30.0, 0.0),
        width: 90.0,
    };
    /// The hand's cost.
    pub const COST: Cell = Cell {
        at: Vec2::new(124.0, 0.0),
        width: 20.0,
    };
    /// The hand's blurb.
    pub const BLURB: Cell = Cell {
        at: Vec2::new(150.0, 0.0),
        width: 180.0,
    };
}

/// The controls line, shown in the hint row when nothing is being aimed.
pub const CONTROLS: &str = "1-6 play   Up/Down target   Enter commit   Esc cancel   Space pass";

/// A style on the live table's band.
pub fn style(size: f32, color: Color) -> TextStyle {
    TextStyle {
        face: Face::BUILT_IN,
        size,
        color,
        depth: Depth::layer(layers::CHROME),
    }
}

/// The colour a seat's things are drawn in.
pub fn seat_color(side: Side) -> Color {
    match side {
        Side::You => palette::YOURS,
        Side::Rival => palette::THEIRS,
    }
}

/// The line under the stack: who holds priority and what a pass would do.
pub fn priority_line(flow: &Flow) -> String {
    let duel = &flow.duel;
    if let Phase::Over(_) = duel.phase {
        return "MATCH OVER".to_owned();
    }
    if duel.priority == Side::Rival {
        return "RIVAL HOLDS PRIORITY - thinking".to_owned();
    }
    if let Some((card, _)) = flow.aiming() {
        return format!("CHOOSE A TARGET for {} - Up/Down, Enter, Esc", card.name());
    }
    let pass = match (duel.passes, duel.stack.last()) {
        (0, _) => "Rival may still respond".to_owned(),
        (_, Some(top)) => format!("Rival passed: Space resolves {}", top.id),
        (_, None) => "Rival passed: Space ends the turn".to_owned(),
    };
    format!("YOU HOLD PRIORITY - {pass}")
}

/// The hint row: what the aimed play would do, or the controls.
pub fn hint_line(flow: &Flow) -> (String, Color) {
    let Some(choosing) = flow.choosing else {
        return (CONTROLS.to_owned(), palette::DIM);
    };
    let Some((card, targets)) = flow.aiming() else {
        return (CONTROLS.to_owned(), palette::DIM);
    };
    let Some(&target) = targets.get(choosing.cursor) else {
        return (CONTROLS.to_owned(), palette::DIM);
    };
    let mut after = flow.duel.clone();
    let play = Action::Play {
        slot: choosing.slot,
        target: Some(target),
    };
    let first = match after.apply(play) {
        Ok(()) => forecast(&after)
            .steps
            .first()
            .map_or_else(String::new, |step| step.says.clone()),
        Err(refused) => refused.to_string(),
    };
    (
        format!("{} on {target}: {first}", card.name()),
        palette::MARK,
    )
}

/// The result banner's first line, or `None` while the match is live.
pub fn banner_line(duel: &Duel) -> Option<String> {
    let Phase::Over(outcome) = duel.phase else {
        return None;
    };
    let word = match outcome {
        Outcome::YouWin => "YOU WIN",
        Outcome::RivalWins => "RIVAL WINS",
        Outcome::Draw => "DRAW",
    };
    let [you, rival] = duel.life();
    Some(format!("{word}  {you} - {rival}"))
}

/// The second banner line.
pub const AGAIN: &str = "Enter: play again";

/// Every row the screen draws, as data: the one reader of the flow.
pub fn screen(flow: &Flow) -> Panel<Art> {
    let duel = &flow.duel;
    let mut panel: Panel<Art> = Panel::default();
    let whose = match duel.active {
        Side::You => "YOUR TURN",
        Side::Rival => "RIVAL'S TURN",
    };
    let header = format!("TURN {} of {} - {whose}", duel.turn, crate::duel::LAST_TURN);
    panel.text(TextRun::new(
        Vec2::new(20.0, 12.0),
        header,
        style(TITLE, palette::INK),
    ));
    for (side, x, label) in [(Side::You, 20.0, "YOU  "), (Side::Rival, 590.0, "RIVAL")] {
        let seat = duel.seat(side);
        let line = format!("{label}  life {}   energy {}", seat.life, seat.energy);
        panel.text(TextRun::new(
            Vec2::new(x, 36.0),
            line,
            style(ROW, seat_color(side)),
        ));
    }
    stack_rows(flow, &mut panel);
    hand_rows(flow, &mut panel);
    panel.text(TextRun::new(
        PRIORITY_AT,
        priority_line(flow),
        style(ROW, palette::INK),
    ));
    let shown = duel.log.len().saturating_sub(3);
    for (row, line) in duel.log[shown..].iter().enumerate() {
        let at = LOG_AT + Vec2::new(0.0, row as f32 * LOG_LEADING);
        panel.text(TextRun::new(at, line.clone(), style(ROW, palette::DIM)));
    }
    let (hint, tone) = hint_line(flow);
    panel.text(TextRun::new(HINT_AT, hint, style(ROW, tone)));
    if let Some(line) = banner_line(duel) {
        let mut banner: Panel<Art> = Panel::default();
        let big = style(BANNER, palette::INK);
        let small = style(TITLE, palette::INK);
        let back = RESULT_BACK;
        banner.text(TextRun::new(centered(back, &big, &line, 220.0), line, big));
        banner.text(TextRun::new(
            centered(back, &small, AGAIN, 284.0),
            AGAIN,
            small,
        ));
        panel.absorb(banner.lifted(layers::BANNER));
    }
    panel
}

/// The stack panel: its title and one row per item, top-first.
fn stack_rows(flow: &Flow, panel: &mut Panel<Art>) {
    let duel = &flow.duel;
    let title = "STACK - top resolves first";
    panel.text(TextRun::new(
        Vec2::new(28.0, 70.0),
        title,
        style(TITLE, palette::INK),
    ));
    if duel.stack.is_empty() {
        let empty = cells::MARK.at(STACK_ROWS);
        let empty = Cell::new(empty.at, 540.0)
            .run("(empty - nothing to resolve)", style(ROW, palette::DIM));
        panel.text(empty);
        return;
    }
    let aiming = flow.aiming();
    let cursor = flow.choosing.map(|choosing| choosing.cursor);
    let fates = forecast(duel).fates;
    for (row, (item, fate)) in duel.stack.iter().rev().zip(&fates).enumerate() {
        let origin = STACK_ROWS + Vec2::new(0.0, row as f32 * LEADING);
        let (mark, tone) = match &aiming {
            None => (" ", seat_color(item.owner)),
            Some((_, legal)) => match legal.iter().position(|id| *id == item.id) {
                Some(at) if Some(at) == cursor => (">", palette::MARK),
                Some(_) => ("+", palette::MARK),
                None => (" ", palette::DIM),
            },
        };
        let ink = style(ROW, tone);
        if mark != " " {
            panel.text(cells::MARK.at(origin).run(mark, ink));
        }
        panel.text(cells::ORDER.at(origin).run(&fate.order_cell(), ink));
        panel.text(cells::ID.at(origin).run(&item.id.to_string(), ink));
        let card = format!("{} {}", item.card.name(), item.owner.name());
        panel.text(cells::CARD.at(origin).run(&card, ink));
        if let Some(target) = item.target {
            panel.text(cells::TARGET.at(origin).run(&format!("-> {target}"), ink));
        }
        panel.text(cells::SAYS.at(origin).run(&fate.says, ink));
    }
}

/// Whether Your hand slot is one a key would play right now.
pub fn slot_live(flow: &Flow, slot: usize) -> bool {
    let duel = &flow.duel;
    duel.phase == Phase::Live && duel.priority == Side::You && duel.playable(Side::You, slot)
}

/// The hand panel: its title and one row per card.
fn hand_rows(flow: &Flow, panel: &mut Panel<Art>) {
    let title = "HAND - 1-6 plays";
    panel.text(TextRun::new(
        Vec2::new(598.0, 70.0),
        title,
        style(TITLE, palette::INK),
    ));
    for (slot, card) in flow.duel.seat(Side::You).hand.iter().enumerate() {
        let origin = HAND_ROWS + Vec2::new(0.0, slot as f32 * LEADING);
        let tone = if slot_live(flow, slot) {
            palette::INK
        } else {
            palette::DIM
        };
        let ink = style(ROW, tone);
        panel.text(cells::SLOT.at(origin).run(&(slot + 1).to_string(), ink));
        panel.text(cells::NAME.at(origin).run(card.name(), ink));
        panel.text(cells::COST.at(origin).run(&card.cost().to_string(), ink));
        panel.text(cells::BLURB.at(origin).run(card.blurb(), ink));
    }
}

/// The panels' backgrounds, and the banner's backing once the match is over.
pub fn draw_table(ctx: &mut DrawCtx) {
    let flow = ctx.world.resource::<Flow>();
    for rect in [STACK_PANEL, HAND_PANEL] {
        ctx.rect(rect, palette::PANEL, Depth::layer(layers::TABLE));
    }
    if banner_line(&flow.duel).is_some() {
        ctx.rect(
            RESULT_BACK,
            palette::TABLE,
            Depth::layer(layers::BANNER_BACK),
        );
    }
}

/// Every row of `screen`, through the identity mapping.
pub fn draw_chrome(ctx: &mut DrawCtx) {
    let panel = screen(ctx.world.resource::<Flow>());
    panel.draw(ctx, &Flat, |icon, _| match icon.art {});
}
