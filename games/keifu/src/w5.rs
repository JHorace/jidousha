//! W5's oracle: board generation, asked as an invariant and as a shape.
//!
//! MODULES.md's W5 oracle is the port's first distributional one. **The invariant**
//! — year 1 is "Grave goods" at the Barrow and "The bell under the tide" at the
//! Coast plus two other places, four quests in place order — is read off the cards
//! the page draws on the recorded seeds and off the card reading on a sweep of fresh
//! ones. **The shape** — "most summers at least one quest carries a fair-chance
//! Dream: mark" — and the loop's stated rules are asked over a fixed battery of
//! seeds, each run through its summers (`w5_shape.rs`). The ghost slot is asked on
//! staged ghost lists (`check_ghost_slot`); since W8 its live source is the death page,
//! and the whole-year battery (`w8_battery.rs`) raises, lays and takes up ghosts in play.
//!
//! INVARIANT: every expectation is a shipped literal — the titles, the place order,
//! the stakes — copied by hand from MODULES.md, SPEC.md and the content.

use jidousha::prelude::Rng;

use crate::checks::Checks;
use crate::content::Content;
use crate::ghost::Ghost;
use crate::house::House;
use crate::ids::Place;
use crate::quest::Source;
use crate::quest_card::read_card;
use crate::scripted::card_lines;
use crate::verify::{SEEDS, session};

/// MODULES.md W5: the place order, as the cards title them (`lore.json` places).
pub const PLACE_ORDER: [&str; 6] = [
    "The Barrow",
    "The Drowned Coast",
    "The High Pass",
    "Emberfall",
    "The King's Court",
    "The Deepwood",
];

/// MODULES.md W5: year 1's two forced quests, by slot: (place, title).
pub const YEAR_ONE: [(&str, &str); 2] = [
    ("The Barrow", "Grave goods"),
    ("The Drowned Coast", "The bell under the tide"),
];

/// The recorded seeds: the oracles' three and W4's sweep.
pub fn recorded() -> Vec<u64> {
    SEEDS.iter().copied().chain(100..124).collect()
}

/// The fresh sweep: seeds no other check founds a house on.
pub fn fresh() -> Vec<u64> {
    (0x5_0000..0x5_0000 + 96).collect()
}

/// Whether four cards' places and titles are year 1's: the two forced ones first,
/// then two of the other four, strictly in place order.
fn year_one_holds(cards: &[(String, String)]) -> bool {
    let at = |place: &str| PLACE_ORDER.iter().position(|p| *p == place);
    cards.len() == 4
        && cards[..2]
            .iter()
            .zip(YEAR_ONE)
            .all(|((place, title), (p, t))| place == p && title == t)
        && cards
            .windows(2)
            .all(|w| matches!((at(&w[0].0), at(&w[1].0)), (Some(a), Some(b)) if a < b))
}

