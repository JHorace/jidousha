//! G8: the arithmetic the decision rows rest on, one sentence each.
//!
//! Key items: `nemesis`, `game_with`.

use super::*;
use crate::turn::new_game;

fn nemesis(id: u32, theme: Theme, followers: u32) -> Nemesis {
    Nemesis {
        id: NemesisId(id),
        theme,
        title_index: 0,
        followers,
        tally: 0,
        spawned_day: 1,
        next_visit: 3,
    }
}

fn game_with(nemeses: Vec<Nemesis>) -> Game {
    let mut game = new_game(1, &mut Rng::from_seed(1));
    game.nemeses = nemeses;
    game
}

fn ordinary(theme: Theme, need: u32, temper: u32) -> Order {
    Order {
        kind: Kind::Ordinary,
        customer: Customer {
            theme,
            need,
            temper,
        },
        served: false,
    }
}

#[test]
fn tier_of_answers_grumbler_at_29_local_menace_at_30_and_59_trending_terror_at_60() {
    assert_eq!(tier_of(29), Tier::Grumbler);
    assert_eq!(tier_of(30), Tier::LocalMenace);
    assert_eq!(tier_of(59), Tier::LocalMenace);
    assert_eq!(tier_of(60), Tier::TrendingTerror);
}

#[test]
fn severity_is_need_plus_temper_and_four_is_the_first_value_that_spawns() {
    let game = game_with(Vec::new());
    let three = ordinary(Theme::Wait, 1, 2);
    let four = ordinary(Theme::Wait, 2, 2);
    assert_eq!(severity(&four.customer), 4);
    assert_eq!(
        outcome_of(&three, &game).if_unmet.consequence,
        Consequence::Nothing
    );
    assert_eq!(
        outcome_of(&four, &game).if_unmet.consequence,
        Consequence::Spawns
    );
}

#[test]
fn a_spawn_at_the_cap_feeds_the_same_theme_nemesis_severity_times_five_followers_else_the_most_followed_oldest_on_ties()
 {
    let game = game_with(vec![
        nemesis(1, Theme::Condiment, 40),
        nemesis(2, Theme::Temperature, 40),
        nemesis(3, Theme::Wait, 10),
    ]);
    let same = ordinary(Theme::Wait, 2, 3);
    let other = ordinary(Theme::Portion, 2, 2);
    assert_eq!(
        outcome_of(&same, &game).if_unmet.consequence,
        Consequence::Feeds {
            id: NemesisId(3),
            followers: 25
        }
    );
    assert_eq!(
        outcome_of(&other, &game).if_unmet.consequence,
        Consequence::Feeds {
            id: NemesisId(1),
            followers: 20
        }
    );
}

#[test]
fn title_for_cycles_through_the_themes_three_titles_by_spawn_count() {
    assert_eq!(title_for(Theme::Condiment, 0), "Mustard Monster");
    assert_eq!(title_for(Theme::Temperature, 1), "Count Tepid");
    assert_eq!(title_for(Theme::Portion, 2), "Marquis de Morsel");
    assert_eq!(title_for(Theme::Wait, 3), "The Hangry Hourglass");
}

#[test]
fn cut_followers_floors_at_zero() {
    let mut n = nemesis(1, Theme::Wait, 10);
    cut_followers(&mut n);
    assert_eq!(n.followers, 0);
}

#[test]
fn social_outlook_with_three_trending_terrors_assigns_4_2_and_0_of_six_slots_in_spawn_order() {
    let all = vec![
        nemesis(1, Theme::Condiment, 60),
        nemesis(2, Theme::Wait, 70),
        nemesis(3, Theme::Portion, 90),
    ];
    let base = vec![
        Customer {
            theme: Theme::Temperature,
            need: 1,
            temper: 1
        };
        6
    ];
    let outlook = social_outlook(&all, &base);
    let shares: Vec<usize> = outlook.per.iter().map(|r| r.share).collect();
    assert_eq!(shares, vec![4, 2, 0]);
    assert_eq!(outlook.customers[0].1.theme, Theme::Condiment);
    assert_eq!(outlook.customers[0].1.need, 2);
}

#[test]
fn a_follower_of_a_defeated_leader_is_resolved_as_ordinary() {
    let game = game_with(Vec::new());
    let order = Order {
        kind: Kind::Follower {
            leader: NemesisId(7),
        },
        ..ordinary(Theme::Wait, 1, 1)
    };
    assert_eq!(outcome_of(&order, &game).if_unmet.leader_followers, 0);
}

#[test]
fn defeat_progress_offers_the_overwhelm_only_with_five_units_and_counts_spends_up() {
    let n = nemesis(1, Theme::Wait, 31);
    assert_eq!(defeat_progress(&n, 5).overwhelm, Some(5));
    assert_eq!(defeat_progress(&n, 4).overwhelm, None);
    assert_eq!(defeat_progress(&n, 5).spends_to_ratio, 3);
    assert!(!defeat_progress(&n, 5).defeated_if_served);
}
