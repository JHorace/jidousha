//! keifu-fixes-r2's checks: the telegraph on the card and the sheet, and the foresight
//! in the dock, each read off the page at seating and then held against the board next
//! summer actually deals.
//!
//! **The telegraph**: year 1, nobody seated — every card carries "Left: danger .., room
//! .." and its sheet "Left: trouble .. here. Next time: ..". The two quests year 1 always
//! posts, Grave goods and the bell, are held to shipped literals worked by hand from
//! `quests.json` (the Barrow's and the Coast's other three templates at trouble 1 in year
//! 2). Then "Stay home", the winter let pass, and every year-2 template quest at a place
//! that had a card must lie inside what its card and sheet said: its trouble, danger,
//! seats and renown inside the ranges, its demand (before easing) at most the shown need.
//!
//! **The foresight**: pointing at NEXT SUMMER reads, in the dock, each place's trouble
//! next summer as seated; seating a party on Grave goods turns the Barrow's "trouble 1"
//! to "calm" at once; after the summer every quest dealt has the trouble shown for its
//! place. A staged ghost is promised by name and place, and comes.
//!
//! INVARIANT: the comparison reads the numbers off the page text, never off
//! `outlook.rs`, so a fault in the readings is a mismatch with the board dealt.

use jidousha::prelude::*;

use crate::checks::Checks;
use crate::house::House;
use crate::ids::Place;
use crate::outlook::Span;
use crate::quest::Source;
use crate::screen::Target;
use crate::scripted::{card_lines, lines_in};
use crate::summer::SHEET;
use crate::verify::{SEEDS, page_of, point_at, session};
use crate::w4::away;
use crate::w5::recorded;

/// Year 1's two forced quests, left: the Barrow's other templates (lamps 2-3 seats danger
/// 2, a name 1-2 danger 1, the king 3 danger 3) at trouble 1 in year 2, and the Coast's
/// (the wreck 2-3 danger 2, the nets 3 danger 3, the tables 1-2 danger 1): danger 2-4,
/// room 1-2, need at most 2 * (3 + 3 + 1) + 1 = 15, renown 2-4.
const YEAR_ONE_CARD: &str = "Left: danger 2-4, room 1-2";
const YEAR_ONE_SHEET: &str =
    "Left: trouble 1 here. Next time: danger 2-4, room 1-2, needs up to 15, pays 2-4.";

/// "2-4" or "3".
fn span(text: &str) -> Option<Span> {
    let text = text.trim_end_matches([',', '.']);
    match text.split_once('-') {
        Some((low, high)) => Some(Span {
            low: low.parse().ok()?,
            high: high.parse().ok()?,
        }),
        None => text.parse().ok().map(|v| Span { low: v, high: v }),
    }
}

/// What a sheet's telegraph line says, parsed back: trouble, danger, seats, need, renown.
#[derive(Clone, Copy, Debug)]
struct Told {
    trouble: i32,
    danger: Span,
    seats: Span,
    need: i32,
    renown: Span,
}

fn told(line: &str) -> Option<Told> {
    let words: Vec<&str> = line.split_whitespace().collect();
    let after = |word: &str| {
        words
            .iter()
            .position(|w| *w == word)
            .and_then(|i| words.get(i + 1))
            .copied()
    };
    Some(Told {
        trouble: after("trouble")?.parse().ok()?,
        danger: span(after("danger")?)?,
        seats: span(after("room")?)?,
        need: after("to")?.trim_end_matches(',').parse().ok()?,
        renown: span(after("pays")?)?,
    })
}

/// The dock's lines now.
fn dock(sim: &HeadlessSim) -> Vec<String> {
    lines_in(&page_of(sim), SHEET)
}

/// Each place's trouble as the foresight reads it, off the dock's lines.
fn foreseen(content: &crate::content::Content, lines: &[String]) -> Vec<(Place, Option<Span>)> {
    Place::ALL
        .iter()
        .copied()
        .filter(|&p| p != Place::SealedDoor)
        .map(|place| {
            let title = &content.lore.places[place.index()].title;
            let shown = lines.iter().find_map(|l| {
                let rest = l.strip_prefix(title.as_str())?.strip_prefix(": ")?;
                if rest == "calm" {
                    Some(Span { low: 0, high: 0 })
                } else {
                    span(rest.strip_prefix("trouble ")?)
                }
            });
            (place, shown)
        })
        .collect()
}

