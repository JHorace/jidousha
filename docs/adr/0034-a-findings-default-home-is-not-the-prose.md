# ADR-0034: A docs finding's default home is the reference or a worked example; prose is for rules

Status: accepted · 2026-08-20 · **the budget answer ADR-0025 forecloses raising and ADR-0030 bought one split's worth of time for**

> **The token budget is not raised and no fourth document is split.** What
> changes is which of the three homes a finding goes to by default. A fact about
> one API item goes in that item's doc comment; a shape or a technique goes in a
> worked example with a pointer; prose is for a rule that spans the surface, and
> a rule that lands in prose displaces something. The trigger is that the novel
> finding count has a floor, so the prose grows for ever unless the default
> changes.

## Context

`jidousha-testing.md` stood at **14,452 tokens of 15,000** when this was written.
The arithmetic behind that number is new and it is what makes this a decision
rather than another squeeze.

**Run 10 established that novel findings have a floor** (§6, "Where the exercise
stands"). Runs 8, 9 and 10 each produced exactly three, across two document
splits, an ADR, twelve fixes and a change to the acceptance bar. At roughly 200
prose tokens a novel finding, that is ~600 tokens a run, for ever, against a
budget with 548 left. The document overflows on run 11 or 12 and every run after
it.

Every previous answer is spent. ADR-0025 split the surface in two and forbade
raising a budget in the same breath. Run 7's triage curated ~1,067 tokens out —
767 structurally from the reference (ADR-0028) and ~300 from prose — and recorded
that "the prose is now near its floor". ADR-0030 split a third document off and
bought back ~2,200, of which ~1,900 has since been spent. `public-api.md` §4 said
what a document filling again while E0 had not passed would mean, and it has now
happened twice.

**The residue also has a shape, which tells you where the growth comes from.**
§4g's neighbour diagnosis: five of run 10's six re-treads are a previous fix
answering the question asked and not the one beside it. A fix filed as a
paragraph answers one question in one place. A fix filed against the *item* or in
a *worked example* is where the reader with the neighbouring question is already
standing.

**And F-134 is the case against two homes, measured.** The capture recipe existed
twice — 44 lines of prose block, and `prototype_kit/capture.rs` working the same
path. They drifted: the document's copy acquired a `?` that cannot compile in the
function it is in, and two `expect` calls in a document that denies `expect_used`
two sections earlier, while the worked example used neither. Nobody noticed for
six runs. That is what a second home costs, and it is the reason this ADR is
about *destination* rather than about *volume*.

## Decision

**A `docs` finding goes to the first of these that fits, not to prose by default.**

1. **A fact about one API item → that item's doc comment.** It costs the
   generated one-liner and nothing else, and it is where a reader holding that
   item is looking. F-132 (`Rect::from_center_size` with a negative `size`) and
   F-133 (`Rng::next_f32`'s endpoints) went here, at ~15 and ~10 tokens against
   the ~200 a paragraph would have cost.
2. **A shape or a technique → a worked example, with a pointer from the prose.**
   The example carries the code and the prose carries the sentence that says why
   to read it. F-131's margin fold is worked in `prototype_kit`; F-130's
   requirement-not-the-constant instances are worked in `slalom/checks.rs`.
3. **A rule that spans the surface → prose**, and something else leaves.

**The test for (3)** is §7's existing one — is this about writing a game rather
than about this API — plus: *would a second worked instance say what a second
paragraph cannot?* If yes, it is (2). If the rule has no single site because it
applies everywhere, it is (3).

**Prose keeps the rule and gives up the copy.** Where a worked example already
walks a path, the document's job is the sentence a reader cannot derive from the
code — the trap, the reason, the thing that cost somebody a cycle — and not a
second transcription of the path.

## Rationale

**The floor is what makes this structural rather than another squeeze.** Every
prior answer treated a full document as a curation problem with a one-time fix.
Three runs at exactly three novel findings say the input rate does not fall, so a
one-time fix buys two runs and no more. The only thing left to change is the
per-finding cost, and the three destinations already exist — what was missing was
a rule about which is the default.

**Destination (1) is nearly free and was always available.** The reference is
generated from doc comments, so a clause there costs the document the length of a
summary line and costs `docs/api/` nothing structural. It was underused because
"a finding" and "a paragraph" had become the same thought.

**Destination (2) spends measurement, and the floor devalues what it spends.**
`crates/jidousha/examples/` is on the E0 run's allowed list (F-020), so a worked
example is a thing every future author may read, and §6 records what spending run
4's lever cost: the exercise can no longer measure whether prose *alone* teaches
the controller lesson. That was a real price when the novel count still looked
like it might walk to zero. It looks different against a floor: preserving the
ability to measure "can prose alone teach X" preserves the ability to measure
something that has stopped varying, while the budget is a hard constraint with a
reader cost run 10 measured at 3,268 lines of entry and called "the honest cost of
entry".

**Two homes for one lesson is not neutral, which §7 argued the other way.** §7
declined the `make-game` skill four times on the grounds that two homes for one
lesson is worse than one crowded home, and that argument stands — but it is an
argument *for* this decision rather than against it. Moving a shape to an example
does not create a second home; it moves the one home to where the code is and
leaves a pointer. F-134 is what the alternative looks like after six runs.

## Consequences

- **The pilot is the capture recipe**, chosen because F-134 named it rather than
  because it was convenient — and **it recovered 63 tokens, not the ~200 it was
  projected to.** That result is worth more than the tokens.

  The block transcribed `prototype_kit/capture.rs` in 44 lines and now shows the
  shape in 33: the `use` list, the offscreen handshake with its `NoAdapter` arm,
  the texture-id check, and a sentence pointing at the worked file for the three
  steps that follow. What did *not* move is the passage's prose — the three traps
  — and checking `capture.rs` is why. Every trap is documented at the worked site
  too, on `CAPTURE_SIZE` and in the poll loop, in the same words. By the rule
  above that looks like duplication and it is not: a trap is "the sentence a
  reader cannot derive from the code", and the reader who most needs it is the
  one who has not opened the example. **Prose keeps the trap and gives up the
  path**, and in this passage the path was the small half.

  So the honest reading of the pilot is that **this rule changes the rate at
  which prose grows, not its level.** It stops each new finding costing a
  paragraph; it does not claw back a document that is already 96% full. The
  recovery needed for that is still a fourth split or a curation pass, and this
  ADR does not pretend to have done either.
- **F-128 stays in prose, and the first pilot chosen for it was wrong.** Putting a
  game's tunable numbers in a resource is a rule about how to structure a game —
  the same class as "write your step as free functions", which lives in Concepts
  with no worked instance either. Neither `prototype_kit` (a drawing showcase with
  no difficulty) nor `slalom` (whose drift is threaded through `gate_center_at` as
  constants, and which exists to be the controller lever) wants a tuning sweep,
  and adding one would invent a use. **A rule with no honest example home is a
  rule, and belongs in (3).** Recorded because the temptation to force a pilot is
  the failure mode of this ADR.
- **`docs/internal/e0-findings.md` §7 gains the rule**, since that is where a
  triage decides where a fix goes.
- The budgets are unchanged: 25k, 15k, 5k. Raising one stays foreclosed, and this
  is the third answer tried before it rather than the first argument for it.
- **If the prose still fills after this**, the remaining lever is a fourth split
  and the entry cost run 10 named is the thing to weigh against it. That
  conversation starts here rather than from scratch.

## Alternatives rejected

- **Raise the 15k budget.** ADR-0025 forecloses it, and the reason holds: the
  budget is the point, because the document has to fit in a game-writing agent's
  context beside the game. A number raised once is a number raised again.
- **Split a fourth document.** ADR-0030's split worked and its own risk note said
  what splitting costs: a run must find another file, and pointers are the hazard.
  Run 10 read 3,268 lines across three files before writing a line and called it
  the honest cost of entry. A fourth raises that for every future run, to buy room
  the floor will consume in three runs. It is the answer *after* this one, not
  instead of it.
- **Move findings to the `make-game` skill.** §7 has declined this four times and
  the strongest reason is unchanged: the skill is written after E0 passes, and it
  is either invisible to a run or it changes what E0 measures.
- **Cap prose with a one-in-one-out rule.** Holds the number and picks what leaves
  by seniority rather than by value — the oldest paragraph is not the least useful
  one. Destination is a better discriminator than age.
- **Do nothing and curate again when it overflows.** What the last three answers
  did. Each bought two runs, and the floor says the next one buys two more.
