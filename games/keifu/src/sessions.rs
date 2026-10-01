//! Sessions that drive the screens and the run's seed: the family screen opened and
//! closed by its buttons, and "begin another house" reseeding from the generator.
//!
//! INVARIANT: as in `oracles.rs`, every expectation is a shipped literal.

use jidousha::prelude::*;
use jidousha::testing::FrameRecorder;

use crate::checks::Checks;
use crate::content::Content;
use crate::house::{House, begin_another_house};
use crate::oracles::{find_from, lines_of};
use crate::screen::{Target, UiState};
use crate::verify::{content_of, hero_named, page_of, point_at, session};

/// The family screen: opened by its button, nine nodes, the tally, and closed again.
pub fn check_family(checks: &mut Checks, recorder: &mut FrameRecorder) -> String {
    let mut sim = session(crate::verify::SEEDS[0]);
    point_at(&mut sim, Target::OpenFamily, true);
    let open = sim.world().resource::<UiState>().family_open;
    checks.require(
        open,
        "clicking \"The family\" did not open the family",
        String::new(),
    );
    let page = page_of(&sim);
    let nodes = page
        .targets
        .iter()
        .filter(|(_, t)| matches!(t, Target::Hero(_)))
        .count();
    checks.require(
        nodes == 9,
        "the family does not show all nine who have lived",
        format!("{nodes} nodes"),
    );
    let tally = "Year 1. 7 living, 2 gone. No tale of the house is told yet. House renown 15.";
    checks.require(
        find_from(&page, 0, tally).is_some(),
        "the family tally is wrong",
        format!(
            "wanted {tally:?}; rows {:?}",
            lines_of(&page)
                .into_iter()
                .filter(|l| l.starts_with("Year"))
                .collect::<Vec<_>>()
        ),
    );
    // Spouse links: two (Brannoc-Aud, Garrick-Elsbeth); parent links: three segments a child.
    checks.require(
        page.links.len() == 2 + 3 * 3,
        "the tree's links are wrong",
        format!("{} links", page.links.len()),
    );
    let garrick = hero_named(&sim, "Garrick");
    point_at(&mut sim, Target::Hero(garrick), false);
    let page = page_of(&sim);
    let remembered = "Knight, elder, aged 62. Living. He wants to lay the Barrow's dead to rest. Still to do: win a triumph at the Barrow. He fears deep water. The Seer said: \"Your child will surpass you.\"";
    let lines = lines_of(&page);
    let shown = lines
        .iter()
        .skip_while(|l| *l != "Garrick Thorne")
        .nth(1)
        .cloned()
        .unwrap_or_default();
    checks.require(
        shown == remembered,
        "Garrick's remembrance on the family screen is wrong",
        format!("wanted {remembered:?}, shown {shown:?}"),
    );
    let maren = hero_named(&sim, "Maren");
    point_at(&mut sim, Target::Hero(maren), false);
    let lines = lines_of(&page_of(&sim));
    let shown = lines
        .iter()
        .skip_while(|l| *l != "Maren Thorne")
        .nth(1)
        .cloned()
        .unwrap_or_default();
    let remembered = "Ranger, prime, aged 38. Living. She wants to avenge her mother. Still to do: return to the Drowned Coast. She fears deep water. The Seer said: \"You will break and be mended.\"";
    checks.require(
        shown == remembered,
        "Maren's remembrance is wrong",
        format!("wanted {remembered:?}, shown {shown:?}"),
    );
    let _ = recorder.draw(&mut sim);
    point_at(&mut sim, Target::CloseFamily, true);
    let closed = !sim.world().resource::<UiState>().family_open;
    checks.require(
        closed,
        "\"Back to the house\" did not close the family",
        String::new(),
    );
    format!(
        "family: {nodes} nodes, {} links, opened and closed by its buttons",
        11
    )
}

/// Seeds are recorded state; the household does not depend on them; a new house reseeds.
pub fn check_seeds(checks: &mut Checks, content: &Content) -> String {
    let mut a = session(7);
    let b = session(7);
    let c = session(8);
    let seed = |s: &HeadlessSim| s.world().resource::<House>().seed;
    checks.require(
        seed(&a) == 7 && seed(&b) == 7 && seed(&c) == 8,
        "the run seed is not the recorded one",
        format!("{} {} {}", seed(&a), seed(&b), seed(&c)),
    );
    let before = crate::family::top_bar(content_of(&a), a.world().resource::<House>());
    let drawn = match begin_another_house(a.world_mut(), content) {
        Ok(new) => new,
        Err(error) => {
            checks.require(false, "begin another house failed", error);
            return "seeds: begin another house failed".to_owned();
        }
    };
    let after = crate::family::top_bar(content, a.world().resource::<House>());
    checks.require(
        drawn != 7 && seed(&a) == drawn && before == after,
        "another house did not reseed from the generator, or its founding differs",
        format!("new seed {drawn:#x}, top bar {before:?} -> {after:?}"),
    );
    format!(
        "seeds: recorded on the house; another house drew {drawn:#x} and founded the same household"
    )
}

