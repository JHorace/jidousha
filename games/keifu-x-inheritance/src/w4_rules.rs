//! W4's rules, asked in verify as shipped literals, and W2's and W3's oracles
//! graduated onto the real card.
//!
//! The rules: CONSTANTS §3's table entry by entry against the forecast; the stakes
//! formula over seeds, trouble and years; the unanswered cost and death chance at
//! their edges; the trouble's stakes and a visited place's history on the sheet.
//!
//! INVARIANT: every expectation here is a shipped literal, copied by hand from
//! CONSTANTS.md, MODULES.md and the content — never computed by the code under test.

use jidousha::prelude::Rng;

use crate::board::Slot;
use crate::checks::Checks;
use crate::forecast::{card_percentages, forecast, percent};
use crate::house::House;
use crate::quest::{base_demand, post};
use crate::quest_card::read_card;
use crate::quest_sheet::{place_history, quest_sheet};
use crate::screen::Target;
use crate::scripted::{card_lines, lines_in};
use crate::verify::{SEEDS, content_of, page_of, point_at, session};
use crate::w4::{away, seat};

/// CONSTANTS §3, "Outcome odds by power minus demand", copied row by row:
/// (power - demand, [disaster, setback, success, triumph] in 36ths, "as shown").
pub const CONSTANTS_3: [(i32, [i32; 4], i32); 21] = [
    (-10, [36, 0, 0, 0], 0),
    (-9, [35, 1, 0, 0], 0),
    (-8, [33, 3, 0, 0], 0),
    (-7, [30, 6, 0, 0], 0),
    (-6, [26, 10, 0, 0], 0),
    (-5, [21, 14, 1, 0], 3),
    (-4, [15, 18, 3, 0], 8),
    (-3, [10, 20, 6, 0], 17),
    (-2, [6, 20, 10, 0], 28),
    (-1, [3, 18, 14, 1], 42),
    (0, [1, 14, 18, 3], 58),
    (1, [0, 10, 20, 6], 72),
    (2, [0, 6, 20, 10], 83),
    (3, [0, 3, 18, 15], 92),
    (4, [0, 1, 14, 21], 97),
    (5, [0, 0, 10, 26], 100),
    (6, [0, 0, 6, 30], 100),
    (7, [0, 0, 3, 33], 100),
    (8, [0, 0, 1, 35], 100),
    (9, [0, 0, 0, 36], 100),
    (10, [0, 0, 0, 36], 100),
];

fn expect<T: PartialEq + std::fmt::Debug>(checks: &mut Checks, what: &str, have: T, want: T) {
    let ok = have == want;
    checks.require(ok, what, format!("have {have:?}, want {want:?}"));
}

