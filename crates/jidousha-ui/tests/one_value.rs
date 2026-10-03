//! The one-value carriers, each asked the question its game asked: a chip's
//! line is derived from the row at the moment it is shown, a feed is the log
//! filtered by the config, and a meter's set is the set the simulation acts
//! on.
//!
//! Key functions: `burn`.
//! Depends on: `jidousha-ui`, `jidousha-core`.
//! INVARIANT: every expectation here is a literal, never arithmetic over the
//! thing under test.

use jidousha_core::Color;
use jidousha_ui::{
    Attention, Chip, ClassSpec, FeedEntry, MeterSpec, Mode, Pause, class_faults, count, faces,
    feed, find_class, meter_faults, reason_line, toggle,
};

// --- chips -------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TraitId {
    Caring,
    Restless,
}

/// The vocabulary the chip's line is read off — a row's name, which a rename
/// moves.
struct Row {
    id: TraitId,
    name: &'static str,
}

#[test]
fn a_chips_line_is_read_off_the_row_at_the_moment_it_is_shown() {
    let mut rows = vec![
        Row {
            id: TraitId::Caring,
            name: "caring",
        },
        Row {
            id: TraitId::Restless,
            name: "restless",
        },
    ];
    let explain = |rows: &[Row], id: TraitId| {
        rows.iter()
            .find(|row| row.id == id)
            .map_or_else(String::new, |row| format!("{} - what it does", row.name))
    };
    let mut chip = Chip::default();
    chip.toggle(TraitId::Restless);
    assert_eq!(
        chip.line(|id| explain(&rows, id)).as_deref(),
        Some("restless - what it does")
    );
    // Rename the row: the line on screen follows, because nothing stored it.
    rows[1].name = "wandering";
    assert_eq!(
        chip.line(|id| explain(&rows, id)).as_deref(),
        Some("wandering - what it does")
    );
}

#[test]
fn a_chip_tapped_on_two_surfaces_is_one_chip() {
    let mut chip = Chip::default();
    chip.toggle(TraitId::Caring);
    assert!(chip.showing(TraitId::Caring));
    // The same word on the other surface, tapped: it goes out everywhere.
    chip.toggle(TraitId::Caring);
    assert_eq!(chip, Chip::default());
    chip.toggle(TraitId::Caring);
    chip.toggle(TraitId::Restless);
    assert!(
        !chip.showing(TraitId::Caring),
        "another chip tapped replaces it"
    );
    chip.shut();
    assert_eq!(chip.lit, None);
}

#[test]
fn a_toggle_opens_replaces_and_shuts() {
    let mut board: Option<usize> = None;
    toggle(&mut board, 2);
    toggle(&mut board, 4);
    assert_eq!(board, Some(4));
    toggle(&mut board, 4);
    assert_eq!(board, None);
}

// --- the class table and the feed --------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    Departed,
    QuestComplete,
    AskDeclined,
    Unregistered,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Art {
    Tower,
    Coin,
    Skull,
}

const CLASSES: &[ClassSpec<Class, Art>] = &[
    ClassSpec {
        class: Class::Departed,
        id: "departed",
        color: Color::WHITE,
        icon: Art::Tower,
        default_mode: Mode::Ignore,
    },
    ClassSpec {
        class: Class::QuestComplete,
        id: "quest-complete",
        color: Color::MAGENTA,
        icon: Art::Coin,
        default_mode: Mode::Log,
    },
    ClassSpec {
        class: Class::AskDeclined,
        id: "ask-declined",
        color: Color::MAGENTA,
        icon: Art::Skull,
        default_mode: Mode::PauseAndFocus,
    },
];

#[test]
fn the_feed_is_the_log_filtered_by_the_config_newest_first() {
    let log = [
        Class::Departed,
        Class::QuestComplete,
        Class::Departed,
        Class::AskDeclined,
    ];
    let attention = Attention::opening(CLASSES);
    assert_eq!(
        feed(log.iter().copied(), &attention, false, 10),
        vec![
            FeedEntry {
                index: 3,
                ignored: false
            },
            FeedEntry {
                index: 1,
                ignored: false
            }
        ]
    );
    let shown = feed(log.iter().copied(), &attention, true, 10);
    assert_eq!(
        shown.len(),
        4,
        "with ignored classes shown, every entry is in"
    );
    assert!(shown[1].ignored && shown[3].ignored);
    assert_eq!(
        feed(log.iter().copied(), &attention, true, 1).len(),
        1,
        "the cap binds"
    );
    // And the same answer when the config changes: the feed holds nothing of its own.
    let mut quieter = attention.clone();
    quieter.set(Class::QuestComplete, Mode::Ignore);
    assert_eq!(
        feed(log.iter().copied(), &quieter, false, 10),
        vec![FeedEntry {
            index: 3,
            ignored: false
        }]
    );
}

#[test]
fn setting_one_mode_moves_one_mode_and_the_stamp_says_every_row() {
    let opening = Attention::opening(CLASSES);
    let mut set = opening.clone();
    set.set(Class::Departed, Mode::PauseAndFocus);
    assert_eq!(set.mode(Class::Departed), Mode::PauseAndFocus);
    assert_eq!(
        set.mode(Class::QuestComplete),
        opening.mode(Class::QuestComplete)
    );
    assert_ne!(set, opening);
    assert_eq!(
        set.stamp(),
        "attention:departed=pause,quest-complete=log,ask-declined=pause"
    );
    assert_eq!(set.table().len(), 3);
}

