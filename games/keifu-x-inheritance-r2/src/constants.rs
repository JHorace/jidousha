//! The rule numbers W0 to W9 read, each copied from `spec/CONSTANTS.md`.
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
/// `DICE_SIDES`: two dice, each uniform 1..6 (CONSTANTS §3).
pub const DICE_SIDES: i32 = 6;
/// `DICE_MIDPOINT`: subtracted from the dice sum (CONSTANTS §3).
pub const DICE_MIDPOINT: i32 = 7;
/// `TRIUMPH_MARGIN`: margin >= 4 is a triumph (CONSTANTS §3).
pub const TRIUMPH_MARGIN: i32 = 4;
/// `SETBACK_MARGIN`: margin -1..-4 is a setback, <= -5 a disaster (CONSTANTS §3).
pub const SETBACK_MARGIN: i32 = 4;
/// `DEATH_PER_DANGER`: each member's death chance in a disaster, per danger (CONSTANTS §3).
pub const DEATH_PER_DANGER: f64 = 0.15;
/// `TRIUMPH_RENOWN`: extra renown on a triumph (CONSTANTS §3).
pub const TRIUMPH_RENOWN: i32 = 1;
/// `YOUTH_QUEST_LEARNING_LIMIT`: a youth learns on a won quest only while base < 5
/// (CONSTANTS §3).
pub const YOUTH_QUEST_LEARNING_LIMIT: i32 = 5;
/// The triumph lesson and the youth lesson: +1 each (CONSTANTS §3).
pub const QUEST_LESSON: i32 = 1;
/// `MAXIMUM_SEATS`: the party array's size (CONSTANTS §3).
pub const MAXIMUM_SEATS: usize = 4;
/// `QUEST_COUNT`: quests on a board (CONSTANTS §3).
pub const QUEST_COUNT: usize = 4;

/// `DEMAND_PER_SEAT` (CONSTANTS §4).
pub const DEMAND_PER_SEAT: i32 = 3;
/// `YEARS_PER_DEMAND_STEP`: +1 demand per seat every six years (CONSTANTS §4).
pub const YEARS_PER_DEMAND_STEP: i32 = 6;
/// `DEMAND_WOBBLE`: the wobble is uniform in -1..=+1 (CONSTANTS §4).
pub const DEMAND_WOBBLE: i32 = 1;
/// `TROUBLE_SEATS`: seats lost per trouble (CONSTANTS §4).
pub const TROUBLE_SEATS: i32 = 1;
/// `TROUBLE_DEMAND`: per-seat demand per trouble (CONSTANTS §4).
pub const TROUBLE_DEMAND: i32 = 1;
/// `DANGER_LIMIT` (CONSTANTS §4).
pub const DANGER_LIMIT: i32 = 4;
/// `TROUBLE_LIMIT`: the most trouble a place holds (CONSTANTS §4).
pub const TROUBLE_LIMIT: i32 = 2;
/// `UNANSWERED_RENOWN`: the base cost of an unanswered quest (CONSTANTS §4).
pub const UNANSWERED_RENOWN: i32 = 1;
/// `TROUBLED_RENOWN`: +1 if the quest was generated with trouble (CONSTANTS §4).
pub const TROUBLED_RENOWN: i32 = 1;
/// `RENOWN_PER_EXPECTATION`: + house renown / 20 (CONSTANTS §4).
pub const RENOWN_PER_EXPECTATION: i32 = 20;

/// `ANSWERABLE_CHANCE`: a board is answerable when its best pair's weaker chance of
/// success or better reaches this (CONSTANTS §4).
pub const ANSWERABLE_CHANCE: f64 = 0.5;
/// `DREAM_CALL_CHANCE`: a quest calls a dreamer who could go when their likely party's
/// chance of success or better reaches this (CONSTANTS §4).
pub const DREAM_CALL_CHANCE: f64 = 0.35;
/// `BOARD_ATTEMPTS`: the most boards planned in one summer (CONSTANTS §4).
pub const BOARD_ATTEMPTS: usize = 16;
/// `EASING_LIMIT`: the most single-point demand reductions when easing (CONSTANTS §4).
pub const EASING_LIMIT: usize = 30;
/// The board score's weight on answerability; a call adds `CALL_SCORE` (CONSTANTS §4,
/// "board score": `min(answerability / 0.5, 1) * 2 + (1 if calls a dreamer)`).
pub const ANSWERABLE_SCORE: f64 = 2.0;
/// The board score's term for calling a dreamer who could go (CONSTANTS §4).
pub const CALL_SCORE: f64 = 1.0;

