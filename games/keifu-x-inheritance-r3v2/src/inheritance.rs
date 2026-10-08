//! What a Thorne inherits (variant, DESIGN.md S4, S5): a newborn's lean, and an
//! heir's share of the dead's black marks.
//!
//! Mainline passes a share of both parents' aptitudes, maybe a fear, and every
//! blessing (SPEC §17.3); the variant adds one lean taken from one parent by a
//! seeded coin. Mainline's heir takes the heirloom and the undone dream (§15.2);
//! the variant's also takes half the marks. Each half is one function that both
//! the page that previews it and the act that does it call.

use crate::constants::{APTITUDE_LIMIT, LEAN_BONUS, MARK_INHERITED_SHARE};
use crate::hero::{Hero, HeroId};
use crate::ids::Aptitude;

/// The aptitude a hero leans to, and the parent they took it after (`None` for a
/// founder or a wanderer, whose lean is their own).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lean {
    /// The aptitude.
    pub aptitude: Aptitude,
    /// The parent it came from.
    pub from: Option<HeroId>,
}

/// A newborn's lean: `first`'s on coin 0, `second`'s on coin 1, falling to the
/// other parent's if the chosen one has none (SPEC-shaped, never in play: every
/// hero has a lean), else to `own_best` with no parent.
///
/// CONTRACT: `births::born` calls this and its page line reads the same `Lean`.
pub fn birthright(
    heroes: &[Hero],
    first: HeroId,
    second: HeroId,
    coin: usize,
    own_best: Aptitude,
) -> Lean {
    let (chosen, other) = if coin == 0 {
        (first, second)
    } else {
        (second, first)
    };
    [chosen, other]
        .into_iter()
        .find_map(|parent| {
            heroes[parent].lean.map(|lean| Lean {
                aptitude: lean.aptitude,
                from: Some(parent),
            })
        })
        .unwrap_or(Lean {
            aptitude: own_best,
            from: None,
        })
}

/// A newborn's base in `aptitude` once the lean's bonus is added, capped.
pub fn leaned(base: i32) -> i32 {
    (base + LEAN_BONUS).min(APTITUDE_LIMIT)
}

/// What an heir of `dead` takes besides the heirloom and the dream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Succession {
    /// Black marks the heir takes on.
    pub marks: i32,
}

/// What an heir of `dead` would take: half the marks, rounded down — so one mark
/// dies with its bearer and a heavy name outlives them.
///
/// CONTRACT: the death page's line and `heirs::choose` both call this.
pub fn succession(heroes: &[Hero], dead: HeroId) -> Succession {
    Succession {
        marks: heroes[dead].marks / MARK_INHERITED_SHARE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{house, id};

    #[test]
    fn coin_zero_gives_the_first_parents_lean_and_coin_one_the_seconds() {
        let (_, house) = house();
        let (garrick, maren) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Maren"));
        let first = birthright(&house.heroes, garrick, maren, 0, Aptitude::Spirit);
        let second = birthright(&house.heroes, garrick, maren, 1, Aptitude::Spirit);
        assert_eq!(
            first,
            Lean {
                aptitude: Aptitude::Might,
                from: Some(garrick)
            }
        );
        assert_eq!(
            second,
            Lean {
                aptitude: Aptitude::Wits,
                from: Some(maren)
            }
        );
    }

    #[test]
    fn a_parent_without_a_lean_gives_way_to_the_other_and_then_to_the_childs_own() {
        let (_, mut house) = house();
        let (garrick, maren) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Maren"));
        house.heroes[garrick].lean = None;
        let lean = birthright(&house.heroes, garrick, maren, 0, Aptitude::Spirit);
        assert_eq!(lean.from, Some(maren));
        house.heroes[maren].lean = None;
        let own = birthright(&house.heroes, garrick, maren, 0, Aptitude::Spirit);
        assert_eq!(
            own,
            Lean {
                aptitude: Aptitude::Spirit,
                from: None
            }
        );
    }

    #[test]
    fn an_heir_takes_half_the_marks_rounded_down() {
        let (_, mut house) = house();
        let garrick = id(&house.heroes, "Garrick");
        for (marks, taken) in [(0, 0), (1, 0), (2, 1), (3, 1), (4, 2), (5, 2)] {
            house.heroes[garrick].marks = marks;
            assert_eq!(
                succession(&house.heroes, garrick).marks,
                taken,
                "{marks} marks"
            );
        }
    }

    #[test]
    fn the_lean_bonus_never_lifts_an_aptitude_past_the_limit() {
        assert_eq!(leaned(4), 5);
        assert_eq!(leaned(9), 9);
    }
}
