//! The turning screen (SPEC §18.1, `scene/scenes/turning.jai`): the turning's pages in
//! the order they were written, each under its kind's heading, a long page continued on
//! further leaves ("..., continued").
//!
//! A death, birth, coming-of-age or arrival page shows the card of the hero it is about;
//! a death page sets the dead's epitaph at its top, under the card (SPEC §18.1), read off
//! the hero so a recomposition shows the moment it happens. An undecided death page ends
//! with "WHO IS HIS
//! HEIR? One choice. It cannot be unmade." and its heir buttons, two columns of up to
//! five, the last "No one. Let it lie."; pointing at one opens that heir's sheet in the
//! dock. While a page is undecided the turning will not go past it: "Go on" there is
//! greyed and does nothing, leaf buttons beyond it do nothing, and "Skip ahead" goes to
//! it instead of leaving. After the choice the leaves are rebuilt and the view stays on
//! that page. The last leaf's "Go on" is "Summer comes".

use jidousha::prelude::*;

use crate::content::Content;
use crate::heirs::heir_buttons;
use crate::hero::Hero;
use crate::house::House;
use crate::passage::{PageKind, Passage, TurnPage};
use crate::screen::{MIN_TEXT, PAD, Page, Target, UiState, ink, layers, wrap};
use crate::summer::{CARD, button, hero_card, lay_out_top_bar};
use crate::telling_view::{PANEL, about, text_area};
use crate::text::fmt;
use crate::words::W;

/// The navigation strip along the panel's foot (`telling_view::text_area` stops above it).
const NAV_H: f32 = 48.0;
/// The pitch of 14 px and 16 px type.
const PITCH: f32 = 17.0;
const TITLE_PITCH: f32 = 20.0;
/// Air between lines, and below the page's heading and title.
const LINE_GAP: f32 = 6.0;
const PART_GAP: f32 = 10.0;
/// A navigation button.
const NAV_BUTTON: Vec2 = Vec2::new(36.0, 34.0);
const NAV_WIDE: f32 = 140.0;
/// The heir buttons: two columns of five rows (CONSTANTS §14).
const HEIR_ROWS: usize = 5;
const HEIR_H: f32 = 30.0;
const HEIR_GAP: f32 = 6.0;
/// The room an undecided death page keeps at its foot: the prompt and the buttons.
const CHOICE_H: f32 = PITCH + PART_GAP + HEIR_ROWS as f32 * (HEIR_H + HEIR_GAP);

/// One leaf: its page, whether it is the page's first and last, and which lines it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leaf {
    /// Index into `Passage::pages`.
    pub page: usize,
    /// The page's first leaf (heading, title, card).
    pub first: bool,
    /// The page's last leaf (an undecided death page's choice).
    pub last: bool,
    /// Its lines, as indices into the page's lines.
    pub lines: std::ops::Range<usize>,
}

/// The heading of a page of `kind` (`ui.turning.page_kinds`).
pub fn heading(content: &Content, kind: PageKind) -> &str {
    let words = &content.words;
    &words[match kind {
        PageKind::Winter => W::TurningPageWinter,
        PageKind::Death => W::TurningPageDeath,
        PageKind::Birth => W::TurningPageBirth,
        PageKind::ComingOfAge => W::TurningPageComingOfAge,
        PageKind::Arrival => W::TurningPageArrival,
        PageKind::Year => W::TurningPageYear,
    }]
}

fn height(text: &str) -> f32 {
    wrap(text, text_area().size().x, MIN_TEXT).len() as f32 * PITCH
}

