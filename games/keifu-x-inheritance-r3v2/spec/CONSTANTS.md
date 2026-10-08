# Lineage — constants

Every number with meaning, with its source. Paths are relative to
`source/application/game/` unless they begin with `source/`. Where a value lives in a
`content/*.json` file as well, the JSON copy was extracted from the same line and the two
must agree; if they ever disagree, the source line cited here wins.

"Base" aptitude means the stored number (`Hero.aptitudes`). "Effective" aptitude means base
plus the phase adjustment, floored at 0 (`entity/entities/hero.jai:140-143`
`hero_aptitude`). The rules say which one each formula reads; getting this wrong changes
results.

## 1. Run shape and calendar

| Name | Value | Meaning | Source |
| --- | --- | --- | --- |
| `DOOR_YEARS` | 25 | Ordinary years before the Door. Years 1..25 are ordinary; the 26th summer ("the last summer") holds only the Door. | `lineage/door.jai:1` |
| `Calendar.DAYS_PER_MONTH` | 28 | Calendar granularity; only months (= seasons) matter to the game. | `calendar.jai:26` |
| `Calendar.MONTHS_PER_YEAR` | 4 | Months are seasons: SPRING 0, SUMMER 1, AUTUMN 2, WINTER 3. Only SUMMER and WINTER are ever visited. | `calendar.jai:16-21,27` |
| `days_until_final_day` | `DOOR_YEARS * 112` = 2800 | Final day = start + 2800 days, i.e. year index 25, month SUMMER. | `game.jai:13`, `calendar.jai:35-41` |
| calendar start | year index 0, month SUMMER, day 0 | `current_year = today.year + 1`, so the first summer is year 1. | `calendar.jai:36`, `lineage/household.jai:7-9` |
| `years_until_door` | `final_day.year - today.year` | 25 in year 1's summer; 0 in the last summer. The Door "stands open" when this is <= 0. | `lineage/door.jai:52-58` |
| `HOUSE_RENOWN_AT_START` | 15 | House renown at founding. | `lineage/household.jai:55,60` |
| house closes at | renown <= 0 | Checked only when leaving the summer telling (see SPEC). Renown is floored at 0 by every change. | `lineage/tale.jai:34-40`, `scene/scenes/telling.jai:184-196` |

## 2. Aptitudes, phases, vocations

| Name | Value | Meaning | Source |
| --- | --- | --- | --- |
| `APTITUDE_COUNT` | 3 | Might, Wits, Spirit (index order 0,1,2; ties resolve to the lower index). | `lineage/lore.jai:1-8` |
| `APTITUDE_LIMIT` | 9 | Cap on base aptitude. Every gain respects it: some by clamping, the winter lessons by the size of the planned amount. Newborns cannot exceed it ((9+9)/4+1+2+2 = 9). | `lineage/lore.jai:8` |
| `COMING_OF_AGE` | 12 | Age >= 12 is "adult": may quest, be seated anywhere as an adult, hears the Seer. | `lineage/lore.jai:83` |
| `PRIME_AGE` | 20 | Youth is 12..19. | `lineage/lore.jai:84` |
| `VETERAN_AGE` | 40 | Prime is 20..39. | `lineage/lore.jai:85` |
| `ELDER_AGE` | 55 | Veteran 40..54; elder 55+. Also the old-age death threshold. | `lineage/lore.jai:86` |
| Phase adjustments (Might, Wits, Spirit) | Child 0,0,0; Youth -1,-1,-1; Prime 0,0,0; Veteran -1,+1,0; Elder -2,+1,0 | Added to base to get effective aptitude (floored at 0). Children never quest, so the child row only affects display. | `lineage/lore.jai:88-94`; `content/lore.json` phases |
| Vocation aptitudes | Knight, Warrior: Might; Ranger, Scholar: Wits; Priest, Sage: Spirit | What a hero trains alone; the calling rolled at coming of age is one of the two for the chosen aptitude, 50/50. | `lineage/lore.jai:175-182` |

## 3. Quest resolution

