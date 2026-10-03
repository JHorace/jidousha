//! W8's whole-year battery: a fixed battery of houses played year after year — the
//! answerable quests seated from the house's likely parties, the rest left unanswered,
//! the first pair who would wed sent to the garden and a teller to the table, the winter
//! let pass, every heir chosen by a rotating policy (no one, sometimes) — and every
//! turning held to the rules that are its own, now that years are real:
//!
//! - everyone living ages by one, and nobody else does;
//! - old age takes only the living, at their new age, with a SLEEP_DEATHS fate telling;
//! - the death pages are the summer's dead in order of death, then old age's in creation
//!   order, and no one else;
//! - births land on their rule: both parents living, 18..45 after the ageing, wed before
//!   this year, the first created first, fewer than three children before, a house never
//!   past twelve or a yard past six;
//! - comings of age land on twelve exactly, and every twelve-year-old has one;
//! - at most one wanderer, never into a house of ten, always to fewer than five adults;
//! - no dead hero acts: none is seated, on a quest page, in a winter seat, or changes
//!   after their death;
//! - every ghost raised comes from a death the telling told: a death page in this run,
//!   and, for a death on a quest, the quest page they were on.
//!
//! The battery also reads, beside W5's founded baseline, how often a played summer's
//! board carries a fair-chance Dream: mark. Its numbers are printed as a summary.
//!
//! INVARIANT: the rules' numbers are written here as shipped literals — 12, 18, 45, 3,
//! 6, 10, 5, 17, 36 — never read from the constants under test.

use std::collections::BTreeSet;

use jidousha::prelude::Rng;

use crate::checks::Checks;
use crate::content::Content;
use crate::hero::{DeedKind, Fate, Hero, HeroId};
use crate::house::House;
use crate::ids::{BondKind, Pool};
use crate::passage::PageKind;
use crate::play::{rotating, seat_answerable, seat_the_winter};
use crate::resolve::set_out;
use crate::season::{leave_the_telling, let_the_winter_pass, summer_comes};

/// The battery's houses, each played up to 25 years.
const BATTERY: std::ops::Range<u64> = 0x8_8000..0x8_8000 + 160;
const YEARS: i32 = 25;

/// What the battery counts.
#[derive(Default)]
struct Tally {
    turnings: usize,
    closed: usize,
    old_age: usize,
    quest_deaths: usize,
    pages: usize,
    waited: usize,
    no_one: usize,
    passed: usize,
    raised: usize,
    taken_up: usize,
    laid: usize,
    inherited: usize,
    buried: usize,
    births: usize,
    comings: usize,
    wanderers: usize,
    certain: usize,
    weddings: usize,
    living_at: Vec<(usize, usize)>,
    summers: usize,
    fair: usize,
    broken: Vec<String>,
}

impl Tally {
    fn require(&mut self, ok: bool, what: impl FnOnce() -> String) {
        if !ok && self.broken.len() < 12 {
            self.broken.push(what());
        }
    }
}

fn living(heroes: &[Hero]) -> usize {
    heroes.iter().filter(|h| h.is_living()).count()
}

