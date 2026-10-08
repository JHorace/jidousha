//! W10's Ending, played through the screen: all four verdict titles by staged lock
//! outcomes, the remembering of the fallen and the living, the family at the Ending, and
//! "Begin another house" — the reset to the authored founding, with a fresh seed and an
//! unbroken chronicle, which a replay of the same presses reproduces.
//!
//! **The four titles.** Each lock's outcome is staged by the four's power, not their dice:
//! a lock is certain to give at 39 or more (the worst dice, 2, still reach 0) and certain
//! not to at 28 or less (the best, 12, stay under). Party A's four made 9/9/9 bring 41, 40
//! and 40; made 9/9/1 they bring 41, 40 and 8; 9/1/1, 41, 8 and 8; 1/1/1, 9, 8 and 8 — so
//! the Door gives three, two, one and no locks whatever the dice roll, and the verdict's
//! title is read off the page after "Try the Door", "Skip ahead" and nothing else.
//!
//! **The remembering.** Before the telling is left, the generator and the dead waiting
//! are copied; the wordings the Ending gives are required to be the generator's next
//! draws taken for the fallen in order of death, then the living in creation order (SPEC
//! §23, §22.2). On the family, every hero's remembrance is their name and an epitaph of
//! at most six sentences.
//!
//! **The reset.** "Begin another house": the house is year 1's as authored — the W0 and W1
//! oracles' literals hold on it — on the seed drawn from the generator (recorded), and the
//! chronicle holds the ended house and the new one. The same presses on the same seed, in a
//! second session, reach the same chronicle and the same house.
//!
//! INVARIANT: titles, texts and seeds are shipped literals.

use jidousha::prelude::*;
use jidousha::testing::FrameRecorder;

use crate::checks::Checks;
use crate::epitaph::{roll_wording, sentences};
use crate::house::{Chronicle, Founded, House};
use crate::screen::{Target, UiState};
use crate::scripted::lines_in;
use crate::verify::{SEEDS, page_of, point_at, session};
use crate::w10::{TITLES, drag_onto_the_door, make, stage_the_last_summer, verdict_lines};

/// Each staging's bases for party A's four, and the locks it gives.
pub const STAGED: [([i32; 3], usize); 4] = [
    ([9, 9, 9], 3),
    ([9, 9, 1], 2),
    ([9, 1, 1], 1),
    ([1, 1, 1], 0),
];

/// The verdict's text by locks given (`door.json`).
pub const TEXTS: [&str; 4] = [
    "The house came to the Door with the wrong four, or too few, or too late. It is sealed again, and will not open in the lifetime of anyone now living. What the house has instead is its names.",
    "One lock gave. Through the gap came cold air and a smell of rain on stone, and then the Door closed on it. It was enough to know there is another side. It was not enough to reach it.",
    "Two locks gave, and the Door stood half open for a night and a day. Those who were there saw lamps on the far side, and someone carrying one. The house will be ready next time, if there is a house.",
    "All three locks gave, and the Door stayed open. Everyone in the house had a hand on it: the ones who taught, the ones who bore children, the ones who died so the rest would know the road.",
];

/// The seed "Begin another house" draws after the Door opened all three on `SEEDS[0]`
/// under `STAGED[0]`: the engine `Rng`'s sequence, recorded the first time it ran.
const SEED_AFTER_THE_DOOR: u64 = 0x3377_94d8_d34b_c82e;

/// The last summer on `seed`, party A's four made `bases` and dragged onto the Door.
pub fn stage_the_door(seed: u64, bases: [i32; 3]) -> HeadlessSim {
    let mut sim = session(seed);
    stage_the_last_summer(&mut sim);
    let party = ["Garrick", "Maren", "Ysolde", "Brannoc"].map(|name| make(&mut sim, name, bases));
    for (seat, &who) in party.iter().enumerate() {
        drag_onto_the_door(&mut sim, who, seat);
    }
    sim
}

/// "Try the Door", then "Skip ahead": the Ending.
pub fn try_and_leave(sim: &mut HeadlessSim) {
    point_at(sim, Target::SetOut, true);
    point_at(sim, Target::Skip, true);
}

/// The four titles, each by its staging, on every recorded seed. Returns a summary line.
pub fn check_titles(checks: &mut Checks) -> String {
    let mut read = Vec::new();
    for seed in SEEDS {
        for (bases, locks) in STAGED {
            let mut sim = stage_the_door(seed, bases);
            try_and_leave(&mut sim);
            let page = verdict_lines(&sim);
            let ending = sim.world().resource::<House>().ending.clone();
            checks.require(
                page.first().map(String::as_str) == Some(TITLES[locks])
                    && page.get(1).map(String::as_str) == Some(TEXTS[locks]),
                "W10: a staged Door's verdict does not read the title and text of the locks it gave",
                format!("seed {seed:#x}, {bases:?}: wanted {:?}; the page reads {page:?}, {ending:?}", TITLES[locks]),
            );
            if seed == SEEDS[0] {
                read.push(page.first().cloned().unwrap_or_default());
            }
        }
    }
    format!(
        "W10 verdicts, staged by the four's power on {} seeds: {read:?} for three, two, one and no locks given",
        SEEDS.len()
    )
}