| Name | Value | Meaning | Source |
| --- | --- | --- | --- |
| `DICE_SIDES` | 6 | Two dice, each uniform 1..6. | `lineage/chance.jai:32`, `lineage/tale.jai:100-101` |
| `DICE_MIDPOINT` | 7 | Subtracted from the dice sum, so the roll term is -5..+5, centred on 0. | `lineage/chance.jai:33` |
| `DICE_OUTCOMES` | 36 | Forecast enumerates all 36 dice pairs, each weight 1/36. | `lineage/chance.jai:34`, `lineage/quest.jai:213-220` |
| margin | `power + d1 + d2 - 7 - demand` | | `lineage/tale.jai:102` |
| `TRIUMPH_MARGIN` | 4 | margin >= 4: TRIUMPH. | `lineage/quest.jai:4,143-148` |
| success band | 0..3 | margin >= 0: SUCCESS. | `lineage/quest.jai:145` |
| `SETBACK_MARGIN` | 4 | margin -1..-4: SETBACK; margin <= -5: DISASTER. | `lineage/quest.jai:5,146-147` |
| `WOUND_PENALTY` | 2 | Power -2 for a wounded member. | `lineage/quest.jai:6,174-176` |
| `DEATH_PER_DANGER` | 0.15 | In a disaster, each member rolls death with chance `min(danger * 0.15, 1.0)`. Danger 1: 15%, 2: 30%, 3: 45%, 4: 60%. | `lineage/quest.jai:7,128-130` |
| `TRIUMPH_RENOWN` | 1 | Extra renown on a triumph (house and each member). | `lineage/quest.jai:8`, `lineage/tale.jai:173` |
| `MAXIMUM_SEATS` | 4 | Party array size. | `lineage/quest.jai:1` |
| `QUEST_COUNT` | 4 | Quests on a board. | `lineage/quest.jai:2` |
| `YOUTH_QUEST_LEARNING_LIMIT` | 5 | A youth in a successful/triumphant party gains +1 in the quest aptitude only while base < 5. | `lineage/tale.jai:211-220` |
| triumph lesson | +1 | On a triumph (not at the Door), the member with the lowest base in the quest aptitude who can still grow gains +1. | `lineage/tale.jai:200-209` |
| `percent(x)` | `int(x * 100 + 0.5)` | Rounding used for every displayed percentage. | `lineage/quest.jai:234-236` |

### Outcome odds by power minus demand

Derived from the formula above (all 36 dice pairs). This is what the forecast shows and what
the roll produces; the port must reproduce exactly these fractions.

| power - demand | Disaster | Setback | Success | Triumph | Success or better (as shown) |
|---|---|---|---|---|---|
| <= -10 | 36/36 | 0 | 0 | 0 | 0% |
| -9 | 35/36 | 1/36 | 0 | 0 | 0% |
| -8 | 33/36 | 3/36 | 0 | 0 | 0% |
| -7 | 30/36 | 6/36 | 0 | 0 | 0% |
| -6 | 26/36 | 10/36 | 0 | 0 | 0% |
| -5 | 21/36 | 14/36 | 1/36 | 0 | 3% |
| -4 | 15/36 | 18/36 | 3/36 | 0 | 8% |
| -3 | 10/36 | 20/36 | 6/36 | 0 | 17% |
| -2 | 6/36 | 20/36 | 10/36 | 0 | 28% |
| -1 | 3/36 | 18/36 | 14/36 | 1/36 | 42% |
| 0 | 1/36 | 14/36 | 18/36 | 3/36 | 58% |
| +1 | 0 | 10/36 | 20/36 | 6/36 | 72% |
| +2 | 0 | 6/36 | 20/36 | 10/36 | 83% |
| +3 | 0 | 3/36 | 18/36 | 15/36 | 92% |
| +4 | 0 | 1/36 | 14/36 | 21/36 | 97% |
| +5 | 0 | 0 | 10/36 | 26/36 | 100% |
| +6 | 0 | 0 | 6/36 | 30/36 | 100% |
| +7 | 0 | 0 | 3/36 | 33/36 | 100% |
| +8 | 0 | 0 | 1/36 | 35/36 | 100% |
| >= +9 | 0 | 0 | 0 | 36/36 | 100% |

## 4. Quest stakes, trouble, the board

