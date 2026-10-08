//! The variant's staging (DESIGN.md "Gates to add"): the sessions `xi.rs`'s checks, the
//! floors and the pictures share — a sworn card, an outsider in the garden, Garrick's
//! death page with marks and a blessing to pass on — and the three surfaces judged by
//! `floors.rs`'s `look` at each of its sizes.

use jidousha::prelude::{HeadlessSim, Rng};
use jidousha::testing::FrameRecorder;

use crate::checks::{Checks, fail};
use crate::content::Content;
use crate::hearth::{Group, Seat};
use crate::hero::{Blessing, HeroId, Scope};
use crate::house::House;
use crate::ids::Pronoun;
use crate::ids::{Place, Tag};
use crate::marks::Mark;
use crate::passage::PageKind;
use crate::screen::Target;
use crate::screen::WINDOW;
use crate::scripted::lines_in;
use crate::verify::{SEEDS, hero_named, point_at, session};
use crate::verify::{dock_pages, dock_read, page_of, target_rect};

/// The content, loaded as the game loads it.
pub fn content() -> Content {
    match crate::content::load() {
        Ok(content) => content,
        Err(error) => fail("the content did not load", &error.to_string()),
    }
}

/// W4's oracle seated (Garrick and Brannoc dragged onto "Grave goods"), its demand
/// staged to 10 as W4's tests stage it: power 12 against 10.
pub fn oracle_at_ten(sim: &mut HeadlessSim) {
    let _ = crate::w4::seat_the_oracle(sim);
    sim.world_mut().resource_mut::<House>().board[0]
        .quest
        .demand = 10;
}

/// The oracle at ten, Garrick's oath sworn by its button, the pointer on the card.
pub fn sworn(sim: &mut HeadlessSim) {
    oracle_at_ten(sim);
    point_at(sim, Target::Swear(0), true);
    point_at(sim, Target::Quest(0), false);
}

/// A wanderer arrived on the session's generator (as `w8::stir` does), aged `age`
/// with `renown`; the household reseated for the hearth when it is winter. Returns them.
pub fn wanderer(sim: &mut HeadlessSim, age: i32, renown: i32) -> HeroId {
    let content = content();
    let mut rng = sim.world().resource::<Rng>().clone();
    let house = sim.world_mut().resource_mut::<House>();
    let _ = crate::wanderer::arrive(&content, house, &mut rng);
    let id = house.heroes.len() - 1;
    house.heroes[id].age = age;
    house.heroes[id].renown = renown;
    if house.calendar.is_winter() {
        house.open_hearth();
    }
    sim.world_mut().insert_resource(rng);
    id
}

/// Year 1 left at home into the winter; a wanderer of 24 with `renown` in the first
/// garden seat and Ysolde in the second. Returns the wanderer.
pub fn garden(sim: &mut HeadlessSim, renown: i32) -> HeroId {
    crate::w7::stay_home_into_winter(sim);
    let wanderer = wanderer(sim, 24, renown);
    let name = sim.world().resource::<House>().heroes[wanderer]
        .name
        .clone();
    crate::w7::seat_at(sim, &name, Seat::Garden(0));
    crate::w7::seat_at(sim, "Ysolde", Seat::Garden(1));
    wanderer
}

/// "Grave goods" at the Barrow, year 1, Garrick's, at `weight`.
pub fn grave_goods(garrick: HeroId, weight: i32) -> Mark {
    Mark {
        title: "Grave goods".to_owned(),
        place: Place::Barrow,
        year: 1,
        by: garrick,
        weight,
    }
}

