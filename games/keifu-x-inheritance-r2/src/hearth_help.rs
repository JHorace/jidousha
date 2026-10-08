//! What each group of winter seats says (`ui.winter`): its heading, the line under it
//! on the hearth screen, and the help the dock opens when it is pointed at — the help's
//! numbers filled from CONSTANTS §9-§10 (the garden's quotes W8's birth rule).

use crate::constants::{
    BIRTH_CHANCE_PERCENT, CHILD_TAUGHT_LIMIT, COURTING_AGE_GAP, MARRYING_AGE, PARENT_AGE_HIGH,
    REST_DREAD_SHED, SELF_TAUGHT_LIMIT, TALE_RENOWN, TEACHABLE_AGE,
};
use crate::content::Content;
use crate::hearth::Group;
use crate::text::fmt;
use crate::words::W;

/// The heading, text and help of each group (`ui.winter`).
pub fn group_words(content: &Content, group: Group) -> (String, String, String) {
    let words = &content.words;
    let n = |v: i32| v.to_string();
    match group {
        Group::Hall => (
            words[W::WinterHall].to_owned(),
            String::new(),
            words[W::WinterHallHelp].to_owned(),
        ),
        Group::Fire => (
            words[W::WinterFire].to_owned(),
            fmt(&words[W::WinterFireText], &[&n(REST_DREAD_SHED)]),
            words[W::WinterFireHelp].to_owned(),
        ),
        Group::Training => (
            words[W::WinterTraining].to_owned(),
            words[W::WinterTrainingText].to_owned(),
            fmt(&words[W::WinterTrainingHelp], &[&n(SELF_TAUGHT_LIMIT)]),
        ),
        Group::Garden => (
            words[W::WinterGarden].to_owned(),
            words[W::WinterGardenText].to_owned(),
            fmt(
                &words[W::WinterGardenHelp],
                &[
                    &n(MARRYING_AGE),
                    &n(COURTING_AGE_GAP),
                    &n(BIRTH_CHANCE_PERCENT),
                    &n(MARRYING_AGE),
                    &n(PARENT_AGE_HIGH),
                ],
            ),
        ),
        Group::Table => (
            words[W::WinterTable].to_owned(),
            words[W::WinterTableText].to_owned(),
            fmt(
                &words[W::WinterTableHelp],
                &[&n(TALE_RENOWN), &n(TALE_RENOWN)],
            ),
        ),
        Group::Benches => (
            words[W::WinterBenches].to_owned(),
            words[W::WinterBenchesText].to_owned(),
            fmt(
                &words[W::WinterBenchesHelp],
                &[&n(TEACHABLE_AGE), &n(CHILD_TAUGHT_LIMIT)],
            ),
        ),
    }
}

/// A group's heading and its help, for the dock.
pub fn group_help(content: &Content, group: Group) -> (String, String) {
    let (heading, _, help) = group_words(content, group);
    (heading, help)
}
