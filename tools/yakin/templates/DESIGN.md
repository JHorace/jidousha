# DESIGN — <task-id>

<!-- The cross-model handoff (DESIGNER.md §4). A designer tick fills it; an
     implement tick of a different model builds from it cold, with no memory of
     the designer and nobody to ask. Every section is mandatory: one with nothing
     to say says why in one line. Delete these comments once filled. -->

task: <task-id>
variant: <V2|V3>
model-as-configured: <the routine's model setting, as the session's context states it; "unknown" if none>
date: <output of: TZ=America/Los_Angeles date '+%Y-%m-%d %H:%M %Z'>

<!-- model-as-configured is the routine's SETTING, not proof of what ran: a
     routine can be served by a fallback model without saying so. The run page
     at claude.ai is the authority on which model this session ran. Never write
     this field as verified. -->

## What the game is

<!-- Three sentences, no more. A system task: what the system is. -->

## The player-facing loop

<!-- What the player does, sees and gets, moment to moment, in order. A system
     task: who invokes it, what they do, what they get back. -->

## Systems

<!-- One bullet per system: what it does · the file it lives in (inside the
     spec's Fence) · the `docs/api/` surface it touches, by file and item name
     as that file names them. In build order. -->

- <system> — <what it does> · `<path>` · touches `<docs/api file>`: <items>

## Gates to add

<!-- Concrete and deterministic: each names the check (a --verify assertion, a
     test named as a sentence), the seeded input it runs on, and the exact
     outcome it asserts. Every `## Done when` line of the spec maps to at least
     one gate here; say which. No wall-clock, no "looks right". -->

- <gate> — input: <seed / scripted input> · asserts: <exact outcome> · covers Done-when: <line>

## Non-goals

<!-- What this build deliberately leaves out, including anything cut so the
     implement stage fits the window and anything that would need an engine
     change (name its FINDINGS entry). -->

## Decisions already made

<!-- Settled here; the implementer does not relitigate them. Each with its one-
     line reason. A game task's decision surfaces come from the spec (make-game
     §D) — elaborate, never invent. -->

## Open calls delegated to the implementer

<!-- Choices left to the implementer on purpose, each with any constraint it
     must respect. "none" if none. -->
