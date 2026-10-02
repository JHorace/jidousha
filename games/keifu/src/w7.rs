//! W7's oracle and the played winter: the hearth opened, seated by the scripted pointer,
//! previewed, let pass, and read.
//!
//! **The oracle** (MODULES.md W7), on every recorded seed: year 1, stay home, into the
//! winter; Pip dragged from the yard onto the first bench, Odo held over the bench's
//! teacher seat — the preview reads "+1 Spirit" mid-drag and once he is released; the
//! winter let pass; the winter page reads "Pip trained under Odo. Spirit rises to 3."
//! and "Odo has taken Pip as a student."; and the sheets, pointed at from those lines,
//! read Pip's "Spirit 3" and Odo's "Teach the young two winters (1/2)". **The played
//! winter**: a rest, a lesson, a wedding, the tale told — Ysolde's road-book forged by
//! the tale (staged at her last stage: no first winter reaches it by the rules) — and
//! the dream moments witnessed, all through the screen. **The controls**: a quiet
//! winter, the turning's leaves and "Skip ahead", a child carried to the hall.
//!
//! INVARIANT: every expectation is a shipped literal copied by hand from MODULES.md,
//! SPEC.md, CONSTANTS.md or the content — never computed by the code under test.

use jidousha::prelude::*;
use jidousha::testing::FrameRecorder;

use crate::board::Slot;
use crate::checks::Checks;
use crate::hearth::{Group, Seat};
use crate::hearth_view::{group_rect, notes_now};
use crate::house::House;
use crate::screen::{Target, UiState, WINDOW};
use crate::scripted::{Pointer, center_of, drag, lines_in};
use crate::summer::{SHEET, TOP_BAR};
use crate::telling_view::PANEL;
use crate::verify::{SEEDS, dock_pages, dock_read, hero_named, page_of, point_at, session};
use crate::w5::recorded;

/// MODULES.md W7: the bench's preview.
pub const ORACLE_PREVIEW: &str = "+1 Spirit";
/// MODULES.md W7, and `lines.winter.trained` / `.new_student` / `.dream.counted` as shipped.
pub const ORACLE_PAGE: [&str; 5] = [
    "What the winter did",
    "The winter of year 1",
    "Pip trained under Odo. Spirit rises to 3.",
    "Odo has taken Pip as a student.",
    "Odo comes one nearer his dream: teach the young two winters, 1 of 2.",
];
/// MODULES.md W7: Pip's Spirit after the winter.
pub const ORACLE_SPIRIT: &str = "Spirit 3";
/// MODULES.md W7: Odo's dream on his sheet, its current stage marked as the W1 oracle's is.
pub const ORACLE_DREAM: &str = "[>] Teach the young two winters (1/2)";
/// Odo's sheet's bond to his new student (`bonds.json` STUDENT, power +1).
pub const ORACLE_BOND: &str = "Student Pip +1";

/// After leaving the telling: the hearth of year `year`'s winter is up.
pub fn require_the_hearth(checks: &mut Checks, sim: &HeadlessSim, seed: u64) {
    let house = sim.world().resource::<House>();
    let page = page_of(sim);
    let bar = lines_in(&page, TOP_BAR);
    let pass = lines_in(&page, crate::hearth_view::PASS_BUTTON);
    checks.require(
        house.telling.is_none()
            && house.passage.is_none()
            && house.calendar.is_winter()
            && bar.contains(&"Winter".to_owned())
            && pass == ["Let the winter pass"],
        "leaving the telling does not open the hearth in the same year's winter",
        format!(
            "seed {seed:#x}: year {}, winter {}, bar {bar:?}, control {pass:?}",
            house.calendar.current_year(),
            house.calendar.is_winter()
        ),
    );
}

/// Let the winter pass and read the turning to its end: "Go on" until summer comes.
pub fn through_the_winter(sim: &mut HeadlessSim) {
    point_at(sim, Target::LetWinterPass, true);
    let mut guard = 0;
    while sim.world().resource::<House>().passage.is_some() && guard < 20 {
        point_at(sim, Target::GoOn, true);
        guard += 1;
    }
}

/// Year 1, nobody seated: "Stay home", then the telling left — the hearth is up.
pub fn stay_home_into_winter(sim: &mut HeadlessSim) {
    point_at(sim, Target::SetOut, true);
    point_at(sim, Target::GoOn, true);
}

/// The lines in a group's panel, as the page shows them now.
pub fn group_lines(sim: &HeadlessSim, group: Group) -> Vec<String> {
    lines_in(&page_of(sim), group_rect(group))
}

