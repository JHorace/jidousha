# ninjo — screen captures

The set the fork's UI.md §5 asks for, as `tools/verify ninjo` last wrote
it. They are committed so the owner can review the screens from the pull
request — the owner judges layout from screenshots, and playtests are for
feel.

**Regenerated, not hand-made.** `tools/verify ninjo` writes them to
`target/verify/ninjo-<name>.png`; these are copies. A change to the
screens means re-running verify and copying the new set, in the same commit
as the change.

| File | Screen | Surface |
|---|---|---|
| `ninjo-settlement-reference.png` | the settlement at world-minute 0 — the whole cast standing at their home tiles, named, before anything is dispatched | 1920x1080 |
| `ninjo-modes-reference.png` | the auto-pause config, with `quest-complete` set to pause — the change this session's photographed run is stopped by | 1920x1080 |
| `ninjo-map-reference.png` | the map at world-minute 40 — three parties out, two mid-travel on visibly different routes | 1920x1080 |
| `ninjo-feed-reference.png` | the feed at world-minute 161, with the world stopped: the reason line, and the entry that caused it ringed in gold | 1920x1080 |
| `ninjo-person-reference.png` | Steve's panel, opened by clicking his figure, with the selection ring on it | 1920x1080 |
| `ninjo-map-narrow.png` | the same map | 600x540 |
| `ninjo-feed-narrow.png` | the same feed | 600x540 |
| `ninjo-tuning-reference.png` | the tuning drawer, MIRE pending in gold, APPLY lit | 1920x1080 |

The settlement, config and character shots are reference-only: they are
pictures of *what is on screen*, and the scaling defects the narrow surface
exists to catch are on the same chrome the map and feed pairs already cover.

The narrow set exists to catch scaling regressions — the chrome fits the
view uniformly and centred (`camera::UiMap`), and a defect there is
invisible to every assertion that is not about pixels.
