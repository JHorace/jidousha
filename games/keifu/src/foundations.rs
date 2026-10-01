//! W0's machinery, asked directly: content tables, the calendar, the text
//! conventions, the writing pools and the bags.
//!
//! These are contracts a played session of W0/W1 never exercises — nobody reaches
//! the last summer or picks a pool line yet — so they are put to the functions.
//! Every expectation is a literal read off the spec or the content by hand.

use jidousha::prelude::*;

use crate::calendar::Calendar;
use crate::chance::{Bag, between, chance, unclaimed_index};
use crate::checks::Checks;
use crate::content::Content;
use crate::family::top_bar;
use crate::house::House;
use crate::ids::{Destiny, DreamKind, Phase, Place, Pool, Tag};
use crate::text::{WritingMemory, count_words, name_list, party_telling, year_telling};

/// The lore tables' sizes and a reading from each (content/README.md).
fn check_tables(checks: &mut Checks, content: &Content) {
    let lore = &content.lore;
    let sizes = [
        lore.aptitudes.len(),
        lore.tags.len(),
        lore.places.len(),
        lore.phases.len(),
        lore.vocations.len(),
        lore.pronouns.len(),
        content.destinies.len(),
        content.dreams.len(),
        content.pools.len(),
        content.names.him.len(),
        content.names.her.len(),
        content.names.houses.len(),
    ];
    checks.require(
        sizes == [3, 8, 7, 5, 6, 2, 10, 9, 12, 32, 32, 16],
        "a content table has the wrong number of entries",
        format!("{sizes:?}"),
    );
    let readings = [
        lore.places[Place::Barrow.index()].title.as_str(),
        lore.places[Place::Barrow.index()].name.as_str(),
        lore.tags[Tag::Heights.index()].noun.as_str(),
        lore.phases[Phase::Elder.index()].telling.as_str(),
        lore.phases[Phase::Child.index()].note.as_str(),
        content.destinies[Destiny::Unspoken.index()]
            .prophecy
            .as_str(),
        content.dreams[DreamKind::SeeTheSea.index()].title.as_str(),
        lore.pronouns[1].object.as_str(),
    ];
    checks.require(
        readings
            == [
                "The Barrow",
                "the Barrow",
                "high places",
                "an elder",
                "{He} stays in the yard, and can be taught in winter.",
                "Not yet spoken.",
                "To see the sea",
                "her",
            ],
        "a lore reading is not the content's",
        format!("{readings:?}"),
    );
    let fragments: Vec<&str> = lore.phase_effect.iter().map(|(k, _)| k.as_str()).collect();
    checks.require(
        fragments.len() == 9 && fragments.contains(&"youth_until"),
        "the phase-effect fragments did not load",
        format!("{fragments:?}"),
    );
}

/// The calendar walked to the last summer, read through the top bar.
fn check_calendar(checks: &mut Checks, content: &Content, house: &mut House) {
    let mut seen = Vec::new();
    let mut years = Vec::new();
    for _ in 0..25 {
        house.calendar.begin_winter();
        let [_, season, ..] = top_bar(content, house);
        if seen.is_empty() {
            seen.push(season);
        }
        house.calendar.begin_summer();
        years.push(house.calendar.current_year());
        if house.calendar.current_year() == 25 {
            let [year, _, _, door, _] = top_bar(content, house);
            seen.push(year);
            seen.push(door);
        }
    }
    let [year, season, _, door, _] = top_bar(content, house);
    seen.extend([year, season, door]);
    // One year per turning: 2, 3, ... 26, written out rather than computed.
    let want: Vec<i32> = vec![
        2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    ];
    checks.require(
        years == want,
        "the calendar does not advance one year per summer",
        format!("{years:?}"),
    );
    checks.require(
        seen == [
            "Winter",
            "Year 25 of 25",
            "The Sealed Door opens in 1 year. It will ask for four.",
            "The last summer",
            "Summer",
            "The Sealed Door stands open. It asks for four.",
        ],
        "the calendar or the Door countdown reads wrong over the run",
        format!("{seen:?}"),
    );
    house.calendar = Calendar::start();
}

