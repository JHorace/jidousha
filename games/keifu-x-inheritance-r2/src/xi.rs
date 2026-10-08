//! The variant's checks (DESIGN.md "Gates to add"), rows 1 and 2 of the task's decision
//! table — the oath on the card, the outsider at the garden — and the rules beside them:
//! who may swear, and two outsiders. Row 3 and the rest are `xi_heirs.rs`.
//!
//! INVARIANT: every expectation here is a shipped literal, copied by hand from the
//! content, CONSTANTS and DESIGN.md — never computed by the code under test. Where a
//! line names a rolled wanderer, only the name is read off the house.

use crate::checks::Checks;
use crate::hearth::Group;
use crate::hero::HeroId;
use crate::house::House;
use crate::ids::BondKind;
use crate::marks::{Mark, house_marks};
use crate::passage::PageKind;
use crate::screen::Target;
use crate::scripted::{card_lines, lines_in};
use crate::summer::SHEET;
use crate::verify::{hero_named, page_of, point_at, session};
use crate::w5::recorded;
use crate::xi_stages::{
    button, content, dock_lines, garden, grave_goods, object, oracle_at_ten, resolve, sworn,
    turning_lines, wanderer,
};

/// DESIGN.md row 1: the card's sworn line, power 12 against 10 (triumph 10 of 36).
pub const SWORN: &str = "Sworn by Garrick: triumph, or a mark of 2 on the name. Fails 72 in 100.";
/// The quest sheet's block.
pub const SHEET_BLOCK: [&str; 5] = [
    "SWORN BY GARRICK",
    "Garrick must triumph, or the name is marked.",
    "Fails 72 in 100.",
    "Kept: +2 renown to Garrick and +2 to the house.",
    "Failed: -2 renown to Garrick and -2 to the house, and a mark of 2 on the name: -1 renown a year while it stands, passed to his heir at half its weight, buried with the last to carry it.",
];
/// `lines.oath.failed` and `lines.oath.kept` for Garrick on "Grave goods".
pub const FAILED: &str = "Garrick swore it in the family's name and failed: -2 renown to him, -2 to the house, and a mark of 2 on the name.";
pub const KEPT: &str = "Garrick swore it in the family's name and kept the oath: +2 renown to him and +2 to the house.";

