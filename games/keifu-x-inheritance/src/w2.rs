//! W2's oracle and its rules, asked of the running game's own house.
//!
//! MODULES.md's W2 oracle seats Maren and Garrick on "The bell under the tide" in
//! year 1 and reads the card's fear line and "you bring". The rules are asked
//! directly here, and since W4 the same strings are read off the real card after
//! two drags (`w4::check_inherited`); the three strings are printed for the owner
//! to hold beside the original's card.
//!
//! Then a staged year-1 story walks every W2 rule once on a copy of the founding
//! household — a bond formed, kept and changed; a fear broken at the Coast; a
//! death grieved, borne and dreaded; a fear conquered; dread shed; the destiny
//! predicates; the Seer speaking — and reads the result back off the sheets.
//!
//! INVARIANT: every expectation here is a shipped literal, copied by hand from
//! MODULES.md, `household.json`, CONSTANTS.md and `lines.json` — never computed by
//! the code under test.

use jidousha::prelude::*;

use crate::bonds::{change, form, steadying_companion};
use crate::checks::Checks;
use crate::content::Content;
use crate::destiny::{
    claimed, crown_claims, fire_claims, may_still_learn, mends, shields_on_quests, speak,
};
use crate::fear::{Occasion, add_dread, conquer, fear_power, refuses, shed_dread};
use crate::grief::grieve;
use crate::hero::{Fate, Hero, HeroId};
use crate::house::House;
use crate::ids::{BondKind, Destiny, Outcome, Place, Tag};
use crate::power::{QuestFacts, fear_items, fear_line, party_power, you_bring};
use crate::verify::{SEEDS, content_of, hero_named, session};

/// MODULES.md W2: the card's fear line items, in seat order (Maren, then Garrick).
pub const W2_FEAR_ITEMS: [&str; 2] = ["Maren -2, steadied", "Garrick -3, steadied"];
/// MODULES.md W2: "you bring 4" (1 + 1 + 2 for parent and child).
pub const W2_YOU_BRING: &str = "you bring 4";
/// The whole line as the card shows it (SPEC-GAPS KG-7's glue).
pub const W2_FEAR_LINE: &str = "Fear: Maren -2, steadied, Garrick -3, steadied";
/// The quest the oracle seats them on, and where it is (`quests.json`).
const BELL: &str = "The bell under the tide";

/// The W2 oracle on every seed. Returns the summary line and the printed vector.
pub fn check_oracle(checks: &mut Checks) -> (String, Vec<String>) {
    let mut vector = Vec::new();
    for seed in SEEDS {
        let sim = session(seed);
        let (maren, garrick) = (hero_named(&sim, "Maren"), hero_named(&sim, "Garrick"));
        let content = content_of(&sim);
        let house = sim.world().resource::<House>();
        checks.require(
            house.calendar.current_year() == 1 && house.patrons == 0,
            "the W2 oracle is asked in year 1 with no patrons",
            format!(
                "seed {seed:#x}: year {}, patrons {}",
                house.calendar.current_year(),
                house.patrons
            ),
        );
        let Some(bell) = content.quest_templates.iter().find(|t| t.title == BELL) else {
            checks.require(
                false,
                "the W2 oracle's quest is in quests.json",
                format!("no template titled {BELL:?}"),
            );
            continue;
        };
        checks.require(
            bell.place == Place::DrownedCoast,
            "the bell under the tide is at the Drowned Coast",
            format!("its place is {:?}", bell.place),
        );
        let party = [maren, garrick];
        let items = fear_items(content, &house.heroes, &party, &bell.tags);
        let bring = you_bring(
            content,
            party_power(&house.heroes, &party, QuestFacts::of(bell), house.patrons),
        );
        checks.require(
            items == W2_FEAR_ITEMS,
            "W2 oracle: the fear line lists Maren then Garrick, both steadied",
            format!("seed {seed:#x}: {items:?}, want {W2_FEAR_ITEMS:?}"),
        );
        checks.require(
            bring == W2_YOU_BRING,
            "W2 oracle: Maren and Garrick bring 4 to the bell",
            format!("seed {seed:#x}: {bring:?}, want {W2_YOU_BRING:?}"),
        );
        let line = fear_line(content, &house.heroes, &party, &bell.tags);
        checks.require(
            line.as_deref() == Some(W2_FEAR_LINE),
            "W2 oracle: the whole fear line reads as the card would show it",
            format!("seed {seed:#x}: {line:?}"),
        );
        if seed == SEEDS[0] {
            vector = vec![
                format!(
                    "W2 vector, fear line, seat 1 (Maren): {:?}",
                    items.first().map_or("", String::as_str)
                ),
                format!(
                    "W2 vector, fear line, seat 2 (Garrick): {:?}",
                    items.get(1).map_or("", String::as_str)
                ),
                format!("W2 vector, power: {bring:?}"),
            ];
        }
    }
    (
        format!(
            "W2 oracle: Maren and Garrick on {BELL:?}, year 1 — fear line and power hold on {} seeds",
            SEEDS.len()
        ),
        vector,
    )
}

