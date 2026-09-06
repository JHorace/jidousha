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
| `ninjo-map-reference.png` | the map at world-minute 44 — two parties mid-travel on visibly different routes, with each site's open-job count under its marker | 1920x1080 |
| `ninjo-feed-reference.png` | the feed at the first completion, with the world stopped: the reason line, and the entry that caused it ringed in gold | 1920x1080 |
| `ninjo-living-reference.png` | world-minute 400 — **nobody was told to go anywhere**, and five of the ten are on the road because they decided to be | 1920x1080 |
| `ninjo-roster-reference.png` | the roster drawer: everyone, their chips, their purse, their desperation, and what they are doing with the reason they are doing it — with a trait chip tapped and its explanation on the row above | 1920x1080 |
| `ninjo-person-reference.png` | Steve's panel, opened by clicking his figure, with the `caring` chip tapped and the line that chip derives at the foot of the sheet | 1920x1080 |
| `ninjo-board-reference.png` | **the Deep Cave's job board**, read by Alex: six rows of name, task chip, pot, duration and state, with his fit for each and the travel from his own door — the one open row is the scout job, and it is the one row he is good at | 1920x1080 |
| `ninjo-ordered-reference.png` | **an order given from a job row**: the Old Crypt's board a few minutes after the far survey was tapped, that row now reading `Alex has it` and Alex on the road for it | 1920x1080 |
| `ninjo-bounce-reference.png` | **the bounce on a claimed row**: the mushroom haul, which Steve took at minute 32, tapped again — the toast under the bar says why and nothing else moves | 1920x1080 |
| `ninjo-map-narrow.png` | the same map | 600x540 |
| `ninjo-feed-narrow.png` | the same feed | 600x540 |
| `ninjo-selection-reference.png` | the owner's double-selection reproduction, as it now resolves: Bob picked on the party strip, then Tim picked by his map sprite — **one** ring, on Tim, his panel open and his chip lit, and nothing on Bob | 1920x1080 |
| `ninjo-tuning-reference.png` | the tuning drawer, MIRE pending in gold, APPLY lit | 1920x1080 |

The settlement, config, living, roster, character, board, ordered, bounce and selection shots are reference-only: they are
pictures of *what is on screen*, and the scaling defects the narrow surface
exists to catch are on the same chrome the map and feed pairs already cover.

The narrow set exists to catch scaling regressions — the chrome fits the
view uniformly and centred (`camera::UiMap`), and a defect there is
invisible to every assertion that is not about pixels.