/// The turning's panel, as the page shows it now.
pub fn turning_lines(sim: &HeadlessSim) -> Vec<String> {
    lines_in(&page_of(sim), PANEL)
}

/// Drag `name` from wherever they sit onto hearth seat `seat`.
pub fn seat_at(sim: &mut HeadlessSim, name: &str, seat: Seat) {
    let hero = hero_named(sim, name);
    let from = center_of(sim, Target::Hero(hero));
    let to = center_of(sim, Target::Seat(Slot::Hearth(seat)));
    drag(sim, &mut Pointer::mouse(), from, to);
}

/// The oracle's mid-drag: Pip on the first bench, Odo held over its teacher seat.
/// Returns the pointer, still held.
pub fn stage_mid_drag(sim: &mut HeadlessSim) -> Pointer {
    stay_home_into_winter(sim);
    seat_at(sim, "Pip", Seat::BenchChild(0));
    let odo = hero_named(sim, "Odo");
    let from = center_of(sim, Target::Hero(odo));
    let to = center_of(sim, Target::Seat(Slot::Hearth(Seat::BenchTeacher(0))));
    let mut mouse = Pointer::mouse();
    mouse.press(sim, from);
    mouse.hold_at(sim, (from + to) * 0.5);
    mouse.hold_at(sim, to);
    mouse
}

/// The oracle seated and let pass: the turning's winter page is up.
pub fn stage_oracle_winter(sim: &mut HeadlessSim) {
    let mut mouse = stage_mid_drag(sim);
    let to = center_of(sim, Target::Seat(Slot::Hearth(Seat::BenchTeacher(0))));
    mouse.release(sim, to);
    point_at(sim, Target::LetWinterPass, true);
}

/// The sheet of the hero a turning line names, read whole through the dock.
fn sheet_from_line(sim: &mut HeadlessSim, hero: usize) -> Vec<String> {
    point_at(sim, Target::Hero(hero), false);
    let mut recorder = FrameRecorder::new(WINDOW);
    let pages = dock_pages(sim, &mut recorder);
    let read = dock_read(&pages).into_iter().map(|l| l.text).collect();
    crate::verify::scroll_dock(sim, 100.0);
    read
}

/// The W7 oracle on every recorded seed. Returns its summary and the vector.
pub fn check_oracle(checks: &mut Checks) -> (String, Vec<String>) {
    let mut vector = Vec::new();
    for seed in recorded() {
        let mut sim = session(seed);
        stay_home_into_winter(&mut sim);
        require_the_hearth(checks, &sim, seed);
        let ui = *sim.world().resource::<UiState>();
        let idle = lines_in(&page_of(&sim), SHEET);
        let mut sim = session(seed);
        let mut mouse = stage_mid_drag(&mut sim);
        let held = sim.world().resource::<UiState>().drag.map(|d| d.hero);
        let mid = group_lines(&sim, Group::Benches);
        let notes = {
            let house = sim.world().resource::<House>();
            notes_now(
                crate::verify::content_of(&sim),
                house,
                sim.world().resource::<UiState>(),
            )
        };
        checks.require(
            held == Some(hero_named(&sim, "Odo"))
                && mid.contains(&ORACLE_PREVIEW.to_owned())
                && notes.benches[0].as_deref() == Some(ORACLE_PREVIEW),
            "mid-drag, Odo held over the bench beside Pip does not preview \"+1 Spirit\"",
            format!("seed {seed:#x}: in hand {held:?}, the benches read {mid:?}, notes {notes:?}"),
        );
        let to = center_of(&sim, Target::Seat(Slot::Hearth(Seat::BenchTeacher(0))));
        mouse.release(&mut sim, to);
        let seated = group_lines(&sim, Group::Benches);
        let house = sim.world().resource::<House>();
        checks.require(
            seated.contains(&ORACLE_PREVIEW.to_owned())
                && house.hearth.at(Seat::BenchChild(0)) == Some(hero_named(&sim, "Pip"))
                && house.hearth.at(Seat::BenchTeacher(0)) == Some(hero_named(&sim, "Odo")),
            "Odo and Pip released onto the bench do not sit there previewing \"+1 Spirit\"",
            format!("seed {seed:#x}: the benches read {seated:?}"),
        );
        point_at(&mut sim, Target::LetWinterPass, true);
        let page = turning_lines(&sim);
        let want: Vec<String> = ORACLE_PAGE.iter().map(|l| (*l).to_owned()).collect();
        checks.require(
            page.len() >= 5 && page[..5] == want[..],
            "letting the winter pass does not tell Pip's lesson and Odo's new student",
            format!("seed {seed:#x}: {page:?}"),
        );
        let (pip, odo) = (hero_named(&sim, "Pip"), hero_named(&sim, "Odo"));
        let pip_sheet = sheet_from_line(&mut sim, pip);
        let odo_sheet = sheet_from_line(&mut sim, odo);
        let house = sim.world().resource::<House>();
        checks.require(
            house.heroes[pip].aptitudes[2] == 3
                && pip_sheet.contains(&ORACLE_SPIRIT.to_owned())
                && odo_sheet.contains(&ORACLE_DREAM.to_owned())
                && odo_sheet.contains(&ORACLE_BOND.to_owned()),
            "after the winter Pip's sheet does not read Spirit 3, or Odo's his dream at 1/2 and his student",
            format!("seed {seed:#x}: Pip {pip_sheet:?}; Odo {odo_sheet:?}"),
        );
        point_at(&mut sim, Target::GoOn, true);
        let house = sim.world().resource::<House>();
        checks.require(
            house.passage.is_none()
                && !house.calendar.is_winter()
                && house.calendar.current_year() == 2,
            "\"Summer comes\" does not bring year 2's summer",
            format!("seed {seed:#x}: year {}", house.calendar.current_year()),
        );
        if seed == SEEDS[0] {
            vector.push(format!(
                "W7 vector, the idle hearth's dock: {idle:?} (pointing {:?})",
                ui.pointing
            ));
            vector.push(format!("W7 vector, the benches mid-drag: {mid:?}"));
            vector.push(format!("W7 vector, the winter page: {page:?}"));
            vector.push(format!("W7 vector, Odo's sheet: {odo_sheet:?}"));
        }
    }
    (
        format!(
            "W7 oracle: Odo on a bench as Pip's teacher previews \"{ORACLE_PREVIEW}\" mid-drag and seated; after the winter Pip's Spirit is 3, \"Odo has taken Pip as a student.\", and Odo's sheet reads \"{ORACLE_DREAM}\" — on {} recorded seeds",
            recorded().len()
        ),
        vector,
    )
}