/// A death page's epitaph: the dead's, as composed now. A death page whose dead has none
/// was made without its step 8 (SPEC §15.1), and is refused loudly.
pub fn page_epitaph<'h>(page: &TurnPage, heroes: &'h [Hero]) -> Option<&'h str> {
    let (PageKind::Death, Some(id)) = (page.kind, page.about) else {
        return None;
    };
    match &heroes[id].epitaph {
        Some(epitaph) => Some(epitaph),
        None => panic!(
            "[keifu_x_inheritance_r3v2] {}'s death page has no epitaph to set at its top\n  likely cause: the \
             page was made without composing it\n  fix: SPEC §15.1 step 8 composes it on the page",
            heroes[id].name
        ),
    }
}

/// What a page's first leaf sets above its lines: the title, the card, a death's epitaph.
fn first_header(page: &TurnPage, heroes: &[Hero]) -> f32 {
    let card = if page.about.is_some() {
        CARD.y + PART_GAP
    } else {
        0.0
    };
    let epitaph = match page_epitaph(page, heroes) {
        Some(epitaph) => height(epitaph) + PART_GAP,
        None => 0.0,
    };
    TITLE_PITCH + PART_GAP + card + epitaph
}

/// Whether a page waits for its heir.
fn waiting(page: &TurnPage) -> bool {
    page.bequest
        .as_ref()
        .is_some_and(crate::passage::Bequest::undecided)
}

/// The turning's leaves, in order: each page split under its heading, an undecided death
/// page keeping room on its last leaf for the choice. A death page's epitaph is read off
/// `heroes`, so the leaves follow it when it is recomposed.
pub fn leaves(passage: &Passage, heroes: &[Hero]) -> Vec<Leaf> {
    let room = text_area().size().y;
    let mut out = Vec::new();
    for (index, page) in passage.pages.iter().enumerate() {
        let foot = if waiting(page) { CHOICE_H } else { 0.0 };
        let mut at = 0;
        let mut first = true;
        loop {
            let mut used = PITCH
                + PART_GAP
                + if first {
                    first_header(page, heroes)
                } else {
                    0.0
                };
            let start = at;
            while at < page.lines.len() && used + height(&page.lines[at]) <= room {
                used += height(&page.lines[at]) + LINE_GAP;
                at += 1;
            }
            // INVARIANT: every leaf after the first holds a line, or the page could never end.
            assert!(
                at > start || first,
                "[keifu_x_inheritance_r3v2] a turning line is taller than a leaf: {:?}\n  likely cause: a very \
                 long line of content\n  fix: give the turning's panel more height",
                page.lines.get(at)
            );
            let done = at >= page.lines.len();
            // The choice needs its room under the last of the lines; else one more leaf.
            let fits = used + foot <= room;
            out.push(Leaf {
                page: index,
                first,
                last: done && fits,
                lines: start..at,
            });
            first = false;
            if done && fits {
                break;
            }
            if done {
                out.push(Leaf {
                    page: index,
                    first: false,
                    last: true,
                    lines: at..at,
                });
                break;
            }
        }
    }
    out
}

/// The last leaf a reader may reach now: the last of the first undecided page, else the
/// last of all.
pub fn furthest(passage: &Passage, leaves: &[Leaf]) -> usize {
    match passage.first_undecided() {
        Some(page) => last_leaf_of(leaves, page),
        None => leaves.len() - 1,
    }
}

/// The first leaf of `page`.
pub fn first_leaf_of(leaves: &[Leaf], page: usize) -> usize {
    match leaves.iter().position(|leaf| leaf.page == page) {
        Some(at) => at,
        None => no_leaf(page),
    }
}

/// The last leaf of `page`.
pub fn last_leaf_of(leaves: &[Leaf], page: usize) -> usize {
    match leaves.iter().rposition(|leaf| leaf.page == page) {
        Some(at) => at,
        None => no_leaf(page),
    }
}

fn no_leaf(page: usize) -> ! {
    panic!(
        "[keifu_x_inheritance_r3v2] turning page {page} has no leaf\n  likely cause: a page index from another \
         passage\n  fix: read pages and leaves off the same passage"
    )
}

