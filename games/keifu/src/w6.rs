//! W6's oracle and the played summer: set out, the telling read leaf by leaf, and the
//! year moving on.
//!
//! **The stay-home oracle** (MODULES.md W6), on every recorded seed: year 1, nobody
//! goes, "Stay home" — the telling's Meanwhile reads four "No one went." lines in place
//! order and "-4 renown", the house is at 11; next summer every quest at a troubled
//! place shows one seat fewer (never below one), +1 danger, "Renown +" one higher and
//! "unanswered -2". **The played summer**: Garrick and Brannoc dragged onto "Grave
//! goods" (W4's drag), set out, the telling typed and paged, the Meanwhile read, the
//! year moved on. **Witnessing's live path**: the real resolution feeding W3's machinery
//! — every played triumph settles Garrick and lays his rest, and no other outcome does.
//! Then the controls' other paths: a leaf button, "Skip ahead", the house closing.
//!
//! INVARIANT: every expectation is a shipped literal copied by hand from MODULES.md,
//! SPEC.md, CONSTANTS.md or the content — the trouble lines, the costs, the 90 letters
//! a second — never computed by the code under test.

use jidousha::prelude::*;

use crate::checks::Checks;
use crate::house::House;
use crate::ids::Outcome;
use crate::quest_card::read_card;
use crate::screen::Target;
use crate::scripted::{card_lines, lines_in};
use crate::summer::{SET_OUT_BUTTON, TOP_BAR};
use crate::telling_view::PANEL;
use crate::verify::{SEEDS, hero_named, page_of, point_at, session};
use crate::w5::recorded;

/// MODULES.md W6: each place's "No one went." line after a first unanswered year
/// (`lines.summer.unanswered` over `lore.json`'s trouble lines with "a year").
pub const STAY_HOME: [&str; 6] = [
    "The Barrow's dead have walked a year unanswered. No one went.",
    "The tide has had the Drowned Coast to itself for a year. No one went.",
    "The High Pass has been shut a year. No one went.",
    "Emberfall has burned a year untended. No one went.",
    "The Court has waited a year for an answer, and is counting. No one went.",
    "The Deepwood has grown a year nearer the road. No one went.",
];
/// The same trouble lines, as a troubled card shows them with nobody going.
pub const TROUBLED_CARD: [&str; 6] = [
    "The Barrow's dead have walked a year unanswered.",
    "The tide has had the Drowned Coast to itself for a year.",
    "The High Pass has been shut a year.",
    "Emberfall has burned a year untended.",
    "The Court has waited a year for an answer, and is counting.",
    "The Deepwood has grown a year nearer the road.",
];

/// The telling panel's lines as the page shows them now.
pub fn telling_lines(sim: &HeadlessSim) -> Vec<String> {
    lines_in(&page_of(sim), PANEL)
}

/// The labels of the buttons on the page now, by target.
fn label(sim: &HeadlessSim, target: Target) -> Option<String> {
    let page = page_of(sim);
    let rect = page.targets.iter().find(|(_, t)| *t == target)?.0;
    lines_in(&page, rect).into_iter().next()
}

/// Let `ticks` ticks pass with no input.
pub fn wait(sim: &mut HeadlessSim, ticks: u64) {
    for _ in 0..ticks {
        sim.tick();
    }
}

