//! The variant's rules off the screen (VARIANT.md): who a quest is personal to, what an
//! outsider earns the house and inherits, and a child born under a mark. Each is staged on
//! the founded house and resolved on chosen dice, so the checks need no seed to land.
//!
//! INVARIANT: every expectation is a shipped literal, copied by hand from VARIANT.md.

use jidousha::prelude::Rng;

use crate::checks::Checks;
use crate::dream::Dream;
use crate::hero::{Fate, HeroId};
use crate::house::House;
use crate::ids::{DreamKind, Outcome, Place};
use crate::marks::{Mark, personal};
use crate::resolve::resolve_rolled;

/// A founded house in year 1's summer, and the content it was founded from.
fn founded() -> (crate::content::Content, House) {
    let content = match crate::content::load() {
        Ok(content) => content,
        Err(error) => crate::checks::fail("the content did not load", &error.to_string()),
    };
    let house = match House::found(&content, 1, &mut Rng::from_seed(1)) {
        Ok(house) => house,
        Err(error) => crate::checks::fail("the household did not found", &error.to_string()),
    };
    (content, house)
}

fn named(house: &House, name: &str) -> HeroId {
    match house.heroes.iter().position(|h| h.name == name) {
        Some(id) => id,
        None => crate::checks::fail("a founding hero is missing", name),
    }
}

/// A wanderer arrived (an outsider), aged `age`.
fn wanderer(content: &crate::content::Content, house: &mut House, age: i32) -> HeroId {
    let _ = crate::wanderer::arrive(content, house, &mut Rng::from_seed(3));
    let id = house.heroes.len() - 1;
    house.heroes[id].age = age;
    id
}

fn seat(house: &mut House, slot: usize, who: &[HeroId]) {
    for (at, &hero) in who.iter().enumerate() {
        house.unseat(hero);
        let seats = &mut house.board[slot].seats;
        if seats.len() <= at {
            seats.push(None);
        }
        seats[at] = Some(hero);
    }
}

