//! Sharing the road (SPEC §12.2, `lineage/bond.jai:163-219`): every pair of members
//! still living after a quest that was not the Door.
//!
//! They become companions if they were nothing; a win counts a shared success, turns
//! rivals who triumphed into friends and companions with two wins into friends; a
//! disaster turns companions into rivals; and a parent and child who failed together
//! each take a point of dread for having seen it.

use jidousha::prelude::Rng;

use crate::bonds::{change, form, kinship_telling};
use crate::constants::{DREAD_LIMIT, FAILURE_SEEN_DREAD, FRIENDSHIP_AFTER};
use crate::fear::{Occasion, add_dread, can_dread};
use crate::hero::{Deed, DeedKind, HeroId};
use crate::house::House;
use crate::ids::{BondKind, Outcome, Pool};
use crate::resolve::Afield;
use crate::text::fmt;
use crate::words::W;

/// `a` and `b` (a before b in the party) share the road (SPEC §12.2).
pub fn share_the_road(
    f: &Afield<'_>,
    house: &mut House,
    rng: &mut Rng,
    a: HeroId,
    b: HeroId,
    out: &mut Vec<String>,
) {
    let words = &f.content.words;
    form(&mut house.heroes, a, b, BondKind::Companion, f.year);
    let kind = |house: &House| match house.heroes[a].bond_to(b) {
        Some(bond) => bond.kind,
        None => panic!(
            "[keifu_x_inheritance_r3v2] {} and {} share the road and hold no bond after forming one\n  likely \
             cause: bonds::form refused the pair\n  fix: SPEC §12.2 step 1 forms a companion bond",
            house.heroes[a].name, house.heroes[b].name
        ),
    };
    if f.outcome >= Outcome::Success {
        for (from, to) in [(a, b), (b, a)] {
            if let Some(bond) = house.heroes[from].bonds.iter_mut().find(|x| x.other == to) {
                bond.shared_successes += 1;
            }
        }
        let shared = house.heroes[a].bond_to(b).map_or(0, |x| x.shared_successes);
        match kind(house) {
            BondKind::Rival if f.outcome == Outcome::Triumph => {
                change(&mut house.heroes, a, b, BondKind::Friend, f.year);
                out.push(fmt(
                    &words[W::BondRivalsToFriends],
                    &[&house.heroes[a].name, &house.heroes[b].name],
                ));
            }
            BondKind::Companion if shared >= FRIENDSHIP_AFTER => {
                change(&mut house.heroes, a, b, BondKind::Friend, f.year);
                let pool = house.writing.pick(f.content, Pool::Friendships, rng);
                out.push(fmt(pool, &[&house.heroes[a].name, &house.heroes[b].name]));
                for (me, other) in [(a, b), (b, a)] {
                    let telling = fmt(
                        &words[W::DeedBefriended],
                        &[&house.heroes[other].full_name()],
                    );
                    let hero = &mut house.heroes[me];
                    // SPEC-GAPS KG-34: the friend is the deed's other hero.
                    hero.deeds.push(Deed {
                        kind: DeedKind::Befriended,
                        year: f.year,
                        age: hero.age,
                        place: Some(f.quest.place),
                        weight: 0,
                        other: Some(other),
                        telling,
                    });
                }
            }
            _ => {}
        }
        return;
    }
    if f.outcome == Outcome::Disaster && kind(house) == BondKind::Companion {
        change(&mut house.heroes, a, b, BondKind::Rival, f.year);
        let place = &f.content.lore.places[f.quest.place.index()].name;
        out.push(fmt(
            &words[W::BondCompanionsToRivals],
            &[&house.heroes[a].name, &house.heroes[b].name, place],
        ));
        return;
    }
    // A setback or a disaster, parent and child: the parent's line, then the child's.
    let (parent, child) = match kind(house) {
        BondKind::Child => (a, b),
        BondKind::Parent => (b, a),
        _ => return,
    };
    for (seer, seen, key) in [
        (parent, child, W::BondParentSawChild),
        (child, parent, W::BondChildSawParent),
    ] {
        if !can_dread(&house.heroes[seer]) {
            continue;
        }
        let Some(bond) = house.heroes[seer].bond_to(seen) else {
            continue;
        };
        let kinship = kinship_telling(f.content, bond.kind, &house.heroes[seen]).to_owned();
        let hero = &house.heroes[seer];
        let after = (hero.fear.dread + FAILURE_SEEN_DREAD).min(DREAD_LIMIT);
        let possessive = &f.content.lore.pronouns[hero.pronoun.index()].possessive;
        out.push(fmt(
            &words[key],
            &[
                &hero.name,
                possessive,
                &kinship,
                &after.to_string(),
                &DREAD_LIMIT.to_string(),
            ],
        ));
        out.extend(add_dread(
            f.content,
            &mut house.heroes,
            seer,
            FAILURE_SEEN_DREAD,
            Occasion::At(f.quest.place),
            f.year,
        ));
    }
}