/// The stay-home oracle on every recorded seed. Returns its summary.
pub fn check_stay_home(checks: &mut Checks) -> String {
    let mut troubled_cards = 0;
    for seed in recorded() {
        let mut sim = session(seed);
        let places: Vec<usize> = sim
            .world()
            .resource::<House>()
            .board
            .iter()
            .map(|p| p.quest.place.index())
            .collect();
        checks.require(
            lines_in(&page_of(&sim), SET_OUT_BUTTON) == ["Stay home"],
            "with nobody seated the set-out control does not read \"Stay home\"",
            format!(
                "seed {seed:#x}: {:?}",
                lines_in(&page_of(&sim), SET_OUT_BUTTON)
            ),
        );
        point_at(&mut sim, Target::SetOut, true);
        let mut want = vec!["Meanwhile".to_owned()];
        want.extend(places.iter().map(|&p| STAY_HOME[p].to_owned()));
        want.push("The house is thought less of for it: -4 renown.".to_owned());
        let read = telling_lines(&sim);
        checks.require(
            read == want,
            "staying home in year 1 does not tell four \"No one went.\" lines in place order and -4 renown",
            format!("seed {seed:#x}: read {read:?}"),
        );
        let bar = lines_in(&page_of(&sim), TOP_BAR);
        checks.require(
            sim.world().resource::<House>().renown == 11
                && bar.contains(&"House renown 11".to_owned()),
            "staying home in year 1 does not leave the house at 11",
            format!(
                "seed {seed:#x}: renown {}, bar {bar:?}",
                sim.world().resource::<House>().renown
            ),
        );
        checks.require(
            label(&sim, Target::GoOn).as_deref() == Some("Winter comes")
                && label(&sim, Target::Leaf(1)).is_none(),
            "the stay-home telling is not one leaf ending in \"Winter comes\"",
            format!("seed {seed:#x}: {:?}", label(&sim, Target::GoOn)),
        );
        point_at(&mut sim, Target::GoOn, true);
        let house = sim.world().resource::<House>();
        let bar = lines_in(&page_of(&sim), TOP_BAR);
        checks.require(
            house.telling.is_none()
                && house.calendar.current_year() == 2
                && bar.contains(&"Year 2 of 25".to_owned()),
            "leaving the telling does not bring year 2's summer",
            format!(
                "seed {seed:#x}: year {}, bar {bar:?}",
                house.calendar.current_year()
            ),
        );
        let content = crate::verify::content_of(&sim);
        let page = page_of(&sim);
        for slot in 0..house.board.len() {
            let quest = &house.board[slot].quest;
            let troubled = places.contains(&quest.place.index());
            let seats = page
                .targets
                .iter()
                .filter(|(_, t)| matches!(t, Target::Seat(crate::board::Slot::Quest { quest: q, .. }) if *q == slot))
                .count() as i32;
            let card = card_lines(&sim, slot);
            let reading = read_card(content, house, slot, &[], None);
            let (want_seats, want_danger, want_renown, want_cost) = if troubled {
                (
                    (quest.calm_seats - 1).max(1),
                    (quest.calm_danger + 1).min(4),
                    quest.calm_danger + 1,
                    2,
                )
            } else {
                (quest.calm_seats, quest.calm_danger, quest.calm_danger, 1)
            };
            let renown_line = format!("Renown +{want_renown}");
            let cost_line = format!("unanswered -{want_cost}");
            let trouble_shown =
                !troubled || card.contains(&TROUBLED_CARD[quest.place.index()].to_owned());
            troubled_cards += usize::from(troubled);
            checks.require(
                seats == want_seats
                    && reading.danger == want_danger
                    && card.contains(&renown_line)
                    && card.contains(&cost_line)
                    && trouble_shown,
                "a year-2 card at a place left unanswered does not show one seat fewer, +1 danger, Renown + one higher and unanswered -2",
                format!(
                    "seed {seed:#x}, {:?} (troubled {troubled}): seats {seats} (want {want_seats}), danger {} (want {want_danger}), card {card:?}",
                    quest.title, reading.danger
                ),
            );
        }
    }
    format!(
        "W6 oracle (stay home): year 1's Meanwhile tells four \"No one went.\" lines in place order and \"-4 renown\", leaving 11, on {} recorded seeds; next summer {troubled_cards} cards at troubled places show one seat fewer, +1 danger, Renown + one higher, unanswered -2",
        recorded().len()
    )
}

/// The margin's telling, from `ui.telling` as shipped.
fn margin_telling(margin: i32) -> String {
    match margin {
        0 => "met it exactly".to_owned(),
        m if m > 0 => format!("beat it by {m}"),
        m => format!("missed by {}", -m),
    }
}

/// CONSTANTS §3's bands, as shipped: TRIUMPH >= 4, SUCCESS 0..3, SETBACK -1..-4, else DISASTER.
pub fn band(margin: i32) -> Outcome {
    if margin >= 4 {
        Outcome::Triumph
    } else if margin >= 0 {
        Outcome::Success
    } else if margin >= -4 {
        Outcome::Setback
    } else {
        Outcome::Disaster
    }
}