/// SPEC §21's conventions, read from content.
fn check_text(checks: &mut Checks, content: &Content) {
    let said = [
        count_words(content, 0),
        count_words(content, 3),
        count_words(content, 12),
        count_words(content, 13),
        year_telling(content, -18),
        year_telling(content, 1),
        year_telling(content, 25),
        year_telling(content, 26),
        name_list(content, &[]),
        name_list(content, &["Odo"]),
        name_list(content, &["Garrick", "Maren", "Pip"]),
        party_telling(content, &[]),
        party_telling(content, &["Odo", "Wren"]),
    ];
    checks.require(
        said == [
            "never",
            "three times",
            "twelve times",
            "13 times",
            "before the first year",
            "in year 1",
            "in year 25",
            "in the last summer",
            "no one yet",
            "Odo",
            "Garrick, Maren and Pip",
            "No one",
            "Odo and Wren",
        ],
        "a text convention reads wrong",
        format!("{said:?}"),
    );
}

/// Pools never repeat their last line, and reach every line; bags give each once.
fn check_draws(checks: &mut Checks, content: &Content, memory: &mut WritingMemory) {
    let mut rng = Rng::from_seed(0x9001);
    let mut repeats = 0;
    let mut reached = vec![false; content.pools[Pool::Wounds.index()].len()];
    let mut previous = None;
    for _ in 0..400 {
        let line = memory.pick(content, Pool::Wounds, &mut rng).to_owned();
        let index = memory.last(Pool::Wounds);
        if index == previous {
            repeats += 1;
        }
        if let Some(i) = index {
            reached[i] = true;
            checks.require(
                content.pools[Pool::Wounds.index()][i] == line,
                "a pool pick returned a line other than the one it remembered",
                line.clone(),
            );
        }
        previous = index;
    }
    checks.require(
        repeats == 0 && reached.iter().all(|r| *r),
        "a writing pool repeated its last line, or never reached one",
        format!("{repeats} repeats, reached {reached:?}"),
    );
    let mut bag = Bag::new(content.names.him.clone());
    let mut drawn: Vec<String> = (0..32).map(|_| bag.draw(&mut rng)).collect();
    drawn.sort();
    drawn.dedup();
    checks.require(
        drawn.len() == 32 && bag.left() == 0,
        "a name bag gave a name twice before refilling",
        format!("{} distinct, {} left", drawn.len(), bag.left()),
    );
    let dice: Vec<i32> = (0..6000).map(|_| between(&mut rng, 1, 6)).collect();
    let faces: Vec<usize> = (1..=6)
        .map(|f| dice.iter().filter(|d| **d == f).count())
        .collect();
    checks.require(
        faces.iter().all(|n| (850..=1150).contains(n)),
        "between(1, 6) is not a fair die",
        format!("{faces:?} of 6000"),
    );
    let hits = (0..10_000).filter(|_| chance(&mut rng, 0.6)).count();
    checks.require(
        (5800..=6200).contains(&hits),
        "chance(0.6) is off",
        format!("{hits} of 10000"),
    );
    let claimed = [true, true, false, true];
    let picks: Vec<usize> = (0..50)
        .map(|_| unclaimed_index(&mut rng, &claimed))
        .collect();
    checks.require(
        picks.iter().all(|p| *p == 2),
        "unclaimed_index picked a claimed index",
        format!("{picks:?}"),
    );
}

/// Every W0 check. Returns the summary line.
pub fn check(checks: &mut Checks, content: &Content) -> String {
    check_tables(checks, content);
    check_text(checks, content);
    match House::found(content, crate::verify::SEEDS[0]) {
        Ok(mut walked) => {
            check_draws(checks, content, &mut walked.writing);
            check_calendar(checks, content, &mut walked);
        }
        Err(error) => checks.require(false, "the house could not be founded", error),
    }
    "foundations: content tables, calendar to the last summer, text conventions, pools, bags, dice"
        .to_owned()
}
