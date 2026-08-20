# ADR-0031: A `HeadlessSim` does not fork; a check rolls the game's own functions forward

Status: accepted · 2026-08-20 · **extends ADR-0022's reasoning to the check surface**

> **`HeadlessSim::fork` is declined for v1, and the sentence that implied it is
> qualified.** "Simulate rather than solve" means rolling *your game's* step
> forward — the pure functions the game owns — not copying a running world.
> Nothing is added to the API; what changes is that three documents now say
> which of the two they meant, and Concepts says it early enough to act on.

## Context

E0 run 9's controller needed to ask "where does the ball end up if I hit it
here", thirteen candidate futures deep on every decision, and read *Testing your
game* as an offer:

> So simulate rather than solve: running the game forward and looking is
> allowed, and it is usually both simpler and more honest than a closed form
> kept in step by hand.

It then found there is no way to do that. `HeadlessSim` exposes `tick`, `draw`,
`world`, `world_mut` and `schedule_debug`; `World` is not `Clone`; `Recording`
replays *input*, not state; and rebuilding from `headless(..)` and replaying to
the current tick is quadratic and hopeless at tick 2,000. Its log:

> So "run the game forward" resolves in practice to "re-implement the game's
> step in the controller and hope it stays in step" — which is the closed form
> the sentence warns against.

**The run is right about the sentence and wrong about the offer.** The number in
that same paragraph — thirteen futures, four hundred ticks each, two seconds for
a whole `--verify` — was measured on `examples/slalom`, whose controller rolls
`gate_center_at` forward, a pure function the *game* owns. Every worked
controller in this repository does that. No document says so, so a reader with a
`&mut World` in front of them reads "run the game forward" as "run the
simulation forward", which is the one reading the surface does not support.

**What a fork would cost, from inside.** `Component` and `Resource` are
`'static + Send + Sync` and nothing more (ADR-0002, ADR-0006); components live in
type-erased archetype columns and resources in a `Vec<(TypeId, Box<dyn Any + Send
+ Sync>)>`. Copying a world therefore needs a `Clone` bound on **both traits** —
a breaking change to every game's `impl Component for Ball {}` — plus a
per-type clone shim carried in the column vtables. That is not a helper; it is a
change to what a component *is*.

And the RNG makes it worse rather than better. `Rng` is a resource seeded from
`GameConfig::seed`, and determinism here is "same seed + same operation history →
same everything" (ADR-0009, core.md). A fork that *shares* a stream is a
simulation whose result depends on how many futures the controller happened to
explore — the exact hazard ADR-0009 exists to prevent. A fork that *copies* the
stream gives every candidate future the same die rolls, which is correct for a
rollout and is a subtlety nobody would infer from the name `fork`.

## Decision

**No `HeadlessSim::fork`, no `World: Clone`, no snapshot/restore in v1.** The
absence is documented as a boundary with the shape to write instead — the
treatment ADR-0022 gave `Rect::sweep` and F-027 gave `App::quit`, both of which
E0 runs have since called the right way to document an absence.

Concretely:

1. *Testing your game* says which "forward" it means, in the paragraph that makes
   the offer, and names the third option: the game's own step functions.
2. **Concepts carries the requirement**, because that is where it is actionable.
   A game whose opponent decision and collision response are free functions
   costs nothing to write and can be rolled forward by a check; the same game
   with those branches inside `Update` systems is a retrofit (F-113).
3. `docs/api/jidousha-controllers.md` keeps the pure-function requirement it
   already had, unchanged. It is the same rule stated where the reader who is
   *using* it stands.

## Rationale

The decision turns on a fact about the shape of the answer rather than on cost.
A controller does not want a copy of the world; it wants **one number** — where
the ball crosses, where the opponent will be. Rolling a whole simulation forward
to get it runs every system the game has, including drawing, scoring and input,
for a question that four lines of arithmetic answer. `examples/slalom`'s
controller is thirteen futures deep and does not tick anything.

Where a fork would genuinely win is a game whose step is *not* separable — a
soup of systems mutating shared state, where no pure function can be carved out.
That game exists, and v1 is not for it: the whole `--verify` convention assumes a
game small enough to reason about.

The pure-function requirement also pays twice. It is what makes the check
possible, and it is what makes the game's own model testable directly — run 9's
`rebound` and `opponent_target` each got a unit test asking the function its
contract, and one of those caught the fault a full session could not (run-9 log
§11).

## Consequences

- The offer sentence becomes a two-way statement: roll the game's functions
  forward, and do not expect to copy the sim. A run that reads it and looks for
  `fork` finds the answer in the same paragraph rather than in the reference.
- **The requirement moves earlier without moving the advice.** ADR-0030 put the
  controller *strategy* in its own document and told the reader to read it third.
  That is right, and it is what filed this requirement where a first-time author
  meets it after paying for it. One sentence in Concepts — a fact about how to
  structure a game, not about how to play one — closes the gap with ADR-0030's
  split intact. ADR-0030 is not superseded and nothing moves between documents.
- If a later version wants a fork, the entry cost is the `Clone` bound and the
  question this ADR raises about the RNG stream, both of which are cheaper to pay
  before v1 has games in the wild than after. That is written down here so the
  next conversation starts from it.

## Alternatives rejected

- **`HeadlessSim::fork(&self) -> HeadlessSim`.** The API run 9 asked for. Costs
  a `Clone` bound on `Component` and `Resource`, a clone shim per column, and a
  decision about the RNG stream that has no answer a reader would guess. Bought
  for a use nothing in the repository has: no worked controller ticks a sim.
- **Snapshot and restore through the encoding that already exists.**
  `InputSnapshot` has a hand-written codec (ADR-0014), so "encode the world, run,
  decode it back" looks free. It is not: that codec is for *input*, and a world
  codec would need one per component type — the `Clone` bound again, wearing
  `serde`'s hat, which ADR-0014 declined for its own reasons.
- **Say nothing and let the next run find out too.** Run 9 lost the time and
  wrote the workaround; a run that does not think of the workaround writes the
  closed form the paragraph warns against, and the document will have caused it.
