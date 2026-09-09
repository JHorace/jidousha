//! Fat Orange Man — a tap-to-feed clicker.
//!
//! Tap the button (or the man himself) and a large orange square grows. Your
//! own count climbs, the communal count climbs faster because forty bots are
//! feeding him too, and a Top-10 leaderboard ranks the lot. There is no end,
//! no reset and no way to lose — the square just gets bigger, which is the
//! joke (`GDD.md`).
//!
//! **This build is the local, deterministic half.** `GDD.md` §6 puts the real
//! global count and the real leaderboard on a SpacetimeDB instance; that is
//! Phase 2, behind `--features online`, and `FINDINGS.md` records where it
//! stands. Everything the seeded bots stand in for — a shared square, a live
//! board, optimistic taps reconciled against an authority — has its seam in
//! `src/backend.rs` so the online arm slots in without the draw or input code
//! changing.
//!
//! Play it:   `cargo run -p fat-orange-man`
//! On the web: `tools/build-web fat-orange-man && tools/serve-web fat-orange-man`
//! Check it:  `tools/verify fat-orange-man`
#![allow(missing_docs)]

use std::process::ExitCode;

use jidousha::prelude::*;

mod backend;
mod capture;
mod checks;
mod draw;
mod game;
mod players;
mod sim;
mod verify;

pub use game::WINDOW;

/// The game's configuration, shared by the window and the verify run so what is
/// verified is what a person plays.
pub fn config() -> GameConfig {
    GameConfig {
        title: "Fat Orange Man",
        seed: 0,
        window_size: WINDOW,
        ..GameConfig::default()
    }
}

/// Every system this game has, in one place and in one order.
///
/// **Update order is the contract** (`verify.rs` asserts it out of
/// `schedule_debug()`): the tap is read, then the backend is pumped and the
/// projection rebuilt, then the bounce relaxes — so a tap grows the square on
/// the frame it was pressed.
///
/// **Draw order is deliberately not band order.** The panel is submitted before
/// the square though it sits in a higher band, so a recorded frame's draw order
/// disagrees with its submission order and a check can see the `PANEL` band do
/// its job (jidousha-testing.md: a band is invisible where it agrees with
/// submission order).
pub fn register(app: &mut App) {
    app.add_system(Startup, sim::open_the_world);
    app.add_system(Update, sim::handle_tap);
    app.add_system(Update, sim::advance_backend);
    app.add_system(Update, sim::ease_pop);
    app.add_system(Draw, draw::draw_panel);
    app.add_system(Draw, draw::draw_square);
    app.add_system(Draw, draw::draw_button);
    app.add_system(Draw, draw::draw_readout);
}

fn main() -> ExitCode {
    // `tools/verify fat-orange-man` runs this same binary with `--verify`: same
    // systems, same config, no window, scripted taps, assertions instead of a
    // person. `std::env::args` is empty on wasm, so this branch is native-only,
    // which is where verification runs.
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    println!("tap FEED HIM (or the man) to feed him — close the window to quit");
    match run(config(), register) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