/// `GHOST_SEATS`: a ghost quest's calm seats (CONSTANTS §5).
pub const GHOST_SEATS: i32 = 2;
/// `GHOST_DANGER`: a ghost quest's calm danger (CONSTANTS §5).
pub const GHOST_DANGER: i32 = 2;
/// A ghost quest needs Spirit (CONSTANTS §5).
pub const GHOST_APTITUDE: Aptitude = Aptitude::Spirit;

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

/// `FRIENDSHIP_AFTER`: companions become friends at two shared successes (CONSTANTS §7).
pub const FRIENDSHIP_AFTER: i32 = 2;
/// Parent and child in a failed party: +1 dread each (CONSTANTS §6).
pub const FAILURE_SEEN_DREAD: i32 = 1;

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
/// `MENDED_BONUS`: +1 to every base aptitude, capped, when mended (CONSTANTS §8).
pub const MENDED_BONUS: i32 = 1;
/// `MENDED_DREAD`: dread +2 when mended (CONSTANTS §8).
pub const MENDED_DREAD: i32 = 2;
/// `CARRIED_RENOWN`: extra house renown per CARRY_THE_HOUSE member on a won quest (CONSTANTS §8).
pub const CARRIED_RENOWN: i32 = 1;
/// `CARRIER_LOSS`: house renown lost when a CARRY_THE_HOUSE hero dies (CONSTANTS §8).
pub const CARRIER_LOSS: i32 = 4;
/// `HEIRS_OFFERED`: the most heirs a death page lists, and nearest kin reads (CONSTANTS §10).
pub const HEIRS_OFFERED: usize = 8;

/// `YARD_SPOTS`: children shown in the yard (CONSTANTS §9).
pub const YARD_SPOTS: usize = 6;
/// `FIRE_SEATS` (CONSTANTS §9).
pub const FIRE_SEATS: usize = 2;
/// Garden seats (CONSTANTS §9).
pub const GARDEN_SEATS: usize = 2;
/// `TALE_SEATS`: the long table (CONSTANTS §9).
pub const TALE_SEATS: usize = 2;
/// `BENCHES`: each a child seat and a teacher seat (CONSTANTS §9).
pub const BENCHES: usize = 2;
/// `SELF_TAUGHT_LIMIT`: training alone stops at this base in the vocation aptitude (CONSTANTS §9).
pub const SELF_TAUGHT_LIMIT: i32 = 6;
/// `CHILD_TAUGHT_LIMIT`: bench lessons stop at this base (CONSTANTS §9).
pub const CHILD_TAUGHT_LIMIT: i32 = 4;
/// `TEACHABLE_AGE`: children younger learn nothing on a bench (CONSTANTS §9).
pub const TEACHABLE_AGE: i32 = 6;
/// `WINTER_GAIN_LIMIT`: an adult lesson's cap before the TEACH_A_GREATER bonus (CONSTANTS §9).
pub const WINTER_GAIN_LIMIT: i32 = 2;
/// A lesson's base amount, alone or taught, and a bench lesson whole (CONSTANTS §9).
pub const WINTER_LESSON: i32 = 1;
/// Added to an adult lesson for a Youth learner, alone or taught (CONSTANTS §9).
pub const YOUNG_LEARNER_BONUS: i32 = 1;
/// Added to an adult lesson for a Veteran or Elder teacher (CONSTANTS §9).
pub const SEASONED_TEACHER_BONUS: i32 = 1;
/// `GREATER_LESSON`: from a TEACH_A_GREATER teacher, adult learners only (CONSTANTS §8).
pub const GREATER_LESSON: i32 = 1;
/// What a TEACH_A_GREATER teacher counts as knowing (CONSTANTS §9: `taught` counts as 9).
pub const GREATER_TAUGHT: i32 = APTITUDE_LIMIT;
/// `TALE_RENOWN`: per adult teller, and to the house once a winter (CONSTANTS §9).
pub const TALE_RENOWN: i32 = 1;
/// `MARRYING_AGE`: both must be this old to wed (CONSTANTS §9).
pub const MARRYING_AGE: i32 = 18;
/// `COURTING_AGE_GAP`: the most years apart two may be and wed (CONSTANTS §9).
pub const COURTING_AGE_GAP: i32 = 15;
/// `BIRTH_CHANCE`, in a hundred: per wed pair per turning (CONSTANTS §10). W8's rule;
/// the garden's help quotes it.
pub const BIRTH_CHANCE_PERCENT: i32 = 60;
/// `PARENT_AGE_HIGH`: both parents at most this old (CONSTANTS §10). W8's rule; the
/// garden's help quotes it, with `PARENT_AGE_LOW` = `MARRYING_AGE`.
pub const PARENT_AGE_HIGH: i32 = 45;
/// `HOUSEHOLD_LIMIT`: also the roster size (CONSTANTS §10).
pub const ROSTER_SEATS: usize = 12;