/// Decision row 1: the oath's stakes are on the card and the sheet before "Set out";
/// failed on a fixed roll the house carries exactly that mark; kept, the renown.
pub fn check_oath(checks: &mut Checks) -> (String, Vec<String>) {
    let mut vector = Vec::new();
    for seed in recorded() {
        let mut sim = session(seed);
        oracle_at_ten(&mut sim);
        let garrick = hero_named(&sim, "Garrick");
        let before = card_lines(&sim, 0);
        checks.require(
            button(&sim, 0).as_deref() == Some("Swear it")
                && !before.iter().any(|l| l.starts_with("Sworn by")),
            "Garrick called on \"Grave goods\" is not offered the oath, or is shown sworn before swearing",
            format!("seed {seed:#x}: button {:?}, card {before:?}", button(&sim, 0)),
        );
        point_at(&mut sim, Target::Swear(0), true);
        let card = card_lines(&sim, 0);
        let oath = sim.world().resource::<House>().board[0].sworn;
        checks.require(
            card.contains(&SWORN.to_owned())
                && button(&sim, 0).as_deref() == Some("Withdraw")
                && oath.map(|o| o.by) == Some(garrick),
            "the sworn card does not show who swore, the chance it fails and the mark a failure puts on the name",
            format!("seed {seed:#x}: card {card:?}, oath {oath:?}"),
        );
        point_at(&mut sim, Target::Quest(0), false);
        let sheet = dock_lines(&mut sim);
        let block = sheet
            .iter()
            .position(|l| l == SHEET_BLOCK[0])
            .map(|at| sheet[at..(at + SHEET_BLOCK.len()).min(sheet.len())].to_vec());
        checks.require(
            block.as_deref() == Some(&SHEET_BLOCK.map(String::from)[..]),
            "the quest sheet does not spell out the sworn oath's stakes",
            format!("seed {seed:#x}: {sheet:?}"),
        );
        // Withdrawn, then sworn again.
        point_at(&mut sim, Target::Swear(0), true);
        let withdrawn = sim.world().resource::<House>().board[0].sworn;
        checks.require(
            withdrawn.is_none()
                && button(&sim, 0).as_deref() == Some("Swear it")
                && !card_lines(&sim, 0).contains(&SWORN.to_owned()),
            "pressing the oath button again does not withdraw the oath",
            format!("seed {seed:#x}: {withdrawn:?}"),
        );
        point_at(&mut sim, Target::Swear(0), true);
        if seed == recorded()[0] {
            vector.push(format!("XI vector, the sworn card: {card:?}"));
            vector.push(format!(
                "XI vector, its sheet's block: {:?}",
                block.unwrap_or_default()
            ));
        }
        // Failed: dice 1 and 1, margin -3, a setback below the triumph needed.
        let lines = resolve(&mut sim, [1, 1]);
        let house = sim.world().resource::<House>();
        let marks: Vec<(HeroId, Mark)> = house_marks(house)
            .into_iter()
            .map(|(id, mark)| (id, mark.clone()))
            .collect();
        let want = [(garrick, grave_goods(garrick, 2))];
        checks.require(
            marks == want
                && house.renown == 13
                && house.heroes[garrick].renown == 4
                && lines.contains(&FAILED.to_owned()),
            "a failed oath does not put exactly a mark of 2 on the name and take 2 renown from the house (15 to 13) and Garrick (6 to 4)",
            format!(
                "seed {seed:#x}: marks {marks:?}, house {}, Garrick {}, lines {lines:?}",
                house.renown, house.heroes[garrick].renown
            ),
        );
        // Kept: dice 6 and 6, margin 7, a triumph.
        let mut sim = session(seed);
        sworn(&mut sim);
        let lines = resolve(&mut sim, [6, 6]);
        let house = sim.world().resource::<House>();
        checks.require(
            house_marks(house).is_empty()
                && house.renown == 20
                && house.heroes[garrick].renown == 11
                && lines.contains(&KEPT.to_owned()),
            "a kept oath does not add 2 to the house (15 + 3 won + 2 = 20) and Garrick (6 + 3 + 2 = 11) and mark nothing",
            format!(
                "seed {seed:#x}: house {}, Garrick {}, lines {lines:?}",
                house.renown, house.heroes[garrick].renown
            ),
        );
        if seed == recorded()[0] {
            vector.push(format!("XI vector, failed: {FAILED:?}; kept: {KEPT:?}"));
        }
    }
    (
        format!(
            "XI row 1, the oath: Garrick sworn on \"Grave goods\" shows \"Fails 72 in 100\" and a mark of 2 on the card and the sheet before Set out; failed on 1+1 the house carries exactly that mark, kept on 6+6 it gains 2 — on {} recorded seeds",
            recorded().len()
        ),
        vector,
    )
}

