//! `?constants=` and `?seed=` — the contracts a shareable link is held to.
//!
//! A link is the compact form of a constants set, and the compact form is
//! what a recording is stamped with, so the two halves owe each other a
//! round trip: whatever `Tuning::stamp` writes, `Tuning::parse` reads back
//! as the same set. Everything else here is a refusal — rejected loudly and
//! by name, never silently clamped — reachable only by writing a bad link,
//! which no conducted session ever does, so nothing but this file measures
//! any of it.

use crate::checks::Checks;
use crate::constants::{ConstantsError, Field, Tuning};
use crate::presets;

/// Every way a link is refused, and the round trip that says the two halves
/// of the compact form agree.
pub fn link_contracts(checks: &mut Checks) {
    // The round trip: what a stamp writes, `parse` reads back.
    for preset in presets::PRESETS {
        let parsed = Tuning::SHIPPED.parse(&preset.tuning.stamp());
        checks.require(
            parsed.as_ref() == Ok(&preset.tuning),
            "a constants stamp does not parse back to the set it was written from",
            format!(
                "{} stamps as {:?} and parses back as {parsed:?}",
                preset.name,
                preset.tuning.stamp()
            ),
        );
    }
    // A short link means "the shipped set with these moved".
    let one = Tuning::SHIPPED.parse("road_cost:1");
    checks.require(
        one == Ok(Tuning::SHIPPED.with(Field::RoadCost, 1)),
        "a link naming one constant did not leave the others alone",
        format!("`road_cost:1` parsed as {one:?}"),
    );
    // Case is not a second name.
    checks.require(
        Tuning::SHIPPED.parse("ROAD_COST:1") == one,
        "a constants key means different things in different cases",
        format!(
            "`ROAD_COST:1` parsed as {:?}",
            Tuning::SHIPPED.parse("ROAD_COST:1")
        ),
    );

    // And the refusals, each named rather than clamped.
    for (text, expected) in refused() {
        let got = Tuning::SHIPPED.parse(text);
        checks.require(
            got.as_ref().err() == Some(&expected),
            "a bad ?constants= was not refused the way it should be",
            format!("{text:?} parsed as {got:?}, and the refusal should be {expected:?}"),
        );
    }
    let over = Tuning::SHIPPED.parse(&format!("road_cost:{}", Tuning::MAX + 1));
    checks.require(
        over.is_err(),
        "an out-of-range ?constants= value was silently clamped",
        format!(
            "road_cost:{} parsed as {over:?}; rejected loudly, not clamped",
            Tuning::MAX + 1
        ),
    );

    // `?seed=`: a whole number in, the same number out; anything else refused.
    checks.require(
        crate::web::parse_seed("7") == Ok(7) && crate::web::parse_seed(" 42 ") == Ok(42),
        "a plain ?seed= does not parse to itself",
        format!("7 parsed as {:?}", crate::web::parse_seed("7")),
    );
    for bad in ["seven", "-1", "1.5", ""] {
        checks.require(
            crate::web::parse_seed(bad).is_err(),
            "a bad ?seed= was not refused",
            format!("{bad:?} parsed as {:?}", crate::web::parse_seed(bad)),
        );
    }
}

/// Every refusal, as (the link that earns it, the refusal it earns).
fn refused() -> Vec<(&'static str, ConstantsError)> {
    vec![
        ("", ConstantsError::Empty),
        ("road_cost", ConstantsError::Pair("road_cost".to_owned())),
        (
            "swamp_cost:2",
            ConstantsError::UnknownKey("swamp_cost".to_owned()),
        ),
        (
            "road_cost:1,road_cost:2",
            ConstantsError::Repeated("road_cost".to_owned()),
        ),
        (
            "road_cost:two",
            ConstantsError::NotANumber {
                key: "road_cost".to_owned(),
                value: "two".to_owned(),
            },
        ),
        (
            "road_cost:99",
            ConstantsError::OutOfRange {
                key: "road_cost".to_owned(),
                value: 99,
            },
        ),
        (
            "road_cost:-1",
            ConstantsError::OutOfRange {
                key: "road_cost".to_owned(),
                value: -1,
            },
        ),
    ]
}

/// The message each refusal prints, for the printable-strings check.
pub fn refusals() -> Vec<String> {
    let mut out: Vec<String> = refused()
        .into_iter()
        .map(|(_, error)| error.message())
        .collect();
    if let Err(message) = crate::web::parse_seed("seven") {
        out.push(message);
    }
    out
}
