//! What every screen draws with: text as data, panels, buttons, and icons.
//!
//! **Nothing here decides anything.** The arithmetic on screen is `Preview`,
//! which `flow::refresh_preview` fills from the same `assess` and `admit` the
//! send verb and the click handler gate on. The UI cannot disagree with the
//! resolution because the UI does not compute (DESIGN invariant 2; ADR-0039).
//!
//! **Every screen hands back its content as data first, and draws it second.**
//! A `Panel` is every string and every icon a screen puts on the frame, with
//! its position — so `verify.rs` can assert a glyph run of exactly
//! `text.chars().count()` at each `at`, `contracts.rs` can check every string
//! is ASCII the font can draw, and `floors.rs` can check UI.md §7's floors
//! against the same values the frame was built from. Three readers, one layout.
//!
//! Text sizes and colours come from `theme.rs`; every rectangle from
//! `layout.rs`. Nothing below names a number of its own.

use jidousha::prelude::*;

use crate::sprites::{Art, Gallery};
use crate::theme;

/// One row of text a screen draws.
#[derive(Clone, Debug)]
pub struct TextRun {
    /// The top-left of the first character's cell.
    pub at: Vec2,
    /// What it says. ASCII, no line breaks - one run is one row.
    pub text: String,
    /// How tall a line is, in world units (which are reference pixels).
    pub size: f32,
    /// What it is drawn in.
    pub color: Color,
    /// Which text band. The board's, or an overlay's.
    pub layer: i16,
}

impl TextRun {
    /// A row on the board's text band.
    pub fn new(at: Vec2, text: impl Into<String>, size: f32, color: Color) -> Self {
        Self {
            at,
            text: text.into(),
            size,
            color,
            layer: theme::layers::TEXT,
        }
    }

    /// The same, on an overlay's.
    pub fn over(at: Vec2, text: impl Into<String>, size: f32, color: Color) -> Self {
        Self {
            layer: theme::layers::OVERLAY_TEXT,
            ..Self::new(at, text, size, color)
        }
    }

    /// The rectangle this row's glyphs occupy.
    pub fn bounds(&self) -> Rect {
        let style = theme::text(self.size, self.color);
        Rect::from_min_size(self.at, Vec2::new(style.width_of(&self.text), self.size))
    }
}

/// One icon a screen draws.
#[derive(Clone, Copy, Debug)]
pub struct IconRun {
    /// The top-left corner.
    pub at: Vec2,
    /// Which role.
    pub art: Art,
    /// Texels per texel. **Integer**, always: the engine samples nearest and a
    /// fractional scale puts a wobble in pixel art (UI.md §1.4).
    pub scale: f32,
    /// Multiplied into the picture.
    pub tint: Color,
    /// Which band.
    pub layer: i16,
}

impl IconRun {
    /// An icon on the board's piece band, untinted.
    pub fn new(at: Vec2, art: Art, scale: f32) -> Self {
        Self {
            at,
            art,
            scale,
            tint: Color::WHITE,
            layer: theme::layers::PIECE,
        }
    }

    /// The rectangle it covers.
    pub fn bounds(&self) -> Rect {
        Rect::from_min_size(self.at, self.art.size_at(self.scale))
    }
}

/// Everything one screen puts on the frame, as data.
///
/// Two spaces, two lists: `runs` and `icons` are the chrome, in **UI units**
/// (960x540 reference pixels), placed by `camera::UiMap` at draw time so the
/// chrome stays a constant size on screen whatever the camera does;
/// `world_runs` and `world_icons` are in **world units** — location labels
/// and anything else that pans with the map.
#[derive(Clone, Debug, Default)]
pub struct Panel {
    /// Every row of chrome text, in UI units.
    pub runs: Vec<TextRun>,
    /// Every chrome icon, in UI units.
    pub icons: Vec<IconRun>,
    /// Every row of map-space text, in world units.
    pub world_runs: Vec<TextRun>,
    /// Every map-space icon, in world units.
    pub world_icons: Vec<IconRun>,
}

impl Panel {
    /// Add a row.
    pub fn text(&mut self, run: TextRun) -> &mut Self {
        self.runs.push(run);
        self
    }

    /// Add an icon.
    pub fn icon(&mut self, run: IconRun) -> &mut Self {
        self.icons.push(run);
        self
    }

    /// Add a block of text, one row per line, advancing by `size`.
    ///
    /// `ctx.text` honours `\n`, but a wrapped block submitted as one call is
    /// one row to every assertion that counts glyphs on a row - so a block is
    /// stored as the rows it is, and the count stays exact.
    pub fn block(&mut self, at: Vec2, text: &str, size: f32, color: Color) -> f32 {
        let mut y = at.y;
        for line in text.lines() {
            self.runs
                .push(TextRun::new(Vec2::new(at.x, y), line, size, color));
            y += size + 2.0;
        }
        y
    }

    /// Add a row of map-space text.
    pub fn world_text(&mut self, run: TextRun) -> &mut Self {
        self.world_runs.push(run);
        self
    }

    /// Add a map-space icon.
    pub fn world_icon(&mut self, run: IconRun) -> &mut Self {
        self.world_icons.push(run);
        self
    }

    /// Every string in this panel, both spaces, for the printable check.
    pub fn all_strings(&self) -> impl Iterator<Item = &str> {
        self.runs
            .iter()
            .chain(self.world_runs.iter())
            .map(|run| run.text.as_str())
    }

    /// Everything in `other`, appended.
    pub fn absorb(&mut self, other: Panel) {
        self.runs.extend(other.runs);
        self.icons.extend(other.icons);
        self.world_runs.extend(other.world_runs);
        self.world_icons.extend(other.world_icons);
    }
}