/// The played summer on every recorded seed: Garrick and Brannoc dragged onto "Grave
/// goods", set out, the story typed and completed, the page read, the Meanwhile read,
/// the year moved on. Witnessing's live path is asked of every one. Returns the summary.
pub fn check_played(checks: &mut Checks) -> (String, Vec<String>) {
    let mut outcomes = [0usize; 4];
    let mut vector = Vec::new();
    for seed in recorded() {
        let mut sim = session(seed);
        crate::w4::seat_the_oracle(&mut sim);
        let (garrick, brannoc) = (hero_named(&sim, "Garrick"), hero_named(&sim, "Brannoc"));
        let others: Vec<usize> = sim.world().resource::<House>().board[1..]
            .iter()
            .map(|p| p.quest.place.index())
            .collect();
        checks.require(
            lines_in(&page_of(&sim), SET_OUT_BUTTON) == ["Set out"],
            "with a party seated the set-out control does not read \"Set out\"",
            format!(
                "seed {seed:#x}: {:?}",
                lines_in(&page_of(&sim), SET_OUT_BUTTON)
            ),
        );
        point_at(&mut sim, Target::SetOut, true);
        let Some(page) = sim
            .world()
            .resource::<House>()
            .telling
            .as_ref()
            .and_then(|t| t.pages.first().cloned())
        else {
            checks.require(
                false,
                "setting out with a party wrote no quest page",
                format!("seed {seed:#x}"),
            );
            continue;
        };
        // The typewriter: nothing of the story on the tick it opens; 90 letters a second.
        let story_typed = |sim: &HeadlessSim| -> String {
            telling_lines(sim)
                .into_iter()
                .find(|l| page.story.starts_with(l.as_str()) && !l.is_empty())
                .unwrap_or_default()
        };
        let opened = story_typed(&sim);
        wait(&mut sim, 30);
        let half = story_typed(&sim);
        let before = telling_lines(&sim);
        checks.require(
            opened.is_empty() && half.chars().count() == 45.min(page.story.chars().count()),
            "the story is not typed at 90 letters a second from the tick its leaf opens",
            format!(
                "seed {seed:#x}: {:?} at open, {:?} after 30 ticks",
                opened, half
            ),
        );
        let outcome_word = ["Disaster", "Setback", "Success", "Triumph"][page.outcome.index()];
        checks.require(
            !before.contains(&outcome_word.to_owned())
                && page.lines.first().is_none_or(|l| !before.contains(l)),
            "the outcome or the page's lines show before the story is typed",
            format!("seed {seed:#x}: {before:?}"),
        );
        point_at(&mut sim, Target::GoOn, true);
        let read = telling_lines(&sim);
        let [a, b] = page.dice;
        let demand = page.quest.demand;
        let margin = 12 + a + b - 7 - demand;
        let roll = format!(
            "Needed Might {demand}. Brought 12. Dice {a} and {b}, less 7: {}.",
            margin_telling(margin)
        );
        let premise = "Robbers went in at dusk. Bring them out, or what is left.";
        let head_ok = read.len() >= 6
            && read[..3]
                == [
                    "The Barrow".to_owned(),
                    "Grave goods".to_owned(),
                    premise.to_owned(),
                ]
            && read.contains(&page.story)
            && read.contains(&outcome_word.to_owned())
            && read.contains(&roll);
        checks.require(
            head_ok && page.power == 12 && page.margin == margin && page.outcome == band(margin) && page.members == [garrick, brannoc],
            "the played summer's page does not read place, title, premise, story, outcome and roll as the roll made them",
            format!("seed {seed:#x}: {read:?}, page power {} margin {} {:?}", page.power, page.margin, page.outcome),
        );
        outcomes[page.outcome.index()] += 1;
        // Witnessing, live: a triumph at the Barrow settles Garrick and lays his rest.
        let settled = sim.world().resource::<House>().heroes[garrick].settled;
        let fulfilled = "Garrick has done it: to lay the Barrow's dead to rest. He is settled now, and dread has no hold on him.";
        let rest = "It leaves a blessing, Garrick's rest: +2 against Undead, for Garrick, Maren and Pip and every child born to them.";
        let told = page.lines.iter().position(|l| l == fulfilled);
        let triumph = page.outcome == Outcome::Triumph;
        checks.require(
            settled == triumph
                && told.is_some() == triumph
                && told.is_none_or(|at| page.lines.get(at + 1).map(String::as_str) == Some(rest)),
            "witnessing's live path: a triumph on Grave goods does not settle Garrick and lay his rest, or another outcome does",
            format!("seed {seed:#x}: {:?}, settled {settled}, lines {:?}", page.outcome, page.lines),
        );
        // On to the Meanwhile: the three unanswered places, then the total.
        let mut guard = 0;
        while telling_lines(&sim).first().is_none_or(|l| l != "Meanwhile") && guard < 12 {
            point_at(&mut sim, Target::GoOn, true);
            guard += 1;
        }
        let mut want = vec!["Meanwhile".to_owned()];
        want.extend(others.iter().map(|&p| STAY_HOME[p].to_owned()));
        want.push("The house is thought less of for it: -3 renown.".to_owned());
        let meanwhile = telling_lines(&sim);
        checks.require(
            meanwhile == want && label(&sim, Target::GoOn).as_deref() == Some("Winter comes"),
            "the played summer's Meanwhile does not tell the three unanswered places and -3 renown, last",
            format!("seed {seed:#x}: {meanwhile:?}"),
        );
        point_at(&mut sim, Target::GoOn, true);
        let house = sim.world().resource::<House>();
        checks.require(
            house.telling.is_none() && house.calendar.current_year() == 2 && house.board.len() == 4,
            "leaving the played summer's telling does not bring year 2's board",
            format!("seed {seed:#x}: year {}", house.calendar.current_year()),
        );
        if seed == SEEDS[0] {
            vector.push(format!(
                "W6 vector, played summer page (seed {seed:#x}): {read:?}"
            ));
            vector.push(format!("W6 vector, its Meanwhile: {meanwhile:?}"));
        }
    }
    (
        format!(
            "W6 played summer: Garrick and Brannoc on \"Grave goods\", set out and read, on {} recorded seeds — disaster {}, setback {}, success {}, triumph {}; every triumph settled Garrick and laid his rest, live",
            recorded().len(),
            outcomes[0],
            outcomes[1],
            outcomes[2],
            outcomes[3]
        ),
        vector,
    )
}

