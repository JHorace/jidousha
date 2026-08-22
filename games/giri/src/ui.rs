//! The screens: roster sheets, party assembly with its live willingness
//! preview, the resolution report, and the chain's progress (DESIGN.md §7).
//!
//! **giri v1 has no assets at all.** Every pixel here is a tinted quad, a line,
//! or the engine's built-in 5x7 monospace atlas through `ctx.text` — a
//! character "portrait" is a coloured quad with an initial on it. No PNG, no
//! `Assets` store, ASCII only, and every line break explicit.
//!
//! **The layout is free functions**, not numbers buried in draw systems: a
//! check clicks `card_rect(1).center()` through the camera, and the pointer
//! handler hit-tests the same rectangle. One layout, two readers.
//!
//! **Nothing here decides anything.** The willingness arithmetic on screen is
//! `Preview`, which `flow::refresh_preview` fills from the same `assess` the
//! send verb gates on. The UI cannot disagree with the resolution because the
//! UI does not compute.

use jidousha::prelude::*;

use crate::beats::{CHAIN, initial, stat_line};
use crate::constants::Tuning;
use crate::flow::{Flow, Preview, Stage};
use crate::model::Social;
use crate::{HALF_H, HALF_W};

/// Draw bands. Named once so no `layer: 1` appears at a call site.
pub mod layers {
    /// Panels and bars, behind everything.
    pub const PANEL: i16 = -2;
    /// Portrait quads and buttons.
    pub const PIECE: i16 = -1;
    /// Every glyph.
    pub const TEXT: i16 = 1;
}

/// The margin every panel keeps from the camera's edge.
pub const MARGIN: f32 = 0.5;
/// A heading's text size, in world units.
pub const TITLE: f32 = 0.7;
/// Body text.
pub const BODY: f32 = 0.42;
/// The small text sheets and reports are set in.
pub const SMALL: f32 = 0.36;

/// Where the roster column starts.
pub const ROSTER_X: f32 = -HALF_W + MARGIN;
/// How wide a roster card is.
pub const CARD_W: f32 = 9.0;
/// How tall one is.
pub const CARD_H: f32 = 3.0;
/// The gap between two.
pub const CARD_GAP: f32 = 0.25;
/// Where the columns start, below the headline.
pub const CONTENT_TOP: f32 = -HALF_H + 2.6;
/// Where the wide column starts.
pub const MAIN_X: f32 = ROSTER_X + CARD_W + 0.6;
/// How wide it is.
pub const MAIN_W: f32 = HALF_W - MARGIN - MAIN_X;

/// The court's colour. Dark, because every glyph on it is light.
pub const BACKDROP: Color = Color::rgb(0.06, 0.06, 0.08);
/// A panel's fill.
pub const PANEL_FILL: Color = Color::rgba(1.0, 1.0, 1.0, 0.05);
/// A selected card's fill.
pub const PICKED_FILL: Color = Color::rgba(0.45, 0.85, 1.0, 0.16);
/// Ordinary text.
pub const INK: Color = Color::rgb(0.88, 0.90, 0.94);
/// Text about a refusal, a death, or a killing.
pub const WARN: Color = Color::rgb(1.0, 0.45, 0.40);
/// Text about a bond or a payout.
pub const GOOD: Color = Color::rgb(0.55, 0.95, 0.70);
/// The dimmer text of a sheet's second line.
pub const FAINT: Color = Color::rgba(0.88, 0.90, 0.94, 0.62);
/// A button that can be pressed.
pub const BUTTON_LIVE: Color = Color::rgb(0.20, 0.55, 0.40);
/// A button that cannot.
pub const BUTTON_DEAD: Color = Color::rgba(1.0, 1.0, 1.0, 0.10);

/// Where roster card `index` is.
pub fn card_rect(index: usize) -> Rect {
    let top = CONTENT_TOP + index as f32 * (CARD_H + CARD_GAP);
    Rect::from_min_size(Vec2::new(ROSTER_X, top), Vec2::new(CARD_W, CARD_H))
}

