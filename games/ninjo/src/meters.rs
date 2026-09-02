//! Meters and faces: the aggregates for the glance, and the faces behind them
//! (GDD §3, wave 0a).
//!
//! **A chip is a count of people, and every count opens into the people it
//! counted.** That is the whole rule: never a bare percentage, never a number
//! whose subjects the player cannot reach. A chip's row carries the question
//! it asks of one character, and the answer is a *reason* — the sentence the
//! faces list shows beside the portrait — so the drill-down is the same
//! derivation as the chip rather than a second one that could disagree.
//!
//! **The chips are data-registered** ([`METERS`]), like the event classes:
//! wave 1's modules add "hungry" and "petitions pending" as rows here, with a
//! question and an icon, and nothing that draws or hit-tests a chip changes.
//!
//! **The truths are the sim's own.** `idle` and `away` are `Lens::at_home` —
//! the same derivation the map draws a character's figure with — so a chip
//! cannot say four people are home while the map draws three.

use crate::lens::Lens;
use crate::sprites::Art;

/// One registered aggregate.
#[derive(Clone, Copy, Debug)]
pub struct MeterSpec {
    /// The id a stamp and a report name it by. ASCII, lowercase.
    pub id: &'static str,
    /// What the chip says.
    pub label: &'static str,
    /// The chip's icon role — a second channel beside the colour (UI.md §1).
    pub icon: Art,
    /// The question the chip asks of one character: `Some(reason)` when they
    /// count, and the reason is what the faces list shows beside them.
    pub asks: fn(&Lens<'_>, usize) -> Option<String>,
}

/// Every chip above the map.
///
/// Two rows, because two are what wave 0b's world can honestly answer:
/// everyone is either standing at home or out with a party. The chips that
/// matter — hungry, petitions pending, in debt — arrive as rows here with the
/// modules that make them true (GDD §5).
pub const METERS: &[MeterSpec] = &[
    MeterSpec {
        id: "idle",
        label: "idle",
        icon: Art::Heart,
        asks: idle,
    },
    MeterSpec {
        id: "away",
        label: "away",
        icon: Art::QuestTower,
        asks: away,
    },
];

/// Standing at home with nothing asked of them.
fn idle(lens: &Lens<'_>, who: usize) -> Option<String> {
    lens.at_home(who)
        .then(|| "at home, and nobody has asked them for anything".to_owned())
}

/// Out of the settlement, and why.
fn away(lens: &Lens<'_>, who: usize) -> Option<String> {
    if lens.at_home(who) {
        return None;
    }
    Some(lens.activity_line(who))
}

/// The faces behind chip `index`: who counts, and why.
pub fn faces(lens: &Lens<'_>, index: usize) -> Vec<(usize, String)> {
    let Some(spec) = METERS.get(index) else {
        return Vec::new();
    };
    (0..lens.people().len())
        .filter_map(|who| (spec.asks)(lens, who).map(|reason| (who, reason)))
        .collect()
}

/// How many people chip `index` counts.
pub fn count(lens: &Lens<'_>, index: usize) -> usize {
    faces(lens, index).len()
}

/// The chips' own validation, plus the claim that makes them trustworthy:
/// **every chip's count is the length of the list it opens into**, and the
/// two chips this build has partition the cast between them.
pub fn registry(checks: &mut crate::checks::Checks, tuning: &crate::constants::Tuning) {
    for (index, spec) in METERS.iter().enumerate() {
        checks.require(
            !spec.id.is_empty()
                && spec
                    .id
                    .chars()
                    .all(|glyph| glyph.is_ascii_lowercase() || glyph == '-'),
            "a meter id is not stamp-shaped ASCII",
            format!("METERS[{index}] is named {:?}", spec.id),
        );
        checks.require(
            METERS.iter().filter(|other| other.id == spec.id).count() == 1,
            "two meters share an id",
            format!("{:?} appears more than once in METERS", spec.id),
        );
        let texels = spec.icon.texels();
        checks.require(
            texels.width == texels.height
                && (crate::attention::CHIP as u32).is_multiple_of(texels.width),
            "a meter chip's icon is not a square whole-scale picture",
            format!(
                "{:?} carries {:?}, which is {}x{} texels",
                spec.id, spec.icon, texels.width, texels.height
            ),
        );
        checks.require(
            METERS
                .iter()
                .filter(|other| other.icon == spec.icon)
                .count()
                == 1,
            "two meter chips carry the same icon",
            format!(
                "{:?} draws {:?}, and a chip's picture is how it is told from its neighbour",
                spec.id, spec.icon
            ),
        );
    }
    // A world where one party is out: the two chips must divide the cast, and
    // every count must be the length of the list it opens into.
    let mut sim = crate::sim::Sim::opening(tuning, crate::modules::ModuleSet::ALL);
    sim.parties[0].activity = crate::sim::Activity::Working { until: 99 };
    let lens = Lens::on(&sim);
    let mut counted = 0usize;
    for (index, spec) in METERS.iter().enumerate() {
        let faces = faces(&lens, index);
        counted += faces.len();
        checks.require(
            count(&lens, index) == faces.len(),
            "a meter's count is not the length of the list it opens into",
            format!(
                "{:?} counts {} and opens {} faces; a chip the player cannot walk into is a \
                 bare number",
                spec.id,
                count(&lens, index),
                faces.len()
            ),
        );
        for (who, reason) in &faces {
            checks.require(
                !reason.is_empty() && lens.person(*who).is_some(),
                "a meter counts somebody with no reason or no registry row",
                format!("{:?} counts person {who} with reason {reason:?}", spec.id),
            );
        }
    }
    checks.require(
        counted == lens.people().len(),
        "the idle and away chips do not divide the cast between them",
        format!(
            "{counted} of {} people are counted by the two chips, and everybody is either at \
             home or out",
            lens.people().len()
        ),
    );
    let away_index = METERS
        .iter()
        .position(|spec| spec.id == "away")
        .unwrap_or(0);
    checks.require(
        count(&lens, away_index) == 1,
        "one party working does not put exactly one face on the away chip",
        format!(
            "the away chip counts {} with one party out",
            count(&lens, away_index)
        ),
    );
}
