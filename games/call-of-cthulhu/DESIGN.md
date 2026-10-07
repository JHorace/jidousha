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
| Yog-Sothoth, the Gate | cold, exact, bored by mortals | 4 exchanges | 2 a line | wrath at temper 2 |
| Dagon, the Deep | patient, wet, nostalgic | 6 exchanges | 1 a line | wrath at temper 4 |
| Nyarlathotep, the Crawling Chaos | charming, a liar's questions | 4 exchanges | 2 a line | wrath at temper 3 |

Each being has **five lore facts**, and each fact has **one question** with three
answers: the one the fact makes true, a plausible wrong guess, and a presumptuous
answer that offends it (worded politely enough that only lore tells it from the guess).
A fact is *known* once studied; the call screen shows every known fact of the caller.

## The sanity arithmetic (`rules::answer_outcome`, `rules::exchange_cost`)

- `drain` = the being's drain less your **Composure** (0-2), never under 1; an exchange
  costs `drain`, doubled once the being is in **wrath** (temper at its limit).
- Every answer takes one exchange off the call by itself (the being gets its way in time).
  **Right**: one more off (the call is 2 shorter), temper -1. **Wrong guess**: no extra —
  the call gets no shorter. **Insult**: as a wrong guess, temper +1, and **2 x drain**
  instead of 1 x drain: anger costs double the drain, and wrath doubles that.
- **Hang up** ends the call at once for **8 x the being's own drain** (Composure does not
  steady a rudeness) and leaves a grudge: that being calls first tomorrow, already irked.
  A call that ends with the being in wrath leaves a grudge too.
- The call ends when no exchanges are left. Sanity 0 ends the run: you lose.
- Start at **60 sanity**, five days, two calls a night; surviving wins ("sound" at 12+,
  else "frayed"). The measured balance (100 seeds each): the reader of lore wins about 7
  in 10, the player who never studies about 1 in 6, the one who only hangs up never.

## The day loop

**Morning** — one action (keys 1-8, or click). Each option states its effect on tonight,
computed by diffing `plan_night` before and after it, so the stated effect cannot differ
from what happens:

1. Study a being's lore (x3): learn its next three unknown facts; if it calls, those are its
   first questions.
2. Appease a being's cult (x3): that being does not call tonight; costs 4 sanity. If it was
   not due tonight the option says so: it buys nothing today.
3. Train Composure: -1 to every drain from now on (max 2).
4. Rest: +8 sanity.

**Evening** — `plan_night(game)` lists tonight's callers: a grudge first, then a seeded
pick from the beings not appeased, two calls in all. Each call asks a being's questions in a
seeded order, the studied facts first. Keys 1-3 answer, 4 hangs up; "R" begins again at the end.

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