#[test]
#[should_panic(expected = "an event class has no row in its class table")]
fn a_class_with_no_row_is_refused_loudly() {
    let attention = Attention::opening(CLASSES);
    let _ = attention.mode(Class::Unregistered);
}

#[test]
fn a_row_is_found_by_its_class() {
    assert_eq!(
        find_class(CLASSES, Class::AskDeclined).map(|spec| spec.id),
        Some("ask-declined")
    );
    assert!(find_class(CLASSES, Class::Unregistered).is_none());
}

#[test]
fn a_well_formed_table_has_no_faults_and_a_duplicate_row_has_two() {
    let ground = Color::rgb(0.1, 0.1, 0.1);
    assert!(class_faults(CLASSES, ground).is_empty());
    let doubled: &[ClassSpec<Class, Art>] = &[CLASSES[0], CLASSES[0]];
    let faults: Vec<&str> = class_faults(doubled, ground)
        .iter()
        .map(|f| f.what)
        .collect();
    assert_eq!(
        faults,
        [
            "two event classes share an id",
            "an event class has more than one row in the table",
            "two event classes share an id",
            "an event class has more than one row in the table",
        ]
    );
}

#[test]
fn the_reason_line_names_what_stopped_the_world_and_where_before_the_sentence() {
    let pause = Pause {
        event: 3,
        class: Class::AskDeclined,
        minute: 480,
    };
    let line = reason_line(CLASSES[2].id, "the Deep Cave", "Bob would rather not");
    assert_eq!(
        line,
        "paused: ask-declined at the Deep Cave - Bob would rather not"
    );
    assert_eq!(pause.class, Class::AskDeclined);
}

// --- meters ------------------------------------------------------------------

/// A camp with purses, and an upkeep it is about to charge.
struct Camp {
    wallets: Vec<i64>,
    shortfalls: Vec<u32>,
}

const DUE: i64 = 4;

fn short(camp: &Camp, who: usize) -> Option<String> {
    (camp.wallets[who] < DUE).then(|| format!("holds {}g of the {DUE}g due", camp.wallets[who]))
}

fn idle(_camp: &Camp, _who: usize) -> Option<String> {
    Some("at home".to_owned())
}

/// The question a meter asks of one person, over this camp.
type Ask = fn(&Camp, usize) -> Option<String>;

const METERS: &[MeterSpec<Art, Ask>] = &[
    MeterSpec {
        id: "short",
        label: "short",
        icon: Art::Coin,
        asks: short,
    },
    MeterSpec {
        id: "idle",
        label: "idle",
        icon: Art::Tower,
        asks: idle,
    },
];

/// The simulation's own act: charge everybody, and count a shortfall where
/// the purse does not cover it.
fn burn(camp: &mut Camp) {
    for who in 0..camp.wallets.len() {
        if camp.wallets[who] < DUE {
            camp.shortfalls[who] += 1;
        } else {
            camp.wallets[who] -= DUE;
        }
    }
}

#[test]
fn a_meters_count_is_the_length_of_the_list_it_opens_into() {
    let camp = Camp {
        wallets: vec![10, 2, 0, 7],
        shortfalls: vec![0; 4],
    };
    let roll = 0..4;
    let listed = faces(roll.clone(), |who| (METERS[0].asks)(&camp, who));
    assert_eq!(listed.len(), 2);
    assert_eq!(
        count(roll, |who| (METERS[0].asks)(&camp, who)),
        listed.len()
    );
    assert_eq!(listed[0], (1, "holds 2g of the 4g due".to_owned()));
}

#[test]
fn a_meters_set_is_the_set_the_simulation_acts_on() {
    // The one-source assertion: the chip is asked first, then the burn is
    // actually run, and the people it went short on are the people the chip
    // named — asserted against the act, not against a second reading of the
    // same predicate.
    let mut camp = Camp {
        wallets: vec![10, 2, 0, 7],
        shortfalls: vec![0; 4],
    };
    let named: Vec<usize> = faces(0..4, |who| (METERS[0].asks)(&camp, who))
        .into_iter()
        .map(|(who, _)| who)
        .collect();
    assert_eq!(
        named,
        vec![1, 2],
        "a one-source assertion over an empty set passes vacuously"
    );
    let before = camp.shortfalls.clone();
    burn(&mut camp);
    let acted: Vec<usize> = (0..4)
        .filter(|who| camp.shortfalls[*who] > before[*who])
        .collect();
    assert_eq!(acted, named);
}

#[test]
fn a_meter_table_is_checked_before_any_world_exists() {
    assert!(meter_faults(METERS).is_empty());
    let clash: &[MeterSpec<Art, ()>] = &[
        MeterSpec {
            id: "short",
            label: "short",
            icon: Art::Coin,
            asks: (),
        },
        MeterSpec {
            id: "Idle",
            label: "idle",
            icon: Art::Coin,
            asks: (),
        },
    ];
    let faults: Vec<&str> = meter_faults(clash).iter().map(|f| f.what).collect();
    assert_eq!(
        faults,
        [
            "two meter chips carry the same icon",
            "a meter id is not stamp-shaped ASCII",
            "two meter chips carry the same icon",
        ]
    );
}
