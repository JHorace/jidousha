//! The three screens as data: one `Panel` per frame (`jidousha::ui`), built
//! from `lines.rs`, plus the boxes under it, drawn through an identity
//! mapping — one world unit is one design unit, 960 x 540.
//!
//! Key functions: `screen`, `boxes`, `draw`; key items: `Art`, `Flat`, `CARD`.

use jidousha::prelude::*;
use jidousha::ui::{Icon, Mapping, Panel, TextRun};

use crate::lines::{Tone, card_lines, end_lines, ledger_lines, order_lines};
use crate::sim::{DAYS, Game, Kind, Phase};
use crate::{layers, palette};

/// No pictures: a role with no members.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Art {}

impl Icon for Art {
    fn size_at(self, _scale: f32) -> Vec2 {
        match self {}
    }
}

/// One design unit is one world unit.
pub(crate) struct Flat;

impl Mapping for Flat {
    fn to_world(&self, ui: Vec2) -> Vec2 {
        ui
    }
    fn scale(&self) -> f32 {
        1.0
    }
}

/// Text sizes: the header, and every other row.
const HEADER: f32 = 18.0;
const ROW: f32 = 12.0;
/// Left margin, the first queue row, and how far apart queue rows are.
const LEFT: f32 = 20.0;
const FIRST_ROW: f32 = 48.0;
const ROW_STEP: f32 = 34.0;
/// A row's second line, under its first.
const SECOND_LINE: f32 = 14.0;
/// Ledger lines start here, this far apart.
const LEDGER_TOP: f32 = 44.0;
const LEDGER_STEP: f32 = 14.0;
/// The notice row and the hint row.
const NOTICE_Y: f32 = 500.0;
const HINT_Y: f32 = 520.0;

/// The nemesis card's backing.
pub(crate) const CARD: Rect = Rect {
    min: Vec2::new(160.0, 170.0),
    max: Vec2::new(800.0, 370.0),
};

fn style(size: f32, color: Color, layer: i16) -> TextStyle {
    TextStyle {
        face: Face::BUILT_IN,
        size,
        color,
        depth: Depth { layer, z: 1.0 },
    }
}

fn tone(tone: Tone) -> Color {
    match tone {
        Tone::Plain => palette::TEXT,
        Tone::Faint => palette::FAINT,
        Tone::Warn => palette::SPAWN,
        Tone::Gold => palette::NEMESIS,
        Tone::Safe => palette::SAFE,
    }
}

/// Where queue row `row` starts.
pub(crate) fn row_at(row: usize) -> Vec2 {
    Vec2::new(LEFT, FIRST_ROW + ROW_STEP * row as f32)
}

