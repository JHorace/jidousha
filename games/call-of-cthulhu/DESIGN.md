# Call of Cthulhu — design note

A phone sits on your desk. At night, eldritch beings call. Every exchange on the line
costs sanity, so the job is to end each call fast; learning a being's lore is what lets
you answer its questions the way that ends the conversation. Angering a being costs
more than the drain. A dating sim, inverted. Prototype: five days, three beings.

Everything below is pure functions over one `Game` value (`model.rs`, `call.rs`);
`screens.rs` draws a `Panel` from it, `input.rs` turns a key or click into a `Choice`,
and `verify.rs` plays it. Nothing random is drawn from the world: every random pick is
`Rng::from_seed(mix(seed, day, slot, ...))`, so the same seed and inputs replay the
same run, and a morning preview can ask "what would tonight be?" without spending a draw.

## The beings (public-domain Lovecraft; the words are this game's)

| being | voice | call | drain | temper |
|---|---|---|---|---|
| Yog-Sothoth, the Gate | cold, exact, bored by mortals | 3 exchanges | 3 a line | wrath at temper 2 |
| Dagon, the Deep | patient, wet, nostalgic | 5 exchanges | 2 a line | wrath at temper 3 |
| Nyarlathotep, the Crawling Chaos | charming, a liar's questions | 4 exchanges | 2 a line | wrath at temper 3; a wrong answer adds 2 |

Each being has **five lore facts**, and each fact has **one question** with three
answers: the one the fact makes true, a plausible wrong guess, and an insult. A fact is
*known* once studied or learned; the player sees all of a being's known facts on the
call screen while choosing.

## The sanity arithmetic (`model::answer_outcome`, `model::exchange_cost`)

- `drain` = the being's drain less your **Composure** (0-2), never under 1.
- An exchange costs `drain`, doubled once the being is in **wrath**.
- **Correct** (the fact is known, or you guessed right): the call has 2 fewer exchanges
  left, the being's temper falls by 1. **Wrong guess**: 1 more exchange left (Nyarlathotep: 2).
  **Insult**: as a wrong guess, temper +1, and **3 x drain** instead of 1 x drain — anger
  costs three times the drain. At wrath (temper at the being's limit) every exchange costs double.
- **Hang up** ends the call at once for **3 x drain** and leaves a grudge: that being
  calls again tonight's next slot (tomorrow's first) at temper +1.
- The call ends when no exchanges are left. Sanity 0 ends the run: you lose.
- Start at **36 sanity**, five days, two calls a night; surviving wins ("sound" at 15+, else "frayed").

## The day loop

**Morning** — one action (keys 1-8, or click). Each option states its effect on tonight,
computed by diffing `plan_night` before and after it, so the stated effect cannot differ
from what happens:

1. Study a being's lore: learn its next unknown fact; if it calls, its first question is that fact.
2. Appease a being's cult (x3): that being does not call tonight; costs 4 sanity.
3. Train Composure: -1 to every drain from now on (max 2).
4. Rest: +6 sanity.

**Evening** — `plan_night(game)` lists tonight's callers: a grudge first, then a seeded
pick from the beings not appeased, two calls in all. Each call asks a being's questions in a
seeded order, the studied fact first. Keys 1-3 answer, 4 hangs up.

## Decision surfaces

- *Answer on the line*: the call screen shows the question, the three answers, my sanity,
  the per-exchange drain, the being's temper and every fact I know about it. An answer I have
  the fact for says what it will do (read from `answer_outcome`, the function the
  resolution also calls); one I do not says "?". Asserted by `verify::check_call_decision`.
- *Spend the morning*: each option states its effect on tonight; asserted by
  `verify::check_morning_decision` (two seeded runs that differ in one morning choice).

## Checks

Three players (a lore-driven one that wins, a first-try chaser that picks the first
answer, a do-nothing that never answers), the three numbers each run, transcript gates
on both decision rows, a win and a loss, the layout floors, then `tools/mutate`.