/// Personal quests marked and not; outsiders; a child born under a mark.
pub fn check_rules(checks: &mut Checks) -> String {
    // A disaster on Garrick alone: marked, the house at 15 - 2 (the danger) - 2 = 11,
    // Garrick at 6 - 2 = 4, the mark exactly (year 1, the Barrow, generation 0).
    let (content, mut house) = founded();
    let (garrick, brannoc) = (named(&house, "Garrick"), named(&house, "Brannoc"));
    seat(&mut house, 0, &[garrick]);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(4), 0, [1, 1]);
    let mark = Mark {
        origin: garrick,
        year: 1,
        place: Place::Barrow,
        generation: 0,
    };
    checks.require(
        page.outcome == Outcome::Disaster
            && house.heroes[garrick].marks == [mark]
            && house.renown == 11
            && house.heroes[garrick].renown == 4,
        "a disaster on Garrick's personal quest marks the name exactly once: the house 15 to 11, Garrick 6 to 4",
        format!("{:?} {:?} {} {}", page.outcome, house.heroes[garrick].marks, house.renown, house.heroes[garrick].renown),
    );
    // Not personal: Brannoc alone, the same disaster — no mark, the house falls by the danger.
    let (content, mut house) = founded();
    seat(&mut house, 0, &[brannoc]);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(4), 0, [1, 1]);
    checks.require(
        house.heroes.iter().all(|h| h.marks.is_empty())
            && !page.lines.iter().any(|l| l.contains("the name is marked"))
            && house.renown == 13,
        "a disaster on a quest that is no one's personal quest marks no one: the house 15 to 13",
        format!("{} {:?}", house.renown, page.lines),
    );
    // Only family is marked: an outsider carrying Garrick's dream at its third stage.
    let (content, mut house) = founded();
    let stranger = wanderer(&content, &mut house, 30);
    let mut dream = match Dream::build(&content, DreamKind::QuietTheBarrow, None, None) {
        Ok(dream) => dream,
        Err(error) => crate::checks::fail("a dream did not build", &error),
    };
    dream.advance_to_stage(2);
    house.heroes[stranger].dream = Some(dream);
    let quest = house.board[0].quest.clone();
    checks.require(
        personal(&content, &house.heroes, quest.facts(), &[stranger]).is_empty()
            && personal(&content, &house.heroes, quest.facts(), &[garrick]) == [garrick],
        "a quest is personal to family only: Garrick yes, an outsider with his dream no",
        String::new(),
    );
    // Outsiders earn the house nothing; with family beside them the house earns.
    let (content, mut house) = founded();
    let (maren, stranger) = (named(&house, "Maren"), wanderer(&content, &mut house, 30));
    house.heroes[stranger].aptitudes = [9, 9, 9];
    seat(&mut house, 1, &[stranger]);
    house.board[1].quest.demand = 1;
    let (renown, own) = (house.renown, house.heroes[stranger].renown);
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(4), 1, [6, 6]);
    checks.require(
        house.renown == renown
            && house.heroes[stranger].renown == own + page.quest.renown + 1
            && page.lines[0].contains("No one of the name went, so the house has none."),
        "an outsider alone on a won quest earns only themselves, and the page says the house has none",
        format!("{:?} house {} (was {renown})", page.lines, house.renown),
    );
    seat(&mut house, 2, &[maren, stranger]);
    house.board[2].quest.demand = 1;
    let renown = house.renown;
    let page = resolve_rolled(&content, &mut house, &mut Rng::from_seed(4), 2, [6, 6]);
    checks.require(
        house.renown == renown + page.quest.renown + 1
            && page.lines[0].ends_with("renown to the house, and to each who went."),
        "with family in the party the house earns the quest's renown",
        format!("{:?} house {} (was {renown})", page.lines, house.renown),
    );
    // An outsider is never an heir and an outsider's death page never waits.
    let (content, mut house) = founded();
    let stranger = wanderer(&content, &mut house, 94);
    let list = crate::heirs::heirs(&house.heroes, garrick);
    house.heroes[stranger].heirloom = house.heroes[garrick].heirloom.clone();
    house.heroes[stranger].fate = Fate::Dead;
    let page =
        crate::death_page::death_page(&content, &mut house, stranger, &mut Rng::from_seed(5));
    let waits = page.bequest.is_some_and(|b| b.leaves);
    checks.require(
        !list.contains(&stranger)
            && !waits
            && page.lines.iter().any(|l| l.contains("was never of the name"))
            && page.lines.iter().any(|l| l.starts_with("Thornfall was laid in the ground")),
        "an outsider is not offered as an heir, and their death page never waits: nothing passes down, the heirloom is buried",
        format!("{list:?} {:?}", page.lines),
    );
    // A child is born under the marks both parents carry, once.
    let (content, mut house) = founded();
    let (maren, brannoc) = (named(&house, "Maren"), named(&house, "Brannoc"));
    crate::bonds::form(
        &mut house.heroes,
        maren,
        brannoc,
        crate::ids::BondKind::Spouse,
        1,
    );
    house.heroes[maren].marks.push(Mark {
        origin: maren,
        year: 1,
        place: Place::Barrow,
        generation: 0,
    });
    house.calendar.begin_winter();
    house.calendar.begin_summer();
    house.calendar.begin_winter();
    let mut born = None;
    for seed in 0..200 {
        let mut copy = house.clone();
        if let Some(page) =
            crate::births::births(&content, &mut copy, &mut Rng::from_seed(seed)).first()
        {
            born = page.about.map(|child| {
                (
                    copy.heroes[child].marks.clone(),
                    copy.marks_carried().len(),
                    page.lines.clone(),
                )
            });
            break;
        }
    }
    checks.require(
        born.as_ref().is_some_and(|(marks, carried, lines)| {
            *marks == [Mark { origin: maren, year: 1, place: Place::Barrow, generation: 1 }]
                && *carried == 1
                && lines.iter().any(|l| l.ends_with("is born under a mark on the name."))
        }),
        "a child of a marked parent is born under that mark, a generation down, and the house still counts it once",
        format!("{born:?}"),
    );
    "X1 rules: a personal disaster marks (15 to 11), a non-personal one does not, only family is marked, outsiders earn the house nothing and never inherit, a child is born under its parent's mark".to_owned()
}
