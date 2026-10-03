//! The UI kit: a screen as data, chips and feeds as one value, and the floors
//! that judge what a screen says it draws.
//!
//! Key types: `Panel`, `TextRun`, `IconRun`, `Chip`, `Attention`, `MeterSpec`,
//! `Floors`.
//! Depends on: `jidousha-core`, `jidousha-render-core`. Must never be depended
//! on by: either of them, or by `jidousha-platform`.
//! INVARIANT: nothing here decides anything a game simulates. A panel is the
//! strings and icons a screen hands back; a chip is which id is lit; a feed is
//! a view over a log the game owns. Every number a floor compares against is
//! the game's (`Floors`), and every string is derived at draw time from the
//! game's own rows — the kit carries the *shape* of a screen and never its
//! content (ADR-0046).
//!
//! The kit was extracted from a game, not designed ahead of one: every part
//! here was produced three or more times by `games/ninjo` and promoted by the
//! wave-1 exemplar audit's report and the owner's verdict on it (ADR-0046,
//! conventions §Extraction). What was deliberately left out, and what would
//! bring it in, is recorded there.
//!
//! ```
//! use jidousha_core::{Color, Depth, Rect, math::Vec2};
//! use jidousha_render_core::TextStyle;
//! use jidousha_ui::{Floors, Panel, TextRun, judge_panel};
//!
//! #[derive(Clone, Copy, Debug, PartialEq)]
//! enum NoArt {}
//! impl jidousha_ui::Icon for NoArt {
//!     fn size_at(self, _scale: f32) -> Vec2 {
//!         match self {}
//!     }
//! }
//!
//! let small = TextStyle { size: 12.0, color: Color::WHITE, depth: Depth::layer(1), ..TextStyle::default() };
//! let mut panel: Panel<NoArt> = Panel::default();
//! panel.text(TextRun::new(Vec2::new(10.0, 10.0), "a settlement", small));
//!
//! let floors = Floors {
//!     min_text: 12.0,
//!     chrome: Rect::from_min_size(Vec2::ZERO, Vec2::new(960.0, 540.0)),
//!     world: Rect::from_min_size(Vec2::ZERO, Vec2::new(4000.0, 4000.0)),
//! };
//! assert!(judge_panel(&panel, &floors, &[], &[]).is_empty());
//! ```

mod attention;
mod chip;
mod floors;
mod meters;
mod panel;
mod text;

pub use attention::{
    Attention, ClassSpec, FeedEntry, Mode, Pause, class_faults, feed, find_class, reason_line,
};
pub use chip::{Chip, toggle};
pub use floors::{Breach, Floors, frame_text_floor, glyph_run, inside, judge_frame, judge_panel};
pub use meters::{MeterSpec, count, faces, meter_faults};
pub use panel::{Icon, IconRun, Mapping, Panel, TextRun};
pub use text::{Cell, centered, clipped, wrap};
