# restaurant-nemesis — a silly restaurant sim where bad service breeds nemeses

Kind: game · Size: L · Window: burn-down · Variant: **set by the wrapper that
runs it.** This spec is variant-neutral and has no queue row of its own: it
runs only through `restaurant-nemesis-r3v1` (V1) and `restaurant-nemesis-r3v2`
(V2), whose overrides rename its game, branch, run folder and PR title.

## Goal

A new game, `games/restaurant-nemesis`: a restaurant sim with a nemesis
system, silly in tone. The player runs service day by day, and some failures
are inevitable by design. A bad enough failure spawns a **problem customer**
who takes a themed title from the failure ("Mustard Monster" for a condiment
failure), gains a sensitivity that makes them harder to satisfy in that theme,
and keeps coming back to terrorize the restaurant — with a social-media
following that spreads their demands to ordinary customers. Prototype scope:
a multi-day run, a handful of failure themes, 2–3 concurrent problem
customers, win or lose on restaurant viability.

## Fence

- **Grants write access to `games/restaurant-nemesis/**`** (new) and
  `Cargo.lock` for that crate's own entry only, plus the run folder
  `tools/yakin/runs/restaurant-nemesis/`.
- Grants nothing else. No new dependencies: the manifest names `jidousha` by
  path and nothing else. Names, titles, dishes and jokes are written for this
  game; no text or art from any commercial game. Assets are made here
  (WORKER.md §5) or taken from the depot.

## The brief

- **Service, day by day.** Each day the restaurant serves a stream of
  customers with demands drawn from a handful of **failure themes** (the
  design picks 3–5 — condiments, temperature, wait time, portion size, …).
  Capacity is short on purpose: the player cannot satisfy everyone, so some
  failures happen whatever they do. Failures are seeded, never wall-clock.
- **Failure → problem customer.** Each failure has a theme and a severity. A
  failure past a severity threshold the design fixes spawns a problem
  customer. At spawn they get:
  - a **title** derived from the failure's theme, deterministic from the seed
    and silly ("Mustard Monster", "The Lukewarm Baron");
  - a **sensitivity** in that theme: their demands in it are harder to meet
    (the design says by how much);
  - a **return schedule**: they come back repeatedly, and each visit is a
    chance to satisfy them or fail them again.
  At most 3 problem customers are active at once; the design says what a
  spawn does when the cap is full.
- **Social media.** Each problem customer has a follower count, and the count
  sets their **tier** — their threat. Followers are themselves customers: a
  share of each day's ordinary customers are followers, and a follower adopts
  some of the leader's themed demands, so a problem customer is a themed,
  global difficulty modifier and the "big boss" of its own followers. The
  design fixes how followers grow (failing the leader, failing a follower),
  how tier follows from count, and what each tier does.
- **Defeat paths — all three must exist:**
  1. satisfy the problem customer **repeatedly** (a tally across visits);
  2. satisfy them **overwhelmingly once** (one visit far beyond their
     sensitivity, at a real cost to the rest of service);
  3. **reduce their follower count** to the point that ends them.
  A defeated problem customer stops visiting and their followers revert to
  ordinary customers.
- **Money and viability.** Service earns money; failures cost it (refunds,
  lost custom). **Managing social media is something the player spends money
  on** — a between-days spend that cuts a chosen problem customer's
  followers — and it competes with whatever else money buys (the design
  picks one or two other sinks, such as stock or staff). The design defines
  viability (money, reputation, or both); losing viability loses the run, and
  surviving the run's last day wins it.
- **Tone.** Silly. The titles, the complaints, the social-media posts are the
  jokes; the arithmetic underneath is plain and shown.
- **Presentation.** Text or simple 2D, per the house pattern for this kind of
  game — readable panels over spectacle.
- **House pattern.** Deterministic sim (seeded RNG for customers, demands and
  failures; replayable input — CLAUDE.md convention 4), a `--verify` mode with
  scripted players and transcript gates (make-game §A.4–A.5), the mutation
  round (§A.6), a captured picture (§A.7).

## The design

