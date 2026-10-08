//! The variant's checks (DESIGN.md "Gates to add"), row 3 of the task's decision table —
//! the heir on the death page — and the rules beside it: an outsider's death, family
//! renown and the yearly drain.
//!
//! INVARIANT: every expectation here is a shipped literal, copied by hand from the
//! content, CONSTANTS and DESIGN.md — never computed by the code under test. Where a
//! line names a rolled wanderer, only the name is read off the house.

use crate::checks::Checks;
use crate::hero::HeroId;
use crate::house::House;
use crate::marks::{Mark, house_marks};
use crate::passage::PageKind;
use crate::screen::Target;
use crate::verify::{hero_named, point_at, session};
use crate::w5::recorded;
use crate::xi_stages::{
    content, dock_lines, grave_goods, object, resolve, turning_lines, wanderer,
};

/// What Maren would inherit, the WOULD INHERIT section in the dock (DESIGN.md row 3).
pub const WOULD_INHERIT: [&str; 6] = [
    "WOULD INHERIT",
    "Thornfall: +1 Might on quests.",
    "The dream: to lay the Barrow's dead to rest",
    "Garrick's rest: +2 against Undead",
    "A mark of 2: Grave goods, year 1",
    "A mark struck: The bell under the tide, year 1",
];

/// Decision row 3: the heir's preview equals what they hold after the choice.
pub fn check_succession(checks: &mut Checks) -> (String, Vec<String>) {
    let mut vector = Vec::new();
    for seed in recorded() {
        let mut sim = session(seed);
        crate::xi_stages::succession(&mut sim);
        let (garrick, maren) = (hero_named(&sim, "Garrick"), hero_named(&sim, "Maren"));
        let dock = dock_lines(&mut sim);
        let tail = dock[dock.len().saturating_sub(WOULD_INHERIT.len())..].to_vec();
        let page = turning_lines(&sim, PageKind::Death);
        let leaves =
            "He leaves marks on the name: Grave goods (4) and The bell under the tide (1).";
        let shown = crate::inheritance::succession(
            &content(),
            &sim.world().resource::<House>().heroes,
            garrick,
            maren,
        );
        checks.require(
            tail == WOULD_INHERIT && page.contains(&leaves.to_owned()),
            "pointing at Maren's heir button does not show what she would inherit, or the page does not name Garrick's marks",
            format!("seed {seed:#x}: dock {dock:?}; page {page:?}"),
        );
        point_at(&mut sim, Target::Heir(1, Some(maren)), true);
        let house = sim.world().resource::<House>();
        let her = &house.heroes[maren];
        let page = turning_lines(&sim, PageKind::Death);
        // Maren has her own dream, so Garrick's undone one becomes her burden.
        let barrow = her.burden.as_ref().map(|d| d.kind)
            == house.heroes[garrick].dream.as_ref().map(|d| d.kind)
            && her.dream.is_some();
        let marks: Vec<(HeroId, Mark)> = house_marks(house)
            .into_iter()
            .map(|(id, mark)| (id, mark.clone()))
            .collect();
        let read_back = (
            her.heirloom.as_ref().map(|h| h.name.as_str()),
            her.blessings.iter().any(|b| b.title == "Garrick's rest"),
            her.marks.clone(),
        );
        checks.require(
            shown.heirloom.as_deref() == read_back.0
                && read_back.0 == Some("Thornfall")
                && shown.dream.is_some()
                && barrow
                && shown.blessings == ["Garrick's rest"]
                && read_back.1
                && read_back.2 == [grave_goods(garrick, 2)]
                && marks == [(maren, grave_goods(garrick, 2))]
                && [
                    "Maren takes up Garrick's rest.",
                    "Maren takes up the marks: Grave goods (2).",
                    "The mark of The bell under the tide is struck from the name.",
                ]
                .iter()
                .all(|l| page.contains(&(*l).to_owned())),
            "what Maren was shown she would inherit is not what she holds after she is chosen",
            format!("seed {seed:#x}: shown {shown:?}; holds {read_back:?}; name's marks {marks:?}; page {page:?}"),
        );
        // No one: the marks go into the ground.
        let mut sim = session(seed);
        crate::xi_stages::succession(&mut sim);
        point_at(&mut sim, Target::Heir(1, None), true);
        let house = sim.world().resource::<House>();
        let page = turning_lines(&sim, PageKind::Death);
        let buried = "The marks go into the ground with him: the name is lighter by 2.";
        checks.require(
            house_marks(house).is_empty()
                && house.heroes[maren].marks.is_empty()
                && page.contains(&buried.to_owned()),
            "choosing no one does not bury Garrick's marks",
            format!("seed {seed:#x}: {page:?}"),
        );
        if seed == recorded()[0] {
            vector.push(format!("XI vector, Maren's WOULD INHERIT: {tail:?}"));
        }
    }
    (
        format!(
            "XI row 3, succession: the dock's WOULD INHERIT for Maren equals what she holds once chosen — Thornfall, the dream, Garrick's rest, Grave goods at 2, the bell struck — and no one buries the marks; on {} recorded seeds",
            recorded().len()
        ),
        vector,
    )
}