/// The W5 invariant: on the recorded seeds as the page draws the cards; on the fresh
/// sweep as the card reading reads them. Returns its summary lines.
pub fn check_invariant(checks: &mut Checks, content: &Content) -> Vec<String> {
    let mut others = [0usize; 6];
    let mut seen = 0;
    for seed in recorded() {
        let sim = session(seed);
        let house = sim.world().resource::<House>();
        let cards: Vec<(String, String)> = (0..house.board.len())
            .map(|slot| {
                let lines = card_lines(&sim, slot);
                let place = lines.first().cloned().unwrap_or_default();
                let title = house.board[slot].quest.title.clone();
                let drawn = lines.contains(&title);
                (place, if drawn { title } else { String::new() })
            })
            .collect();
        checks.require(
            year_one_holds(&cards),
            "W5 oracle: year 1's cards are Grave goods at the Barrow, the bell at the Coast, \
             and two others, in place order",
            format!("recorded seed {seed:#x}: {cards:?}"),
        );
        seen += 1;
    }
    for seed in fresh() {
        let mut rng = Rng::from_seed(seed);
        let house = match House::found(content, seed, &mut rng) {
            Ok(house) => house,
            Err(error) => {
                checks.require(false, "W5 oracle: a fresh house founds", error);
                continue;
            }
        };
        let cards: Vec<(String, String)> = (0..house.board.len())
            .map(|slot| {
                let card = read_card(content, &house, slot, &[], None);
                (card.place, card.title)
            })
            .collect();
        checks.require(
            year_one_holds(&cards),
            "W5 oracle: year 1's cards are Grave goods at the Barrow, the bell at the Coast, \
             and two others, in place order",
            format!("fresh seed {seed:#x}: {cards:?}"),
        );
        for (place, _) in &cards[2.min(cards.len())..] {
            if let Some(at) = PLACE_ORDER.iter().position(|p| p == place) {
                others[at] += 1;
            }
        }
        seen += 1;
    }
    // Two of the four other places each year: every one of them is drawn somewhere
    // in the sweep, and the forced places never twice.
    checks.require(
        others[0] == 0 && others[1] == 0 && others[2..].iter().all(|n| *n > 0),
        "W5 oracle: year 1's two drawn places range over the other four",
        format!("{others:?}"),
    );
    let fresh_count = fresh().len();
    vec![
        format!(
            "W5 oracle (invariant): year 1 is \"Grave goods\" at the Barrow, \"The bell under \
             the tide\" at the Coast, plus two others in place order, on {} recorded seeds \
             (read off the drawn cards) and {fresh_count} fresh (off the card reading): {seen} houses",
            recorded().len()
        ),
        format!(
            "W5 year-1 draws over the fresh sweep: High Pass {}, Emberfall {}, Court {}, \
             Deepwood {} (of {})",
            others[2],
            others[3],
            others[4],
            others[5],
            2 * fresh_count
        ),
    ]
}

/// The ghost slot (SPEC §5.2 step 3, §14.4), on staged ghost lists: only the first
/// ghost is offered, at its place, if the place is free; its quest is Spirit with
/// two seats and danger two and no wobble; the card names it "The ghost of <name>";
/// a ghost's quest writes no template memory.
pub fn check_ghost_slot(checks: &mut Checks, content: &Content) -> String {
    let mut boards = 0;
    for seed in fresh().into_iter().take(24) {
        let mut house = staged_ghosts(content, seed);
        let mut rng = Rng::from_seed(seed);
        let before = house.templates_last.clone();
        house.prepare_summer(content, &mut rng);
        let ghost_slots: Vec<usize> = (0..house.board.len())
            .filter(|&s| matches!(house.board[s].quest.source, Source::Ghost(_)))
            .collect();
        let garrick = house.ghosts[0].hero;
        let first_posted = ghost_slots.len() == 1
            && house.board[ghost_slots[0]].quest.source == Source::Ghost(garrick);
        checks.require(
            first_posted,
            "W5 ghost slot: the first ghost's quest, and only it, is on the board",
            format!("seed {seed:#x}: ghost slots {ghost_slots:?}"),
        );
        let Some(&slot) = ghost_slots.first() else {
            continue;
        };
        let q = &house.board[slot].quest;
        // Year 2, no trouble: seats 2, danger 2, renown 2, demand 2 * (3 + 2) = 10.
        let stakes = (
            q.title.as_str(),
            q.place,
            q.aptitude,
            q.seats,
            q.danger,
            q.renown,
            q.demand,
        );
        let want = (
            "Lay Garrick's ghost",
            Place::Emberfall,
            crate::ids::Aptitude::Spirit,
            2,
            2,
            2,
            10 - house.board_report.as_ref().map_or(0, |r| r.eased[slot]),
        );
        checks.require(
            stakes == want,
            "W5 ghost slot: the ghost quest's title, place and stakes",
            format!("seed {seed:#x}: {stakes:?}, want {want:?}"),
        );
        let card = read_card(content, &house, slot, &[], None);
        checks.require(
            card.dream.as_deref() == Some("The ghost of Garrick") && card.place == "Emberfall",
            "W5 ghost slot: the card says \"The ghost of Garrick\" in place of a Dream: line",
            format!("seed {seed:#x}: {:?} at {:?}", card.dream, card.place),
        );
        checks.require(
            house.templates_last[Place::Emberfall.index()] == before[Place::Emberfall.index()],
            "W5 ghost slot: a ghost's quest writes no template memory at its place",
            format!("seed {seed:#x}: {:?}", house.templates_last),
        );
        boards += 1;
    }
    // A ghost whose place is taken waits: in year 1 the Barrow is the opening quest's.
    let mut house = staged_ghosts(content, 7);
    house.calendar = crate::calendar::Calendar::start();
    house.ghosts[0].place = Place::Barrow;
    house.prepare_summer(content, &mut Rng::from_seed(7));
    let waits = house
        .board
        .iter()
        .all(|p| matches!(p.quest.source, Source::Template(_)))
        && house.board[0].quest.title == "Grave goods";
    checks.require(
        waits,
        "W5 ghost slot: a ghost whose place is taken waits, and the one behind it is not offered",
        format!(
            "{:?}",
            house
                .board
                .iter()
                .map(|p| (&p.quest.title, p.quest.place))
                .collect::<Vec<_>>()
        ),
    );
    format!(
        "W5 ghost slot (staged, two ghosts): the first offered at Emberfall on {boards} boards as \
         \"Lay Garrick's ghost\", Spirit, 2 seats, danger 2, demand 10; the second never; a \
         taken place waits"
    )
}

