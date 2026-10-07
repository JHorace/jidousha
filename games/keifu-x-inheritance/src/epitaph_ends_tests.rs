//! W9's epitaph rules, part two: DREAM, PROPHECY, LOVE, END and LEFT, each selection rule
//! branch by branch, as a test named as a sentence (SPEC §20).
//!
//! INVARIANT: every expectation is a shipped literal copied by hand from SPEC.md and
//! `content/epitaph.json`'s templates — never computed by the code under test.

use crate::bonds::form;
use crate::epitaph_tests::{both, deed, kill, part};
use crate::hero::{Bond, DeedKind, DreamFate, Fate, Hero};
use crate::ids::{BondKind, Destiny, LegacyKind, Part};
use crate::testkit::{founded, id};

#[test]
fn dream_reads_the_own_dream_done_passed_left_laid_living_crowned_or_died() {
    let (content, mut heroes) = founded();
    let (garrick, maren, pip, elsbeth) = (
        id(&heroes, "Garrick"),
        id(&heroes, "Maren"),
        id(&heroes, "Pip"),
        id(&heroes, "Elsbeth"),
    );
    assert_eq!(part(&content, &heroes, "Wren", Part::Dream, true), "");
    assert_eq!(
        part(&content, &heroes, "Garrick", Part::Dream, true),
        "He wanted to lay the Barrow's dead to rest, and had come two steps of three."
    );
    assert_eq!(
        part(&content, &heroes, "Pip", Part::Dream, true),
        "He wanted to see the sea, and had not yet begun it."
    );
    assert_eq!(
        part(&content, &heroes, "Elsbeth", Part::Dream, true),
        "She wanted to see the sea, and died before it was done."
    );
    assert_eq!(
        part(&content, &heroes, "Maren", Part::Dream, true),
        "She wanted to avenge her mother, and had come one step of three."
    );
    kill(&mut heroes, garrick, 3, 64, "fell at the Barrow");
    heroes[garrick].dream_fate = DreamFate::PassedOn;
    // PASSED_ON with no heir named reads as a death.
    assert_eq!(
        part(&content, &heroes, "Garrick", Part::Dream, false),
        "He wanted to lay the Barrow's dead to rest, and died before it was done."
    );
    heroes[garrick].bequest_heir = Some(maren);
    assert_eq!(
        both(&content, &heroes, "Garrick", Part::Dream),
        (
            "His dream, to lay the Barrow's dead to rest, was left to Maren.".to_owned(),
            "He wanted to lay the Barrow's dead to rest, and died before it was done. Maren \
             carries it now."
                .to_owned()
        )
    );
    heroes[garrick].dream_fate = DreamFate::LeftToNoOne;
    assert_eq!(
        part(&content, &heroes, "Garrick", Part::Dream, true),
        "He wanted to lay the Barrow's dead to rest. It was left to no one, and it has not left \
         the house."
    );
    heroes[garrick].dream_fate = DreamFate::LaidToRest;
    heroes[garrick].laid_year = Some(6);
    assert_eq!(
        part(&content, &heroes, "Garrick", Part::Dream, true),
        "He wanted to lay the Barrow's dead to rest. It was left to no one, and walked until \
         year 6, when the house laid it to rest."
    );
    heroes[garrick].dream_fate = DreamFate::Undecided;
    heroes[garrick].fate = Fate::Departed;
    assert_eq!(
        part(&content, &heroes, "Garrick", Part::Dream, true),
        "He wanted to lay the Barrow's dead to rest, and left it undone for a crown."
    );
    if let Some(dream) = heroes[garrick].dream.as_mut() {
        dream.advance_to_stage(3);
    }
    assert_eq!(
        both(&content, &heroes, "Garrick", Part::Dream),
        (
            "His dream was to lay the Barrow's dead to rest. He lived to see it done.".to_owned(),
            "He wanted one thing, to lay the Barrow's dead to rest, and did it.".to_owned()
        )
    );
    // A dream that was someone's before carries the inherited suffix.
    if let Some(dream) = heroes[pip].dream.as_mut() {
        dream.owner = Some(elsbeth);
        dream.advance_to_stage(3);
    }
    assert_eq!(
        part(&content, &heroes, "Pip", Part::Dream, false),
        "He wanted one thing, to see the sea, as Elsbeth had before him, and did it."
    );
    // " my " in the title is told for the owner, not the bearer.
    let aud = id(&heroes, "Aud");
    let mut grown = heroes[aud].dream.clone().expect("Aud dreams");
    grown.owner = Some(aud);
    grown.advance_to_stage(3);
    heroes[pip].dream = Some(grown);
    assert_eq!(
        part(&content, &heroes, "Pip", Part::Dream, false),
        "He wanted one thing, to see her child grown, as Aud had before him, and did it."
    );
}

