# fat-orange-man — what `docs/api/` cost

The findings this build owes back, in the format `docs/internal/e0-findings.md`
uses (`make-game` §C). G-numbers continue the sequence across games; ninjo's
`FINDINGS.md` ends at G-023, so this game starts at **G-024**.

**Reading discipline.** Phase 1 was written from `docs/api/` (all four),
`crates/jidousha/examples/` (`prototype_kit` whole, `pong` and `slalom` and
`scripted_player` in part), and `games/ninjo/` read as a sibling *game* for its
crate manifest shape and its `web.rs` (a game, not the engine — allowed, said
here so the line stays auditable). No file under `crates/*/src/` was opened;
no `docs/internal/`; no ADR but 0038 (named by the workflow) and a read of
`docs/adr/INDEX.md`-adjacent `0038` text itself. `CLAUDE.md`, the `make-game`
skill, `tools/verify`, `tools/build-web`, `tools/serve-web`, `clippy.toml` and
the root `Cargo.toml` were read as workflow, not as engine internals.

**Phase 2** additionally read the `spacetimedb` / `spacetimedb-sdk` 2.6.0 crate
source in `~/.cargo/registry` (third-party, not engine code) to settle the
`#[table(accessor = …)]` macro syntax and the `DbConnectionBuilder` method
names, and the `spacetime generate`d `module_bindings/` it wrote into this
crate. Still no engine `src/`, no `docs/internal/`, no unnamed ADR.

---

## G-024 — the controllers document has no shape for a game with no target

Class: docs · Game: fat-orange-man · Document: `docs/api/jidousha-controllers.md` · Open

**What I was doing:** writing the `--verify` player for a clicker — no
opponent, no moving target, one input (tap), no way to lose.

**What I expected:** the document to say what "write three players, not one"
becomes when there is nothing to aim at.

**What happened:** every section is about an adversary or a target that moves
faster than you can chase it. "Write three players" is win / lose / mediocre;
"the three numbers" are returner / objective / aim; "aim at where the target
will be" and the closed-form opponent inequality assume a paddle. None of it
maps. I adapted it — three **tap cadences** (idle / human / masher) and three
numbers (taps issued, taps registered, ending rank) — and the adaptation was
sound, but it was invention against a document that only answers for a genre
this game is not. The document already carves out `examples/slalom` as "the
version with no opponent"; a clicker is a step further out (no opponent *and*
no target), and one sentence naming that case — "when the player is just an
input rate, the three are cadences and the three numbers are throughput and
standing" — would have replaced a guess with a citation.

**On its authority I:** wrote `players.rs` and the cadence checks in
`verify.rs` from first principles rather than from the document. Nothing broke;
the cost was confidence, not a bug. Owner: `jidousha-controllers.md`.

---

## G-025 — a cosmetic draw-time transform has no free function, and a check has to carry its live value (the game's own)

Class: game · Game: fat-orange-man · Marked as the game's decision, not an engine gap

`docs/api/jidousha-api.md` Concepts says to write the decisions a check will
want as **free functions** — and for the sim decisions here (`square_size`,
`is_feed_tap`, `project_board`) that held perfectly: `verify.rs` calls them
directly. But the tap bounce (`Pop`) is a multiplier the *draw* system applies
to the square's side and nothing else — there is no sim decision to factor out,
because the bounce is not part of the simulation. A check that recomputed the
expected drawn size as `square_size(global)` alone failed: the run ends on a
tap tick, so the square is mid-bounce and 11% larger than the curve says.

The fix is not a free function; it is for the check to **carry the live
multiplier out of the run** (`Run::pop_end`) and fold it into the expectation.
The lesson for the next wave: the "write it as a free function" rule is about
decisions the sim makes; a purely presentational transform applied in `Draw`
has no such function, and its verification reads the value the frame was drawn
with rather than deriving it. This is a note about how to check this game, not
a gap in the engine's documents.

---

## G-026 — the growth constant and the bots' opening counts are a coupled pair (the game's own)

Class: game · Game: fat-orange-man · Marked as the game's decision, not an engine gap

