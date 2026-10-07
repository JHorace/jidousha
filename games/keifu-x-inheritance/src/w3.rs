//! W3's oracle and its rules, asked of the running game's own house.
//!
//! MODULES.md's W3 oracle seats Garrick on a Barrow quest, reads the card's
//! "Dream:" mark and the quest sheet's line, and has him triumph there: he is
//! settled and "Garrick's rest: +2 against Undead" is laid on Garrick, Maren and
//! Pip. Quests arrive with W4 and resolution with W6, so the oracle is asked of the
//! rules directly — the call, a staged triumph moment witnessed by the whole house
//! — and its vector is printed for the owner to hold beside the original.
//!
//! Then a staged story walks the rest of W3 on a copy of the house — counting, a
//! carried dream fulfilled for its owner, the own dream before the burden, a blade
//! forged that pushes Thornfall to the heir of the blood, dream rivals — and reads
//! the result back off the sheets and the family screen.
//!
//! INVARIANT: every expectation here is a shipped literal, copied by hand from
//! MODULES.md, `household.json`, `quests.json`, `dreams.json`, `legacies.json`,
//! `lines.json` and `ui-text.json` — never computed by the code under test.

use crate::calls::{Call, Telling, call_line, dream_call, dreamers_line};
use crate::checks::Checks;
use crate::content::{Content, QuestTemplate};
use crate::dream::Dream;
use crate::hero::{Hero, HeroId};
use crate::house::House;
use crate::ids::{Outcome, Place, Tag, WinterAction};
use crate::moment::{Moment, QuestMoment};
use crate::power::{QuestFacts, member_power};
use crate::rivals::dream_rivals;
use crate::verify::{SEEDS, content_of, hero_named, session};
use crate::witness::{witness, witness_all};

/// The opening Barrow quest (`quests.json`), and a Barrow quest against the Undead.
const GRAVE_GOODS: &str = "Grave goods";
const LAMPS: &str = "The lamps in the Barrow";
/// MODULES.md W3: the card's mark, with Ysolde called to a road she has not walked.
pub const W3_CARD: &str = "Dream: Garrick, Ysolde";
/// MODULES.md W3: the quest sheet's line for Garrick.
pub const W3_SHEET: &str = "Garrick's dream: Win a triumph at the Barrow. He must triumph.";
/// `lines.dream.fulfilled` for Garrick and his told title.
pub const W3_SETTLING: &str = "Garrick has done it: to lay the Barrow's dead to rest. He is settled now, and dread has no hold on him.";
/// `lines.legacy.blessing` for Garrick's rest.
pub const W3_LEGACY: &str = "It leaves a blessing, Garrick's rest: +2 against Undead, for Garrick, Maren and Pip and every child born to them.";
/// MODULES.md W3: the blessing's name, effect and recipients.
pub const W3_BLESSING: (&str, &str) = ("Garrick's rest", "+2 against Undead");
/// MODULES.md W3: whom it is laid on, in creation order.
pub const W3_RECIPIENTS: [&str; 3] = ["Garrick", "Maren", "Pip"];

fn expect<T: PartialEq + std::fmt::Debug>(checks: &mut Checks, what: &str, have: T, want: T) {
    let ok = have == want;
    checks.require(ok, what, format!("have {have:?}, want {want:?}"));
}

fn template<'c>(
    checks: &mut Checks,
    content: &'c Content,
    title: &str,
) -> Option<&'c QuestTemplate> {
    let found = content.quest_templates.iter().find(|t| t.title == title);
    checks.require(
        found.is_some(),
        "a W3 quest is in quests.json",
        format!("no template titled {title:?}"),
    );
    found
}

/// The staged triumph: Garrick alone wins "Grave goods" at the Barrow, and every
/// living hero witnesses it. Returns the lines the moment wrote.
pub fn stage_triumph(content: &Content, house: &mut House) -> Vec<String> {
    let Some(garrick) = house.heroes.iter().position(|h| h.name == "Garrick") else {
        return Vec::new();
    };
    let Some(grave) = content
        .quest_templates
        .iter()
        .find(|t| t.title == GRAVE_GOODS)
    else {
        return Vec::new();
    };
    let party = [garrick];
    let moment = Moment::Quest(QuestMoment {
        place: grave.place,
        tags: &grave.tags,
        outcome: Outcome::Triumph,
        party: &party,
    });
    witness_all(content, house, &moment)
}