/// The sheet states no founding hero is in, staged on a copy of the house and read
/// back as the sheet's lines: wounded, settled, every fear state, a Door heir's
/// prophecy, a carried and a fulfilled dream, the dead, and more than six bonds.
pub fn check_staged_sheets(checks: &mut Checks) -> String {
    let sim = session(crate::verify::SEEDS[0]);
    let content = content_of(&sim);
    let house = sim.world().resource::<House>();
    let by = |name: &str| {
        house
            .heroes
            .iter()
            .position(|h| h.name == name)
            .unwrap_or(0)
    };
    let (garrick, maren, pip, brannoc, elsbeth, wren) = (
        by("Garrick"),
        by("Maren"),
        by("Pip"),
        by("Brannoc"),
        by("Elsbeth"),
        by("Wren"),
    );
    let lines = |heroes: &[crate::hero::Hero], id: usize| -> Vec<String> {
        crate::sheet::hero_sheet(content, heroes, id)
            .lines
            .into_iter()
            .map(|line| line.text)
            .collect()
    };
    let mut staged = 0;
    let mut expect = |checks: &mut Checks,
                      what: &str,
                      heroes: &[crate::hero::Hero],
                      id: usize,
                      want: &[&str]| {
        staged += 1;
        let have = lines(heroes, id);
        let missing: Vec<&&str> = want
            .iter()
            .filter(|w| !have.iter().any(|h| h == **w))
            .collect();
        checks.require(
            missing.is_empty(),
            "a staged sheet is missing a line",
            format!("{what}: missing {missing:?}; the sheet reads {have:?}"),
        );
    };
    let mut heroes = house.heroes.clone();
    heroes[garrick].wounded = true;
    expect(
        checks,
        "Garrick wounded",
        &heroes,
        garrick,
        &[
            "Renown 6. Wounded: -2 power.",
            "Another wound will kill. Rest to heal.",
        ],
    );
    heroes[garrick].wounded = false;
    heroes[garrick].settled = true;
    expect(
        checks,
        "Garrick settled",
        &heroes,
        garrick,
        &["Renown 6. Settled.", "Settled. Dread has no hold."],
    );
    heroes[garrick].fear.conquered = true;
    expect(
        checks,
        "Garrick conquered",
        &heroes,
        garrick,
        &["FEAR, CONQUERED", "A strength now: +2 power against Water."],
    );
    heroes[garrick].fear.conquered = false;
    heroes[garrick].fear.broken = true;
    expect(
        checks,
        "Garrick broken",
        &heroes,
        garrick,
        &[
            "FEAR, BROKEN",
            "Will not go against Water, ever. Children born after are likelier to carry it.",
        ],
    );
    heroes[garrick].fear.born_brave = true;
    expect(
        checks,
        "Garrick born brave",
        &heroes,
        garrick,
        &["BORN BRAVE"],
    );
    heroes[maren].destiny.kind = crate::ids::Destiny::OpenTheSealedDoor;
    heroes[maren].destiny.blood_of = Some("Ysolde".to_owned());
    expect(
        checks,
        "Maren of Door blood",
        &heroes,
        maren,
        &[
            "Blood of Ysolde: you will open the Sealed Door.",
            "DESTINY",
            "Gift: +5 at every lock of the Door.",
        ],
    );
    if let Some(dream) = heroes[maren].dream.as_mut() {
        dream.advance_to_stage(3);
    }
    expect(checks, "Maren fulfilled", &heroes, maren, &["Fulfilled."]);
    let mut carried = house.heroes[elsbeth].dream.clone();
    if let Some(dream) = carried.as_mut() {
        dream.owner = Some(elsbeth);
    }
    heroes[pip].burden = carried;
    expect(
        checks,
        "Pip carrying Elsbeth's dream",
        &heroes,
        pip,
        &[
            "BURDEN",
            "to see the sea",
            "Taken up from Elsbeth Thorne.",
            "Succeed against Water",
            "If it is ever done, it will leave a tale, worth 1 renown a year for ever.",
        ],
    );
    expect(
        checks,
        "Brannoc's heirloom promise",
        &heroes,
        brannoc,
        &["If it is ever done, it will leave an heirloom worth +2."],
    );
    expect(
        checks,
        "Elsbeth, dead",
        &heroes,
        elsbeth,
        &[
            "Ranger, veteran, aged 41",
            "Died before the first year, aged 41",
            "Wits 7 (+1)",
            "Wits 6 of 9, +1 for being a veteran.",
            "Husband Garrick +2",
            "Daughter Maren +2",
        ],
    );
    // Eight bonds for Wren, all shown kinds: six shown, "and 2 more".
    for other in [garrick, maren, pip, elsbeth, by("Ysolde"), by("Odo")] {
        heroes[wren].bonds.push(crate::hero::Bond {
            kind: crate::ids::BondKind::Friend,
            other,
            since: 0,
            taught: false,
            shared_successes: 0,
        });
    }
    expect(
        checks,
        "Wren with eight bonds",
        &heroes,
        wren,
        &["and 2 more", "Father Brannoc +2"],
    );
    // A sibling for Wren: kin by a shared parent, and nothing else makes kin.
    let mut sibling = heroes[wren].clone();
    sibling.bonds.clear();
    heroes.push(sibling);
    let twin = heroes.len() - 1;
    checks.require(
        crate::hero::kin(&heroes, wren, twin) && !crate::hero::kin(&heroes, wren, garrick),
        "kin by a shared parent is wrong",
        "Wren and a child of Brannoc and Aud; Wren and her friend Garrick".to_owned(),
    );
    format!("staged sheets: {staged} states no founding hero is in")
}
