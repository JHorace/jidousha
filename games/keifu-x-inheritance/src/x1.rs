//! The variant's decision checks (VARIANT.md), played through the scripted pointer: the
//! three decision rows of the spec, each asserted from the transcript — what the player
//! could read before the commit, then what the commit did.
//!
//! **Row 1, a personal quest** (`check_personal`): Garrick dragged alone onto "Grave goods"
//! (his dream needs a triumph there, so it is personal to him) and pointed at — the card
//! and the sheet name the mark and what it costs — then "Set out" on a seed where it fails,
//! and the house carries exactly that mark. **Row 2, marrying in** (`check_marrying_in`): an
//! unproven outsider refused in the garden, the same outsider at the threshold accepted.
//! **Row 3, the heir** (`check_heir_inheritance`): the death page's candidate lines equal
//! what choosing does. `check_weighing` follows a mark through the turning to "Year 2
//! begins".
//!
//! INVARIANT: every expectation is a shipped literal, copied by hand from VARIANT.md and
//! the content — never computed by the code under test. `X1_SEED` and its outcome were
//! found by sweeping seeds 0..64 once, off-screen, and shipped.

use jidousha::prelude::*;

use crate::board::Slot;
use crate::checks::Checks;
use crate::hearth::{Group, Seat};
use crate::house::House;
use crate::ids::{Place, Trait};
use crate::marks::Mark;
use crate::screen::Target;
use crate::scripted::{Pointer, card_lines, center_of, drag, lines_in};
use crate::summer::SHEET;
use crate::telling_view::PANEL;
use crate::verify::{SEEDS, content_of, hero_named, page_of, point_at, session};
use crate::w6::telling_lines;
use crate::w7::{group_lines, seat_at, stay_home_into_winter, turning_lines};
use crate::w8::{go_to_the_choice, heir_labels, stage_garricks_winter};

/// A seed on which Garrick alone on "Grave goods" (demand 11, power 7) rolls dice 1 and 1:
/// a disaster. The sweep over 0..64 found it; Garrick lives.
pub const X1_SEED: u64 = 4;
/// The card's setback-or-worse (33 of 36) and disaster (15 of 36) at power 7 against 11.
pub const X1_CARD: [&str; 2] = ["Setback 92%", "disaster 42%"];
/// `ui.quest_card.personal` for Garrick.
pub const X1_MARKED_IF: &str = "Marked if it fails: Garrick";
/// `ui.quest_sheet.personal` at 33 of 36 failing (setback 18 + disaster 15 = 92 in 100).
pub const X1_PERSONAL: &str = "PERSONAL: Garrick's dream. If it fails (92 in 100) the name is marked: -2 house renown and -2 to Garrick now, then -1 a year to the house through year 8, and the blood carries it.";
/// `lines.mark.fallen` for Garrick at the Barrow.
pub const X1_FALLEN: &str =
    "Garrick fails at the Barrow, and the name is marked: -2 renown to the house, -2 to Garrick.";

fn mark(origin: usize, year: i32, generation: i32) -> Mark {
    Mark {
        origin,
        year,
        place: Place::Barrow,
        generation,
    }
}