/// W4's rules. Returns the summary line.
pub fn check_rules(checks: &mut Checks) -> String {
    let (passed, _) = checks.counts();
    // The table, entry by entry, at two different demands so no row is an accident
    // of one power.
    for (gap, ways, shown) in CONSTANTS_3 {
        for demand in [10, 23] {
            let odds = forecast(demand + gap, demand, true);
            expect(
                checks,
                &format!("CONSTANTS §3 at {gap:+}: the 36 pairs"),
                odds.ways,
                ways,
            );
            expect(
                checks,
                &format!("CONSTANTS §3 at {gap:+}: success or better as shown"),
                card_percentages(&odds)[0],
                shown,
            );
        }
    }
    expect(
        checks,
        "an empty party forecasts nothing",
        forecast(40, 0, false).ways,
        [0; 4],
    );
    expect(
        checks,
        "percent rounds k/36 half up",
        [1, 3, 6, 10, 13, 18, 35].map(percent),
        [3, 8, 17, 28, 36, 50, 97],
    );
    let sim = session(SEEDS[0]);
    let content = content_of(&sim);
    let index = |title: &str| {
        content
            .quest_templates
            .iter()
            .position(|t| t.title == title)
            .unwrap_or(0)
    };
    let (grave, lamps) = (index("Grave goods"), index("The lamps in the Barrow"));
    // Grave goods: seats 2..2, danger 2. The lamps: seats 2..3, danger 2.
    let mut calm = Vec::new();
    for seed in 0..48 {
        let mut rng = Rng::from_seed(seed);
        let q = post(content, grave, 0, 1, &mut rng);
        checks.require(
            (q.seats, q.danger, q.renown, q.trouble) == (2, 2, 2, 0)
                && (9..=11).contains(&q.demand),
            "Grave goods in year 1, calm: 2 seats, danger 2, renown 2, demand 9..11",
            format!("seed {seed}: {q:?}"),
        );
        let q = post(content, lamps, 2, 13, &mut rng);
        calm.push(q.calm_seats);
        checks.require(
            q.seats == 1 && (q.danger, q.renown) == (4, 4) && (8..=10).contains(&q.demand),
            "the lamps at trouble 2 in year 13: 1 seat, danger 4, renown 4, demand 1 x 9 +- 1",
            format!("seed {seed}: {q:?}"),
        );
        let q = post(content, lamps, 1, 25, &mut rng);
        let want = if q.calm_seats == 3 { 2 * 10 } else { 10 };
        checks.require(
            q.seats == q.calm_seats - 1
                && (q.danger, q.renown) == (3, 3)
                && (q.demand - want).abs() <= 1,
            "the lamps at trouble 1 in year 25: a seat fewer, danger 3, demand seats x 10 +- 1",
            format!("seed {seed}: {q:?}"),
        );
    }
    calm.sort_unstable();
    calm.dedup();
    expect(
        checks,
        "the lamps roll both 2 and 3 calm seats",
        calm,
        vec![2, 3],
    );
    expect(
        checks,
        "the year adds a demand per seat in years 7, 13, 19 and 25",
        [1, 6, 7, 12, 13, 18, 19, 24, 25].map(|year| base_demand(2, 2, 0, year)),
        [10, 10, 12, 12, 14, 14, 16, 16, 18],
    );
    let mut q = post(content, grave, 0, 1, &mut Rng::from_seed(1));
    expect(
        checks,
        "unanswered: 1, +1 troubled, + renown / 20",
        [0, 19, 20, 39, 40].map(|renown| q.unanswered_cost(renown)),
        [1, 1, 2, 2, 3],
    );
    q.trouble = 1;
    expect(checks, "unanswered when troubled", q.unanswered_cost(15), 2);
    expect(
        checks,
        "death in a disaster: 15 in 100 per danger, at most all",
        [1, 2, 3, 4, 7].map(|danger| {
            q.danger = danger;
            q.death_percent()
        }),
        [15, 30, 45, 60, 100],
    );
    // The sheet's trouble stakes, and a visited place's history.
    let mut house: House = sim.world().resource::<House>().clone();
    {
        let q = &mut house.board[1].quest;
        q.trouble = 1;
        q.calm_seats = 2;
        q.seats = 1;
        q.danger = 3;
        q.renown = 3;
    }
    let coast = house.board[1].quest.place.index();
    house.places[coast].visits = 3;
    house.places[coast].disasters = 1;
    let sheet = quest_sheet(content, &house, 1, &[]);
    let stakes = "The tide has had the Drowned Coast to itself for a year. Room for 1 where there was room for 2. Each who goes must bring 1 more, the danger is 1 higher, and it pays 1 more renown. Answer it, however it goes, and it eases.";
    checks.require(
        sheet.lines.iter().any(|l| l.text == stakes),
        "the sheet tells a troubled quest's stakes",
        format!(
            "{:?}",
            sheet.lines.iter().map(|l| &l.text).collect::<Vec<_>>()
        ),
    );
    expect(
        checks,
        "the Coast's history: visits in words, then its fallen",
        place_history(content, &house, 1),
        vec![
            "Quested here three times: 0 in triumph, 1 in disaster.".to_owned(),
            "Elsbeth Thorne was lost at the Drowned Coast.".to_owned(),
        ],
    );
    // The card at stakes year 1 never shows: a trouble of one, renown above danger,
    // a house of 40 renown, two patrons — Garrick and Brannoc on Grave goods.
    let mut house: House = sim.world().resource::<House>().clone();
    let (garrick, brannoc) = (
        crate::verify::hero_named(&sim, "Garrick"),
        crate::verify::hero_named(&sim, "Brannoc"),
    );
    house.renown = 40;
    house.patrons = 2;
    {
        let q = &mut house.board[0].quest;
        // 12 + 2 (two Strong traits, the variant) + 2 patrons: demand 16 keeps the margin at 0.
        q.demand = 16;
        q.renown = 5;
        q.danger = 4;
    }
    house.board[1].quest.trouble = 1;
    let card = read_card(content, &house, 0, &[garrick, brannoc], None);
    expect(
        checks,
        "the card counts patrons, its own renown and the house's renown",
        (
            card.you_bring.as_deref(),
            card.renown.as_str(),
            card.unanswered.as_str(),
        ),
        (Some("you bring 16"), "Renown +5", "unanswered -3"),
    );
    expect(
        checks,
        "the card's odds at 16 against 16 are CONSTANTS §3's at 0",
        card.odds.clone(),
        Some(["Succeed 58%", "triumph 8%", "Setback 42%", "disaster 3%"].map(String::from)),
    );
    let sheet = quest_sheet(content, &house, 0, &[garrick, brannoc]);
    checks.require(
        sheet
            .lines
            .iter()
            .any(|l| l.text == "A patron at Court" && l.value.as_deref() == Some("+2"))
            && sheet
                .lines
                .iter()
                .any(|l| l.text == "You bring" && l.value.as_deref() == Some("16")),
        "the sheet's breakdown ends with the patrons and adds up to the card",
        format!(
            "{:?}",
            sheet
                .lines
                .iter()
                .map(|l| (&l.text, &l.value))
                .collect::<Vec<_>>()
        ),
    );
    expect(
        checks,
        "an empty quest at trouble 1 shows its trouble line for a year",
        read_card(content, &house, 1, &[], None).idle,
        Some("The tide has had the Drowned Coast to itself for a year.".to_owned()),
    );
    // A board posted under trouble: a seat fewer, danger and renown up, demand up.
    let mut troubled: House = sim.world().resource::<House>().clone();
    troubled.places[crate::ids::Place::Barrow.index()].trouble = 1;
    troubled.post_board(content, &mut Rng::from_seed(9));
    let q = &troubled.board[0].quest;
    checks.require(
        (q.trouble, q.seats, q.danger, q.renown) == (1, 1, 3, 3)
            && troubled.board[0].seats.len() == 1
            && (5..=7).contains(&q.demand),
        "Grave goods posted at trouble 1: one seat, danger 3, renown 3, demand 1 x 6 +- 1",
        format!("{q:?}"),
    );
    // W2's party on the bell, line by line: Thornfall is Might, so it is not carried.
    let (maren, odo) = (
        crate::verify::hero_named(&sim, "Maren"),
        crate::verify::hero_named(&sim, "Odo"),
    );
    let house: House = sim.world().resource::<House>().clone();
    let bell = quest_sheet(content, &house, 1, &[maren, garrick]);
    let from = bell.lines.iter().position(|l| l.text == "You bring");
    let breakdown: Vec<(&str, Option<&str>)> = from
        .map(|at| &bell.lines[at..at + 7])
        .unwrap_or_default()
        .iter()
        .map(|l| (l.text.as_str(), l.value.as_deref()))
        .collect();
    expect(
        checks,
        "W2's party on the bell, line by line",
        breakdown,
        vec![
            ("You bring", Some("4")),
            ("Maren, Spirit 3", None),
            ("  fears deep water", Some("-2")),
            ("Garrick, Spirit 4", None),
            ("  fears deep water", Some("-3")),
            ("Maren and Garrick, child and parent", Some("+2")),
            ("Two dice, less 7, are added to that.", None),
        ],
    );
    // A call that hangs on the party: Odo, wanting to see a student he taught succeed
    // without him, is called to stay behind only when Maren is going.
    let mut staged: House = sim.world().resource::<House>().clone();
    match crate::dream::Dream::build(content, crate::ids::DreamKind::WorthyStudent, None, None) {
        Ok(mut dream) => {
            dream.advance_to_stage(2);
            staged.heroes[odo].dream = Some(dream);
        }
        Err(error) => checks.require(false, "WORTHY_STUDENT builds", error),
    }
    crate::bonds::form(
        &mut staged.heroes,
        odo,
        maren,
        crate::ids::BondKind::Student,
        1,
    );
    for bond in staged.heroes[odo]
        .bonds
        .iter_mut()
        .filter(|b| b.other == maren)
    {
        bond.taught = true;
    }
    expect(
        checks,
        "the Dream: line reads the party: Odo is called only when his student goes",
        (
            read_card(content, &staged, 0, &[], None).dream,
            read_card(content, &staged, 0, &[maren], None).dream,
        ),
        (
            Some("Dream: Garrick, Ysolde".to_owned()),
            Some("Dream: Garrick, Ysolde, Odo".to_owned()),
        ),
    );
    let sheet = quest_sheet(content, &staged, 0, &[maren]);
    let call = "Odo's dream: See a student succeed without you. He must stay behind.";
    checks.require(
        sheet.lines.iter().any(|l| l.text == call),
        "the sheet tells a stay-behind call",
        format!("wanted {call:?}"),
    );
    let (now, _) = checks.counts();
    format!(
        "W4 rules: {} checks over CONSTANTS §3 entry by entry, stakes, costs and the sheet",
        now - passed
    )
}

