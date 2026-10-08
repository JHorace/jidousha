//! W10's full-dynasty battery: a fixed battery of houses founded and played to the end —
//! the W8 battery's way of playing each of the twenty-five years, then, in the last
//! summer, the outlook's best four sent to the Door (less whoever would refuse it), the
//! Door tried and the Ending entered — the port's last instrument, the one that reads
//! the code from the founding to the verdict.
//!
//! Its bounds were fixed before it first ran, and each is a rule of SPEC §16 or §23:
//!
//! - every house ends exactly once: it closes in a telling, or it tries the Door in the
//!   26th summer — or, with no living adult to send, cannot (counted and printed);
//! - the locks are tried in their order, and each lock's party is the last one's living;
//! - the verdict's title is the locks given, and its lines are one per dead member, one per
//!   living bearer, and one for the rest;
//! - each lock's chance of giving is the forecast's at the power it rolled (CONSTANTS §3
//!   against the Door's 34): over the battery, the locks given lie within 4 sigma of the
//!   sum of those chances;
//! - at the Ending every hero has an epitaph, none over six sentences, every one naming its
//!   subject; the mourned are remembered and mourned no longer; the frames rolled for the
//!   fallen and then the living never repeat one after another;
//! - no heirloom is held twice.
//!
//! It prints the verdict distribution, each lock's rate beside the forecast's, the outlook's
//! "all three" beside how often all three gave (OQ-15: the outlook multiplies three chances
//! as if nothing carried over), and the houses' survival shape — for two players: the W8
//! battery's, who never teaches, and the same player teaching every winter.
//!
//! INVARIANT: the rules' numbers here are shipped literals — 26, 6, 4 sigma, 36 — never
//! read from the constants under test.

use jidousha::prelude::Rng;

use crate::checks::Checks;
use crate::content::Content;
use crate::ending::Verdict;
use crate::epitaph::sentences;
use crate::forecast::forecast;
use crate::hero::HeroId;
use crate::house::House;
use crate::ids::Outcome;
use crate::play::{
    choose_every_heir, rotating, seat_answerable, seat_the_door, seat_the_lessons, seat_the_winter,
};
use crate::resolve::set_out;
use crate::season::{leave_the_telling, let_the_winter_pass, summer_comes};

/// The battery's houses.
const BATTERY: std::ops::Range<u64> = 0xa_0000..0xa_0000 + 240;

/// What the battery counts.
#[derive(Default)]
struct Tally {
    houses: usize,
    closed_by: [usize; 6],
    tried: usize,
    nobody: usize,
    verdicts: [usize; 4],
    lock_tried: [usize; 3],
    lock_opened: [usize; 3],
    lock_expected: [f64; 3],
    lock_variance: [f64; 3],
    outlook_sum: f64,
    all_three: usize,
    party_sizes: usize,
    adults_at_door: usize,
    deaths_at_door: usize,
    fallen: usize,
    living: usize,
    living_sentences: [usize; 7],
    broken: Vec<String>,
}

impl Tally {
    fn require(&mut self, ok: bool, what: impl FnOnce() -> String) {
        if !ok && self.broken.len() < 12 {
            self.broken.push(what());
        }
    }
}

/// Who plays the battery's houses: the W8 battery's player, or the same player who also
/// teaches every winter (`play::seat_the_lessons`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Player {
    /// Answerable quests, a wedding and a teller.
    Keeper,
    /// The keeper, and the training yard and the benches filled.
    Teacher,
}

/// Play `house` through its twenty-five years as `player` does. If it closes, the dead it
/// left mourned and the frame rolled last before its Ending.
fn play_the_years(
    content: &Content,
    house: &mut House,
    rng: &mut Rng,
    player: Player,
) -> Option<(Vec<HeroId>, Option<usize>)> {
    let seed = house.seed;
    for _ in 0..25 {
        seat_answerable(house);
        set_out(content, house, rng);
        let waiting = (house.mourned.clone(), house.writing.last_frame);
        leave_the_telling(content, house, rng);
        if house.closed {
            return Some(waiting);
        }
        seat_the_winter(house);
        if player == Player::Teacher {
            seat_the_lessons(content, house);
        }
        let_the_winter_pass(content, house, rng);
        choose_every_heir(content, house, rotating(seed));
        summer_comes(content, house, rng);
    }
    None
}

