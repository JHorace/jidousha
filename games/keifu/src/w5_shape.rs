//! W5's shape: a fixed battery of seeds, each run through its summers, asked the
//! oracle's "most summers at least one quest carries a fair-chance Dream: mark" and
//! the board loop's stated rules, with the distribution printed.
//!
//! **What "its summers" are, before W6 and W8.** Nothing resolves a quest or turns
//! the year yet, so a run's summers are years 1 to 25 on the founding household as
//! founded: each summer advances the calendar and prepares the board (§5.1) from the
//! run's own generator, with the template memory carried. The demand creeps with the
//! year as it does in play; the household does not grow, age or die. That is the
//! state W5 can generate from, and the summary says so beside its numbers.
//!
//! **The bound.** The spec gives no rate for "most", so the conservative reading is
//! asserted: strictly more than half of the battery's summers. The battery is fixed,
//! so the run is deterministic; the bound was set before the battery was first run.
//!
//! INVARIANT: the rules' thresholds are written here as shipped literals — 18 of 36
//! is one half, 16 attempts, 30 reductions, the score's 2 and 1 — never read from
//! the constants under test.

use jidousha::prelude::Rng;

use crate::checks::Checks;
use crate::content::Content;
use crate::easing::Easing;
use crate::hero::HeroId;
use crate::house::House;
use crate::quest::Source;
use crate::quest_card::read_card;
use crate::reading::{could_go, pair_chances};

/// The battery: fixed seeds, each run through `SUMMERS` summers.
pub fn battery() -> Vec<u64> {
    (0x6_0000..0x6_0000 + 24).collect()
}

/// Years 1..=25: every ordinary summer (CONSTANTS §1 `DOOR_YEARS`, as a literal).
pub const SUMMERS: i32 = 25;

/// What the battery counts.
#[derive(Default)]
struct Tally {
    summers: usize,
    fair: usize,
    called_as_read: usize,
    per_seed: Vec<(u64, usize)>,
    per_year: Vec<(usize, usize)>,
    sort_violations: usize,
    remembered: usize,
    repeats: usize,
    memory_mismatches: usize,
    welcome_breaks: usize,
    welcome: usize,
    attempts: [usize; 17],
    eased: usize,
    easing_breaks: usize,
    stops: [usize; 3],
    reductions: usize,
}

/// One summer's card-level answer: the first quest, by slot, whose card names a
/// dreamer with a fair chance — read off the card's own "Dream:" line.
pub fn fair_mark(content: &Content, house: &House) -> Option<(usize, HeroId)> {
    (0..house.board.len()).find_map(|slot| {
        let card = read_card(content, house, slot, &[], None);
        let names = card.dream.as_deref()?.strip_prefix("Dream: ")?.to_owned();
        names.split(", ").find_map(|name| {
            let id = house.heroes.iter().position(|h| h.name == name)?;
            could_go(
                content,
                &house.heroes,
                &house.board[slot].quest,
                house.patrons,
                id,
            )
            .then_some((slot, id))
        })
    })
}

/// Run the battery; returns the summary lines.
pub fn check_shape(checks: &mut Checks, content: &Content) -> Vec<String> {
    let mut t = Tally {
        per_year: vec![(0, 0); SUMMERS as usize + 1],
        ..Tally::default()
    };
    for seed in battery() {
        let mut rng = Rng::from_seed(seed);
        let mut house = match House::found(content, seed, &mut rng) {
            Ok(house) => house,
            Err(error) => crate::checks::fail("a battery house could not be founded", &error),
        };
        let mut fair_here = 0;
        for year in 1..=SUMMERS {
            if year > 1 {
                house.calendar.begin_winter();
                house.calendar.begin_summer();
                house.prepare_summer(content, &mut rng);
            }
            let fair = fair_mark(content, &house).is_some();
            fair_here += usize::from(fair);
            t.fair += usize::from(fair);
            t.summers += 1;
            t.per_year[year as usize].0 += 1;
            t.per_year[year as usize].1 += usize::from(fair);
            properties(checks, &mut t, &house, seed, year);
        }
        t.per_seed.push((seed, fair_here));
    }
    let floors = staged_floor(checks, content);
    // MODULES.md W5, "most summers": strictly more than half.
    checks.require(
        t.fair * 2 > t.summers,
        "W5 oracle (shape): most summers carry a fair-chance Dream: mark",
        format!("{} of {} summers", t.fair, t.summers),
    );
    let rules = [
        (t.sort_violations, "the board is not in place order"),
        (t.repeats, "a drawn place repeated its remembered template"),
        (
            t.memory_mismatches,
            "the template memory is not the kept board's",
        ),
        (
            t.welcome_breaks,
            "the 16-attempt loop broke the welcome rule",
        ),
        (
            t.easing_breaks,
            "easing engaged or stopped against SPEC §5.2",
        ),
    ];
    for (count, what) in rules {
        checks.require(count == 0, what, format!("{count} summers"));
    }
    checks.require(
        t.eased > 0 && t.eased < t.summers && t.remembered > 0,
        "W5 battery: the battery never exercised easing or memory, or eased every board",
        format!(
            "eased {} of {}, remembered {}",
            t.eased, t.summers, t.remembered
        ),
    );
    let mut lines = summary(&t);
    lines.push(floors);
    lines
}

