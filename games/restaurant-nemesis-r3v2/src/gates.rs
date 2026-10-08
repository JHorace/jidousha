//! The decision-row gates (DESIGN.md G5, G6, G7) and the cap gate (G9): seeded
//! runs through the real input path, each judged on the frame drawn the tick
//! before the deciding key, then held to what the sim did after it.
//!
//! Expected strings and numbers are shipped literals worked out from the
//! design's table, never read back from the game.
//!
//! Key functions: `spawn_row`, `card_runs`, `ledger_runs`, `at_the_cap`;
//! key item: `SEED_SPAWN`.

use jidousha::prelude::*;

use crate::checks::{Checks, look, panel_has};
use crate::lines::spend_line;
use crate::players::{Session, Stats, greedy, scenario};
use crate::rules::{
    Consequence, cut_followers, defeat_progress, outcome_of, social_outlook, tier_of, title_for,
};
use crate::sim::*;

/// The first seed in 0..256 where the greedy chef's day 1 spawns exactly one
/// nemesis, who is affordable to ratio on night 1 and still visits first on
/// days 3 and 5 under the tally policy (DESIGN.md §Open calls, relaxed: no
/// seed keeps them alone through day 5).
pub(crate) const SEED_SPAWN: u64 = 9;
/// What that nemesis is, worked out once and shipped.
const SPAWN_THEME: Theme = Theme::Condiment;
const SPAWN_TITLE: &str = "Mustard Monster";

fn service(day: u32) -> impl Fn(&Game) -> bool {
    move |g| g.day == day && g.phase == Phase::Service { focus: None }
}

fn card_open(day: u32) -> impl Fn(&Game) -> bool {
    move |g| g.day == day && matches!(g.phase, Phase::Service { focus: Some(_) })
}

/// Day 1 of `SEED_SPAWN` up to the greedy chef's Enter.
fn day_one_closing(checks: &mut Checks, card: Key) -> Session {
    let mut session = Session::new(SEED_SPAWN);
    let policy = scenario(card);
    let mut stats = Stats::default();
    let closing = |g: &Game| greedy(g) == Some(Key::Enter) && g.day == 1;
    let ok = session.run_until(&policy, &closing, &mut stats);
    checks.require(
        ok,
        "the scenario never reached day 1's close",
        format!("seed {SEED_SPAWN}, {} ticks", session.ticks),
    );
    session
}

/// Row one: the unmet order that crosses the threshold says so before Enter,
/// and the nemesis it spawns is the one it said.
pub(crate) fn spawn_row(checks: &mut Checks) {
    let check = "G5 capacity";
    let mut session = day_one_closing(checks, Key::S);
    let (_, panel) = look(checks, "day 1 at the close", &mut session);
    let before_panel = panel.clone();
    let game = session.game().clone();
    let risky: Vec<(usize, u32, Theme)> = game
        .queue
        .iter()
        .enumerate()
        .filter(|(_, o)| !o.served)
        .map(|(row, o)| (row, outcome_of(o, &game).if_unmet))
        .filter(|(_, u)| u.consequence == Consequence::Spawns)
        .map(|(row, u)| (row, u.severity, u.theme))
        .collect();
    let previewed: i32 = game
        .queue
        .iter()
        .filter(|o| !o.served)
        .map(|o| outcome_of(o, &game).if_unmet.money)
        .sum();
    let unmet = game.queue.iter().filter(|o| !o.served).count() as i32;
    let Some(&(row, severity, theme)) = risky.first() else {
        checks.require(
            false,
            "no order at day 1's close would spawn",
            check.to_owned(),
        );
        return;
    };
    let shown = format!("{} sev {severity} -> SPAWN", theme.name());
    checks.require(
        risky.len() == 1 && severity >= 4 && panel_has(&panel, &shown) && theme == SPAWN_THEME,
        "the order that will spawn does not say so before the close",
        format!(
            "{check}: risky rows {risky:?}; looked for {shown:?} on row {}",
            row + 1
        ),
    );
    let before = game.money;
    session.press(Key::Enter);
    let after = session.game().clone();
    let (_, panel) = look(checks, "night 1 ledger", &mut session);
    let line = format!(
        "SPAWNED: {SPAWN_TITLE} ({0}), sensitivity +2 {0}, 30 fol, returns day 3",
        SPAWN_THEME.name()
    );
    checks.require(
        after.nemeses.len() == 1
            && after.nemeses[0].theme == theme
            && title_for(theme, 0) == SPAWN_TITLE
            && panel_has(&panel, &line)
            && after.money == before + previewed
            && previewed == -8 * unmet
            && panel_has(&before_panel, "SPAWN, -$8 -"),
        "the spawn is not the one the row previewed",
        format!(
            "{check}: {} nemeses, money {before} -> {} (previewed {previewed} for {unmet} unmet); \
             want the ledger to read {line:?}",
            after.nemeses.len(),
            after.money
        ),
    );
    let mut stats = Stats::default();
    let policy = scenario(Key::S);
    session.run_until(&policy, &service(3), &mut stats);
    let (_, panel) = look(checks, "day 3 queue", &mut session);
    let row_line = format!("wants {}, need 3", SPAWN_THEME.name());
    session.press(Key::Digit1);
    let (_, card) = look(checks, "day 3 card", &mut session);
    let sensitivity = format!(
        "{} - {1}, sensitivity +2 {1}",
        SPAWN_TITLE.to_uppercase(),
        SPAWN_THEME.name()
    );
    checks.require(
        panel_has(&panel, &row_line) && panel_has(&card, &sensitivity),
        "the returning nemesis does not carry the sensitivity it spawned with",
        format!("{check}: looked for {row_line:?} on the queue and {sensitivity:?} on the card"),
    );
    checks.note(format!(
        "{check}: seed {SEED_SPAWN} row {} previewed {shown:?}; {SPAWN_TITLE} spawned, returns day 3 at need 3",
        row + 1
    ));
}