| Name | Value | Meaning | Source |
| --- | --- | --- | --- |
| `DEMAND_PER_SEAT` | 3 | | `lineage/quest.jai:10` |
| `YEARS_PER_DEMAND_STEP` | 6 | Integer division: +1 per seat in years 7-12, +2 in 13-18, +3 in 19-24, +4 in 25. | `lineage/quest.jai:11` |
| demand | `seats * (3 + calm_danger + trouble + (year - 1) / 6)` then `+ wobble` | `seats` here is the troubled (reduced) seat count; `calm_danger` is the template's danger, not the raised danger. Integer division. | `lineage/quest.jai:45-53`, `generation/quest-context/quest-context.jai:212-214` |
| `DEMAND_WOBBLE` | 1 | Wobble uniform in {-1, 0, +1}, rolled after seats. Not applied to ghost quests or Door locks. | `generation/quest-context/quest-context.jai:3,214` |
| seats | template `seats_low..seats_high` uniform, then `max(seats - trouble, 1)` | `TROUBLE_SEATS` = 1 seat per trouble. | `lineage/quest.jai:14,49`, `generation/quest-context/quest-context.jai:212` |
| danger | `min(calm_danger + trouble, 4)` | `DANGER_LIMIT` = 4. | `lineage/quest.jai:16,50` |
| quest renown | `calm_danger + trouble` | Not capped (can exceed danger when danger is capped). | `lineage/quest.jai:51` |
| `TROUBLE_DEMAND` | 1 | Per-seat demand per trouble. | `lineage/quest.jai:15` |
| `TROUBLE_LIMIT` | 2 | Max trouble per place. | `lineage/quest.jai:13`, `lineage/tale.jai:86` |
| trouble easing | SUCCESS/TRIUMPH: trouble = 0; SETBACK/DISASTER: trouble - 1 (floor 0) | Any answered quest at the place. | `lineage/quest.jai:55-58` |
| unanswered trouble | +1 (cap 2) | Per unanswered quest at that place. | `lineage/tale.jai:82-91` |
| `UNANSWERED_RENOWN` | 1 | Base cost of an unanswered quest. | `lineage/quest.jai:17,41-43` |
| `TROUBLED_RENOWN` | 1 | +1 if the quest's trouble (as generated) > 0. | `lineage/quest.jai:18` |
| `RENOWN_PER_EXPECTATION` | 20 | +`house renown / 20` (integer division, renown at the moment that quest is processed). | `lineage/quest.jai:19,42` |
| Disaster house renown | `-danger` (raised danger) | | `lineage/tale.jai:121-124` |
| Success house renown | `+quest renown (+1 triumph) + carriers` | carriers = number of CARRY_THE_HOUSE members x 1. | `lineage/tale.jai:171-177` |
| `ANSWERABLE_CHANCE` | 0.5 | Board welcome threshold for the best pair (min of the two success chances). | `generation/quest-context/quest-context.jai:32` |
| `DREAM_CALL_CHANCE` | 0.35 | A quest "calls a dreamer who could go" if that dreamer's best likely party has success-or-better >= 35%. | `generation/quest-context/quest-context.jai:33` |
| integer equivalents | answerable ⇔ power >= demand; could go ⇔ power >= demand - 1 | P(success or better) only takes the values k/36 for k in {0,1,3,6,10,15,21,26,30,33,35,36}; 0.5 falls between 15 and 21, 0.35 between 10 and 15. Float rounding of the 1/36 sums cannot move either threshold. | derived from `lineage/quest.jai:213-232` |
| `BOARD_ATTEMPTS` | 16 | Max boards planned per summer. | `generation/quest-context/quest-context.jai:34` |
| board score | `min(answerability / 0.5, 1) * 2 + (1 if calls a dreamer)` | Max 3. A strictly higher score replaces the kept board. | `generation/quest-context/quest-context.jai:48-52` |
| `EASING_LIMIT` | 30 | Max single-point demand reductions when easing. | `generation/quest-context/quest-context.jai:35,150-159` |
| easing floor | stop when `demand <= seats` | A quest is never eased below one demand per seat. | `generation/quest-context/quest-context.jai:156` |

## 5. Ghost quests