/// A house in year 2's summer with two ghosts, both of them dead: Garrick's at
/// Emberfall, then Odo's at the Deepwood.
pub fn staged_ghosts(content: &Content, seed: u64) -> House {
    let mut house = match House::found(content, seed, &mut Rng::from_seed(seed)) {
        Ok(house) => house,
        Err(error) => crate::checks::fail("the house could not be founded", &error),
    };
    let named = |name: &str| house.heroes.iter().position(|h| h.name == name);
    let (Some(garrick), Some(odo)) = (named("Garrick"), named("Odo")) else {
        crate::checks::fail("a founding hero is missing", "Garrick or Odo");
    };
    let dream = |id: usize| match house.heroes[id].dream.clone() {
        Some(dream) => dream,
        None => crate::checks::fail("a staged ghost has no dream", &house.heroes[id].name),
    };
    let ghosts = vec![
        Ghost {
            hero: garrick,
            dream: dream(garrick),
            place: Place::Emberfall,
        },
        Ghost {
            hero: odo,
            dream: dream(odo),
            place: Place::Deepwood,
        },
    ];
    house.ghosts = ghosts;
    // The ghosts' heroes are dead, as a ghost's hero is (SPEC §15.2).
    for id in [garrick, odo] {
        house.heroes[id].fate = crate::hero::Fate::Dead;
    }
    house.calendar.begin_winter();
    house.calendar.begin_summer();
    house
}

/// Eight boards of four that between them put every template on a card: for each
/// template rank k, the first four places' k-th, then the last four places' k-th.
pub fn template_boards(content: &Content) -> Vec<[usize; 4]> {
    let at = |place: Place, k: usize| crate::generation::templates_at(content, place)[k];
    let places: Vec<Place> = Place::ALL[..6].to_vec();
    let mut boards = Vec::new();
    for k in 0..4 {
        for window in [&places[..4], &places[2..]] {
            boards.push([0, 1, 2, 3].map(|i| at(window[i], k)));
        }
    }
    boards
}

/// Replace the session's board with `templates`, posted calm in year 1, in the
/// order given (place order), and reseat the household.
pub fn stage_board(sim: &mut jidousha::prelude::HeadlessSim, templates: &[usize; 4]) {
    let content = match crate::content::load() {
        Ok(content) => content,
        Err(error) => crate::checks::fail("the content did not load", &error.to_string()),
    };
    let house = sim.world_mut().resource_mut::<House>();
    let mut rng = Rng::from_seed(1);
    house.board = templates
        .iter()
        .map(|&t| {
            let quest = crate::quest::post(&content, t, 0, 1, &mut rng);
            crate::board::Posted {
                seats: vec![None; quest.seats as usize],
                quest,
            }
        })
        .collect();
    house.reseat();
}

/// Replace the session's house with `staged_ghosts`' and post its year-2 board.
pub fn stage_ghost_board(sim: &mut jidousha::prelude::HeadlessSim) {
    let content = match crate::content::load() {
        Ok(content) => content,
        Err(error) => crate::checks::fail("the content did not load", &error.to_string()),
    };
    let mut house = staged_ghosts(&content, SEEDS[0]);
    house.prepare_summer(&content, &mut Rng::from_seed(SEEDS[0]));
    *sim.world_mut().resource_mut::<House>() = house;
}
