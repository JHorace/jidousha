//! The rule numbers W0 to W3 read, each copied from `spec/CONSTANTS.md`.
//!
//! Numbers are rules, not hand-authored words, so they live in source. Where a
//! number also appears in a content file, `content::load` checks the two agree
//! and refuses to start if they do not (CONSTANTS.md: "if they ever disagree, the
//! source line cited here wins" — a disagreement is a spec bug to report, never a
//! thing to paper over).

use crate::ids::{Aptitude, BondKind, DreamKind, Phase};

/// `DOOR_YEARS`: ordinary years before the Door (CONSTANTS §1).
pub const DOOR_YEARS: i32 = 25;
/// `HOUSE_RENOWN_AT_START` (CONSTANTS §1).
pub const HOUSE_RENOWN_AT_START: i32 = 15;
/// `Calendar.MONTHS_PER_YEAR`; months are seasons (CONSTANTS §1).
pub const MONTHS_PER_YEAR: usize = 4;
/// The calendar month that is summer (CONSTANTS §1).
pub const SUMMER: usize = 1;
/// The calendar month that is winter (CONSTANTS §1).
pub const WINTER: usize = 3;

/// `APTITUDE_LIMIT` (CONSTANTS §2).
pub const APTITUDE_LIMIT: i32 = 9;
/// `COMING_OF_AGE`: age >= 12 is adult (CONSTANTS §2).
pub const COMING_OF_AGE: i32 = 12;
/// Phase thresholds: Youth 12, Prime 20, Veteran 40, Elder 55 (CONSTANTS §2).
pub const PHASE_FROM_AGE: [i32; 5] = [0, COMING_OF_AGE, 20, 40, 55];
/// Phase adjustments, Might/Wits/Spirit, per phase (CONSTANTS §2).
pub const PHASE_ADJUSTMENT: [[i32; 3]; 5] =
    [[0, 0, 0], [-1, -1, -1], [0, 0, 0], [-1, 1, 0], [-2, 1, 0]];

/// The adjustment `phase` makes to `aptitude`.
pub fn phase_adjustment(phase: Phase, aptitude: Aptitude) -> i32 {
    PHASE_ADJUSTMENT[phase.index()][aptitude.index()]
}

/// `WOUND_PENALTY` (CONSTANTS §3).
pub const WOUND_PENALTY: i32 = 2;

/// `DREAD_LIMIT` (CONSTANTS §6).
pub const DREAD_LIMIT: i32 = 5;
/// `COURAGE_TO_CONQUER` (CONSTANTS §6).
pub const COURAGE_TO_CONQUER: i32 = 3;
/// `CONQUERED_FEAR_BONUS` (CONSTANTS §6).
pub const CONQUERED_FEAR_BONUS: i32 = 2;

/// `REST_DREAD_SHED`: dread shed by resting at the fire, not if broken (CONSTANTS §6).
pub const REST_DREAD_SHED: i32 = 1;

/// Fear penalty: `2 + dread / 2`, integer division (CONSTANTS §6).
pub fn fear_penalty(dread: i32) -> i32 {
    2 + dread / 2
}

/// Bond rank, power and grief, per kind (CONSTANTS §7).
pub const BOND_RANK_POWER_GRIEF: [(i32, i32, i32); 8] = [
    (0, 0, 0),
    (1, 1, 1),
    (1, -1, 0),
    (3, 2, 2),
    (2, 1, 1),
    (2, 1, 1),
    (4, 2, 2),
    (4, 2, 2),
];

/// A bond kind's rank: a bond is replaced only by a kind of strictly higher rank
/// (CONSTANTS §7, SPEC §12.1).
pub fn bond_rank(kind: BondKind) -> i32 {
    BOND_RANK_POWER_GRIEF[kind.index()].0
}

