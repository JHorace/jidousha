//! The telling as data (SPEC §3.1 `tale`, §8): what set out wrote — one page per
//! resolved quest, in board order, and the "Meanwhile" lines; in the last summer, the
//! Door's prologue and one page per lock tried.
//!
//! Resolution (`resolve.rs`) writes it and nothing reads it but the telling screen
//! (`telling_view.rs`) and the checks. It is the house's, so the summer that made it
//! and the screen that shows it cannot disagree.

use crate::hero::HeroId;
use crate::ids::Outcome;
use crate::quest::Quest;

/// One resolved quest's page (SPEC §7.1 step 2: "The page records power, dice,
/// margin, outcome, members, and the story").
#[derive(Clone, Debug, PartialEq)]
pub struct QuestPage {
    /// The quest as it was posted.
    pub quest: Quest,
    /// Who went, in seat order (the dead and the crowned among them).
    pub members: Vec<HeroId>,
    /// What they brought (SPEC §6), at the moment it resolved.
    pub power: i32,
    /// The two dice.
    pub dice: [i32; 2],
    /// `power + d1 + d2 - 7 - demand`.
    pub margin: i32,
    /// The band the margin fell in.
    pub outcome: Outcome,
    /// The quest's ending for the outcome, the party's names in it.
    pub story: String,
    /// Everything the quest did, in the order it was written.
    pub lines: Vec<String>,
}

/// A summer's telling (SPEC §7 step 1: reset, and the year recorded).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Telling {
    /// The year it tells.
    pub year: i32,
    /// One page per resolved quest, in board order.
    pub pages: Vec<QuestPage>,
    /// Unanswered lines, the unanswered total, home healing — in that order.
    pub meanwhile: Vec<String>,
    /// The last summer's Door (SPEC §16.2): its prologue and its locks, whose pages are
    /// `pages`, one to a lock tried.
    pub door: Option<crate::door::DoorRecord>,
}
