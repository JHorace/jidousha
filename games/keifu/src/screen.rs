//! What is on the screen: one `Page` computed from the house, read by Draw and by verify.
//!
//! `page` is the projection both phases share (docs/api "A projection both phases
//! read is written once"): the Draw system submits exactly its rows and boxes,
//! the Update system hit-tests its targets, and the verify run judges its rows
//! against the readability floors and the oracle strings. The camera is one world
//! unit per pixel on a 1280x720 window, so every number here is a pixel.

use jidousha::prelude::*;

use crate::content::Content;
use crate::hero::HeroId;
use crate::house::House;

/// The window, and so the world: one unit per pixel.
pub const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);
/// The page's height in world units.
pub const PAGE_H: f32 = WINDOW.height as f32;
/// The page's width, derived from the window's aspect.
pub const PAGE_W: f32 = PAGE_H * WINDOW.aspect();
/// The smallest type the game sets, in world units (pixels).
pub const MIN_TEXT: f32 = 14.0;
/// The gap kept between text and the edge of its panel.
pub const PAD: f32 = 12.0;

/// The camera every frame is drawn with.
pub fn camera() -> Camera {
    Camera {
        center: Vec2::new(PAGE_W * 0.5, PAGE_H * 0.5),
        height: PAGE_H,
        clear_color: ink::PAGE,
        ..Camera::default()
    }
}

/// Draw bands.
pub mod layers {
    /// Panels and cards.
    pub const PANEL: i16 = 0;
    /// Figures, pips, tree links.
    pub const MARK: i16 = 1;
    /// Type.
    pub const TEXT: i16 = 2;
    /// The family overlay's backdrop, over the whole summer screen.
    pub const OVERLAY: i16 = 3;
    /// The overlay's marks.
    pub const OVERLAY_MARK: i16 = 4;
    /// The overlay's type.
    pub const OVERLAY_TEXT: i16 = 5;
}

/// The palette.
pub mod ink {
    use jidousha::prelude::Color;

    /// The page.
    pub const PAGE: Color = Color::rgb(0.06, 0.06, 0.08);
    /// A panel.
    pub const PANEL: Color = Color::rgb(0.11, 0.11, 0.14);
    /// A panel under the pointer.
    pub const HOT: Color = Color::rgb(0.19, 0.19, 0.25);
    /// Body type.
    pub const BODY: Color = Color::rgb(0.92, 0.92, 0.94);
    /// Secondary type.
    pub const NOTE: Color = Color::rgb(0.62, 0.64, 0.70);
    /// Headings.
    pub const HEADING: Color = Color::rgb(0.95, 0.80, 0.42);
    /// Warnings, and renown at 4 or below.
    pub const WARN: Color = Color::rgb(0.95, 0.38, 0.33);
    /// Something gone: the dead, a bond to the dead, a stage done.
    pub const GONE: Color = Color::rgb(0.45, 0.45, 0.50);
    /// The departed (crowned).
    pub const GOLD: Color = Color::rgb(0.86, 0.72, 0.30);
    /// Might, Wits, Spirit figure tints.
    pub const APTITUDE: [Color; 3] = [
        Color::rgb(0.93, 0.55, 0.25),
        Color::rgb(0.30, 0.75, 0.72),
        Color::rgb(0.90, 0.50, 0.70),
    ];
    /// A child's figure.
    pub const CHILD: Color = Color::rgb(0.78, 0.74, 0.62);
    /// An empty pip.
    pub const PIP_EMPTY: Color = Color::rgb(0.30, 0.30, 0.36);
    /// A dread pip.
    pub const DREAD: Color = Color::rgb(0.66, 0.55, 0.88);
    /// Tree links.
    pub const LINK: Color = Color::rgb(0.42, 0.42, 0.50);
}

/// What a pointer can be over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// A hero's card (summer) or node (family).
    Hero(HeroId),
    /// "The family".
    OpenFamily,
    /// "Back to the house".
    CloseFamily,
}

/// Which screen is up and what the pointer is on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiState {
    /// The family overlay is open.
    pub family_open: bool,
    /// The hero under the pointer, on whichever screen is up.
    pub pointing: Option<HeroId>,
}

impl Resource for UiState {}