/// The power a bond of `kind` adds in a shared party (CONSTANTS §7).
pub fn bond_power(kind: BondKind) -> i32 {
    BOND_RANK_POWER_GRIEF[kind.index()].1
}

/// The dread a mourner holding a bond of `kind` to the dead takes (CONSTANTS §6, §7).
pub fn bond_grief(kind: BondKind) -> i32 {
    BOND_RANK_POWER_GRIEF[kind.index()].2
}

/// The mirror kind, as the other hero holds the bond (SPEC §12.1).
pub fn bond_mirror(kind: BondKind) -> BondKind {
    match kind {
        BondKind::Mentor => BondKind::Student,
        BondKind::Student => BondKind::Mentor,
        BondKind::Parent => BondKind::Child,
        BondKind::Child => BondKind::Parent,
        other => other,
    }
}

/// `DOOR_DESTINY_POWER`: +5 at each Door lock for the Door's promisee (CONSTANTS §8).
pub const DOOR_DESTINY_POWER: i32 = 5;
/// `CROWN_RENOWN`: personal renown the crown needs to claim (CONSTANTS §8).
pub const CROWN_RENOWN: i32 = 8;
/// `PATRON_POWER`: +1 per patron on every quest with anyone seated (CONSTANTS §8).
pub const PATRON_POWER: i32 = 1;
/// `OUTLIVING_DREAD`: extra grief for OUTLIVE_THOSE_YOU_LOVE, when grief applies (CONSTANTS §8).
pub const OUTLIVING_DREAD: i32 = 2;

/// `YARD_SPOTS`: children shown in the yard (CONSTANTS §9).
pub const YARD_SPOTS: usize = 6;
/// `HOUSEHOLD_LIMIT`: also the roster size (CONSTANTS §10).
pub const ROSTER_SEATS: usize = 12;

/// `LEGACY_HEIRLOOM_BONUS` (CONSTANTS §11).
pub const LEGACY_HEIRLOOM_BONUS: i32 = 2;
/// `TALE_YEARLY_RENOWN` (CONSTANTS §11).
pub const TALE_YEARLY_RENOWN: i32 = 1;
/// `DREAM_STAGE_COUNT` (CONSTANTS §11).
pub const DREAM_STAGE_COUNT: usize = 3;
/// `NARROW_BLESSING_POWER`: a blessing against a tag or at a place (CONSTANTS §11).
pub const NARROW_BLESSING_POWER: i32 = 2;
/// `BROAD_BLESSING_POWER`: a blessing on every quest (CONSTANTS §11).
pub const BROAD_BLESSING_POWER: i32 = 1;
/// Blade names: ten, used in order, cycling (CONSTANTS §11).
pub const BLADE_NAMES: usize = 10;
/// `COURT_DREAM_RENOWN`: "Earn 4 renown", personal renown (CONSTANTS §11).
pub const COURT_DREAM_RENOWN: i32 = 4;
/// The stage goals that are not 1: (dream, stage index, goal) (CONSTANTS §11).
pub const STAGE_GOALS_ABOVE_ONE: [(DreamKind, usize, i32); 3] = [
    (DreamKind::WorthyStudent, 0, 2),
    (DreamKind::WalkEveryRoad, 0, 3),
    (DreamKind::QuietTheBarrow, 1, 2),
];

/// A stage's goal: 1, except the three CONSTANTS §11 names.
pub fn stage_goal(kind: DreamKind, stage: usize) -> i32 {
    STAGE_GOALS_ABOVE_ONE
        .iter()
        .find(|(dream, index, _)| *dream == kind && *index == stage)
        .map_or(1, |(_, _, goal)| *goal)
}

/// `DOOR_LOCKS`: demands of the Might, Wits, Spirit locks (CONSTANTS §12).
pub const DOOR_LOCKS: [i32; 3] = [34, 34, 34];

/// Bonds shown on the hero sheet before "and N more" (CONSTANTS §14).
pub const BONDS_SHOWN: usize = 6;
