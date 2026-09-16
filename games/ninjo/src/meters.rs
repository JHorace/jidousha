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

use crate::constants::Tuning;
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
    ///
    /// It takes the constants because the pressure chips are questions about
    /// what upkeep costs, and that cost is a drawer row (`needs::cost_of`) —
    /// a chip that carried its own copy of the number would be the second
    /// answer GDD §1 refuses.
    pub asks: fn(&Lens<'_>, &Tuning, usize) -> Option<String>,
}

/// Every chip above the map.
///
/// **Four rows since wave 1.3.** The first two are where everybody is:
/// standing at home, or out with a party. The second two are **the pressure
/// surface** — who cannot meet their upkeep this interval, and who the need
/// has pressed past the chip's threshold — and they are the reason the
/// framework was built data-registered (GDD §3, wave 0a: "the chips are
/// data-registered so wave-1 modules add theirs").
///
/// The first two partition the present cast; the second two cut across it, so
/// a person can be idle *and* short, which is exactly the state the player is
/// meant to see and act on.
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
    MeterSpec {
        id: "short",
        label: "short",
        icon: Art::Coin,
        asks: short,
    },
    MeterSpec {
        id: "desperate",
        label: "desperate",
        icon: Art::Flame,
        asks: desperate,
    },
];

/// Standing at home with nothing asked of them.
fn idle(lens: &Lens<'_>, _tuning: &Tuning, who: usize) -> Option<String> {
    lens.at_home(who)
        .then(|| "at home, and nobody has asked them for anything".to_owned())
}

/// Out of the settlement, and why.
fn away(lens: &Lens<'_>, _tuning: &Tuning, who: usize) -> Option<String> {
    if lens.at_home(who) {
        return None;
    }
    Some(lens.activity_line(who))
}

/// **Cannot meet their upkeep out of what they are holding** — the same
/// comparison `needs::burn` is about to make, so the chip counts the set the
/// simulation will act on rather than a set that resembles it.
///
/// The reason is what they hold against what they owe, because a face with
/// "short" beside it is a bare number with a portrait on it.
fn short(lens: &Lens<'_>, tuning: &Tuning, who: usize) -> Option<String> {
    if !crate::needs::is_short(lens, tuning, who) {
        return None;
    }
    // Short, because a face row is 240 reference pixels wide and what a clip
    // takes off a reason is the half that says why. The *whole* why is the
    // person's source line, one tap further in, on the panel a face opens.
    Some(format!(
        "holds {}g of the {}g due",
        lens.wallet(who),
        crate::needs::owed_by(tuning, lens.traits(who))
    ))
}

/// **Pressed past the threshold**, and how many intervals it took.
///
/// The row says the number and what made it; the *sentence* that makes two
/// identical desperations two different problems (GDD §3) is the person's
/// source line, which is one tap further in — a face row opens that person's
/// panel, and the panel carries the line in full. It is not on the row because
/// the row is 240 reference pixels wide and a clipped source line is a source
/// line with its particulars cut off, which is the one thing it must not be.
fn desperate(lens: &Lens<'_>, _tuning: &Tuning, who: usize) -> Option<String> {
    crate::needs::is_desperate(lens, who).then(|| {
        format!(
            "desperation {}, short {}",
            lens.desperation(who),
            lens.shortfalls(who)
        )
    })
}

/// The faces behind chip `index`: who counts, and why.
///
/// **Over the lens's roll**, so a chip counts the camp rather than the
/// registry: somebody who has not arrived has no figure, no party and no place
/// on any chip (`CAST.md` §4's arrival column).
pub fn faces(lens: &Lens<'_>, tuning: &Tuning, index: usize) -> Vec<(usize, String)> {
    let Some(spec) = METERS.get(index) else {
        return Vec::new();
    };
    lens.roll()
        .into_iter()
        .filter_map(|who| (spec.asks)(lens, tuning, who).map(|reason| (who, reason)))
        .collect()
}

/// How many people chip `index` counts.
pub fn count(lens: &Lens<'_>, tuning: &Tuning, index: usize) -> usize {
    faces(lens, tuning, index).len()
}

