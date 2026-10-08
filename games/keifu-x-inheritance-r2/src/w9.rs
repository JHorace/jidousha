//! W9's oracle, read off the family screen: Elsbeth's year-1 epitaph.
//!
//! **The oracle** (MODULES.md W9), on every recorded seed: open the family screen in year 1
//! and point at Elsbeth. The remembrance panel names her, then holds her epitaph, which is
//! exactly her specified sentences in one of the three frame orders — so it is asserted to
//! be one of the six shipped strings below (three frames, each with the two-sentence fear
//! wording and without her roads, or the one-sentence wording and with them). Then, part by
//! part: the first sentence names "Elsbeth Thorne"; she was lost at the Drowned Coast before
//! the first year, aged 41; she wanted to see the sea and died before it was done; she loved
//! Garrick Thorne and Maren; one of the two fear-of-high-places wordings; "went out eleven
//! times in all" only beside the one-sentence fear; and nothing about where she came from.
//!
//! INVARIANT: every expectation is a shipped literal copied by hand from MODULES.md and
//! `content/epitaph.json` — never computed by the code under test.

use jidousha::prelude::HeadlessSim;

use crate::checks::Checks;
use crate::house::House;
use crate::passage::PageKind;
use crate::screen::Target;
use crate::scripted::lines_in;
use crate::tree::REMEMBRANCE;
use crate::verify::{hero_named, page_of, point_at, session};
use crate::w5::recorded;

/// MODULES.md W9: each of Elsbeth's sentences, as `epitaph.json` words them for her.
pub const LOST: &str = "She was lost at the Drowned Coast, before the first year. She was 41.";
pub const SEA: &str = "She wanted to see the sea, and died before it was done.";
pub const LOVED: &str = "She loved Garrick Thorne, and Maren.";
pub const FEAR_TWO: &str =
    "She never stopped being afraid of high places. She went twice against it all the same.";
pub const FEAR_ONE: &str = "She was afraid of high places all her life, and went anyway.";
pub const ROADS: &str = "went out eleven times in all";

/// Her whole epitaph in each frame order: (frame, one-sentence fear, the text).
pub const ORACLE_EPITAPHS: [(usize, bool, &str); 6] = [
    (
        0,
        false,
        "Elsbeth Thorne never stopped being afraid of high places. She went twice against it all \
         the same. She wanted to see the sea, and died before it was done. She loved Garrick \
         Thorne, and Maren. She was lost at the Drowned Coast, before the first year. She was 41.",
    ),
    (
        0,
        true,
        "Elsbeth Thorne went out eleven times in all. She was afraid of high places all her life, \
         and went anyway. She wanted to see the sea, and died before it was done. She loved \
         Garrick Thorne, and Maren. She was lost at the Drowned Coast, before the first year. \
         She was 41.",
    ),
    (
        1,
        false,
        "Elsbeth Thorne was lost at the Drowned Coast, before the first year. She was 41. She \
         wanted to see the sea, and died before it was done. She never stopped being afraid of \
         high places. She went twice against it all the same. She loved Garrick Thorne, and \
         Maren.",
    ),
    (
        1,
        true,
        "Elsbeth Thorne was lost at the Drowned Coast, before the first year. She was 41. She \
         went out eleven times in all. She wanted to see the sea, and died before it was done. \
         She was afraid of high places all her life, and went anyway. She loved Garrick Thorne, \
         and Maren.",
    ),
    (
        2,
        false,
        "Elsbeth Thorne wanted to see the sea, and died before it was done. She never stopped \
         being afraid of high places. She went twice against it all the same. She loved Garrick \
         Thorne, and Maren. She was lost at the Drowned Coast, before the first year. She was 41.",
    ),
    (
        2,
        true,
        "Elsbeth Thorne wanted to see the sea, and died before it was done. She went out eleven \
         times in all. She was afraid of high places all her life, and went anyway. She loved \
         Garrick Thorne, and Maren. She was lost at the Drowned Coast, before the first year. \
         She was 41.",
    ),
];

/// Every ORIGIN wording `epitaph.json` has, in the words that would show it.
const ORIGINS: [&str; 6] = [
    "was not born here",
    "came up the road",
    "was born",
    "was the child of",
    "counting of years",
    "before anyone counted",
];

/// The remembrance panel's lines with `name` pointed at, on the family screen in year 1.
pub fn remembered(seed: u64, name: &str) -> Vec<String> {
    let mut sim = session(seed);
    point_at(&mut sim, Target::OpenFamily, true);
    let hero = hero_named(&sim, name);
    point_at(&mut sim, Target::Hero(hero), false);
    lines_in(&page_of(&sim), REMEMBRANCE)
}

/// One epitaph held to the oracle's sentences part by part; returns what is wrong.
fn by_part(epitaph: &str) -> Vec<&'static str> {
    let mut wrong = Vec::new();
    // Every sentence after the first keeps "She"; the first names her.
    let named =
        epitaph.starts_with("Elsbeth Thorne ") && epitaph.matches("Elsbeth Thorne").count() == 1;
    if !named {
        wrong.push("the first sentence does not name Elsbeth Thorne");
    }
    let unnamed = epitaph.replacen("Elsbeth Thorne ", "She ", 1);
    for (sentence, what) in [
        (LOST, "lost before the first year"),
        (SEA, "the sea"),
        (LOVED, "love"),
    ] {
        if !unnamed.contains(sentence) {
            wrong.push(what);
        }
    }
    let (two, one) = (unnamed.contains(FEAR_TWO), unnamed.contains(FEAR_ONE));
    if two == one {
        wrong.push("not exactly one of the two fear wordings");
    }
    if unnamed.contains(ROADS) != one {
        wrong.push(
            "\"went out eleven times in all\" without the one-sentence fear, or missing beside it",
        );
    }
    if ORIGINS.iter().any(|o| unnamed.contains(o)) {
        wrong.push("something about where she came from");
    }
    if unnamed.matches('.').count() != 6 {
        wrong.push("not six sentences");
    }
    wrong
}

