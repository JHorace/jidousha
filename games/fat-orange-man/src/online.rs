//! The SpacetimeDB backend (Phase 2, GDD §6) — `--features online` + `--online`.
//!
//! `Online` is the networked arm of `backend::Backend`. It connects to a
//! SpacetimeDB instance, subscribes to `global_state` and `player`, calls the
//! `feed` reducer on a tap, and reads the leaderboard back out of the client
//! cache. A background thread (`run_threaded`) keeps that cache current, so
//! `snapshot` is a plain read and `advance` only reconciles the optimistic
//! `pending` count against what the server has acknowledged.
//!
//! **Not replayable.** Wall-clock, network order, other players — everything the
//! rest of this game is built to exclude. That is why it is a feature and never
//! the default, and why no `--verify` check touches it.

use std::process::ExitCode;
use std::thread::JoinHandle;

use jidousha::prelude::*;
use spacetimedb_sdk::{DbContext, Identity, Table};

use crate::backend::{BOARD_ROWS, Backend, FeedState};
use crate::game::{Entry, project_board};
use crate::module_bindings::{
    DbConnection, GlobalStateTableAccess, Player, PlayerTableAccess, feed,
};

/// Where the instance and module default to when `--online` is passed with no
/// override. Match `spacetime publish --server local fat-orange-man`.
const DEFAULT_URI: &str = "http://127.0.0.1:3000";
const DEFAULT_MODULE: &str = "fat-orange-man";

/// How long to spin on `frame_tick` waiting for the connection to land before
/// giving up, in loop iterations (~5s at 1ms sleeps).
const CONNECT_SPINS: usize = 5_000;

/// The live-instance backend.
pub struct Online {
    conn: DbConnection,
    /// The background message pump. Held so it is not detached; never joined —
    /// the process outlives it.
    _pump: JoinHandle<()>,
    /// Our identity, resolved at connect.
    identity: Identity,
    /// Taps sent to the server this session.
    taps_sent: u64,
    /// The last personal count the server acknowledged.
    server_personal: u64,
}