/// The Ending's remembering, held to §23 over one house: `mourned` were the dead it found
/// waiting, `last_frame` the frame rolled last before it.
fn remembering(
    t: &mut Tally,
    seed: u64,
    house: &House,
    mourned: &[HeroId],
    last_frame: Option<usize>,
) {
    let living: Vec<HeroId> = (0..house.heroes.len())
        .filter(|&h| house.heroes[h].is_living())
        .collect();
    t.fallen += mourned.len();
    t.living += living.len();
    t.require(house.mourned.is_empty(), || {
        format!("seed {seed:#x}: the Ending left the dead mourned")
    });
    let mut frames: Vec<usize> = last_frame.into_iter().collect();
    for &who in mourned.iter().chain(&living) {
        frames.extend(house.heroes[who].wording.map(|w| w.frame));
    }
    t.require(frames.windows(2).all(|w| w[0] != w[1]), || {
        format!("seed {seed:#x}: two wordings in a row share a frame: {frames:?}")
    });
    for (id, hero) in house.heroes.iter().enumerate() {
        let Some(epitaph) = &hero.epitaph else {
            t.require(false, || {
                format!("seed {seed:#x}: {} has no epitaph at the Ending", hero.name)
            });
            continue;
        };
        let count = sentences(epitaph);
        t.require(count <= 6 && epitaph.contains(&hero.full_name()), || {
            format!(
                "seed {seed:#x}: {}'s epitaph has {count} sentences or names no one: {epitaph:?}",
                hero.name
            )
        });
        if living.contains(&id) {
            t.living_sentences[count.min(6)] += 1;
        }
    }
    let mut held: Vec<&str> = house
        .heroes
        .iter()
        .filter_map(|h| h.heirloom.as_ref().map(|x| x.name.as_str()))
        .collect();
    let count = held.len();
    held.sort_unstable();
    held.dedup();
    t.require(held.len() == count, || {
        format!("seed {seed:#x}: an heirloom is held twice")
    });
}

/// The Door, held to §16.2 and §23 over one house.
fn the_door(t: &mut Tally, content: &Content, seed: u64, house: &mut House, rng: &mut Rng) {
    let adults = crate::reading::adults(&house.heroes).len();
    let best = crate::door::best_four(content, house);
    let party = seat_the_door(content, house);
    if party.is_empty() {
        t.nobody += 1;
        return;
    }
    t.tried += 1;
    t.party_sizes += party.len();
    t.adults_at_door += adults;
    t.outlook_sum += f64::from(best.all_percent());
    set_out(content, house, rng);
    let Some(telling) = house.telling.clone() else {
        t.require(false, || {
            format!("seed {seed:#x}: trying the Door wrote no telling")
        });
        return;
    };
    let Some(door) = telling.door.clone() else {
        t.require(false, || {
            format!("seed {seed:#x}: the last summer's telling has no Door")
        });
        return;
    };
    t.require(door.party == party && telling.meanwhile.is_empty(), || {
        format!(
            "seed {seed:#x}: the Door's party is not the seated four, or a Meanwhile was written"
        )
    });
    let mut standing = party.clone();
    for (at, (tried, page)) in door.tried.iter().zip(&telling.pages).enumerate() {
        t.require(
            tried.lock == at && page.quest.title == content.door.locks[at].title,
            || format!("seed {seed:#x}: lock {at} was tried out of order"),
        );
        t.require(tried.standing.iter().all(|m| standing.contains(m)), || {
            format!("seed {seed:#x}: someone stood at lock {at} who had not stood at the last")
        });
        t.require(tried.standing.contains(&tried.bearer), || {
            format!("seed {seed:#x}: lock {at}'s bearer was not standing before it")
        });
        standing = tried.standing.clone();
        let ways = forecast(page.power, 34, true);
        let p = f64::from(ways.ways(Outcome::Success) + ways.ways(Outcome::Triumph)) / 36.0;
        t.lock_tried[at] += 1;
        t.lock_expected[at] += p;
        t.lock_variance[at] += p * (1.0 - p);
        t.lock_opened[at] += usize::from(tried.opened);
        t.require(tried.opened == (page.outcome >= Outcome::Success), || {
            format!("seed {seed:#x}: lock {at} opened against its outcome")
        });
    }
    let opened = door.locks_opened();
    t.all_three += usize::from(opened == 3);
    let mourned = house.mourned.clone();
    let last_frame = house.writing.last_frame;
    leave_the_telling(content, house, rng);
    t.deaths_at_door += party
        .iter()
        .filter(|&&m| !house.heroes[m].is_living())
        .count();
    match house.ending.as_ref().map(|e| e.verdict.clone()) {
        Some(Verdict::Door { locks, lines }) => {
            t.verdicts[locks.min(3)] += 1;
            let dead = party
                .iter()
                .filter(|&&m| !house.heroes[m].is_living())
                .count();
            let bearers_home = party
                .iter()
                .filter(|&&m| {
                    house.heroes[m].is_living()
                        && door.tried.iter().any(|x| x.opened && x.bearer == m)
                })
                .count();
            let rest = party.len() - dead - bearers_home;
            t.require(locks == opened && lines.len() == dead + bearers_home + usize::from(rest > 0), || {
                format!("seed {seed:#x}: the verdict says {locks} locks and {} lines for {opened} opened, {dead} dead, {bearers_home} bearers home", lines.len())
            });
        }
        other => t.require(false, || {
            format!("seed {seed:#x}: after the Door the Ending is {other:?}")
        }),
    }
    remembering(t, seed, house, &mourned, last_frame);
}

