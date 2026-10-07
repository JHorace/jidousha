//! W4's oracle and its rules, asked by driving the running game with the pointer.
//!
//! MODULES.md's W4 oracle is the first played one: seat Garrick and Brannoc on
//! "Grave goods" in year 1 — by dragging them — and read the card. The fixed parts
//! are shipped literals ("Needs Might", "you bring 14", the §6 lines behind it). The
//! rolled part is asserted as a shape: the demand within 9..11 on every seed, and
//! the card's percentages equal to CONSTANTS §3's table at the margin the card
//! shows — the mapping, over many seeds, not one sample. Then the drags that do
//! not seat (a release over nothing, a lost pointer, a tap, a refuser, a swap) and
//! a drag by finger.
//!
//! INVARIANT: every expectation here is a shipped literal, copied by hand from
//! MODULES.md, CONSTANTS.md §3 and the content — never computed by the code under test.

use crate::board::Slot;
use crate::checks::Checks;
use crate::house::House;
use crate::screen::{Target, UiState};
use crate::scripted::{Pointer, card_lines, center_of, drag, lines_in};
use crate::verify::{SEEDS, hero_named, page_of, point_at, session};

/// MODULES.md W4: the card's need, and what Garrick and Brannoc bring.
pub const W4_NEEDS: &str = "Needs Might";
/// MODULES.md W4 says "you bring 12"; the variant's two Strong traits (+1 each on a
/// Might quest) make it 14 (VARIANT.md, a rewritten oracle).
pub const W4_YOU_BRING: &str = "you bring 14";
/// SPEC §6 behind it, as the quest sheet prints it: Garrick's Might 5 (7, -2 an
/// elder), Thornfall +1, Strong +1, Brannoc's Might 6 and Strong +1; no fear, no bond,
/// no patron.
pub const W4_LINES: [(&str, Option<&str>); 6] = [
    ("You bring", Some("14")),
    ("Garrick, Might 5", None),
    ("  carries Thornfall", Some("+1")),
    ("  is strong", Some("+1")),
    ("Brannoc, Might 6", None),
    ("  is strong", Some("+1")),
];

/// CONSTANTS §3, the rows the oracle can land on, as the card and sheet print them:
/// (power - demand, the card's four, the sheet's triumph/success/setback/disaster).
/// The card's "Succeed" is the table's "as shown" column (SPEC-GAPS KG-25).
pub const W4_ODDS: [(i32, [&str; 4], [&str; 4]); 3] = [
    (
        3,
        ["Succeed 92%", "triumph 42%", "Setback 8%", "disaster 0%"],
        ["42%", "50%", "8%", "0%"],
    ),
    (
        4,
        ["Succeed 97%", "triumph 58%", "Setback 3%", "disaster 0%"],
        ["58%", "39%", "3%", "0%"],
    ),
    (
        5,
        ["Succeed 100%", "triumph 72%", "Setback 0%", "disaster 0%"],
        ["72%", "28%", "0%", "0%"],
    ),
];

/// The seeds the rolled part is asked over: the oracle's three, and a sweep.
fn sweep() -> Vec<u64> {
    SEEDS.iter().copied().chain(100..124).collect()
}

/// Seat Garrick and Brannoc on "Grave goods" by dragging, holding Brannoc over the
/// card first. Returns the card's lines mid-hold, with the session left after the release.
pub fn seat_the_oracle(sim: &mut jidousha::prelude::HeadlessSim) -> Vec<String> {
    let (garrick, brannoc) = (hero_named(sim, "Garrick"), hero_named(sim, "Brannoc"));
    let mut mouse = Pointer::mouse();
    let from = center_of(sim, Target::Hero(garrick));
    let seat = center_of(sim, Target::Seat(Slot::Quest { quest: 0, seat: 0 }));
    drag(sim, &mut mouse, from, seat);
    let from = center_of(sim, Target::Hero(brannoc));
    let card = crate::board_view::quest_rect(0).center();
    mouse.press(sim, from);
    mouse.hold_at(sim, card);
    let held = card_lines(sim, 0);
    mouse.release(sim, card);
    // Resting on the card raises its sheet; step off the board to read the card.
    away(sim);
    held
}