/// The content, as the game loads it.
fn load() -> crate::content::Content {
    match crate::content::load() {
        Ok(content) => content,
        Err(error) => crate::checks::fail("the content did not load", &error.to_string()),
    }
}

/// Leave the telling through its own controls: "Skip ahead" where it is offered, else
/// "Go on", until the hearth is up.
fn leave_the_telling(sim: &mut HeadlessSim) {
    let mut guard = 0;
    while sim.world().resource::<House>().telling.is_some() && guard < 200 {
        let skip = crate::verify::target_rect(sim, Target::Skip).is_some();
        point_at(sim, if skip { Target::Skip } else { Target::GoOn }, true);
        guard += 1;
    }
}

/// Read the foresight with the pointer on NEXT SUMMER.
fn read_foresight(sim: &mut HeadlessSim) -> Vec<String> {
    point_at(sim, Target::Outlook, false);
    let lines = dock(sim);
    away(sim);
    lines
}

/// The telegraph and the foresight, on every recorded seed. Returns the summary.
pub fn check_telegraph_and_foresight(checks: &mut Checks) -> String {
    let (mut held, mut ghosts_skipped, mut places) = (0, 0, 0);
    for seed in recorded() {
        let mut sim = session(seed);
        let content = load();
        let board: Vec<Place> = sim
            .world()
            .resource::<House>()
            .board
            .iter()
            .map(|p| p.quest.place)
            .collect();
        let mut said: Vec<(Place, Told)> = Vec::new();
        for (slot, place) in board.iter().enumerate() {
            let card = card_lines(&sim, slot);
            let card_line = card.iter().find(|l| l.starts_with("Left: danger")).cloned();
            point_at(&mut sim, Target::Quest(slot), false);
            let sheet_line = dock(&sim)
                .into_iter()
                .find(|l| l.starts_with("Left: trouble"));
            away(&mut sim);
            let parsed = sheet_line.as_deref().and_then(told);
            let agree = match (&card_line, parsed) {
                (Some(card), Some(t)) => {
                    *card == format!("Left: danger {}, room {}", t.danger.told(), t.seats.told())
                }
                _ => false,
            };
            checks.require(
                agree,
                "r2 telegraph: an empty card and its sheet both say what leaving it does, and agree",
                format!("seed {seed:#x}, slot {slot}: card {card:?}; sheet line {sheet_line:?}"),
            );
            if matches!(place, Place::Barrow | Place::DrownedCoast) {
                checks.require(
                    card_line.as_deref() == Some(YEAR_ONE_CARD)
                        && sheet_line.as_deref() == Some(YEAR_ONE_SHEET),
                    "r2 telegraph: year 1's forced quests telegraph their places' other templates at trouble 1",
                    format!("seed {seed:#x}, {place:?}: card {card_line:?}, sheet {sheet_line:?}"),
                );
            }
            if let Some(t) = parsed {
                said.push((*place, t));
            }
        }
        let foresight = read_foresight(&mut sim);
        let shown = foreseen(&content, &foresight);
        let as_seated = shown.iter().all(|(place, span)| {
            let want = if board.contains(place) { 1 } else { 0 };
            *span
                == Some(Span {
                    low: want,
                    high: want,
                })
        });
        checks.require(
            foresight.first().map(String::as_str) == Some("NEXT SUMMER") && as_seated,
            "r2 foresight: with nobody seated in year 1, the four posted places will be troubled 1 and the others calm",
            format!("seed {seed:#x}: {foresight:?}"),
        );
        crate::w7::stay_home_into_winter(&mut sim);
        crate::w7::through_the_winter(&mut sim);
        let house = sim.world().resource::<House>();
        for (slot, posted) in house.board.iter().enumerate() {
            let quest = &posted.quest;
            places += 1;
            let promised = shown
                .iter()
                .find(|(p, _)| *p == quest.place)
                .and_then(|(_, s)| *s);
            checks.require(
                promised.is_some_and(|s| s.holds(quest.trouble)),
                "r2 foresight: next summer's quest has the trouble the foresight showed for its place",
                format!(
                    "seed {seed:#x}: {:?} at {:?}, trouble {}, shown {promised:?}",
                    quest.title, quest.place, quest.trouble
                ),
            );
            let Some((_, t)) = said.iter().find(|(p, _)| *p == quest.place) else {
                continue;
            };
            if matches!(quest.source, Source::Ghost(_)) {
                ghosts_skipped += 1;
                continue;
            }
            let eased = house.board_report.as_ref().map_or(0, |r| r.eased[slot]);
            held += 1;
            checks.require(
                quest.trouble == t.trouble
                    && t.danger.holds(quest.danger)
                    && t.seats.holds(quest.seats)
                    && t.renown.holds(quest.renown)
                    && quest.demand + eased <= t.need,
                "r2 telegraph: a quest left in year 1 comes back in year 2 inside the range its card and sheet showed",
                format!(
                    "seed {seed:#x}: {:?} trouble {} danger {} seats {} renown {} demand {} (+{eased} eased); told {t:?}",
                    quest.title, quest.trouble, quest.danger, quest.seats, quest.renown, quest.demand
                ),
            );
        }
    }
    format!(
        "r2 telegraph and foresight: {} recorded seeds left year 1 alone; {held} year-2 quests at a telegraphed place inside their shown ranges ({ghosts_skipped} ghost quests outside the telegraph), {places} quests at the trouble foreseen for their place",
        recorded().len()
    )
}

