//! The telegraph and the foresight, read off the screen and held to what the sim then
//! deals (SPEC §7.2; the decision rows of `tools/yakin/tasks/keifu-fixes.md`).
//!
//! **The telegraph**: on every recorded seed, year 1's four cards each say "Left alone:
//! danger A-B, needs up to N"; nobody goes, the year turns, and every quest dealt at a
//! place that was left lies inside what its card said — the template is another of the
//! place's four, the danger is in A-B and the demand is at most N. Two years running, so
//! the second reads trouble at its worst. **The foresight**: the idle dock reads "NEXT
//! SUMMER" and a line for each place; seating a party on a quest moves that place's
//! trouble; and the quest next summer deals at each place has a template the line named,
//! a danger inside its range and a trouble inside its trouble.
//!
//! INVARIANT: the Barrow's and the Coast's year-1 expectations are shipped literals
//! worked by hand from `quests.json` (the Barrow-king: 3 seats less 1, 2 x (3 + 3 + 1),
//! +1 for the wobble = 15); the sweep's bounds are read off the card the player read.

use jidousha::prelude::*;

use crate::checks::Checks;
use crate::house::House;
use crate::outlook::Span;
use crate::screen::Target;
use crate::scripted::{Pointer, card_lines, center_of, drag, lines_in};
use crate::summer::SHEET;
use crate::verify::{SEEDS, hero_named, page_of, point_at, scroll_dock, session};
use crate::w5::recorded;

/// Year 1's forced quests, by place, and what their cards say of what is left: the
/// Barrow and the Coast each have three other templates, at danger 1-3 plus one.
const YEAR_ONE_CARDS: [(&str, &str); 2] = [
    ("The Barrow", "Left alone: danger 2-4, needs up to 15"),
    (
        "The Drowned Coast",
        "Left alone: danger 2-4, needs up to 15",
    ),
];

/// "3" or "2-4" as a span.
fn read_span(text: &str) -> Option<Span> {
    match text.split_once('-') {
        Some((lo, hi)) => Some(Span {
            lo: lo.parse().ok()?,
            hi: hi.parse().ok()?,
        }),
        None => Some(Span::of(text.parse().ok()?)),
    }
}

/// "danger 2-4" and "needs up to 15" out of a card's telegraph line.
fn read_left_alone(card: &[String]) -> Option<(Span, i32)> {
    let line = card.iter().find(|l| l.starts_with("Left alone: danger "))?;
    let rest = line.strip_prefix("Left alone: danger ")?;
    let (danger, needs) = rest.split_once(", needs up to ")?;
    let span = read_span(danger)?;
    Some((span, needs.parse().ok()?))
}

/// Stay home, leave the telling and the winter: next summer's board is up.
pub fn stay_home_to_next_summer(sim: &mut HeadlessSim) {
    point_at(sim, Target::SetOut, true);
    point_at(sim, Target::GoOn, true);
    crate::w7::through_the_winter(sim);
}

/// What one board slot says of itself, to hold the next board to.
struct Told {
    place: usize,
    template: Option<usize>,
    left: Option<(Span, i32)>,
}

fn told(sim: &HeadlessSim) -> Vec<Told> {
    let house = sim.world().resource::<House>();
    (0..house.board.len())
        .map(|slot| Told {
            place: house.board[slot].quest.place.index(),
            template: house.board[slot].quest.template(),
            left: read_left_alone(&card_lines(sim, slot)),
        })
        .collect()
}