/// Everything the frame says, as data.
pub(crate) fn screen(game: &Game) -> Panel<Art> {
    let mut panel = Panel::default();
    let chrome = |size, color| style(size, color, layers::CHROME);
    match game.phase {
        Phase::Service { focus } => {
            let header = format!(
                "DAY {} OF {DAYS}   money ${}   rep {}   capacity {} left",
                game.day, game.money, game.rep, game.capacity_left
            );
            panel.text(TextRun::new(
                Vec2::new(LEFT, 12.0),
                header,
                chrome(HEADER, palette::TEXT),
            ));
            for row in 0..game.queue.len() {
                let (first, second, second_tone) = order_lines(game, row);
                let first_color = match game.queue[row].kind {
                    Kind::Nemesis(_) => palette::NEMESIS,
                    _ => palette::TEXT,
                };
                let at = row_at(row);
                panel.text(TextRun::new(at, first, chrome(ROW, first_color)));
                panel.text(TextRun::new(
                    at + Vec2::new(0.0, SECOND_LINE),
                    second,
                    chrome(ROW, tone(second_tone)),
                ));
            }
            let nemesis = focus.and_then(|row| match game.queue.get(row)?.kind {
                Kind::Nemesis(id) => game.find_nemesis(id),
                _ => None,
            });
            if let Some(nemesis) = nemesis {
                let mut card = Panel::default();
                for (index, line) in card_lines(nemesis, game.capacity_left)
                    .into_iter()
                    .enumerate()
                {
                    let (size, y) = if index == 0 {
                        (HEADER, CARD.min.y + 12.0)
                    } else {
                        (ROW, CARD.min.y + 24.0 + 20.0 * index as f32)
                    };
                    card.text(TextRun::new(
                        Vec2::new(CARD.min.x + 12.0, y),
                        line,
                        style(size, palette::NEMESIS, layers::CHROME),
                    ));
                }
                panel.absorb(card.lifted(layers::OVERLAY));
            }
            notice_and_hint(
                &mut panel,
                game,
                if focus.is_some() {
                    "S: serve them   O: overwhelm them   Esc: back to the queue"
                } else {
                    "1-9: serve a row (a nemesis row opens their card)   ENTER: close the kitchen"
                },
            );
        }
        Phase::Ledger => {
            panel.text(TextRun::new(
                Vec2::new(LEFT, 12.0),
                format!("NIGHT {} - LEDGER", game.day),
                chrome(HEADER, palette::TEXT),
            ));
            for (index, (line, line_tone)) in ledger_lines(game).into_iter().enumerate() {
                panel.text(TextRun::new(
                    Vec2::new(LEFT, LEDGER_TOP + LEDGER_STEP * index as f32),
                    line,
                    chrome(ROW, tone(line_tone)),
                ));
            }
            notice_and_hint(
                &mut panel,
                game,
                "1-3: spend on a nemesis   P: prep   ENTER: open tomorrow",
            );
        }
        Phase::Over { won, reason } => {
            for (index, line) in end_lines(game, won, reason).into_iter().enumerate() {
                let size = if index == 0 { HEADER } else { ROW };
                let ink = chrome(
                    size,
                    if index < 2 {
                        palette::NEMESIS
                    } else {
                        palette::TEXT
                    },
                );
                let y = 150.0
                    + if index == 0 {
                        0.0
                    } else {
                        16.0 + 18.0 * index as f32
                    };
                panel.text(TextRun::new(
                    Vec2::new(480.0 - ink.width_of(&line) * 0.5, y),
                    line,
                    ink,
                ));
            }
        }
    }
    panel
}

fn notice_and_hint(panel: &mut Panel<Art>, game: &Game, hint: &str) {
    if let Some(notice) = &game.notice {
        panel.text(TextRun::new(
            Vec2::new(LEFT, NOTICE_Y),
            notice.clone(),
            style(ROW, palette::NOTICE, layers::CHROME),
        ));
    }
    panel.text(TextRun::new(
        Vec2::new(LEFT, HINT_Y),
        hint,
        style(ROW, palette::FAINT, layers::CHROME),
    ));
}

/// The boxes under the words: row separators, and the card's backing.
pub(crate) fn boxes(game: &Game) -> Vec<(Rect, Color, Depth)> {
    let mut all = Vec::new();
    if let Phase::Service { focus } = game.phase {
        for row in 1..game.queue.len() {
            let y = row_at(row).y - 4.0;
            all.push((
                Rect::from_min_size(Vec2::new(LEFT, y), Vec2::new(920.0, 1.0)),
                palette::SEPARATOR,
                Depth::layer(layers::BACK),
            ));
        }
        if focus.is_some() {
            let edge = Rect {
                min: CARD.min - Vec2::splat(2.0),
                max: CARD.max + Vec2::splat(2.0),
            };
            all.push((
                edge,
                palette::CARD_EDGE,
                Depth {
                    layer: layers::OVERLAY,
                    z: -1.0,
                },
            ));
            all.push((
                CARD,
                palette::CARD,
                Depth {
                    layer: layers::OVERLAY,
                    z: 0.0,
                },
            ));
        }
    }
    all
}

/// The one `Draw` system.
pub(crate) fn draw(ctx: &mut DrawCtx) {
    let game = ctx.world.resource::<Game>();
    for (rect, color, depth) in boxes(game) {
        ctx.rect(rect, color, depth);
    }
    screen(game).draw(ctx, &Flat, |icon, _| match icon.art {});
}