/// The played winter: year 1 left at home; Garrick by the fire, Maren and Brannoc in the
/// garden, Ysolde at the table at her road's last stage, Pip and Odo on the first bench,
/// Wren alone on the second. Returns the turning's lines.
pub fn stage_played_winter(sim: &mut HeadlessSim) -> Vec<String> {
    stage_played_seating(sim);
    point_at(sim, Target::LetWinterPass, true);
    let mut read = Vec::new();
    let mut guard = 0;
    while guard < 10 {
        read.extend(turning_lines(sim));
        guard += 1;
        if crate::verify::target_rect(sim, Target::Leaf(guard)).is_none() {
            break;
        }
        point_at(sim, Target::Leaf(guard), true);
    }
    read
}

/// The played winter's seating, by the scripted pointer, not yet let pass.
pub fn stage_played_seating(sim: &mut HeadlessSim) {
    stay_home_into_winter(sim);
    let ysolde = hero_named(sim, "Ysolde");
    if let Some(dream) = sim.world_mut().resource_mut::<House>().heroes[ysolde]
        .dream
        .as_mut()
    {
        dream.advance_to_stage(2);
    }
    for (name, seat) in [
        ("Garrick", Seat::Fire(0)),
        ("Maren", Seat::Garden(0)),
        ("Brannoc", Seat::Garden(1)),
        ("Ysolde", Seat::Table(0)),
        ("Pip", Seat::BenchChild(0)),
        ("Odo", Seat::BenchTeacher(0)),
        ("Wren", Seat::BenchChild(1)),
    ] {
        seat_at(sim, name, seat);
    }
    crate::w4::away(sim);
}