/// Who may swear: not on a ghost's quest or a lock of the Door, not an outsider, not by a
/// burden, not a hero who is not going; and lifting the swearer off withdraws it.
pub fn check_oath_eligibility(checks: &mut Checks) -> String {
    let content = content();
    let seed = recorded()[0];
    let mut sim = session(seed);
    oracle_at_ten(&mut sim);
    let garrick = hero_named(&sim, "Garrick");
    let party = sim.world().resource::<House>().party(0);
    let may = |house: &House, hero: HeroId| {
        crate::oath::may_swear(&content, &house.heroes, &house.board[0].quest, &party, hero)
    };
    let mut ghost = sim.world().resource::<House>().clone();
    ghost.board[0].quest.source = crate::quest::Source::Ghost(hero_named(&sim, "Elsbeth"));
    let mut door = sim.world().resource::<House>().clone();
    door.board[0].quest.source = crate::quest::Source::Door(0);
    let mut burden = sim.world().resource::<House>().clone();
    burden.heroes[garrick].burden = burden.heroes[garrick].dream.take();
    let mut outsider = sim.world().resource::<House>().clone();
    outsider.heroes[garrick].family = false;
    let house = sim.world().resource::<House>();
    let brannoc = hero_named(&sim, "Brannoc");
    checks.require(
        may(house, garrick) == Some(crate::ids::Outcome::Triumph)
            && may(&ghost, garrick).is_none()
            && may(&door, garrick).is_none()
            && may(&burden, garrick).is_none()
            && may(&outsider, garrick).is_none()
            && may(house, brannoc).is_none(),
        "the oath is offered where the variant forbids it: a ghost's quest, the Door, a burden's call, an outsider, or an uncalled hero",
        format!(
            "seed {seed:#x}: own {:?}, ghost {:?}, door {:?}, burden {:?}, outsider {:?}, Brannoc {:?}",
            may(house, garrick),
            may(&ghost, garrick),
            may(&door, garrick),
            may(&burden, garrick),
            may(&outsider, garrick),
            may(house, brannoc)
        ),
    );
    // On the page: an outsider called by the same dream shows no button.
    let wanderer = wanderer(&mut sim, 30, 0);
    {
        let house = sim.world_mut().resource_mut::<House>();
        house.heroes[wanderer].dream = house.heroes[garrick].dream.clone();
        house.board[0].seats = vec![Some(wanderer), None];
    }
    let called = crate::calls::dream_call(
        &content,
        &sim.world().resource::<House>().heroes,
        wanderer,
        sim.world().resource::<House>().board[0].quest.facts(),
        &[wanderer],
    );
    checks.require(
        called.is_some() && button(&sim, 0).is_none(),
        "an outsider called by their dream is offered the oath",
        format!(
            "seed {seed:#x}: called {called:?}, button {:?}",
            button(&sim, 0)
        ),
    );
    // Lifting the swearer off the card withdraws the oath; nobody else may swear.
    let mut sim = session(seed);
    sworn(&mut sim);
    let free = sim
        .world()
        .resource::<House>()
        .roster
        .iter()
        .position(Option::is_none)
        .unwrap_or(0);
    let from = crate::scripted::center_of(&sim, Target::Hero(garrick));
    let to = crate::scripted::center_of(&sim, Target::Seat(crate::board::Slot::Roster(free)));
    crate::scripted::drag(&mut sim, &mut crate::scripted::Pointer::mouse(), from, to);
    let left = sim.world().resource::<House>().board[0].sworn;
    checks.require(
        left.is_none()
            && button(&sim, 0).is_none()
            && !card_lines(&sim, 0).contains(&SWORN.to_owned()),
        "lifting the swearer off the card does not withdraw the oath",
        format!("seed {seed:#x}: {left:?}, button {:?}", button(&sim, 0)),
    );
    "XI oath rules: no oath on a ghost's quest, the Door, a burden's call, for an outsider or an uncalled hero; the swearer lifted off withdraws it".to_owned()
}

