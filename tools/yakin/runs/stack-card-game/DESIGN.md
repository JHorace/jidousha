# DESIGN — stack-card-game (V1, inline)

The design lives in `games/stack-card-game/DESIGN.md` (rules, card pool, why
stack manipulation dominates). Build order: rules + cards (pure) → NPC → ECS
shell + drawing → `--verify` (decision-row checks, three players, determinism)
→ mutation round → capture → web build.
Files touched: `games/stack-card-game/**`, this run folder, `Cargo.lock`
(own crate entry).
Done-when checks: DESIGN.md exists; `tools/verify stack-card-game` pass;
diff-stat fence; `tools/build-web` + `serve-web --check`; PR.