/// Where the row for dungeon `index` is.
pub fn dungeon_row_rect(index: usize) -> Rect {
    let top = CONTENT_TOP + 0.7 + index as f32 * 1.5;
    Rect::from_min_size(Vec2::new(MAIN_X, top), Vec2::new(MAIN_W, 1.4))
}

/// Where the willingness preview's first entry is drawn.
///
/// A free function because two readers want it: `draw_assembly` lays the panel
/// out from it, and `verify.rs` counts the glyph run on that row against the
/// string the entry should be. A layout number that only the draw system knows
/// is a layout nothing can check.
pub fn willingness_row_y(dungeons: usize) -> f32 {
    dungeon_row_rect(dungeons).min.y + 1.1
}

/// Where report line `index` is drawn.
pub fn report_row_y(index: usize) -> f32 {
    CONTENT_TOP + 0.9 + index as f32 * 0.55
}

/// Where the send verb is.
pub fn send_button() -> Rect {
    Rect::from_min_size(Vec2::new(MAIN_X, HALF_H - 3.0), Vec2::new(6.0, 1.3))
}

/// Where the continue verb is — the same corner, so a run's clicks are one
/// habit rather than two.
pub fn continue_button() -> Rect {
    Rect::from_min_size(Vec2::new(MAIN_X, HALF_H - 3.0), Vec2::new(6.0, 1.3))
}

/// Break `text` into lines of at most `columns` characters, on spaces.
///
/// `ctx.text` does not wrap — `\n` is the only line break there is — so a game
/// that draws a sentence wraps it itself or draws it off the side of the world.
pub fn wrap(text: &str, columns: usize) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > columns {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines.join("\n")
}

/// How many characters of `size` fit in `width` world units.
///
/// The advance is `size * 7 / 9` (docs/api: every printable character, spaces
/// included, is exactly that wide), so this is the one place the ratio appears.
pub fn columns_in(width: f32, size: f32) -> usize {
    (width / (size * 7.0 / 9.0)) as usize
}

fn style(size: f32, color: Color) -> TextStyle {
    TextStyle {
        size,
        color,
        depth: Depth::layer(layers::TEXT),
    }
}

fn label(ctx: &mut DrawCtx, at: Vec2, text: &str, size: f32, color: Color) {
    ctx.text(at, text, style(size, color));
}

/// The headline: which beat this is, and the dilemma in a sentence.
///
/// Registered **before** `draw_backdrop`, which submits the bar behind it. The
/// bands are what put the bar underneath, not the submission order — where a
/// game's submission order already agrees with its layers, no assertion over a
/// recorded frame can see a band at all.
pub fn draw_headline(ctx: &mut DrawCtx) {
    let flow = ctx.world.resource::<Flow>();
    label(
        ctx,
        Vec2::new(ROSTER_X, -HALF_H + 0.35),
        &headline(flow),
        TITLE,
        INK,
    );
    let sentence = match flow.spec() {
        Some(beat) => wrap(beat.dilemma, columns_in(HALF_W * 2.0 - MARGIN * 2.0, SMALL)),
        None => wrap(
            "The chain is finished. Every beat came out the way its numbers said it would.",
            columns_in(HALF_W * 2.0 - MARGIN * 2.0, SMALL),
        ),
    };
    label(
        ctx,
        Vec2::new(ROSTER_X, -HALF_H + 1.25),
        &sentence,
        SMALL,
        FAINT,
    );
}

/// The headline's text, as one string, so a check can read what is drawn.
pub fn headline(flow: &Flow) -> String {
    match flow.spec() {
        Some(beat) => format!(
            "giri  beat {} of {} - {}",
            flow.beat + 1,
            CHAIN.len(),
            beat.title
        ),
        None => format!("giri  {} of {} beats done", CHAIN.len(), CHAIN.len()),
    }
}

