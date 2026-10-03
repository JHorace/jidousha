//! Measured text: wrapping to a column, clipping to a width, centring in a
//! rectangle, and the cell that does the clipping for you.
//!
//! Key types: `Cell`. Key functions: `wrap`, `clipped`, `centered`.
//! Depends on: `jidousha-core`, `jidousha-render-core`.
//! INVARIANT: every measurement here is the engine's own (`TextStyle`), so a
//! font's advance ratio appears nowhere in a game — a game that wrote
//! `width / size` drew its line off the side of the world the day the face
//! changed.

use jidousha_core::{Rect, math::Vec2};
use jidousha_render_core::TextStyle;

use crate::panel::TextRun;

/// Break `text` into lines of at most `columns` characters, on spaces — and
/// through a word that is longer than the column on its own.
///
/// `ctx.text` does not wrap; `\n` is the only line break there is. So a game
/// that draws a generated sentence wraps it itself, with the column count
/// from `TextStyle::columns_in`, or draws it off the side of the world.
///
/// **A line break the caller wrote survives.** Each of the caller's own lines
/// is wrapped on its own and the breaks are kept — the only contract under
/// which "no line is wider than the column" is true of every string rather
/// than of every string without a newline in it.
///
/// **An over-long word is split rather than left long.** A machine string
/// with no space in it — a constants stamp, a path — is the string that
/// breaks a wrapper whose rule is "unless", and it is always the one added
/// after the rule was written. No hyphen is inserted: the break falls where
/// it falls, because an inserted character is one a parser would refuse.
///
/// ```
/// use jidousha_ui::wrap;
///
/// assert_eq!(wrap("the world is running", 9), "the world\nis\nrunning");
/// assert_eq!(wrap("one\ntwo three", 20), "one\ntwo three");
/// assert_eq!(wrap("k_inf:1,k_kill:5", 6), "k_inf:\n1,k_ki\nll:5");
/// ```
#[must_use]
pub fn wrap(text: &str, columns: usize) -> String {
    if text.contains('\n') {
        return text
            .split('\n')
            .map(|line| wrap(line, columns))
            .collect::<Vec<_>>()
            .join("\n");
    }
    let columns = columns.max(1);
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > columns {
            lines.push(std::mem::take(&mut line));
        }
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

/// As much of `text` as fits across `width` in `style`, cut at a word and
/// marked with three dots.
///
/// The tight answer (`TextStyle::fits_in`) for a string you have in your
/// hand. The cut falls back to the last space, because a row that stops
/// mid-word reads as a rendering fault rather than as a row that ran out of
/// room; a string that fits comes back untouched.
///
/// ```
/// use jidousha_render_core::TextStyle;
/// use jidousha_ui::clipped;
///
/// let small = TextStyle { size: 12.0, ..TextStyle::default() };
/// assert_eq!(clipped(&small, "idle", 200.0), "idle");
/// let cut = clipped(&small, "holds 3g of the 12g due", 90.0);
/// assert!(cut.ends_with("..."), "{cut:?}");
/// assert!(small.width_of(&cut) <= 90.0);
/// ```
#[must_use]
pub fn clipped(style: &TextStyle, text: &str, width: f32) -> String {
    let fits = style.fits_in(text, width);
    if fits >= text.chars().count() {
        return text.to_owned();
    }
    let head: String = text.chars().take(fits.saturating_sub(3)).collect();
    let cut = head.rfind(' ').unwrap_or(head.len());
    format!("{}...", head[..cut].trim_end())
}

/// Where a label goes to sit centred across `rect`, with its top at `top` —
/// the one baseline rule for a button's label.
///
/// The horizontal centring is the rule; `top` is the game's, because where a
/// label sits inside a control is a fact about that control's height and the
/// face's ascent, and the game states it once per control shape.
///
/// ```
/// use jidousha_core::{Rect, math::Vec2};
/// use jidousha_render_core::TextStyle;
/// use jidousha_ui::centered;
///
/// let button = Rect::from_min_size(Vec2::new(100.0, 10.0), Vec2::new(48.0, 32.0));
/// let small = TextStyle { size: 12.0, ..TextStyle::default() };
/// let at = centered(button, &small, "OK", button.min.y + 10.0);
/// assert_eq!(at.y, 20.0);
/// assert_eq!(at.x + small.width_of("OK") * 0.5, button.center().x);
/// ```
#[must_use]
pub fn centered(rect: Rect, style: &TextStyle, text: &str, top: f32) -> Vec2 {
    let width = style.width_of(text);
    Vec2::new(rect.center().x - width * 0.5, top)
}

/// A place for one row of text, and how wide it may be — the cell idiom:
/// `at + offset, clipped(text, width)` written once instead of at every row.
///
/// ```
/// use jidousha_core::{Color, Depth, math::Vec2};
/// use jidousha_render_core::TextStyle;
/// use jidousha_ui::Cell;
///
/// let small = TextStyle { size: 12.0, color: Color::WHITE, depth: Depth::layer(1), ..TextStyle::default() };
/// let name = Cell::new(Vec2::new(36.0, 2.0), 60.0);
/// let row = name.run("Bartholomew the Unready", small);
/// assert!(row.text.ends_with("..."));
/// assert!(row.bounds().size().x <= 60.0);
/// assert_eq!(name.at(Vec2::new(100.0, 100.0)).at, Vec2::new(136.0, 102.0));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    /// The top-left of the row's first character.
    pub at: Vec2,
    /// The room it has, in the same units as `at`.
    pub width: f32,
}

impl Cell {
    /// A cell at `at`, `width` wide.
    pub fn new(at: Vec2, width: f32) -> Self {
        Self { at, width }
    }

    /// The same cell, moved by `origin` — a row's cell stated as an offset
    /// from the row, placed at the row.
    #[must_use]
    pub fn at(self, origin: Vec2) -> Self {
        Self {
            at: self.at + origin,
            width: self.width,
        }
    }

    /// The row this cell holds: `text` clipped to the cell's width, measured
    /// and drawn in `style`.
    #[must_use]
    pub fn run(self, text: &str, style: TextStyle) -> TextRun {
        TextRun::new(self.at, clipped(&style, text, self.width), style)
    }
}