/// The played winter, read. Returns its summary.
pub fn check_played(checks: &mut Checks) -> String {
    let mut sim = session(SEEDS[0]);
    stay_home_into_winter(&mut sim);
    for (name, seat) in [("Maren", Seat::Garden(0)), ("Brannoc", Seat::Garden(1))] {
        seat_at(&mut sim, name, seat);
    }
    let garden = group_lines(&sim, Group::Garden);
    checks.require(
        garden.contains(&"will wed".to_owned()),
        "Maren and Brannoc in the garden do not preview \"will wed\"",
        format!("{garden:?}"),
    );
    let mut sim = session(SEEDS[0]);
    let read = stage_played_winter(&mut sim);
    let rests = [
        "Garrick rested by the fire.",
        "Garrick slept late and mended.",
        "Garrick kept to the hearth all winter.",
    ];
    let weddings = [
        "Maren and Brannoc were wed at midwinter. The whole house stood witness.",
        "Maren and Brannoc were wed by the hearth, with snow at the windows.",
        "Maren and Brannoc walked in the garden until it was decided. They are wed.",
    ];
    let tales = [
        "Ysolde told the tale at the long table. People will repeat it.",
        "Ysolde told it plainly, and the hall was silent to the end.",
        "Ysolde told the tale three nights running, and it grew a little each night.",
    ];
    let body: Vec<&str> = read
        .iter()
        .map(String::as_str)
        .filter(|l| !l.starts_with("What the winter did") && !l.starts_with("The winter of year"))
        .collect();
    let fixed = [
        "Pip trained under Odo. Spirit rises to 3.",
        "Odo has taken Pip as a student.",
        "Odo comes one nearer his dream: teach the young two winters, 1 of 2.",
        "Wren sat on the bench alone all winter. A child needs someone to learn from.",
    ];
    let ok = body.len() == 9
        && rests
            .iter()
            .any(|r| body[0] == format!("{r} Dread eased to 1."))
        && body[1..5] == fixed
        && weddings.contains(&body[5])
        && tales
            .iter()
            .any(|t| body[6] == format!("{t} +1 renown to the house, and to Ysolde."))
        && body[7]
            == "Ysolde has done it: to walk every road. She is settled now, and dread has no hold on her."
        && body[8]
            == "It leaves an heirloom: Ysolde's road-book. +2 Wits on quests. Every road there is, written down by Ysolde Vane and finished in year 1.";
    checks.require(
        ok,
        "the played winter does not tell the rest, the lessons, the wedding, the tale and the road-book, in that order",
        format!("{body:?}"),
    );
    let house = sim.world().resource::<House>();
    let (garrick, maren, brannoc, ysolde) = (
        hero_named(&sim, "Garrick"),
        hero_named(&sim, "Maren"),
        hero_named(&sim, "Brannoc"),
        hero_named(&sim, "Ysolde"),
    );
    let wed = house.heroes[maren]
        .bond_to(brannoc)
        .is_some_and(|b| b.kind == crate::ids::BondKind::Spouse);
    let book = house.heroes[ysolde]
        .heirloom
        .as_ref()
        .map(|h| h.name.as_str());
    checks.require(
        wed && book == Some("Ysolde's road-book")
            && house.heroes[garrick].fear.dread == 1
            && house.renown == 12
            && house.heroes[ysolde].renown == 2,
        "after the played winter Maren and Brannoc are not wed, Ysolde holds no road-book, or the renown is not 11 + 1",
        format!(
            "wed {wed}, book {book:?}, Garrick's dread {}, house {}, Ysolde {}",
            house.heroes[garrick].fear.dread, house.renown, house.heroes[ysolde].renown
        ),
    );
    let ysolde_sheet = sheet_from_line(&mut sim, ysolde);
    checks.require(
        ysolde_sheet.contains(&"Ysolde's road-book".to_owned()),
        "Ysolde's sheet does not show the road-book she forged at the table",
        format!("{ysolde_sheet:?}"),
    );
    "W7 played winter: Garrick rested (dread 2 to 1), Pip learned under Odo, Wren waited for a teacher, Maren and Brannoc wed, Ysolde told the tale and her road-book was forged at the table, Odo's dream counted — the winter page in resolution order".to_owned()
}

/// Each group's help as the dock shows it when its heading is pointed at: `ui.winter`'s
/// `*_help` with CONSTANTS §9-§10's numbers, as shipped.
pub const GROUP_HELP: [(Group, &str, &str); 6] = [
    (
        Group::Hall,
        "THE HALL",
        "There are more hands than seats. Whoever is left in the hall keeps the house through the winter and gains nothing by it.",
    ),
    (
        Group::Fire,
        "BY THE FIRE",
        "Only the fire heals. A wounded hero left in the hall goes into summer wounded.",
    ),
    (
        Group::Training,
        "THE TRAINING YARD",
        "Alone, a hero trains their own calling, up to 6. A teacher passes on their best aptitude, up to their own. Veterans and elders teach well. The young learn fast.",
    ),
    (
        Group::Garden,
        "THE GARDEN",
        "Two may wed when both are 18 or older and no more than 15 years apart. Kin cannot wed. From the year after they wed, a pair may have a child, 60 times in a hundred each winter, while both are 18 to 45. Rivals who walk here a winter make peace.",
    ),
    (
        Group::Table,
        "THE LONG TABLE",
        "Tell the tale: +1 renown to the teller, and +1 to the house, once a winter however many tell it. It is also the last step of many dreams.",
    ),
    (
        Group::Benches,
        "THE BENCHES IN THE YARD",
        "From the age of 6, a child learns 1 a winter from whoever sits beside them, up to 4. Their first teacher decides their calling. Drag the child from the yard, and an adult beside them.",
    ),
];