`GROWTH` (the sqrt curve's coefficient) and the bots' `base`/`per_100` seeds
are coupled the way Concepts couples the thinnest collider and the top speed:
the first draft gave 40 bots opening counts in the hundreds against
`GROWTH = 0.35`, and `square_size` was pinned at `SQUARE_MAX` from tick 1 — the
communal square never visibly grew, which is the entire mechanic (GDD §4.3,
§7). Rebalancing meant moving both together: bots opening in the tens, and
`GROWTH` down to `0.091` so ~2,000 feeds is half a screen and ~10,000 fills
it. The `--verify` capture is what caught it — the first PNG showed a maxed
square — which is the "open the picture and name what you see" step doing its
job. Recorded so the next person tuning either number knows to look at the
other.

---

## Phase 2 (SpacetimeDB) — built and exercised

The GDD's architecture (§6) is server-authoritative real-time multiplayer on
SpacetimeDB. Phase 1 built the whole loop locally with the backend seam
(`backend.rs`). Phase 2 landed the live arm:

- **Reducer module** `../../../fat-orange-man/server/` (outside this workspace —
  server code, not a game). `player` + `global_state` tables, `feed` /
  `set_display_name` / `identity_connected` / `init` reducers, pinned to
  `spacetimedb = "=2.6.0"` to match the local `spacetime` CLI. Built with
  `spacetime build`, published to a local instance.
- **Client** `src/online.rs` behind `--features online` + `--online` (native
  only). `Backend` is now an enum — `Local(Local)` / `Online(Box<Online>)` — and
  the callers in `sim.rs` still never branch on which. `Online` connects,
  subscribes to both tables, calls `feed` on a tap, keeps the client cache
  current on a background thread, and reconciles the optimistic count against
  the server's echo.
- **Exercised** headless via `--online --smoke` (no display on this machine):
  connect → three taps → `global_state` climbs by three on the server and in the
  client's subscription. `spacetime sql` confirms it independently.

- **G-027 (resolved — the online arm is native-only):** `spacetimedb-sdk`'s wasm
  path needs its `browser` feature, whose `web-sys`/`wasm-bindgen` pins fight the
  engine's own web stack. Rather than force that, the SDK dependency is
  `cfg(not(target_arch = "wasm32"))` and every `online` code path is
  `#[cfg(all(feature = "online", not(target_arch = "wasm32")))]`. Result: all
  four configs compile (default native/wasm, online native/wasm), and a web
  build with `--features online` **degrades to the seeded bots** rather than
  failing. Wiring the real backend into the deployed page is future work and
  starts by resolving that `wasm-bindgen` version fight.
- **G-028 (resolved):** pinned `spacetimedb = "=2.6.0"` / `spacetimedb-sdk =
  "=2.6.0"` to match the CLI's 2.6.0 ABI; `spacetime start` runs a local
  standalone instance fine on this machine; module published and the smoke test
  passes end to end.
- **G-029 (docs, open):** the engine documents have **no shape for a backend
  that is not a pure function of the tick** — by design (`docs/api/` is the
  deterministic-single-process API), but the `make-game` reading fence assumes
  every question is answered there. Nothing said whether a networked, background-
  threaded client can be an engine `Resource` (it can — `spacetimedb_sdk`'s
  `DbConnection` is `Send + Sync`), where its pump belongs (an `Update` system
  calling `advance`), or how the optimistic-then-reconciled pattern from GDD §4.1
  maps onto `fn(&mut World)`. Worked it out from the SDK's own docs and the
  engine's resource model; recorded so the next networked prototype inherits a
  starting point. Half the game's decision (the seam shape) and half a gap the
  documents could name in one paragraph: "the engine is deterministic and
  single-process on purpose; a game that wants a network backend puts the
  connection in a `Resource`, pumps it from one `Update` system, and keeps its
  `--verify` mode on a local deterministic stand-in."
- **G-030 (the game's own):** `Backend` as an enum trips
  `clippy::large_enum_variant` once `Online` (a `DbConnection` plus handles) is a
  variant — the network arm is an order of magnitude bigger than `Local`. Boxed
  it (`Online(Box<Online>)`); the delegating `match` arms are unchanged because
  `Box` auto-derefs. A note for any prototype that grows a second heavy backend.

## Environment note (resolved mid-session)

The session opened on a machine with Rust 1.98.0 as a distro package and no
`rustup`, so `rust-toolchain.toml` (1.94.1) was ignored, `wasm32-unknown-unknown`
could not be added, and `cargo clippy --workspace` tripped a 1.98-era lint in
`crates/jidousha-assets`. The owner installed `rustup`; it auto-provisioned
1.94.1 + the wasm target from the pin, `wasm-bindgen-cli` 0.2.127 was installed,
`tools/doctor` returned `ENV_OK`, and every gate below ran clean. Recorded
because the failure mode — a direct toolchain quietly ignoring the pin — looks
like a code problem and is not one.