/// Row 1: the mark's consequence is in the transcript before the commit; failing the quest
/// leaves the house carrying exactly that mark.
pub fn check_personal(checks: &mut Checks) -> String {
    let mut sim = session(X1_SEED);
    crate::w4::seat(&mut sim, "Garrick", Slot::Quest { quest: 0, seat: 0 });
    point_at(&mut sim, Target::Quest(0), false);
    let card = card_lines(&sim, 0);
    let sheet = lines_in(&page_of(&sim), SHEET);
    checks.require(
        card.iter().any(|l| l == X1_MARKED_IF)
            && X1_CARD.iter().all(|c| card.contains(&(*c).to_owned())),
        "a personal quest's card does not say who is marked if it fails, beside its odds",
        format!("seed {X1_SEED}: {card:?}"),
    );
    checks.require(
        sheet.iter().any(|l| l == X1_PERSONAL),
        "a personal quest's sheet does not give the chance it fails and what a failure costs, before the commit",
        format!("seed {X1_SEED}: {sheet:?}"),
    );
    let garrick = hero_named(&sim, "Garrick");
    let before = {
        let house = sim.world().resource::<House>();
        (
            house.renown,
            house.heroes[garrick].renown,
            house.marks_carried().len(),
        )
    };
    checks.require(
        before == (15, 6, 0),
        "before the commit the house is at 15, Garrick at 6, and no mark is carried",
        format!("{before:?}"),
    );
    point_at(&mut sim, Target::SetOut, true);
    point_at(&mut sim, Target::GoOn, true);
    let read = telling_lines(&sim);
    checks.require(
        read.iter().any(|l| l == X1_FALLEN),
        "the failed personal quest's page does not tell the name marked",
        format!("seed {X1_SEED}: {read:?}"),
    );
    let house = sim.world().resource::<House>();
    // 15, less the three unanswered (-3), the disaster's danger (-2) and the mark (-2).
    let after = (house.renown, house.heroes[garrick].renown);
    checks.require(
        after == (8, 4) && house.marks_carried() == [mark(garrick, 1, 0)],
        "after the failure the house is at 8, Garrick at 4, and the house carries exactly Garrick's mark of year 1 at the Barrow",
        format!("seed {X1_SEED}: {after:?}, {:?}", house.marks_carried()),
    );
    format!(
        "X1 row 1: Garrick alone on Grave goods, marked-if-it-fails on the card and the sheet before \"Set out\"; seed {X1_SEED} fails it (disaster), the house at 8 carrying one mark"
    )
}

/// A mark followed through the turning: Maren given Garrick's mark of year 1, the winter
/// let pass, and "Year 2 begins" says the name carries it.
pub fn check_weighing(checks: &mut Checks) -> String {
    let mut sim = session(SEEDS[0]);
    stay_home_into_winter(&mut sim);
    let maren = hero_named(&sim, "Maren");
    {
        let house = sim.world_mut().resource_mut::<House>();
        house.heroes[maren].marks.push(mark(maren, 1, 0));
    }
    point_at(&mut sim, Target::LetWinterPass, true);
    let year = year_page(&mut sim);
    checks.require(
        year.iter()
            .any(|l| l == "The name carries a mark: -1 renown."),
        "the turning's year page does not say the name carries a mark and what it costs",
        format!("{year:?}"),
    );
    let renown = sim.world().resource::<House>().renown;
    checks.require(
        renown == 10,
        "a year at home leaves the house at 11 and the mark takes it to 10",
        format!("{renown}"),
    );
    "X1 weighing: a mark on Maren costs the house a renown on \"Year 2 begins\" (11 to 10)"
        .to_owned()
}

/// The lines of the turning's year page, read leaf by leaf.
fn year_page(sim: &mut HeadlessSim) -> Vec<String> {
    let index = {
        let house = sim.world().resource::<House>();
        let Some(passage) = &house.passage else {
            return Vec::new();
        };
        passage
            .pages
            .iter()
            .position(|p| p.kind == crate::passage::PageKind::Year)
    };
    index
        .map(|i| crate::w8::page_leaves(sim, i).concat())
        .unwrap_or_default()
}

/// A wanderer arrived on the session's house, of Ysolde's age, with `renown`; returns them.
pub fn stage_outsider(sim: &mut HeadlessSim, renown: i32) -> usize {
    let content = match crate::content::load() {
        Ok(content) => content,
        Err(error) => crate::checks::fail("the content did not load", &error.to_string()),
    };
    let ysolde = hero_named(sim, "Ysolde");
    let mut rng = sim.world().resource::<Rng>().clone();
    let house = sim.world_mut().resource_mut::<House>();
    let _ = crate::wanderer::arrive(&content, house, &mut rng);
    let outsider = house.heroes.len() - 1;
    house.heroes[outsider].age = house.heroes[ysolde].age;
    house.heroes[outsider].renown = renown;
    house.open_hearth();
    sim.world_mut().insert_resource(rng);
    outsider
}

