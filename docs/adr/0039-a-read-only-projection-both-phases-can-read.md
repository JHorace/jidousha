# ADR-0039: `World::view` — one reader for a projection both phases need

Status: accepted · 2026-08-22

## Context

A game's UI is nearly always a picture of some projection of the world — a
roster, a scoreboard, the preview of what a move would do — and the same
projection is what the rules are applied to. Reading it in one place is not a
tidiness preference: giri's DESIGN makes it an invariant, because a willingness
preview that disagreed with the resolution would be the game lying to the
player.

The surface makes that reader impossible to write once. `World::query` and
`WorldView::query` are separate inherent methods on separate types; there is no
trait either implements, no `From<&World> for WorldView`, and no way to make a
`WorldView` outside the Draw phase — `WorldView::new` is `pub(crate)`. So a
projection read by an Update system and a Draw system is written twice:

```rust
pub fn read(world: &World) -> Self { /* eleven lines */ }
pub fn view(world: &WorldView<'_>) -> Self { /* the same eleven lines */ }
```

That is giri's `Social`, character for character identical apart from the
receiver, both feeding one `assemble` (games/giri/FINDINGS.md G-001). The
duplication is **forced by the surface rather than chosen**, which is what makes
it an engine problem: no game can avoid it, and a game whose UI *is* the
projection meets it on day one.

The reporting run was careful to say the shape of the fix is a decision rather
than an omission, and named two candidates. This records which, and why the
third — doing nothing — was not it.

## Decision

**`World::view(&self) -> WorldView<'_>`.** One method, on the type a game
already holds, returning the type Draw systems already receive. A game writes
one reader taking `&WorldView<'_>` and calls it from both phases:

```rust
fn read_the_field(world: &WorldView<'_>) -> Field { /* once */ }

fn orbit_the_anchor(world: &mut World) {          // Update
    let field = read_the_field(&world.view());
}

fn draw_the_field(ctx: &mut DrawCtx) {            // Draw
    let field = read_the_field(&ctx.world);
}
```

- It lives in `draw.rs`, beside `WorldView`, so the module dependency keeps
  running one way: `draw` knows `World`, `World` does not know `draw`.
- **ADR-0008 is untouched, and that is checked.** Read-only here is the *type*:
  `WorldView` has no method that mutates, so a view cannot become a write
  wherever it is handed. `tests/compile-fail/view_query_cannot_write.rs` pins it
  — `world.view().query::<&mut T>()` fails with the same
  `#[diagnostic::on_unimplemented]` sentences the Draw-phase snippet asserts, so
  the new door and the old one give the same compile error.
- It is not a second way to read a world. `World`'s own methods are what an
  Update system uses to *change* things; a view is what a reader that changes
  nothing takes, and the reason to take one is that a Draw system can hand you
  the same thing.

## Rationale

- **The type is already right.** `WorldView` is a `&World` plus a bound, and
  `WorldView::query` returns `QueryIter<'w, Q>` borrowed from the world rather
  than from the view — so a view made from a `&World` behaves exactly as one
  made by `DrawCtx`, with no lifetime machinery reaching a game's signature.
- **A game's signature is the thing to protect.** The reader's type is
  `&WorldView<'_>` — one concrete type, greppable, copyable from an example.
- **It closes the gap where the gap actually is.** The duplication was not
  eleven lines of tedium; it was two collectors that can drift apart. One
  reader cannot drift.
- **Nothing is weakened to get it.** The only new capability is *making* a
  read-only view, which was already reachable for the phase that most needed it
  to be safe.

## Alternatives rejected

- **A `Read` trait implemented by both `World` and `WorldView`.** It would work,
  and it puts a trait bound in every game's reader signature — `fn read<R:
  Read>(world: &R)` — for a surface whose whole premise is that a game agent
  copies signatures out of a document. It also adds a second way to spell
  "query a world" (through the trait, or inherently), which the conventions
  forbid outright. The reporting run reached the same conclusion from the
  outside, before it could see any of this.
- **`WorldView::from(&World)` / `WorldView::new` made public.** Same capability,
  worse name: the game-facing question is "how do I get a view of *this*
  world", and the answer reads best on the world. A constructor also invites
  `WorldView::new(&world)` to be read as *building* something, which it is not.
- **Make `WorldView` `Copy` and pass it by value.** Prettier at the call site
  (`read(ctx.world)`), and a strictly larger change: `&WorldView<'_>` already
  works from both phases today, and adding `Copy` would be a second calling
  convention for the same reader. Revisit only if by-reference proves to cost
  something real.
- **Do nothing.** Defensible for eleven lines; not defensible for the failure
  mode. The two collectors are what a preview and a resolution read the world
  through, and the day they disagree the game shows a number it will not honour
  — with both halves compiling, both tested, and neither wrong on its own.

## Consequences

- `docs/api/jidousha-api.md` gains one method and one paragraph of Concepts;
  `WorldView` stops being a type a game never writes and becomes one it names
  in a signature, so it comes out of `tools/check-api-coverage`'s exemption list
  and `examples/headless_sim.rs` demonstrates it.
- giri's `Social::read`/`Social::view` collapse to one `Social::read`, and every
  Update-phase caller gains `&world.view()`. Beat outcomes are unchanged, which
  is the proof the two collectors really were identical.