/// The bars and panels every screen sits on.
pub fn draw_backdrop(ctx: &mut DrawCtx) {
    let depth = Depth::layer(layers::PANEL);
    ctx.rect(
        Rect::from_min_size(
            Vec2::new(-HALF_W + 0.2, -HALF_H + 0.2),
            Vec2::new(HALF_W * 2.0 - 0.4, 2.1),
        ),
        PANEL_FILL,
        depth,
    );
    ctx.rect(
        Rect::from_min_size(
            Vec2::new(MAIN_X - 0.2, CONTENT_TOP - 0.2),
            Vec2::new(MAIN_W + 0.4, HALF_H - MARGIN - CONTENT_TOP + 0.4),
        ),
        PANEL_FILL,
        depth,
    );
}

/// Every sheet: every stat and every edge, for everyone (invariant 2).
pub fn draw_roster(ctx: &mut DrawCtx) {
    let social = Social::view(&ctx.world);
    let flow = ctx.world.resource::<Flow>();
    for (index, member) in social.members.iter().enumerate() {
        let card = card_rect(index);
        let picked = flow.party.contains(&member.entity);
        ctx.rect(
            card,
            if picked { PICKED_FILL } else { PANEL_FILL },
            Depth::layer(layers::PANEL),
        );
        let portrait = Rect::from_min_size(
            Vec2::new(card.min.x + 0.3, card.min.y + 0.3),
            Vec2::splat(1.5),
        );
        ctx.rect(
            portrait,
            portrait_tint(index, member.alive),
            Depth::layer(layers::PIECE),
        );
        let letter = initial(member.name).to_string();
        let letter_style = style(1.0, Color::rgb(0.08, 0.08, 0.10));
        label(
            ctx,
            Vec2::new(
                portrait.center().x - letter_style.width_of(&letter) * 0.5,
                portrait.center().y - 0.5,
            ),
            &letter,
            1.0,
            Color::rgb(0.08, 0.08, 0.10),
        );
        let text_x = portrait.max.x + 0.35;
        label(
            ctx,
            Vec2::new(text_x, card.min.y + 0.3),
            member.name,
            BODY,
            if member.alive { INK } else { WARN },
        );
        label(
            ctx,
            Vec2::new(text_x, card.min.y + 0.95),
            &stat_line(member),
            SMALL,
            FAINT,
        );
        label(
            ctx,
            Vec2::new(card.min.x + 0.3, card.min.y + 1.95),
            &regard_line(&social, member.entity),
            SMALL,
            FAINT,
        );
        label(
            ctx,
            Vec2::new(card.min.x + 0.3, card.min.y + 2.45),
            &status_line(&social, member, picked),
            SMALL,
            if member.alive { GOOD } else { WARN },
        );
    }
}

/// A character's outgoing edges, as one line. Absent edges are zero and are not
/// drawn, which is what "sparse" means on a sheet.
pub fn regard_line(social: &Social, who: Entity) -> String {
    let mut parts: Vec<String> = Vec::new();
    for member in &social.members {
        let value = social.regard(who, member.entity);
        if value != 0 {
            parts.push(format!("{} {:+}", member.name, value));
        }
    }
    if parts.is_empty() {
        "regard: none".to_owned()
    } else {
        format!("regard: {}", parts.join("  "))
    }
}

/// What a card says under the numbers.
pub fn status_line(social: &Social, member: &crate::model::Member, picked: bool) -> String {
    match member.killed_by {
        Some(killer) => format!("DEAD - killed by {}", social.name(killer)),
        None if picked => "IN THE PARTY".to_owned(),
        None => String::new(),
    }
}

fn portrait_tint(index: usize, alive: bool) -> Color {
    const WHEEL: [Color; 4] = [
        Color::rgb(0.86, 0.72, 0.42),
        Color::rgb(0.60, 0.78, 0.92),
        Color::rgb(0.78, 0.62, 0.88),
        Color::rgb(0.62, 0.88, 0.68),
    ];
    let tint = WHEEL[index % WHEEL.len()];
    if alive {
        tint
    } else {
        Color::rgba(tint.r * 0.35, tint.g * 0.35, tint.b * 0.35, 1.0)
    }
}

