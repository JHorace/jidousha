//! Summer resolution (SPEC §7, §7.1, §7.2): set out, and every posted quest is either
//! answered — resolved on a page of its own, step by step in the stated order — or
//! not, and costs the house; then the unanswered total, and healing at home.
//!
//! The dice go through `forecast::margin` and `forecast::outcome`, the functions the
//! card's odds are counted from, so the roll and the forecast read one table (W4's
//! one-source rule). Each quest forecasts at the moment it resolves, so an earlier
//! quest's death, crown or renown can move a later one's numbers [emergent] (OQ-6).

use jidousha::prelude::Rng;

use crate::chance::{between, index};
use crate::constants::DICE_SIDES;
use crate::content::Content;
use crate::destiny::{crown_claims, fire_claims};
use crate::facing::face_the_fear;
use crate::forecast::{margin, outcome};
use crate::ghost::{ghost_text, lay_ghost};
use crate::harm::{burn, crown, deed, place_name, suffer_disaster, wound};
use crate::hero::{DeedKind, HeroId};
use crate::house::House;
use crate::ids::Outcome;
use crate::moment::{Moment, QuestMoment};
use crate::power::party_power;
use crate::quest::{Quest, Source};
use crate::reward::reward;
use crate::road::share_the_road;
use crate::telling::{QuestPage, Telling};
use crate::text::{fmt, name_list};
use crate::witness::witness_all;
use crate::words::W;

/// What the rules of one resolved quest share: the content, the quest as posted, how
/// it went, and the year.
pub struct Afield<'c> {
    /// The content.
    pub content: &'c Content,
    /// The quest.
    pub quest: &'c Quest,
    /// How it went.
    pub outcome: Outcome,
    /// The current year.
    pub year: i32,
}

/// Set out (SPEC §7): reset the telling, resolve or leave unanswered every posted
/// quest in board order, take the unanswered total, heal whoever stayed home wounded,
/// and keep the telling on the house.
pub fn set_out(content: &Content, house: &mut House, rng: &mut Rng) {
    assert!(
        house.telling.is_none() && !house.closed && house.ending.is_none(),
        "[keifu] set out with a telling already open or the house closed\n  likely cause: the \
         set-out control was offered off the summer screen\n  fix: offer it only in summer"
    );
    let mut telling = Telling {
        year: house.calendar.current_year(),
        ..Telling::default()
    };
    // 2. The last summer: the Door, and nothing else — no unanswered costs, no healing.
    if house.calendar.door_stands_open() {
        telling.door = Some(crate::door::try_the_door(
            content,
            house,
            rng,
            None,
            &mut telling.pages,
        ));
        house.telling = Some(telling);
        return;
    }
    let mut cost = 0;
    // SPEC-GAPS KG-35: each party is read off its seats as it resolves; nothing bounces
    // a hero an earlier quest's grief broke, so they go, at their fear's cost.
    for slot in 0..house.board.len() {
        if house.party(slot).is_empty() {
            cost += unanswered(content, house, slot, &mut telling.meanwhile);
        } else {
            telling.pages.push(resolve_quest(content, house, rng, slot));
        }
    }
    let words = &content.words;
    if cost > 0 {
        house.add_renown(-cost);
        telling
            .meanwhile
            .push(fmt(&words[W::SummerUnansweredTotal], &[&cost.to_string()]));
    }
    // Healing at home: everyone still in a roster seat, in seat order.
    for seat in 0..house.roster.len() {
        let Some(id) = house.roster[seat] else {
            continue;
        };
        if house.heroes[id].wounded {
            house.heroes[id].wounded = false;
            telling.meanwhile.push(fmt(
                &words[W::SummerMendedAtHome],
                &[&house.heroes[id].name],
            ));
        }
    }
    house.telling = Some(telling);
}