/// Lay the turning out: the bar, the leaf on screen, its navigation, and the dock.
pub fn lay_out(page: &mut Page, content: &Content, house: &House, passage: &Passage, ui: &UiState) {
    let words = &content.words;
    lay_out_top_bar(page, content, house);
    page.shape(PANEL, ink::PANEL, layers::PANEL);
    let leaves = leaves(passage, &house.heroes);
    let current = ui.leaf.min(leaves.len() - 1);
    let leaf = &leaves[current];
    let turn = &passage.pages[leaf.page];
    let area = text_area();
    let kind = heading(content, turn.kind);
    let head = if leaf.first {
        kind.to_owned()
    } else {
        fmt(&words[W::TurningContinued], &[kind])
    };
    let mut y = text(page, &head, area.min.y, MIN_TEXT, ink::HEADING, None) + PART_GAP;
    if leaf.first {
        y = text(page, &turn.title, y, 16.0, ink::BODY, None) + PART_GAP;
        if let Some(id) = turn.about {
            let rect = Rect::from_min_size(Vec2::new(area.min.x, y), CARD);
            hero_card(
                page,
                content,
                &house.heroes,
                id,
                rect,
                ui.pointing == Some(id),
            );
            page.targets.push((rect, Target::Hero(id)));
            y += CARD.y + PART_GAP;
        }
        if let Some(epitaph) = page_epitaph(turn, &house.heroes) {
            y = text(page, epitaph, y, MIN_TEXT, ink::BODY, None) + PART_GAP;
        }
    }
    for line in &turn.lines[leaf.lines.clone()] {
        y = text(page, line, y, MIN_TEXT, ink::BODY, Some(house)) + LINE_GAP;
    }
    if leaf.last && waiting(turn) {
        lay_out_choice(page, content, house, leaf.page, turn, y);
    }
    lay_out_nav(page, content, passage, &leaves, current);
    crate::dock::lay_out(page, content, house, ui);
}

/// "WHO IS HIS HEIR? One choice. It cannot be unmade." and the heir buttons under it.
fn lay_out_choice(
    page: &mut Page,
    content: &Content,
    house: &House,
    index: usize,
    turn: &TurnPage,
    y: f32,
) {
    let Some(bequest) = &turn.bequest else {
        return;
    };
    let area = text_area();
    let dead = &house.heroes[bequest.dead];
    let his = content.lore.pronouns[dead.pronoun.index()]
        .possessive
        .to_uppercase();
    let prompt = fmt(&content.words[W::TurningHeirPrompt], &[&his]);
    let top = area.max.y - CHOICE_H;
    let y = y.max(top);
    text(page, &prompt, y, MIN_TEXT, ink::HEADING, None);
    let width = (area.size().x - HEIR_GAP) * 0.5;
    let buttons = heir_buttons(content, &house.heroes, bequest.dead, &bequest.heirs);
    for (at, choice) in buttons.iter().enumerate() {
        let (column, row) = (at / HEIR_ROWS, at % HEIR_ROWS);
        let rect = Rect::from_min_size(
            Vec2::new(
                area.min.x + column as f32 * (width + HEIR_GAP),
                y + PITCH + PART_GAP + row as f32 * (HEIR_H + HEIR_GAP),
            ),
            Vec2::new(width, HEIR_H),
        );
        button(
            page,
            rect,
            &choice.label,
            Target::Heir(index, choice.heir),
            layers::PANEL,
        );
    }
}

