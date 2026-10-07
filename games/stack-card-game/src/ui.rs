//! The screen, as a model. `build` turns the game into rectangles and labels;
//! `draw` submits them. A check calls the same `build` and reads the strings
//! the player reads, so what it asserts is what is on screen.

use crate::cards::{Card, Side, info};
use crate::game::{Game, SLOT_KEYS};
use crate::rules::{Fate, Item, fate, preview};
use jidousha::prelude::*;

pub const VIEW_HEIGHT: f32 = 20.0;
pub const TEXT: f32 = 0.58;
const LINE: f32 = 0.95;
const MARGIN: f32 = 0.8;

pub mod palette {
    use jidousha::prelude::Color;
    pub const BACKDROP: Color = Color::rgb(0.06, 0.07, 0.10);
    pub const PANEL: Color = Color::rgb(0.11, 0.13, 0.19);
    pub const TEXT: Color = Color::rgb(0.88, 0.90, 0.95);
    pub const DIM: Color = Color::rgb(0.55, 0.60, 0.70);
    pub const YOU: Color = Color::rgb(0.45, 0.85, 1.0);
    pub const NPC: Color = Color::rgb(1.0, 0.62, 0.45);
    pub const MARK: Color = Color::rgb(1.0, 0.85, 0.3);
    pub const DAMAGE: Color = Color::rgb(0.55, 0.18, 0.18);
    pub const HEAL: Color = Color::rgb(0.18, 0.45, 0.25);
    pub const COUNTER: Color = Color::rgb(0.18, 0.30, 0.58);
    pub const ORDER: Color = Color::rgb(0.55, 0.45, 0.14);
    pub const TWIST: Color = Color::rgb(0.42, 0.22, 0.55);
}

mod layer {
    pub const PANEL: i16 = 0;
    pub const CARD: i16 = 1;
    pub const TEXT: i16 = 2;
    pub const OVERLAY: i16 = 4;
}

#[derive(Clone, Debug)]
pub struct Block {
    pub rect: Rect,
    pub color: Color,
    pub layer: i16,
}

/// A line of text, where it lands, and the box it must stay inside.
#[derive(Clone, Debug)]
pub struct Label {
    pub at: Vec2,
    pub text: String,
    pub size: f32,
    pub color: Color,
    pub layer: i16,
    pub clip: Rect,
}

impl Label {
    pub fn bounds(&self) -> Rect {
        let style = TextStyle {
            size: self.size,
            ..TextStyle::default()
        };
        Rect::from_min_size(self.at, style.measure(&self.text).size)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mark {
    None,
    Legal,
    Cursor,
}

/// One stack item as the player reads it.
#[derive(Clone, Debug)]
pub struct StackRow {
    pub left: String,
    pub right: String,
    pub mark: Mark,
    pub rect: Rect,
}

#[derive(Clone, Debug, Default)]
pub struct Screen {
    pub blocks: Vec<Block>,
    pub labels: Vec<Label>,
    pub stack_rows: Vec<StackRow>,
    pub status: String,
    pub priority_line: String,
    pub log_lines: Vec<String>,
    pub stack_panel: Option<Rect>,
    pub hand_rects: Vec<Rect>,
}

fn kind_color(card: Card) -> Color {
    match card {
        Card::Ember | Card::Cleaver | Card::Siege => palette::DAMAGE,
        Card::Mend => palette::HEAL,
        Card::Negate | Card::Hush => palette::COUNTER,
        Card::Flip | Card::Bury | Card::Raise => palette::ORDER,
        Card::Redirect | Card::Echo => palette::TWIST,
    }
}

fn side_color(side: Side) -> Color {
    match side {
        Side::You => palette::YOU,
        Side::Npc => palette::NPC,
    }
}

/// What an item does, in the few characters a row has.
pub fn describe(item: &Item) -> String {
    let data = info(item.card);
    let aim = item.aim.map_or("", Side::name);
    if data.damage > 0 {
        format!("{}>{aim}", data.damage)
    } else if data.heal > 0 {
        format!("+{} {aim}", data.heal)
    } else if let Some(target) = item.target {
        format!("->@{}", target.0)
    } else {
        "reverse".to_owned()
    }
}

/// Greedy word wrap to `columns` characters.
pub fn wrap(text: &str, columns: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.len() + 1 + word.len() <= columns => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_owned()),
        }
    }
    lines
}

