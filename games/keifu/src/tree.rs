//! The family screen (SPEC §19.2): every hero who ever lived, parents above children,
//! spouses linked, and the remembrance of whoever is pointed at.

use jidousha::prelude::*;

use crate::content::Content;
use crate::ending::Verdict;
use crate::family::{remembrance, spouse_pairs, tally, tally_sentence, tree_rows};
use crate::hero::{Fate, HeroId};
use crate::house::House;
use crate::screen::{MIN_TEXT, PAD, PAGE_H, PAGE_W, Page, Target, UiState, ink, layers, wrap};
use crate::summer::{button, screen_rect};
use crate::text::fmt;
use crate::words::W;

/// A node.
pub const NODE: Vec2 = Vec2::new(150.0, 46.0);
/// Where the first generation's nodes sit.
const TREE_TOP: f32 = 96.0;
/// From one generation to the next.
const GENERATION_STEP: f32 = 104.0;
/// The tree's horizontal extent.
const TREE_LEFT: f32 = 24.0;
const TREE_RIGHT: f32 = PAGE_W - 24.0;
/// The remembrance panel.
pub const REMEMBRANCE: Rect = Rect {
    min: Vec2::new(24.0, 512.0),
    max: Vec2::new(PAGE_W - 24.0, PAGE_H - 16.0),
};
/// "Back to the house".
pub const CLOSE_BUTTON: Rect = Rect {
    min: Vec2::new(PAGE_W - 224.0, 18.0),
    max: Vec2::new(PAGE_W - 24.0, 54.0),
};
const LINK_THICKNESS: f32 = 2.0;

/// Where every hero's node sits.
pub fn node_rects(house: &House) -> Vec<(HeroId, Rect)> {
    let mut out = Vec::new();
    for (generation, row) in tree_rows(&house.heroes).iter().enumerate() {
        let span = (TREE_RIGHT - TREE_LEFT) / row.len() as f32;
        for (index, id) in row.iter().enumerate() {
            let center = Vec2::new(
                TREE_LEFT + (index as f32 + 0.5) * span,
                TREE_TOP + generation as f32 * GENERATION_STEP + NODE.y * 0.5,
            );
            out.push((*id, Rect::from_center_size(center, NODE)));
        }
    }
    out
}

/// Lay the family screen out, over everything.
pub fn lay_out(page: &mut Page, content: &Content, house: &House, ui: &UiState) {
    let words = &content.words;
    let screen = screen_rect();
    // At the Ending the tree has its own heading and subline, and "The verdict" returns
    // to the verdict page (SPEC §23).
    let (heading, subline, close) = match house.ending.as_ref().map(|e| &e.verdict) {
        Some(verdict) => (
            match verdict {
                Verdict::Door { .. } => W::EndingTreeHeadingDoor,
                Verdict::Closed { .. } => W::EndingTreeHeadingClosed,
            },
            fmt(
                &words[W::EndingTreeSubline],
                &[&tally_sentence(content, house)],
            ),
            W::EndingVerdict,
        ),
        None => (W::FamilyHeading, tally(content, house), W::FamilyClose),
    };
    page.shape(screen, ink::PAGE, layers::OVERLAY);
    page.text(
        layers::OVERLAY_TEXT,
        Vec2::new(24.0, 14.0),
        &words[heading],
        20.0,
        ink::HEADING,
        screen,
    );
    page.text(
        layers::OVERLAY_TEXT,
        Vec2::new(24.0, 44.0),
        subline,
        MIN_TEXT,
        ink::NOTE,
        screen,
    );
    button(
        page,
        CLOSE_BUTTON,
        &words[close],
        Target::CloseFamily,
        layers::OVERLAY_MARK - 1,
    );

    let nodes = node_rects(house);
    let rect_of = |id: HeroId| nodes.iter().find(|(n, _)| *n == id).map(|(_, r)| *r);
    for (a, b) in spouse_pairs(&house.heroes) {
        if let (Some(a), Some(b)) = (rect_of(a), rect_of(b)) {
            let (left, right) = if a.min.x < b.min.x { (a, b) } else { (b, a) };
            let y = left.center().y;
            page.links
                .push((Vec2::new(left.max.x, y), Vec2::new(right.min.x, y)));
        }
    }
    for (child, rect) in &nodes {
        let parents: Vec<Rect> = house.heroes[*child]
            .parents
            .iter()
            .flatten()
            .filter_map(|parent| rect_of(*parent))
            .collect();
        let Some(first) = parents.first() else {
            continue;
        };
        // From under the parent pair (or single parent) down to the child.
        let from_x = parents.iter().map(|p| p.center().x).sum::<f32>() / parents.len() as f32;
        let from_y = if parents.len() > 1 {
            first.center().y
        } else {
            first.max.y
        };
        let elbow = rect.min.y - 18.0;
        let to = Vec2::new(rect.center().x, rect.min.y);
        page.links
            .push((Vec2::new(from_x, from_y), Vec2::new(from_x, elbow)));
        page.links
            .push((Vec2::new(from_x, elbow), Vec2::new(to.x, elbow)));
        page.links.push((Vec2::new(to.x, elbow), to));
    }
    for (id, rect) in &nodes {
        let hero = &house.heroes[*id];
        let (fill, text) = match hero.fate {
            Fate::Living => (
                if ui.pointing == Some(*id) {
                    ink::HOT
                } else {
                    ink::PANEL
                },
                ink::BODY,
            ),
            Fate::Dead => (ink::PANEL, ink::GONE),
            Fate::Departed => (ink::PANEL, ink::GOLD),
        };
        page.shape(*rect, fill, layers::OVERLAY_MARK);
        let x = rect.min.x + 8.0;
        page.text(
            layers::OVERLAY_TEXT,
            Vec2::new(x, rect.min.y + 6.0),
            hero.name.clone(),
            16.0,
            text,
            *rect,
        );
        page.text(
            layers::OVERLAY_TEXT,
            Vec2::new(x, rect.min.y + 26.0),
            hero.house.clone(),
            MIN_TEXT,
            ink::NOTE,
            *rect,
        );
        page.targets.push((*rect, Target::Hero(*id)));
    }

    page.shape(REMEMBRANCE, ink::PANEL, layers::OVERLAY_MARK);
    let width = REMEMBRANCE.size().x - 2.0 * PAD;
    let x = REMEMBRANCE.min.x + PAD;
    let mut y = REMEMBRANCE.min.y + PAD;
    let lines = match ui.pointing {
        Some(id) => remembrance(content, house, id),
        None => vec![words[W::FamilyHelp].to_owned()],
    };
    for (index, line) in lines.iter().enumerate() {
        let (size, color) = if index == 0 && ui.pointing.is_some() {
            (18.0, ink::BODY)
        } else {
            (MIN_TEXT, ink::NOTE)
        };
        for (piece_index, piece) in wrap(line, width, size).into_iter().enumerate() {
            let at = Vec2::new(x, y);
            if piece_index == 0 {
                page.text(layers::OVERLAY_TEXT, at, piece, size, color, REMEMBRANCE);
            } else {
                page.continue_text(layers::OVERLAY_TEXT, at, piece, size, color, REMEMBRANCE);
            }
            y += size + 4.0;
        }
    }
}

/// Draw a tree link as a line, for the Draw system.
pub fn link_style() -> (f32, Color, Depth) {
    (
        LINK_THICKNESS,
        ink::LINK,
        Depth::layer(layers::OVERLAY_MARK),
    )
}
