//! The colours, named once.

use jidousha::prelude::Color;

/// The screen behind everything: near black with a trace of violet.
pub const BG: Color = Color::rgb(0.05, 0.04, 0.08);
/// Ordinary text.
pub const TEXT: Color = Color::rgb(0.90, 0.89, 0.92);
/// Quieter text: notes, effects, hints.
pub const NOTE: Color = Color::rgb(0.66, 0.65, 0.74);
/// Headings and the lore the player has earned.
pub const GOLD: Color = Color::rgb(0.92, 0.78, 0.36);
/// What costs sanity.
pub const WARN: Color = Color::rgb(0.92, 0.38, 0.38);
/// What helps.
pub const GOOD: Color = Color::rgb(0.45, 0.82, 0.55);
/// The empty part of a bar.
pub const EMPTY: Color = Color::rgb(0.16, 0.15, 0.22);
/// The filled part of the sanity bar.
pub const SANITY: Color = Color::rgb(0.40, 0.62, 0.92);
