//! The three decision-row gates (DESIGN.md G3–G5), one per row of the spec's
//! decision-surface table, each on a stack staged before tick 1.
//!
//! Every expectation is a shipped literal — the rows as a person reads them,
//! the order they resolve in — never arithmetic over the constants under test
//! (make-game §A.6), so a mutation of a card's damage or of the resolution
//! order moves the game and not the check.

use jidousha::prelude::*;
use jidousha::ui::{Panel, glyph_run};

use crate::cards::Card;
use crate::checks::{Album, Checks, Driver};
use crate::duel::{Duel, Phase, Seat};
use crate::rules::{Item, ItemId, Side, forecast, legal_targets};
use crate::screen::{Art, HAND_PANEL, LEADING, PRIORITY_AT, STACK_ROWS, palette, screen};
use crate::{Choosing, Flow};

/// The seed every staged table carries; it only matters to `Enter` on a result.
const STAGED_SEED: u64 = 7;

/// One staged item, top-first: id, card, owner, whom it affects, its target.
type Staged = (u32, Card, Side, Side, Option<u32>);

/// A table on turn 3, Your turn, You holding priority, both at 15 life.
pub fn table(you_hand: &[Card], rival_energy: u32, top_first: &[Staged]) -> Flow {
    let stack: Vec<Item> = top_first
        .iter()
        .rev()
        .map(|&(id, card, owner, affects, target)| Item {
            id: ItemId(id),
            card,
            owner,
            affects,
            target: target.map(ItemId),
        })
        .collect();
    let next_id = stack.iter().map(|item| item.id.0).max().unwrap_or(0) + 1;
    let seat = |hand: Vec<Card>, energy| Seat {
        life: 15,
        energy,
        hand,
        deck: vec![Card::Bolt, Card::Counter, Card::Mend, Card::Bolt],
    };
    let duel = Duel {
        seats: [
            seat(you_hand.to_vec(), 3),
            seat(vec![Card::Bolt, Card::Counter, Card::Blast], rival_energy),
        ],
        stack,
        turn: 3,
        active: Side::You,
        priority: Side::You,
        passes: 0,
        next_id,
        phase: Phase::Live,
        log: Vec::new(),
    };
    Flow {
        duel,
        choosing: None,
        rival_wait: 0,
        seed: STAGED_SEED,
    }
}

/// G3's table: a Bolt of yours, the Rival's Blast over it, its Counter on top.
pub fn respond_table() -> Flow {
    use Card::*;
    use Side::*;
    table(
        &[Counter, Bolt, Redirect, Mend],
        0,
        &[
            (3, Counter, Rival, Rival, Some(1)),
            (2, Blast, Rival, You, None),
            (1, Bolt, You, Rival, None),
        ],
    )
}

/// The stack panel's row `row` as (text, colour) pairs, left to right.
pub fn stack_row(panel: &Panel<Art>, row: usize) -> Vec<(String, Color)> {
    let y = STACK_ROWS.y + row as f32 * LEADING;
    let mut runs: Vec<_> = panel
        .runs
        .iter()
        .filter(|run| (run.at.y - y).abs() < 0.01 && run.at.x < HAND_PANEL.min.x)
        .collect();
    runs.sort_by(|a, b| a.at.x.total_cmp(&b.at.x));
    runs.iter()
        .map(|run| (run.text.clone(), run.style.color))
        .collect()
}

/// Just the texts of `stack_row`.
pub fn row_texts(panel: &Panel<Art>, row: usize) -> Vec<String> {
    stack_row(panel, row)
        .into_iter()
        .map(|(text, _)| text)
        .collect()
}

/// The texts of rows `0..count`, for a message.
fn rows_seen(panel: &Panel<Art>, count: usize) -> String {
    (0..count)
        .map(|row| row_texts(panel, row).join(" | "))
        .collect::<Vec<_>>()
        .join(" / ")
}

/// The priority line as the panel carries it.
pub fn priority_text(panel: &Panel<Art>) -> String {
    panel
        .runs
        .iter()
        .find(|run| run.at == PRIORITY_AT)
        .map_or_else(String::new, |run| run.text.clone())
}