/// One expectation of the story.
fn expect<T: PartialEq + std::fmt::Debug>(checks: &mut Checks, what: &str, have: T, want: T) {
    let ok = have == want;
    checks.require(ok, what, format!("have {have:?}, want {want:?}"));
}

fn sheet_lines(content: &Content, heroes: &[Hero], id: HeroId) -> Vec<String> {
    crate::sheet::hero_sheet(content, heroes, id)
        .lines
        .into_iter()
        .map(|line| line.text)
        .collect()
}

/// The staged year-1 story. Returns its summary line.
pub fn check_rules(checks: &mut Checks) -> String {
    let before = checks.counts().0 + checks.counts().1;
    let sim = session(SEEDS[0]);
    let content = content_of(&sim);
    let mut heroes = sim.world().resource::<House>().heroes.clone();
    let at = |name: &str| {
        heroes
            .iter()
            .position(|h| h.name == name)
            .unwrap_or(usize::MAX)
    };
    let [garrick, maren, pip, ysolde, brannoc, odo, wren] = [
        "Garrick", "Maren", "Pip", "Ysolde", "Brannoc", "Odo", "Wren",
    ]
    .map(at);
    let bond = |heroes: &[Hero], a: HeroId, b: HeroId| {
        heroes[a].bond_to(b).map(|bond| (bond.kind, bond.since))
    };

    // Bonds: Odo takes Pip as a student; a companionship does not undo a friendship;
    // Brannoc and Ysolde make peace.
    form(&mut heroes, pip, odo, BondKind::Mentor, 1);
    expect(
        checks,
        "a formed bond is mentor on the learner's side",
        bond(&heroes, pip, odo),
        Some((BondKind::Mentor, 1)),
    );
    expect(
        checks,
        "and student on the teacher's",
        bond(&heroes, odo, pip),
        Some((BondKind::Student, 1)),
    );
    form(&mut heroes, garrick, odo, BondKind::Companion, 1);
    expect(
        checks,
        "a lower rank leaves a bond as it was",
        bond(&heroes, odo, garrick),
        Some((BondKind::Friend, -19)),
    );
    form(&mut heroes, garrick, odo, BondKind::Rival, 1);
    expect(
        checks,
        "an equal rank leaves a bond as it was",
        bond(&heroes, garrick, odo),
        Some((BondKind::Friend, -19)),
    );
    form(&mut heroes, maren, garrick, BondKind::Spouse, 1);
    expect(
        checks,
        "a lower rank never replaces a parent",
        bond(&heroes, maren, garrick),
        Some((BondKind::Parent, -37)),
    );
    change(&mut heroes, brannoc, ysolde, BondKind::Friend, 1);
    expect(
        checks,
        "a change overwrites one side",
        bond(&heroes, brannoc, ysolde),
        Some((BondKind::Friend, 1)),
    );
    expect(
        checks,
        "and the other",
        bond(&heroes, ysolde, brannoc),
        Some((BondKind::Friend, 1)),
    );
    expect(
        checks,
        "Garrick's steadying companion beside Ysolde, Odo and Maren is Odo",
        steadying_companion(&heroes, &[ysolde, odo, maren, garrick], garrick),
        Some(odo),
    );

    // Fear: Maren takes 4 dread at the Coast and breaks.
    let lines = add_dread(
        content,
        &mut heroes,
        maren,
        4,
        Occasion::At(Place::DrownedCoast),
        1,
    );
    expect(checks, "dread stops at 5", heroes[maren].fear.dread, 5);
    expect(checks, "at 5 the hero breaks and it is told", lines, vec![
        "Something in Maren gave way. She will not go against deep water again, for anyone. It is a scar now, and her children may be born with it.".to_owned(),
    ]);
    expect(
        checks,
        "the break leaves a scar naming the tag and the place",
        heroes[maren].scars.clone(),
        vec!["Broken by deep water at the Drowned Coast".to_owned()],
    );
    expect(
        checks,
        "a broken fear refuses the bell",
        refuses(&heroes[maren], &[Tag::Water]),
        true,
    );
    expect(
        checks,
        "and still costs power, now -4",
        fear_power(&heroes[maren], &[Tag::Water]),
        -4,
    );
    let bell = QuestFacts {
        place: Place::DrownedCoast,
        aptitude: crate::ids::Aptitude::Spirit,
        tags: &[Tag::Water],
        door_lock: false,
    };
    expect(
        checks,
        "the fear line follows her dread",
        fear_items(content, &heroes, &[maren, garrick], bell.tags),
        vec![
            "Maren -4, steadied".to_owned(),
            "Garrick -3, steadied".to_owned(),
        ],
    );
    expect(
        checks,
        "a broken hero takes no more dread",
        add_dread(
            content,
            &mut heroes,
            maren,
            1,
            Occasion::At(Place::Barrow),
            1,
        )
        .len(),
        0,
    );

    // Grief: Garrick dies. Maren (broken) bears it; Odo (outlive) grieves 1 + 2.
    heroes[garrick].fate = Fate::Dead;
    let lines = grieve(content, &mut heroes, garrick, 1);
    expect(
        checks,
        "Garrick's death is grieved in bond order and borne at the end",
        lines,
        vec![
            "Odo grieves for his friend. Dread 3 of 5.".to_owned(),
            "Maren grieves for her father, and bears it.".to_owned(),
        ],
    );
    expect(
        checks,
        "an outliver's grief strikes two deeper",
        heroes[odo].fear.dread,
        3,
    );
    expect(
        checks,
        "grief comes once",
        grieve(content, &mut heroes, garrick, 1).len(),
        0,
    );
    expect(
        checks,
        "the dead no longer steady",
        steadying_companion(&heroes, &[garrick, odo], odo),
        None,
    );

    // Conquering: Odo conquers crowds at the King's Court.
    let lines = conquer(content, &mut heroes, odo, Place::KingsCourt, 1);
    expect(checks, "conquering is told", lines, vec![
        "Odo has conquered his fear of crowds. What was a weight is a strength: +2 against Crowds from now on. His children may be born brave.".to_owned(),
    ]);
    expect(checks, "conquering clears dread", heroes[odo].fear.dread, 0);
    expect(
        checks,
        "a conquered fear gives +2",
        fear_power(&heroes[odo], &[Tag::Crowds]),
        2,
    );

    // Shedding: Wren takes 2 dread at Emberfall, then rests by the fire.
    add_dread(
        content,
        &mut heroes,
        wren,
        2,
        Occasion::At(Place::Emberfall),
        1,
    );
    expect(checks, "a child can take dread", heroes[wren].fear.dread, 2);
    expect(
        checks,
        "resting sheds one",
        (shed_dread(&mut heroes[wren]), heroes[wren].fear.dread),
        (true, 1),
    );
    expect(
        checks,
        "the broken shed nothing",
        (shed_dread(&mut heroes[maren]), heroes[maren].fear.dread),
        (false, 5),
    );

    // Destinies.
    expect(
        checks,
        "Brannoc's fire shields him on quests; Odo is not shielded",
        (
            shields_on_quests(&heroes[brannoc]),
            shields_on_quests(&heroes[odo]),
        ),
        (true, false),
    );
    expect(
        checks,
        "the fire claims Brannoc on a failed fire quest only",
        Outcome::ALL
            .iter()
            .map(|o| fire_claims(&heroes[brannoc], &[Tag::Fire], *o))
            .collect::<Vec<_>>(),
        vec![true, true, false, false],
    );
    expect(
        checks,
        "the crown claims nobody in the founding",
        heroes
            .iter()
            .any(|h| crown_claims(h, Place::KingsCourt, Outcome::Triumph)),
        false,
    );
    expect(
        checks,
        "Maren's first disaster mends",
        mends(&heroes[maren]),
        true,
    );
    expect(
        checks,
        "Garrick's grown firstborn stops his learning; Maren still learns",
        (
            may_still_learn(&heroes, garrick),
            may_still_learn(&heroes, maren),
        ),
        (false, true),
    );
    let held: Vec<Destiny> = claimed(content, &heroes)
        .into_iter()
        .filter(|(_, held)| *held)
        .map(|(kind, _)| kind)
        .collect();
    // Garrick's death freed CHILD_WILL_SURPASS_YOU.
    expect(
        checks,
        "the living hold fire, outlive and mended",
        held,
        vec![
            Destiny::FireWillEndYou,
            Destiny::OutliveThoseYouLove,
            Destiny::BreakAndBeMended,
        ],
    );
    let spoken = speak(content, &mut heroes, pip, &mut Rng::from_seed(SEEDS[0]));
    let unheld = [
        Destiny::WearACrown,
        Destiny::ChildWillSurpassYou,
        Destiny::DieInYourBed,
        Destiny::CarryTheHouse,
        Destiny::TeachAGreater,
    ];
    checks.require(
        spoken.is_some_and(|kind| unheld.contains(&kind)),
        "the Seer speaks Pip a destiny no living hero holds",
        format!("spoke {spoken:?}"),
    );
    expect(
        checks,
        "a spoken destiny is never spoken again",
        speak(content, &mut heroes, pip, &mut Rng::from_seed(2)),
        None,
    );

    // The sheets read the result.
    let maren_sheet = sheet_lines(content, &heroes, maren);
    for want in [
        "FEAR, BROKEN",
        "Broken by deep water at the Drowned Coast",
        "Father Garrick, gone",
    ] {
        checks.require(
            maren_sheet.iter().any(|l| l == want),
            "Maren's sheet shows what the story did to her",
            format!("missing {want:?} in {maren_sheet:?}"),
        );
    }
    let odo_sheet = sheet_lines(content, &heroes, odo);
    for want in [
        "FEAR, CONQUERED",
        "A strength now: +2 power against Crowds.",
        "Student Pip +1",
    ] {
        checks.require(
            odo_sheet.iter().any(|l| l == want),
            "Odo's sheet shows what the story did to him",
            format!("missing {want:?} in {odo_sheet:?}"),
        );
    }
    let pip_sheet = sheet_lines(content, &heroes, pip);
    checks.require(
        !pip_sheet.iter().any(|l| l == "Not yet spoken."),
        "Pip's sheet shows the destiny the Seer spoke",
        format!("{pip_sheet:?}"),
    );
    let after = checks.counts().0 + checks.counts().1;
    format!(
        "W2 rules: a staged year-1 story, {} checks over bonds, fears, grief and destinies",
        after - before
    )
}
