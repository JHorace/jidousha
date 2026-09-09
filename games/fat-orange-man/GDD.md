# Game Design Document
## Working Title: *Feed the Orange Man*

**Version:** 0.1 (MVP Scope)
**Date:** September 9, 2026
**Genre:** Idle / Clicker, mobile-style web game
**Platform:** Web (mobile-first responsive layout)
**Tech Stack:** SpacetimeDB (server-authoritative state + real-time sync), `jhorace/jidousha` (agent-driven game engine, client-side)

---

## 1. Concept Overview

*Feed the Orange Man* is a lightweight, mobile-feeling clicker game. Players tap a button to feed a large, round, orange character. Every tap increments a global "feed counter" tied to that player. As the player (and the community) feeds him more, a visual orange square grows larger on screen, representing his ever-expanding girth. The game is built around simple, addictive, low-friction interaction — the kind of tap-to-progress loop common in mobile idle games — with a social/competitive layer via a real-time leaderboard.

This document scopes the **first iteration (MVP)** of the game: a bare-bones but fully playable and shareable loop. Later iterations (resets, food types, animations, prestige systems, etc.) are noted as future considerations but are explicitly **out of scope** for this build.

---

## 2. Goals for This Iteration

- Prove out the core tap loop end-to-end on SpacetimeDB + jidousha.
- Validate real-time multiplayer state sync (everyone sees the counter and square grow live).
- Ship something that feels good on a phone screen with one thumb.
- Keep scope minimal: no resets, no economy, no food variety yet.

---

## 3. Core Gameplay Loop

1. Player opens the game and sees:
   - The Orange Man (represented as an orange square in this iteration).
   - A "Feed" button.
   - Their personal feed count.
   - A Top 10 leaderboard.
2. Player taps "Feed."
3. Their personal counter increments by 1.
4. The global feed counter increments by 1.
5. The orange square grows very slightly larger, visible to **all connected players** in real time.
6. Leaderboard updates live as players climb.
7. There is no end state, no reset, and no fail condition — the square simply grows forever. This is intentional: the joke/appeal is the absurdity of endless, uncapped growth.

---

## 4. Core Features (MVP Scope)

