//! The state seam: where the feed counts and the leaderboard come from.
//!
//! Everything the game draws about *progress* — the personal count, the global
//! count that sizes the square, the board — is read from `FeedState`, a plain
//! projection with no world in it. `Backend` is the source of truth behind it.
//! Today there is one source, `Local`: a seeded field of bot feeders that drift
//! upward on a fixed schedule, so a session is replayable and a `--verify` run
//! means something. Phase 2 adds a second arm that reads a SpacetimeDB instance
//! instead; the method surface here (`feed`, `advance`, `snapshot`) is the
//! whole of what the rest of the game asks of a backend, and is what that arm
//! has to satisfy.
//!
//! **Optimistic increment lives here** (GDD §4.1). `feed` bumps a local count
//! immediately so a tap feels instant; `Local` acknowledges it in the same
//! call, so `pending` is always zero. A networked arm would hold the tap in
//! `pending` until the server echoed it back and reconcile then — the field is
//! carried now so that reconciliation has somewhere to land.

use jidousha::prelude::*;

use crate::game::{Board, Entry, project_board};

/// How many bot feeders share the board with the player.
const BOT_COUNT: usize = 40;

/// How many rows the leaderboard shows before the pinned "you" row.
pub const BOARD_ROWS: usize = 10;

/// The handles the bots draw their names from, cycled with a numeric suffix
/// once exhausted. ASCII only — the built-in font draws a box for anything else
/// and no assertion over drawn quads can see it (jidousha-testing.md).
const HANDLES: [&str; 24] = [
    "BIGLYFAN",
    "COVFEFE",
    "MAGA_MIKE",
    "SAD_LARRY",
    "SLEEPY_DON",
    "BRAINWORM",
    "TREMENDOUS",
    "LOWENERGY",
    "HAMBERDER",
    "NAMBIA",
    "CADET_BONE",
    "TACO_TUES",
    "GERMS_R_BAD",
    "BLEACH_INJ",
    "SHARPIEGATE",
    "COVID_CURE",
    "WITCH_HUNT",
    "PERFECT_CALL",
    "TARIFF_MAN",
    "GOLDEN_ESC",
    "MAR_A_LARD",
    "DIET_COKE_BTN",
    "HORSEFACE",
    "PERSON_WOMAN",
];

/// The player's own feed count, split into what is confirmed and what is only
/// optimistically applied. `Local` confirms on the spot; the split is the
/// reconciliation seam for a networked backend.
#[derive(Clone, Copy, Debug, Default)]
struct You {
    /// Feeds the backend has acknowledged.
    confirmed: u64,
    /// Feeds tapped locally and not yet acknowledged. Always 0 under `Local`.
    pending: u64,
}

impl You {
    fn total(self) -> u64 {
        self.confirmed + self.pending
    }
}

/// One bot feeder: a name, a starting count, and a drift rate.
#[derive(Clone, Debug)]
struct Bot {
    name: String,
    /// Feeds it had at tick zero.
    base: u64,
    /// Feeds it gains per 100 ticks — integer arithmetic, so `count_at` is a
    /// pure function of the tick and identical on every machine.
    per_100: u64,
}

impl Bot {
    fn count_at(&self, tick: u64) -> u64 {
        self.base + self.per_100 * tick / 100
    }
}

/// The seeded local field: bots plus the player.
#[derive(Clone, Debug)]
pub struct Local {
    bots: Vec<Bot>,
    you: You,
}

impl Local {
    /// Seed the field from the world's `Rng` (itself seeded from
    /// `GameConfig::seed`), so the same seed is the same opening board.
    pub fn seeded(rng: &mut Rng) -> Self {
        let bots = (0..BOT_COUNT)
            .map(|index| {
                let handle = HANDLES[index % HANDLES.len()];
                let name = if index < HANDLES.len() {
                    handle.to_owned()
                } else {
                    format!("{handle}_{}", index / HANDLES.len() + 1)
                };
                // Kept low on purpose: 40 bots opening in the hundreds, not the
                // thousands, so the square starts small and the growth curve is
                // something a session watches happen rather than a value it is
                // already pinned at (GDD §4.3, §7).
                Bot {
                    name,
                    base: u64::from(rng.below(30)),
                    per_100: u64::from(rng.below(5)) + 1,
                }
            })
            .collect();
        Self {
            bots,
            you: You::default(),
        }
    }

    fn bot_global(&self, tick: u64) -> u64 {
        self.bots.iter().map(|bot| bot.count_at(tick)).sum()
    }

    fn entries(&self, tick: u64) -> Vec<Entry> {
        let mut entries: Vec<Entry> = self
            .bots
            .iter()
            .map(|bot| Entry {
                name: bot.name.clone(),
                count: bot.count_at(tick),
                you: false,
            })
            .collect();
        entries.push(Entry {
            name: "YOU".to_owned(),
            count: self.you.total(),
            you: true,
        });
        entries
    }
}

/// The source of truth behind `FeedState`.
///
/// A struct rather than an enum while there is one source; Phase 2 promotes it
/// (`Local` / `Online`) without the callers in `sim.rs` changing, because they
/// only ever call the three methods below.
pub struct Backend {
    local: Local,
}

impl Backend {
    /// Build the local backend, seeding its field from the world's `Rng`.
    pub fn local(rng: &mut Rng) -> Self {
        Self {
            local: Local::seeded(rng),
        }
    }

    /// Register one tap. Optimistic: applied to the local count at once (GDD
    /// §4.1). `Local` also confirms it here, so nothing is ever left pending.
    pub fn feed(&mut self) {
        self.local.you.confirmed += 1;
    }

    /// Advance the backend by one tick. For `Local` the bots are a pure
    /// function of the tick, so this is a no-op that exists for the seam: a
    /// networked arm pumps its connection and reconciles `pending` here.
    pub fn advance(&mut self, _tick: u64) {}

    /// The projection every draw system and every check reads.
    pub fn snapshot(&self, tick: u64) -> FeedState {
        let personal = self.local.you.total();
        let global = personal + self.local.bot_global(tick);
        let board = project_board(self.local.entries(tick), BOARD_ROWS);
        FeedState {
            personal,
            global,
            board,
        }
    }
}

impl Resource for Backend {}

/// The projection: what the game knows about feeding, as plain data.
///
/// Rebuilt every tick by `sim::advance_backend` and read (never written) by the
/// draw systems and the checks — one reader shape for both, so the picture and
/// the assertions cannot come apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FeedState {
    /// The player's own feed count, optimistic taps included.
    pub personal: u64,
    /// Everyone's feeds together — this is what sizes the square (GDD §4.3).
    pub global: u64,
    /// The leaderboard, already ranked.
    pub board: Board,
}

impl Resource for FeedState {}
