//! Staged scenes: each decision row set up on purpose and asked directly.
//!
//! A scene builds the real game headless with a `Game` it has arranged, types
//! through the same keys a player does, and reads the screen's own panel and
//! the recorded frame before the act and the run after it. Expectations are
//! shipped literals: a tally of 3, an all-out of quality 7 against a demand
//! of 4, a floor of 10 followers, 4 and 2 followers of 23 at levels 3 and 2.

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder};
use jidousha::ui::judge_frame;

use crate::checks::Checks;
use crate::content::Theme;
use crate::players::Typist;
use crate::screens::{OneToOne, screen};
use crate::sim::{DefeatPath, Game, Nemesis, Order, Phase, Serve, Who};
use crate::{Command, WINDOW, camera, config, register};

/// A headless game playing `game`, after its first tick.
pub fn staged(game: Game) -> HeadlessSim {
    let mut sim = headless(config(), register);
    sim.world_mut().insert_resource(game);
    sim.tick();
    sim
}

/// Type one command: a press tick and a release tick.
pub fn type_command(sim: &mut HeadlessSim, typist: &mut Typist, command: Command) {
    for _ in 0..2 {
        let input = typist.input_for(Some(command));
        sim.world_mut().insert_resource(input);
        sim.tick();
    }
}

/// Every line of the current screen's panel, joined.
pub fn screen_text(game: &Game) -> String {
    screen(game).all_strings().collect::<Vec<_>>().join("\n")
}

/// Draw a frame and check the panel's every row is on it.
pub fn photograph(sim: &mut HeadlessSim, checks: &mut Checks, what: &str) -> FrameRecord {
    let mut recorder = FrameRecorder::new(WINDOW);
    let frame = recorder.draw(sim);
    let panel = screen(sim.world().resource::<Game>());
    let breaches = judge_frame(
        &panel,
        &frame,
        recorder.font_texture(),
        &OneToOne,
        camera().visible_bounds(),
    );
    checks.require(
        breaches.is_empty(),
        "a row the screen says it draws is not on the frame",
        format!("{what}: {breaches:?}"),
    );
    frame
}

/// A fresh nemesis in `theme`, staged straight into a run.
pub fn stage_nemesis(game: &mut Game, theme: Theme, followers: i32, tally: u8) -> usize {
    game.nemeses.push(Nemesis {
        title: theme.titles()[0].to_owned(),
        theme,
        severity: 3,
        followers,
        tally,
        born: (0, 0),
        defeated: None,
    });
    game.nemeses.len() - 1
}

/// A service round in progress with nemesis `index` visiting at the head of
/// the queue and the other orders cleared, so all the kitchen is free.
fn nemesis_round(index: usize, theme: Theme) -> (Game, Order) {
    let mut game = Game::new(5);
    crate::day::open_day(&mut game);
    let mut demands = [0; 4];
    demands[theme.index()] = 4;
    demands[(theme.index() + 1) % 4] = 1;
    let order = Order {
        who: Who::Nemesis(index),
        demands,
        serve: Serve::Skip,
    };
    game.queue = vec![order];
    game.selected = 0;
    (game, order)
}

/// Row 2, tally: three careful visits, defeated on the third and not before;
/// the card says so before each.
pub fn tally(checks: &mut Checks) -> String {
    let mut report = Vec::new();
    let mut game = Game::new(5);
    let index = stage_nemesis(&mut game, Theme::Condiment, 60, 0);
    let (mut staged_game, _) = nemesis_round(index, Theme::Condiment);
    staged_game.nemeses = game.nemeses.clone();
    let mut sim = staged(staged_game);
    let mut typist = Typist::default();
    for visit in 1..=3u8 {
        {
            let game = sim.world_mut().resource_mut::<Game>();
            if game.phase != Phase::Service || game.queue.is_empty() {
                report.push(format!("visit {visit}: the run left service"));
                break;
            }
            let order = game.queue[0];
            game.queue = vec![order];
            game.selected = 0;
        }
        photograph(&mut sim, checks, "tally card");
        let text = screen_text(sim.world().resource::<Game>());
        let says = if visit == 3 {
            "CAREFUL (2): DEFEATED, 3rd visit".to_owned()
        } else {
            format!("CAREFUL (2): tally {visit}/3")
        };
        checks.require(
            text.contains(&says) && text.contains(&format!("1 tally {}/3 visits", visit - 1)),
            "the card does not show the tally before the visit",
            format!("visit {visit}: want {says:?} on the card; card:\n{text}"),
        );
        type_command(&mut sim, &mut typist, Command::Digit(2));
        type_command(&mut sim, &mut typist, Command::Go);
        let game = sim.world().resource::<Game>();
        let defeated = game.nemeses[index].defeated;
        let want = visit == 3;
        checks.require(
            defeated.is_some() == want
                && (!want || matches!(defeated, Some((DefeatPath::Tally, _, _)))),
            "the tally does not defeat on exactly the third satisfying visit",
            format!(
                "after visit {visit}: {defeated:?}, tally {}",
                game.nemeses[index].tally
            ),
        );
        report.push(format!("visit {visit}: {}", defeated.is_some()));
        // Put the nemesis at the head of the next round's queue.
        let game = sim.world_mut().resource_mut::<Game>();
        if game.phase == Phase::Service {
            let mut demands = [0; 4];
            demands[Theme::Condiment.index()] = 4;
            demands[Theme::Temperature.index()] = 1;
            game.queue.insert(
                0,
                Order {
                    who: Who::Nemesis(index),
                    demands,
                    serve: Serve::Skip,
                },
            );
        }
    }
    format!("careful x3 -> defeated after {}", report.join(", "))
}

