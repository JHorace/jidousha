//! `?constants=` — the contracts a shareable tuning link is held to (UI.md §12).
//!
//! A link is the compact form of a constants set, and the compact form is what a
//! recording is stamped with, so the two halves owe each other a round trip:
//! whatever `Tuning::stamp` writes, `Tuning::parse` reads back as the same set.
//! Everything else here is a refusal — a key that is not a constant, a value
//! that is not a number, a value outside the range, a key given twice — because
//! UI.md §12 says a bad link is rejected loudly and never silently clamped, and
//! a clamp is exactly what a parser writes when nobody is checking.
//!
//! **Reachable only by writing a bad link**, which no played beat ever does, so
//! nothing but this file measures any of it.

use crate::checks::Checks;
use crate::constants::{ConstantsError, Field, Tuning};
use crate::presets;

/// Every way a `?constants=` link is refused, and the round trip that says the
/// two halves of the compact form agree.
pub fn link_contracts(checks: &mut Checks) {
    // The round trip: what a stamp writes, `parse` reads back — which is what
    // makes a shared link and a recorded stamp the same artifact.
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
    // A short link means "the shipped set with these moved", which is what makes
    // `?constants=k_kill:4` a thing worth sending somebody.
    let one = Tuning::SHIPPED.parse("k_kill:4");
    checks.require(
        one == Ok(Tuning::SHIPPED.with(Field::KKill, 4)),
        "a link naming one constant did not leave the others alone",
        format!("`k_kill:4` parsed as {one:?}"),
    );
    // Case is not a second name (constants.rs's module header).
    checks.require(
        Tuning::SHIPPED.parse("K_KILL:4") == one,
        "a constants key means different things in different cases",
        format!(
            "`K_KILL:4` parsed as {:?}",
            Tuning::SHIPPED.parse("K_KILL:4")
        ),
    );

    // And the refusals, each named rather than clamped (UI.md §12).
    for (text, expected) in refused() {
        let got = Tuning::SHIPPED.parse(text);
        checks.require(
            got.as_ref().err() == Some(&expected),
            "a bad ?constants= was not refused the way it should be",
            format!("{text:?} parsed as {got:?}, and the refusal should be {expected:?}"),
        );
    }
    // The one that matters most, stated as its own claim: a value past the end
    // of the range is refused rather than pulled back to it.
    let over = Tuning::SHIPPED.parse(&format!("k_kill:{}", Tuning::MAX + 1));
    checks.require(
        over.is_err(),
        "an out-of-range ?constants= value was silently clamped",
        format!(
            "k_kill:{} parsed as {over:?}; UI.md §12 says rejected loudly, not clamped",
            Tuning::MAX + 1
        ),
    );
}

/// Every refusal, as (the link that earns it, the refusal it earns).
fn refused() -> Vec<(&'static str, ConstantsError)> {
    vec![
        ("", ConstantsError::Empty),
        ("k_kill", ConstantsError::Pair("k_kill".to_owned())),
        (
            "k_charm:2",
            ConstantsError::UnknownKey("k_charm".to_owned()),
        ),
        (
            "k_kill:4,k_kill:5",
            ConstantsError::Repeated("k_kill".to_owned()),
        ),
        (
            "k_kill:six",
            ConstantsError::NotANumber {
                key: "k_kill".to_owned(),
                value: "six".to_owned(),
            },
        ),
        (
            "k_kill:99",
            ConstantsError::OutOfRange {
                key: "k_kill".to_owned(),
                value: 99,
            },
        ),
        (
            "k_kill:-1",
            ConstantsError::OutOfRange {
                key: "k_kill".to_owned(),
                value: -1,
            },
        ),
    ]
}

/// The message each refusal prints, for the printable-strings check: every one
/// of them is a string the page draws (`library::printable_strings`).
pub fn refusals() -> Vec<String> {
    refused()
        .into_iter()
        .map(|(_, error)| error.message())
        .collect()
}