/// The other paths through the controls: a leaf button and the typewriter's restart,
/// "Skip ahead", and a house that closes. Returns the summary.
pub fn check_controls(checks: &mut Checks) -> String {
    // A leaf button turns to its leaf and types the story again; "Skip ahead" leaves.
    let mut sim = session(SEEDS[0]);
    crate::w4::seat_the_oracle(&mut sim);
    point_at(&mut sim, Target::SetOut, true);
    point_at(&mut sim, Target::GoOn, true);
    point_at(&mut sim, Target::GoOn, true);
    let ui = *sim.world().resource::<crate::screen::UiState>();
    point_at(&mut sim, Target::Leaf(0), true);
    let back = *sim.world().resource::<crate::screen::UiState>();
    checks.require(
        ui.leaf == 1 && back.leaf == 0 && !back.revealed,
        "\"Go on\" does not complete the story then turn the leaf, or a leaf button does not turn back and retype",
        format!("after two presses {ui:?}; after the first leaf's button {back:?}"),
    );
    // A member's card is read on the telling, never lifted.
    let garrick = hero_named(&sim, "Garrick");
    point_at(&mut sim, Target::Leaf(0), true);
    let card = crate::scripted::center_of(&sim, Target::Hero(garrick));
    let mut mouse = crate::scripted::Pointer::mouse();
    mouse.press(&mut sim, card);
    mouse.hold_at(&mut sim, card + Vec2::new(0.0, 120.0));
    let held = sim.world().resource::<crate::screen::UiState>().drag;
    mouse.release(&mut sim, card + Vec2::new(0.0, 120.0));
    checks.require(
        held.is_none(),
        "a member's card can be picked up on the telling",
        format!("{held:?}"),
    );
    point_at(&mut sim, Target::Skip, true);
    let house = sim.world().resource::<House>();
    checks.require(
        house.telling.is_none() && house.calendar.current_year() == 2,
        "\"Skip ahead\" does not leave the telling",
        format!("year {}", house.calendar.current_year()),
    );
    // A house at renown 1 that stays home: the Meanwhile ends with the closing line,
    // the last button reads "The last of it", and leaving shows the closed verdict.
    let mut sim = session(SEEDS[0]);
    sim.world_mut().resource_mut::<House>().renown = 1;
    point_at(&mut sim, Target::SetOut, true);
    let read = telling_lines(&sim);
    checks.require(
        read.last().map(String::as_str)
            == Some("The house's name is spent. No one will send to it again.")
            && label(&sim, Target::GoOn).as_deref() == Some("The last of it"),
        "a spent house's telling does not end with the closing line and \"The last of it\"",
        format!("{read:?}, {:?}", label(&sim, Target::GoOn)),
    );
    point_at(&mut sim, Target::GoOn, true);
    let page = page_of(&sim);
    let ended = lines_in(&page, crate::ending_view::PANEL);
    checks.require(
        sim.world().resource::<House>().closed
            && ended.first().map(String::as_str) == Some("The house closed its doors")
            && ended.contains(&"It was year 1, with the Door still 25 years off.".to_owned()),
        "leaving a spent house's telling does not close it with the closed verdict",
        format!("{ended:?}"),
    );
    point_at(&mut sim, Target::BeginAgain, true);
    let house = sim.world().resource::<House>();
    checks.require(
        !house.closed
            && house.renown == 15
            && house.calendar.current_year() == 1
            && house.telling.is_none(),
        "\"Begin another house\" does not found a new house in year 1",
        format!(
            "closed {}, renown {}, year {}",
            house.closed,
            house.renown,
            house.calendar.current_year()
        ),
    );
    "W6 controls: a leaf button turns and retypes, \"Skip ahead\" leaves, a spent house closes on leaving and begins again".to_owned()
}