/// One line of type, with the panel it must stay inside.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// Top-left of the first character's cell.
    pub at: Vec2,
    /// What it says.
    pub text: String,
    /// How it is set.
    pub style: TextStyle,
    /// The panel it belongs to.
    pub panel: Rect,
    /// Which logical line it is part of: the rows one wrapped line became share it.
    pub logical: usize,
}

impl Row {
    /// The box the pen sweeps.
    pub fn bounds(&self) -> Rect {
        Rect::from_min_size(self.at, self.style.measure(&self.text).size)
    }
}

/// A filled rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    /// Where.
    pub rect: Rect,
    /// In what.
    pub color: Color,
    /// In which band.
    pub layer: i16,
}

/// Everything one frame draws, and where the pointer can land.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Page {
    /// Type.
    pub rows: Vec<Row>,
    /// Panels, figures, pips.
    pub shapes: Vec<Shape>,
    /// Tree links: from, to.
    pub links: Vec<(Vec2, Vec2)>,
    /// Hit targets, front first.
    pub targets: Vec<(Rect, Target)>,
    /// Logical lines begun so far.
    lines_begun: usize,
}

impl Page {
    /// Add a line of type in band `layer`.
    pub fn text(
        &mut self,
        layer: i16,
        at: Vec2,
        text: impl Into<String>,
        size: f32,
        color: Color,
        panel: Rect,
    ) {
        self.lines_begun += 1;
        self.continue_text(layer, at, text, size, color, panel);
    }

    /// Add a row that continues the last logical line (a wrapped piece of it).
    pub fn continue_text(
        &mut self,
        layer: i16,
        at: Vec2,
        text: impl Into<String>,
        size: f32,
        color: Color,
        panel: Rect,
    ) {
        self.rows.push(Row {
            logical: self.lines_begun,
            at,
            text: text.into(),
            style: TextStyle {
                size,
                color,
                depth: Depth::layer(layer),
                ..TextStyle::default()
            },
            panel,
        });
    }

    /// Add a filled rectangle in band `layer`.
    pub fn shape(&mut self, rect: Rect, color: Color, layer: i16) {
        self.shapes.push(Shape { rect, color, layer });
    }

    /// Every logical line, its wrapped rows joined back with spaces, with the
    /// indices of the rows it was drawn as.
    pub fn logical_lines(&self) -> Vec<(String, Vec<usize>)> {
        let mut out: Vec<(String, Vec<usize>)> = Vec::new();
        let mut last = None;
        for (index, row) in self.rows.iter().enumerate() {
            match out.last_mut() {
                Some((line, rows)) if last == Some(row.logical) => {
                    line.push(' ');
                    line.push_str(&row.text);
                    rows.push(index);
                }
                _ => out.push((row.text.clone(), vec![index])),
            }
            last = Some(row.logical);
        }
        out
    }

    /// What is under `point`, front first.
    pub fn target_at(&self, point: Vec2) -> Option<Target> {
        self.targets
            .iter()
            .find(|(rect, _)| rect.contains(point))
            .map(|(_, target)| *target)
    }
}

/// Break `text` into lines that fit `width` at `size`, at spaces.
pub fn wrap(text: &str, width: f32, size: f32) -> Vec<String> {
    let style = TextStyle {
        size,
        ..TextStyle::default()
    };
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split(' ') {
        let candidate = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{current} {word}")
        };
        if style.width_of(&candidate) <= width || current.is_empty() {
            current = candidate;
        } else {
            lines.push(std::mem::replace(&mut current, word.to_owned()));
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// The page for the screen that is up.
pub fn page(content: &Content, house: &House, ui: &UiState) -> Page {
    let mut page = Page::default();
    crate::summer::lay_out(&mut page, content, house, ui);
    if ui.family_open {
        // The overlay covers the summer screen, so only its targets are live.
        let mut overlay = Page::default();
        crate::tree::lay_out(&mut overlay, content, house, ui);
        let offset = page.lines_begun;
        page.rows.extend(overlay.rows.into_iter().map(|row| Row {
            logical: row.logical + offset,
            ..row
        }));
        page.lines_begun += overlay.lines_begun;
        page.shapes.extend(overlay.shapes);
        page.links.extend(overlay.links);
        page.targets = overlay.targets;
    }
    page
}