/// Rest the pointer on the top bar, where it points at nothing.
pub fn away(sim: &mut jidousha::prelude::HeadlessSim) {
    crate::verify::point(sim, jidousha::prelude::Vec2::new(700.0, 40.0), false);
}

/// Hold Brannoc over "Grave goods" with Garrick seated, and stop there: the
/// mid-drag picture's stage. He is held over the card's blank corner beside its
/// seats, so the hand covers neither the tile that previews him nor any of the
/// card's type; a card's body lands him on its first free seat.
pub fn stage_mid_drag(sim: &mut jidousha::prelude::HeadlessSim) {
    let (garrick, brannoc) = (hero_named(sim, "Garrick"), hero_named(sim, "Brannoc"));
    let mut mouse = Pointer::mouse();
    let from = center_of(sim, Target::Hero(garrick));
    let seat = center_of(sim, Target::Seat(Slot::Quest { quest: 0, seat: 0 }));
    drag(sim, &mut mouse, from, seat);
    let from = center_of(sim, Target::Hero(brannoc));
    let card = crate::board_view::quest_rect(0);
    let seats = center_of(sim, Target::Seat(Slot::Quest { quest: 0, seat: 1 }));
    mouse.press(sim, from);
    mouse.hold_at(
        sim,
        jidousha::prelude::Vec2::new(card.max.x - 44.0, seats.y),
    );
}