/// Garrick's winter (`w8::stage_garricks_winter`), and on him two marks — "Grave goods"
/// of 4, "The bell under the tide" of 1 — and the blessing "Garrick's rest".
pub fn marked_garrick(sim: &mut HeadlessSim) -> HeroId {
    crate::w8::stage_garricks_winter(sim);
    let garrick = hero_named(sim, "Garrick");
    let hero = &mut sim.world_mut().resource_mut::<House>().heroes[garrick];
    hero.marks = vec![
        grave_goods(garrick, 4),
        Mark {
            title: "The bell under the tide".to_owned(),
            place: Place::DrownedCoast,
            year: 1,
            by: garrick,
            weight: 1,
        },
    ];
    hero.blessings = vec![Blessing {
        title: "Garrick's rest".to_owned(),
        scope: Scope::AgainstTag(Tag::Undead),
        power: 2,
    }];
    garrick
}

/// The marked Garrick dead, his page offering heirs, the pointer on Maren's button.
pub fn succession(sim: &mut HeadlessSim) {
    let _ = marked_garrick(sim);
    point_at(sim, Target::LetWinterPass, true);
    crate::w8::go_to_the_choice(sim);
    let maren = hero_named(sim, "Maren");
    point_at(sim, Target::Heir(1, Some(maren)), false);
}

/// A picture's stage, by name: "sworn", "garden" (an unproven outsider, the pointer
/// on the garden), "succession".
pub fn stage(sim: &mut HeadlessSim, name: &str) {
    match name {
        "sworn" => sworn(sim),
        "garden" => {
            let _ = garden(sim, 2);
            crate::w7_controls::point_at_group(sim, Group::Garden);
        }
        "succession" => succession(sim),
        other => fail(
            "a picture names a stage the variant does not have",
            &format!("xi:{other}"),
        ),
    }
}

/// The variant's three surfaces, each judged. Returns how many.
pub fn xi_surfaces(
    checks: &mut Checks,
    tally: &mut crate::floors::Tally,
    recorder: &mut FrameRecorder,
    _label: &str,
) -> usize {
    let mut judged = 0;
    for (name, title) in [
        ("sworn", "XI, the sworn card with its sheet"),
        ("garden", "XI, an unproven outsider in the garden"),
        (
            "succession",
            "XI, Garrick's page with what Maren would inherit",
        ),
    ] {
        let mut sim = session(SEEDS[0]);
        stage(&mut sim, name);
        crate::floors::look(checks, tally, recorder, &mut sim, title, false);
        judged += 1;
    }
    judged
}

/// The text of the oath button on board slot `slot`'s card, if it is there.
pub fn button(sim: &HeadlessSim, slot: usize) -> Option<String> {
    let rect = target_rect(sim, Target::Swear(slot))?;
    Some(lines_in(&page_of(sim), rect).join(" "))
}

/// Resolve board slot 0 with `dice`, on the session's house and generator. Returns the
/// page's lines.
pub fn resolve(sim: &mut HeadlessSim, dice: [i32; 2]) -> Vec<String> {
    let content = content();
    let mut rng = sim.world().resource::<Rng>().clone();
    let house = sim.world_mut().resource_mut::<House>();
    let page = crate::resolve::resolve_rolled(&content, house, &mut rng, 0, dice);
    sim.world_mut().insert_resource(rng);
    page.lines
}

/// Every line of the open turning's pages of `kind`.
pub fn turning_lines(sim: &HeadlessSim, kind: PageKind) -> Vec<String> {
    let house = sim.world().resource::<House>();
    house
        .passage
        .iter()
        .flat_map(|p| p.pages.iter())
        .filter(|p| p.kind == kind)
        .flat_map(|p| p.lines.iter().cloned())
        .collect()
}

/// The sheet in the dock, every line, paged through.
pub fn dock_lines(sim: &mut HeadlessSim) -> Vec<String> {
    let mut recorder = FrameRecorder::new(WINDOW);
    let pages = dock_pages(sim, &mut recorder);
    dock_read(&pages)
        .into_iter()
        .map(|line| line.text)
        .collect()
}

/// "him" or "her".
pub fn object(pronoun: Pronoun) -> &'static str {
    match pronoun {
        Pronoun::He => "him",
        Pronoun::She => "her",
    }
}