| Name | Value | Meaning | Source |
| --- | --- | --- | --- |
| `GHOST_SEATS` | 2 | Calm seats. Trouble still applies. | `lineage/ghost.jai:7,73` |
| `GHOST_DANGER` | 2 | Calm danger. Trouble still applies. | `lineage/ghost.jai:8` |
| ghost aptitude | Spirit | | `lineage/ghost.jai:64` |
| ghost demand | `set_quest_stakes` formula, no wobble | | `lineage/ghost.jai:73` |

## 6. Fear, dread, courage

| Name | Value | Meaning | Source |
| --- | --- | --- | --- |
| `DREAD_LIMIT` | 5 | Dread is clamped to 5; reaching 5 breaks the hero. | `lineage/fear.jai:10,43-45` |
| `COURAGE_TO_CONQUER` | 3 | Courage >= 3 conquers the fear. | `lineage/fear.jai:11,104` |
| fear penalty | `2 + dread / 2` (integer division) | Dread 0-1: -2; 2-3: -3; 4-5: -4. Applies to any quest carrying the tag, while not conquered. | `lineage/fear.jai:12-18` |
| `CONQUERED_FEAR_BONUS` | 2 | +2 on quests carrying the conquered tag (also born-brave). | `lineage/fear.jai:14`, `lineage/quest.jai:170-172` |
| facing the fear | dread +1, +1 more on SETBACK/DISASTER, -1 if a steadying companion is in the party | Net 0 or less: no change. | `lineage/fear.jai:110-112` |
| `REST_DREAD_SHED` | 1 | By the fire, not if broken. | `lineage/hearth.jai:14,324-340` |
| parent/child failure dread | +1 each | Parent and child in the same failed (SETBACK/DISASTER) party. | `lineage/bond.jai:198-218` |
| grief dread | kin (spouse, parent, child) 2; friend, mentor, student 1; rival, companion 0 | Per the mourner's bond to the dead. | `lineage/bond.jai:30-39,241-278` |

## 7. Bonds

| Kind | Rank | Power in party | Grief | Steadies | Source |
| --- | --- | --- | --- | --- | --- |
| Companion | 0 | 0 | 0 | no | `lineage/bond.jai:31` |
| Friend | 1 | +1 | 1 | yes | `lineage/bond.jai:32` |
| Rival | 1 | -1 | 0 | no | `lineage/bond.jai:33` |
| Spouse | 3 | +2 | 2 | yes | `lineage/bond.jai:34` |
| Mentor | 2 | +1 | 1 | yes | `lineage/bond.jai:35` |
| Student | 2 | +1 | 1 | yes | `lineage/bond.jai:36` |
| Parent | 4 | +2 | 2 | yes | `lineage/bond.jai:37` |
| Child | 4 | +2 | 2 | yes | `lineage/bond.jai:38` |

| Name | Value | Meaning | Source |
| --- | --- | --- | --- |
| `FRIENDSHIP_AFTER` | 2 | Companions become friends at 2 shared successes (successes counted for any bond kind). | `lineage/bond.jai:28,180` |
| "steadies" | bond power > 0 | | `lineage/bond.jai:123-125` |

## 8. Destinies