/// The W4 oracle on every seed of the sweep. Returns the summary and the printed vector.
pub fn check_oracle(checks: &mut Checks) -> (String, Vec<String>) {
    let mut vector = Vec::new();
    let mut demands = Vec::new();
    for seed in sweep() {
        let mut sim = session(seed);
        let (garrick, brannoc) = (hero_named(&sim, "Garrick"), hero_named(&sim, "Brannoc"));
        let title = {
            let house = sim.world().resource::<House>();
            house.board.first().map(|p| p.quest.title.clone())
        };
        checks.require(
            title.as_deref() == Some("Grave goods"),
            "W4 oracle: year 1's first card is Grave goods",
            format!("seed {seed:#x}: {title:?}"),
        );
        let held = seat_the_oracle(&mut sim);
        let party = sim.world().resource::<House>().party(0);
        checks.require(
            party == [garrick, brannoc],
            "W4 oracle: two drags seat Garrick then Brannoc on Grave goods",
            format!("seed {seed:#x}: party {party:?}"),
        );
        let lines = card_lines(&sim, 0);
        checks.require(
            held == lines,
            "W4 oracle: the card while Brannoc is held over it reads as it does once he is seated",
            format!("seed {seed:#x}: held {held:?}, seated {lines:?}"),
        );
        let needs = lines.iter().find(|l| l.starts_with(W4_NEEDS)).cloned();
        let demand = needs
            .as_deref()
            .and_then(|l| l.strip_prefix("Needs Might "))
            .and_then(|n| n.parse::<i32>().ok());
        let stored = sim.world().resource::<House>().board[0].quest.demand;
        // MODULES.md W4: "9, 10 or 11 (lower only if the board was eased)" — so the
        // demand plus whatever easing took off this card is 9, 10 or 11.
        let eased = sim
            .world()
            .resource::<House>()
            .board_report
            .as_ref()
            .map_or(0, |r| r.eased[0]);
        checks.require(
            demand.is_some_and(|d| (9..=11).contains(&(d + eased))) && demand == Some(stored),
            "W4 oracle: the card says \"Needs Might\" with a demand of 9, 10 or 11, lower only \
             if the board was eased",
            format!("seed {seed:#x}: {needs:?}, quest demand {stored}, eased {eased}"),
        );
        checks.require(
            lines.iter().any(|l| l == W4_YOU_BRING),
            "W4 oracle: the card says \"you bring 14\" (the variant's traits)",
            format!("seed {seed:#x}: {lines:?}"),
        );
        let Some(demand) = demand else { continue };
        demands.push(demand);
        let row = W4_ODDS.iter().find(|(gap, _, _)| *gap == 14 - demand);
        let shown: Vec<&String> = lines.iter().filter(|l| l.ends_with('%')).collect();
        checks.require(
            row.is_some_and(|(_, card, _)| shown == card.iter().collect::<Vec<_>>()),
            "W4 oracle: the card's percentages are CONSTANTS §3's at power minus demand",
            format!("seed {seed:#x}: demand {demand}, shown {shown:?}, table {row:?}"),
        );
        // The quest sheet behind the card.
        point_at(&mut sim, Target::Quest(0), false);
        let page = page_of(&sim);
        let sheet = lines_in(&page, crate::summer::SHEET);
        let mut breakdown = Vec::new();
        for (text, value) in W4_LINES {
            let at = sheet.iter().position(|l| l == text);
            let beside = at.and_then(|i| sheet.get(i + 1)).cloned();
            let ok = at.is_some() && value.is_none_or(|v| beside.as_deref() == Some(v));
            breakdown.push(format!(
                "{text:?}{}",
                value.map_or(String::new(), |v| format!(" {v}"))
            ));
            checks.require(
                ok,
                "W4 oracle: the quest sheet's power breakdown, line by line",
                format!("seed {seed:#x}: wanted {text:?} {value:?} in {sheet:?}"),
            );
        }
        let outcomes = [
            "Beat it by 4: +3 renown. The least able learns.",
            "Meet it: +2 renown.",
            "Miss by up to 4: one is wounded.",
            "Miss by more: -2 renown, all wounded, each dies 30 in 100.",
        ];
        for (index, outcome) in outcomes.iter().enumerate() {
            let at = sheet.iter().position(|l| l == outcome);
            let beside = at.and_then(|i| sheet.get(i + 1));
            let want = row.map(|(_, _, sheet)| sheet[index]);
            checks.require(
                beside.map(String::as_str) == want,
                "W4 oracle: the sheet's outcome odds are CONSTANTS §3's",
                format!("seed {seed:#x}: {outcome:?} beside {beside:?}, want {want:?}"),
            );
        }
        let needs_line = format!("NEEDS MIGHT {demand}");
        checks.require(
            sheet.contains(&needs_line),
            "W4 oracle: the sheet's need",
            format!("seed {seed:#x}: wanted {needs_line:?}"),
        );
        // Variant: the PERSONAL line makes this sheet longer than the dock, so the history
        // panel at its foot is read with the wheel turned to the end.
        crate::verify::scroll_dock(&mut sim, -(page.dock.total as f32));
        let tail = page_of(&sim);
        let history = lines_in(&tail, history_panel(&tail));
        checks.require(
            history == ["The house has not quested here yet."],
            "the Barrow's history panel: never quested",
            format!("seed {seed:#x}: {history:?}"),
        );
        if seed == SEEDS[0] {
            vector = vec![
                format!(
                    "W4 vector, card: {:?} / {:?} / {}",
                    needs.unwrap_or_default(),
                    W4_YOU_BRING,
                    shown
                        .iter()
                        .map(|s| format!("{s:?}"))
                        .collect::<Vec<_>>()
                        .join(" ")
                ),
                format!("W4 vector, sheet: {}", breakdown.join(", ")),
            ];
        }
    }
    demands.sort_unstable();
    demands.dedup();
    checks.require(
        demands == [9, 10, 11],
        "W4 oracle: over the sweep, the wobble reaches all three demands",
        format!("demands seen {demands:?}"),
    );
    (
        format!(
            "W4 oracle: Garrick and Brannoc dragged onto \"Grave goods\" on {} seeds; demands {demands:?}, every card's odds CONSTANTS §3's",
            sweep().len()
        ),
        vector,
    )
}

/// The history panel on a raised quest sheet: the panel its rows name that is not the sheet.
fn history_panel(page: &crate::screen::Page) -> jidousha::prelude::Rect {
    page.rows
        .iter()
        .map(|row| row.panel)
        .find(|panel| *panel != crate::summer::SHEET && crate::summer::SHEET.contains_rect(*panel))
        .unwrap_or(crate::summer::SHEET)
}

