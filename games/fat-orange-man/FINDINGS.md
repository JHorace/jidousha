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

## Phase 2 (SpacetimeDB)

The GDD's architecture (§6) is server-authoritative real-time multiplayer on
SpacetimeDB. Phase 1 built the whole loop locally with the backend seam
(`backend.rs`) in place for the online arm. Two open questions belong to Phase 2:

- **G-027 (open, Phase 2):** `spacetimedb-sdk` 2.10 resolves and builds
  natively from this registry, but its `browser` feature pins `web-sys =0.3.77`
  / a `wasm-bindgen` set that has to be reconciled against the engine's own web
  stack (`wasm-bindgen` 0.2.127 in `Cargo.lock`). Whether the SDK's Rust
  client runs at all on `wasm32-unknown-unknown` inside a `tools/build-web`
  bundle is unresolved — and if it does not, the online arm is native-only and
  the deployed web build stays on the local bots.
- **G-028 (open, Phase 2):** the `spacetime` CLI on this machine is v2.6.0
  while the SDK is v2.10.0. A local instance + published reducer module is
  needed before the online arm can be exercised end to end. The reducer module
  is not a Jidousha game and lives outside this workspace.

## Environment note (resolved mid-session)

The session opened on a machine with Rust 1.98.0 as a distro package and no
`rustup`, so `rust-toolchain.toml` (1.94.1) was ignored, `wasm32-unknown-unknown`
could not be added, and `cargo clippy --workspace` tripped a 1.98-era lint in
`crates/jidousha-assets`. The owner installed `rustup`; it auto-provisioned
1.94.1 + the wasm target from the pin, `wasm-bindgen-cli` 0.2.127 was installed,
`tools/doctor` returned `ENV_OK`, and every gate below ran clean. Recorded
because the failure mode — a direct toolchain quietly ignoring the pin — looks
like a code problem and is not one.