/// One logical line, wrapped into the text area from `y`; pointing at it points at the
/// hero it names first, when `house` is given. Returns the y below it.
fn text(
    page: &mut Page,
    line: &str,
    y: f32,
    size: f32,
    color: Color,
    house: Option<&House>,
) -> f32 {
    let area = text_area();
    let pitch = if size > MIN_TEXT { TITLE_PITCH } else { PITCH };
    let pieces = wrap(line, area.size().x, size);
    for (index, piece) in pieces.iter().enumerate() {
        let at = Vec2::new(area.min.x, y + index as f32 * pitch);
        if index == 0 {
            page.text(layers::TEXT, at, piece.clone(), size, color, PANEL);
        } else {
            page.continue_text(layers::TEXT, at, piece.clone(), size, color, PANEL);
        }
    }
    let bottom = y + pieces.len() as f32 * pitch;
    if let Some(id) = house.and_then(|house| about(&house.heroes, line)) {
        let rect = Rect::from_min_size(
            Vec2::new(area.min.x, y),
            Vec2::new(area.size().x, bottom - y),
        );
        page.targets.push((rect, Target::Hero(id)));
    }
    bottom
}

/// The numbered leaf buttons shown: all of them, or a window of `LEAF_BUTTONS` around
/// the leaf on screen when there are more (presentation; every leaf stays reachable).
pub fn leaf_window(count: usize, current: usize) -> std::ops::Range<usize> {
    if count <= LEAF_BUTTONS {
        return 0..count;
    }
    let start = current
        .saturating_sub(LEAF_BUTTONS / 2)
        .min(count - LEAF_BUTTONS);
    start..start + LEAF_BUTTONS
}

/// How many numbered leaf buttons fit left of "Skip ahead".
const LEAF_BUTTONS: usize = 12;

/// A numbered button per leaf, "Skip ahead", and "Go on" — "Summer comes" on the last;
/// "Go on" greyed where it cannot go on (an undecided page's last leaf).
fn lay_out_nav(
    page: &mut Page,
    content: &Content,
    passage: &Passage,
    leaves: &[Leaf],
    current: usize,
) {
    let words = &content.words;
    let top = PANEL.max.y - NAV_H + (NAV_H - NAV_BUTTON.y) * 0.5;
    let window = leaf_window(leaves.len(), current);
    for leaf in window.clone() {
        let slot = (leaf - window.start) as f32;
        let rect = Rect::from_min_size(
            Vec2::new(PANEL.min.x + PAD + slot * (NAV_BUTTON.x + 6.0), top),
            NAV_BUTTON,
        );
        button(
            page,
            rect,
            &(leaf + 1).to_string(),
            Target::Leaf(leaf),
            layers::PANEL,
        );
        if leaf == current {
            let mark = Rect::from_min_size(
                Vec2::new(rect.min.x, rect.max.y + 2.0),
                Vec2::new(NAV_BUTTON.x, 3.0),
            );
            page.shape(mark, ink::GOLD, layers::MARK);
        }
    }
    let next = if current + 1 < leaves.len() {
        W::TurningNext
    } else {
        W::TurningLast
    };
    let go_on = Rect::from_min_size(
        Vec2::new(PANEL.max.x - PAD - NAV_WIDE, top),
        Vec2::new(NAV_WIDE, NAV_BUTTON.y),
    );
    if current >= furthest(passage, leaves) && passage.first_undecided().is_some() {
        // Held: greyed, and a press does nothing (`pointer::press`).
        page.shape(go_on, ink::PANEL, layers::PANEL);
        let style = TextStyle {
            size: MIN_TEXT,
            ..TextStyle::default()
        };
        let label = &words[next];
        let at = Vec2::new(
            go_on.center().x - style.width_of(label) * 0.5,
            go_on.center().y - MIN_TEXT * 0.5,
        );
        page.text(layers::PANEL + 2, at, label, MIN_TEXT, ink::GONE, go_on);
        page.targets.push((go_on, Target::GoOn));
    } else {
        button(page, go_on, &words[next], Target::GoOn, layers::PANEL);
    }
    let skip = Rect::from_min_size(
        Vec2::new(go_on.min.x - 12.0 - NAV_WIDE, top),
        Vec2::new(NAV_WIDE, NAV_BUTTON.y),
    );
    button(
        page,
        skip,
        &words[W::TurningSkip],
        Target::Skip,
        layers::PANEL,
    );
}
