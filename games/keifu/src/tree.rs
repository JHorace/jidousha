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

/// The narrowest a node is drawn — room for a seven-letter name at 16 px — and the air
/// between two nodes on a line.
const NODE_MIN_W: f32 = 104.0;
const NODE_GAP: f32 = 8.0;
/// The tree's foot: the generations stop short of the remembrance panel.
const TREE_FOOT: f32 = 504.0;

/// The tree's lines, top to bottom: each generation in order, wrapped onto as many lines
/// as its heroes need at `NODE_MIN_W` — never between a hero and the spouse placed after
/// them, so a spouse link stays on one line.
pub fn tree_lines(house: &House) -> Vec<Vec<HeroId>> {
    let fits = ((TREE_RIGHT - TREE_LEFT + NODE_GAP) / (NODE_MIN_W + NODE_GAP)) as usize;
    let heroes = &house.heroes;
    let married = |a: HeroId, b: HeroId| {
        heroes[a]
            .bond_to(b)
            .is_some_and(|bond| bond.kind == crate::ids::BondKind::Spouse)
    };
    let mut out = Vec::new();
    for row in tree_rows(heroes) {
        let mut line: Vec<HeroId> = Vec::new();
        for id in row {
            if line.len() == fits {
                // Carry a hero over with the spouse who follows them.
                let carried = match line.last() {
                    Some(&last) if line.len() > 1 && married(last, id) => line.pop(),
                    _ => None,
                };
                out.push(std::mem::take(&mut line));
                line.extend(carried);
            }
            line.push(id);
        }
        if !line.is_empty() {
            out.push(line);
        }
    }
    out
}

/// From one line of the tree to the next: a generation's step, or less to fit `lines`.
fn line_step(lines: usize) -> f32 {
    let between = lines.saturating_sub(1).max(1) as f32;
    GENERATION_STEP.min((TREE_FOOT - TREE_TOP - NODE.y) / between)
}

/// Where every hero's node sits: line by line, the lines spread down to the tree's foot
/// at most a generation's step apart, each line's nodes spread across the width and
/// narrowed (to no less than `NODE_MIN_W`) when many share it.
pub fn node_rects(house: &House) -> Vec<(HeroId, Rect)> {
    let lines = tree_lines(house);
    let step = line_step(lines.len());
    let mut out = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let span = (TREE_RIGHT - TREE_LEFT) / line.len() as f32;
        let size = Vec2::new(NODE.x.min(span - NODE_GAP), NODE.y);
        for (index, id) in line.iter().enumerate() {
            let center = Vec2::new(
                TREE_LEFT + (index as f32 + 0.5) * span,
                TREE_TOP + at as f32 * step + NODE.y * 0.5,
            );
            out.push((*id, Rect::from_center_size(center, size)));
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
    // Wrapped short of the close control: the Ending's subline is the longer.
    for (index, piece) in wrap(&subline, CLOSE_BUTTON.min.x - 12.0 - 24.0, MIN_TEXT)
        .into_iter()
        .enumerate()
    {
        let at = Vec2::new(24.0, 44.0 + index as f32 * 17.0);
        if index == 0 {
            page.text(layers::OVERLAY_TEXT, at, piece, MIN_TEXT, ink::NOTE, screen);
        } else {
            page.continue_text(layers::OVERLAY_TEXT, at, piece, MIN_TEXT, ink::NOTE, screen);
        }
    }
    button(
        page,
        CLOSE_BUTTON,
        &words[close],
        Target::CloseFamily,
        layers::OVERLAY_MARK - 1,
    );

    let nodes = node_rects(house);
    let lines = tree_lines(house).len();
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
        // Half-way between the lines, however close they have had to come.
        let elbow = rect.min.y - ((line_step(lines) - NODE.y) * 0.5).min(18.0);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::BondKind;
    use crate::testkit::{house, id};

    /// The founding household with `more` wanderers like Odo (no parents) added.
    fn crowded(more: usize) -> House {
        let (_, mut house) = house();
        let odo = id(&house.heroes, "Odo");
        for n in 0..more {
            let mut hero = house.heroes[odo].clone();
            hero.name = format!("Ann{n}");
            hero.bonds.clear();
            house.heroes.push(hero);
        }
        house
    }

    #[test]
    fn a_generation_wider_than_the_screen_wraps_onto_lines_of_eleven() {
        let house = crowded(14);
        let lines = tree_lines(&house);
        assert!(lines.iter().all(|line| line.len() <= 11), "{lines:?}");
        let total: usize = lines.iter().map(Vec::len).sum();
        assert_eq!(total, house.heroes.len());
        let rects = node_rects(&house);
        for (i, (_, a)) in rects.iter().enumerate() {
            assert!(a.size().x >= 104.0 && a.max.y <= 504.0, "{a:?}");
            for (_, b) in &rects[i + 1..] {
                assert!(!a.overlaps(*b), "{a:?} {b:?}");
            }
        }
    }

    #[test]
    fn a_hero_and_the_spouse_after_them_wrap_together() {
        let mut house = crowded(14);
        let row = &tree_rows(&house.heroes)[0];
        // Wed the eleventh of the first generation to the next one along.
        let (a, b) = (row[10], row[11]);
        crate::bonds::form(&mut house.heroes, a, b, BondKind::Spouse, 1);
        let lines = tree_lines(&house);
        let line_of = |h: HeroId| lines.iter().position(|line| line.contains(&h));
        assert_eq!(line_of(a), line_of(b), "{lines:?}");
        assert!(
            lines[0].len() < 11,
            "the line broke before the pair: {lines:?}"
        );
    }

    #[test]
    fn many_lines_close_up_to_stay_above_the_remembrance() {
        let house = crowded(80);
        let rects = node_rects(&house);
        assert!(rects.iter().all(|(_, r)| r.max.y <= 504.0));
        assert!(tree_lines(&house).len() >= 8);
    }
}
