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
| `ninjo-settlement-reference.png` | the settlement at world-minute 0 — the whole cast standing at their home tiles, before anything is dispatched. **Ten figures for ten people since 2026-09-08**; before that it was the picture of the double-drawn cast, and all ten pairs were in it (`FINDINGS.md` G-023). **Named since 2026-09-09 only where a name fits**: the label rule drops the seven words that were landing on other people's heads, which is what G-022 measured and this is the after picture of | 1920x1080 |
| `ninjo-modes-reference.png` | the auto-pause config, with `quest-complete` set to pause — the change this session's photographed run is stopped by | 1920x1080 |
| `ninjo-map-reference.png` | the map at world-minute 44 — two parties mid-travel on visibly different routes, with each site's open-job count under its marker | 1920x1080 |
| `ninjo-feed-reference.png` | the feed at the first completion, with the world stopped: the reason line, and the entry that caused it ringed in gold | 1920x1080 |
| `ninjo-living-reference.png` | world-minute 400 — **nobody was told to go anywhere**, and five of the ten are on the road because they decided to be | 1920x1080 |
| `ninjo-roster-reference.png` | the roster drawer: everyone, their chips, their purse, their desperation, and what they are doing with the reason they are doing it — with a trait chip tapped and its explanation wrapped across the two rows above — the second of which is the dormancy clause the one-row clip used to eat | 1920x1080 |
| `ninjo-person-reference.png` | Steve's panel, opened by clicking his figure, with the `caring` chip tapped and the line that chip derives at the foot of the sheet | 1920x1080 |
| `ninjo-board-reference.png` | **the Deep Cave's job board**, read by Alex: six rows of name, task chip, pot, duration and state, with his fit for each, the travel from his own door, and — on the one open row — what the scorer says he would do about a posting there (`reluctant - good at scout work`). The footer carries the wage the next tap would offer and whom it would go to | 1920x1080 |
| `ninjo-ordered-reference.png` | **a posting agreed to**: the Old Crypt's board a few minutes after the far survey was posted to Alex, that row now reading `Alex has it` and Alex on the road for it — with nobody selected, so the board shows no fit and no verdict | 1920x1080 |
| `ninjo-bounce-reference.png` | **the bounce on a claimed row**: the mushroom haul, which Steve took at minute 32, tapped again — the toast under the bar says why and nothing else moves | 1920x1080 |
| `ninjo-map-narrow.png` | the same map | 600x540 |
| `ninjo-feed-narrow.png` | the same feed | 600x540 |
| `ninjo-selection-reference.png` | the owner's double-selection reproduction, as it now resolves: Bob picked on the roster (the door the retired party strip's chip used to be), then Tim picked by his map sprite — **one** ring, on Tim, his panel open and his name in gold, and nothing on Bob but his ordinary parchment label | 1920x1080 |
| `ninjo-roadring-reference.png` | **the ring on a token**: Bob picked off the roster at world-minute 52 while he is walking to the Watchtower — the one ring is on the figure on the road and his own doorstep stands empty, which is the selection's other state (UI.md §3b) and had no picture at all while the map drew everybody twice | 1920x1080 |
| `ninjo-declined-reference.png` | **being told no**: the vault door posted to Alex, who would rather scout — the world stopped by `ask-declined` and the banner carrying his reason, which is the first thing in this game that interrupts a player who has not asked it to | 1920x1080 |
| `ninjo-ledger-reference.png` | **the postings ledger**: a standing open posting for fight work that six people have answered, the refused posting above it with the reason on its second line, and the four standing rates with their steppers and their STAND buttons | 1920x1080 |
| `ninjo-breakdown-reference.png` | **a verdict's arithmetic, on a refusal** (UI.md §3e): the Old Crypt's board read by Alex with the `?` on the crypt seal tapped — every term of the sum with the trait row or the fact that produced it, the total, and the line saying what beat it (`staying home scores 3`). The thing a player could not account for before this session | 1920x1080 |
| `ninjo-feedwhy-reference.png` | **the same sum for a decision already made**: the feed with the `?` tapped on `Odd took the second seal at the Old Crypt` — `+6 need - desperation 3 / +4 want - renown / +6 aptitude - fighter / = 16 in all`, which is "why did they go there" answered after the fact from the entry itself | 1920x1080 |
| `ninjo-zoomed-reference.png` | **the settlement one notch of the wheel out**: every map word gone, every figure still there, and Bob still named in gold because the selected character's name is chrome (UI.md §4). The picture the owner judges the default camera against | 1920x1080 |
| `ninjo-picker-reference.png` | **a job's candidate picker, open with nobody selected** (UI.md §3c): the Deep Cave's deep survey, and the whole cast under it — best fit first, each with their fit for scout work, the journey from wherever they are standing, what the scorer says they would do about 20g, and where they are (Bob at the Black Vault until d1 10:28, four of them walking home). The picture of a board that can be aimed without reaching the map it is covering (`FINDINGS.md` G-026) | 1920x1080 |
| `ninjo-chosen-reference.png` | **the board after choosing from it**: Tim named — not the best fit on the list, which is the point — the footer reading `TO Tim`, his panel up beside the board, and the deep survey still `open`, because the picker names a person and the row is what posts | 1920x1080 |
| `ninjo-tuning-reference.png` | the tuning drawer, MIRE pending in gold, APPLY lit | 1920x1080 |

The settlement, config, living, roster, character, board, ordered, bounce,
declined, ledger, selection, road-ring, breakdown, feed-breakdown,
candidate-picker, after-choosing and zoomed-out shots are reference-only: they are
pictures of *what is on screen*, and the scaling defects the narrow surface
exists to catch are on the same chrome the map and feed pairs already cover.

The narrow set exists to catch scaling regressions — the chrome fits the
view uniformly and centred (`camera::UiMap`), and a defect there is
invisible to every assertion that is not about pixels.
