# keifu-x-inheritance — fork Keifu and heighten its inheritance identity

Kind: game · Variant: V2 · Size: L · Window: burn-down (the handoff said
"night"; `burn-down` is the schema's value for the Wednesday window)

## Goal

Fork Keifu into a variant, `games/keifu-x-inheritance`, that diverges freely
toward what makes Keifu distinct — a family line whose name, renown and blood
carry forward — while mainline `games/keifu` stays a faithful port of lineage.
The variant adds three things: a heritable black mark for failed personal
quests, a real distinction between family and outsider characters, and
rudimentary trait genetics.

## Owner ruling carried by this spec

Owner ruling 2026-10-06, carried here because DOCTRINE.md predates it:
**mainline `games/keifu` is never touched by this task.** The fork exists so
mainline stays a faithful port while this variant diverges. This task reads
mainline (docs and source) and writes only its own folder.

## Fence

- **Grants write access to `games/keifu-x-inheritance/**`** (new) and
  `Cargo.lock` for that crate's own entry only, plus the run folder
  `tools/yakin/runs/keifu-x-inheritance/`. The workspace already globs
  `games/*`, so no manifest edit is needed or allowed.
- **`games/keifu/**` is read-only** for both stages: the designer and the
  implementer read it whole; neither writes a byte of it.
- Grants nothing else. No new dependencies: the copy's manifest names
  `jidousha` by path and nothing else, exactly as mainline's does.

## Design stage (the designer routine, DESIGNER.md)

Before designing, **read mainline Keifu's docs and its source, not just
`docs/api/`**: `games/keifu/spec/SPEC.md` (§3 state, §11.6 courtship, §12
bonds, §13–15 destinies, legacies, death pages and inheritance),
`spec/CONSTANTS.md`, `SPEC-GAPS.md`, `FINDINGS.md`, and the modules those
sections name (`heirs.rs`, `household.rs`, `legacy.rs`, `hero.rs`,
`newcomers.rs`, `wanderer.rs`, `quest.rs`, …). This is a delta design: the
designer must know exactly what the variant deviates from, and DESIGN.md
states each delta as "mainline does X; the variant does Y". Reading
`games/keifu/` source is in-fence for this task (it is a game, not
`crates/**`); DESIGNER.md §3's ban on engine source still holds.

Design these three, scoped to what the night's remainder can build — cut
anything bigger into Non-goals (DESIGNER.md §3, "Size"):

- **The black mark.** A failed personal quest becomes a black mark on the
  family name: heritable, passed down repeatedly. Design how marks stack,
  how (and whether) they decay across generations, and how they interact
  with renown — house renown and personal renown both. Mainline has no
  thing called a personal quest; its nearest analogue is a hero's dream
  (SPEC §9). DESIGN.md says what a personal quest is in the variant and what
  counts as failing one.
- **Outsiders and family.** Outsiders are cheap bodies but confer no family
  renown. An outsider accrues personal renown that does not transfer to the
  house; marrying into the family requires that personal renown to reach a
  threshold. Design the threshold, what changes on marrying in, and what an
  outsider's death or departure leaves behind.
- **Trait inheritance.** Broader trait inheritance — rudimentary genetics.
  Design which traits pass, how two parents' traits combine, and how much
  chance is in it (seeded, so a run replays).

Open calls go to the implementer explicitly, per `templates/DESIGN.md`'s
"Open calls delegated to the implementer" section — nothing left implicit.

## Implement stage (the worker routine, WORKER.md)

1. **First act, and its own commit:** copy `games/keifu` to
   `games/keifu-x-inheritance` as a new workspace member and rename the crate
   and its ids (package and binary name, `tools/verify` / `tools/build-web`
   name, window title, any asset or path string naming `keifu` that would
   collide or mislead). That commit changes nothing else, and
   `python3 tools/verify keifu-x-inheritance` passes on it — so a reviewer can
   read every later diff as divergence from a known-identical copy.
2. Then build DESIGN.md. Mainline's oracles that the variant's rules
   deliberately break are rewritten to the variant's rules, each named in the
   PR's Deviations; the rest stay as copied.
3. Gates per house pattern: deterministic sim, `--verify` transcript gates for
   every decision row below (make-game §A.4–A.5), the mutation round on the
   new checks (§A.6), pictures of the new surfaces (§A.7).

## Decisions this task adds

| decision | must know | surface | action | one function | asserted by |
|---|---|---|---|---|---|
| Whether to send a family member on a personal quest | the chance it fails; the black mark a failure would put on the name, and what that mark does to renown now and to heirs later | where the quest is chosen or seated, at the moment of choosing (the design names the panel) | the commit input of that surface (the design names it) | one function computing a failure's mark and its renown effect — the preview and the resolution both call it (the design names it) | a scripted `--verify` check that asserts the mark consequence is in the transcript before the commit, then fails the quest on a fixed seed and asserts the house carries exactly that mark |
| Whether to let an outsider marry into the family | the outsider's personal renown; the threshold; what marrying in changes (renown transfer, heir eligibility, traits passed) | wherever courtship or marriage is decided (mainline SPEC §11.6; the design names the panel) | the commit input of that surface | one eligibility function the panel and the marriage both read | a `--verify` check that shows a below-threshold outsider refused and an at-threshold one accepted, with the threshold and the outsider's renown in the transcript |
| Which heir to choose, given what they inherit | each candidate's inherited traits and inherited marks | the death page / heir choice (mainline SPEC §15) | choosing the heir, as mainline does | one inheritance function that both the page's preview and the birth/succession use | a `--verify` check on a fixed seed asserting the shown traits and marks equal the heir's actual state after choosing |

The design stage elaborates these rows (it names panels and functions); it
does not add or drop a decision (DOCTRINE §6).

## Done when

- `games/keifu-x-inheritance/` exists as a workspace member, and its first
  commit on the branch is the pure copy-and-rename with its verify green.
- `python3 tools/verify keifu-x-inheritance` reports `pass` in
  `target/verify/keifu-x-inheritance.json`, with a check for each decision row
  above.
- `python3 tools/verify keifu` still reports `pass`, and
  `git diff --stat origin/main...` names no path under `games/keifu/`.
- The diff names only `games/keifu-x-inheritance/`, `Cargo.lock` (that crate's
  entry only) and `tools/yakin/runs/keifu-x-inheritance/`.
- `python3 tools/build-web keifu-x-inheritance && python3 tools/serve-web keifu-x-inheritance --check`
  passes.
- A PR titled `[yakin:V2] <this task's queue title>` is open with the WORKER.md
  body; its Deviations list every departure from DESIGN.md and every mainline
  oracle the variant rewrote.
