//! The turning's pages (SPEC §3.1 `passage`, §18, §18.1): what the turn of the year
//! wrote, page by page, in the order it was written.
//!
//! The winter's page comes first; then a death page for each of the mourned, a page
//! for each birth, each coming of age and the wanderer's arrival, and the "Year N
//! begins" page; the last page carries the turning's closing line. A death page that
//! leaves something to someone holds its heir list, fixed when the page was made
//! (§15.1), and stays undecided until the player chooses (§15.2) — and while one is
//! undecided the year does not turn (`season::summer_comes`).

use crate::hero::HeroId;
use crate::winter::WinterPlan;

/// Which kind of page (SPEC §18.1): its heading is `ui.turning.page_kinds.<KIND>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageKind {
    /// "What the winter did".
    Winter,
    /// "The house is one fewer".
    Death,
    /// "The house is one more".
    Birth,
    /// "The Seer speaks".
    ComingOfAge,
    /// "A wanderer at the door".
    Arrival,
    /// "The year turns".
    Year,
}

/// What a death page leaves, and to whom (SPEC §15.1-§15.2).
#[derive(Clone, Debug, PartialEq)]
pub struct Bequest {
    /// The dead.
    pub dead: HeroId,
    /// Whether there is anything to leave — an heirloom or an undone dream — and so a
    /// choice to wait for; a page with nothing to leave is decided when it is made.
    pub leaves: bool,
    /// The heirs offered, in rank order, fixed when the page was made (SPEC §15.1).
    pub heirs: Vec<HeroId>,
    /// The choice, once made: an heir, or no one. `None` while undecided.
    pub chosen: Option<Option<HeroId>>,
    /// Where the choice's lines go: right after the bequest lines (§15.2).
    pub bequest_end: usize,
}

impl Bequest {
    /// Whether the page still waits for its choice.
    pub fn undecided(&self) -> bool {
        self.leaves && self.chosen.is_none()
    }
}

/// One page of the turning.
#[derive(Clone, Debug, PartialEq)]
pub struct TurnPage {
    /// Its kind.
    pub kind: PageKind,
    /// "The winter of year 1", "In memory of Garrick Thorne", "Year 2 begins".
    pub title: String,
    /// Its lines, in the order they were written.
    pub lines: Vec<String>,
    /// A death page's bequest.
    pub bequest: Option<Bequest>,
    /// The hero the page is about (the dead, the newborn, the newcomer), for its card.
    pub about: Option<HeroId>,
}

/// This turning's pages, from the winter's passing until summer comes.
#[derive(Clone, Debug, PartialEq)]
pub struct Passage {
    /// The year whose winter it tells.
    pub year: i32,
    /// The pages, in the order the turning wrote them; the winter's first.
    pub pages: Vec<TurnPage>,
    /// What each winter seat did, for the checks (the previews were drawn from the same).
    pub done: WinterPlan,
}

impl Passage {
    /// The first page still waiting for an heir, if any.
    pub fn first_undecided(&self) -> Option<usize> {
        self.pages
            .iter()
            .position(|page| page.bequest.as_ref().is_some_and(Bequest::undecided))
    }
}
