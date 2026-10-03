//! The Door's prologue (SPEC §16.2 step 2), unit-tested line by line: the party named,
//! each member's age, descent and every fragment that applies, who went beside whom, and
//! what they brought to each lock.
//!
//! INVARIANT: every expectation is a shipped literal from `lines.json` filled by hand.

use crate::door_tests::at_the_door;
use crate::hero::{DeedKind, HeroId};
use crate::ids::{Aptitude, Tag};
use crate::testkit::{id, seat};

#[test]
fn the_prologue_names_the_four_what_each_carried_who_went_beside_whom_and_what_they_brought() {
    let (content, mut house) = at_the_door();
    let party: Vec<HeroId> = ["Garrick", "Maren", "Ysolde", "Brannoc"]
        .iter()
        .map(|n| id(&house.heroes, n))
        .collect();
    seat(&mut house, 0, &party);
    let lines = crate::door_prologue::prologue(&content, &house, &party);
    assert_eq!(
        lines,
        [
            "In the last summer the Sealed Door stood open, as it had been foretold for \
             twenty-five years. The house sent Garrick Thorne, Maren Thorne, Ysolde Vane and \
             Brannoc Hale.",
            "Garrick was 62, and of the house before its years were counted. He carried \
             Thornfall. Carried by every Thorne since the first. Nobody remembers who made it.",
            "Maren was 38, the daughter of Garrick and Elsbeth.",
            "Ysolde was 24, and of the house before its years were counted. The Seer had said \
             it: \"You will open the Sealed Door.\" +5 at every lock. She had been afraid of \
             the dark all her life, and went down into it anyway: -2.",
            "Brannoc was 31, and of the house before its years were counted.",
            "Garrick went down beside his daughter Maren (+2), at every lock.",
            "Ysolde went down beside her rival Brannoc (-1), at every lock.",
            "Against three locks of 34, 34, and 34, they brought Might 22, Wits 22, and \
             Spirit 18. The dark and the cold lay ahead.",
        ]
    );
}

#[test]
fn the_prologue_tells_a_grandchild_an_arrival_the_blood_a_conquest_a_blessing_and_a_wound() {
    let (content, mut house) = at_the_door();
    let (pip, wren, ysolde, odo) = (
        id(&house.heroes, "Pip"),
        id(&house.heroes, "Wren"),
        id(&house.heroes, "Ysolde"),
        id(&house.heroes, "Odo"),
    );
    // Pip: Maren's son, Garrick and Elsbeth's grandson, promised by blood, wounded.
    house.heroes[pip].age = 20;
    house.heroes[pip].destiny.kind = crate::ids::Destiny::OpenTheSealedDoor;
    house.heroes[pip].destiny.blood_of = Some("Ysolde".to_owned());
    house.heroes[pip].wounded = true;
    // Wren: afraid of the cold no more — conquered in year 9; blessed everywhere.
    house.heroes[wren].age = 18;
    house.heroes[wren].fear.tag = Tag::Cold;
    house.heroes[wren].fear.conquered = true;
    house.heroes[wren].deeds.push(crate::epitaph_tests::deed(
        DeedKind::ConqueredFear,
        9,
        14,
        None,
        0,
    ));
    house.heroes[wren].blessings.push(crate::hero::Blessing {
        title: "Odo's patience".to_owned(),
        scope: crate::hero::Scope::Everywhere,
        power: 1,
    });
    // Ysolde born brave of the dark; Odo come up the road in year 3.
    house.heroes[ysolde].fear.born_brave = true;
    house.heroes[ysolde].fear.conquered = true;
    house.heroes[odo].deeds.push(crate::epitaph_tests::deed(
        DeedKind::Arrived,
        3,
        40,
        None,
        0,
    ));
    let lines = crate::door_prologue::prologue(&content, &house, &[pip, wren, ysolde, odo]);
    assert_eq!(
        lines[1],
        "Pip was 20, the son of Maren and the grandson of Garrick and Elsbeth. The Door was \
         promised to Ysolde, and he went as Ysolde's blood: +5 at every lock. He had been \
         afraid of the dark all his life, and went down into it anyway: -2. He went \
         wounded: -2."
    );
    assert_eq!(
        lines[2],
        "Wren was 18, the daughter of Brannoc and Aud. She had conquered her fear of the cold \
         in year 9, and here it counted: +2. Odo's patience went with her: +1."
    );
    assert_eq!(
        lines[3],
        "Ysolde was 24, and of the house before its years were counted. The Seer had said \
         it: \"You will open the Sealed Door.\" +5 at every lock. She was born unafraid of \
         the dark, and here it counted: +2."
    );
    assert_eq!(lines[4], "Odo was 47, and had come up the road in year 3.");
    // No conquest on record: the conquered line without a year.
    house.heroes[wren].deeds.clear();
    let lines = crate::door_prologue::prologue(&content, &house, &[wren]);
    assert_eq!(
        lines[1],
        "Wren was 18, the daughter of Brannoc and Aud. She had conquered her fear of the cold, \
         and here it counted: +2. Odo's patience went with her: +1."
    );
}

