//! The reward for a won quest (SPEC §7.3, `lineage/tale.jai:171-232`): renown for the
//! house and for each who went, the carriers' extra, the triumph deeds, and the two
//! lessons a quest can teach.

use jidousha::prelude::Rng;

use crate::constants::{
    APTITUDE_LIMIT, CARRIED_RENOWN, QUEST_LESSON, TRIUMPH_RENOWN, YOUTH_QUEST_LEARNING_LIMIT,
};
use crate::destiny::may_still_learn;
use crate::harm::{deed, place_name};
use crate::hero::{DeedKind, HeroId};
use crate::house::House;
use crate::ids::{Aptitude, Destiny, Outcome, Phase, Pool};
use crate::outsiders::is_family;
use crate::resolve::Afield;
use crate::text::fmt;
use crate::words::W;

/// Whether `id` can still grow in `aptitude`: base below 9 and may still learn (§13).
fn can_grow(house: &House, id: HeroId, aptitude: Aptitude) -> bool {
    house.heroes[id].base(aptitude) < APTITUDE_LIMIT && may_still_learn(&house.heroes, id)
}

/// +1 in the quest's aptitude, with `lines.quest.lesson` and a LESSONS line.
fn learn(f: &Afield<'_>, house: &mut House, rng: &mut Rng, id: HeroId, out: &mut Vec<String>) {
    let aptitude = f.quest.aptitude;
    let pool = house.writing.pick(f.content, Pool::Lessons, rng);
    let hero = &mut house.heroes[id];
    hero.aptitudes[aptitude.index()] += QUEST_LESSON;
    out.push(fmt(
        &f.content.words[W::QuestLesson],
        &[
            &fmt(pool, &[&hero.name]),
            &f.content.lore.aptitudes[aptitude.index()],
            &hero.base(aptitude).to_string(),
        ],
    ));
}

/// The reward (SPEC §7.3) for `members` on a SUCCESS or TRIUMPH. At the Door (§16.2) the
/// renown is all of it: no reward or carrier lines, no triumph deeds, no lessons.
pub fn reward(
    f: &Afield<'_>,
    house: &mut House,
    rng: &mut Rng,
    members: &[HeroId],
    out: &mut Vec<String>,
) {
    let words = &f.content.words;
    let at_door = f.quest.is_door_lock();
    let triumph = f.outcome == Outcome::Triumph;
    let renown = f.quest.renown + if triumph { TRIUMPH_RENOWN } else { 0 };
    // Variant: the house earns a quest's renown only if the family went; carriers are
    // counted over the family, and outsiders earn only themselves.
    let family_went = members.iter().any(|&m| is_family(&house.heroes[m]));
    let carriers: Vec<HeroId> = members
        .iter()
        .copied()
        .filter(|&m| is_family(&house.heroes[m]))
        .filter(|&m| house.heroes[m].destiny.kind == Destiny::CarryTheHouse)
        .collect();
    if family_went {
        house.add_renown(renown + carriers.len() as i32 * CARRIED_RENOWN);
    }
    if !at_door {
        let whom = match members {
            [one] => house.heroes[*one].name.clone(),
            _ => words[W::QuestRewardEach].to_owned(),
        };
        let line = if family_went {
            W::QuestReward
        } else {
            W::QuestRewardOutsiders
        };
        out.push(fmt(&words[line], &[&renown.to_string(), &whom]));
        for &carrier in &carriers {
            out.push(fmt(
                &words[W::QuestCarrierBonus],
                &[&house.heroes[carrier].name, &CARRIED_RENOWN.to_string()],
            ));
        }
    }
    for &member in members {
        let hero = &mut house.heroes[member];
        hero.renown += renown;
        if triumph && !at_door {
            let telling = fmt(&words[W::DeedTriumph], &[place_name(f)]);
            deed(f, hero, DeedKind::Triumph, f.quest.danger, telling);
        }
    }
    if at_door {
        return;
    }
    let aptitude = f.quest.aptitude;
    if triumph {
        // The lowest base among those who can still grow; the earliest on ties.
        let mut lowest: Option<HeroId> = None;
        for &member in members {
            if can_grow(house, member, aptitude)
                && lowest.is_none_or(|l| {
                    house.heroes[member].base(aptitude) < house.heroes[l].base(aptitude)
                })
            {
                lowest = Some(member);
            }
        }
        if let Some(learner) = lowest {
            learn(f, house, rng, learner, out);
        }
    }
    // After the triumph lesson, so a youth can learn twice from one triumph [emergent].
    for &member in members {
        if house.heroes[member].phase() == Phase::Youth
            && can_grow(house, member, aptitude)
            && house.heroes[member].base(aptitude) < YOUTH_QUEST_LEARNING_LIMIT
        {
            learn(f, house, rng, member, out);
        }
    }
}
