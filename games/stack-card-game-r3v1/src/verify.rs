//! stub
use crate::players::{Player, play_match};
pub(crate) fn run() -> std::process::ExitCode {
    for player in [Player::Reader, Player::RawPower, Player::Idle] {
        for seed in 1..=12 {
            let r = play_match(player, seed, false);
            println!(
                "{} seed {seed}: {:?} rounds {} ticks {} life {}-{} win {}/{} plan {:.1} off {:.1}",
                player.name(),
                r.outcome,
                r.rounds,
                r.ticks,
                r.you_life,
                r.rival_life,
                r.answered,
                r.windows,
                r.planned,
                r.landed_off
            );
        }
    }
    std::process::ExitCode::SUCCESS
}