/// An outsider's death page waits for no heir: the heirloom is buried, the dream walks.
pub fn check_outsider_death(checks: &mut Checks) -> String {
    let seed = recorded()[0];
    let mut sim = session(seed);
    crate::w7::stay_home_into_winter(&mut sim);
    let outsider = wanderer(&mut sim, 93, 0);
    let content = content();
    {
        let house = sim.world_mut().resource_mut::<House>();
        house.heroes[outsider].heirloom = Some(crate::hero::Heirloom {
            name: "Emberwake".to_owned(),
            sprite: "reward-sword".to_owned(),
            aptitude: crate::ids::Aptitude::Might,
            bonus: 2,
            provenance: "Forged in year 1.".to_owned(),
        });
        house.heroes[outsider].dream =
            crate::dream::Dream::build(&content, crate::ids::DreamKind::SeeTheSea, None, None).ok();
    }
    let before = house_marks(sim.world().resource::<House>()).len();
    point_at(&mut sim, Target::LetWinterPass, true);
    let house = sim.world().resource::<House>();
    let page = house
        .passage
        .iter()
        .flat_map(|p| p.pages.iter())
        .find(|p| p.bequest.as_ref().is_some_and(|b| b.dead == outsider));
    let object = object(house.heroes[outsider].pronoun);
    let buried = format!("Emberwake was laid in the ground with {object}.");
    let ok = page.is_some_and(|p| {
        p.bequest
            .as_ref()
            .is_some_and(|b| !b.leaves && b.heirs.is_empty() && !b.undecided())
            && p.lines.contains(&buried)
    }) && house.ghosts.iter().any(|g| g.hero == outsider)
        && house_marks(house).len() == before;
    checks.require(
        ok,
        "an outsider's death page gathers heirs, or does not bury the heirloom and raise the dream's ghost",
        format!("seed {seed:#x}: {:?}", page.map(|p| &p.lines)),
    );
    "XI an outsider's death: decided on making, the heirloom buried, the ghost raised".to_owned()
}

/// Only the family earns for the name: an outsider alone wins renown for themselves.
pub fn check_family_renown(checks: &mut Checks) -> String {
    let seed = recorded()[0];
    let won = |with_garrick: bool| {
        let mut sim = session(seed);
        let outsider = wanderer(&mut sim, 30, 0);
        let garrick = hero_named(&sim, "Garrick");
        {
            let house = sim.world_mut().resource_mut::<House>();
            house.reseat();
            house.unseat(outsider);
            if with_garrick {
                house.unseat(garrick);
            }
            house.board[0].seats = vec![Some(outsider), with_garrick.then_some(garrick)];
            house.board[0].quest.demand = 0;
        }
        let before = sim.world().resource::<House>().renown;
        let lines = resolve(&mut sim, [6, 6]);
        let house = sim.world().resource::<House>();
        (house.renown - before, house.heroes[outsider].renown, lines)
    };
    let (alone, outsider, lines) = won(false);
    let line = "+3 renown to each who went. Outsiders bring the house nothing.";
    let (beside, _, _) = won(true);
    checks.require(
        alone == 0 && outsider == 3 && lines.contains(&line.to_owned()) && beside > 0,
        "an outsider's win credits the house, or a family member's does not",
        format!("seed {seed:#x}: alone {alone:+} (outsider {outsider}), beside Garrick {beside:+}; {lines:?}"),
    );
    "XI family renown: an outsider alone wins 3 for themselves and nothing for the house; beside Garrick the house gains".to_owned()
}

/// The name's marks drain the house at the turning: two on a living family member, one
/// on the dead (which does not count).
pub fn check_drain(checks: &mut Checks) -> String {
    let seed = recorded()[0];
    let turned = |marked: bool| {
        let mut sim = session(seed);
        crate::w7::stay_home_into_winter(&mut sim);
        if marked {
            let (maren, elsbeth) = (hero_named(&sim, "Maren"), hero_named(&sim, "Elsbeth"));
            let house = sim.world_mut().resource_mut::<House>();
            house.heroes[maren].marks = vec![grave_goods(maren, 2), grave_goods(maren, 1)];
            house.heroes[elsbeth].marks = vec![grave_goods(elsbeth, 3)];
        }
        point_at(&mut sim, Target::LetWinterPass, true);
        let renown = sim.world().resource::<House>().renown;
        (renown, turning_lines(&sim, PageKind::Year))
    };
    let (marked, year) = turned(true);
    let (clean, _) = turned(false);
    let line = "The name carries 2 marks: -2 renown.";
    checks.require(
        year.contains(&line.to_owned()) && clean - marked == 2,
        "the name's marks do not drain the house 1 a mark at the turning",
        format!("seed {seed:#x}: {year:?}; renown {marked} against {clean}"),
    );
    "XI the drain: two marks on the living name, one on the dead — \"The name carries 2 marks: -2 renown.\"".to_owned()
}
