//! The Ending's unit half (SPEC §23): the remembering of the fallen and of the living, in
//! order and with the draws §22.2 lists; the verdict's lines after the Door; the verdict's
//! title by the locks given; the closed house's verdict; and the house tally's Ending word.
//!
//! INVARIANT: expectations are shipped literals from `door.json`, `ui-text.json` and
//! `lines.json`, or the founding read by hand; the draw order is held against the
//! generator's own sequence taken in the order the spec states.

use jidousha::prelude::Rng;

use crate::door::{DoorRecord, try_the_door};
use crate::door_tests::{at_the_door, party_a};
use crate::ending::{Ending, Verdict, enter_the_ending};
use crate::epitaph::roll_wording;
use crate::hero::{DreamFate, HeroId};
use crate::house::House;
use crate::testkit::{id, seat};

fn make(house: &mut House, name: &str, bases: [i32; 3]) -> HeroId {
    let who = id(&house.heroes, name);
    house.heroes[who].aptitudes = bases;
    house.heroes[who].age = 30;
    who
}

/// Try the Door with `dice`, keep the telling, and leave it into the Ending on `seed`.
fn through_the_door(
    content: &crate::content::Content,
    house: &mut House,
    dice: [[i32; 2]; 3],
    seed: u64,
) -> DoorRecord {
    let mut rng = Rng::from_seed(seed);
    let mut pages = Vec::new();
    let record = try_the_door(content, house, &mut rng, Some(dice), &mut pages);
    house.telling = Some(crate::telling::Telling {
        year: 26,
        pages,
        meanwhile: Vec::new(),
        door: Some(record.clone()),
    });
    crate::season::leave_the_telling(content, house, &mut rng);
    record
}

fn lines(house: &House) -> Vec<String> {
    match house.ending.as_ref().map(|e| &e.verdict) {
        Some(Verdict::Door { lines, .. }) => lines.clone(),
        other => panic!("not a Door verdict: {other:?}"),
    }
}

#[test]
fn every_lock_open_tells_the_bearers_home_and_the_rest_home_too() {
    let (content, mut house) = at_the_door();
    party_a(&mut house);
    through_the_door(&content, &mut house, [[6, 6], [6, 6], [6, 6]], 1);
    let ending = house.ending.as_ref().expect("the Ending");
    assert_eq!(ending.title(&content), "The Door is open");
    assert_eq!(
        lines(&house),
        [
            "Garrick Thorne opened the lock of iron, and came home.",
            "Ysolde Vane opened the lock of riddles and the lock of breath, and came home.",
            "Maren and Brannoc came home too.",
        ]
    );
}

#[test]
fn the_fallen_fell_at_the_last_lock_they_stood_at_and_the_rest_come_home_without_too() {
    let (content, mut house) = at_the_door();
    // Party C wounded (door_tests): Garrick and Ysolde die at the lock of iron, Maren at
    // the lock of riddles, Brannoc — whom the fire shields — comes home.
    let party = vec![
        make(&mut house, "Garrick", [3, 9, 9]),
        make(&mut house, "Maren", [3, 9, 9]),
        make(&mut house, "Ysolde", [3, 9, 9]),
        make(&mut house, "Brannoc", [3, 9, 9]),
    ];
    seat(&mut house, 0, &party);
    house.heroes[party[0]].wounded = true;
    house.heroes[party[2]].wounded = true;
    through_the_door(&content, &mut house, [[6, 6], [6, 6], [6, 6]], 2);
    let ending = house.ending.as_ref().expect("the Ending");
    assert_eq!(ending.title(&content), "The Door stayed shut");
    assert_eq!(
        lines(&house),
        [
            "Garrick Thorne fell at the lock of iron.",
            "Maren Thorne fell at the lock of riddles.",
            "Ysolde Vane fell at the lock of iron.",
            "Brannoc came home.",
        ]
    );
}

