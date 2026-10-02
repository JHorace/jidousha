//! The resolution battery: over a fixed battery of seeds, every house is played summer
//! after summer — parties seated from the house's likely ones, set out, the telling
//! left — and every quest the dice decide is tallied by the party's margin of power
//! over demand. The dice are the forecast's own numbers made real, so at each margin
//! the outcomes must sit inside bounds derived from CONSTANTS §3's 36ths.
//!
//! Each page is also held exactly: its margin is its power plus its dice less 7 and
//! its demand, and its outcome is that margin's band. Over all of them, each face of
//! a die turns up a sixth of the time, within the same bounds.
//!
//! INVARIANT: CONSTANTS §3's table is copied here by hand, entry by entry, as shipped;
//! nothing reads it back from `forecast.rs`, the code under test.

use jidousha::prelude::Rng;

use crate::board::Posted;
use crate::checks::Checks;
use crate::content::Content;
use crate::destiny::{fire_claims, mends, shields_on_quests};
use crate::house::House;
use crate::ids::Outcome;
use crate::reading::{adults, likely_party};
use crate::resolve::set_out;
use crate::season::leave_the_telling;
use crate::w6::band;

/// CONSTANTS §3, "Outcome odds by power minus demand": disaster, setback, success,
/// triumph in 36ths, from -10 (and below) to +9 (and above).
pub const TABLE: [(i32, [u32; 4]); 20] = [
    (-10, [36, 0, 0, 0]),
    (-9, [35, 1, 0, 0]),
    (-8, [33, 3, 0, 0]),
    (-7, [30, 6, 0, 0]),
    (-6, [26, 10, 0, 0]),
    (-5, [21, 14, 1, 0]),
    (-4, [15, 18, 3, 0]),
    (-3, [10, 20, 6, 0]),
    (-2, [6, 20, 10, 0]),
    (-1, [3, 18, 14, 1]),
    (0, [1, 14, 18, 3]),
    (1, [0, 10, 20, 6]),
    (2, [0, 6, 20, 10]),
    (3, [0, 3, 18, 15]),
    (4, [0, 1, 14, 21]),
    (5, [0, 0, 10, 26]),
    (6, [0, 0, 6, 30]),
    (7, [0, 0, 3, 33]),
    (8, [0, 0, 1, 35]),
    (9, [0, 0, 0, 36]),
];

/// The battery's seeds and how many summers each house is played.
const BATTERY: std::ops::Range<u64> = 0x6_6000..0x6_6000 + 400;
const SUMMERS: i32 = 6;

/// How far an observed count may stray: four standard deviations of a binomial, and
/// half a count for rounding. A cell §3 makes impossible (0 or 36 in 36) may not
/// stray at all.
fn within(observed: u32, trials: u32, ways: u32) -> bool {
    let n = f64::from(trials);
    let p = f64::from(ways) / 36.0;
    if ways == 0 || ways == 36 {
        return f64::from(observed) == n * p;
    }
    (f64::from(observed) - n * p).abs() <= 4.0 * (n * p * (1.0 - p)).sqrt() + 0.5
}

/// Seat each posted quest from the house's likely parties: on slot `s` of summer `y`
/// of seed `seed`, the first `1 + (seed + y + s) mod seats` of the likely party, from
/// the adults no earlier slot took — so parties of every size meet every demand.
fn seat_parties(house: &mut House, seed: u64, year: i32) {
    let mut free = adults(&house.heroes);
    for slot in 0..house.board.len() {
        let quest = house.board[slot].quest.clone();
        let party = likely_party(&house.heroes, &free, &quest, None);
        let take = 1 + ((seed as usize) + year as usize + slot) % quest.seats as usize;
        let party: Vec<usize> = party.into_iter().take(take).collect();
        free.retain(|id| !party.contains(id));
        for &hero in &party {
            house.unseat(hero);
        }
        let Posted { seats, .. } = &mut house.board[slot];
        for (at, &hero) in party.iter().enumerate() {
            seats[at] = Some(hero);
        }
    }
}