/// Drag hero `id` onto hearth seat `seat`.
pub fn seat_hero(sim: &mut HeadlessSim, id: usize, seat: Seat) {
    let from = center_of(sim, Target::Hero(id));
    let to = center_of(sim, Target::Seat(Slot::Hearth(seat)));
    drag(sim, &mut Pointer::mouse(), from, to);
}

/// Row 2: the garden refuses an outsider below the threshold, with their renown and the
/// threshold in the panel, and accepts one at it.
pub fn check_marrying_in(checks: &mut Checks) -> String {
    let mut wed_renown = [0, 0];
    for (index, renown) in [4, 5].into_iter().enumerate() {
        let mut sim = session(SEEDS[0]);
        stay_home_into_winter(&mut sim);
        let outsider = stage_outsider(&mut sim, renown);
        seat_at(&mut sim, "Ysolde", Seat::Garden(0));
        seat_hero(&mut sim, outsider, Seat::Garden(1));
        let name = sim.world().resource::<House>().heroes[outsider]
            .name
            .clone();
        let garden = group_lines(&sim, Group::Garden);
        let want = if renown == 4 {
            format!("{name} unproven: renown 4 of 5")
        } else {
            "will wed".to_owned()
        };
        checks.require(
            garden.contains(&want),
            "the garden does not read an outsider's renown against the threshold of 5, or \"will wed\" at it",
            format!("renown {renown}: {garden:?}"),
        );
        point_at(&mut sim, Target::LetWinterPass, true);
        let page = turning_lines(&sim);
        let (family, house_name, renown_after, married) = {
            let house = sim.world().resource::<House>();
            let hero = &house.heroes[outsider];
            let sheet = crate::sheet::hero_sheet(content_of(&sim), &house.heroes, outsider);
            let outsider_line = sheet
                .lines
                .iter()
                .any(|l| l.text.starts_with("An outsider"));
            (
                hero.family,
                hero.house.clone(),
                house.renown,
                (
                    outsider_line,
                    page.iter().any(|l| {
                        l.contains(
                            "takes the name of Vane, and the house is the richer by 2 renown.",
                        )
                    }),
                ),
            )
        };
        wed_renown[index] = renown_after;
        if renown == 4 {
            checks.require(
                !family && house_name != "Vane" && !married.1,
                "an unproven outsider is refused: still an outsider, no new name, no dowry",
                format!("{family} {house_name} {page:?}"),
            );
        } else {
            checks.require(
                family && house_name == "Vane" && married.1 && !married.0,
                "an outsider at the threshold marries in: family, the name Vane, the dowry told, no outsider line on their sheet",
                format!("{family} {house_name} {married:?} {page:?}"),
            );
        }
    }
    checks.require(
        wed_renown[1] == wed_renown[0] + 2,
        "marrying in brings the house the dowry: 2 beyond what the same winter gives without it",
        format!("{wed_renown:?}"),
    );
    "X1 row 2: an outsider at renown 4 refused (\"unproven: renown 4 of 5\"), at 5 \"will wed\", family and Vane after, the house +2".to_owned()
}

/// What the death page offers for Garrick marked, in order (`lines.heir.carries` and the
/// "No one" line), then the buttons mainline's W8 oracle lists.
pub const X1_HEIR_LINES: [&str; 7] = [
    "Maren, daughter: Sharp; marks 1 (+1)",
    "Pip, grandson: no trait; marks 1 (+1)",
    "Odo, friend: Steadfast; marks 1 (+1)",
    "Ysolde, of the house: no trait; marks 1 (+1)",
    "Brannoc, of the house: Strong; marks 1 (+1)",
    "Wren, of the house: no trait; marks 1 (+1)",
    "No one: the marks fall to Maren, blood of the name.",
];

/// Garrick marked and aged to his death, the winter let pass, the choice on the screen.
pub fn marked_choice(maren_dead: bool) -> HeadlessSim {
    let mut sim = session(SEEDS[0]);
    stage_garricks_winter(&mut sim);
    let (garrick, maren) = (hero_named(&sim, "Garrick"), hero_named(&sim, "Maren"));
    {
        let house = sim.world_mut().resource_mut::<House>();
        house.heroes[garrick].marks.push(mark(garrick, 1, 0));
        if maren_dead {
            house.heroes[maren].fate = crate::hero::Fate::Dead;
            house.unseat(maren);
        }
    }
    point_at(&mut sim, Target::LetWinterPass, true);
    go_to_the_choice(&mut sim);
    sim
}