/// Row two, three runs: one per defeat path.
pub(crate) fn card_runs(checks: &mut Checks) {
    let check = "G6 card";
    // Tally: one served visit on day 3, the deciding one on day 5.
    let mut session = day_one_closing(checks, Key::S);
    let policy = scenario(Key::S);
    let mut stats = Stats::default();
    session.run_until(&policy, &card_open(3), &mut stats);
    let (_, first) = look(checks, "day 3 card", &mut session);
    let early = nemesis_progress(&session);
    session.press(Key::S);
    session.run_until(&policy, &card_open(5), &mut stats);
    let (_, second) = look(checks, "day 5 card", &mut session);
    let late = nemesis_progress(&session);
    session.press(Key::S);
    let game = session.game();
    checks.require(
        panel_has(&first, "won over 0 of 2 - S serves (3 units)")
            && !panel_has(&first, "S serves (3 units), DEFEATS them")
            && panel_has(
                &second,
                "won over 1 of 2 - S serves (3 units), DEFEATS them",
            )
            && early == Some(false)
            && late == Some(true)
            && game.defeated.first().map(|n| n.id) == Some(NemesisId(1))
            && game.find_nemesis(NemesisId(1)).is_none()
            && game.day == 5,
        "the tally path did not defeat on the visit the card said",
        format!(
            "{check} tally: defeated_if_served {early:?} then {late:?}; after S on day {} \
             {} defeated, {} active",
            game.day,
            game.defeated.len(),
            game.nemeses.len()
        ),
    );

    // Overwhelm: day 3, with the whole day's capacity.
    let mut session = day_one_closing(checks, Key::O);
    let policy = scenario(Key::O);
    session.run_until(&policy, &card_open(3), &mut stats);
    let (_, card) = look(checks, "day 3 card, overwhelm", &mut session);
    let cost = session
        .game()
        .nemeses
        .first()
        .map(|n| defeat_progress(n, session.game().capacity_left).overwhelm);
    let capacity = session.game().capacity_left;
    session.press(Key::O);
    let game = session.game();
    checks.require(
        panel_has(
            &card,
            "overwhelm - O (5 units), leaves 0 of 5 for the rest, DEFEATS them",
        ) && capacity == 5
            && cost == Some(Some(5))
            && game.defeated.first().map(|n| n.id) == Some(NemesisId(1))
            && game.day == 3
            && game.capacity_left == 0,
        "the overwhelm path did not defeat on the visit the card said",
        format!(
            "{check} overwhelm: capacity {capacity}, overwhelm {cost:?}; after O day {}, \
             {} defeated, {} capacity left",
            game.day,
            game.defeated.len(),
            game.capacity_left
        ),
    );

    // Followers cut: two spends on night 1.
    let mut session = day_one_closing(checks, Key::S);
    session.press(Key::Enter);
    let (_, ledger) = look(checks, "night 1 ledger, ratio", &mut session);
    let spends = session
        .game()
        .nemeses
        .first()
        .map(|n| defeat_progress(n, 0).spends_to_ratio);
    let money = session.game().money;
    let row = format!(
        "[1] {} - Local Menace, 30 fol, 2 spends to ratio",
        SPAWN_TITLE.to_uppercase()
    );
    session.press(Key::Digit1);
    session.press(Key::Digit1);
    let game = session.game().clone();
    session.press(Key::Enter);
    let tomorrow = session.game();
    let lingering = tomorrow
        .queue
        .iter()
        .filter(|o| !matches!(o.kind, Kind::Ordinary))
        .count();
    checks.require(
        panel_has(&ledger, &row)
            && spends == Some(2)
            && game.defeated.len() == 1
            && game.nemeses.is_empty()
            && game.day == 1
            && money - game.money == 50
            && lingering == 0,
        "the follower path did not defeat on the spend the ledger said",
        format!(
            "{check} ratio: looked for {row:?}; spends {spends:?}; money {money} -> {}; \
             {} defeated on night {}; {lingering} follower or nemesis rows on day 2",
            game.money,
            game.defeated.len(),
            game.day
        ),
    );
    checks.note(format!(
        "{check}: tally won over on day 5, overwhelmed on day 3, ratioed on night 1 (seed {SEED_SPAWN})"
    ));
}