impl Online {
    /// Connect, subscribe, and start the pump. `uri`/`module` fall back to the
    /// local defaults.
    pub fn connect(uri: Option<&str>, module: Option<&str>) -> Result<Self, String> {
        let uri = uri.unwrap_or(DEFAULT_URI);
        let module = module.unwrap_or(DEFAULT_MODULE);

        let conn = DbConnection::builder()
            .with_uri(uri)
            .with_database_name(module)
            .on_connect_error(|_ctx, err| {
                eprintln!("[fat-orange-man] SpacetimeDB connect error: {err}");
            })
            .on_disconnect(|_ctx, err| {
                eprintln!("[fat-orange-man] SpacetimeDB disconnected: {err:?}");
            })
            .build()
            .map_err(|err| {
                format!(
                    "could not open a SpacetimeDB connection to {uri} ({module}): {err}\n\
                     likely cause: no instance is running there, or the module is not published\n\
                     fix: `spacetime start` in one shell, then \
                     `spacetime publish --server local {module}` from ../../../fat-orange-man/server"
                )
            })?;

        let _pump = conn.run_threaded();

        conn.subscription_builder()
            .on_error(|_ctx, err| eprintln!("[fat-orange-man] subscription error: {err}"))
            .subscribe(["SELECT * FROM global_state", "SELECT * FROM player"]);

        // Spin until the connection reports an identity — `build` returns before
        // the handshake completes.
        let mut identity = None;
        for _ in 0..CONNECT_SPINS {
            if let Some(id) = conn.try_identity() {
                identity = Some(id);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        let identity = identity.ok_or_else(|| {
            format!("connected to {uri} but no identity arrived within the timeout")
        })?;

        Ok(Self {
            conn,
            _pump,
            identity,
            taps_sent: 0,
            server_personal: 0,
        })
    }

    /// Send a tap. Fire-and-forget; the echo is reconciled in `advance`.
    pub fn feed(&mut self) {
        match self.conn.reducers.feed() {
            Ok(()) => self.taps_sent += 1,
            Err(err) => eprintln!("[fat-orange-man] feed() reducer call failed: {err}"),
        }
    }

    /// Reconcile the optimistic count against what the server now shows.
    pub fn advance(&mut self) {
        self.server_personal = self
            .conn
            .db
            .player()
            .iter()
            .find(|row| row.identity == self.identity)
            .map(|row| row.feed_count)
            .unwrap_or(self.server_personal);
    }

    /// The projection, read out of the client cache the pump keeps current.
    pub fn snapshot(&self) -> FeedState {
        let global = self
            .conn
            .db
            .global_state()
            .iter()
            .map(|row| row.total_feed_count)
            .next()
            .unwrap_or(0);

        // Optimistic taps not yet acknowledged sit on top of the server count.
        let pending = self.taps_sent.saturating_sub(self.server_personal);
        let personal = self.server_personal + pending;

        let mut entries: Vec<Entry> = self
            .conn
            .db
            .player()
            .iter()
            .map(|row: Player| {
                let you = row.identity == self.identity;
                Entry {
                    name: display_name(&row, you),
                    count: if you { personal } else { row.feed_count },
                    you,
                }
            })
            .collect();
        if !entries.iter().any(|entry| entry.you) {
            entries.push(Entry {
                name: "YOU".to_owned(),
                count: personal,
                you: true,
            });
        }

        FeedState {
            personal,
            global: global.max(personal),
            board: project_board(entries, BOARD_ROWS),
        }
    }
}

/// A headless end-to-end check of the live path, for a machine with no display
/// (`--online --smoke`). Not a `--verify`: it talks to a real instance and is
/// not replayable, so `tools/test` never runs it. It builds the real game
/// headless with the `Online` backend, taps three times through the same
/// `Backend::feed` the button calls, pumps until the server echoes, and
/// asserts the personal and global counts climbed by at least three.
pub fn smoke() -> ExitCode {
    let mut sim = headless(crate::config(), crate::register);
    // A few ticks for Startup to connect and the first subscription rows to land.
    for _ in 0..20 {
        sim.tick();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let before = sim.world().resource::<FeedState>().clone();

    const TAPS: u64 = 3;
    for _ in 0..TAPS {
        sim.world_mut().resource_mut::<Backend>().feed();
    }
    // Pump until the *server-authoritative* global count reflects the feeds —
    // `personal` moves optimistically the moment `feed` is called, so waiting on
    // it would exit before the round trip. `global` only ever comes from the
    // subscribed `global_state` row.
    for _ in 0..600 {
        sim.tick();
        std::thread::sleep(std::time::Duration::from_millis(10));
        if sim.world().resource::<FeedState>().global >= before.global + TAPS {
            break;
        }
    }
    let after = sim.world().resource::<FeedState>().clone();

    println!(
        "online smoke: personal {} -> {}, global {} -> {}, board {} rows (+{} pinned)",
        before.personal,
        after.personal,
        before.global,
        after.global,
        after.board.top.len(),
        usize::from(after.board.your_row.is_some()),
    );
    let ok = after.personal >= before.personal + TAPS && after.global >= before.global + TAPS;
    if ok {
        println!("online smoke: OK");
        ExitCode::SUCCESS
    } else {
        eprintln!("online smoke: the server did not acknowledge {TAPS} feeds within the timeout");
        ExitCode::FAILURE
    }
}

/// A feeder's leaderboard label: their chosen handle, or a short slice of the
/// identity hex (GDD §4.2 — anonymised identity is an acceptable fallback).
fn display_name(row: &Player, you: bool) -> String {
    if you {
        return "YOU".to_owned();
    }
    match &row.display_name {
        Some(name) if !name.trim().is_empty() => name.chars().take(13).collect(),
        _ => {
            let hex = row.identity.to_hex();
            format!("ANON_{}", &hex.as_str()[..6.min(hex.as_str().len())])
        }
    }
}