/// The telegraph on every recorded seed, held to the board the next two summers deal.
pub fn check_telegraph(checks: &mut Checks) -> String {
    let (mut cards, mut dealt) = (0, 0);
    for seed in recorded() {
        let mut sim = session(seed);
        for year in 1..=3 {
            let board = told(&sim);
            cards += board.len();
            for (index, quest) in board.iter().enumerate() {
                checks.require(
                    quest.left.is_some(),
                    "a year's card does not tell what its place's next quest could be if left alone",
                    format!("seed {seed:#x}, year {year}, card {index}"),
                );
            }
            for (slot, quest) in board.iter().enumerate() {
                let trouble = sim.world().resource::<House>().board[slot].quest.trouble;
                point_at(&mut sim, Target::Quest(slot), false);
                let mut sheet = lines_in(&page_of(&sim), SHEET);
                if !sheet.iter().any(|l| l.starts_with("Left alone, trouble ")) {
                    // A troubled quest's sheet is longer than the dock: wheel to its end.
                    scroll_dock(&mut sim, -50.0);
                    sheet = lines_in(&page_of(&sim), SHEET);
                }
                let head = if trouble < 2 {
                    format!(
                        "Left alone, trouble rises to {}. Next quest: room ",
                        trouble + 1
                    )
                } else {
                    "Left alone, trouble stays at its worst. Next quest: room ".to_owned()
                };
                let told_here = sheet.iter().find(|l| l.starts_with("Left alone, trouble "));
                let (danger, needs) = told_here
                    .and_then(|l| l.split_once("danger "))
                    .and_then(|(_, rest)| rest.split_once(", needs up to "))
                    .map(|(d, n)| (read_span(d), n.trim_end_matches('.').parse::<i32>().ok()))
                    .unwrap_or((None, None));
                checks.require(
                    told_here.is_some_and(|l| l.starts_with(&head))
                        && Some((danger, needs))
                            == quest.left.map(|(d, n)| (Some(d), Some(n))),
                    "a quest's sheet does not tell what its card tells, with trouble rising to the right number",
                    format!("seed {seed:#x}, year {year}, trouble {trouble}: {told_here:?} against {:?}", quest.left),
                );
            }
            if year == 1 {
                let house = sim.world().resource::<House>();
                for (place, want) in YEAR_ONE_CARDS {
                    let slot = (0..house.board.len()).find(|&s| {
                        crate::verify::content_of(&sim).lore.places
                            [house.board[s].quest.place.index()]
                        .title
                            == place
                    });
                    let card = slot.map(|s| card_lines(&sim, s)).unwrap_or_default();
                    checks.require(
                        card.iter().any(|l| l == want),
                        "year 1's forced card does not read the shipped telegraph",
                        format!("seed {seed:#x}, {place}: wanted {want:?} in {card:?}"),
                    );
                }
            }
            // Year 3's stay-home would close the house (renown 3 less 8): its card is read, not played.
            if year == 3 {
                break;
            }
            stay_home_to_next_summer(&mut sim);
            let house = sim.world().resource::<House>();
            for posted in &house.board {
                let quest = &posted.quest;
                let Some(before) = board.iter().find(|t| t.place == quest.place.index()) else {
                    continue;
                };
                let Some((danger, needs)) = before.left else {
                    continue;
                };
                dealt += 1;
                checks.require(
                    quest.template() != before.template
                        && danger.holds(quest.danger)
                        && quest.demand <= needs,
                    "a quest dealt at a place left alone is outside what its card said (another template, \
                     danger in the range, demand at most the number)",
                    format!(
                        "seed {seed:#x}, year {year}: {:?} danger {} demand {}; card said {danger:?}, up to {needs}",
                        quest.title, quest.danger, quest.demand
                    ),
                );
            }
        }
    }
    checks.require(
        dealt >= 100,
        "the telegraph sweep matched too few dealt quests to mean anything",
        format!("{dealt} dealt over {} seeds", recorded().len()),
    );
    format!(
        "telegraph: {cards} cards over {} seeds each say what a place's next quest could be if left alone; {dealt} quests dealt there the next summer all lie inside it",
        recorded().len()
    )
}

/// One place's line of the foresight: its title starts it.
fn place_lines(sim: &HeadlessSim) -> Vec<String> {
    lines_in(&page_of(sim), SHEET)
        .into_iter()
        .filter(|l| l.contains(", trouble "))
        .collect()
}