/// Run the battery; returns the summary lines.
pub fn check_whole_years(checks: &mut Checks, content: &Content) -> Vec<String> {
    let mut t = Tally {
        living_at: vec![(0, 0); YEARS as usize + 1],
        ..Tally::default()
    };
    let mut ever_ghosts = 0;
    for seed in BATTERY {
        let mut rng = Rng::from_seed(seed);
        let Ok(mut house) = House::found(content, seed, &mut rng) else {
            crate::checks::fail("a battery house did not found", &format!("seed {seed:#x}"));
        };
        let mut told: BTreeSet<HeroId> = BTreeSet::new();
        let mut on_quest_pages: BTreeSet<(HeroId, i32)> = BTreeSet::new();
        for year in 1..=YEARS {
            t.summers += 1;
            t.fair += usize::from(crate::w5_shape::fair_mark(content, &house).is_some());
            t.living_at[year as usize].0 += 1;
            t.living_at[year as usize].1 += living(&house.heroes);
            seat_answerable(&mut house);
            let before_summer = house.heroes.clone();
            let ghosts_walking = house.ghosts.len();
            set_out(content, &mut house, &mut rng);
            t.laid += ghosts_walking - house.ghosts.len();
            let telling = house.telling.clone().unwrap_or_default();
            for page in &telling.pages {
                for &m in &page.members {
                    t.require(before_summer[m].is_living(), || {
                        format!(
                            "seed {seed:#x} y{year}: {} was dead and went",
                            before_summer[m].name
                        )
                    });
                    on_quest_pages.insert((m, year));
                }
            }
            for (id, was) in before_summer.iter().enumerate() {
                if was.is_living() && house.heroes[id].fate == Fate::Dead {
                    t.quest_deaths += 1;
                }
            }
            leave_the_telling(&mut house);
            if house.closed {
                t.closed += 1;
                break;
            }
            let seated: Vec<HeroId> = crate::hearth::Seat::all()
                .iter()
                .filter_map(|&s| house.hearth.at(s))
                .chain(house.roster.iter().flatten().copied())
                .collect();
            t.require(seated.iter().all(|&h| house.heroes[h].is_living()), || {
                format!("seed {seed:#x} y{year}: a dead hero sits at the hearth")
            });
            seat_the_winter(&mut house);
            t.weddings += usize::from(house.hearth.at(crate::hearth::Seat::Garden(0)).is_some());
            let before = house.clone();
            let_the_winter_pass(content, &mut house, &mut rng);
            t.turnings += 1;
            one_turning(&mut t, content, seed, year, &before, &house, &mut told);
            let ghosts_before = house.ghosts.len();
            let passed_before = count_fate(&house, crate::hero::DreamFate::PassedOn);
            crate::play::choose_every_heir(content, &mut house, rotating(seed));
            t.raised += house.ghosts.len().saturating_sub(ghosts_before);
            t.passed += count_fate(&house, crate::hero::DreamFate::PassedOn) - passed_before;
            chosen(&mut t, &house);
            for ghost in &house.ghosts {
                let dead = &house.heroes[ghost.hero];
                let page_told = told.contains(&ghost.hero);
                let quest_told = dead.death_place.is_none()
                    || on_quest_pages.contains(&(ghost.hero, dead.fate_year));
                t.require(dead.fate == Fate::Dead && page_told && quest_told, || {
                    format!(
                        "seed {seed:#x} y{year}: {}'s ghost walks with no death told",
                        dead.name
                    )
                });
            }
            ever_ghosts = ever_ghosts.max(house.ghosts.len());
            if year == YEARS {
                break;
            }
            summer_comes(content, &mut house, &mut rng);
        }
        for hero in &house.heroes {
            if hero.fate != Fate::Living {
                let late = hero
                    .deeds
                    .iter()
                    .any(|d| d.year > hero.fate_year && d.kind != DeedKind::Arrived);
                t.require(!late, || {
                    format!("seed {seed:#x}: {} has a deed after death", hero.name)
                });
            }
        }
    }
    let ok = t.broken.is_empty();
    checks.require(
        ok,
        "a turned year broke one of the turning's own rules",
        format!("{:?}", t.broken),
    );
    checks.require(
        t.births > 50 && t.comings > 50 && t.wanderers > 50 && t.old_age > 50 && t.raised > 0 && t.passed > 0,
        "the whole-year battery met too few births, comings of age, wanderers, old-age deaths, ghosts or passed dreams to say anything",
        format!(
            "births {}, comings {}, wanderers {}, old age {}, ghosts {}, passed {}",
            t.births, t.comings, t.wanderers, t.old_age, t.raised, t.passed
        ),
    );
    let mean = |y: usize| {
        let (n, sum) = t.living_at[y];
        if n == 0 { 0.0 } else { sum as f64 / n as f64 }
    };
    vec![
        format!(
            "W8 whole years: {} houses x up to {YEARS} years = {} turnings, {} houses closed; ages rose by one a year, no dead hero acted, births and comings of age on their rules' ages, every ghost from a death told — {} broken",
            BATTERY.end - BATTERY.start,
            t.turnings,
            t.closed,
            t.broken.len()
        ),
        format!(
            "W8 whole years, the turning: old age took {}, quests {}; {} death pages ({} waited for an heir: {} to no one, {} dreams passed, {} ghosts raised, {} heirlooms inherited, {} buried); {} births, {} comings of age, {} wanderers ({} certain); {} weddings sent; ghosts laid {}, taken up at a coming of age {}",
            t.old_age,
            t.quest_deaths,
            t.pages,
            t.waited,
            t.no_one,
            t.passed,
            t.raised,
            t.inherited,
            t.buried,
            t.births,
            t.comings,
            t.wanderers,
            t.certain,
            t.weddings,
            t.laid,
            t.taken_up
        ),
        format!(
            "W8 whole years, the household: mean living {:.1} in year 1, {:.1} in year 5, {:.1} in year 10, {:.1} in year 15, {:.1} in year 25 (houses still open: {}, {}, {}, {}, {})",
            mean(1),
            mean(5),
            mean(10),
            mean(15),
            mean(25),
            t.living_at[1].0,
            t.living_at[5].0,
            t.living_at[10].0,
            t.living_at[15].0,
            t.living_at[25].0
        ),
        format!(
            "W8 whole years, W5's mark on played summers: a fair-chance Dream: mark in {} of {} ({:.1}%) — beside W5's founded baseline, 568 of 600 (94.7%); the most ghosts walking at once {ever_ghosts}",
            t.fair,
            t.summers,
            100.0 * t.fair as f64 / t.summers.max(1) as f64
        ),
    ]
}

