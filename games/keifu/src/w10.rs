//! W10's oracle, live: four seated on the Door by the scripted pointer, the card read
//! seated and mid-drag, the sheet read through the dock, the Door tried, and the verdict's
//! title read off the page.
//!
//! **The oracle** (MODULES.md W10), on every recorded seed: the founding household in the
//! last summer, party A staged (`door_tests.rs` works its numbers by hand: Garrick, Maren,
//! Ysolde and Brannoc, aged 30). Three are dragged on — the card says "you bring 30", "30"
//! and "29" — then Brannoc is held over the card, and the card already reads the four:
//! "you bring 38", "35", "36", each lock's odds, and "All three locks open: 59 in 100." It
//! reads the same when he is put down; the Door's sheet in the dock reads each member's
//! solo power at each lock and the bonds that make up the card's numbers; the top bar's
//! outlook, whose best four they are, reads the same three numbers and the same chance.
//! "Try the Door": the prologue says they brought 38, 35 and 36; the telling ends on
//! "After the Door"; the verdict's title is the one for the locks given. Party B is read
//! the same way at margins 0, -1 and +3: 34, 33, 37, and 22 in 100.
//!
//! INVARIANT: every expectation is a shipped literal worked by hand from `household.json`,
//! `door.json`, `ui-text.json` and CONSTANTS §3 — the numbers `door_tests.rs` derives
//! line by line — never computed by the code under test.

use jidousha::prelude::*;

use crate::board::Slot;
use crate::checks::Checks;
use crate::house::House;
use crate::screen::Target;
use crate::scripted::{Pointer, center_of, drag, lines_in};
use crate::summer::{BOARD, TOP_BAR};
use crate::verify::{SEEDS, hero_named, page_of, point_at, session};

/// The Door card's lines with nobody before it.
pub const EMPTY_CARD: [&str; 4] = [
    "The Sealed Door",
    "Dark, Cold. It asks for four.",
    "Three locks, tried in turn by the same four. Whoever falls at one lock does not reach the next. There is no summer after this one.",
    "No one stands before it.",
];
/// Party A's card: margins +4, +1, +2 (CONSTANTS §3: 35, 26 and 30 of 36 open).
pub const PARTY_A_CARD: [&str; 13] = [
    "The Sealed Door",
    "Dark, Cold. It asks for four.",
    "Three locks, tried in turn by the same four. Whoever falls at one lock does not reach the next. There is no summer after this one.",
    "All three locks open: 59 in 100.",
    "The lock of iron: needs Might 34",
    "you bring 38",
    "Succeed 97%, triumph 58%. Setback 3%, disaster 0%.",
    "The lock of riddles: needs Wits 34",
    "you bring 35",
    "Succeed 72%, triumph 17%. Setback 28%, disaster 0%.",
    "The lock of breath: needs Spirit 34",
    "you bring 36",
    "Succeed 83%, triumph 28%. Setback 17%, disaster 0%.",
];
/// Party A without Brannoc: Might 10+8+10+2 = 30, Wits 7+9+12+2 = 30, Spirit 8+7+12+2 =
/// 29 — margins -4, -4, -5: 3, 3 and 1 of 36, all three 9 of 46656.
pub const THREE_OF_A: [&str; 4] = [
    "All three locks open: 0 in 100.",
    "you bring 30",
    "you bring 30",
    "you bring 29",
];
/// Party B's card: margins 0, -1, +3 (21, 15 and 33 of 36 open).
pub const PARTY_B_CARD: [&str; 10] = [
    "All three locks open: 22 in 100.",
    "The lock of iron: needs Might 34",
    "you bring 34",
    "Succeed 58%, triumph 8%. Setback 42%, disaster 3%.",
    "The lock of riddles: needs Wits 34",
    "you bring 33",
    "Succeed 42%, triumph 3%. Setback 58%, disaster 8%.",
    "The lock of breath: needs Spirit 34",
    "you bring 37",
    "Succeed 92%, triumph 42%. Setback 8%, disaster 0%.",
];
/// Party A's sheet in the dock: each member's solo power at each lock and the bonds.
pub const PARTY_A_SHEET: [&str; 26] = [
    "The Sealed Door",
    "All three open: 59 in 100.",
    "The lock of iron",
    "bring 38, open 97 in 100",
    "Garrick, Might 10",
    "Maren, Might 8",
    "Ysolde, Might 10",
    "Brannoc, Might 9",
    "Bonds between them",
    "+1",
    "The lock of riddles",
    "bring 35, open 72 in 100",
    "Garrick, Wits 7",
    "Maren, Wits 9",
    "Ysolde, Wits 12",
    "Brannoc, Wits 6",
    "Bonds between them",
    "+1",
    "The lock of breath",
    "bring 36, open 83 in 100",
    "Garrick, Spirit 8",
    "Maren, Spirit 7",
    "Ysolde, Spirit 12",
    "Brannoc, Spirit 8",
    "Bonds between them",
    "+1",
];
/// The top bar's outlook when party A is the house's best four.
pub const PARTY_A_OUTLOOK: &str =
    "Your best four today bring 38, 35, 36: all three open 59 in 100.";