/// A house founded on `seed` and played to its Ending by the teaching player, for the
/// floors and the pictures: its generator with it.
pub fn ended(content: &Content, seed: u64) -> (House, Rng) {
    let mut rng = Rng::from_seed(seed);
    let Ok(mut house) = House::found(content, seed, &mut rng) else {
        crate::checks::fail("a battery house did not found", &format!("seed {seed:#x}"));
    };
    if play_the_years(content, &mut house, &mut rng, Player::Teacher).is_none()
        && !seat_the_door(content, &mut house).is_empty()
    {
        set_out(content, &mut house, &mut rng);
        leave_the_telling(content, &mut house, &mut rng);
    }
    (house, rng)
}

/// Run the battery under both players; returns the summary lines.
pub fn check_full_dynasty(checks: &mut Checks, content: &Content) -> Vec<String> {
    let mut lines = battery(checks, content, Player::Keeper, "the keeper");
    lines.extend(battery(checks, content, Player::Teacher, "the teacher"));
    lines
}

/// One player's battery.
fn battery(checks: &mut Checks, content: &Content, player: Player, who: &str) -> Vec<String> {
    let mut t = Tally::default();
    for seed in BATTERY {
        t.houses += 1;
        let mut rng = Rng::from_seed(seed);
        let Ok(mut house) = House::found(content, seed, &mut rng) else {
            crate::checks::fail("a battery house did not found", &format!("seed {seed:#x}"));
        };
        if let Some((mourned, last_frame)) = play_the_years(content, &mut house, &mut rng, player) {
            let year = house.calendar.current_year();
            remembering(&mut t, seed, &house, &mourned, last_frame);
            t.closed_by[((year - 1) / 5).clamp(0, 5) as usize] += 1;
            t.require(
                matches!(
                    house.ending.as_ref().map(|e| &e.verdict),
                    Some(Verdict::Closed { .. })
                ),
                || format!("seed {seed:#x}: a closed house has no closed verdict"),
            );
            continue;
        }
        t.require(
            house.calendar.current_year() == 26 && house.board.len() == 1,
            || format!("seed {seed:#x}: the last summer is not the 26th, or not the Door alone"),
        );
        the_door(&mut t, content, seed, &mut house, &mut rng);
    }
    checks.require(
        t.broken.is_empty(),
        "a house played to the end broke a rule of the Door or the Ending",
        format!("{who}: {:?}", t.broken),
    );
    let mut within = true;
    let mut locks = Vec::new();
    for at in 0..3 {
        let sigma = t.lock_variance[at].sqrt().max(1e-9);
        let off = (t.lock_opened[at] as f64 - t.lock_expected[at]).abs() / sigma;
        within &= off <= 4.0;
        locks.push(format!(
            "{} {} of {} tried (forecast {:.1}, {:.1} sigma)",
            content.door.locks[at].name,
            t.lock_opened[at],
            t.lock_tried[at],
            t.lock_expected[at],
            off
        ));
    }
    checks.require(
        within,
        "the locks gave at a rate the forecast at their powers does not",
        format!("{who}: {}", locks.join("; ")),
    );
    checks.require(
        t.tried > 20,
        "too few houses reached the Door for the battery to say anything",
        format!("{who}: {} of {} tried the Door", t.tried, t.houses),
    );
    let closed: usize = t.closed_by.iter().sum();
    let per = |n: f64, of: usize| if of == 0 { 0.0 } else { n / of as f64 };
    vec![
        format!(
            "W10 full dynasty, {who}: {} houses founded and played to the end — {} closed before the Door (years 1-5: {}, 6-10: {}, 11-15: {}, 16-20: {}, 21-25: {}), {} tried the Door, {} could send no one; {} broken",
            t.houses,
            closed,
            t.closed_by[0],
            t.closed_by[1],
            t.closed_by[2],
            t.closed_by[3],
            t.closed_by[4],
            t.tried,
            t.nobody,
            t.broken.len()
        ),
        format!(
            "W10 verdicts, {who}: \"The Door stayed shut\" {}, \"opened a hand's breadth\" {}, \"stood half open\" {}, \"is open\" {}, \"The house closed its doors\" {}",
            t.verdicts[0], t.verdicts[1], t.verdicts[2], t.verdicts[3], closed
        ),
        format!(
            "W10 the locks, {who}, against CONSTANTS §3 at the powers they rolled: {}",
            locks.join("; ")
        ),
        format!(
            "W10 the outlook, {who}: the best four's \"all three open\" averaged {:.1} in 100; all three gave in {} of {} ({:.1}%) — the outlook a product of three chances as if nothing carried over (OQ-15)",
            per(t.outlook_sum, t.tried),
            t.all_three,
            t.tried,
            100.0 * per(t.all_three as f64, t.tried)
        ),
        format!(
            "W10 at the Door, {who}: mean party {:.2} of mean {:.1} living adults; {} died there; remembered at the Ending: {} fallen, {} living — the living's epitaphs by sentences 0..6: {:?}",
            per(t.party_sizes as f64, t.tried),
            per(t.adults_at_door as f64, t.tried),
            t.deaths_at_door,
            t.fallen,
            t.living,
            t.living_sentences
        ),
    ]
}
