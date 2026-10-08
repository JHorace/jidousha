# Call of Cthulhu (r2) — design note

A text game. Eldritch beings phone you at night; every exchange on the line
costs sanity, so the job is to get them off the phone. Learning a being's lore
is what lets you give the answer that ends the call soonest — a dating sim
inverted. Five days; lose when sanity reaches 0, win by surviving the fifth
night.

## The beings

Three, each with a cult, a base drain, a temper (anger, persistent across the
run) and four facts of lore. Each fact has one question the being asks about
it, with three answers: one drawn from the lore, one that is merely wrong, and
one that insults it.

| being | cult | base drain | temperament |
|---|---|---|---|
| Father Dagon | the Esoteric Order of Dagon (Innsmouth) | 2 | slow, tidal, patient |
| Nyarlathotep, the Crawling Chaos | the Church of Starry Wisdom | 3 | mocking, theatrical |
| Yog-Sothoth, the Gate and the Key | the Whateley household | 4 | cold, exact |

All lore is written for this game, drawing on Lovecraft's public-domain mythos.

## The call

- A call opens with **interest 5** — how much more the being wants to talk.
  The call ends (you hang up) the moment interest reaches 0.
- The being asks a question; you answer with **1/2/3** (or tap the answer).
  One function, `rules::answer_outcome`, says what an answer does:

  | answer | interest | temper | sanity cost |
  |---|---|---|---|
  | lore | -3 | 0 | drain |
  | wrong | -1 | 0 | drain |
  | insult | 0 | +1 | drain + 6 (the shock) |

- **drain** = base drain + temper - composure, never below 1.
- **Silence** is not free: every 8 seconds on the line without an answer costs
  one drain ("it listens to you breathe"). A player who does nothing loses.
- **Anger** is worse than the drain three ways: the 6-point shock, an
  exchange that does not shorten the call, and +1 temper — which is +1 drain on
  every later exchange with that being, for the rest of the run.
- The call screen shows the question, the three answers, sanity, the drain,
  the being's temper and every fact you know about it. When the question is
  about a fact you know, every answer carries the outcome
  `answer_outcome` returns for it (interest change and sanity cost); otherwise
  the answers are unmarked and you are guessing. Answer order is shuffled per
  question by the seed.

## The day loop

Each day is a **morning** (one action) and a **night** (the calls).

Morning actions:

- **Train composure** (max 2): every exchange drains 1 less, for the rest of
  the run.
- **Study a being** (one per being with facts left): learn its next fact. If it
  calls tonight it asks about that fact first.
- **Disrupt a being's cult**: it will not call tonight, and its temper rises by
  1 (it notices).

Tonight's calls come from one function, `rules::tonight(&Run)`: two of the
three beings by the seed (three on the fifth night), minus a disrupted one,
each with its question order (today's studied fact first, then a seeded
shuffle of the rest, cycling). The morning screen prints tonight's callers as
`tonight` returns them, and each option states its effect on tonight — that
statement is computed by applying the action to a copy of the day and diffing
`tonight` before and after, so the preview and the evening read the same
function.

## Sanity arithmetic

Sanity starts at 100 and only falls. A call answered entirely from lore costs
2 exchanges; entirely wrong, 5; each insult adds an exchange-less 6 plus 1
drain for the rest of the run. At base drain 3, a call costs 6 sanity known
and 15 guessed; ten or eleven calls over five nights are 66-165 — survivable
only with preparation. A run surviving with sanity 50 or more gets the better
ending ("you sleep through the sixth night"); below that, "the phone still
rings in your dreams".

## Inputs

- Morning: digit keys 1-7 (or tap a row).
- Call: 1/2/3 (or tap an answer).
- Between screens (call ended, the end): Enter or Space (or tap anywhere).

## Determinism

The run seed is `GameConfig::seed` unless a check inserts a `RunSeed` before
tick 1. Every random choice (tonight's callers, question order, answer order)
is a pure function of (seed, day, being, question) through `Rng::from_seed`;
no system draws from the world's `Rng`, so the schedule a morning previews is
the schedule the night plays.
