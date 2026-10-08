# Findings — where the brief and the repository differ

The brief was right in substance. These are the places where the repository differs from
it, and what I did about each.

## 1. There is a design document: `PROTOTYPE.md`

The brief says "There is no readme and no design doc: the code is the only spec." The repo
root has `PROTOTYPE.md` (37 KB). It holds Lineage's design brief (the five questions the
prototype asks, the reservoir ideas, the screens) and build notes from three milestones: the
founding household, invented names, tuning tables, decisions taken where the brief was
silent, scripted-run results, and "where this stands".

- **It is not a reliable spec.** It is a changelog. The early sections are stale against the
  later ones and against the code. For example: youth begins at 14 in the age table but 12
  in code; births were 40% and are now 60%; the Door's locks were 24 and are now 34; wanderer
  standings are given as "under 10, 10-24, 25+" where the code uses 0/12/30. **SPEC.md
  follows the code everywhere** and does not cite PROTOTYPE.md as a source of rules. OQ-23
  asks the owner to confirm the code's values are the intended ones.
- **It is useful to the owner** in Phase 1: it states the intent behind many rules, which
  helps in judging the open questions. It also records scripted-run results (seeds 3, 7, 11
  and 23; first deaths, renown curves, Door odds) that a port playtest can be compared
  against at the level of distributions.
- Port sessions should not need it. Nothing in it is required that is not in SPEC.md,
  CONSTANTS.md or content/.

## 2. "A party builder comparable to Darkest Dungeon"

That is broadly right. Some details worth knowing:

- There is no dungeon, combat, inventory or item management. A quest is **one dice roll per
  party**: aptitude sum plus modifiers, plus two dice less 7, against a number. There are
  four outcomes. Everything else hangs off that roll.
- What the brief calls "traumas" are **fears**: dread up to 5, then breaking, which leaves a
  scar and a permanent refusal. Fears can also be conquered.
- "Material inheritance" is **one heirloom per hero** (+1 or +2 to one aptitude), passed on
  by the player's choice at death. There are also blessings, which children inherit
  automatically, and house tales, which earn renown every year.
- The game is really a **25-year dynasty**: ageing, marriage, births, comings of age,
  prophecies, unfinished dreams handed down, ghosts, and the Sealed Door as the one final
  exam. There is no score; the ending is a verdict (0-3 locks opened) and a family tree of
  epitaphs.

## 3. Game and engine are separable

As the brief hoped. The game is `source/application/game/` and nothing else, apart from
three choices it makes in engine files: a 640x360 canvas, mouse-and-touch input only, and the
title "Lineage". INVENTORY.md lists every file on each side.

## 4. All content is in the source

There are no game data files. `data/game/` is empty. Names, quests, dreams, destinies, line
pools, epitaph sentences, the guide and the founding household are all Jai literals.
`content/` holds them, copied by line.

## 5. Assets

The assets are PNGs, as the brief said. Every game sprite is a 16x16 single-colour mask,
tinted when drawn. Only 20 of the 128 sprites in `assets/game/sprites/` are referenced, and
INVENTORY.md lists the unused ones. There are no game sounds or music. The only audio is the
engine's UI click.

## 6. No saving

A run lives only as long as the process does. The save hooks exist but are empty and never
called. The port does not need save/load to be rules-identical.

## 7. Adaptations made

None to the output set. Two additions within it:

- `content/ui-text.json` holds presentation text, including the guide. Whether those words
  are binding is OQ-25.
- `CONSTANTS.md` adds derived tables (outcome odds by margin, old-age chance by age) and an
  integer form of the two board thresholds, so the port can check its numbers without
  re-deriving them.