#[test]
fn prophecy_tells_each_destiny_and_its_coming() {
    let (content, mut heroes) = founded();
    let say = |heroes: &[Hero], name: &str| part(&content, heroes, name, Part::Prophecy, true);
    assert_eq!(say(&heroes, "Pip"), "");
    assert_eq!(
        say(&heroes, "Garrick"),
        "The Seer said his child would surpass him, and his firstborn was born stronger than he \
         had ever been."
    );
    assert_eq!(
        say(&heroes, "Brannoc"),
        "The Seer said fire would be the end of him, and nothing else could touch him."
    );
    assert_eq!(
        say(&heroes, "Odo"),
        "The Seer said he would outlive those he loved, and every grave the house dug weighed on \
         him."
    );
    assert_eq!(
        say(&heroes, "Maren"),
        "The Seer said she would break and be mended. She never broke."
    );
    assert_eq!(
        say(&heroes, "Ysolde"),
        "The Seer promised her the Door, and she did not live to reach it."
    );
    let (garrick, brannoc, maren, ysolde, odo) = (
        id(&heroes, "Garrick"),
        id(&heroes, "Brannoc"),
        id(&heroes, "Maren"),
        id(&heroes, "Ysolde"),
        id(&heroes, "Odo"),
    );
    heroes[garrick].destiny.fulfilled = false;
    heroes[brannoc].destiny.fulfilled = true;
    heroes[maren].destiny.fulfilled = true;
    assert_eq!(
        say(&heroes, "Garrick"),
        "The Seer said his child would surpass him."
    );
    assert_eq!(
        say(&heroes, "Brannoc"),
        "The Seer said fire would be the end of him, and it was."
    );
    assert_eq!(
        say(&heroes, "Maren"),
        "The Seer said she would break and be mended, and she was."
    );
    heroes[ysolde]
        .deeds
        .push(deed(DeedKind::StoodAtTheDoor, 26, 49, None, 1));
    assert_eq!(
        say(&heroes, "Ysolde"),
        "The Seer promised her the Door, and she kept the appointment."
    );
    heroes[ysolde].destiny.blood_of = Some("Odo".to_owned());
    assert_eq!(
        say(&heroes, "Ysolde"),
        "The Door was promised to Odo, and she answered it as Odo's blood."
    );
    heroes[ysolde].deeds.clear();
    assert_eq!(
        say(&heroes, "Ysolde"),
        "The Door was promised to Odo, and fell to her as Odo's blood."
    );
    for (kind, came, want) in [
        (
            Destiny::WearACrown,
            false,
            "The Seer said she would wear a crown. It never came.",
        ),
        (
            Destiny::WearACrown,
            true,
            "The Seer said she would wear a crown, and she did.",
        ),
        (
            Destiny::DieInYourBed,
            false,
            "The Seer promised her a death in her bed, and no road could take her first.",
        ),
        (
            Destiny::CarryTheHouse,
            false,
            "The Seer said she would carry the house, and she carried it.",
        ),
    ] {
        heroes[ysolde].destiny.kind = kind;
        heroes[ysolde].destiny.fulfilled = came;
        assert_eq!(say(&heroes, "Ysolde"), want);
    }
    // The greater student: the first taught, in bond order, whose best base beats the
    // teacher's best base. Wren only equals Odo's; Pip passes it.
    heroes[odo].destiny.kind = Destiny::TeachAGreater;
    heroes[odo].aptitudes = [3, 4, 5];
    assert_eq!(
        say(&heroes, "Odo"),
        "The Seer said he would teach a greater than himself, and he never learned another \
         thing."
    );
    let (wren, pip) = (id(&heroes, "Wren"), id(&heroes, "Pip"));
    heroes[wren].aptitudes = [5, 5, 5];
    heroes[pip].aptitudes = [6, 2, 2];
    // Brannoc, a student never taught, is greater still and does not count.
    let brannoc = id(&heroes, "Brannoc");
    heroes[brannoc].aptitudes = [9, 9, 9];
    for (student, taught) in [(brannoc, false), (wren, true), (pip, true)] {
        heroes[odo].bonds.push(Bond {
            kind: BondKind::Student,
            other: student,
            since: 1,
            taught,
            shared_successes: 0,
        });
    }
    assert_eq!(
        say(&heroes, "Odo"),
        "The Seer said he would teach a greater than himself, and he did: Pip learned from him \
         and went past him."
    );
}