/// The oracle on every recorded seed. Returns the summary and the vector.
pub fn check_oracle(checks: &mut Checks) -> (String, Vec<String>) {
    let mut vector = Vec::new();
    let mut frames = [0usize; 3];
    let mut one_sentence = 0;
    for seed in recorded() {
        let lines = remembered(seed, "Elsbeth");
        let epitaph = lines.get(1).cloned().unwrap_or_default();
        let found = ORACLE_EPITAPHS.iter().find(|(_, _, text)| *text == epitaph);
        checks.require(
            lines.len() == 2 && lines[0] == "Elsbeth Thorne" && found.is_some(),
            "Elsbeth's remembrance is not her name and one of her six epitaphs (three frame orders, two fear wordings)",
            format!("seed {seed:#x}: {lines:?}"),
        );
        let wrong = by_part(&epitaph);
        checks.require(
            wrong.is_empty(),
            "Elsbeth's epitaph breaks a sentence of the oracle",
            format!("seed {seed:#x}: {wrong:?} in {epitaph:?}"),
        );
        if let Some((frame, one, _)) = found {
            frames[*frame] += 1;
            one_sentence += usize::from(*one);
        }
        if seed == recorded()[0] {
            vector.push(format!(
                "W9 vector, Elsbeth's remembrance (seed {seed:#x}): {lines:?}"
            ));
        }
    }
    (
        format!(
            "W9 oracle: Elsbeth's year-1 remembrance is her name and exactly her sentences — lost at the Drowned Coast before the first year aged 41, the sea undone, Garrick Thorne and Maren, one of the two fears of high places, \"went out eleven times in all\" only beside the one-sentence fear, nothing of where she came from — in one of the three frame orders, named \"Elsbeth Thorne\" first, on {} recorded seeds (frames {frames:?}, the one-sentence fear on {one_sentence})",
            recorded().len()
        ),
        vector,
    )
}

/// Stage a questing death and its page: year 1 left at home, Brannoc falls at the Barrow
/// by `harm::die` (the rule a disaster's roll calls, with `fate.fell`'s telling), the
/// winter let pass by the pointer, and the turning turned to his death page.
pub fn stage_questing_death(sim: &mut HeadlessSim) {
    use crate::resolve::Afield;
    crate::w7::stay_home_into_winter(sim);
    let brannoc = hero_named(sim, "Brannoc");
    let content = match crate::content::load() {
        Ok(content) => content,
        Err(error) => crate::checks::fail("the content did not load", &error.to_string()),
    };
    {
        let house = sim.world_mut().resource_mut::<House>();
        let Some(quest) = house.board.first().map(|posted| posted.quest.clone()) else {
            crate::checks::fail("the questing death needs year 1's board", "no quest posted");
        };
        let field = Afield {
            content: &content,
            quest: &quest,
            outcome: crate::ids::Outcome::Disaster,
            year: 1,
        };
        let fate = crate::text::fmt(
            &content.words[crate::words::W::FateFell],
            &[crate::harm::place_name(&field)],
        );
        crate::harm::die(&field, house, brannoc, fate, &mut Vec::new());
    }
    point_at(sim, Target::LetWinterPass, true);
    crate::w8::turn_to_kind(sim, PageKind::Death);
}

/// Stage an old-age death and its page: year 1 left at home, Odo aged to his death (93,
/// and 94 after the turning's +1, where old age is certain), the winter let pass by the
/// pointer, and the turning turned to his death page.
pub fn stage_old_age(sim: &mut HeadlessSim) {
    crate::w7::stay_home_into_winter(sim);
    let odo = hero_named(sim, "Odo");
    sim.world_mut().resource_mut::<House>().heroes[odo].age = 93;
    point_at(sim, Target::LetWinterPass, true);
    crate::w8::turn_to_kind(sim, PageKind::Death);
}

/// The two staged death pages carry their dead's epitaph under the card, named first.
pub fn check_pages(checks: &mut Checks) -> String {
    let mut read = Vec::new();
    for (name, stage) in [
        ("Brannoc", stage_questing_death as fn(&mut HeadlessSim)),
        ("Odo", stage_old_age),
    ] {
        let mut sim = session(crate::verify::SEEDS[0]);
        stage(&mut sim);
        let id = hero_named(&sim, name);
        let house = sim.world().resource::<House>();
        let epitaph = house.heroes[id].epitaph.clone().unwrap_or_default();
        let full = house.heroes[id].full_name();
        let panel = lines_in(&page_of(&sim), crate::telling_view::PANEL);
        checks.require(
            panel.get(2) == Some(&epitaph)
                && epitaph.contains(&full)
                && house.heroes[id].fate_year == 1,
            "a death page does not set its dead's epitaph, named, under the title",
            format!("{name}: {panel:?}"),
        );
        read.push(format!("{name}: {epitaph:?}"));
    }
    format!(
        "W9 death pages: a questing death's and an old age's epitaph at the top of the page — {}",
        read.join("; ")
    )
}
