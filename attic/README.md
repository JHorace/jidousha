# attic/

Retired prototypes (ADR-0038). Read, never compiled.

Excluded from the cargo workspace and from every tooling glob: nothing here is
built, linted, tested, or deployed, and nothing here is expected to compile
against the engine as it stands today. A prototype that stops earning its build
time moves here rather than being deleted — "what did that one look like?" is a
question that keeps being asked, and a branch nobody can name is not an answer.

Moving one out is a move back into `games/` plus whatever it takes to build
again. Nothing here is maintained.

## What is here

- **`giri/`** — the four-beat social prototype (2026, waves P1-P3). It proved
  the thing ninjo is built on: trait-driven willingness makes characters feel
  like people. It also proved its own limit — four beats cannot absorb a bad
  outcome, and a table of simultaneous numbers is not a legible way to meet
  one (`giri/DESIGN.md` §0 records that verdict in its own voice). The fork it
  produced became **`games/ninjo/`**, which is giri's successor: the same
  social machinery, delivered through a world with time and places in it.
  Moved here by wave 1.1, whole — `DESIGN.md`, `FINDINGS.md`, `UI.md`,
  `VARIANT.md` and its screenshots are the ledgers it is kept for, and several
  ADRs still cite it by name. Nothing in ninjo depends on it; the port was
  copy-adapt, and the workspace builds with it gone.