| Name | Value | Meaning | Source |
| --- | --- | --- | --- |
| `DOOR_DESTINY_POWER` | 5 | +5 power at each lock for OPEN_THE_SEALED_DOOR holders. | `lineage/destiny.jai:26`, `lineage/quest.jai:178-180` |
| `CROWN_RENOWN` | 8 | Personal renown needed for the crown to claim. | `lineage/destiny.jai:27,132-136` |
| `PATRON_POWER` | 1 | +1 per patron on every quest (non-empty party), including Door locks. | `lineage/destiny.jai:28`, `lineage/quest.jai:209-211` |
| `SURPASSING_CHILD_BONUS` | 2 | +2 to all three base aptitudes of the first child born to a CHILD_WILL_SURPASS_YOU parent (per such parent; both parents stack). | `lineage/destiny.jai:29`, `generation/hero-context/hero-context.jai:115-121` |
| `MENDED_BONUS` | 1 | +1 to all base aptitudes (cap 9) when mended. | `lineage/destiny.jai:30`, `lineage/tale.jai:303-305` |
| `MENDED_DREAD` | 2 | Dread +2 when mended. | `lineage/destiny.jai:31`, `lineage/tale.jai:301` |
| `OUTLIVING_DREAD` | 2 | Extra grief dread for OUTLIVE_THOSE_YOU_LOVE mourners (only when the base grief > 0). | `lineage/destiny.jai:32`, `lineage/bond.jai:263` |
| `OUTLIVING_AGE_FACTOR` | 0.5 | Old-age chance multiplier. | `lineage/destiny.jai:33`, `lineage/passage.jai:51` |
| `BED_AGE_FACTOR` | 2.0 | Old-age chance multiplier for DIE_IN_YOUR_BED. | `lineage/destiny.jai:34`, `lineage/passage.jai:52` |
| `CARRIED_RENOWN` | 1 | Extra house renown per CARRY_THE_HOUSE member on a won quest (not added to personal renown). | `lineage/destiny.jai:35`, `lineage/tale.jai:175-177` |
| `CARRIER_LOSS` | 4 | House renown lost when a CARRY_THE_HOUSE hero dies (not when crowned). | `lineage/destiny.jai:36`, `lineage/tale.jai:377-383` |
| `GREATER_LESSON` | 1 | Extra lesson amount from a TEACH_A_GREATER teacher (adult learners only). | `lineage/destiny.jai:37`, `lineage/hearth.jai:106` |

## 9. Winter

| Name | Value | Meaning | Source |
| --- | --- | --- | --- |
| `FIRE_SEATS` | 2 | | `lineage/hearth.jai:1` |
| training seats | 2 (learner, teacher) | | `lineage/hearth.jai:18` |
| garden seats | 2 | | `lineage/hearth.jai:19` |
| `TALE_SEATS` | 2 | | `lineage/hearth.jai:2` |
| `BENCHES` | 2 | Each a child seat and a teacher seat (4 seats). | `lineage/hearth.jai:3,21` |
| `YARD_SPOTS` | 6 | Children's holding seats; also the cap on living children for births. | `lineage/hearth.jai:4` |
| `SELF_TAUGHT_LIMIT` | 6 | Training alone stops at base 6 in the vocation aptitude. | `lineage/hearth.jai:9` |
| `CHILD_TAUGHT_LIMIT` | 4 | Bench lessons stop at base 4. | `lineage/hearth.jai:10` |
| `TEACHABLE_AGE` | 6 | Children under 6 learn nothing on a bench. | `lineage/hearth.jai:11` |
| `WINTER_GAIN_LIMIT` | 2 | Cap on an adult lesson before the TEACH_A_GREATER bonus. | `lineage/hearth.jai:12` |
| adult lesson (teacher) | `min(1 + [teacher veteran/elder] + [learner youth], 2) + [greater] `, then `min(.., taught - known, 9 - known)` | `taught` = teacher's base in their best aptitude (9 for a TEACH_A_GREATER teacher). | `lineage/hearth.jai:74-109` |
| adult lesson (alone) | `min(1 + [youth], 6 - known)` in the vocation aptitude | | `lineage/hearth.jai:53-71` |
| bench lesson | 1 | Child aged >= 6, base < 4, teacher base > child base in the teacher's best aptitude. | `lineage/hearth.jai:87-100` |
| `TALE_RENOWN` | 1 | +1 personal renown per adult teller; +1 house renown once per winter. | `lineage/hearth.jai:13,268-286` |
| `MARRYING_AGE` | 18 | Both must be >= 18 to wed. | `lineage/hearth.jai:153` |
| `COURTING_AGE_GAP` | 15 | Age difference must be <= 15. | `lineage/hearth.jai:154` |

## 10. Turn of the year

