//! The variant's `--verify` checks, run as tests so a regression names itself in
//! `cargo test` before the whole verify run is paid for.

use crate::checks::Checks;

fn holds(check: impl FnOnce(&mut Checks) -> String) {
    let mut checks = Checks::default();
    let line = check(&mut checks);
    let (passed, failed) = checks.counts();
    if failed > 0 {
        let _ = checks.verdict();
    }
    assert_eq!(failed, 0, "{line}: {failed} failed after {passed} passed");
    assert!(passed > 0, "{line}: nothing was checked");
}

#[test]
fn a_failed_personal_quest_marks_the_name_as_the_card_and_sheet_said_it_would() {
    holds(crate::x1::check_personal);
}

#[test]
fn a_mark_costs_the_house_a_renown_when_the_year_turns() {
    holds(crate::x1::check_weighing);
}

#[test]
fn the_garden_refuses_an_unproven_outsider_and_takes_one_at_the_threshold() {
    holds(crate::x1::check_marrying_in);
}

#[test]
fn the_death_pages_candidate_lines_are_what_choosing_does() {
    holds(crate::x1::check_heir_inheritance);
}

#[test]
fn only_family_is_marked_outsiders_earn_nothing_and_a_child_is_born_under_a_mark() {
    holds(crate::x1_rules::check_rules);
}