struct Painter {
    screen: Screen,
}

impl Painter {
    fn rect(&mut self, rect: Rect, color: Color, layer: i16) {
        self.screen.blocks.push(Block { rect, color, layer });
    }

    fn text(&mut self, clip: Rect, at: Vec2, text: &str, size: f32, color: Color) {
        self.screen.labels.push(Label {
            at,
            text: text.to_owned(),
            size,
            color,
            layer: layer::TEXT,
            clip,
        });
    }
}

fn width_of(text: &str, size: f32) -> f32 {
    TextStyle {
        size,
        ..TextStyle::default()
    }
    .width_of(text)
}

/// The prompt a player needs: who holds priority, and what they can do.
pub fn priority_line(game: &Game) -> String {
    let core = &game.core;
    if let Some(targeting) = &game.ui.targeting {
        return format!(
            "TARGET: Up/Down, Enter ok, Esc cancel ({} legal)",
            targeting.legal.len()
        );
    }
    match core.priority {
        Side::You if core.stack.is_empty() => {
            "YOUR PRIORITY: 1-7 pick, Enter play, Space pass".to_owned()
        }
        Side::You => "YOUR PRIORITY: respond, or Space to resolve".to_owned(),
        Side::Npc => "NPC HAS PRIORITY: it may still respond".to_owned(),
    }
}

