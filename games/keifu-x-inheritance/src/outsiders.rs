//! Family and outsiders (VARIANT.md): who is of the name, and who may be wed into it.
//!
//! A hero is family or an outsider (`Hero::family`). Outsiders are cheap hands: what
//! they win earns the house nothing, they never inherit, and they may marry into the
//! family only once their own renown reaches `MARRY_IN_RENOWN`. `may_marry_in` is the
//! one eligibility function: the garden's verdict and the wedding both read it.

use crate::constants::MARRY_IN_RENOWN;
use crate::hero::{Hero, HeroId};

/// Whether `hero` is of the family: a founder, a child of the family, or one who
/// married in.
pub fn is_family(hero: &Hero) -> bool {
    hero.family
}

/// When exactly one of the pair is an outsider and their renown is below the
/// threshold, that outsider and their renown: the marriage is refused until they
/// have proven themselves. `None` means no outsider bars the match.
pub fn may_marry_in(heroes: &[Hero], a: HeroId, b: HeroId) -> Option<(HeroId, i32)> {
    let outsider = match (is_family(&heroes[a]), is_family(&heroes[b])) {
        (false, true) => a,
        (true, false) => b,
        _ => return None,
    };
    let renown = heroes[outsider].renown;
    (renown < MARRY_IN_RENOWN).then_some((outsider, renown))
}

/// The outsider who becomes family if `a` and `b` wed: exactly one of the pair is an
/// outsider (and, by `may_marry_in`, has earned it).
pub fn marrying_in(heroes: &[Hero], a: HeroId, b: HeroId) -> Option<HeroId> {
    match (is_family(&heroes[a]), is_family(&heroes[b])) {
        (false, true) => Some(a),
        (true, false) => Some(b),
        _ => None,
    }
}