/// Row 2, overwhelm: one all-out visit beats a fresh nemesis outright.
pub fn overwhelm(checks: &mut Checks) -> (String, FrameRecord) {
    let mut base = Game::new(5);
    let index = stage_nemesis(&mut base, Theme::Temperature, 60, 0);
    let (mut game, _) = nemesis_round(index, Theme::Temperature);
    game.nemeses = base.nemeses.clone();
    let mut sim = staged(game);
    let mut typist = Typist::default();
    let frame = photograph(&mut sim, checks, "overwhelm card");
    let text = screen_text(sim.world().resource::<Game>());
    checks.require(
        text.contains("ALL-OUT (4): DEFEATED, outright")
            && text.contains("2 overwhelm: quality 7 vs need 4")
            && text.contains("CAREFUL (2): tally 1/3"),
        "the card does not say an all-out beats this nemesis outright",
        format!("card:\n{text}"),
    );
    let (day, round) = {
        let game = sim.world().resource::<Game>();
        (game.day, game.round)
    };
    type_command(&mut sim, &mut typist, Command::Digit(3));
    type_command(&mut sim, &mut typist, Command::Go);
    let defeated = sim.world().resource::<Game>().nemeses[index].defeated;
    checks.require(
        defeated == Some((DefeatPath::Overwhelmed, day, round)),
        "one all-out visit did not defeat the nemesis on that visit",
        format!("defeated {defeated:?}, want Overwhelmed on day {day} round {round}"),
    );
    (
        format!("all-out on day {day} round {round} -> {defeated:?}"),
        frame,
    )
}

/// Row 2, followers: a takedown that leaves 5 followers ends the nemesis the
/// next morning, and the ledger said so first.
pub fn following(checks: &mut Checks) -> String {
    let mut game = Game::new(5);
    let index = stage_nemesis(&mut game, Theme::Wait, 35, 0);
    let mut sim = staged(game);
    let mut typist = Typist::default();
    let before = screen_text(sim.world().resource::<Game>());
    type_command(&mut sim, &mut typist, Command::Digit(1));
    photograph(&mut sim, checks, "ledger with takedown");
    let after = screen_text(sim.world().resource::<Game>());
    let still = sim.world().resource::<Game>().nemeses[index].defeated;
    checks.require(
        before.contains("with takedown: 5 followers - GIVES UP tomorrow")
            && after.contains("as chosen: 5 followers - GIVES UP tomorrow")
            && still.is_none(),
        "the ledger does not say the takedown ends the nemesis before it is bought",
        format!(
            "before:\n{before}\nafter:\n{after}\nstill active: {}",
            still.is_none()
        ),
    );
    type_command(&mut sim, &mut typist, Command::Go);
    let game = sim.world().resource::<Game>();
    let defeated = game.nemeses[index].defeated;
    checks.require(
        defeated == Some((DefeatPath::Following, 1, 0)),
        "the takedown did not end the nemesis the morning it opened",
        format!(
            "defeated {defeated:?}, followers {}",
            game.nemeses[index].followers
        ),
    );
    format!("takedown 35 -> 5 -> {defeated:?}")
}