/// The founding household's outlook in year 1 (`door_tests.rs`: every four at 0 in 100).
pub const YEAR_ONE_OUTLOOK: &str =
    "Your best four today bring 22, 22, 18: all three open 0 in 100.";
/// The prologue's summary for party A.
pub const PARTY_A_SUMMARY: &str = "Against three locks of 34, 34, and 34, they brought Might 38, Wits 35, and Spirit 36. The dark and the cold lay ahead.";
/// The verdict's title by locks given (`door.json`, MODULES.md W10).
pub const TITLES: [&str; 4] = [
    "The Door stayed shut",
    "The Door opened a hand's breadth",
    "The Door stood half open",
    "The Door is open",
];

/// The founding household moved to the last summer: the calendar to the 26th summer, its
/// board — the Door — posted, the household reseated. Nobody ages; nothing else moves.
pub fn stage_the_last_summer(sim: &mut HeadlessSim) {
    let content = match crate::content::load() {
        Ok(content) => content,
        Err(error) => crate::checks::fail("the content did not load", &error.to_string()),
    };
    let mut rng = sim.world().resource::<Rng>().clone();
    let house = sim.world_mut().resource_mut::<House>();
    for _ in 0..25 {
        house.calendar.begin_winter();
        house.calendar.begin_summer();
    }
    house.prepare_summer(&content, &mut rng);
    sim.world_mut().insert_resource(rng);
    sim.tick();
}

/// Give `name` the base aptitudes `bases` at 30.
pub fn make(sim: &mut HeadlessSim, name: &str, bases: [i32; 3]) -> usize {
    let who = hero_named(sim, name);
    let house = sim.world_mut().resource_mut::<House>();
    house.heroes[who].aptitudes = bases;
    house.heroes[who].age = 30;
    who
}

/// Party A's four, made (not seated).
pub fn make_party_a(sim: &mut HeadlessSim) -> [usize; 4] {
    [
        make(sim, "Garrick", [9, 7, 8]),
        make(sim, "Maren", [8, 9, 7]),
        make(sim, "Ysolde", [7, 9, 9]),
        make(sim, "Brannoc", [9, 6, 8]),
    ]
}

/// Drag `who` onto the Door's seat `seat` with the scripted mouse.
pub fn drag_onto_the_door(sim: &mut HeadlessSim, who: usize, seat: usize) {
    let from = center_of(sim, Target::Hero(who));
    let to = center_of(sim, Target::Seat(Slot::Quest { quest: 0, seat }));
    drag(sim, &mut Pointer::mouse(), from, to);
}

/// The Door card's lines as the page shows them now.
pub fn door_card(sim: &HeadlessSim) -> Vec<String> {
    lines_in(&page_of(sim), BOARD)
}