/// Decision row 2: an outsider below the threshold is refused, at it accepted, with the
/// renown and the threshold on the garden, its help, the sheet and the winter page.
pub fn check_marry_in(checks: &mut Checks) -> (String, Vec<String>) {
    let mut vector = Vec::new();
    for seed in recorded() {
        // Unproven: renown 2.
        let mut low = session(seed);
        let outsider = garden(&mut low, 2);
        let name = low.world().resource::<House>().heroes[outsider]
            .name
            .clone();
        let pronoun = low.world().resource::<House>().heroes[outsider].pronoun;
        let note = crate::w7::group_lines(&low, Group::Garden);
        crate::w7_controls::point_at_group(&mut low, Group::Garden);
        let help = lines_in(&page_of(&low), SHEET).join(" ");
        point_at(&mut low, Target::Hero(outsider), false);
        let sheet = lines_in(&page_of(&low), SHEET);
        checks.require(
            note.iter().any(|l| l == "unproven, renown 2 of 4")
                && help.contains("An outsider weds in only at renown 4,")
                && sheet.contains(&"Renown 2, an outsider: 4 to wed into the house".to_owned()),
            "the garden does not show an outsider's renown against the threshold before the winter is let pass",
            format!("seed {seed:#x}: note {note:?}, help {help:?}, sheet {sheet:?}"),
        );
        // Garrick to his death, to read his heirs after the winter.
        let garrick = hero_named(&low, "Garrick");
        low.world_mut().resource_mut::<House>().heroes[garrick].age = 93;
        point_at(&mut low, Target::LetWinterPass, true);
        let winter = turning_lines(&low, PageKind::Winter);
        let refused = format!(
            "{name} is unproven: renown 2 of 4. The house will not have {} yet.",
            object(pronoun)
        );
        let house = low.world().resource::<House>();
        let ysolde = hero_named(&low, "Ysolde");
        checks.require(
            winter.contains(&refused)
                && house.heroes[outsider].bond_to(ysolde).is_none()
                && !house.heroes[outsider].family,
            "an unproven outsider is not refused at the garden",
            format!("seed {seed:#x}: {winter:?}"),
        );
        let low_renown = house.renown;
        crate::w8::go_to_the_choice(&mut low);
        let low_heirs: Vec<String> = crate::w8::heir_labels(&page_of(&low))
            .into_iter()
            .map(|(_, l)| l)
            .collect();
        // Proven: renown 4.
        let mut high = session(seed);
        let outsider = garden(&mut high, 4);
        let note = crate::w7::group_lines(&high, Group::Garden);
        high.world_mut().resource_mut::<House>().heroes[garrick].age = 93;
        point_at(&mut high, Target::LetWinterPass, true);
        let winter = turning_lines(&high, PageKind::Winter);
        let wed =
            format!("{name} weds into the house and takes the name Vane: renown +2 to the house.");
        let house = high.world().resource::<House>();
        let spouses = house.heroes[outsider]
            .bond_to(ysolde)
            .is_some_and(|b| b.kind == BondKind::Spouse)
            && house.heroes[ysolde]
                .bond_to(outsider)
                .is_some_and(|b| b.kind == BondKind::Spouse);
        checks.require(
            note.iter().any(|l| l == "will wed, and take the name Vane")
                && winter.contains(&wed)
                && spouses
                && house.heroes[outsider].family
                && house.heroes[outsider].house == "Vane"
                && house.renown == low_renown + 2,
            "an outsider at the threshold does not wed in, take the name Vane and bring 2 renown to the house",
            format!(
                "seed {seed:#x}: note {note:?}, winter {winter:?}, spouses {spouses}, renown {} against {low_renown}",
                house.renown
            ),
        );
        crate::w8::go_to_the_choice(&mut high);
        let high_heirs: Vec<String> = crate::w8::heir_labels(&page_of(&high))
            .into_iter()
            .map(|(_, l)| l)
            .collect();
        let named = format!("{name}, of the house");
        checks.require(
            high_heirs.contains(&named) && !low_heirs.contains(&named),
            "only the outsider who wed in is offered as Garrick's heir",
            format!("seed {seed:#x}: wed in {high_heirs:?}; unproven {low_heirs:?}"),
        );
        if seed == recorded()[0] {
            vector.push(format!("XI vector, the garden unproven: \"unproven, renown 2 of 4\"; the winter: {refused:?}"));
            vector.push(format!("XI vector, the garden proven: \"will wed, and take the name Vane\"; the winter: {wed:?}"));
        }
    }
    (
        format!(
            "XI row 2, marrying in: an outsider of renown 2 is refused (\"unproven, renown 2 of 4\"), of renown 4 weds in, takes the name and is an heir — on {} recorded seeds",
            recorded().len()
        ),
        vector,
    )
}

/// Two outsiders in the garden: "neither is family", and no wedding.
pub fn check_neither_family(checks: &mut Checks) -> String {
    let seed = recorded()[0];
    let mut sim = session(seed);
    crate::w7::stay_home_into_winter(&mut sim);
    let first = wanderer(&mut sim, 24, 9);
    let second = wanderer(&mut sim, 25, 9);
    let names = [first, second].map(|id| sim.world().resource::<House>().heroes[id].name.clone());
    crate::w7::seat_at(&mut sim, &names[0], crate::hearth::Seat::Garden(0));
    crate::w7::seat_at(&mut sim, &names[1], crate::hearth::Seat::Garden(1));
    let note = crate::w7::group_lines(&sim, Group::Garden);
    point_at(&mut sim, Target::LetWinterPass, true);
    let winter = turning_lines(&sim, PageKind::Winter);
    let line = format!(
        "{} and {} are neither of the house. They may not wed under this roof.",
        names[0], names[1]
    );
    let house = sim.world().resource::<House>();
    checks.require(
        note.iter().any(|l| l == "neither is family")
            && winter.contains(&line)
            && house.heroes[first].bond_to(second).is_none(),
        "two outsiders are not refused the garden",
        format!("seed {seed:#x}: note {note:?}, winter {winter:?}"),
    );
    "XI two outsiders: \"neither is family\", and no wedding".to_owned()
}