/// The stack's ids, top-first.
pub fn ids(duel: &Duel) -> Vec<u32> {
    duel.stack.iter().rev().map(|item| item.id.0).collect()
}

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|each| (*each).to_owned()).collect()
}

/// G3, row 1: at a three-item window every item and the order is on screen,
/// and the resolutions that follow are the forecast's steps, one by one.
pub fn respond(checks: &mut Checks, album: &mut Album) -> String {
    let mut driver = Driver::new(Some(respond_table()), STAGED_SEED);
    let frame = driver.shoot(album, "decision 1: the three-item window");
    let window = driver.flow().clone();
    let preview = forecast(&window.duel);
    let step_ids: Vec<u32> = preview.steps.iter().map(|step| step.item.0).collect();
    let step_says: Vec<&str> = preview
        .steps
        .iter()
        .map(|step| step.says.as_str())
        .collect();
    checks.require(
        step_ids == [3, 2] && step_says == ["Counter: voids #1", "Blast: You 15>10"],
        "decision 1: the forecast of the staged window is not Counter then Blast",
        format!("steps {step_ids:?} saying {step_says:?}"),
    );
    let fate_says: Vec<&str> = preview
        .fates
        .iter()
        .map(|fate| fate.says.as_str())
        .collect();
    let cells: Vec<String> = preview.fates.iter().map(|fate| fate.order_cell()).collect();
    checks.require(
        fate_says == ["Counter: voids #1", "Blast: You 15>10", "voided by #3"]
            && cells == ["1", "2", "x"]
            && preview.final_life(&window.duel) == [10, 15],
        "decision 1: the forecast's per-item lines are not what the window will do",
        format!(
            "says {fate_says:?}, order {cells:?}, final life {:?}",
            preview.final_life(&window.duel)
        ),
    );
    let panel = screen(&window);
    let want = [
        strings(&["1", "#3", "Counter Rival", "-> #1", "Counter: voids #1"]),
        strings(&["2", "#2", "Blast Rival", "Blast: You 15>10"]),
        strings(&["x", "#1", "Bolt You", "voided by #3"]),
    ];
    let shown: Vec<Vec<String>> = (0..3).map(|row| row_texts(&panel, row)).collect();
    checks.require(
        shown == want,
        "decision 1: the stack panel does not list the items top-first with order and forecast",
        format!("rows {}", rows_seen(&panel, 3)),
    );
    let font = album.shots[album.shots.len() - 1].font;
    let mut missing = Vec::new();
    for run in panel.runs.iter().filter(|run| run.at.x < HAND_PANEL.min.x) {
        let width = run.style.width_of(&run.text);
        let drawn = glyph_run(&frame, font, run.at, run.style.size, width);
        if drawn != run.text.chars().count() {
            missing.push(format!("{:?} drew {drawn}", run.text));
        }
    }
    checks.require(
        missing.is_empty(),
        "decision 1: a row of the stack panel is not all on the frame",
        format!("short rows: {missing:?}"),
    );
    let hand: Vec<bool> = (0..4)
        .map(|slot| crate::screen::slot_live(&window, slot))
        .collect();
    let hand_ink = panel
        .runs
        .iter()
        .filter(|run| run.at.x >= HAND_PANEL.min.x && run.at.y >= STACK_ROWS.y)
        .filter(|run| run.at.y < STACK_ROWS.y + 4.0 * LEADING)
        .all(|run| run.style.color == Color::WHITE);
    let redirect = legal_targets(Card::Redirect, &window.duel.stack);
    checks.require(
        hand == [true; 4] && hand_ink && redirect == [ItemId(2), ItemId(1)],
        "decision 1: the hand does not mark every response playable",
        format!("playable {hand:?}, all rows in ink {hand_ink}, Redirect may aim at {redirect:?}"),
    );
    let mut resolved = Vec::new();
    while !driver.flow().duel.stack.is_empty() && resolved.len() < 6 {
        driver.press(Key::Space);
        driver.until_yours(200);
        let duel = &driver.flow().duel;
        let line = duel.log.last().cloned().unwrap_or_default();
        resolved.push((line, duel.life(), ids(duel)));
    }
    let predicted: Vec<_> = preview
        .steps
        .iter()
        .map(|step| {
            let after: Vec<u32> = step.ids_after().iter().map(|id| id.0).collect();
            (step.says.clone(), step.life_after, after)
        })
        .collect();
    checks.require(
        resolved == predicted,
        "decision 1: what resolved is not what the window forecast",
        format!("resolved {resolved:?}; forecast {predicted:?}"),
    );
    format!(
        "decision 1 (respond): window rows {}; resolved {} steps as forecast",
        rows_seen(&panel, 3),
        resolved.len()
    )
}