/// The chips' own validation, plus the claim that makes them trustworthy:
/// **every chip's count is the length of the list it opens into**, every chip
/// counts only people who are in the camp, and — the one-source assertion the
/// pressure surface owes (GDD §9) — **each chip's set is exactly the set the
/// simulation would act on**.
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
    // A world where one party is out: the two placement chips must divide the
    // present cast, and every count must be the length of the list it opens
    // into.
    let mut sim = crate::sim::Sim::opening(tuning, crate::modules::ModuleSet::ALL);
    sim.parties[0].activity = crate::sim::Activity::Working { until: 99 };
    let lens = Lens::on(&sim);
    let mut placed = 0usize;
    for (index, spec) in METERS.iter().enumerate() {
        let faces = faces(&lens, tuning, index);
        if matches!(spec.id, "idle" | "away") {
            placed += faces.len();
        }
        checks.require(
            count(&lens, tuning, index) == faces.len(),
            "a meter's count is not the length of the list it opens into",
            format!(
                "{:?} counts {} and opens {} faces; a chip the player cannot walk into is a \
                 bare number",
                spec.id,
                count(&lens, tuning, index),
                faces.len()
            ),
        );
        for (who, reason) in &faces {
            checks.require(
                !reason.is_empty() && lens.person(*who).is_some(),
                "a meter counts somebody with no reason or no registry row",
                format!("{:?} counts person {who} with reason {reason:?}", spec.id),
            );
            checks.require(
                lens.present(*who),
                "a meter counts somebody who has not arrived in the camp",
                format!(
                    "{:?} counts {}, who arrives at minute {}; a chip counts the camp and not \
                     the registry",
                    spec.id,
                    lens.name(*who),
                    lens.arrives(*who)
                ),
            );
        }
    }
    checks.require(
        placed == lens.roll().len(),
        "the idle and away chips do not divide the present cast between them",
        format!(
            "{placed} of {} people in the camp are counted by the two placement chips, and \
             everybody present is either at home or out",
            lens.roll().len()
        ),
    );
    let away_index = METERS
        .iter()
        .position(|spec| spec.id == "away")
        .unwrap_or(0);
    checks.require(
        count(&lens, tuning, away_index) == 1,
        "one party working does not put exactly one face on the away chip",
        format!(
            "the away chip counts {} with one party out",
            count(&lens, tuning, away_index)
        ),
    );
    // **The one-source assertion** (GDD §9's band-chip discipline, wave 1.3):
    // the `short` chip's set is the set `needs::burn` is about to take money
    // from and fail to find it, and the `desperate` chip's set is the set the
    // roster's own desperation puts past the threshold. Staged on a world
    // emptied of purses so the chips are not vacuously empty, and asserted
    // against a *burn actually run* rather than against a second reading of
    // the same predicate.
    let mut staged = crate::sim::Sim::opening(tuning, crate::modules::ModuleSet::ALL);
    staged.everybody_here();
    staged.people[1].wallet = 0;
    staged.people[7].wallet = 0;
    staged.people[3].desperation = DESPERATE_FLOOR;
    let short_index = METERS
        .iter()
        .position(|spec| spec.id == "short")
        .unwrap_or(0);
    let desperate_index = METERS
        .iter()
        .position(|spec| spec.id == "desperate")
        .unwrap_or(0);
    let predicted: Vec<usize> = {
        let lens = Lens::on(&staged);
        faces(&lens, tuning, short_index)
            .into_iter()
            .map(|(who, _)| who)
            .collect()
    };
    let desperate_faces: Vec<usize> = {
        let lens = Lens::on(&staged);
        faces(&lens, tuning, desperate_index)
            .into_iter()
            .map(|(who, _)| who)
            .collect()
    };
    checks.require(
        !predicted.is_empty() && !desperate_faces.is_empty(),
        "the pressure chips counted nobody on a world staged to press",
        format!(
            "the short chip named {predicted:?} and the desperate chip named \
             {desperate_faces:?}; a one-source assertion over two empty sets passes vacuously"
        ),
    );
    let before: Vec<u64> = staged
        .people
        .iter()
        .map(|person| person.shortfalls)
        .collect();
    crate::needs::burn(&mut staged, tuning, 0, 0);
    let acted: Vec<usize> = staged
        .people
        .iter()
        .enumerate()
        .filter(|(who, person)| person.shortfalls > before[*who])
        .map(|(who, _)| who)
        .collect();
    checks.require(
        acted == predicted,
        "the short chip counts a different set from the one the simulation acts on",
        format!(
            "the chip named {predicted:?} and the burn went short on {acted:?}; a warning \
             surface is asserted equal to its consequence's inputs (GDD §9)"
        ),
    );
    let over: Vec<usize> = {
        let lens = Lens::on(&staged);
        lens.roll()
            .into_iter()
            .filter(|who| lens.desperation(*who) >= crate::needs::DESPERATE_AT)
            .collect()
    };
    let after: Vec<usize> = {
        let lens = Lens::on(&staged);
        faces(&lens, tuning, desperate_index)
            .into_iter()
            .map(|(who, _)| who)
            .collect()
    };
    checks.require(
        after == over,
        "the desperate chip counts a different set from the one over the threshold",
        format!("the chip named {after:?} and the roster puts {over:?} at or over the line"),
    );
}

/// The desperation the one-source staging puts one person at, so the
/// `desperate` chip has somebody on it — a shipped literal, at the threshold,
/// never derived from `needs::DESPERATE_AT`, because a check that computed its
/// own expectation from the constant under test could not see it move.
const DESPERATE_FLOOR: i64 = 6;