/// Break `text` into lines of at most `columns` characters, on spaces — and
/// through a word that is longer than the column on its own.
///
/// `ctx.text` does not wrap - `\n` is the only line break there is - so a game
/// that draws a generated sentence wraps it itself or draws it off the side of
/// the world.
///
/// **The over-long word is split rather than left long, and that rule was paid
/// for.** It used to say the opposite, on the grounds that giri's vocabulary had
/// no such word; the constants stamp is one -
/// `k_inf:1,k_kill:5,k_loyal:4,...` is a hundred and forty-five characters with
/// no space in it - and the line an APPLY raises ran a third of the way off the
/// screen before a hand playtest saw it. A wrapper whose contract is "no line is
/// wider than the column" has to be true of every string, because the string
/// that breaks it is always the one added after the rule was written.
pub fn wrap(text: &str, columns: usize) -> String {
    let columns = columns.max(1);
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > columns {
            lines.push(std::mem::take(&mut line));
        }
        // Split through, in column-wide pieces, when the word cannot fit a line
        // of its own. The break falls where it falls: a stamp is machine text,
        // and a hyphen inserted into one is a character `parse` would refuse.
        let mut rest: &str = word;
        if rest.chars().count() > columns {
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            while rest.chars().count() > columns {
                let cut = rest
                    .char_indices()
                    .nth(columns)
                    .map_or(rest.len(), |(index, _)| index);
                let (head, tail) = rest.split_at(cut);
                lines.push(head.to_owned());
                rest = tail;
            }
        } else if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(rest);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines.join("\n")
}

/// How many characters of `size` fit across `width` world units.
///
/// One call to the engine's own measurement (`TextStyle::columns_in`), so the
/// font's advance ratio appears nowhere in this game.
pub fn columns(width: f32, size: f32) -> usize {
    theme::text(size, theme::INK).columns_in(width)
}

/// Fill a rectangle.
pub fn fill(ctx: &mut DrawCtx, rect: Rect, color: Color, layer: i16) {
    ctx.rect(rect, color, Depth::layer(layer));
}

/// Draw a border `thickness` wide, inside `rect`.
pub fn border(ctx: &mut DrawCtx, rect: Rect, color: Color, thickness: f32, layer: i16) {
    let depth = Depth::layer(layer);
    let size = rect.size();
    ctx.rect(
        Rect::from_min_size(rect.min, Vec2::new(size.x, thickness)),
        color,
        depth,
    );
    ctx.rect(
        Rect::from_min_size(
            Vec2::new(rect.min.x, rect.max.y - thickness),
            Vec2::new(size.x, thickness),
        ),
        color,
        depth,
    );
    ctx.rect(
        Rect::from_min_size(rect.min, Vec2::new(thickness, size.y)),
        color,
        depth,
    );
    ctx.rect(
        Rect::from_min_size(
            Vec2::new(rect.max.x - thickness, rect.min.y),
            Vec2::new(thickness, size.y),
        ),
        color,
        depth,
    );
}

/// A button: a face, and the pressed-looking shadow under it.
///
/// `live` is the whole difference between a button that will do something and
/// one that will not — colour *and* the shadow, because colour alone is one
/// channel and UI.md §1 asks for two.
pub fn button(ctx: &mut DrawCtx, rect: Rect, live: bool, layer: i16) {
    let (face, shadow) = if live {
        (theme::GOLD, theme::GOLD_DEEP)
    } else {
        (theme::BUTTON_DEAD, theme::GHOST)
    };
    fill(
        ctx,
        Rect::from_min_size(rect.min + Vec2::new(0.0, 3.0), rect.size()),
        shadow,
        layer,
    );
    fill(ctx, rect, face, layer + 1);
}

/// Centre a label horizontally in `rect`, at its `top`.
pub fn centered(rect: Rect, text: &str, size: f32, top: f32) -> Vec2 {
    let width = theme::text(size, theme::INK).width_of(text);
    Vec2::new(rect.center().x - width * 0.5, top)
}

/// Draw a panel's contents: map-space lists as they are, chrome through the
/// UI mapping — one transform, applied at the last moment, so every reader of
/// the layout reads it untransformed.
pub fn draw(ctx: &mut DrawCtx, panel: &Panel, map: &crate::camera::UiMap) {
    let gallery = ctx.world.resource::<Gallery>().clone();
    // Map-space content is culled to the camera, like the terrain: a label
    // panned off the screen submits nothing (the game-side culling DESIGN §8
    // asks of the game).
    let view = ctx.world.resource::<Camera>().visible_bounds();
    for icon in &panel.world_icons {
        if !icon.bounds().overlaps(view) {
            continue;
        }
        let sprite = gallery.sprite(icon.art, icon.scale, icon.layer, icon.tint);
        ctx.sprite(&Transform::at(icon.at), &sprite);
    }
    for run in &panel.world_runs {
        if !run.bounds().overlaps(view) {
            continue;
        }
        ctx.text(
            run.at,
            &run.text,
            TextStyle {
                face: Face::BUILT_IN,
                size: run.size,
                color: run.color,
                depth: Depth::layer(run.layer),
            },
        );
    }
    for icon in &panel.icons {
        let sprite = gallery.sprite(icon.art, icon.scale * map.scale, icon.layer, icon.tint);
        ctx.sprite(&Transform::at(map.to_world(icon.at)), &sprite);
    }
    for run in &panel.runs {
        ctx.text(
            map.to_world(run.at),
            &run.text,
            TextStyle {
                face: Face::BUILT_IN,
                size: run.size * map.scale,
                color: run.color,
                depth: Depth::layer(run.layer),
            },
        );
    }
}