/// The turning panel's lines now.
fn panel(sim: &HeadlessSim) -> Vec<String> {
    lines_in(&page_of(sim), PANEL)
}

/// Row 3: the candidate lines equal what choosing does.
pub fn check_heir_inheritance(checks: &mut Checks) -> String {
    let mut sim = marked_choice(false);
    let lines = panel(&sim);
    let at = lines.iter().position(|l| l == X1_HEIR_LINES[0]);
    checks.require(
        at.is_some_and(|i| lines[i..].starts_with(&X1_HEIR_LINES.map(str::to_owned))),
        "the death page does not show each candidate's traits and marks, in order, then where \"No one\" leaves them",
        format!("{lines:?}"),
    );
    let labels: Vec<String> = heir_labels(&page_of(&sim))
        .into_iter()
        .map(|(_, l)| l)
        .collect();
    checks.require(
        labels.len() == 7 && labels[0] == "Maren, daughter" && labels[6] == "No one. Let it lie.",
        "the choice's buttons are mainline's: six heirs and no one",
        format!("{labels:?}"),
    );
    let (garrick, maren) = (hero_named(&sim, "Garrick"), hero_named(&sim, "Maren"));
    let renown = sim.world().resource::<House>().heroes[maren].renown;
    let Some((target, _)) = heir_labels(&page_of(&sim)).into_iter().next() else {
        crate::checks::fail("the choice has no buttons", "heir_labels is empty");
    };
    point_at(&mut sim, target, true);
    let house = sim.world().resource::<House>();
    let hero = &house.heroes[maren];
    checks.require(
        hero.traits == [Trait::Sharp]
            && hero.marks == [mark(garrick, 1, 1)]
            && hero.renown == renown - 1
            && house.marks_carried() == [mark(garrick, 1, 1)],
        "choosing Maren gives her exactly what her line showed: Sharp, one mark, a renown paid",
        format!(
            "{:?} {:?} {} (was {renown})",
            hero.traits, hero.marks, hero.renown
        ),
    );
    let after = panel(&sim);
    checks.require(
        after
            .iter()
            .any(|l| l == "Maren takes the name's shame with the rest: a mark."),
        "the page does not tell Maren taking the shame",
        format!("{after:?}"),
    );
    // "No one": the blood carries it all the same.
    let mut sim = marked_choice(false);
    let Some((no_one, _)) = heir_labels(&page_of(&sim)).into_iter().last() else {
        crate::checks::fail("the choice has no buttons", "heir_labels is empty");
    };
    point_at(&mut sim, no_one, true);
    let house = sim.world().resource::<House>();
    checks.require(
        house.heroes[maren].marks == [mark(garrick, 1, 1)]
            && panel(&sim)
                .iter()
                .any(|l| l == "Garrick's mark falls to Maren, blood of the name."),
        "choosing no one leaves the marks with the blood of the name, and says so",
        format!("{:?}", house.heroes[maren].marks),
    );
    // With Maren dead there is no blood: the shame goes into the ground.
    let mut sim = marked_choice(true);
    let ground = panel(&sim)
        .iter()
        .any(|l| l == "No one: the marks go into the ground.");
    let Some((no_one, _)) = heir_labels(&page_of(&sim)).into_iter().last() else {
        crate::checks::fail("the choice has no buttons", "heir_labels is empty");
    };
    point_at(&mut sim, no_one, true);
    let house = sim.world().resource::<House>();
    checks.require(
        ground
            && house.marks_carried().is_empty()
            && panel(&sim)
                .iter()
                .any(|l| l == "Garrick's shame goes into the ground with him."),
        "with no blood living the marks go into the ground with the dead, said before and after",
        format!("{ground} {:?}", house.marks_carried()),
    );
    "X1 row 3: Garrick's death page shows each candidate's traits and marks and where \"No one\" leaves them; Maren's choice, no one, and the ground give exactly that".to_owned()
}