/// G4, row 2: the priority line says what a pass will do, and a pass does it.
pub fn pass(checks: &mut Checks, album: &mut Album) -> String {
    let fresh = respond_table();
    let mut answered = respond_table();
    answered.duel.passes = 1;
    let mut empty = table(&[Card::Bolt, Card::Delay], 0, &[]);
    empty.duel.passes = 1;
    empty.duel.seats[0].energy = 1;
    let lines = [&fresh, &answered, &empty].map(|flow| priority_text(&screen(flow)));
    checks.require(
        lines
            == [
                "YOU HOLD PRIORITY - Rival may still respond",
                "YOU HOLD PRIORITY - Rival passed: Space resolves #3",
                "YOU HOLD PRIORITY - Rival passed: Space ends the turn",
            ],
        "decision 2: the priority line does not say what a pass would do",
        format!("lines {lines:?}"),
    );
    let preview = forecast(&answered.duel);
    let mut driver = Driver::new(Some(answered), STAGED_SEED);
    driver.shoot(album, "decision 2: the Rival has passed");
    driver.press(Key::Space);
    let duel = driver.flow().duel.clone();
    let first = preview.steps.first();
    let same = first.is_some_and(|step| {
        step.life_after == duel.life()
            && step.stack_after == duel.stack
            && duel.log.last() == Some(&step.says)
    });
    checks.require(
        same && ids(&duel) == [2]
            && duel.life() == [15, 15]
            && duel.log.last().map(String::as_str) == Some("Counter: voids #1")
            && duel.passes == 0
            && duel.priority == Side::You,
        "decision 2: one pass after the Rival's did not resolve exactly the forecast's first step",
        format!(
            "stack {:?}, life {:?}, log {:?}, passes {}, priority {:?}; forecast step {:?}",
            ids(&duel),
            duel.life(),
            duel.log.last(),
            duel.passes,
            duel.priority,
            first.map(|step| (&step.says, step.life_after)),
        ),
    );
    let mut ender = Driver::new(Some(empty), STAGED_SEED);
    ender.press(Key::Space);
    let next = &ender.flow().duel;
    let hands = [next.seats[0].hand.len(), next.seats[1].hand.len()];
    let energy = [next.seats[0].energy, next.seats[1].energy];
    checks.require(
        next.turn == 4 && next.active == Side::Rival && energy == [3, 3] && hands == [3, 4],
        "decision 2: a pass on an empty stack after the Rival's did not end the turn",
        format!(
            "turn {}, active {:?}, energy {energy:?}, hands {hands:?} (were 2 and 3)",
            next.turn, next.active
        ),
    );
    format!("decision 2 (pass): lines read {lines:?}; resolve and end-turn both as stated")
}

/// G5's table: Your Bolt between the Rival's Mend and its Counter on the Bolt.
fn target_table() -> Flow {
    use Card::*;
    use Side::*;
    table(
        &[Delay, Redirect],
        0,
        &[
            (3, Counter, Rival, Rival, Some(2)),
            (2, Bolt, You, Rival, None),
            (1, Mend, Rival, Rival, None),
        ],
    )
}

/// The mark cell and colour of each row while aiming, as (mark, colour) pairs.
fn marks(panel: &Panel<Art>, rows: usize) -> Vec<(String, Color)> {
    (0..rows)
        .map(|row| {
            let cells = stack_row(panel, row);
            let tone = cells.first().map_or(Color::BLACK, |(_, color)| *color);
            let mark = cells
                .iter()
                .find(|(text, _)| text == ">" || text == "+")
                .map_or(" ".to_owned(), |(text, _)| text.clone());
            (mark, tone)
        })
        .collect()
}