#[test]
fn love_tells_spouses_and_children_else_the_last_friend_kin_or_not() {
    let (content, mut heroes) = founded();
    let (maren, pip, wren, odo, ysolde, brannoc) = (
        id(&heroes, "Maren"),
        id(&heroes, "Pip"),
        id(&heroes, "Wren"),
        id(&heroes, "Odo"),
        id(&heroes, "Ysolde"),
        id(&heroes, "Brannoc"),
    );
    let say = |heroes: &[Hero], name: &str, coin| part(&content, heroes, name, Part::Love, coin);
    assert_eq!(
        say(&heroes, "Garrick", true),
        "He loved Elsbeth Thorne, and Maren."
    );
    assert_eq!(say(&heroes, "Maren", true), "She loved her child, Pip.");
    assert_eq!(say(&heroes, "Ysolde", true), "");
    assert_eq!(
        (say(&heroes, "Odo", true), say(&heroes, "Odo", false)),
        (
            "His friend was Garrick Thorne.".to_owned(),
            "He had one friend, Garrick Thorne, and it was enough.".to_owned()
        )
    );
    form(&mut heroes, odo, brannoc, BondKind::Friend, 2);
    assert_eq!(say(&heroes, "Odo", true), "His friend was Brannoc Hale.");
    form(&mut heroes, wren, maren, BondKind::Parent, 2);
    heroes[wren].parents[0] = Some(maren);
    assert_eq!(
        say(&heroes, "Maren", true),
        "She loved her children, Pip and Wren."
    );
    form(&mut heroes, pip, wren, BondKind::Friend, 2);
    assert_eq!(
        say(&heroes, "Pip", true),
        "He was closest to his sister, Wren."
    );
    form(&mut heroes, ysolde, odo, BondKind::Spouse, 2);
    assert_eq!(say(&heroes, "Ysolde", true), "She loved Odo Fenn.");
}

#[test]
fn end_tells_the_living_the_door_and_the_dead_before_and_after_the_first_year() {
    let (content, mut heroes) = founded();
    assert_eq!(
        part(&content, &heroes, "Garrick", Part::End, true),
        "He was 62 when the story ended, and still living."
    );
    assert_eq!(
        part(&content, &heroes, "Pip", Part::End, true),
        "He was 10 when the story ended, and not yet grown."
    );
    assert_eq!(
        part(&content, &heroes, "Elsbeth", Part::End, true),
        "She was lost at the Drowned Coast, before the first year. She was 41."
    );
    let ysolde = id(&heroes, "Ysolde");
    heroes[ysolde]
        .deeds
        .push(deed(DeedKind::StoodAtTheDoor, 26, 49, None, 0));
    assert_eq!(
        part(&content, &heroes, "Ysolde", Part::End, true),
        "She went down to the Sealed Door at 49, and came home."
    );
    for lock in [0, 2] {
        let opened = deed(DeedKind::OpenedALock, 26, 49, None, lock);
        heroes[ysolde].deeds.push(opened);
    }
    assert_eq!(
        part(&content, &heroes, "Ysolde", Part::End, true),
        "She went down to the Sealed Door at 49, opened the lock of iron and the lock of breath, \
         and came home."
    );
    let garrick = id(&heroes, "Garrick");
    kill(&mut heroes, garrick, 3, 64, "fell at the Barrow");
    assert_eq!(
        both(&content, &heroes, "Garrick", Part::End),
        (
            "In year 3 he fell at the Barrow. He was 64.".to_owned(),
            "He fell at the Barrow in year 3, aged 64.".to_owned()
        )
    );
    // Year 1 is in play: no longer "before the first year".
    kill(&mut heroes, garrick, 1, 62, "fell at the Barrow");
    assert_eq!(
        part(&content, &heroes, "Garrick", Part::End, false),
        "He fell at the Barrow in year 1, aged 62."
    );
}

