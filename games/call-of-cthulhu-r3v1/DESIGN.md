# Call of Cthulhu (r3v1) — design note

A text game. You keep the only telephone in Arkham that the Old Ones ring.
Every exchange on the line costs sanity; the call ends when the being has
said its piece. Learning a being's lore is what lets you answer it the way
that ends the call soonest. Insulting it is worse than talking.

## The beings

| being | drain / exchange | line (exchanges to say its piece) | temper (anger that is wrath) | cult |
|---|---|---|---|---|
| Cthulhu, the Dreamer in R'lyeh | 2 | 5 | 3 | the Esoteric Order of Dagon |
| Nyarlathotep, the Crawling Chaos | 2 | 4 | 2 | the Church of Starry Wisdom |
| Yog-Sothoth, the Gate and the Key | 3 | 4 | 3 | the Silver Twilight lodge |

Each being has **three lore facts**, and each fact **two questions**. A
question has three answers: the **lore** answer (the fact says it), a
**wrong** one (plausible, harmless) and an **insult** (plausible too — the
fact says what not to say). The three answers are shown in an order fixed
per question, so position tells you nothing.

## The call

The being on the line has a `line` counter (its table value at the start)
and an `anger` counter (0). Each exchange it asks one question and you
answer with **keys 1-3** (or a click/tap on the answer row). One function,
`rules::answer_outcome`, says what the answer does, and both the hint beside
each answer and the resolution read it:

- **lore**: line -2, anger +0
- **wrong**: line -1, anger +0
- **insult**: line +0, anger +1

The exchange costs `drain - composure + ANGER_SURCHARGE * anger_after`
sanity (drain floored at 1; `ANGER_SURCHARGE` = 2). When anger reaches the
being's temper it is **wrath**: a further `WRATH` = 10 sanity and the being
hangs up on you. Otherwise the call ends when `line <= 0` — you hang up.
So anger costs more than the drain three ways: the surcharge on this and
every later exchange, a line that does not shorten, and wrath.

The call screen shows, while you choose: the question, the three answers,
sanity, the per-exchange drain, the being's temper (`anger/temper`), the
line left, and **what you know of this being** (its learned lore). An
answer whose fact you know carries its outcome hint (`ends sooner: line
-2, sanity -3`); one you do not reads `outcome unknown`.

## The day loop

Five days. Each day is a **morning** then an **evening**.

**Morning** — choose one action with keys 1-6 (or a tap):

- **Study <being>** — learn its next unlearned fact. Tonight its questions
  on that fact become answerable, and it opens with them (it can tell what
  you were reading).
- **Train composure** — +1 composure (max 2): every exchange drains 1 less.
- **Bargain with <cult>** — offered for each being that will call tonight:
  that being stays silent tonight and the one not calling takes its place.

`rules::tonight(&DayState) -> Night` derives tonight's calls — who calls,
in which order, each call's question sequence, and its drain — from the
day's state (seed, day, lore, composure, bargain) alone. The morning
screen names tonight's callers, and each option carries its **effect on
tonight**, computed by diffing `tonight(now)` against `tonight(after this
option)`. The evening plays exactly `tonight(state)`.

**Evening** — two calls, in `tonight`'s order. Between calls, and after the
night, a summary screen; Space (or a tap) continues.

## Sanity, win and lose

Sanity starts at 100. Reaching 0 at any exchange loses the run on the spot
("the receiver is still warm"). Surviving the fifth night wins; finishing
with sanity at or above 30 is the better ending (you sleep without dreams).
Space restarts on the end screen.

## Determinism

`tonight` seeds its own `Rng` from `(seed, day)`, so a morning's preview
and the evening read the same numbers. Nothing else draws randomness. Same
seed and the same inputs replay the same run.

## Tuning, and why these numbers

The first numbers (drains 3/3/4, an insult that also lengthened the line,
surcharge 4, wrath 20) lost every run for every player — a blind answer is
an insult one time in three, so ignorance compounded into wrath. The tuned
numbers above give, over forty seeds: the scholar (looks ahead with
`answer_outcome`) wins 38, the novice (first option every morning, top
answer when blind) wins 11, the mute (never studies) wins none. A blind
call still costs about three times a known one; that ratio is the game.

## Verify

Three players (scholar / novice / mute) over forty seeds, each held to a
shipped band; the scholar wins and the mute loses by play through the keys
on the canonical seed. A decision-row check for each table row, a mutation list in
`mutants/`, and one captured PNG of a call screen.