/// Seating changes the foresight at once; a seated summer keeps it; a ghost is foreseen.
pub fn check_foresight_live(checks: &mut Checks) -> String {
    for seed in SEEDS {
        let mut sim = session(seed);
        let content = load();
        let barrow = |lines: &[String]| {
            foreseen(&content, lines)
                .into_iter()
                .find(|(p, _)| *p == Place::Barrow)
                .and_then(|(_, s)| s)
        };
        let before = read_foresight(&mut sim);
        crate::w4::seat_the_oracle(&mut sim);
        let card = card_lines(&sim, 0);
        let after = read_foresight(&mut sim);
        checks.require(
            barrow(&before) == Some(Span { low: 1, high: 1 })
                && barrow(&after) == Some(Span { low: 0, high: 0 })
                && after.contains(&"The Barrow: calm".to_owned())
                && !card.iter().any(|l| l.starts_with("Left:")),
            "r2 foresight: seating Garrick and Brannoc on Grave goods turns the Barrow from trouble 1 to calm, and the seated card telegraphs nothing",
            format!("seed {seed:#x}: before {before:?}; after {after:?}; card {card:?}"),
        );
        point_at(&mut sim, Target::SetOut, true);
        leave_the_telling(&mut sim);
        crate::w7::through_the_winter(&mut sim);
        let house = sim.world().resource::<House>();
        let barrow_now = house.places[Place::Barrow.index()].trouble;
        checks.require(
            barrow_now == 0 && house.calendar.current_year() == 2,
            "r2 foresight: the Barrow, answered, is calm next summer as foreseen",
            format!(
                "seed {seed:#x}: year {}, Barrow trouble {barrow_now}",
                house.calendar.current_year()
            ),
        );
    }
    // A staged ghost: Garrick's, at Emberfall, foreseen by name and place, and posted.
    let mut sim = session(SEEDS[0]);
    crate::w5::stage_ghost_board(&mut sim);
    let garrick = crate::verify::hero_named(&sim, "Garrick");
    let lines = read_foresight(&mut sim);
    let promise = "The ghost of Garrick comes to Emberfall next summer.";
    checks.require(
        lines.contains(&promise.to_owned()),
        "r2 foresight: the first ghost is foreseen by name and place",
        format!("{lines:?}"),
    );
    crate::w7::stay_home_into_winter(&mut sim);
    crate::w7::through_the_winter(&mut sim);
    let posted = sim
        .world()
        .resource::<House>()
        .board
        .iter()
        .any(|p| p.quest.source == Source::Ghost(garrick) && p.quest.place == Place::Emberfall);
    checks.require(
        posted,
        "r2 foresight: the foreseen ghost's quest is posted next summer",
        "no ghost quest for Garrick at Emberfall".to_owned(),
    );
    format!(
        "r2 foresight live: seating on Grave goods turns the Barrow calm on {} seeds and it is; \"{promise}\" and it comes",
        SEEDS.len()
    )
}

/// The capture stages: the foresight open in the dock, and an empty card's telegraph
/// beside its sheet.
pub fn stage(sim: &mut HeadlessSim, name: &str) {
    match name {
        "outlook" => point_at(sim, Target::Outlook, false),
        _ => point_at(sim, Target::Quest(0), false),
    }
}