#[test]
fn left_tells_what_the_living_made_the_patron_and_what_the_dead_left_to_whom() {
    let (content, mut heroes) = founded();
    let (garrick, maren, ysolde) = (
        id(&heroes, "Garrick"),
        id(&heroes, "Maren"),
        id(&heroes, "Ysolde"),
    );
    let say = |heroes: &[Hero], name: &str| part(&content, heroes, name, Part::Left, true);
    assert_eq!(say(&heroes, "Pip"), "");
    assert_eq!(say(&heroes, "Garrick"), "He carried Thornfall to the end.");
    heroes[ysolde].legacy = (LegacyKind::Heirloom, "Ysolde's road-book".to_owned());
    assert_eq!(
        say(&heroes, "Ysolde"),
        "She made her own road-book, and it will be handed down."
    );
    heroes[ysolde].legacy = (LegacyKind::Tale, "The tale of Ysolde's roads".to_owned());
    assert_eq!(
        say(&heroes, "Ysolde"),
        "She made a tale that will outlast her: The tale of Ysolde's roads."
    );
    heroes[ysolde].legacy = (LegacyKind::Blessing, "Ysolde's luck".to_owned());
    assert_eq!(
        say(&heroes, "Ysolde"),
        "She laid a blessing on those who come after: Ysolde's luck."
    );
    // The dead: Garrick's heirloom, undecided, left to no one yet.
    kill(&mut heroes, garrick, 3, 64, "fell at the Barrow");
    heroes[garrick].bequest_heirloom = Some("Thornfall".to_owned());
    assert_eq!(say(&heroes, "Garrick"), "");
    heroes[garrick].bequest_decided = true;
    assert_eq!(say(&heroes, "Garrick"), "Thornfall was buried with him.");
    heroes[garrick].bequest_heirloom = Some("the Hale cradle-ring".to_owned());
    assert_eq!(
        say(&heroes, "Garrick"),
        "The Hale cradle-ring was buried with him."
    );
    heroes[garrick].bequest_heir = Some(maren);
    assert_eq!(
        say(&heroes, "Garrick"),
        "The Hale cradle-ring went to Maren."
    );
    heroes[garrick].legacy = (LegacyKind::Blessing, "Garrick's rest".to_owned());
    assert_eq!(
        say(&heroes, "Garrick"),
        "He left a blessing, Garrick's rest, and the Hale cradle-ring to Maren."
    );
    // With the heirloom buried, the legacy is told alone.
    heroes[garrick].bequest_heir = None;
    assert_eq!(
        say(&heroes, "Garrick"),
        "He left a blessing on those who come after: Garrick's rest."
    );
    heroes[garrick].bequest_heir = Some(maren);
    heroes[garrick].bequest_heirloom = None;
    assert_eq!(
        say(&heroes, "Garrick"),
        "He left a blessing on those who come after: Garrick's rest."
    );
    heroes[garrick].legacy = (LegacyKind::Tale, "The tale of Garrick's rest".to_owned());
    assert_eq!(
        say(&heroes, "Garrick"),
        "He left a tale behind: The tale of Garrick's rest."
    );
    heroes[garrick].legacy = (LegacyKind::Heirloom, "Garrick's road-book".to_owned());
    assert_eq!(
        say(&heroes, "Garrick"),
        "He made his own road-book, and it went to Maren."
    );
    // Made, and no heir to have it: nothing is said.
    heroes[garrick].bequest_heir = None;
    assert_eq!(say(&heroes, "Garrick"), "");
    heroes[garrick].fate = Fate::Departed;
    assert_eq!(
        say(&heroes, "Garrick"),
        "The house has had a patron at Court since."
    );
}