/// The oracle's triumph, then Garrick carries Brannoc's blade dream to a triumph
/// at Emberfall: the sheets W3 changes most, staged. Returns every line written.
pub fn stage_settled(content: &Content, house: &mut House) -> Vec<String> {
    let mut lines = stage_triumph(content, house);
    let named = |house: &House, name: &str| house.heroes.iter().position(|h| h.name == name);
    let (Some(garrick), Some(brannoc)) = (named(house, "Garrick"), named(house, "Brannoc")) else {
        return lines;
    };
    let mut blade = house.heroes[brannoc].dream.clone();
    if let Some(dream) = blade.as_mut() {
        dream.owner = Some(brannoc);
        dream.advance_to_stage(2);
    }
    house.heroes[garrick].burden = blade;
    let party = [garrick];
    let moment = Moment::Quest(QuestMoment {
        place: Place::Emberfall,
        tags: &[Tag::Fire],
        outcome: Outcome::Triumph,
        party: &party,
    });
    lines.extend(witness(content, house, garrick, &moment));
    lines
}

fn holders(heroes: &[Hero], title: &str) -> Vec<String> {
    heroes
        .iter()
        .filter(|h| h.blessings.iter().any(|b| b.title == title))
        .map(|h| h.name.clone())
        .collect()
}

/// The W3 oracle on every seed. Returns the summary line and the printed vector.
pub fn check_oracle(checks: &mut Checks) -> (String, Vec<String>) {
    let mut vector = Vec::new();
    for seed in SEEDS {
        let sim = session(seed);
        let content = content_of(&sim);
        let garrick = hero_named(&sim, "Garrick");
        let mut house = sim.world().resource::<House>().clone();
        let (Some(grave), Some(lamps)) = (
            template(checks, content, GRAVE_GOODS),
            template(checks, content, LAMPS),
        ) else {
            continue;
        };
        expect(
            checks,
            "both W3 quests are at the Barrow",
            (grave.place, lamps.place),
            (Place::Barrow, Place::Barrow),
        );
        expect(
            checks,
            "the lamps carry the Undead; grave goods only the dark",
            (lamps.tags.clone(), grave.tags.clone()),
            (vec![Tag::Dark, Tag::Undead], vec![Tag::Dark]),
        );
        let seated = [garrick];
        let triumph = Some(Call {
            burden: false,
            telling: Telling::Triumph,
        });
        let mark = dream_call(
            content,
            &house.heroes,
            garrick,
            QuestFacts::of(grave),
            &seated,
        );
        expect(
            checks,
            "W3 oracle: seated on grave goods, Garrick's dream calls him, and needs a triumph",
            mark,
            triumph,
        );
        expect(
            checks,
            "W3 oracle: on the lamps in the Barrow too",
            dream_call(
                content,
                &house.heroes,
                garrick,
                QuestFacts::of(lamps),
                &seated,
            ),
            triumph,
        );
        let card = dreamers_line(content, &house.heroes, QuestFacts::of(grave), &seated);
        expect(
            checks,
            "W3 oracle: the card's Dream: line",
            card.as_deref(),
            Some(W3_CARD),
        );
        let sheet = mark.map(|call| call_line(content, &house.heroes, garrick, call));
        expect(
            checks,
            "W3 oracle: the quest sheet's dream line",
            sheet.as_deref(),
            Some(W3_SHEET),
        );
        let lamps_before = member_power(&house.heroes[garrick], QuestFacts::of(lamps));

        let lines = stage_triumph(content, &mut house);
        expect(
            checks,
            "W3 oracle: the triumph settles Garrick and lays his rest, and writes nothing else",
            lines.clone(),
            vec![W3_SETTLING.to_owned(), W3_LEGACY.to_owned()],
        );
        let hero = &house.heroes[garrick];
        expect(
            checks,
            "W3 oracle: Garrick is settled, without dread, his dream fulfilled",
            (
                hero.settled,
                hero.fear.dread,
                hero.dream.as_ref().is_some_and(Dream::is_fulfilled),
            ),
            (true, 0, true),
        );
        let blessing = hero.blessings.first();
        let effect = blessing.map(|b| crate::blessing::blessing_effect(content, b));
        expect(
            checks,
            "W3 oracle: the blessing's name and effect",
            (blessing.map(|b| b.title.as_str()), effect.as_deref()),
            (Some(W3_BLESSING.0), Some(W3_BLESSING.1)),
        );
        let recipients = holders(&house.heroes, W3_BLESSING.0);
        expect(
            checks,
            "W3 oracle: laid on exactly Garrick, Maren and Pip",
            recipients.clone(),
            W3_RECIPIENTS.map(str::to_owned).to_vec(),
        );
        // Line 7: +2 on the lamps (Undead), nothing on grave goods (Dark only).
        let hero = &house.heroes[garrick];
        expect(
            checks,
            "power line 7: Garrick's rest adds 2 against the Undead and nothing elsewhere",
            (
                member_power(hero, QuestFacts::of(lamps)) - lamps_before,
                member_power(hero, QuestFacts::of(grave)),
            ),
            // 6 as mainline, plus the variant's Strong.
            (2, 7),
        );
        expect(
            checks,
            "a settled dreamer is called no more",
            dream_call(
                content,
                &house.heroes,
                garrick,
                QuestFacts::of(grave),
                &seated,
            ),
            None,
        );
        if seed == SEEDS[0] {
            vector = vec![
                format!(
                    "W3 vector, mark: Garrick seated on {GRAVE_GOODS:?} at the Barrow is called: {}",
                    match mark {
                        Some(call) => format!("yes, own dream, needs {:?}", call.telling),
                        None => "no".to_owned(),
                    }
                ),
                format!("W3 vector, card: {:?}", card.unwrap_or_default()),
                format!("W3 vector, quest sheet: {:?}", sheet.unwrap_or_default()),
                format!(
                    "W3 vector, settling: {:?}",
                    lines.first().cloned().unwrap_or_default()
                ),
                format!(
                    "W3 vector, legacy: {:?}",
                    lines.get(1).cloned().unwrap_or_default()
                ),
                format!(
                    "W3 vector, blessing: {:?}, {:?}, laid on {}",
                    blessing.map(|b| b.title.clone()).unwrap_or_default(),
                    effect.unwrap_or_default(),
                    recipients.join(", ")
                ),
            ];
        }
    }
    (
        format!(
            "W3 oracle: Garrick's call, triumph, settling and rest hold on {} seeds",
            SEEDS.len()
        ),
        vector,
    )
}