### 4.1 Feed Button / Counter
- Single tap target (button or tapping the Orange Man himself).
- Each tap = +1 to:
  - The player's individual feed count (persisted per user).
  - The global/shared feed count (drives the square's size).
- Client should optimistically increment on tap, then reconcile with the authoritative SpacetimeDB value to keep taps feeling instant even on latency.
- No cooldown, no click limit for MVP — rate limiting can be considered post-MVP if abuse becomes an issue.

### 4.2 Leaderboard
- Displays **Top 10 feeders** ranked by individual feed count, descending.
- Updates live as counts change (via SpacetimeDB subscription, not polling).
- Displays the current player's own rank/count even if they're outside the Top 10 (e.g., pinned row below the list: "You: #47 — 312 feeds").
- Ties broken by earliest timestamp reaching that count (first-to-score wins the tie), or simply stable sort — implementation detail, not a player-facing concern for MVP.

### 4.3 The Orange Man (Visual Growth)
- Represented in this iteration as a simple **orange square**.
- Size is a direct function of the **global** feed count (not the individual player's count) — this is a shared, communal pet that everyone feeds together.
- Growth curve: monotonically increasing, no cap, no reset. Suggest a decelerating growth curve (e.g., size ∝ sqrt(total_feeds) or log(total_feeds)) so the square doesn't immediately blow past the viewport — this is a visual/UX necessity, not a gameplay mechanic. Actual scaling authority stays with design/eng to tune for feel.
- No sprite/character art needed yet — the square *is* the placeholder for the character, matching the brief.

---

## 5. Out of Scope for This Iteration

To keep the MVP shippable, the following are explicitly **not** included in v0.1:

- Any reset, prestige, or decay mechanic for the square's size.
- Multiple food types, feeding animations, or particle effects.
- Character art beyond the orange square placeholder.
- Sound design.
- Player accounts/auth beyond whatever minimal identity SpacetimeDB session/client identity provides.
- Anti-cheat / rate limiting beyond basic server-side validation.
- Monetization, cosmetics, or any economy layer.

These are natural candidates for v0.2+ (see Section 8).

---

## 6. Technical Architecture

### 6.1 SpacetimeDB (Server / State Layer)
SpacetimeDB serves as the authoritative, real-time backend. Suggested schema for MVP:

**Tables**
- `Player`
  - `identity` (SpacetimeDB identity, primary key)
  - `display_name` (string, optional — falls back to anonymized identity if not set)
  - `feed_count` (u64)
  - `first_seen` (timestamp)
- `GlobalState`
  - singleton row
  - `total_feed_count` (u64)

**Reducers**
- `feed()` — called on each tap.
  - Increments `Player.feed_count` for the calling identity (creating the row if it doesn't exist).
  - Increments `GlobalState.total_feed_count`.
  - Server-authoritative: the client cannot set counts directly, only invoke `feed()`.
- `set_display_name(name)` — optional, quality-of-life for leaderboard readability.

**Subscriptions / Queries**
- Client subscribes to `GlobalState` for live square-size updates.
- Client subscribes to a `Player` query ordered by `feed_count DESC LIMIT 10` for the leaderboard.
- Client subscribes to its own `Player` row for personal count + rank display.

This structure keeps all game-affecting state server-side, so growth and rank can't be spoofed client-side — an important property even for a joke game, since a public leaderboard invites tampering attempts.

### 6.2 jidousha (Client / Game Engine Layer)
- Handles rendering the orange square, the feed button, and the UI chrome (leaderboard panel, personal count).
- Owns the input loop (tap/click handling) and calls the `feed()` reducer on SpacetimeDB.
- Owns the local, optimistic-update layer for responsiveness, reconciled against SpacetimeDB's authoritative state on the next sync tick.
- Note: because `jhorace/jidousha` isn't a widely documented public engine, this document intentionally stays high-level on its specific API surface — the implementation team should confirm exact conventions (scene setup, reducer-call bindings, subscription helpers, etc.) directly against the repo's own docs/examples before build-out.

### 6.3 Client/Server Interaction Flow
```
[Player taps]
   → jidousha client: optimistic local increment (instant visual feedback)
   → client calls feed() reducer on SpacetimeDB
   → SpacetimeDB validates + commits: Player.feed_count++, GlobalState.total_feed_count++
   → SpacetimeDB pushes updated rows to all subscribed clients
   → jidousha client: reconciles local state, redraws square size, re-sorts leaderboard
```

---

## 7. UX / Mobile Feel Notes

- Single-thumb operation: feed button should be large, centered or bottom-anchored, easily reachable on a phone screen.
- Minimal chrome: personal count near the button, leaderboard as a collapsible panel or side drawer so it doesn't compete with the tap target.
- Immediate feedback on tap (scale/bounce on the square or button) even before server confirmation — this is what makes clicker games feel responsive despite network round-trips.
- Square growth should be visually perceptible over a session but not so fast that it becomes visually chaotic — see growth curve note in 4.3.

---

## 8. Future Considerations (Not This Iteration)

Documented here only to make sure the MVP architecture doesn't accidentally block them later:

- **Reset/prestige cycles** — e.g., the Orange Man "pops" at some threshold and restarts, awarding some permanent currency. Would require a `season` or `epoch` field on `GlobalState` and historical leaderboard snapshots.
- **Food variety** — different food types with different feed values or visual effects.
- **Real character art** — replacing the orange square with an actual illustrated/animated character once the loop is validated.
- **Idle/passive generation** — feeds-per-second upgrades, common in idle games.
- **Social features** — friends lists, guilds/teams, regional leaderboards.
- **Anti-abuse** — server-side rate limiting on `feed()` if bot-tapping becomes a problem.

---

## 9. Open Questions for the Team

- Should `total_feed_count` (driving the square) be truly global across all players, or should there be per-session/per-room instances? (MVP assumes one single global shared square.)
- What identity system will back `Player.identity` — is anonymous SpacetimeDB identity sufficient for MVP, or is a lightweight name/handle needed at launch for the leaderboard to feel personal?
- Confirm jidousha's actual subscription/reducer-call idioms directly from the repo before implementation, since its API conventions weren't available to verify here.