/// Point at a group's heading.
pub fn point_at_group(sim: &mut HeadlessSim, group: Group) {
    let r = if group == Group::Hall {
        crate::hearth_view::hall_rect()
    } else {
        group_rect(group)
    };
    let heading = Rect::from_min_size(r.min, Vec2::new(r.size().x, 14.0));
    crate::verify::point(sim, heading.center(), false);
}

/// The quiet winter, the turning's controls, a child carried to the hall, a hand
/// released over nothing, each group's help. Returns the summary.
pub fn check_controls(checks: &mut Checks) -> String {
    let mut sim = session(SEEDS[0]);
    stay_home_into_winter(&mut sim);
    for (group, heading, help) in GROUP_HELP {
        point_at_group(&mut sim, group);
        let dock = lines_in(&page_of(&sim), SHEET);
        checks.require(
            dock == [heading, help],
            "a group's help in the dock is not its shipped heading and help",
            format!("{group:?}: {dock:?}"),
        );
    }
    let fire = group_lines(&sim, Group::Fire);
    checks.require(
        fire.contains(&"Rest. A wound heals, 1 dread is shed.".to_owned()),
        "the fire's text does not say one dread is shed",
        format!("{fire:?}"),
    );
    // A child can be carried anywhere in winter: Wren from the yard to the hall.
    let wren = hero_named(&sim, "Wren");
    let free = sim
        .world()
        .resource::<House>()
        .roster
        .iter()
        .position(Option::is_none);
    if let Some(free) = free {
        let from = center_of(&sim, Target::Hero(wren));
        let to = center_of(&sim, Target::Seat(Slot::Roster(free)));
        drag(&mut sim, &mut Pointer::mouse(), from, to);
    }
    let landed = sim.world().resource::<House>().slot_of(wren);
    checks.require(
        free.is_some() && landed == free.map(Slot::Roster),
        "a child cannot be carried from the yard to the hall in winter",
        format!("free hall seat {free:?}, Wren now in {landed:?}"),
    );
    // Released over nothing, a hero returns where they sat.
    let odo = hero_named(&sim, "Odo");
    let before = sim.world().resource::<House>().slot_of(odo);
    let from = center_of(&sim, Target::Hero(odo));
    drag(
        &mut sim,
        &mut Pointer::mouse(),
        from,
        Vec2::new(700.0, 30.0),
    );
    let after = sim.world().resource::<House>().slot_of(odo);
    checks.require(
        before.is_some() && before == after,
        "a hero released over nothing in winter does not return to their seat",
        format!("{before:?} then {after:?}"),
    );
    // Nobody in a winter seat: the quiet winter.
    point_at(&mut sim, Target::LetWinterPass, true);
    let quiet = turning_lines(&sim);
    checks.require(
        quiet[..3]
            == [
                "What the winter did",
                "The winter of year 1",
                "A quiet winter. The house kept to the fire.",
            ]
            && lines_in(&page_of(&sim), SHEET).first().map(String::as_str)
                == Some("The year turns. Everyone is a year older. Point at a line to read who it is about."),
        "an empty hearth does not pass as \"A quiet winter.\"",
        format!("{quiet:?}"),
    );
    point_at(&mut sim, Target::Skip, true);
    let house = sim.world().resource::<House>();
    checks.require(
        house.passage.is_none()
            && house.calendar.current_year() == 2
            && !house.calendar.is_winter(),
        "\"Skip ahead\" on the turning does not bring the summer",
        format!("year {}", house.calendar.current_year()),
    );
    // A winter page long enough to continue: the oracle's lines told eight times over.
    let mut sim = session(SEEDS[0]);
    stage_oracle_winter(&mut sim);
    {
        let house = sim.world_mut().resource_mut::<House>();
        if let Some(passage) = house.passage.as_mut() {
            passage.winter = (0..8).flat_map(|_| passage.winter.clone()).collect();
        }
    }
    point_at(&mut sim, Target::GoOn, true);
    let second = turning_lines(&sim);
    let ui = *sim.world().resource::<UiState>();
    checks.require(
        ui.leaf == 1
            && second.first().map(String::as_str) == Some("What the winter did, continued")
            && sim.world().resource::<House>().passage.is_some(),
        "a winter page longer than a leaf does not continue under \"What the winter did, continued\"",
        format!("leaf {}, {second:?}", ui.leaf),
    );
    "W7 controls: a child carried to the hall, a hand released over nothing returns, the quiet winter, the turning's leaves and \"Skip ahead\"".to_owned()
}