/// An unanswered quest (SPEC §7.2): its cost at the house's renown now, the place's
/// trouble +1 (at most 2), and the place's trouble line. Returns the cost.
fn unanswered(content: &Content, house: &mut House, slot: usize, out: &mut Vec<String>) -> i32 {
    let quest = &house.board[slot].quest;
    let cost = quest.unanswered_cost(house.renown);
    let place = quest.place;
    let record = &mut house.places[place.index()];
    record.trouble = crate::outlook::trouble_if_unanswered(record.trouble);
    let lore = &content.lore;
    let trouble = fmt(
        &lore.places[place.index()].trouble_line,
        &[&lore.year_counts[record.trouble as usize]],
    );
    out.push(fmt(&content.words[W::SummerUnanswered], &[&trouble]));
    cost
}

/// The quest's ending for `outcome`, the party's first names in it (SPEC §7.1 step 2).
fn story(
    content: &Content,
    house: &House,
    quest: &Quest,
    outcome: Outcome,
    members: &[HeroId],
) -> String {
    let names: Vec<&str> = members
        .iter()
        .map(|&m| house.heroes[m].name.as_str())
        .collect();
    let party = name_list(content, &names);
    let ending = match quest.source {
        Source::Template(template) => {
            content.quest_templates[template].endings[outcome.index()].clone()
        }
        Source::Ghost(dead) => {
            let Some(ghost) = house.ghosts.iter().find(|ghost| ghost.hero == dead) else {
                panic!(
                    "[keifu] {}'s ghost quest is posted and the ghost is not on the list\n  \
                     likely cause: the ghost was removed while its quest stood\n  fix: SPEC \
                     §14.4 removes a ghost only when it is laid or taken up",
                    house.heroes[dead].name
                );
            };
            let text = &content.ghost.endings[outcome.index()];
            ghost_text(content, text, &house.heroes[dead], &ghost.dream)
        }
        // The Door writes its own story, naming the bearer (SPEC §16.2, `door.rs`).
        Source::Door(lock) => panic!(
            "[keifu] lock {lock} of the Door was told as a posted quest\n  likely cause: the \
             Door's board was set out as an ordinary summer\n  fix: SPEC §7 step 2 — the last \
             summer resolves the Door (door::try_the_door)"
        ),
    };
    fmt(&ending, &[&party])
}

/// Resolve the quest on board slot `slot` (SPEC §7.1): roll two dice, then the rest.
pub fn resolve_quest(
    content: &Content,
    house: &mut House,
    rng: &mut Rng,
    slot: usize,
) -> QuestPage {
    let dice = [between(rng, 1, DICE_SIDES), between(rng, 1, DICE_SIDES)];
    resolve_rolled(content, house, rng, slot, dice)
}

/// Resolve the quest on board slot `slot` with `dice` thrown (SPEC §7.1), in exactly
/// the stated order. The checks stage an outcome by choosing the dice.
pub fn resolve_rolled(
    content: &Content,
    house: &mut House,
    rng: &mut Rng,
    slot: usize,
    dice: [i32; 2],
) -> QuestPage {
    let quest = house.board[slot].quest.clone();
    let members = house.party(slot);
    let (q, m) = (quest.clone(), members.clone());
    let told = move |house: &House, outcome: Outcome| story(content, house, &q, outcome, &m);
    resolve_party(content, house, rng, quest, members, dice, &told)
}

