//! The Ending (SPEC §23, `scene/scenes/ending.jai`): what the house does on entering it,
//! and the verdict it reads.
//!
//! Entering (`:12-15`), in order and once: **remember the fallen** — each hero still
//! mourned (the deaths of the last summer: the Door's, or the summer the house closed), in
//! order of death: a new wording rolled, the bequest decided, a held heirloom to the heir
//! by §14.2's rule (no choice) or recorded as buried, the dream's fate NEVER_DREAMT or
//! FULFILLED where it applies (an undone dream is neither passed nor ghosted), and the
//! epitaph composed — no death page, no further grief; then **remember the living** —
//! every living hero, in creation order, a new wording and an epitaph. Those are §22.2's
//! last draws: one wording per un-mourned dead and per living hero.
//!
//! The verdict: after the Door, its title and text by the locks opened, then one line per
//! member of the first lock's party — the dead fell at the last lock they stood at, the
//! bearers who opened locks came home, the rest came home together; once the house closed,
//! the closed title and verdict and the year it closed.

use jidousha::prelude::Rng;

use crate::content::Content;
use crate::door::DoorRecord;
use crate::epitaph::{recompose, remember, roll_wording};
use crate::hero::{DreamFate, HeroId};
use crate::house::House;
use crate::legacy::heir;
use crate::text::{fmt, name_list};
use crate::words::W;

/// How the house ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The Door was tried: how many locks gave, and the party's lines.
    Door {
        /// 0..=3.
        locks: usize,
        /// One line per member of the first lock's party, the rest together last.
        lines: Vec<String>,
    },
    /// The house closed: the year, and the years the Door was still off.
    Closed {
        /// The year it closed.
        year: i32,
        /// `years_until_door` then.
        years_off: i32,
    },
}

/// The Ending, once entered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ending {
    /// How it ended.
    pub verdict: Verdict,
}

impl Ending {
    /// The verdict's title: by locks opened, or the closed house's.
    pub fn title<'c>(&self, content: &'c Content) -> &'c str {
        match &self.verdict {
            Verdict::Door { locks, .. } => &content.door.verdict_titles[*locks],
            Verdict::Closed { .. } => &content.door.closed_title,
        }
    }
}

/// Enter the Ending (SPEC §23): remember the fallen, then the living, and keep the verdict
/// — the Door's when `door` is its record, the closed house's otherwise.
pub fn enter_the_ending(
    content: &Content,
    house: &mut House,
    rng: &mut Rng,
    door: Option<&DoorRecord>,
) {
    assert!(
        house.ending.is_none(),
        "[keifu_x_inheritance_r3v2] the Ending was entered twice\n  likely cause: the telling was left again \
         after the house ended\n  fix: SPEC §23 remembers once, on entering"
    );
    remember_the_fallen(content, house, rng);
    remember_the_living(content, house, rng);
    let verdict = match door {
        Some(record) => Verdict::Door {
            locks: record.locks_opened(),
            lines: verdict_lines(content, house, record),
        },
        None => Verdict::Closed {
            year: house.calendar.current_year(),
            years_off: house.calendar.years_until_door(),
        },
    };
    house.ending = Some(Ending { verdict });
}

/// §23 step 1: each hero still mourned, in order of death.
fn remember_the_fallen(content: &Content, house: &mut House, rng: &mut Rng) {
    for dead in std::mem::take(&mut house.mourned) {
        house.heroes[dead].wording = Some(roll_wording(&mut house.writing, rng));
        house.heroes[dead].bequest_decided = true;
        if let Some(heirloom) = house.heroes[dead].heirloom.take() {
            // SPEC-GAPS KG-68: the heir takes it with no deed and no line; the bequest
            // records it, as a death page's would.
            let to = heir(&house.heroes, dead);
            house.heroes[dead].bequest_heirloom = Some(heirloom.name.clone());
            house.heroes[dead].bequest_heir = to;
            if let Some(to) = to {
                house.heroes[to].heirloom = Some(heirloom);
            }
        }
        let hero = &mut house.heroes[dead];
        match &hero.dream {
            None => hero.dream_fate = DreamFate::NeverDreamt,
            Some(dream) if dream.is_fulfilled() => hero.dream_fate = DreamFate::Fulfilled,
            Some(_) => {}
        }
        recompose(content, &mut house.heroes, dead);
    }
}

/// §23 step 2: every living hero, in creation order, a new wording and an epitaph.
fn remember_the_living(content: &Content, house: &mut House, rng: &mut Rng) {
    for id in 0..house.heroes.len() {
        if house.heroes[id].is_living() {
            remember(content, &mut house.heroes, &mut house.writing, id, rng);
        }
    }
}

/// The verdict's lines (SPEC §23), one per member of the first lock's party in party
/// order: the dead "fell at" the last lock they stood at, a bearer who opened locks "opened
/// X and Y, and came home"; then the rest together, "came home", "too" after a bearer's.
/// SPEC-GAPS KG-67: the rest's line comes last, names them by first name, and says "too"
/// only after a bearer came home.
pub fn verdict_lines(content: &Content, house: &House, record: &DoorRecord) -> Vec<String> {
    let words = &content.words;
    let heroes = &house.heroes;
    let mut lines = Vec::new();
    let mut rest: Vec<HeroId> = Vec::new();
    let mut a_bearer_came_home = false;
    for &member in &record.party {
        let opened: Vec<&str> = record
            .tried
            .iter()
            .filter(|t| t.opened && t.bearer == member)
            .map(|t| content.door.locks[t.lock].name.as_str())
            .collect();
        let opened = if opened.is_empty() {
            String::new()
        } else {
            fmt(
                &words[W::DoorVerdictOpened],
                &[&name_list(content, &opened)],
            )
        };
        let hero = &heroes[member];
        if !hero.is_living() {
            let last = record
                .tried
                .iter()
                .rfind(|t| t.standing.contains(&member))
                .map(|t| content.door.locks[t.lock].name.as_str())
                .unwrap_or_default();
            lines.push(fmt(
                &words[W::DoorVerdictFell],
                &[&hero.full_name(), &opened, last],
            ));
        } else if !opened.is_empty() {
            lines.push(fmt(
                &words[W::DoorVerdictCameHome],
                &[&hero.full_name(), &opened],
            ));
            a_bearer_came_home = true;
        } else {
            rest.push(member);
        }
    }
    if !rest.is_empty() {
        let names: Vec<&str> = rest.iter().map(|&m| heroes[m].name.as_str()).collect();
        let too = if a_bearer_came_home {
            &words[W::DoorVerdictToo]
        } else {
            ""
        };
        lines.push(fmt(
            &words[W::DoorVerdictReturned],
            &[&name_list(content, &names), too],
        ));
    }
    lines
}

/// "N lived under this roof. <tally>" (SPEC §23, `ui.ending.lived_here`).
pub fn lived_here(content: &Content, house: &House) -> String {
    fmt(
        &content.words[W::EndingLivedHere],
        &[
            &house.heroes.len().to_string(),
            &crate::family::tally_sentence(content, house),
        ],
    )
}
