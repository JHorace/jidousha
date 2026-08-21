# ADR-0033: A game that expects to be tuned puts its numbers in a resource; nothing is added to configure one from outside

Status: accepted · 2026-08-20 · **extends ADR-0031's "simulate rather than solve" to the sweep**

> **No `GameConfig` passthrough, no `tools/verify` parameter, no engine change at
> all.** What lands is a recommended shape, in the document that makes the offer:
> a game with numbers it intends to sweep declares them as one `Copy` resource
> with the shipped set as an associated constant, and its `--verify` mode inserts
> a candidate before the first tick. Below the bar where that pays, rewriting the
> source between runs is blessed rather than tolerated.

## Context

*Testing your game* opens with the best-received paragraph in the three
documents — E0 run 10 called it "the single most valuable thing" — and it stops
one step short:

> **A tick is cheap, and thousands of them are not something to budget for.** …
> So simulate rather than solve: running the game forward and looking is allowed.

A run that takes the offer immediately wants to run the game forward *under
different numbers*, because that is what tuning is. Run 10 did exactly that,
across four measured rounds, and found the shape in its way:

> every one of those rounds is a recompile, because the constants are `const`s
> in the game. That is correct Rust and the right thing for determinism, and it
> still means a tuning sweep is a shell script that rewrites the source file with
> a regex between runs, which is what I did. Nothing in the surface offers a knob
> — no `GameConfig` passthrough, no way for `--verify` to take a parameter.

**The run is right about `const`s and wrong that there is no knob.** There are
two facts about the surface that a game author standing outside it cannot see,
and together they are the whole answer:

- `headless(config, setup)` builds a **fresh** `Simulation` every call and holds
  no global state beyond an idempotent panic hook. Forty candidate settings are
  forty sims in one process. The cost of a candidate is a build plus its ticks,
  not a `cargo build`.
- `Startup` runs *inside* the first tick (`schedule.rs`, and Concepts says so),
  and `HeadlessSim::world_mut` is available before that tick. So there is a
  window between building a game and running it in which a harness can put a
  resource into the world, and `Startup` will find it there.

Neither is new. Both are already documented, one sentence apart, in files a run
reads — and no run has put them together, because nothing asks it to. This is the
`Time::fixed_dt`-in-`Startup` shape run 10 also flagged: two true sentences in
different sections, and the reader left to do the join.

## Decision

**Nothing is added to the API.** What changes is that *Testing your game* carries
the join, as a worked shape, in the paragraph that makes the offer:

```rust
#[derive(Clone, Copy)]
struct Tuning { paddle_speed: f32, ball_speed: f32, ramp: f32 }
impl Resource for Tuning {}

impl Tuning {
    /// What the game ships with — the row the sweep chose.
    const SHIPPED: Self = Self { paddle_speed: 20.0, ball_speed: 42.0, ramp: 1.12 };
}

fn spawn_court(world: &mut World) {
    // Whatever a harness put here before the first tick, or the shipped set.
    let tuning = world.find_resource::<Tuning>().copied().unwrap_or(Tuning::SHIPPED);
    world.insert_resource(tuning);
    // … spawn the paddles and the ball from `tuning`
}
```

and the sweep is a `for` loop over `headless(..)` in the game's own `--verify`.

**And the recompile loop is blessed rather than deprecated.** A game with two
numbers should keep them `const`: a constant is checked at compile time, reads
with no indirection and can appear in a `const fn`, and a resource is none of
those. The bar the document states is a sweep you expect to run more than twice.

## Rationale

**The alternative shapes all put the game's numbers somewhere that is not the
game.** That is the through-line, and it is the same one ADR-0022 and ADR-0031
drew: the engine owns the substrate, the game owns its model. A tuning constant
is as far inside the game's model as a value gets — run 10's four rounds moved
paddle thickness, ball top speed and the rally ramp *together*, because in that
game they are one decision. An engine-side knob would have to either flatten that
to string key-value pairs or know what a Pong is.

**Putting them in a resource keeps them in the game and costs one indirection.**
The resource table already exists, `find_resource` already means "absence is
expected here", and the fallback is not a silent failure — "no override was
requested" is the ordinary case, and it is the case the person playing the game
in a window is always in.

**The `Copy`-struct-with-`SHIPPED` shape is chosen over a `Default` impl**
because ADR-0012 bans convenience `Default`s and because `SHIPPED` says something
`Default` does not: these are the numbers the sweep chose, not neutral ones.

**One consequence is worth stating as a gain rather than a cost.** A swept game's
best row is a value in its source, so the sweep and the shipped game cannot drift
— which the regex script cannot promise, since its last write is whatever the
last candidate happened to be.

## Consequences

- *Testing your game* gains the worked shape and the bar for using it. Concepts
  is untouched: this is a fact about checking a game, and the reader who needs it
  has a `--verify` mode in front of them already.
- Three tests in `crates/jidousha-core/tests/app.rs` pin the two facts the shape
  rests on — a resource inserted before the first tick is what `Startup` finds, a
  run that sets nothing gets the shipped numbers, and each `headless` call builds
  a fresh game. They are behavioural tests of documented behaviour, so the
  document cannot rot silently against the engine.
- **The pure-function requirement pays a third time.** ADR-0031 asks a game to
  write its step and its decision as free functions so a controller can roll them
  forward; a free function taking `&Tuning` is a swept game with no further
  restructuring. A game whose numbers are `const`s *inside* those functions has
  the retrofit F-113 describes.
- If a later version wants a sweep the *shell* can drive, the entry point is a
  game's own `--verify` argument parsing rather than anything in the engine, and
  this ADR is where that conversation starts.

## Alternatives rejected

- **A game payload on `GameConfig`** — `user: Box<dyn Any>`, or a type parameter.
  `GameConfig` is the driver's configuration: title, seed, timestep, window size,
  every field of it a thing the *engine* reads. A game payload there is a second
  resource table beside the resource table, reachable by exactly one downcast,
  and it would be the only place in the surface where a game hands the engine
  something the engine does not look at.
- **`tools/verify <example> --set ball_speed=42`.** `tools/verify` takes no
  arguments on purpose — it is the reproducible run, and a run parameterised from
  a shell is one whose result cannot be checked in. A sweep belongs inside the
  game's `--verify`, where its rows can be printed, asserted on and read back in
  a diff.
- **`HeadlessSim::with_resource(..)` as a builder step.** A second way to do what
  `world_mut().insert_resource(..)` already does, one line shorter, in a surface
  whose first rule is that there is one way. Declined on ADR-0012's grounds.
- **Bless only the source-rewriting script.** It works and it is honest, and it
  is what the document now recommends below the bar. As *the* answer it teaches a
  reader that the engine cannot do the thing the engine can do, and it leaves the
  shipped constants and the sweep's verdict in two places that a person keeps in
  step by hand.
- **Say nothing.** Run 10 asked for one of the two answers explicitly and said a
  reader "has to invent one of the two". The document made the offer; declining
  to say where it leads is what makes the next run invent it again.