/// Resolve `quest` for `members` with `dice` thrown (SPEC §7.1), in exactly the stated
/// order — a posted quest's, or a lock of the Sealed Door's (§16.2), whose `told` story
/// names its bearer. At the Door there are no reward or disaster lines, no triumph deeds
/// or lessons, and no sharing of the road; everything else applies.
pub fn resolve_party(
    content: &Content,
    house: &mut House,
    rng: &mut Rng,
    quest: Quest,
    members: Vec<HeroId>,
    dice: [i32; 2],
    told: &dyn Fn(&House, Outcome) -> String,
) -> QuestPage {
    let at_door = quest.is_door_lock();
    // 1-2. Forecast with the party and the patrons; band the margin the dice make.
    let power = party_power(&house.heroes, &members, quest.facts(), house.patrons);
    let margin = margin(power, dice[0], dice[1], quest.demand);
    let outcome = outcome(margin);
    let f = Afield {
        content,
        quest: &quest,
        outcome,
        year: house.calendar.current_year(),
    };
    let story = told(house, outcome);
    let words = &content.words;
    let mut lines = Vec::new();
    // 3. The place's history.
    let record = &mut house.places[quest.place.index()];
    record.visits += 1;
    match outcome {
        Outcome::Triumph => record.triumphs += 1,
        Outcome::Disaster => record.disasters += 1,
        _ => {}
    }
    // 4. Quests faced, and the first quest ever.
    for &member in &members {
        let hero = &mut house.heroes[member];
        hero.quests_faced += 1;
        // SPEC-GAPS KG-40: "the first time ever" is the count reaching 1.
        if hero.quests_faced == 1 {
            let telling = fmt(
                &words[W::DeedFirstQuest],
                &[place_name(&f), &hero.age.to_string()],
            );
            deed(&f, hero, DeedKind::FirstQuest, quest.danger, telling);
        }
    }
    // 5. The reward.
    let won = outcome >= Outcome::Success;
    if won {
        reward(&f, house, rng, &members, &mut lines);
    }
    // 6. Trouble eases: gone on a win, one less on a loss.
    let record = &mut house.places[quest.place.index()];
    record.trouble = crate::outlook::trouble_if_answered(record.trouble, won);
    // 7. A disaster costs the house its danger (and tells it, but not at the Door).
    if outcome == Outcome::Disaster {
        house.add_renown(-quest.danger);
        if !at_door {
            lines.push(fmt(
                &words[W::QuestDisasterRenown],
                &[&quest.danger.to_string()],
            ));
        }
    }
    // 8. Each member faces the fear.
    for &member in &members {
        face_the_fear(&f, house, rng, &members, member, &mut lines);
    }
    // 9. A setback: the fire takes everyone it claims, the unlucky one last; else a wound.
    if outcome == Outcome::Setback {
        let unlucky = members[index(rng, members.len())];
        for &member in members.iter().filter(|&&m| m != unlucky) {
            if fire_claims(&house.heroes[member], &quest.tags, outcome) {
                burn(&f, house, member, &mut lines);
            }
        }
        if fire_claims(&house.heroes[unlucky], &quest.tags, outcome) {
            burn(&f, house, unlucky, &mut lines);
        } else {
            wound(&f, house, rng, unlucky, &mut lines);
        }
    }
    // 10. A disaster: each member suffers it.
    if outcome == Outcome::Disaster {
        for &member in &members {
            suffer_disaster(&f, house, rng, member, &mut lines);
        }
    }
    // 11. Every pair still living shares the road — unless this is the Door.
    for (i, &a) in members.iter().enumerate().filter(|_| !at_door) {
        for &b in &members[i + 1..] {
            if house.heroes[a].is_living() && house.heroes[b].is_living() {
                share_the_road(&f, house, rng, a, b, &mut lines);
            }
        }
    }
    // 12. Every living hero witnesses it; the party were present.
    let moment = Moment::Quest(QuestMoment {
        place: quest.place,
        tags: &quest.tags,
        outcome,
        party: &members,
    });
    lines.extend(witness_all(content, house, &moment));
    // 13. Every member, living or dead, has walked this road.
    for &member in &members {
        let roads = &mut house.heroes[member].roads_walked;
        if !roads.contains(&quest.place) {
            roads.push(quest.place);
        }
    }
    // 14. A triumph: the crown claims whom it claims.
    if outcome == Outcome::Triumph {
        for &member in &members {
            if house.heroes[member].is_living()
                && crown_claims(&house.heroes[member], quest.place, outcome)
            {
                crown(&f, house, rng, member, &mut lines);
            }
        }
    }
    // 15. A won ghost's quest lays the ghost.
    if let (true, Source::Ghost(dead)) = (won, quest.source) {
        lay_ghost(&f, house, dead, &members, &mut lines);
    }
    QuestPage {
        members,
        power,
        dice,
        margin,
        outcome,
        story,
        lines,
        quest,
    }
}