#[test]
fn an_heirloom_of_ones_own_making_is_carried_as_ones_own() {
    let (content, mut house) = at_the_door();
    let maren = id(&house.heroes, "Maren");
    house.heroes[maren].heirloom = Some(crate::hero::Heirloom {
        name: "Maren's road-book".to_owned(),
        sprite: "reward-guidebook".to_owned(),
        aptitude: Aptitude::Wits,
        bonus: 2,
        provenance: "Kept by Maren, year 4.".to_owned(),
    });
    let lines = crate::door_prologue::prologue(&content, &house, &[maren]);
    assert_eq!(
        lines[1],
        "Maren was 38, the daughter of Garrick and Elsbeth. She carried her own road-book. Kept \
         by Maren, year 4."
    );
}

#[test]
fn the_prologue_counts_dread_in_the_penalty_and_leaves_out_bonds_of_the_road_and_narrow_blessings()
{
    let (content, mut house) = at_the_door();
    let party: Vec<HeroId> = ["Garrick", "Maren", "Ysolde", "Brannoc"]
        .iter()
        .map(|n| id(&house.heroes, n))
        .collect();
    let (maren, ysolde) = (party[1], party[2]);
    // Dread 3: a penalty of 2 + 3/2 = 3.
    house.heroes[ysolde].fear.dread = 3;
    // A blessing against the walking dead counts at no lock, and is not told.
    house.heroes[ysolde].blessings.push(crate::hero::Blessing {
        title: "Garrick's rest".to_owned(),
        scope: crate::hero::Scope::AgainstTag(Tag::Undead),
        power: 2,
    });
    // Companions of the road: a bond that is not shown, so no one went beside anyone for it.
    crate::bonds::form(
        &mut house.heroes,
        maren,
        ysolde,
        crate::ids::BondKind::Companion,
        3,
    );
    seat(&mut house, 0, &party);
    let lines = crate::door_prologue::prologue(&content, &house, &party);
    assert_eq!(
        lines[3],
        "Ysolde was 24, and of the house before its years were counted. The Seer had said \
         it: \"You will open the Sealed Door.\" +5 at every lock. She had been afraid of \
         the dark all her life, and went down into it anyway: -3."
    );
    assert_eq!(
        lines[5],
        "Garrick went down beside his daughter Maren (+2), at every lock."
    );
    assert_eq!(
        lines[6],
        "Ysolde went down beside her rival Brannoc (-1), at every lock."
    );
    assert_eq!(lines.len(), 8);
}

#[test]
fn the_prologues_summary_counts_a_patron_at_court_at_every_lock() {
    let (content, mut house) = at_the_door();
    let party: Vec<HeroId> = ["Garrick", "Maren", "Ysolde", "Brannoc"]
        .iter()
        .map(|n| id(&house.heroes, n))
        .collect();
    seat(&mut house, 0, &party);
    house.patrons = 1;
    let lines = crate::door_prologue::prologue(&content, &house, &party);
    assert_eq!(
        lines.last().map(String::as_str),
        Some(
            "Against three locks of 34, 34, and 34, they brought Might 23, Wits 23, and Spirit \
             19. The dark and the cold lay ahead."
        )
    );
}