/// Run the battery. Returns the summary lines: one overall, one per margin observed.
pub fn check_battery(checks: &mut Checks, content: &Content) -> Vec<String> {
    let mut tally: Vec<[u32; 4]> = vec![[0; 4]; TABLE.len()];
    let mut faces = [0u32; 6];
    let mut rolls = [[0u32; 2]; 4];
    let (mut pages, mut summers, mut closed, mut deaths) = (0u32, 0u32, 0u32, 0usize);
    for seed in BATTERY {
        let mut rng = Rng::from_seed(seed);
        let mut house = match House::found(content, seed, &mut rng) {
            Ok(house) => house,
            Err(error) => crate::checks::fail("a battery house did not found", &error),
        };
        for _ in 0..SUMMERS {
            let year = house.calendar.current_year();
            seat_parties(&mut house, seed, year);
            let before = house.heroes.clone();
            set_out(content, &mut house, &mut rng);
            summers += 1;
            let telling = house.telling.clone().unwrap_or_default();
            for page in &telling.pages {
                pages += 1;
                let [a, b] = page.dice;
                let margin = page.power + a + b - 7 - page.quest.demand;
                checks.require(
                    (1..=6).contains(&a) && (1..=6).contains(&b) && page.margin == margin && page.outcome == band(margin),
                    "a resolved quest's dice, margin or outcome is not CONSTANTS §3's",
                    format!("seed {seed:#x}, year {year}, {:?}: power {} dice {a} {b} demand {} margin {} {:?}", page.quest.title, page.power, page.quest.demand, page.margin, page.outcome),
                );
                for die in page.dice {
                    if let Some(face) = faces.get_mut((die - 1).clamp(0, 5) as usize) {
                        *face += 1;
                    }
                }
                // The death roll: every member a disaster's roll reached (not claimed by
                // the fire, not mended, not shielded), and whether it killed them.
                if page.outcome == Outcome::Disaster {
                    let place = &content.lore.places[page.quest.place.index()].name;
                    for &m in &page.members {
                        let was = &before[m];
                        if fire_claims(was, &page.quest.tags, Outcome::Disaster)
                            || mends(was)
                            || shields_on_quests(was)
                        {
                            continue;
                        }
                        let danger = page.quest.danger.clamp(1, 4) as usize - 1;
                        rolls[danger][0] += 1;
                        rolls[danger][1] +=
                            u32::from(house.heroes[m].fate_telling == format!("fell at {place}"));
                    }
                }
                let gap = (page.power - page.quest.demand).clamp(-10, 9);
                tally[(gap + 10) as usize][page.outcome.index()] += 1;
            }
            leave_the_telling(content, &mut house, &mut rng);
            if house.closed {
                closed += 1;
                break;
            }
        }
        deaths += house
            .heroes
            .iter()
            .filter(|h| h.fate == crate::hero::Fate::Dead && h.fate_year >= 1)
            .count();
    }
    let mut lines = Vec::new();
    for (row, (gap, ways)) in TABLE.iter().enumerate() {
        let seen = tally[row];
        let n: u32 = seen.iter().sum();
        if n == 0 {
            continue;
        }
        let ok = (0..4).all(|o| within(seen[o], n, ways[o]));
        checks.require(
            ok,
            "outcomes at a margin stray outside the bounds CONSTANTS §3 sets",
            format!("power - demand {gap:+}: {n} quests, observed {seen:?}, §3 {ways:?} in 36ths"),
        );
        let cell = |o: usize| {
            format!(
                "{:.1}%/{:.1}%",
                100.0 * f64::from(seen[o]) / f64::from(n),
                100.0 * f64::from(ways[o]) / 36.0
            )
        };
        lines.push(format!(
            "W6 battery, power - demand {gap:+}: n {n}; disaster {}, setback {}, success {}, triumph {} (observed/§3)",
            cell(0), cell(1), cell(2), cell(3)
        ));
    }
    // CONSTANTS §3 DEATH_PER_DANGER: 15, 30, 45, 60 in 100 at danger 1-4.
    let mut deaths_line = Vec::new();
    for (danger, [n, hit]) in rolls.iter().enumerate() {
        let want = [15.0, 30.0, 45.0, 60.0][danger] / 100.0;
        let n_f = f64::from(*n);
        checks.require(
            (f64::from(*hit) - n_f * want).abs() <= 4.0 * (n_f * want * (1.0 - want)).sqrt() + 0.5,
            "a disaster's death roll does not kill at CONSTANTS §3's rate",
            format!(
                "danger {}: {hit} of {n} killed, want {:.0}%",
                danger + 1,
                want * 100.0
            ),
        );
        deaths_line.push(format!("danger {}: {hit}/{n}", danger + 1));
    }
    checks.require(
        rolls.iter().map(|[n, _]| n).sum::<u32>() > 400,
        "the battery's disasters rolled too few deaths to say anything",
        format!("{rolls:?}"),
    );
    let dice: u32 = faces.iter().sum();
    checks.require(
        faces.iter().all(|&f| within(f, dice, 6)),
        "a die's faces do not each turn up a sixth of the time",
        format!("faces {faces:?} of {dice}"),
    );
    checks.require(
        pages > 3000,
        "the battery resolved too few quests to say anything",
        format!("{pages} quests"),
    );
    let mut out = vec![format!(
        "W6 battery: {} houses x up to {SUMMERS} summers = {summers} summers, {pages} quests resolved, {closed} houses closed, {deaths} died questing; every page's margin and band exact; faces {faces:?}; each margin's outcomes within 4 sigma of CONSTANTS §3",
        BATTERY.end - BATTERY.start
    )];
    out.push(format!(
        "W6 battery, the death roll (rolled/killed, CONSTANTS §3 15/30/45/60%): {}",
        deaths_line.join(", ")
    ));
    out.extend(lines);
    out
}