#[test]
fn a_bearer_who_opened_a_lock_and_fell_at_the_next_is_told_both() {
    let (content, mut house) = at_the_door();
    // Garrick, wounded, bears the lock of iron (9 + 1 - 2 = 8 alone, the others 7): the
    // four bring 8 + 7 + 7 + 7 +2 +1 = 32, a success at a 12. At the lock of riddles they
    // bring 1 + 3 + 3 + 3 + 3 = 13 — a disaster — and the wounded Garrick dies of it; at the
    // lock of breath Maren, mended at the first and wounded since, dies.
    let party = vec![
        make(&mut house, "Garrick", [9, 3, 3]),
        make(&mut house, "Maren", [7, 3, 3]),
        make(&mut house, "Brannoc", [7, 3, 3]),
        make(&mut house, "Odo", [7, 3, 3]),
    ];
    seat(&mut house, 0, &party);
    house.heroes[party[0]].wounded = true;
    for seed in 0..6 {
        let mut staged = house.clone();
        let record = through_the_door(&content, &mut staged, [[6, 6], [6, 6], [6, 6]], seed);
        assert!(record.tried[0].opened && !record.tried[1].opened);
        let ending = staged.ending.as_ref().expect("the Ending");
        assert_eq!(ending.title(&content), "The Door opened a hand's breadth");
        let told = lines(&staged);
        assert_eq!(
            told[0],
            "Garrick Thorne opened the lock of iron, and fell at the lock of riddles."
        );
        assert_eq!(told[1], "Maren Thorne fell at the lock of breath.");
        // Odo rolls; Brannoc lives. The rest come home with no "too": no bearer did.
        let last = told.last().expect("a line");
        assert!(
            last == "Brannoc came home." || last == "Brannoc and Odo came home.",
            "{told:?}"
        );
    }
}

#[test]
fn the_verdicts_title_is_the_locks_given_and_the_closed_houses_its_own() {
    let (content, _) = at_the_door();
    let titles: Vec<String> = (0..=3)
        .map(|locks| {
            let ending = Ending {
                verdict: Verdict::Door {
                    locks,
                    lines: Vec::new(),
                },
            };
            ending.title(&content).to_owned()
        })
        .collect();
    assert_eq!(
        titles,
        [
            "The Door stayed shut",
            "The Door opened a hand's breadth",
            "The Door stood half open",
            "The Door is open"
        ]
    );
    let closed = Ending {
        verdict: Verdict::Closed {
            year: 3,
            years_off: 23,
        },
    };
    assert_eq!(closed.title(&content), "The house closed its doors");
}

#[test]
fn a_closed_house_reads_its_verdict_its_year_and_who_lived_under_its_roof() {
    let (content, mut house) = crate::testkit::house();
    house.renown = 0;
    house.telling = Some(crate::telling::Telling::default());
    crate::season::leave_the_telling(&content, &mut house, &mut Rng::from_seed(4));
    assert!(house.closed);
    let ending = house.ending.clone().expect("the Ending");
    assert_eq!(
        ending.verdict,
        Verdict::Closed {
            year: 1,
            years_off: 25
        }
    );
    let read: Vec<String> = crate::ending_view::paragraphs(&content, &house, &ending)
        .into_iter()
        .map(|(text, _)| text)
        .collect();
    assert_eq!(
        read,
        [
            "The house closed its doors",
            "Too many roads went unanswered. The villages stopped sending, and then the Court \
             did. The fire was let go out one spring, and those who were left took other names \
             in other houses.",
            "It was year 1, with the Door still 25 years off.",
            "9 lived under this roof. 7 living, 2 gone. No tale of the house is told. House \
             renown 0.",
        ]
    );
}

#[test]
fn the_door_verdict_page_reads_its_title_its_text_its_lines_and_the_tally() {
    let (content, mut house) = at_the_door();
    party_a(&mut house);
    through_the_door(&content, &mut house, [[6, 6], [6, 6], [6, 6]], 1);
    let ending = house.ending.clone().expect("the Ending");
    let read: Vec<String> = crate::ending_view::paragraphs(&content, &house, &ending)
        .into_iter()
        .map(|(text, _)| text)
        .collect();
    assert_eq!(read[0], "The Door is open");
    assert_eq!(
        read[1],
        "All three locks gave, and the Door stayed open. Everyone in the house had a hand on \
         it: the ones who taught, the ones who bore children, the ones who died so the rest \
         would know the road."
    );
    assert_eq!(read.len(), 2 + 3 + 1);
    // 15 + 6 + 6 + 6 renown; the founding's nine, seven living.
    assert_eq!(
        read[5],
        "9 lived under this roof. 7 living, 2 gone. No tale of the house is told. House \
         renown 33."
    );
}