/// The wide column: the jobs and the preview, or the report, or the end.
pub fn draw_main(ctx: &mut DrawCtx) {
    let flow = ctx.world.resource::<Flow>();
    match flow.stage {
        Stage::Assembly => draw_assembly(ctx),
        Stage::Report => draw_report(ctx),
        Stage::Complete => draw_complete(ctx),
    }
}

/// One line of text the wide column draws: where, what, how big, what colour.
///
/// The panel's whole content as data. `draw_main` renders these and `verify.rs`
/// asserts a glyph run of exactly `text.chars().count()` at each `at` - one
/// layout, two readers, and the drawn frame is tied to the string rather than
/// to a count somebody wrote down.
#[derive(Clone, Debug)]
pub struct TextRun {
    /// The top-left of the first character's cell.
    pub at: Vec2,
    /// What it says. ASCII, no line breaks - one run is one row.
    pub text: String,
    /// How tall a line is, in world units.
    pub size: f32,
    /// What it is drawn in.
    pub color: Color,
}

/// A willingness entry, as the panel states it.
pub fn willingness_line(entry: &crate::model::Willingness) -> String {
    format!(
        "{} {} - {}",
        entry.name,
        if entry.joins() { "JOINS" } else { "REFUSES" },
        entry.arithmetic()
    )
}

/// Every row the assembly panel draws.
pub fn assembly_runs(
    beat: &crate::beats::BeatSpec,
    flow: &Flow,
    preview: &Preview,
) -> Vec<TextRun> {
    let mut runs = vec![TextRun {
        at: Vec2::new(MAIN_X, CONTENT_TOP),
        text: "THE JOB - click a name to offer it, click again to take it back".to_owned(),
        size: SMALL,
        color: FAINT,
    }];
    for (index, dungeon) in beat.dungeons.iter().enumerate() {
        let row = dungeon_row_rect(index);
        runs.push(TextRun {
            at: Vec2::new(row.min.x + 0.2, row.min.y + 0.15),
            text: crate::job_line(dungeon),
            size: BODY,
            color: INK,
        });
        runs.push(TextRun {
            at: Vec2::new(row.min.x + 0.2, row.min.y + 0.75),
            text: format!("wants {}", dungeon.requires.describe()),
            size: SMALL,
            color: FAINT,
        });
    }
    let mut y = willingness_row_y(beat.dungeons.len());
    runs.push(TextRun {
        at: Vec2::new(MAIN_X, y - 0.7),
        text: "WILLINGNESS".to_owned(),
        size: SMALL,
        color: FAINT,
    });
    if preview.entries.is_empty() {
        runs.push(TextRun {
            at: Vec2::new(MAIN_X, y),
            text: "nobody offered yet".to_owned(),
            size: SMALL,
            color: FAINT,
        });
    }
    for entry in &preview.entries {
        runs.push(TextRun {
            at: Vec2::new(MAIN_X, y),
            text: willingness_line(entry),
            size: BODY,
            color: if entry.joins() { GOOD } else { WARN },
        });
        y += 0.75;
        for term in &entry.terms {
            runs.push(TextRun {
                at: Vec2::new(MAIN_X + 0.5, y),
                text: format!(
                    "vs {}: regard {:+}, incompat -{}",
                    term.name, term.regard, term.incompat
                ),
                size: SMALL,
                color: FAINT,
            });
            y += 0.5;
        }
    }
    let button = send_button();
    runs.push(TextRun {
        at: Vec2::new(button.min.x + 0.5, button.min.y + 0.35),
        text: "SEND THEM".to_owned(),
        size: BODY,
        color: INK,
    });
    if !preview.blocked.is_empty() {
        runs.push(TextRun {
            at: Vec2::new(button.max.x + 0.5, button.min.y + 0.45),
            text: preview.blocked.clone(),
            size: SMALL,
            color: WARN,
        });
    }
    let _ = flow;
    runs
}