fn nemesis_progress(session: &Session) -> Option<bool> {
    let game = session.game();
    game.nemeses
        .first()
        .map(|n| defeat_progress(n, game.capacity_left).defeated_if_served)
}

/// Row three: two runs differing in one spend; the preview says how tomorrow
/// differs, and it does.
pub(crate) fn ledger_runs(checks: &mut Checks) {
    let check = "G7 ledger";
    let mut a = day_one_closing(checks, Key::S);
    a.press(Key::Enter);
    let mut b = day_one_closing(checks, Key::S);
    b.press(Key::Enter);
    let (_, panel) = look(checks, "night 1 ledger, spend", &mut b);
    let shown = "spend $25: 30 -> 15 fol, Local Menace -> Grumbler, 2 -> 1 followers";
    let game = b.game().clone();
    let mut copy = game.nemeses.clone();
    cut_followers(&mut copy[0]);
    let then = social_outlook(&copy, &game.tomorrow_base).per[0];
    let now = social_outlook(&game.nemeses, &game.tomorrow_base).per[0];
    let recomputed = format!(
        "spend $25: {} -> {} fol, {} -> {}, {} -> {} followers",
        now.followers,
        then.followers,
        now.tier.name(),
        then.tier.name(),
        now.share,
        then.share
    );
    b.press(Key::Digit1);
    a.press(Key::Enter);
    b.press(Key::Enter);
    let (qa, qb) = (a.game().queue.clone(), b.game().queue.clone());
    let followers = |q: &[Order]| {
        q.iter()
            .enumerate()
            .filter(|(_, o)| matches!(o.kind, Kind::Follower { .. }))
            .map(|(i, _)| i)
            .collect::<Vec<_>>()
    };
    let (fa, fb) = (followers(&qa), followers(&qb));
    let rest_equal = qa.len() == qb.len() && (2..qa.len()).all(|i| qa[i] == qb[i]);
    let tier_b = b.game().nemeses.first().map(|n| tier_of(n.followers));
    let spent = a.game().money - b.game().money;
    checks.require(
        panel_has(&panel, shown)
            && recomputed == shown
            && spend_line(&game, 0) == shown
            && fa == vec![0, 1]
            && fb == vec![0]
            && rest_equal
            && spent == SOCIAL_COST
            && tier_b == Some(Tier::Grumbler),
        "the spend did not change tomorrow as the ledger previewed",
        format!(
            "{check}: preview {recomputed:?}; followers A {fa:?} B {fb:?}; rows 3.. equal \
             {rest_equal}; money apart {spent}; B's tier {tier_b:?}"
        ),
    );
    checks.note(format!(
        "{check}: {shown:?}; tomorrow's followers {fa:?} without, {fb:?} with"
    ));
}