Whichever stage designs — the worker inline (V1) or the designer routine (V2);
the wrapper says which — writes `tools/yakin/runs/restaurant-nemesis/DESIGN.md`
from `tools/yakin/templates/DESIGN.md`, with its header's `variant` set to the
run's variant. There is no second design note under `games/`. Besides the
template's sections, it fixes, with numbers:

- the failure themes and how a day's customers and demands are generated;
- failure severity, the spawn threshold, and what a spawn does at the cap;
- title derivation and the sensitivity's size;
- the return schedule;
- follower growth, the follower → tier mapping, and what each tier does to
  the day's customers;
- each defeat path's condition;
- the money sinks, what a social-media spend buys, and viability;
- the run length and the win and lose conditions.

Anything that will not build in the window goes to Non-goals (DESIGNER.md §3,
"Size", binds whichever stage designs): the scope line above is the floor.

## Non-goals (this round)

The owner knows these are the fun part; they wait for a later round. A design
that reaches for them is out of scope:

- **Problem customers interacting with each other** — no alliances,
  rivalries, shared followers, merged bosses or combined demands. Each problem
  customer acts alone on the restaurant.
- **Mutation over time** — a problem customer's theme, title and sensitivity
  are fixed at spawn. Only their follower count, and the tier it sets, changes.
- Multiplayer, real-time cooking or dexterity play, meta-progression between
  runs, more than 3 concurrent problem customers.

## Decisions this task adds

| decision | must know | surface | action | one function | asserted by |
|---|---|---|---|---|---|
| Which waiting order to spend the next unit of capacity on | the waiting customers and their demands, including themed demands adopted from a leader; which are problem customers or followers, and whose; for each order left unmet, its failure theme and severity and whether that failure would spawn a problem customer | the service screen, with each order's at-risk outcome stated on the order while choosing | assigning capacity to an order (the design names the input) | one order-outcome function (satisfaction, failure theme and severity, spawn or not) that the on-order preview and the resolution both read (the design names it) | a scripted `--verify` run on a fixed seed whose unmet order crosses the spawn threshold: the transcript shows the severity and the spawn warning before the choice, then a problem customer spawns with a title derived from that theme and a sensitivity in that theme |
| How to serve a returning problem customer: bank one more satisfaction toward the tally, or go all-out for the overwhelming win | their title, theme and sensitivity; their tier and follower count; their progress on each of the three defeat paths; what going all-out costs the rest of service | a nemesis card shown while the problem customer is being served | choosing how to serve them (the design names the input) | one defeat-progress function that the card and the defeat resolution both read | three scripted `--verify` runs, one per defeat path (repeat tally, overwhelming visit, followers cut); each asserts the card showed the progress before the deciding act and the problem customer is defeated on exactly the visit or day the function says |
| How to spend money between days: social media, or another sink | each problem customer's follower count and tier; what share of tomorrow's customers are their followers and which themed demands they carry; what a spend would remove; the viability margin | the between-days ledger screen | choosing the day's spends (the design names the input) | one follower function (spend → followers → tier → tomorrow's follower share and themed demands) that the ledger's preview and the next day's customer generation both read | two seeded `--verify` runs that differ only in one social-media spend: the preview's stated follower drop, tier and follower share are in the transcript at choosing, and the next day's customers differ exactly as stated |

The design elaborates these rows (it names inputs, panels and functions); it
does not add or drop a decision (DOCTRINE §6).

## Done when

- `tools/yakin/runs/restaurant-nemesis/DESIGN.md` exists and fixes every
  number §"The design" lists.
- A multi-day run plays end to end; scripted `--verify` runs reach both a win
  and a viability loss, and at least one of them has 2 or more problem
  customers active on the same day.
- `python3 tools/verify restaurant-nemesis` reports `pass` in
  `target/verify/restaurant-nemesis.json`, with a check for each decision row
  above.
- `git diff --stat origin/main...` names only `games/restaurant-nemesis/`,
  `Cargo.lock` (that crate's entry only) and
  `tools/yakin/runs/restaurant-nemesis/`.
- `python3 tools/build-web restaurant-nemesis && python3 tools/serve-web restaurant-nemesis --check`
  passes.
- A PR titled `[yakin:<variant>] <the running task's queue title>` is open
  with the WORKER.md body; its Deviations list every departure from DESIGN.md.