/// The foresight in the idle dock, and what it promises held to the next board.
pub fn check_foresight(checks: &mut Checks) -> String {
    let mut sim = session(SEEDS[0]);
    let dock = lines_in(&page_of(&sim), SHEET);
    checks.require(
        dock.get(1).map(String::as_str) == Some("NEXT SUMMER")
            && dock
                .get(2)
                .is_some_and(|l| l.starts_with("Four of these six places")),
        "the idle dock does not open next summer's foresight under the help",
        format!("{dock:?}"),
    );
    let before = place_lines(&sim);
    checks.require(
        before.len() == 6
            && before[0].starts_with("The Barrow, trouble 1: one of ")
            && before[1].starts_with("The Drowned Coast, trouble 1: one of "),
        "with nobody seated the foresight does not give six places, the quests' places at trouble 1",
        format!("{before:?}"),
    );
    // Year 1's Barrow is Grave goods: it is the remembered template, so it is not offered.
    checks.require(
        before[0].contains("The lamps in the Barrow")
            && before[0].contains("A name scratched out")
            && before[0].contains("The Barrow-king wakes")
            && !before[0].contains("Grave goods"),
        "the Barrow's foresight does not offer its other three templates and leave Grave goods out",
        before[0].clone(),
    );
    // Seat two on the Barrow's quest: it is answered, and its trouble stays at nothing.
    let (garrick, brannoc) = (hero_named(&sim, "Garrick"), hero_named(&sim, "Brannoc"));
    for (hero, seat) in [(garrick, 0), (brannoc, 1)] {
        let from = center_of(&sim, Target::Hero(hero));
        let to = center_of(
            &sim,
            Target::Seat(crate::board::Slot::Quest { quest: 0, seat }),
        );
        drag(&mut sim, &mut Pointer::mouse(), from, to);
    }
    let after = place_lines(&sim);
    checks.require(
        after.len() == 6
            && after[0].starts_with("The Barrow, trouble 0: one of ")
            && after[1].starts_with("The Drowned Coast, trouble 1: one of "),
        "seating a party on the Barrow's quest does not move the Barrow's foresight to trouble 0",
        format!("{after:?}"),
    );
    // Held to the board the sim then deals, on every recorded seed.
    let (mut lines, mut dealt) = (0, 0);
    for seed in recorded() {
        let mut sim = session(seed);
        let promised = place_lines(&sim);
        lines += promised.len();
        stay_home_to_next_summer(&mut sim);
        let content = crate::verify::content_of(&sim);
        let house = sim.world().resource::<House>();
        for posted in &house.board {
            let quest = &posted.quest;
            let title = &content.lore.places[quest.place.index()].title;
            let Some(line) = promised
                .iter()
                .find(|l| l.starts_with(&format!("{title}, trouble ")))
            else {
                continue;
            };
            dealt += 1;
            let named = quest
                .template()
                .is_some_and(|_| line.contains(&quest.title))
                || matches!(quest.source, crate::quest::Source::Ghost(_));
            let range = line
                .rsplit_once("Danger ")
                .and_then(|(_, d)| read_span(d.trim_end_matches('.')));
            let trouble = line
                .split_once(", trouble ")
                .and_then(|(_, t)| t.split_once(':'))
                .and_then(|(t, _)| read_span(t));
            checks.require(
                named
                    && range.is_some_and(|r| r.holds(quest.danger))
                    && trouble.is_some_and(|t| t.holds(quest.trouble)),
                "a quest dealt next summer is not one the foresight named, inside its danger range and its trouble",
                format!("seed {seed:#x}: {:?} danger {} trouble {} against {line:?}", quest.title, quest.danger, quest.trouble),
            );
        }
    }
    checks.require(
        dealt >= 100,
        "the foresight sweep matched too few dealt quests to mean anything",
        format!("{dealt} dealt"),
    );
    format!(
        "foresight: the idle dock gives six places under NEXT SUMMER, seating moves the Barrow to trouble 0; {lines} place lines on {} seeds, and the {dealt} quests dealt there next summer are each one the line named, in its danger range",
        recorded().len()
    )
}