/// At the cap a failure feeds a nemesis instead of spawning (G9).
pub(crate) fn at_the_cap(checks: &mut Checks) {
    let check = "G9 cap";
    let mut session = Session::new(SEED_SPAWN);
    let game = session.game_mut();
    let make = |id, theme, followers| Nemesis {
        id: NemesisId(id),
        theme,
        title_index: 0,
        followers,
        tally: 0,
        spawned_day: 1,
        next_visit: 9,
    };
    game.nemeses = vec![
        make(1, Theme::Condiment, 30),
        make(2, Theme::Temperature, 50),
        make(3, Theme::Wait, 30),
    ];
    game.next_id = 4;
    game.queue = vec![Order {
        kind: Kind::Ordinary,
        customer: Customer {
            theme: Theme::Portion,
            need: 2,
            temper: 2,
        },
        served: false,
    }];
    let (_, panel) = look(checks, "the cap", &mut session);
    session.press(Key::Enter);
    let game = session.game();
    let fed = game.nemeses.iter().map(|n| n.followers).collect::<Vec<_>>();
    checks.require(
        panel_has(&panel, "portion sev 4 -> feeds The Lukewarm Baron +20 fol")
            && fed == vec![30, 70, 30]
            && game.nemeses.len() == 3,
        "a failure at the cap did not feed as previewed",
        format!("{check}: followers after {fed:?}, want [30, 70, 30]"),
    );
    checks.note(format!(
        "{check}: fed The Lukewarm Baron +20 at three active"
    ));
}

/// A nemesis left unmet grows by 30, comes back two days on, and — trending
/// now — costs 3 reputation that night (the arithmetic, as literals).
pub(crate) fn nemesis_unmet(checks: &mut Checks) {
    let check = "nemesis unmet";
    let mut session = Session::new(SEED_SPAWN);
    let game = session.game_mut();
    game.nemeses = vec![Nemesis {
        id: NemesisId(1),
        theme: Theme::Wait,
        title_index: 0,
        followers: 30,
        tally: 0,
        spawned_day: 1,
        next_visit: 1,
    }];
    game.next_id = 2;
    game.money = 40;
    game.rep = 30;
    game.queue = vec![Order {
        kind: Kind::Nemesis(NemesisId(1)),
        customer: Customer {
            theme: Theme::Wait,
            need: 3,
            temper: 3,
        },
        served: false,
    }];
    let (_, panel) = look(checks, check, &mut session);
    session.press(Key::Enter);
    let game = session.game();
    let after = game
        .nemeses
        .first()
        .map(|n| (n.followers, n.next_visit, tier_of(n.followers)));
    checks.require(
        panel_has(&panel, "unmet: +30 fol -$20 -5 rep")
            && after == Some((60, 3, Tier::TrendingTerror))
            && (game.money, game.rep) == (20, 22),
        "an unmet nemesis did not grow, return and trend as the row said",
        format!(
            "{check}: (followers, next visit, tier) {after:?}, want (60, 3, TrendingTerror); \
             money {} rep {}, want 20 and 22",
            game.money, game.rep
        ),
    );
    checks.note(format!(
        "{check}: +30 fol to 60, Trending Terror, -3 rep that night, back on day 3"
    ));
}

/// Whether `seed` still meets `SEED_SPAWN`'s precondition — checked every
/// run, so a rule change that moves the scenario off its seed says so.
pub(crate) fn spawn_seed_holds_now(checks: &mut Checks) {
    let holds = spawn_seed_holds(SEED_SPAWN);
    checks.require(
        holds,
        "SEED_SPAWN no longer sets up the decision gates",
        format!(
            "seed {SEED_SPAWN}: want one nemesis after day 1, $50 on night 1, and that nemesis \
             first in the queue on days 3 and 5"
        ),
    );
}

fn spawn_seed_holds(seed: u64) -> bool {
    let mut session = Session::new(seed);
    let policy = scenario(Key::S);
    let mut stats = Stats::default();
    let closing = |g: &Game| greedy(g) == Some(Key::Enter) && g.day == 1;
    if !session.run_until(&policy, &closing, &mut stats) {
        return false;
    }
    session.press(Key::Enter);
    let night = session.game();
    if night.phase != Phase::Ledger || night.nemeses.len() != 1 || night.money < 50 {
        return false;
    }
    // The first nemesis must still be active and visiting on days 3 and 5;
    // others may have spawned (no seed in 0..1024 keeps it alone — see the
    // PR's deviations).
    for day in [3, 5] {
        session.run_until(&policy, &service(day), &mut stats);
        let g = session.game();
        let first = g.queue.first().map(|o| o.kind);
        if g.day != day || first != Some(Kind::Nemesis(NemesisId(1))) {
            return false;
        }
        session.press(Key::Digit1);
        session.press(Key::S);
    }
    true
}