/// `OLD_AGE_BASE_CHANCE`: the old-age death chance at 55, after the turning's +1
/// (CONSTANTS §10).
pub const OLD_AGE_BASE_CHANCE: f64 = 0.04;
/// `OLD_AGE_YEARLY_CHANCE`: +2.5 points a year past 55 (CONSTANTS §10).
pub const OLD_AGE_YEARLY_CHANCE: f64 = 0.025;
/// The age old age begins to roll at: `ELDER_AGE` (CONSTANTS §2, §10).
pub const OLD_AGE_FROM: i32 = 55;
/// `OUTLIVING_AGE_FACTOR`: OUTLIVE_THOSE_YOU_LOVE's old-age multiplier (CONSTANTS §8).
pub const OUTLIVING_AGE_FACTOR: f64 = 0.5;
/// `BED_AGE_FACTOR`: DIE_IN_YOUR_BED's old-age multiplier (CONSTANTS §8).
pub const BED_AGE_FACTOR: f64 = 2.0;
/// `CHILDREN_PER_PAIR`: a pair's children, living or dead, before births stop (CONSTANTS §10).
pub const CHILDREN_PER_PAIR: usize = 3;
/// `HOUSEHOLD_LIMIT`: no birth when this many live (CONSTANTS §10).
pub const HOUSEHOLD_LIMIT: usize = 12;
/// `WANDERER_ROOM`: no wanderer when this many live (CONSTANTS §10).
pub const WANDERER_ROOM: usize = 10;
/// `WANDERER_BASE_CHANCE` (CONSTANTS §10).
pub const WANDERER_BASE_CHANCE: f64 = 0.2;
/// `WANDERER_RENOWN_DRAW`: +1 point a house renown (CONSTANTS §10).
pub const WANDERER_RENOWN_DRAW: f64 = 0.01;
/// `WANDERER_CHANCE_LIMIT` (CONSTANTS §10).
pub const WANDERER_CHANCE_LIMIT: f64 = 0.6;
/// `FEWEST_ADULTS`: fewer living adults and a wanderer comes for certain (CONSTANTS §10).
pub const FEWEST_ADULTS: usize = 5;
/// The most personal renown a wanderer arrives with, from 0 (CONSTANTS §10).
pub const WANDERER_RENOWN_HIGH: i32 = 2;
/// The least a newborn's base aptitude can be (CONSTANTS §10, the `max(.., 1)`).
pub const NEWBORN_APTITUDE_LEAST: i32 = 1;
/// `INHERITED_FEAR_CHANCE`: a parent's unconquered, unbroken fear passes (CONSTANTS §10).
pub const INHERITED_FEAR_CHANCE: f64 = 0.4;
/// `BROKEN_FEAR_CHANCE`: a broken parent's fear passes (CONSTANTS §10).
pub const BROKEN_FEAR_CHANCE: f64 = 0.8;
/// `BORN_BRAVE_CHANCE`: a conquered parent's child is born brave (CONSTANTS §10).
pub const BORN_BRAVE_CHANCE: f64 = 0.5;
/// `SURPASSING_CHILD_BONUS`: +2 to all three of a CHILD_WILL_SURPASS_YOU parent's first
/// child (CONSTANTS §8).
pub const SURPASSING_CHILD_BONUS: i32 = 2;
/// `TEACHER_AGE_FOR_DREAM`: WORTHY_STUDENT is not rolled for anyone younger (CONSTANTS §10).
pub const TEACHER_AGE_FOR_DREAM: i32 = 35;
/// `WANDERER_AGE_LOW`..`HIGH` (CONSTANTS §10).
pub const WANDERER_AGES: (i32, i32) = (17, 36);
/// Wanderer standings by house renown: threshold, gift low/high, other low/high
/// (CONSTANTS §10, "Wanderer standings").
pub const WANDERER_STANDINGS: [(i32, i32, i32, i32, i32); 3] =
    [(0, 3, 5, 1, 3), (12, 4, 6, 2, 3), (30, 5, 7, 2, 4)];