/// Row 3, one side: a ledger with one nemesis at 70 followers, a takedown or
/// not, and what tomorrow's customers turned out to be.
fn spend_run(takedown: bool, checks: &mut Checks) -> (String, usize, Vec<u8>) {
    let mut game = Game::new(5);
    let index = stage_nemesis(&mut game, Theme::Portion, 70, 0);
    let mut sim = staged(game);
    let mut typist = Typist::default();
    if takedown {
        type_command(&mut sim, &mut typist, Command::Digit(1));
    }
    photograph(&mut sim, checks, "ledger");
    let text = screen_text(sim.world().resource::<Game>());
    let preview = text
        .lines()
        .find(|line| line.contains("as chosen:"))
        .unwrap_or("")
        .trim()
        .to_owned();
    type_command(&mut sim, &mut typist, Command::Go);
    let game = sim.world().resource::<Game>();
    let fans: Vec<u8> = game
        .today
        .iter()
        .flatten()
        .filter(|order| order.who == Who::Follower(index))
        .map(|order| order.demands[Theme::Portion.index()])
        .collect();
    (preview, fans.len(), fans)
}

/// Row 3: two runs differing only in one takedown; each preview is what the
/// next day's customers turned out to be.
pub fn spend(checks: &mut Checks) -> String {
    let (kept, kept_count, kept_levels) = spend_run(false, checks);
    let (cut, cut_count, cut_levels) = spend_run(true, checks);
    checks.require(
        kept == "as chosen: 70 followers, tier 2, 4 of 23 follow, wanting PORTION 3"
            && kept_count == 4
            && kept_levels.iter().all(|&level| level == 3),
        "with no takedown, tomorrow's followers are not what the ledger said",
        format!("preview {kept:?}; tomorrow {kept_count} followers at {kept_levels:?}"),
    );
    checks.require(
        cut == "as chosen: 40 followers, tier 1, 2 of 23 follow, wanting PORTION 2"
            && cut_count == 2
            && cut_levels.iter().all(|&level| level == 2),
        "with a takedown, tomorrow's followers are not what the ledger said",
        format!("preview {cut:?}; tomorrow {cut_count} followers at {cut_levels:?}"),
    );
    format!(
        "no spend: {kept_count} fans at {kept_levels:?}; takedown: {cut_count} at {cut_levels:?}"
    )
}

/// The night: rent comes off and every active nemesis gains 10 followers,
/// which is what the next ledger starts from.
pub fn night(checks: &mut Checks) -> String {
    let mut game = Game::new(5);
    let index = stage_nemesis(&mut game, Theme::Condiment, 30, 0);
    crate::day::open_day(&mut game);
    game.round = 3;
    game.queue.clear();
    let money = game.money;
    crate::sim::cook_round(&mut game);
    let followers = game.nemeses[index].followers;
    checks.require(
        game.phase == Phase::Ledger && followers == 40 && game.money == money - 25,
        "the night did not add 10 followers and take the rent",
        format!(
            "phase {:?}, followers 30 -> {followers} (want 40), money {money} -> {} (want -25)",
            game.phase, game.money
        ),
    );
    format!(
        "followers 30 -> {followers}, money {money} -> {}",
        game.money
    )
}

/// The spends cost what the ledger says and buy what it says: a takedown
/// and a temp cook take $35 between them, and the temp cook makes the
/// kitchen 5 a round.
pub fn spends_paid(checks: &mut Checks) -> String {
    let mut game = Game::new(5);
    stage_nemesis(&mut game, Theme::Wait, 70, 0);
    let mut sim = staged(game);
    let mut typist = Typist::default();
    type_command(&mut sim, &mut typist, Command::Digit(1));
    type_command(&mut sim, &mut typist, Command::Temp);
    let before = sim.world().resource::<Game>().money;
    let text = screen_text(sim.world().resource::<Game>());
    type_command(&mut sim, &mut typist, Command::Go);
    let game = sim.world().resource::<Game>();
    let (capacity, _) = game.kitchen();
    checks.require(
        text.contains("spends $35") && game.money == before - 35 && capacity == 5,
        "the night's spends did not cost or buy what the ledger said",
        format!(
            "ledger said spends $35: {}; money {before} -> {} (want -35); kitchen {capacity} (want 5)",
            text.contains("spends $35"),
            game.money
        ),
    );
    format!(
        "takedown + temp: money {before} -> {}, kitchen {capacity}",
        game.money
    )
}