/// Every line of the telling, leaf by leaf through its numbered buttons, each story let
/// type out whole.
pub fn telling_read(sim: &mut HeadlessSim) -> Vec<String> {
    let mut read = Vec::new();
    let mut leaf = 0;
    while leaf < 60 && crate::verify::target_rect(sim, Target::Leaf(leaf)).is_some() {
        point_at(sim, Target::Leaf(leaf), true);
        crate::w6::wait(sim, 400);
        read.extend(lines_in(&page_of(sim), crate::telling_view::PANEL));
        leaf += 1;
    }
    read
}

/// The verdict page's lines.
pub fn verdict_lines(sim: &HeadlessSim) -> Vec<String> {
    lines_in(&page_of(sim), crate::ending_view::PANEL)
}

/// The oracle on `seed`: party A dragged on, read, tried; the title read. Returns the
/// locks given and the vector of readings for the summary.
fn oracle_on(checks: &mut Checks, seed: u64) -> (usize, Vec<String>) {
    let mut sim = session(seed);
    let bar = lines_in(&page_of(&sim), TOP_BAR);
    checks.require(
        bar.iter().any(|l| l == YEAR_ONE_OUTLOOK),
        "W10: year 1's top bar does not read the best four's outlook",
        format!("seed {seed:#x}: wanted {YEAR_ONE_OUTLOOK:?}; the bar reads {bar:?}"),
    );
    stage_the_last_summer(&mut sim);
    checks.require(
        door_card(&sim) == EMPTY_CARD
            && crate::verify::target_rect(&sim, Target::SetOut).is_some()
            && lines_in(&page_of(&sim), crate::summer::SET_OUT_BUTTON) == ["Try the Door"],
        "W10: the last summer's board is not the Door, empty, under \"Try the Door\"",
        format!("seed {seed:#x}: the card reads {:?}", door_card(&sim)),
    );
    // "Try the Door" with nobody before it does nothing.
    point_at(&mut sim, Target::SetOut, true);
    checks.require(
        sim.world().resource::<House>().telling.is_none(),
        "W10: \"Try the Door\" went with nobody before it",
        format!("seed {seed:#x}"),
    );
    let party = make_party_a(&mut sim);
    for (seat, &who) in party[..3].iter().enumerate() {
        drag_onto_the_door(&mut sim, who, seat);
    }
    let three = door_card(&sim);
    let three_read: Vec<&String> = three
        .iter()
        .filter(|l| l.starts_with("All three") || l.starts_with("you bring"))
        .collect();
    checks.require(
        three_read.iter().map(|s| s.as_str()).eq(THREE_OF_A),
        "W10 oracle: three of party A on the Door do not bring 30, 30, 29 at 0 in 100",
        format!("seed {seed:#x}: the card reads {three:?}"),
    );
    // Brannoc held over the card: it reads the four before he is put down.
    let mut mouse = Pointer::mouse();
    let from = center_of(&sim, Target::Hero(party[3]));
    let over = BOARD.center();
    mouse.press(&mut sim, from);
    mouse.hold_at(&mut sim, (from + over) * 0.5);
    mouse.hold_at(&mut sim, over);
    let held = door_card(&sim);
    mouse.release(&mut sim, over);
    let seated = door_card(&sim);
    checks.require(
        held == PARTY_A_CARD && seated == PARTY_A_CARD,
        "W10 oracle: party A's Door card does not read its three \"you bring\" and \"All three locks open: 59 in 100.\"",
        format!("seed {seed:#x}: held {held:?}, seated {seated:?}"),
    );
    checks.require(
        sim.world().resource::<House>().party(0) == party,
        "W10 oracle: the four did not sit on the Door in the order dragged",
        format!(
            "seed {seed:#x}: {:?}",
            sim.world().resource::<House>().party(0)
        ),
    );
    let bar = lines_in(&page_of(&sim), TOP_BAR);
    checks.require(
        bar.iter().any(|l| l == PARTY_A_OUTLOOK),
        "W10 oracle: the top bar's outlook does not read the card's numbers for the best four",
        format!("seed {seed:#x}: the bar reads {bar:?}"),
    );
    point_at(&mut sim, Target::Quest(0), false);
    let pages = crate::verify::dock_pages(
        &mut sim,
        &mut jidousha::testing::FrameRecorder::new(crate::screen::WINDOW),
    );
    let sheet: Vec<String> = crate::verify::dock_read(&pages)
        .into_iter()
        .map(|l| l.text)
        .collect();
    checks.require(
        sheet == PARTY_A_SHEET,
        "W10 oracle: the Door sheet does not read each member's solo power and the bonds",
        format!("seed {seed:#x}: the sheet reads {sheet:?}"),
    );
    point_at(&mut sim, Target::SetOut, true);
    let given = sim
        .world()
        .resource::<House>()
        .telling
        .as_ref()
        .and_then(|t| t.door.as_ref())
        .map_or(usize::MAX, |d| d.locks_opened());
    let told = telling_read(&mut sim);
    checks.require(
        told.first().map(String::as_str) == Some("The last summer")
            && told.iter().any(|l| l == PARTY_A_SUMMARY),
        "W10 oracle: the telling does not open on the prologue with what the four brought",
        format!("seed {seed:#x}: {told:?}"),
    );
    let label = crate::verify::target_rect(&sim, Target::GoOn)
        .map(|rect| lines_in(&page_of(&sim), rect))
        .unwrap_or_default();
    checks.require(
        label == ["After the Door"],
        "W10 oracle: the Door's telling does not end on \"After the Door\"",
        format!("seed {seed:#x}: {label:?}"),
    );
    point_at(&mut sim, Target::GoOn, true);
    let verdict = verdict_lines(&sim);
    let want = TITLES.get(given).copied().unwrap_or("?");
    checks.require(
        verdict.first().map(String::as_str) == Some(want),
        "W10 oracle: the verdict's title is not the one for the locks that gave",
        format!("seed {seed:#x}: {given} locks, the page reads {verdict:?}"),
    );
    let vector = vec![
        format!("W10 vector, the Door card (seed {seed:#x}): {seated:?}"),
        format!("W10 vector, the top bar: {PARTY_A_OUTLOOK:?}"),
        format!(
            "W10 vector, the verdict: {given} locks gave, {:?}",
            verdict.first()
        ),
    ];
    (given, vector)
}

