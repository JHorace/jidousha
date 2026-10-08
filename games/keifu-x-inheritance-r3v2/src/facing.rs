//! Facing the fear (SPEC §10.1, `lineage/fear.jai:88-126`): each member of a resolved
//! quest that carries their feared tag, in party order.
//!
//! A steadying companion turns a won quest into courage; otherwise dread grows — one
//! more on a loss, one less with the companion — through the one dread rule, which
//! breaks the hero at 5. The courage branch comes before the "can dread" check, so a
//! settled hero still gains courage (OQ-8).
//!
//! [emergent] `lines.fear.steadied` cannot be reached: the amount is 0 or less only
//! with a companion on a won quest, and that case took the courage branch first.
//! The line is written exactly where the spec puts it all the same.

use jidousha::prelude::Rng;

use crate::bonds::steadying_companion;
use crate::constants::{COURAGE_TO_CONQUER, DREAD_LIMIT};
use crate::fear::{Occasion, add_dread, can_dread, conquer};
use crate::hero::HeroId;
use crate::house::House;
use crate::ids::{Outcome, Pool};
use crate::resolve::Afield;
use crate::text::fmt;
use crate::words::W;

/// `member` faces the quest's tag, beside `party` (SPEC §10.1).
pub fn face_the_fear(
    f: &Afield<'_>,
    house: &mut House,
    rng: &mut Rng,
    party: &[HeroId],
    member: HeroId,
    out: &mut Vec<String>,
) {
    let words = &f.content.words;
    let fear = &house.heroes[member].fear;
    // Step 1: an unconquered, unbroken fear of a tag the quest carries.
    if fear.conquered || fear.broken || !f.quest.tags.contains(&fear.tag) {
        return;
    }
    let noun = f.content.lore.tags[fear.tag.index()].noun.clone();
    house.heroes[member].fears_faced += 1;
    let companion = steadying_companion(&house.heroes, party, member);
    let won = f.outcome >= Outcome::Success;
    if let (true, Some(companion)) = (won, companion) {
        let pool = house.writing.pick(f.content, Pool::Courages, rng);
        let hero = &mut house.heroes[member];
        hero.fear.courage += 1;
        let courage = hero.fear.courage;
        let object = &f.content.lore.pronouns[hero.pronoun.index()].object;
        out.push(fmt(
            &words[W::FearCourage],
            &[
                &fmt(pool, &[&hero.name, &noun]),
                &house.heroes[companion].name,
                object,
                &courage.to_string(),
                &COURAGE_TO_CONQUER.to_string(),
            ],
        ));
        if courage >= COURAGE_TO_CONQUER {
            out.extend(conquer(
                f.content,
                &mut house.heroes,
                member,
                f.quest.place,
                f.year,
            ));
        }
        return;
    }
    if !can_dread(&house.heroes[member]) {
        return;
    }
    let lost = matches!(f.outcome, Outcome::Setback | Outcome::Disaster);
    let amount = 1 + i32::from(lost) - i32::from(companion.is_some());
    if amount <= 0 {
        let steadier = companion.map_or("", |c| house.heroes[c].name.as_str());
        out.push(fmt(
            &words[W::FearSteadied],
            &[steadier, &house.heroes[member].name],
        ));
        return;
    }
    // The line, then the dread rule (which may break them: its line follows).
    let pool = house.writing.pick(f.content, Pool::Dreads, rng);
    let hero = &house.heroes[member];
    let after = (hero.fear.dread + amount).min(DREAD_LIMIT);
    out.push(fmt(
        &words[W::FearDread],
        &[
            &fmt(pool, &[&hero.name, &noun]),
            &after.to_string(),
            &DREAD_LIMIT.to_string(),
        ],
    ));
    out.extend(add_dread(
        f.content,
        &mut house.heroes,
        member,
        amount,
        Occasion::At(f.quest.place),
        f.year,
    ));
}
