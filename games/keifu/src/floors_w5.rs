//! W5's readability floors: the full board of four, every template on a card, and a
//! ghost's card — judged by `floors.rs`'s `look` at each of its sizes — and the help
//! kept in the idle dock, never in the board's room.

use jidousha::testing::FrameRecorder;

use crate::checks::Checks;
use crate::floors::{Tally, look};
use crate::house::House;
use crate::screen::Target;
use crate::summer::SHEET;
use crate::verify::{page_of, point_at, session};

/// W5's surfaces: a full board with all four cards seated; every one of the 24
/// templates on a card, eight staged boards of four, with each quest sheet; and a
/// ghost's card and sheet. On every board the help stays in the dock: no card, and
/// nothing else on the board, carries it. Returns how many were judged.
pub fn w5_surfaces(
    checks: &mut Checks,
    tally: &mut Tally,
    recorder: &mut FrameRecorder,
    label: &str,
) -> usize {
    use crate::board::Slot;
    use crate::w4::{away, seat};
    let before = tally.surfaces;
    let mut sim = session(crate::verify::SEEDS[0]);
    let seated = [
        ("Garrick", 0),
        ("Brannoc", 0),
        ("Maren", 1),
        ("Odo", 2),
        ("Ysolde", 3),
    ];
    for (name, quest) in seated {
        let at = sim.world().resource::<House>().board[quest]
            .seats
            .iter()
            .position(Option::is_none)
            .unwrap_or(0);
        seat(&mut sim, name, Slot::Quest { quest, seat: at });
    }
    away(&mut sim);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W5, four cards seated",
        false,
    );
    help_stays_in_the_dock(checks, &sim, &format!("{label}, four seated"));
    for (index, board) in crate::w5::template_boards(crate::verify::content_of(&sim))
        .into_iter()
        .enumerate()
    {
        let mut sim = session(crate::verify::SEEDS[0]);
        crate::w5::stage_board(&mut sim, &board);
        let name = format!("W5, templates board {index}");
        look(checks, tally, recorder, &mut sim, &name, false);
        help_stays_in_the_dock(checks, &sim, &format!("{label}, {name}"));
        for quest in 0..4 {
            point_at(&mut sim, Target::Quest(quest), false);
            let sheet = format!("{name}, quest sheet {quest}");
            look(checks, tally, recorder, &mut sim, &sheet, false);
        }
    }
    let mut sim = session(crate::verify::SEEDS[0]);
    crate::w5::stage_ghost_board(&mut sim);
    look(
        checks,
        tally,
        recorder,
        &mut sim,
        "W5, a ghost on the board",
        false,
    );
    let ghost = sim
        .world()
        .resource::<House>()
        .board
        .iter()
        .position(|p| matches!(p.quest.source, crate::quest::Source::Ghost(_)));
    if let Some(slot) = ghost {
        point_at(&mut sim, Target::Quest(slot), false);
        look(
            checks,
            tally,
            recorder,
            &mut sim,
            "W5, the ghost's quest sheet",
            false,
        );
    }
    checks.require(
        ghost.is_some(),
        "the staged ghost board posts no ghost quest",
        label.to_owned(),
    );
    tally.surfaces - before
}

/// The help is the idle dock's and only the dock's: with nothing pointed at, the
/// dock reads the help, then next summer's foresight, every card is drawn, and no row outside the dock
/// carries the help or the old empty-slot copy.
fn help_stays_in_the_dock(checks: &mut Checks, sim: &jidousha::prelude::HeadlessSim, name: &str) {
    let page = page_of(sim);
    let help = crate::verify::content_of(sim).words[crate::words::W::SummerHelp].to_owned();
    let cards = page
        .targets
        .iter()
        .filter(|(_, t)| matches!(t, Target::Quest(_)))
        .count();
    let elsewhere: Vec<&str> = page
        .rows
        .iter()
        .filter(|row| row.panel != SHEET)
        .filter(|row| help.contains(row.text.trim()) && row.text.trim().len() > 12)
        .map(|row| row.text.as_str())
        .collect();
    checks.require(
        crate::scripted::lines_in(&page, SHEET).first() == Some(&help)
            && cards == 4
            && elsewhere.is_empty(),
        "a full board crowds the dock, loses a card, or carries the help outside the dock",
        format!("{name}: {cards} cards; help rows outside the dock {elsewhere:?}"),
    );
}
