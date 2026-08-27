# giri-rt — screen captures

The set the fork's UI.md §5 asks for, as `tools/verify giri-rt` last wrote
it. They are committed so the owner can review the screens from the pull
request — the owner judges layout from screenshots, and playtests are for
feel.

**Regenerated, not hand-made.** `tools/verify giri-rt` writes them to
`target/verify/giri-rt-<name>.png`; these are copies. A change to the
screens means re-running verify and copying the new set, in the same commit
as the change.

| File | Screen | Surface |
|---|---|---|
| `giri-rt-map-reference.png` | the map at world-minute 40 — three parties out, two mid-travel on visibly different routes | 1920x1080 |
| `giri-rt-log-reference.png` | the log drawer after the first quests completed, every row world-time stamped | 1920x1080 |
| `giri-rt-map-narrow.png` | the same map | 600x540 |
| `giri-rt-log-narrow.png` | the same log | 600x540 |
| `giri-rt-tuning-reference.png` | the tuning drawer, MIRE pending in gold, APPLY lit | 1920x1080 |

The narrow set exists to catch scaling regressions — the chrome fits the
view uniformly and centred (`camera::UiMap`), and a defect there is
invisible to every assertion that is not about pixels.