| Name | Value | Meaning | Source |
| --- | --- | --- | --- |
| `OLD_AGE_BASE_CHANCE` | 0.04 | Chance at age 55 (age after this turning's +1). | `lineage/passage.jai:1,46-54` |
| `OLD_AGE_YEARLY_CHANCE` | 0.025 | +2.5 points per year past 55; capped at 1.0 after multipliers. | `lineage/passage.jai:2` |
| `BIRTH_CHANCE` | 0.6 | Per eligible wed pair per turning. | `lineage/passage.jai:3,365` |
| `PARENT_AGE_LOW` | 18 (= `MARRYING_AGE`) | Both parents 18..45 inclusive (ages after this turning's +1). | `lineage/passage.jai:4,346-348` |
| `PARENT_AGE_HIGH` | 45 | | `lineage/passage.jai:5` |
| `CHILDREN_PER_PAIR` | 3 | Counted over children of that pair, living or dead. | `lineage/passage.jai:6,336-344,361` |
| `HOUSEHOLD_LIMIT` | 12 | No birth when living >= 12. Also the roster size. | `lineage/household.jai:1`, `lineage/passage.jai:363` |
| `WANDERER_ROOM` | 10 | No wanderer when living >= 10. | `lineage/household.jai:3`, `lineage/passage.jai:446` |
| `WANDERER_BASE_CHANCE` | 0.2 | | `lineage/passage.jai:7` |
| `WANDERER_RENOWN_DRAW` | 0.01 | +1 point per house renown. | `lineage/passage.jai:8` |
| `WANDERER_CHANCE_LIMIT` | 0.6 | Chance = `min(0.2 + 0.01 * renown, 0.6)`; reaches the cap at renown 40. | `lineage/passage.jai:9,439-441` |
| `FEWEST_ADULTS` | 5 | Fewer than 5 living adults: a wanderer arrives with certainty (room permitting). | `lineage/passage.jai:10,448-449` |
| `HEIRS_OFFERED` | 8 | Max heirs listed on a death page. | `lineage/passage.jai:11,212` |
| `NEWBORN_APTITUDE_SHARE` | 4 | Newborn base = `max((p1 + p2) / 4 + U{0,1}, 1)` per aptitude (base values, integer division). | `generation/hero-context/templates-hero.jai:50`, `generation/hero-context/hero-context.jai:110-113` |
| `INHERITED_FEAR_CHANCE` | 0.4 | Per parent, in order, for an unconquered, unbroken fear. | `generation/hero-context/templates-hero.jai:51` |
| `BROKEN_FEAR_CHANCE` | 0.8 | Same, for a broken parent. | `generation/hero-context/templates-hero.jai:52` |
| `BORN_BRAVE_CHANCE` | 0.5 | Same, for a parent whose fear is conquered. | `generation/hero-context/templates-hero.jai:53` |
| `TEACHER_AGE_FOR_DREAM` | 35 | WORTHY_STUDENT is not rolled for anyone younger (while other dreams are unclaimed). | `generation/hero-context/templates-hero.jai:54`, `generation/hero-context/hero-context.jai:54-56` |
| `WANDERER_AGE_LOW`..`HIGH` | 17..36 | Uniform. | `generation/hero-context/templates-hero.jai:31-32` |
| wanderer renown | 0..2 | Uniform personal renown. | `generation/hero-context/hero-context.jai:94` |

### Old-age death chance per turning

`min((0.04 + 0.025 * (age - 55)) * factor, 1.0)`; 0 below 55 and for FIRE_WILL_END_YOU.

| Age | Ordinary | Outlive (x0.5) | Bed (x2) |
|---|---|---|---|
| 55 | 4.0% | 2.0% | 8% |
| 56 | 6.5% | 3.25% | 13% |
| 58 | 11.5% | 5.75% | 23% |
| 60 | 16.5% | 8.25% | 33% |
| 62 | 21.5% | 10.75% | 43% |
| 63 | 24.0% | 12.0% | 48% |
| 65 | 29.0% | 14.5% | 58% |
| 70 | 41.5% | 20.75% | 83% |
| 74 | 51.5% | 25.75% | 100% |
| 80 | 66.5% | 33.25% | 100% |
| 93 | 99.0% | 49.5% | 100% |
| 94 | 100% | 50.75% | 100% |

### Wanderer standings

Chosen by house renown at the moment of arrival (the highest row whose threshold is met).
`generation/hero-context/templates-hero.jai:42-49`, `generation/hero-context/hero-context.jai:68-72`.

| House renown | Gift aptitude | Other two aptitudes |
|---|---|---|
| 0..11 | 3..5 | 1..3 |
| 12..29 | 4..6 | 2..3 |
| 30+ | 5..7 | 2..4 |

All ranges inclusive and uniform. The gift aptitude is uniform over the three.

## 11. Legacies

| Name | Value | Meaning | Source |
| --- | --- | --- | --- |
| `LEGACY_HEIRLOOM_BONUS` | 2 | Heirlooms made by dreams are +2. | `lineage/legacy.jai:33` |
| Thornfall | +1 Might | The one authored heirloom. | `lineage/household.jai:94` |
| `TALE_YEARLY_RENOWN` | 1 | +1 house renown per house tale per turning. | `lineage/legacy.jai:34,251-266` |
| `NARROW_BLESSING_POWER` | 2 | Against a tag / at a place. | `lineage/legacy.jai:35` |
| `BROAD_BLESSING_POWER` | 1 | On every quest. | `lineage/legacy.jai:36` |
| blade names | 10, used in order, cycling | The n-th blade forged in a run gets name `(n-1) mod 10`. | `lineage/legacy.jai:38-41,204-205` |
| `COURT_DREAM_RENOWN` | 4 | "Earn 4 renown" (personal renown). | `lineage/dream.jai:293` |
| `DREAM_STAGE_COUNT` | 3 | | `lineage/dream.jai:13` |
| stage goals | 1, except "Teach the young two winters" 2, "Quest at three different places" 3, "Succeed against the Undead twice" 2 | | `lineage/dream.jai:231,264,275` |

## 12. The Door

| Name | Value | Meaning | Source |
| --- | --- | --- | --- |
| `DOOR_SEATS` | 4 | | `lineage/door.jai:2` |
| `DOOR_DANGER` | 3 | Death chance in a lock disaster: 45%. | `lineage/door.jai:3` |
| `DOOR_RENOWN` | 5 | Renown per opened lock (+1 on triumph), house and each member. | `lineage/door.jai:4` |
| `DOOR_LOCKS` | 34, 34, 34 | Demand of the Might, Wits, Spirit locks. Fixed: no wobble, no year creep, no trouble. | `lineage/door.jai:5` |
| Door tags | Dark, Cold | Fears of Dark or Cold apply at every lock. | `lineage/lore.jai:66` |
| best-four score | `P(all three) + (P1 + P2 + P3) / 1000` | Used only to choose the displayed "best four". | `lineage/door.jai:96-105` |

## 13. Epitaphs and writing

| Name | Value | Meaning | Source |
| --- | --- | --- | --- |
| `EPITAPH_SENTENCES` | 6 | Sentence budget (counted as "." characters). | `lineage/epitaph.jai:14` |
| `EPITAPH_FRAME_COUNT` | 3 | Frame rolled uniformly, never the same as the previous roll in the run. | `lineage/epitaph.jai:15,62-70` |
| epitaph variants | 9 independent fair coins | One per part, rolled with the frame. | `lineage/epitaph.jai:68` |
| writing pools | 12 pools, uniform, never the previous line of that pool | | `lineage/writing.jai:1-26,157-163`, `lineage/chance.jai:15-21` |

## 14. Presentation numbers (port is free to change)

Listed so nobody hunts for them; none affects rules.

| Name | Value | Source |
| --- | --- | --- |
| Canvas | 640 x 360 | `source/platform/window.jai:16-17` |
| Hero card | 40 x 42 | `entity/entities/hero.jai:5-8` |
| Hero sheet | 224 x 316 | `entity/entities/hero.jai:10-13` |
| Roster grid | 12 seats, 3 columns | `lineage/household.jai:1-5` |
| Renown warning | renown <= 4 drawn red | `lineage/panels.jai:14,340` |
| Typewriter speed | 90 letters/second (telling story text) | `scene/scenes/telling.jai:97,67-69` |
| Heir buttons | 2 columns x 5 rows (8 heirs + "No one") | `scene/scenes/turning.jai:142-146` |
| Bonds shown on sheet | 6, then "and N more" | `lineage/sheet.jai:4,218-234` |
| Guide | 8 entries per page, 2 pages | `modal-screens/guide.jai:61-65` |