/// W2's and W3's oracles, graduated: read off the real card after real drags.
pub fn check_inherited(checks: &mut Checks) -> (String, Vec<String>) {
    use crate::w2::{W2_FEAR_LINE, W2_YOU_BRING};
    use crate::w3::{W3_CARD, W3_SHEET};
    let mut vector = Vec::new();
    for seed in SEEDS {
        let mut sim = session(seed);
        seat(&mut sim, "Maren", Slot::Quest { quest: 1, seat: 0 });
        seat(&mut sim, "Garrick", Slot::Quest { quest: 1, seat: 1 });
        away(&mut sim);
        let bell = card_lines(&sim, 1);
        checks.require(
            bell.iter().any(|l| l == W2_FEAR_LINE) && bell.iter().any(|l| l == W2_YOU_BRING),
            "W2 oracle, on the card: Maren and Garrick dragged onto the bell",
            format!("seed {seed:#x}: {bell:?}"),
        );
        let mut sim = session(seed);
        seat(&mut sim, "Garrick", Slot::Quest { quest: 0, seat: 0 });
        away(&mut sim);
        let grave = card_lines(&sim, 0);
        point_at(&mut sim, Target::Quest(0), false);
        let sheet = lines_in(&page_of(&sim), crate::summer::SHEET);
        checks.require(
            grave.iter().any(|l| l == W3_CARD) && sheet.iter().any(|l| l == W3_SHEET),
            "W3 oracle, on the card and its sheet: Garrick dragged onto Grave goods",
            format!("seed {seed:#x}: card {grave:?}; sheet {sheet:?}"),
        );
        if seed == SEEDS[0] {
            let found = |lines: &[String], want: &str| {
                lines
                    .iter()
                    .find(|l| *l == want)
                    .cloned()
                    .unwrap_or_default()
            };
            vector = vec![
                format!(
                    "W2 vector, off the card: {:?} / {:?}",
                    found(&bell, W2_FEAR_LINE),
                    found(&bell, W2_YOU_BRING)
                ),
                format!(
                    "W3 vector, off the card: {:?}; off its sheet: {:?}",
                    found(&grave, W3_CARD),
                    found(&sheet, W3_SHEET)
                ),
            ];
        }
    }
    (
        format!(
            "W2 and W3 oracles read off the real card after drags on {} seeds",
            SEEDS.len()
        ),
        vector,
    )
}