/// Party B on the Door, seated directly, read off the card.
fn party_b(checks: &mut Checks) {
    let mut sim = session(SEEDS[0]);
    stage_the_last_summer(&mut sim);
    let party = [
        make(&mut sim, "Garrick", [8, 6, 8]),
        make(&mut sim, "Maren", [7, 9, 8]),
        make(&mut sim, "Brannoc", [8, 6, 9]),
        make(&mut sim, "Odo", [7, 9, 9]),
    ];
    for (seat, &who) in party.iter().enumerate() {
        drag_onto_the_door(&mut sim, who, seat);
    }
    let card = door_card(&sim);
    checks.require(
        card[3..] == PARTY_B_CARD,
        "W10 oracle: party B's Door card does not read 34, 33, 37 and 22 in 100",
        format!("the card reads {card:?}"),
    );
}

/// W10's oracle on every recorded seed, and party B. Returns the summary and the vector.
pub fn check_oracle(checks: &mut Checks) -> (String, Vec<String>) {
    let mut given = Vec::new();
    let mut vector = Vec::new();
    for seed in SEEDS {
        let (locks, read) = oracle_on(checks, seed);
        given.push(locks);
        if seed == SEEDS[0] {
            vector = read;
        }
    }
    party_b(checks);
    (
        format!(
            "W10 oracle: party A dragged onto the Door on {} seeds reads \"you bring\" 38, 35, 36 and \"All three locks open: 59 in 100.\" held and seated, the sheet and the top bar the same; party B 34, 33, 37 and 22 in 100; tried, the verdict's title for the locks given ({given:?})",
            SEEDS.len()
        ),
        vector,
    )
}