/// The drags that do not seat, a swap, a refuser, and a drag by finger.
pub fn check_drags(checks: &mut Checks) -> String {
    let mut sim = session(SEEDS[0]);
    let (ysolde, odo, garrick, maren) = (
        hero_named(&sim, "Ysolde"),
        hero_named(&sim, "Odo"),
        hero_named(&sim, "Garrick"),
        hero_named(&sim, "Maren"),
    );
    let seated = |sim: &jidousha::prelude::HeadlessSim| {
        let house = sim.world().resource::<House>();
        (
            house.roster,
            house
                .board
                .iter()
                .map(|p| p.seats.clone())
                .collect::<Vec<_>>(),
        )
    };
    let before = seated(&sim);
    let mut mouse = Pointer::mouse();
    // Released over the top bar: Ysolde returns.
    let from = center_of(&sim, Target::Hero(ysolde));
    mouse.press(&mut sim, from);
    let in_hand = sim.world().resource::<UiState>().drag.map(|d| d.hero);
    mouse.hold_at(&mut sim, crate::board_view::quest_rect(0).center());
    let previewed = card_lines(&sim, 0);
    drag_release(
        &mut sim,
        &mut mouse,
        jidousha::prelude::Vec2::new(700.0, 40.0),
    );
    checks.require(
        in_hand == Some(ysolde) && seated(&sim) == before,
        "a drag released over nothing returns the hero and moves no one",
        format!("in hand {in_hand:?}"),
    );
    checks.require(
        previewed.iter().any(|l| l == "you bring 0") && previewed.iter().any(|l| l == "Fear: Ysolde -2"),
        "the preview counts the hand: Ysolde over Grave goods brings 0 (Might 2, Dark -2) and fears it",
        format!("{previewed:?}"),
    );
    away(&mut sim);
    checks.require(
        sim.world().resource::<UiState>().drag.is_none()
            && card_lines(&sim, 0)
                .iter()
                .any(|l| l == "No one is going. Room for 2."),
        "after the release the card is empty again",
        format!("{:?}", card_lines(&sim, 0)),
    );
    // The pointer taken away mid-drag: nothing moves.
    let from = center_of(&sim, Target::Hero(odo));
    let seat = center_of(&sim, Target::Seat(Slot::Quest { quest: 1, seat: 0 }));
    mouse.press(&mut sim, from);
    mouse.hold_at(&mut sim, seat);
    mouse.lose(&mut sim, seat);
    checks.require(
        seated(&sim) == before && sim.world().resource::<UiState>().drag.is_none(),
        "a drag the system takes away (focus lost) is undone",
        String::new(),
    );
    // A tap picks nothing up.
    let at = center_of(&sim, Target::Hero(maren));
    mouse.press_release(&mut sim, at);
    checks.require(
        seated(&sim) == before && sim.world().resource::<UiState>().drag.is_none(),
        "a tap on a hero picks nothing up",
        String::new(),
    );
    // A swap: Odo onto Garrick's seat sends Garrick to Odo's roster seat.
    let odo_seat = sim.world().resource::<House>().slot_of(odo);
    let from = center_of(&sim, Target::Hero(garrick));
    let seat = center_of(&sim, Target::Seat(Slot::Quest { quest: 0, seat: 0 }));
    drag(&mut sim, &mut mouse, from, seat);
    let from = center_of(&sim, Target::Hero(odo));
    let onto = center_of(&sim, Target::Hero(garrick));
    drag(&mut sim, &mut mouse, from, onto);
    let house = sim.world().resource::<House>();
    checks.require(
        house.party(0) == [odo] && house.slot_of(garrick) == odo_seat,
        "a drop on a seated hero swaps them into the dragged hero's seat",
        format!(
            "party {:?}, Garrick at {:?}",
            house.party(0),
            house.slot_of(garrick)
        ),
    );
    // A refuser: Ysolde broken (Dark) is sent back from Grave goods; the card says why.
    sim.world_mut().resource_mut::<House>().heroes[ysolde]
        .fear
        .broken = true;
    let from = center_of(&sim, Target::Hero(ysolde));
    let seat = center_of(&sim, Target::Seat(Slot::Quest { quest: 0, seat: 1 }));
    mouse.press(&mut sim, from);
    mouse.hold_at(&mut sim, seat);
    let warned = card_lines(&sim, 0);
    mouse.release(&mut sim, seat);
    let house = sim.world().resource::<House>();
    checks.require(
        warned.iter().any(|l| l == "Ysolde will not go.")
            && house.party(0) == [odo]
            && house.slot_of(ysolde) == Some(Slot::Roster(0)),
        "a refuser held over a quest is named, and released onto it goes to the first free roster seat",
        format!("held {warned:?}; party {:?}, Ysolde at {:?}", house.party(0), house.slot_of(ysolde)),
    );
    // A finger: Maren dragged onto the bell by touch.
    let mut finger = Pointer::finger();
    let from = center_of(&sim, Target::Hero(maren));
    let seat = center_of(&sim, Target::Seat(Slot::Quest { quest: 1, seat: 0 }));
    drag(&mut sim, &mut finger, from, seat);
    let party = sim.world().resource::<House>().party(1);
    checks.require(
        party == [maren],
        "a drag by finger seats as the mouse does (the touch mirror)",
        format!("the bell's party {party:?}"),
    );
    // A finger whose touch is cancelled mid-drag: undone.
    let from = center_of(&sim, Target::Hero(maren));
    let away = center_of(&sim, Target::Seat(Slot::Quest { quest: 1, seat: 1 }));
    finger.press(&mut sim, from);
    finger.hold_at(&mut sim, away);
    finger.lose(&mut sim, away);
    let party = sim.world().resource::<House>().party(1);
    checks.require(
        party == [maren] && sim.world().resource::<UiState>().drag.is_none(),
        "a cancelled touch undoes its drag",
        format!("the bell's party {party:?}"),
    );
    "drags: released over nothing, lost, tapped, swapped, refused, by finger and cancelled"
        .to_owned()
}

