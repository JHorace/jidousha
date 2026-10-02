//! The turn of the year (SPEC §18, `lineage/passage.jai:56-112`): everything that
//! happens between the winter let pass and the summer that comes, in the stated order.
//!
//! In order: the winter's page; everyone living a year older; old age; a death page for
//! each of the mourned, the summer's dead first, then the mourned cleared; births;
//! comings of age; a wanderer; the house's tales told; for each living hero, their phase
//! line if their phase changed (not into Youth), then the year's dream moment; "Year N+1
//! begins" if those last two steps wrote anything; and the closing line on the last page.
//! The death pages that leave something wait for the player (`heirs::choose`); nothing
//! in the year after them waits on their choice.

use jidousha::prelude::Rng;

use crate::births::births;
use crate::coming_of_age::comings_of_age;
use crate::constants::TALE_YEARLY_RENOWN;
use crate::content::Content;
use crate::death_page::{death_page, old_age};
use crate::house::House;
use crate::ids::Phase;
use crate::moment::Moment;
use crate::newcomers::phase_effect;
use crate::passage::{PageKind, Passage, TurnPage};
use crate::text::{fmt, lowered};
use crate::wanderer::wanderer;
use crate::winter::WinterPlan;
use crate::witness::witness;
use crate::words::W;

/// Turn the year after the winter (`winter`, its lines; `done`, its seats' plan).
pub fn turn_the_year(
    content: &Content,
    house: &mut House,
    winter: Vec<String>,
    done: WinterPlan,
    rng: &mut Rng,
) {
    let words = &content.words;
    let year = house.calendar.current_year();
    // 1. The winter's page.
    let lines = if winter.is_empty() {
        vec![words[W::TurningQuietWinter].to_owned()]
    } else {
        winter
    };
    let mut pages = vec![TurnPage {
        kind: PageKind::Winter,
        title: fmt(&words[W::TurningWinterTitle], &[&year.to_string()]),
        lines,
        bequest: None,
        about: None,
    }];
    // 2. Ageing, with each phase remembered for step 9.
    let phases_before: Vec<Option<Phase>> = house
        .heroes
        .iter()
        .map(|h| h.is_living().then(|| h.phase()))
        .collect();
    for hero in house.heroes.iter_mut().filter(|h| h.is_living()) {
        hero.age += 1;
    }
    // 3. Old age.
    old_age(content, house, rng);
    // 4. The death pages, in order of death; then the mourned are cleared.
    let mourned = std::mem::take(&mut house.mourned);
    for dead in mourned {
        pages.push(death_page(content, house, dead));
    }
    // 5-7. Births, comings of age, a wanderer.
    pages.extend(births(content, house, rng));
    pages.extend(comings_of_age(content, house, rng));
    pages.extend(wanderer(content, house, rng));
    // 8. The tales.
    let mut year_lines = tales(content, house);
    // 9. Phase lines and the year's moment, hero by hero.
    for id in 0..house.heroes.len() {
        if !house.heroes[id].is_living() {
            continue;
        }
        let hero = &house.heroes[id];
        let now = hero.phase();
        // SPEC-GAPS KG-56: a hero new this turning (born, or arrived) has no phase before it.
        let before = phases_before.get(id).copied().flatten();
        if before.is_some_and(|b| b != now) && now != Phase::Youth {
            year_lines.push(fmt(
                &words[W::PhaseChanged],
                &[
                    &hero.name,
                    &hero.age.to_string(),
                    &content.lore.phases[now.index()].telling,
                    &phase_effect(content, now, hero.pronoun),
                ],
            ));
        }
        year_lines.extend(witness(content, house, id, &Moment::YearTurn));
    }
    // 10. "Year N+1 begins".
    if !year_lines.is_empty() {
        pages.push(TurnPage {
            kind: PageKind::Year,
            title: fmt(&words[W::TurningYearTitle], &[&(year + 1).to_string()]),
            lines: year_lines,
            bequest: None,
            about: None,
        });
    }
    // 11. The closing line, on the last page.
    let closing = closing_line(content, house);
    if let Some(last) = pages.last_mut() {
        last.lines.push(closing);
    }
    house.passage = Some(Passage { year, pages, done });
}

/// The tales told at the turning (SPEC §14.3): +1 renown a tale, with `lines.tales.one`
/// or `lines.tales.many` (the newest named); nothing with none.
fn tales(content: &Content, house: &mut House) -> Vec<String> {
    let words = &content.words;
    let count = house.tales.len() as i32;
    let Some(newest) = house.tales.last().map(|t| t.title.clone()) else {
        return Vec::new();
    };
    let renown = count * TALE_YEARLY_RENOWN;
    house.add_renown(renown);
    vec![if count == 1 {
        fmt(
            &words[W::TalesOne],
            &[&newest, &TALE_YEARLY_RENOWN.to_string()],
        )
    } else {
        fmt(
            &words[W::TalesMany],
            &[&count.to_string(), &lowered(&newest), &renown.to_string()],
        )
    }]
}

/// "The year turns. The Sealed Door opens in R years." with R = years until the Door,
/// less one; at R <= 0, "The year turns. The Sealed Door stands open. It asks for four."
/// (SPEC §18 step 11; the year word as the top bar's, SPEC-GAPS KG-2).
pub fn closing_line(content: &Content, house: &House) -> String {
    let words = &content.words;
    let r = house.calendar.years_until_door() - 1;
    if r <= 0 {
        return words[W::TurningDoorOpen].to_owned();
    }
    let word = if r == 1 { W::YearWord } else { W::YearsWord };
    fmt(
        &words[W::TurningDoorCountdown],
        &[&r.to_string(), &words[word]],
    )
}