/// Every row the report draws - the story surface, as data.
pub fn report_runs(flow: &Flow) -> Vec<TextRun> {
    let mut runs = vec![TextRun {
        at: Vec2::new(MAIN_X, CONTENT_TOP),
        text: "WHAT HAPPENED, AND THE ARITHMETIC THAT DID IT".to_owned(),
        size: SMALL,
        color: FAINT,
    }];
    for (index, line) in flow.report.iter().enumerate() {
        runs.push(TextRun {
            at: Vec2::new(MAIN_X, report_row_y(index)),
            text: line.clone(),
            // Mechanical narration first, colour second: the killing and the
            // grudge it left read as costs, the payout and the bond as gains.
            color: if line.contains("killed") || line.contains("saw it") {
                WARN
            } else if line.contains("takes") || line.contains("bond") {
                GOOD
            } else {
                INK
            },
            size: SMALL,
        });
    }
    runs.push(TextRun {
        at: Vec2::new(
            continue_button().min.x + 0.5,
            continue_button().min.y + 0.35,
        ),
        text: "CONTINUE".to_owned(),
        size: BODY,
        color: INK,
    });
    runs
}

/// Every row the end-of-chain screen draws.
pub fn complete_runs() -> Vec<TextRun> {
    let mut runs = vec![
        TextRun {
            at: Vec2::new(MAIN_X, CONTENT_TOP + 1.0),
            text: "THE CHAIN IS COMPLETE".to_owned(),
            size: TITLE,
            color: GOOD,
        },
        TextRun {
            at: Vec2::new(
                continue_button().min.x + 0.5,
                continue_button().min.y + 0.35,
            ),
            text: "PLAY IT AGAIN".to_owned(),
            size: BODY,
            color: INK,
        },
    ];
    for (index, line) in wrap(
        "Four beats, no dice, and nothing hidden. Everything that happened was \
         on the sheets before you pressed send.",
        columns_in(MAIN_W, SMALL),
    )
    .lines()
    .enumerate()
    {
        runs.push(TextRun {
            at: Vec2::new(MAIN_X, CONTENT_TOP + 2.2 + index as f32 * SMALL),
            text: line.to_owned(),
            size: SMALL,
            color: FAINT,
        });
    }
    runs
}

fn draw_runs(ctx: &mut DrawCtx, runs: &[TextRun]) {
    for run in runs {
        label(ctx, run.at, &run.text, run.size, run.color);
    }
}

fn draw_assembly(ctx: &mut DrawCtx) {
    let flow = ctx.world.resource::<Flow>();
    let preview = ctx.world.resource::<Preview>();
    let Some(beat) = flow.spec() else {
        return;
    };
    if let Some(row) = beat.dungeons.get(flow.dungeon).map(|_| flow.dungeon) {
        ctx.rect(
            dungeon_row_rect(row),
            PICKED_FILL,
            Depth::layer(layers::PANEL),
        );
    }
    let button = send_button();
    ctx.rect(
        button,
        if preview.can_send {
            BUTTON_LIVE
        } else {
            BUTTON_DEAD
        },
        Depth::layer(layers::PIECE),
    );
    let runs = assembly_runs(beat, flow, preview);
    draw_runs(ctx, &runs);
}

fn draw_report(ctx: &mut DrawCtx) {
    let runs = report_runs(ctx.world.resource::<Flow>());
    draw_button(ctx);
    draw_runs(ctx, &runs);
}

fn draw_complete(ctx: &mut DrawCtx) {
    let runs = complete_runs();
    draw_button(ctx);
    draw_runs(ctx, &runs);
}

fn draw_button(ctx: &mut DrawCtx) {
    ctx.rect(continue_button(), BUTTON_LIVE, Depth::layer(layers::PIECE));
}

/// The constants in effect, on screen for the same reason they are in every
/// verify report: a run nobody can reproduce is not evidence (DESIGN §8a).
pub fn draw_constants(ctx: &mut DrawCtx) {
    let tuning = ctx.world.resource::<Tuning>();
    label(
        ctx,
        Vec2::new(ROSTER_X, HALF_H - MARGIN - 4.0 * SMALL),
        &tuning.readout(),
        SMALL,
        FAINT,
    );
}