/// G5, row 3: aiming marks exactly the legal targets, and the order shown after
/// a Delay is the order that resolves.
pub fn target(checks: &mut Checks, album: &mut Album) -> String {
    let (mark, dim) = (palette::MARK, palette::DIM);
    let mut driver = Driver::new(Some(target_table()), STAGED_SEED);
    driver.press(Key::Digit2);
    let aiming = driver.flow().clone();
    driver.shoot(album, "decision 3: aiming a Redirect");
    let seen = marks(&screen(&aiming), 3);
    let legal = legal_targets(Card::Redirect, &aiming.duel.stack);
    let want = vec![
        (" ".to_owned(), dim),
        (">".to_owned(), mark),
        ("+".to_owned(), mark),
    ];
    checks.require(
        seen == want && legal == [ItemId(2), ItemId(1)],
        "decision 3: aiming a Redirect does not mark exactly the effect items",
        format!("marks {seen:?}; legal {legal:?}"),
    );
    driver.press(Key::Escape);
    let cleared = marks(&screen(driver.flow()), 3);
    checks.require(
        driver.flow().choosing.is_none() && cleared.iter().all(|(each, _)| each == " "),
        "decision 3: Esc did not clear the marks",
        format!("marks {cleared:?}, choosing {:?}", driver.flow().choosing),
    );
    driver.press(Key::Digit1);
    let delay = marks(&screen(driver.flow()), 3);
    for _ in 0..3 {
        driver.press(Key::ArrowDown);
    }
    let wrapped = driver.flow().choosing;
    let marked: Vec<&str> = delay.iter().map(|(each, _)| each.as_str()).collect();
    checks.require(
        marked == [">", "+", "+"] && wrapped == Some(Choosing { slot: 0, cursor: 0 }),
        "decision 3: a Delay does not mark all three with the cursor wrapping",
        format!("marks {marked:?}; after three Downs {wrapped:?}"),
    );
    driver.press(Key::Enter);
    driver.until_yours(200);
    let after = driver.flow().clone();
    driver.shoot(album, "decision 3: the Delay is on the stack");
    let panel = screen(&after);
    let want_rows = [
        strings(&["1", "#4", "Delay You", "-> #3", "Delay: #3 to bottom"]),
        strings(&["4", "#3", "Counter Rival", "-> #2", "Counter: fizzles"]),
        strings(&["2", "#2", "Bolt You", "Bolt: Rival 15>12"]),
        strings(&["3", "#1", "Mend Rival", "Mend: Rival 12>15"]),
    ];
    let rows: Vec<Vec<String>> = (0..4).map(|row| row_texts(&panel, row)).collect();
    let steps: Vec<u32> = forecast(&after.duel)
        .steps
        .iter()
        .map(|step| step.item.0)
        .collect();
    checks.require(
        rows == want_rows && steps == [4, 2, 1, 3],
        "decision 3: the panel after the Delay does not show the new order",
        format!("rows {}; forecast steps {steps:?}", rows_seen(&panel, 4)),
    );
    driver.press(Key::Space);
    let moved = screen(driver.flow());
    let order: Vec<Vec<String>> = (0..3)
        .map(|row| row_texts(&moved, row).into_iter().take(2).collect())
        .collect();
    let mut resolved = Vec::new();
    while let Some(top) = driver.flow().duel.stack.last().map(|item| item.id.0) {
        if resolved.len() >= 6 {
            break;
        }
        resolved.push(top);
        driver.press(Key::Space);
        driver.until_yours(200);
    }
    let life = driver.flow().duel.life();
    checks.require(
        order
            == [
                strings(&["1", "#2"]),
                strings(&["2", "#1"]),
                strings(&["3", "#3"]),
            ]
            && resolved == [2, 1, 3]
            && life == [15, 15],
        "decision 3: the order shown after the Delay is not the order that resolved",
        format!("shown {order:?}; resolved {resolved:?}; life {life:?}"),
    );
    format!(
        "decision 3 (target): Redirect marked {legal:?}; after Delay shown {order:?}, resolved \
         {resolved:?}"
    )
}
