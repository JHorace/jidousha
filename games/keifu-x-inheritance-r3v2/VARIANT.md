# Keifu X Inheritance — the variant

A fork of `games/keifu` (the faithful port of Lineage) that deepens what a
family inherits. Mainline stays the port; this crate began as a byte-for-byte
copy of it (commit `yakin(keifu-x-inheritance-r3v2): fork keifu`) and diverges
only by the rules in `tools/yakin/runs/keifu-x-inheritance-r3v2/DESIGN.md`.
The copied `spec/SPEC.md` stays mainline's; the deltas are owned here and there.

## What changes

- **Family and outsiders.** The founders and their blood are the Thorne
  family; a wanderer is an outsider. An outsider's won quests and told tales
  earn the house nothing, and an outsider is never an heir.
- **Black marks.** A family member who fails a personal quest (a template
  quest their own dream calls them to go on) stains the name: renown lost now,
  more every year the mark is carried, and half the marks pass to an heir.
- **Marrying in.** An outsider wed to a Thorne becomes family, if the house
  thinks them proven enough.
- **The lean.** A child leans to one parent's lean, by a seeded coin, and is
  born one stronger in it.

## The six constants (`src/constants.rs`)

| constant | value |
|---|---|
| `FAMILY_RENOWN_TO_WED` | 4 |
| `MARK_HOUSE_RENOWN` | 2 |
| `MARK_PERSONAL_RENOWN` | 2 |
| `MARK_YEARLY_RENOWN` | 1 |
| `MARK_INHERITED_SHARE` | 2 |
| `LEAN_BONUS` | 1 |

The checks: `src/xi_marks.rs`, `src/xi_outsiders.rs`, `src/xi_inheritance.rs`,
the mutation list `mutants/xi.txt`.