/// A first teacher's teaching shows at the coming of age: +1, below the cap (SPEC §17.5).
pub const TEACHING_SHOWS: i32 = 1;

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
/// `DOOR_SEATS`: the one party the Door asks for (CONSTANTS §12).
pub const DOOR_SEATS: i32 = 4;
/// `DOOR_DANGER`: each lock's danger — a death chance of 45% in a lock's disaster
/// (CONSTANTS §12).
pub const DOOR_DANGER: i32 = 3;
/// `DOOR_RENOWN`: renown per opened lock, house and each member (+1 on a triumph)
/// (CONSTANTS §12).
pub const DOOR_RENOWN: i32 = 5;
/// The best-four score's divisor: `P(all three) + (P1 + P2 + P3) / 1000` (CONSTANTS §12).
pub const BEST_FOUR_DIVISOR: f64 = 1000.0;

/// `EPITAPH_SENTENCES`: an epitaph's budget, counted as "." characters (CONSTANTS §13).
pub const EPITAPH_SENTENCES: usize = 6;
/// `EPITAPH_FRAME_COUNT`: the frames a wording rolls among (CONSTANTS §13).
pub const EPITAPH_FRAME_COUNT: usize = 3;

/// The telling's typewriter: story letters per second (CONSTANTS §14). Presentation:
/// render-side pacing over the fixed sim, which nothing in the house reads.
pub const TYPEWRITER_LETTERS_PER_SECOND: f32 = 90.0;

/// Bonds shown on the hero sheet before "and N more" (CONSTANTS §14).
pub const BONDS_SHOWN: usize = 6;

/// The variant's (Keifu X Inheritance, `tools/yakin/runs/keifu-x-inheritance-r2/DESIGN.md`
/// decision 2): the personal renown an outsider needs to wed into the family.
pub const MARRY_IN_RENOWN: i32 = 4;
/// The variant's (decision 3): an outsider wedding in brings `renown / DOWRY_SHARE` to
/// the house, in integer division.
pub const DOWRY_SHARE: i32 = 2;
/// The variant's (decision 9): house renown each mark on the name drains at the turning.
pub const MARK_DRAIN: i32 = 1;
/// The variant's (decision 9): a mark arrives at an heir at `weight / MARK_HALVING`;
/// a mark that reaches 0 is struck.
pub const MARK_HALVING: i32 = 2;
/// The variant's (decision 11): a newborn's aptitude is `dominant / DOMINANT_SHARE +
/// U{0,1}`, at least `NEWBORN_APTITUDE_LEAST`.
pub const DOMINANT_SHARE: i32 = 2;
/// The variant's (decision 11): the bonus to an aptitude both parents have as their best.
pub const BRED_TRUE_BONUS: i32 = 1;