fn sheet_lines(content: &Content, heroes: &[Hero], id: HeroId) -> Vec<String> {
    crate::sheet::hero_sheet(content, heroes, id)
        .lines
        .into_iter()
        .map(|l| l.text)
        .collect()
}

fn shows(checks: &mut Checks, what: &str, have: &[String], want: &[&str]) {
    for line in want {
        checks.require(
            have.iter().any(|h| h == line),
            what,
            format!("missing {line:?} in {have:?}"),
        );
    }
}

/// The staged W3 story. Returns its summary line.
pub fn check_rules(checks: &mut Checks) -> String {
    let before = checks.counts().0 + checks.counts().1;
    let sim = session(SEEDS[0]);
    let content = content_of(&sim);
    let mut house = sim.world().resource::<House>().clone();
    let at = |house: &House, name: &str| {
        house
            .heroes
            .iter()
            .position(|h| h.name == name)
            .unwrap_or(usize::MAX)
    };
    let [garrick, maren, pip, ysolde, elsbeth] =
        ["Garrick", "Maren", "Pip", "Ysolde", "Elsbeth"].map(|n| at(&house, n));

    // Ysolde walks new roads: two counted, the third done.
    let mut roads = Vec::new();
    for place in [Place::Barrow, Place::DrownedCoast, Place::HighPass] {
        let party = [ysolde];
        roads.extend(witness(
            content,
            &mut house,
            ysolde,
            &Moment::Quest(QuestMoment {
                place,
                tags: &[],
                outcome: Outcome::Setback,
                party: &party,
            }),
        ));
        house.heroes[ysolde].roads_walked.push(place);
    }
    expect(
        checks,
        "new roads are counted to three, then the stage is done",
        roads,
        vec![
            "Ysolde comes one nearer her dream: quest at three different places, 1 of 3."
                .to_owned(),
            "Ysolde comes one nearer her dream: quest at three different places, 2 of 3."
                .to_owned(),
            "Ysolde is a step nearer her dream. What is left: quest at every place not yet walked."
                .to_owned(),
        ],
    );

    // Garrick triumphs (the oracle), then carries Brannoc's blade dream to a triumph:
    // his own is fulfilled, so the burden moves, and Thornfall passes to Maren.
    let lines = stage_settled(content, &mut house).split_off(2);
    expect(checks, "a settled hero's burden is carried to a blade, and Thornfall goes to his daughter", lines, vec![
        "Garrick has done what Brannoc could not: to forge a blade worth a name. It is finished. He is settled now, and dread has no hold on him.".to_owned(),
        "Garrick has two hands and one of them is full. Thornfall passes to Maren.".to_owned(),
        "It leaves an heirloom: Emberwake. +2 Might on quests. Forged by Garrick Thorne at Emberfall, and named in year 1 after a triumph.".to_owned(),
    ]);
    shows(
        checks,
        "Garrick's sheet shows what W3 did to him",
        &sheet_lines(content, &house.heroes, garrick),
        &[
            "Renown 6. Settled.",
            "Fulfilled.",
            "Settled. Dread has no hold.",
            "Taken up from Brannoc Hale.",
            "Emberwake",
            "+2 Might on quests.",
            "BLESSED",
            "Garrick's rest",
            "+2 against Undead",
            "LEAVES",
        ],
    );
    shows(
        checks,
        "Maren's sheet carries Thornfall and her father's rest",
        &sheet_lines(content, &house.heroes, maren),
        &[
            "Thornfall",
            "+1 Might on quests.",
            "Garrick's rest",
            "+2 against Undead",
        ],
    );
    let remembered = crate::family::remembrance(content, &house, garrick);
    let done = "He wanted to lay the Barrow's dead to rest and has done it.";
    checks.require(
        remembered.iter().any(|l| l.contains(done)),
        "the family remembers Garrick's dream done",
        format!("wanted {done:?} in {remembered:?}"),
    );

    // Pip carries Elsbeth's dream to its last stage and tells the tale: a house tale.
    let mut sea = house.heroes[elsbeth].dream.clone();
    if let Some(dream) = sea.as_mut() {
        dream.owner = Some(elsbeth);
        dream.advance_to_stage(2);
    }
    house.heroes[pip].burden = sea;
    let lines = witness(
        content,
        &mut house,
        pip,
        &Moment::Winter(WinterAction::TellTheTale),
    );
    expect(checks, "Pip's own dream does not move at the hearth, so Elsbeth's does, and leaves a tale", lines, vec![
        "Pip has done what Elsbeth could not: to see the sea. It is finished. He is settled now, and dread has no hold on him.".to_owned(),
        "It leaves a tale: The tale of Elsbeth and the sea. It will be told as long as there is a house: +1 renown every year.".to_owned(),
    ]);
    expect(
        checks,
        "the family tally counts the tale",
        crate::family::tally(content, &house).contains("One tale of the house is told."),
        true,
    );

    // Dream rivals: Ysolde takes up an avenging dream, as Maren has one.
    let mut avenge = house.heroes[maren].dream.clone();
    if let Some(dream) = avenge.as_mut() {
        dream.setup = Some(crate::dream::Setup {
            place: Place::KingsCourt,
            tag: Tag::Crowds,
            lost: None,
        });
    }
    if let Some(dream) = avenge {
        house.heroes[ysolde].dream = Some(dream.clone());
        let lines = dream_rivals(content, &mut house.heroes, ysolde, &dream, 1);
        expect(checks, "two avengers of different dead are dream rivals", lines, vec![
            "Ysolde wants what Maren wants: to avenge her mother. There is only room for one of them to be first. They are rivals.".to_owned(),
        ]);
        shows(
            checks,
            "Ysolde's sheet names her rival",
            &sheet_lines(content, &house.heroes, ysolde),
            &["Rival Maren -1"],
        );
    }
    let after = checks.counts().0 + checks.counts().1;
    format!(
        "W3 rules: a staged story, {} checks over witnessing, fulfilment, legacies and rivals",
        after - before
    )
}