/// The remembering, the family at the Ending, the way back, and the reset. Returns the
/// summary lines.
pub fn check_ending(checks: &mut Checks, recorder: &mut FrameRecorder) -> Vec<String> {
    let mut sim = stage_the_door(SEEDS[0], STAGED[0].0);
    point_at(&mut sim, Target::SetOut, true);
    // The generator and the dead waiting, as the telling is left.
    let (waiting, living, mut rng, mut writing) = {
        let house = sim.world().resource::<House>();
        let living: Vec<usize> = (0..house.heroes.len())
            .filter(|&h| house.heroes[h].is_living())
            .collect();
        (
            house.mourned.clone(),
            living,
            sim.world().resource::<Rng>().clone(),
            house.writing.clone(),
        )
    };
    point_at(&mut sim, Target::Skip, true);
    let mut order_kept = true;
    {
        let house = sim.world().resource::<House>();
        for &who in waiting.iter().chain(&living) {
            order_kept &= house.heroes[who].wording == Some(roll_wording(&mut writing, &mut rng));
        }
    }
    checks.require(
        order_kept,
        "W10: the Ending did not remember the fallen in order of death, then the living in creation order",
        format!("fallen {waiting:?}, living {living:?}"),
    );
    // The family at the Ending: its heading, every node remembered by an epitaph.
    point_at(&mut sim, Target::OpenFamily, true);
    let page = page_of(&sim);
    let heading = lines_in(&page, crate::summer::screen_rect())
        .first()
        .cloned()
        .unwrap_or_default();
    checks.require(
        heading == "Everyone who lived under this roof",
        "W10: the Ending's family does not open under its own heading",
        format!("{heading:?}"),
    );
    let close = crate::verify::target_rect(&sim, Target::CloseFamily)
        .map(|rect| lines_in(&page, rect))
        .unwrap_or_default();
    checks.require(
        close == ["The verdict"],
        "W10: the Ending's family does not offer \"The verdict\"",
        format!("{close:?}"),
    );
    let everyone = sim.world().resource::<House>().heroes.len();
    let mut counts = [0usize; 7];
    for id in 0..everyone {
        point_at(&mut sim, Target::Hero(id), false);
        let lines = lines_in(&page_of(&sim), crate::tree::REMEMBRANCE);
        let name = sim.world().resource::<House>().heroes[id].full_name();
        let epitaph = lines.get(1).cloned().unwrap_or_default();
        let count = sentences(&epitaph);
        counts[count.min(6)] += 1;
        checks.require(
            lines.first() == Some(&name) && count > 0 && count <= 6 && epitaph.contains(&name),
            "W10: someone at the Ending is not remembered by a named epitaph of at most six sentences",
            format!("{name}: {lines:?}"),
        );
    }
    let _ = crate::verify::frame(recorder, &mut sim);
    point_at(&mut sim, Target::CloseFamily, true);
    checks.require(
        verdict_lines(&sim).first().map(String::as_str) == Some(TITLES[3]),
        "W10: \"The verdict\" does not return to the verdict",
        format!("{:?}", verdict_lines(&sim)),
    );
    let reset = check_reset(checks, recorder, sim);
    vec![
        format!(
            "W10 the Ending (seed {:#x}, all three open): {} fallen then {} living remembered in order, every one of {everyone} on the family named by an epitaph (sentences 0..6: {counts:?}); \"The verdict\" returns",
            SEEDS[0],
            waiting.len(),
            living.len()
        ),
        reset,
    ]
}

/// "Begin another house" on the ended `sim`, and its replay. Returns a summary line.
fn check_reset(checks: &mut Checks, recorder: &mut FrameRecorder, mut sim: HeadlessSim) -> String {
    point_at(&mut sim, Target::BeginAgain, true);
    let (drawn, chronicle, calendar, ended, telling) = {
        let house = sim.world().resource::<House>();
        (
            house.seed,
            sim.world()
                .find_resource::<Chronicle>()
                .cloned()
                .unwrap_or_default(),
            house.calendar,
            house.ending.is_some() || house.closed,
            house.telling.is_some() || house.passage.is_some(),
        )
    };
    let want = Chronicle(vec![
        Founded {
            seed: SEEDS[0],
            ended: Some((TITLES[3].to_owned(), 26)),
        },
        Founded {
            seed: SEED_AFTER_THE_DOOR,
            ended: None,
        },
    ]);
    checks.require(
        drawn == SEED_AFTER_THE_DOOR && chronicle == want,
        "W10: another house did not draw the recorded seed, or the chronicle broke",
        format!("drew {drawn:#x}; chronicle {chronicle:?}"),
    );
    checks.require(
        calendar.current_year() == 1 && !calendar.is_winter() && !ended && !telling,
        "W10: another house is not year 1's summer, fresh",
        format!("{calendar:?}, ended {ended}, telling {telling}"),
    );
    checks.require(
        *sim.world().resource::<UiState>()
            == UiState {
                typing_from: sim.world().resource::<UiState>().typing_from,
                ..UiState::default()
            },
        "W10: another house does not open on a fresh screen",
        format!("{:?}", sim.world().resource::<UiState>()),
    );
    // The W0 and W1 oracles' literals hold on the new house.
    let _ = crate::oracles::check_w0_and_w1_on(checks, recorder, &mut sim, drawn);
    // A replay: the same presses on the same seed.
    let mut again = stage_the_door(SEEDS[0], STAGED[0].0);
    try_and_leave(&mut again);
    point_at(&mut again, Target::OpenFamily, true);
    point_at(&mut again, Target::CloseFamily, true);
    point_at(&mut again, Target::BeginAgain, true);
    let same = {
        let (a, b) = (
            sim.world().resource::<House>(),
            again.world().resource::<House>(),
        );
        a.seed == b.seed && a.heroes == b.heroes && a.calendar == b.calendar
    } && again.world().find_resource::<Chronicle>()
        == sim.world().find_resource::<Chronicle>();
    checks.require(
        same,
        "W10: a replay of the same presses on the same seed does not reach the same new house",
        format!("{:?}", again.world().find_resource::<Chronicle>()),
    );
    format!(
        "W10 reset: \"Begin another house\" drew {drawn:#x} from the generator; the chronicle reads the open Door, then the new house; W0's top bar and W1's Garrick hold on it; a replay reaches the same"
    )
}