/// Build the whole screen for `view` (the camera's visible rectangle).
pub fn build(game: &Game, view: Rect) -> Screen {
    let core = &game.core;
    let mut p = Painter {
        screen: Screen::default(),
    };
    let top = view.min.y;
    let left = view.min.x + MARGIN;
    let inner_w = view.size().x - 2.0 * MARGIN;
    let steps = preview(core);

    // --- header: the opponent, and whose turn it is -------------------
    let head = Rect::from_min_size(Vec2::new(left, top + 0.3), Vec2::new(inner_w, 1.4));
    let npc = format!(
        "NPC  life {}  mana {}  hand {}  deck {}",
        core.life[1],
        core.mana[1],
        core.hands[1].len(),
        core.decks[1].len()
    );
    p.text(head, head.min, &npc, TEXT, palette::NPC);
    let turn = format!("turn {}/24  active {}", core.turn, core.active.name());
    p.text(
        head,
        Vec2::new(head.max.x - width_of(&turn, TEXT), head.min.y),
        &turn,
        TEXT,
        palette::DIM,
    );

    // --- the stack panel ---------------------------------------------
    let panel_top = top + 2.2;
    let panel_h = 8.4;
    let panel_w = inner_w * 0.62;
    let panel = Rect::from_min_size(Vec2::new(left, panel_top), Vec2::new(panel_w, panel_h));
    p.rect(panel, palette::PANEL, layer::PANEL);
    p.screen.stack_panel = Some(panel);
    p.text(
        panel,
        panel.min + Vec2::new(0.4, 0.2),
        "STACK top first",
        TEXT,
        palette::DIM,
    );
    let hdr = "resolve YOU-NPC";
    p.text(
        panel,
        Vec2::new(panel.max.x - 0.4 - width_of(hdr, TEXT), panel.min.y + 0.2),
        hdr,
        TEXT,
        palette::DIM,
    );
    let rows_max = ((panel_h - 1.4) / LINE) as usize;
    // A stack deeper than the panel says so, rather than dropping its bottom.
    let shown = if core.stack.len() > rows_max {
        rows_max - 1
    } else {
        core.stack.len()
    };
    if core.stack.len() > shown {
        let more = format!("(+{} deeper, they resolve last)", core.stack.len() - shown);
        let y = panel.min.y + 1.3 + shown as f32 * LINE;
        p.text(
            panel,
            Vec2::new(panel.min.x + 1.1, y),
            &more,
            TEXT,
            palette::DIM,
        );
    }
    for (row, item) in core.stack.iter().rev().enumerate().take(shown) {
        let y = panel.min.y + 1.3 + row as f32 * LINE;
        let rect = Rect::from_min_size(
            Vec2::new(panel.min.x + 0.2, y - 0.1),
            Vec2::new(panel_w - 0.4, LINE - 0.05),
        );
        let mark = match &game.ui.targeting {
            Some(t) if t.current() == item.id => Mark::Cursor,
            Some(t) if t.legal.contains(&item.id) => Mark::Legal,
            _ => Mark::None,
        };
        match mark {
            Mark::Cursor => p.rect(rect, Color::rgba(1.0, 0.85, 0.3, 0.45), layer::PANEL + 1),
            Mark::Legal => p.rect(rect, Color::rgba(1.0, 0.85, 0.3, 0.16), layer::PANEL + 1),
            Mark::None => {}
        }
        let left_text = format!(
            "@{} {} {} {}",
            item.id.0,
            item.owner.name(),
            info(item.card).name,
            describe(item)
        );
        let right_text = match fate(&steps, item.id) {
            Some(Fate::Resolves(k)) => {
                let after = steps[k - 1].life_after;
                format!("#{k} {}-{}", after[0], after[1])
            }
            Some(Fate::Countered(k)) => format!("x by #{k}"),
            None => "-".to_owned(),
        };
        let marker = match mark {
            Mark::Cursor => ">",
            Mark::Legal => "*",
            Mark::None => " ",
        };
        p.text(
            panel,
            Vec2::new(panel.min.x + 0.35, y),
            marker,
            TEXT,
            palette::MARK,
        );
        p.text(
            panel,
            Vec2::new(panel.min.x + 1.1, y),
            &left_text,
            TEXT,
            side_color(item.owner),
        );
        p.text(
            panel,
            Vec2::new(panel.max.x - 0.4 - width_of(&right_text, TEXT), y),
            &right_text,
            TEXT,
            palette::TEXT,
        );
        p.screen.stack_rows.push(StackRow {
            left: left_text,
            right: right_text,
            mark,
            rect,
        });
    }
    if core.stack.is_empty() {
        p.text(
            panel,
            panel.min + Vec2::new(0.4, 1.4),
            "(empty - the stack resolves top first)",
            TEXT,
            palette::DIM,
        );
    }

    // --- the log ------------------------------------------------------
    let log = Rect::from_min_size(
        Vec2::new(panel.max.x + 0.4, panel_top),
        Vec2::new(inner_w - panel_w - 0.4, panel_h),
    );
    p.rect(log, palette::PANEL, layer::PANEL);
    let columns = TextStyle {
        size: TEXT,
        ..TextStyle::default()
    }
    .columns_in(log.size().x - 0.8);
    let slots = ((panel_h - 0.4) / LINE) as usize;
    let mut wrapped: Vec<String> = Vec::new();
    for line in core.log.iter().rev() {
        let mut parts = wrap(line, columns);
        parts.reverse();
        wrapped.extend(parts);
        if wrapped.len() >= slots {
            break;
        }
    }
    wrapped.truncate(slots);
    wrapped.reverse();
    for (row, line) in wrapped.iter().enumerate() {
        p.text(
            log,
            log.min + Vec2::new(0.4, 0.3 + row as f32 * LINE),
            line,
            TEXT,
            palette::DIM,
        );
    }
    p.screen.log_lines = wrapped;

    // --- status: priority, and the last refusal -----------------------
    let status = Rect::from_min_size(Vec2::new(left, top + 10.8), Vec2::new(inner_w, 2.5));
    let prompt = priority_line(game);
    p.text(status, status.min, &prompt, TEXT, palette::MARK);
    let columns = TextStyle {
        size: TEXT,
        ..TextStyle::default()
    }
    .columns_in(inner_w);
    for (row, line) in wrap(&game.ui.message, columns).iter().enumerate() {
        p.text(
            status,
            status.min + Vec2::new(0.0, LINE * (row + 1) as f32),
            line,
            TEXT,
            palette::NPC,
        );
    }
    p.screen.priority_line = prompt;
    p.screen.status = game.ui.message.clone();

    // --- you, and your hand -------------------------------------------
    let me = Rect::from_min_size(Vec2::new(left, top + 13.4), Vec2::new(inner_w, 1.2));
    let you = format!(
        "YOU  life {}  mana {}  hand {}  deck {}",
        core.life[0],
        core.mana[0],
        core.hands[0].len(),
        core.decks[0].len()
    );
    p.text(me, me.min, &you, TEXT, palette::YOU);

    let hand = &core.hands[0];
    let gap = 0.4;
    let slot_w = (inner_w - gap * (SLOT_KEYS.len() as f32 - 1.0)) / SLOT_KEYS.len() as f32;
    let card_h = 4.6;
    let card_top = top + 14.8;
    let size = TEXT.min((slot_w - 0.4) / (0.7778 * 8.0));
    let cols = TextStyle {
        size,
        ..TextStyle::default()
    }
    .columns_in(slot_w - 0.4);
    for (slot, &card) in hand.iter().enumerate() {
        let data = info(card);
        let x = left + slot as f32 * (slot_w + gap);
        let rect = Rect::from_min_size(Vec2::new(x, card_top), Vec2::new(slot_w, card_h));
        let playable = core.options(Side::You, slot).is_ok();
        let selected = game.ui.selected == Some(slot);
        if selected {
            p.rect(
                Rect {
                    min: rect.min - Vec2::splat(0.18),
                    max: rect.max + Vec2::splat(0.18),
                },
                palette::MARK,
                layer::CARD - 1,
            );
        }
        let base = kind_color(card);
        let body = if playable {
            base
        } else {
            Color::rgb(base.r * 0.4, base.g * 0.4, base.b * 0.4)
        };
        p.rect(rect, body, layer::CARD);
        let ink = if playable {
            palette::TEXT
        } else {
            palette::DIM
        };
        let mut y = rect.min.y + 0.25;
        let mut put = |p: &mut Painter, text: &str, color: Color| {
            p.text(rect, Vec2::new(rect.min.x + 0.2, y), text, size, color);
            y += size * 1.3;
        };
        put(&mut p, &format!("[{}] c{}", slot + 1, data.cost), ink);
        put(&mut p, data.name, ink);
        put(
            &mut p,
            if data.sorcery { "sorcery" } else { "instant" },
            palette::DIM,
        );
        for part in wrap(data.text, cols) {
            put(&mut p, &part, ink);
        }
        p.screen.hand_rects.push(rect);
    }

    // --- the end ------------------------------------------------------
    if let Some(result) = game.result_line() {
        p.rect(view, Color::rgba(0.0, 0.0, 0.0, 0.7), layer::OVERLAY);
        let big = 2.2;
        let hint = "Enter or Space: new match";
        let mid = view.center();
        let clip = view;
        p.screen.labels.push(Label {
            at: Vec2::new(mid.x - width_of(result, big) * 0.5, mid.y - 2.0),
            text: result.to_owned(),
            size: big,
            color: palette::TEXT,
            layer: layer::OVERLAY + 1,
            clip,
        });
        p.screen.labels.push(Label {
            at: Vec2::new(mid.x - width_of(hint, 0.8) * 0.5, mid.y + 1.0),
            text: hint.to_owned(),
            size: 0.8,
            color: palette::DIM,
            layer: layer::OVERLAY + 1,
            clip,
        });
    }
    p.screen
}

/// Submit a built screen.
pub fn draw(ctx: &mut DrawCtx) {
    let view = ctx.world.resource::<Camera>().visible_bounds();
    let screen = build(ctx.world.resource::<Game>(), view);
    for block in &screen.blocks {
        ctx.rect(block.rect, block.color, Depth::layer(block.layer));
    }
    for label in &screen.labels {
        ctx.text(
            label.at,
            &label.text,
            TextStyle {
                size: label.size,
                color: label.color,
                depth: Depth::layer(label.layer),
                ..TextStyle::default()
            },
        );
    }
}
