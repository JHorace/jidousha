//! The layout: the window, the design space the camera shows 1:1, the
//! palette, the bands, the floors, and every rectangle a surface sits in.
//!
//! Stated once here so the draw, the hit-test, the floors and the verify run's
//! taps all read the same numbers.

use jidousha::prelude::*;
use jidousha::ui::{Floors, Icon, Mapping};

/// The window, and every recorder: one size, so one aspect (16:9).
pub(crate) const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);
/// The design space, which is the world the camera shows.
pub(crate) const DESIGN: Vec2 = Vec2::new(540.0 * WINDOW.aspect(), 540.0);

pub(crate) const TABLE: Color = Color::rgb(0.06, 0.07, 0.10);
pub(crate) const BOX: Color = Color::rgb(0.13, 0.15, 0.20);
pub(crate) const BOX_LIT: Color = Color::rgb(0.22, 0.25, 0.33);
pub(crate) const MARKED: Color = Color::rgb(0.18, 0.30, 0.24);
pub(crate) const AIMED: Color = Color::rgb(0.45, 0.36, 0.12);
pub(crate) const OVERLAY: Color = Color::rgb(0.10, 0.11, 0.16);
pub(crate) const INK: Color = Color::rgb(0.90, 0.88, 0.82);
pub(crate) const DIM: Color = Color::rgb(0.55, 0.56, 0.62);
pub(crate) const GOLD: Color = Color::rgb(0.95, 0.78, 0.30);
pub(crate) const YOU_TONE: Color = Color::rgb(0.45, 0.80, 1.0);
pub(crate) const NPC_TONE: Color = Color::rgb(1.0, 0.50, 0.40);

/// Draw bands, named once.
pub(crate) mod layers {
    pub(crate) const BOXES: i16 = 0;
    pub(crate) const TEXT: i16 = 1;
    pub(crate) const OVERLAY: i16 = 2;
    pub(crate) const OVERLAY_TEXT: i16 = 3;
}

/// The readability floors, in design units.
pub(crate) const FLOORS: Floors = Floors {
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

/// The stack panel's box.
pub(crate) const STACK_BOX: Rect = Rect {
    min: Vec2::new(16.0, 40.0),
    max: Vec2::new(600.0, 376.0),
};
/// Where the first stack row sits, and how tall each is.
pub(crate) const STACK_TOP: f32 = 70.0;
pub(crate) const ROW: f32 = 24.0;
/// How many stack rows fit in the box.
pub(crate) const STACK_ROWS: usize = 11;
/// The priority box, top right.
pub(crate) const PRIORITY_BOX: Rect = Rect {
    min: Vec2::new(612.0, 40.0),
    max: Vec2::new(944.0, 120.0),
};
/// The pass button inside it.
pub(crate) const PASS_BUTTON: Rect = Rect {
    min: Vec2::new(852.0, 82.0),
    max: Vec2::new(936.0, 112.0),
};
/// The feed of what happened, under the priority box.
pub(crate) const FEED_BOX: Rect = Rect {
    min: Vec2::new(612.0, 128.0),
    max: Vec2::new(944.0, 376.0),
};
pub(crate) const FEED_ROWS: usize = 14;
/// How wide a feed row may be.
pub(crate) const FEED_WIDTH: f32 = FEED_BOX.max.x - FEED_BOX.min.x - 16.0;
/// The hand row along the bottom.
pub(crate) const HAND_TOP: f32 = 412.0;
pub(crate) const CARD_SIZE: Vec2 = Vec2::new(148.0, 116.0);
pub(crate) const CARD_GAP: f32 = 6.0;
pub(crate) const HAND_SLOTS: usize = 6;
/// The result overlay.
pub(crate) const RESULT_BOX: Rect = Rect {
    min: Vec2::new(200.0, 150.0),
    max: Vec2::new(760.0, 360.0),
};

/// A card slot's rectangle in the hand row.
pub(crate) fn card_box(slot: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(16.0 + slot as f32 * (CARD_SIZE.x + CARD_GAP), HAND_TOP),
        CARD_SIZE,
    )
}

/// A stack row's rectangle; row 0 is the top of the stack.
pub(crate) fn stack_row(row: usize) -> Rect {
    Rect::from_min_size(
        Vec2::new(STACK_BOX.min.x + 6.0, STACK_TOP + row as f32 * ROW),
        Vec2::new(STACK_BOX.size().x - 12.0, ROW - 2.0),
    )
}

/// The camera every surface is drawn through, the verify run's included.
pub(crate) fn camera() -> Camera {
    Camera {
        center: DESIGN * 0.5,
        height: DESIGN.y,
        clear_color: TABLE,
        viewport: WINDOW,
    }
}

/// The game draws no pictures; the kit's icon slot is an empty vocabulary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Art {}

impl Icon for Art {
    fn size_at(self, _scale: f32) -> Vec2 {
        match self {}
    }
}

/// Design space is world space: the camera shows the design rect exactly.
pub(crate) struct Flat;

impl Mapping for Flat {
    fn to_world(&self, ui: Vec2) -> Vec2 {
        ui
    }
    fn scale(&self) -> f32 {
        1.0
    }
}