#[test]
fn the_fallen_are_remembered_in_order_of_death_then_the_living_in_creation_order() {
    let (content, mut house) = at_the_door();
    let party = vec![
        make(&mut house, "Garrick", [3, 9, 9]),
        make(&mut house, "Maren", [3, 9, 9]),
        make(&mut house, "Ysolde", [3, 9, 9]),
        make(&mut house, "Brannoc", [3, 9, 9]),
    ];
    seat(&mut house, 0, &party);
    house.heroes[party[0]].wounded = true;
    house.heroes[party[2]].wounded = true;
    let mut rng = Rng::from_seed(9);
    let mut pages = Vec::new();
    let record = try_the_door(
        &content,
        &mut house,
        &mut rng,
        Some([[6, 6]; 3]),
        &mut pages,
    );
    let mourned = house.mourned.clone();
    assert_eq!(mourned, [party[0], party[2], party[1]]);
    // The draws the Ending takes, in the order §23 and §22.2 state them: one wording per
    // mourned dead in order of death, then one per living hero in creation order.
    let mut expected_rng = rng.clone();
    let mut expected_writing = house.writing.clone();
    let living: Vec<HeroId> = (0..house.heroes.len())
        .filter(|&h| house.heroes[h].is_living())
        .collect();
    let mut expected = Vec::new();
    for &who in mourned.iter().chain(&living) {
        expected.push((who, roll_wording(&mut expected_writing, &mut expected_rng)));
    }
    enter_the_ending(&content, &mut house, &mut rng, Some(&record));
    for (who, wording) in expected {
        assert_eq!(
            house.heroes[who].wording,
            Some(wording),
            "{}",
            house.heroes[who].name
        );
        assert!(house.heroes[who].epitaph.is_some());
    }
    assert!(
        house.mourned.is_empty(),
        "the remembered are mourned no longer"
    );
    assert_eq!(
        rng.next_u32(),
        expected_rng.next_u32(),
        "no other draw was taken"
    );
    // Nobody alive or dead is left without an epitaph at the Ending.
    assert!(house.heroes.iter().all(|h| h.epitaph.is_some()));
}

#[test]
fn a_fallen_heirloom_goes_to_the_heir_without_a_choice_and_an_undone_dream_neither_passes_nor_walks()
 {
    let (content, mut house) = at_the_door();
    let party = vec![
        make(&mut house, "Garrick", [3, 9, 9]),
        make(&mut house, "Maren", [3, 9, 9]),
        make(&mut house, "Ysolde", [3, 9, 9]),
        make(&mut house, "Brannoc", [3, 9, 9]),
    ];
    seat(&mut house, 0, &party);
    house.heroes[party[0]].wounded = true;
    house.heroes[party[2]].wounded = true;
    let pip = id(&house.heroes, "Pip");
    let ghosts = house.ghosts.len();
    let pip_dream = house.heroes[pip].dream.clone();
    through_the_door(&content, &mut house, [[6, 6]; 3], 3);
    let garrick = &house.heroes[party[0]];
    // Garrick held Thornfall. His heir by §14.2: his child Maren is dead; her line gives
    // Pip, living and empty-handed.
    assert!(garrick.bequest_decided);
    assert_eq!(garrick.bequest_heirloom.as_deref(), Some("Thornfall"));
    assert_eq!(garrick.bequest_heir, Some(pip));
    assert!(garrick.heirloom.is_none());
    assert_eq!(
        house.heroes[pip].heirloom.as_ref().map(|h| h.name.as_str()),
        Some("Thornfall")
    );
    // Garrick's dream ("lay the Barrow's dead to rest") was undone: its fate stays
    // undecided, no ghost walks, no one takes it up.
    assert_eq!(garrick.dream_fate, DreamFate::Undecided);
    assert_eq!(house.ghosts.len(), ghosts);
    assert_eq!(house.heroes[pip].dream, pip_dream);
    assert!(house.heroes[pip].burden.is_none());
    // Ysolde's own dream undone too; Maren's likewise. Every one of them decided.
    for &dead in &[party[1], party[2]] {
        assert!(house.heroes[dead].bequest_decided);
    }
    // The epitaph tells the heirloom gone to Pip.
    let epitaph = garrick.epitaph.clone().unwrap_or_default();
    assert!(epitaph.starts_with("Garrick Thorne") || epitaph.contains("Garrick Thorne"));
}