/// Easing's floor, staged: the household all wounded in year 25, so no likely party
/// forms and every chance is 0 — a tie at every step, which lowers the second quest
/// of the pair until its demand is one per seat (SPEC §5.2). Checked by the same
/// rules as the battery.
fn staged_floor(checks: &mut Checks, content: &Content) -> String {
    let mut t = Tally {
        per_year: vec![(0, 0); SUMMERS as usize + 1],
        ..Tally::default()
    };
    for seed in battery().into_iter().take(6) {
        let mut rng = Rng::from_seed(seed);
        let mut house = match House::found(content, seed, &mut rng) {
            Ok(house) => house,
            Err(error) => crate::checks::fail("a staged house could not be founded", &error),
        };
        for hero in &mut house.heroes {
            hero.wounded = true;
        }
        while house.calendar.current_year() < SUMMERS {
            house.calendar.begin_winter();
            house.calendar.begin_summer();
        }
        house.prepare_summer(content, &mut rng);
        properties(checks, &mut t, &house, seed, SUMMERS);
        let floored = house.board_report.as_ref().is_some_and(|r| {
            r.easing == Easing::Floor
                && r.pair_slots.is_some_and(|(_, second)| {
                    let q = &house.board[second].quest;
                    q.demand == q.seats && r.eased[second] > 0
                })
        });
        checks.require(
            floored,
            "W5 easing (staged, all wounded): a tie lowers the second of the pair to one demand per seat",
            format!("seed {seed:#x}: {:?}", house.board_report.as_ref().map(|r| (r.easing, &r.eased))),
        );
    }
    format!(
        "W5 easing (staged, all wounded, year {SUMMERS}): {} boards eased, stopped at the floor {} \
         ({} points), out of rule {}",
        t.eased, t.stops[1], t.reductions, t.easing_breaks
    )
}

