# giri — screen captures

The set UI.md §8 asks for, as `tools/verify giri` last wrote it: one PNG per
screen mode at the reference surface and at a narrow one. They are committed so
the owner can review the screens from the pull request — the owner judges layout
from screenshots, and playtests are for feel.

**Regenerated, not hand-made.** `tools/verify giri` writes them to
`target/verify/giri-<name>.png`; these are copies. A change to the screens
means re-running verify and copying the new set, in the same commit as the
change.

| File | Screen | Surface |
|---|---|---|
| `board-reference.png` | the board, a quest taken, nobody staged | 1920x1080 |
| `staged-reference.png` | the board, a powder-keg party staged, band chip up | 1920x1080 |
| `resolution-reference.png` | the resolution takeover, with a betrayal in it | 1920x1080 |
| `board-narrow.png` | the same board | 600x540 |
| `staged-narrow.png` | the same staged board | 600x540 |
| `resolution-narrow.png` | the same takeover | 600x540 |
| `tuning-reference.png` | the tuning drawer, a preset pending | 1920x1080 |

The tuning drawer has no narrow capture, and that is a decision rather than an
omission: it is a dev surface rather than a screen mode, its stepper rows are
the smallest type in the game, and at 600x540 the one picture of it would be the
one nobody in this set could read (UI.md §12).

The narrow set exists to catch scaling regressions (UI.md §6): the whole
960x540 design has to be on screen, uniformly scaled, letterboxed symmetrically.
A narrow capture whose text is the same size as the reference one's is the
defect this round fixed, come back.