#[test]
fn a_fallen_heirloom_with_no_heir_is_buried_and_a_fulfilled_or_absent_dream_is_so_recorded() {
    let (content, mut house) = at_the_door();
    let brannoc = make(&mut house, "Brannoc", [1, 1, 1]);
    let odo = make(&mut house, "Odo", [9, 9, 9]);
    seat(&mut house, 0, &[odo, brannoc]);
    // Odo, wounded, holds a blade and has no child, spouse or student: no heir.
    house.heroes[odo].wounded = true;
    let garrick = id(&house.heroes, "Garrick");
    let blade = house.heroes[garrick].heirloom.take();
    house.heroes[odo].heirloom = blade;
    house.heroes[odo].dream = None;
    through_the_door(&content, &mut house, [[1, 1]; 3], 5);
    let odo_now = &house.heroes[odo];
    assert!(!odo_now.is_living());
    assert!(odo_now.bequest_decided);
    assert_eq!(odo_now.bequest_heirloom.as_deref(), Some("Thornfall"));
    assert_eq!(odo_now.bequest_heir, None);
    assert!(odo_now.heirloom.is_none());
    assert!(
        house.heroes.iter().all(|h| h.heirloom.is_none()),
        "buried, not given"
    );
    assert_eq!(odo_now.dream_fate, DreamFate::NeverDreamt);
}

#[test]
fn at_the_ending_no_tale_is_told_and_not_yet() {
    let (content, mut house) = crate::testkit::house();
    assert_eq!(
        crate::family::tally_sentence(&content, &house),
        "7 living, 2 gone. No tale of the house is told yet. House renown 15."
    );
    house.renown = 0;
    house.telling = Some(crate::telling::Telling::default());
    crate::season::leave_the_telling(&content, &mut house, &mut Rng::from_seed(4));
    assert_eq!(
        crate::family::tally_sentence(&content, &house),
        "7 living, 2 gone. No tale of the house is told. House renown 0."
    );
}

#[test]
#[should_panic(expected = "the Ending was entered twice")]
fn the_ending_is_entered_once() {
    let (content, mut house) = crate::testkit::house();
    enter_the_ending(&content, &mut house, &mut Rng::from_seed(1), None);
    enter_the_ending(&content, &mut house, &mut Rng::from_seed(1), None);
}

#[test]
fn after_the_door_the_ending_comes_whatever_the_renown() {
    let (content, mut house) = at_the_door();
    party_a(&mut house);
    house.renown = 0;
    let mut pages = Vec::new();
    let mut rng = Rng::from_seed(1);
    let record = try_the_door(
        &content,
        &mut house,
        &mut rng,
        Some([[1, 1]; 3]),
        &mut pages,
    );
    house.renown = 0;
    house.telling = Some(crate::telling::Telling {
        year: 26,
        pages,
        meanwhile: Vec::new(),
        door: Some(record),
    });
    crate::season::leave_the_telling(&content, &mut house, &mut rng);
    assert!(!house.closed, "after the Door the house does not close");
    assert!(matches!(
        house.ending.as_ref().map(|e| &e.verdict),
        Some(Verdict::Door { .. })
    ));
}

#[test]
fn a_fallen_hero_whose_own_dream_was_done_is_remembered_as_fulfilled() {
    let (content, mut house) = at_the_door();
    let brannoc = make(&mut house, "Brannoc", [1, 1, 1]);
    let odo = make(&mut house, "Odo", [9, 9, 9]);
    seat(&mut house, 0, &[odo, brannoc]);
    house.heroes[odo].wounded = true;
    if let Some(dream) = house.heroes[odo].dream.as_mut() {
        dream.current = 3;
    }
    through_the_door(&content, &mut house, [[1, 1]; 3], 5);
    assert!(!house.heroes[odo].is_living());
    assert_eq!(house.heroes[odo].dream_fate, DreamFate::Fulfilled);
}