/// The loop's stated rules on one posted summer.
fn properties(checks: &mut Checks, t: &mut Tally, house: &House, seed: u64, year: i32) {
    let Some(report) = house.board_report.as_ref() else {
        checks.require(
            false,
            "W5: a board was posted with no report",
            format!("seed {seed:#x} year {year}"),
        );
        return;
    };
    let board = &house.board;
    let places: Vec<usize> = board.iter().map(|p| p.quest.place.index()).collect();
    t.sort_violations += usize::from(board.len() != 4 || places.windows(2).any(|w| w[0] >= w[1]));
    // Template memory: a drawn quest never takes the remembered template; the
    // forced quests of year 1 and a ghost's quest are not drawn.
    let mut mismatch = false;
    for (slot, posted) in board.iter().enumerate() {
        let q = &posted.quest;
        let remembered = report.memory_before[q.place.index()];
        let forced = year == 1 && slot < 2;
        if let (Source::Template(template), false) = (q.source, forced) {
            t.remembered += usize::from(remembered.is_some());
            t.repeats += usize::from(remembered == Some(template));
        }
    }
    for place in 0..house.templates_last.len() {
        let on_board = board.iter().find(|p| p.quest.place.index() == place);
        let want = match on_board.map(|p| p.quest.source) {
            Some(Source::Template(template)) => Some(template),
            _ => report.memory_before[place],
        };
        mismatch |= house.templates_last[place] != want;
    }
    t.memory_mismatches += usize::from(mismatch);
    // The welcome rule: a plan is kept only if strictly better; the loop stops at the
    // first welcome (score 3 = answerable and calling); otherwise it runs 16.
    let scores = &report.scores;
    let best = scores.iter().copied().fold(f64::MIN, f64::max);
    let first_best = scores.iter().position(|s| *s == best).map(|i| i + 1);
    let first_welcome = scores.iter().position(|s| *s == 3.0);
    let ways = report.reading.ways;
    let literal = (f64::from(ways) / 36.0 / 0.5).min(1.0) * 2.0
        + if report.reading.call.is_some() {
            1.0
        } else {
            0.0
        };
    let stops = match first_welcome {
        Some(at) => scores.len() == at + 1,
        None => scores.len() == 16,
    };
    let welcome_ok = stops && first_best == Some(report.kept) && report.score == literal;
    t.welcome_breaks += usize::from(!welcome_ok);
    t.welcome += usize::from(first_welcome.is_some());
    t.attempts[scores.len().min(16)] += 1;
    t.called_as_read += usize::from(report.reading.call.is_some());
    // Easing engages exactly when the kept board's answerability is below one half
    // (18 of 36); it lowers only the remembered pair, at most 30 points, and stops for
    // the reason it says.
    let engaged = report.easing != Easing::NotNeeded;
    let total: i32 = report.eased.iter().sum();
    let mut easing_ok = engaged == (ways < 18);
    if engaged {
        t.eased += 1;
        t.reductions += total as usize;
        let Some((a, b)) = report.pair_slots else {
            t.easing_breaks += 1;
            return;
        };
        let off_pair = report
            .eased
            .iter()
            .enumerate()
            .any(|(s, e)| *e != 0 && s != a && s != b);
        let (ca, cb) = pair_chances(
            &house.heroes,
            &board[a].quest,
            &board[b].quest,
            house.patrons,
        );
        let weaker = if ca < cb { a } else { b };
        easing_ok &= !off_pair && total <= 30;
        match report.easing {
            Easing::BothAnswerable => {
                t.stops[0] += 1;
                easing_ok &= ca >= 18 && cb >= 18 && total < 30;
            }
            Easing::Floor => {
                t.stops[1] += 1;
                easing_ok &= !(ca >= 18 && cb >= 18)
                    && board[weaker].quest.demand <= board[weaker].quest.seats
                    && total < 30;
            }
            Easing::Limit => {
                t.stops[2] += 1;
                easing_ok &= total == 30;
            }
            Easing::NotNeeded => easing_ok = false,
        }
    } else {
        easing_ok &= total == 0;
    }
    if !easing_ok {
        t.easing_breaks += 1;
        checks.require(
            false,
            "W5: easing broke SPEC §5.2",
            format!(
                "seed {seed:#x} year {year}: {:?}, eased {:?}, ways {ways}",
                report.easing, report.eased
            ),
        );
    }
}

fn summary(t: &Tally) -> Vec<String> {
    let percent = |n: usize, of: usize| {
        if of == 0 {
            0.0
        } else {
            100.0 * n as f64 / of as f64
        }
    };
    let per_seed: Vec<String> = t
        .per_seed
        .iter()
        .map(|(seed, n)| format!("{seed:#x} {n}/{SUMMERS}"))
        .collect();
    let per_year: Vec<String> = t
        .per_year
        .iter()
        .enumerate()
        .skip(1)
        .map(|(year, (of, n))| format!("y{year} {:.0}%", percent(*n, *of)))
        .collect();
    let mean = t
        .attempts
        .iter()
        .enumerate()
        .map(|(n, c)| n * c)
        .sum::<usize>() as f64
        / t.summers.max(1) as f64;
    vec![
        format!(
            "W5 oracle (shape): {} seeds x {SUMMERS} summers = {} on the founding household as \
             founded (no resolution or turning before W6/W8); a fair-chance Dream: mark on a card \
             in {} ({:.1}%), bound: more than half (MODULES \"most\"; the spec gives no rate); the \
             reader's own call on the kept board in {} ({:.1}%)",
            t.per_seed.len(),
            t.summers,
            t.fair,
            percent(t.fair, t.summers),
            t.called_as_read,
            percent(t.called_as_read, t.summers)
        ),
        format!("W5 shape per seed: {}", per_seed.join(", ")),
        format!("W5 shape per year: {}", per_year.join(", ")),
        format!(
            "W5 properties: sort violations {}, template-memory repeats {} (of {} remembered \
             places drawn), memory mismatches {}, welcome-rule breaks {}, easing out of rule {}",
            t.sort_violations,
            t.repeats,
            t.remembered,
            t.memory_mismatches,
            t.welcome_breaks,
            t.easing_breaks
        ),
        format!(
            "W5 loop: welcome in {} summers; attempts mean {mean:.1}, 16 in {}; easing engaged in \
             {} (stopped both-answerable {}, floor {}, limit {}; {} points taken off)",
            t.welcome, t.attempts[16], t.eased, t.stops[0], t.stops[1], t.stops[2], t.reductions
        ),
    ]
}