fn count_fate(house: &House, fate: crate::hero::DreamFate) -> usize {
    house.heroes.iter().filter(|h| h.dream_fate == fate).count()
}

/// After the heirs are chosen: how each waiting page was decided.
fn chosen(t: &mut Tally, house: &House) {
    let Some(passage) = &house.passage else {
        return;
    };
    for page in &passage.pages {
        let Some(bequest) = &page.bequest else {
            continue;
        };
        if !bequest.leaves {
            continue;
        }
        t.waited += 1;
        let heir = bequest.chosen.flatten();
        t.no_one += usize::from(heir.is_none());
        if house.heroes[bequest.dead].bequest_heirloom.is_some() {
            if heir.is_some() {
                t.inherited += 1;
            } else {
                t.buried += 1;
            }
        }
    }
}

/// One turning, held to its rules: `before` is the house as the winter was let pass.
fn one_turning(
    t: &mut Tally,
    content: &Content,
    seed: u64,
    year: i32,
    before: &House,
    after: &House,
    told: &mut BTreeSet<HeroId>,
) {
    let tag = |what: &str| format!("seed {seed:#x} y{year}: {what}");
    let Some(passage) = &after.passage else {
        t.require(false, || tag("no turning"));
        return;
    };
    let old = before.heroes.len();
    let mut old_age = Vec::new();
    for (id, was) in before.heroes.iter().enumerate() {
        let now = &after.heroes[id];
        if was.is_living() {
            t.require(now.age == was.age + 1, || {
                tag(&format!("{} aged {} to {}", was.name, was.age, now.age))
            });
            if now.fate == Fate::Dead {
                old_age.push(id);
                let sleep = content.pools[Pool::SleepDeaths.index()].contains(&now.fate_telling);
                t.require(
                    sleep
                        && now.fate_year == year
                        && now.fate_age == was.age + 1
                        && now.death_place.is_none(),
                    || tag(&format!("{} died of age wrongly", was.name)),
                );
            }
        } else {
            t.require(
                now.age == was.age && now.aptitudes == was.aptitudes && now.renown == was.renown,
                || tag(&format!("{}, dead, changed", was.name)),
            );
        }
    }
    t.old_age += old_age.len();
    // A ghost leaves the list at a turning only when a coming of age takes it up.
    t.taken_up += before.ghosts.len().saturating_sub(after.ghosts.len());
    let deaths: Vec<HeroId> = passage
        .pages
        .iter()
        .filter(|p| p.kind == PageKind::Death)
        .filter_map(|p| p.about)
        .collect();
    let want: Vec<HeroId> = before.mourned.iter().copied().chain(old_age).collect();
    t.require(deaths == want, || {
        tag(&format!("death pages {deaths:?}, want {want:?}"))
    });
    t.pages += deaths.len();
    told.extend(deaths);
    for page in &passage.pages {
        if let Some(b) = &page.bequest {
            t.require(b.heirs.len() <= 8 && !b.heirs.contains(&b.dead), || {
                tag("an heir list over eight or naming the dead")
            });
        }
    }
    // Births: the new heroes aged 0.
    for (id, child) in after.heroes.iter().enumerate().skip(old) {
        if child.deeds.iter().any(|d| d.kind == DeedKind::Arrived) {
            continue;
        }
        t.births += 1;
        let [Some(a), Some(b)] = child.parents else {
            t.require(false, || tag("a newborn without two parents"));
            continue;
        };
        let (pa, pb) = (&after.heroes[a], &after.heroes[b]);
        let since = pa
            .bond_to(b)
            .filter(|x| x.kind == BondKind::Spouse)
            .map(|x| x.since);
        let earlier = after.heroes[..id]
            .iter()
            .filter(|h| h.parents.contains(&Some(a)) && h.parents.contains(&Some(b)))
            .count();
        t.require(
            child.age == 0
                && child.born_year == year
                && a < b
                && pa.is_living()
                && pb.is_living()
                && (18..=45).contains(&pa.age)
                && (18..=45).contains(&pb.age)
                && since.is_some_and(|s| s < year)
                && earlier < 3
                && child.house == pa.house,
            || tag(&format!("{} born off the rule", child.name)),
        );
    }
    let alive = living(&after.heroes);
    let young = after
        .heroes
        .iter()
        .filter(|h| h.is_living() && h.age < 12)
        .count();
    t.require(alive <= 12 && young <= 6, || {
        tag(&format!("{alive} living, {young} children"))
    });
    // Comings of age: exactly the living who turned twelve.
    let came: Vec<HeroId> = (0..after.heroes.len())
        .filter(|&id| {
            after.heroes[id]
                .deeds
                .iter()
                .any(|d| d.kind == DeedKind::CameOfAge && d.year == year)
        })
        .collect();
    let twelve: Vec<HeroId> = (0..old)
        .filter(|&id| {
            before.heroes[id].is_living()
                && before.heroes[id].age == 11
                && after.heroes[id].is_living()
        })
        .collect();
    t.require(came == twelve, || {
        tag(&format!("came of age {came:?}, turned twelve {twelve:?}"))
    });
    for &id in &came {
        let hero = &after.heroes[id];
        t.require(
            hero.age == 12
                && hero.dream.is_some()
                && hero.destiny.kind != crate::ids::Destiny::Unspoken,
            || {
                tag(&format!(
                    "{} came of age without a dream or the Seer",
                    hero.name
                ))
            },
        );
    }
    t.comings += came.len();
    // The wanderer.
    let arrivals = passage
        .pages
        .iter()
        .filter(|p| p.kind == PageKind::Arrival)
        .count();
    let newcomers: Vec<&Hero> = after.heroes[old..]
        .iter()
        .filter(|h| h.deeds.iter().any(|d| d.kind == DeedKind::Arrived))
        .collect();
    let before_arrival = alive - newcomers.len();
    let adults_then = after
        .heroes
        .iter()
        .filter(|h| h.is_living() && h.age >= 12)
        .count()
        - newcomers.len();
    t.require(arrivals <= 1 && arrivals == newcomers.len(), || {
        tag("more than one wanderer")
    });
    t.require(arrivals == 0 || before_arrival < 10, || {
        tag("a wanderer into a house of ten")
    });
    t.require(
        arrivals == 1 || adults_then >= 5 || before_arrival >= 10,
        || tag("no wanderer to fewer than five adults"),
    );
    for w in newcomers {
        t.require((17..=36).contains(&w.age), || {
            tag(&format!("a wanderer of {}", w.age))
        });
    }
    t.wanderers += arrivals;
    t.certain += usize::from(arrivals == 1 && adults_then < 5);
}