fn drag_release(
    sim: &mut jidousha::prelude::HeadlessSim,
    pointer: &mut Pointer,
    at: jidousha::prelude::Vec2,
) {
    pointer.hold_at(sim, at);
    pointer.release(sim, at);
}

/// Drag the hero named `name` from their card onto `slot`, with the mouse.
pub fn seat(sim: &mut jidousha::prelude::HeadlessSim, name: &str, slot: Slot) {
    let hero = hero_named(sim, name);
    let from = center_of(sim, Target::Hero(hero));
    let to = center_of(sim, Target::Seat(slot));
    drag(sim, &mut Pointer::mouse(), from, to);
}

/// The board's worst case for the floors, staged on the session's own house: two
/// patrons; "Grave goods" at trouble 2 with three seats and Maren, Garrick and
/// Ysolde on it, the Barrow visited thirteen times; the bell at trouble 2 down to
/// one seat, its Coast visited once and remembering Elsbeth.
pub fn stage_worst(sim: &mut jidousha::prelude::HeadlessSim) {
    {
        let house = sim.world_mut().resource_mut::<House>();
        house.patrons = 2;
        let barrow = house.board[0].quest.place.index();
        house.places[barrow] = crate::board::PlaceRecord {
            visits: 13,
            triumphs: 4,
            disasters: 2,
            trouble: 2,
        };
        let q = &mut house.board[0].quest;
        q.trouble = 2;
        q.calm_seats = 3;
        q.seats = 3;
        q.danger = 4;
        q.renown = 4;
        house.board[0].seats = vec![None; 3];
        let coast = &mut house.board[1].quest;
        coast.trouble = 2;
        coast.calm_seats = 3;
        coast.seats = 1;
        house.board[1].seats = vec![None];
        house.places[crate::ids::Place::DrownedCoast.index()].visits = 1;
    }
    seat(sim, "Maren", Slot::Quest { quest: 0, seat: 0 });
    seat(sim, "Garrick", Slot::Quest { quest: 0, seat: 1 });
    seat(sim, "Ysolde", Slot::Quest { quest: 0, seat: 2 });
    away(sim);
}
