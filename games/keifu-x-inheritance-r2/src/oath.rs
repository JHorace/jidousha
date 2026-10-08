//! The variant's oath (DESIGN decisions 7, 8 and 10): a family member swears a called
//! dream's quest in the family's name; kept, it honours the name, failed, it marks it.
//!
//! Mainline has no personal quest: a dream calls its dreamer to a quest (SPEC §9.6) and
//! that is all. Here a family adult seated on a template quest that calls them by their
//! own dream may swear it (`may_swear`); the call's telling is what the oath needs —
//! triumph, succeed or go (TRIUMPH, SUCCESS, SETBACK) — and an outcome below that fails
//! it. The card's button (`Target::Swear`) toggles the card's one oath, the card and the
//! quest sheet show its stakes, and the resolution judges it at step 5b — all reading
//! `consequence`, so the stakes shown are the stakes paid.

use crate::calls::{Telling, dream_call};
use crate::content::Content;
use crate::forecast::{Forecast, forecast, percent};
use crate::hero::{Hero, HeroId};
use crate::house::House;
use crate::ids::Outcome;
use crate::marks::Mark;
use crate::power::party_power;
use crate::quest::{Quest, Source};
use crate::resolve::Afield;
use crate::text::fmt;
use crate::words::W;

/// The card's one oath: who swore it, and the outcome it needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Oath {
    /// The swearer.
    pub by: HeroId,
    /// The least outcome that keeps it.
    pub need: Outcome,
}

/// What an oath on a quest stakes: a mark of `weight` if failed, `weight` renown if
/// kept (DESIGN decision 8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Consequence {
    /// The quest's danger.
    pub weight: i32,
}

/// The stakes of an oath on `quest` — the one function the card, the sheet and the
/// resolution read.
pub fn consequence(quest: &Quest) -> Consequence {
    Consequence {
        weight: quest.danger,
    }
}

/// Whether `hero` may swear `quest` with `party` seated, and what it would need: a
/// template quest; the hero family, adult and in the party; called by their own dream
/// to triumph, succeed or go — not by a burden, not to stay behind.
pub fn may_swear(
    content: &Content,
    heroes: &[Hero],
    quest: &Quest,
    party: &[HeroId],
    hero: HeroId,
) -> Option<Outcome> {
    let me = &heroes[hero];
    if !matches!(quest.source, Source::Template(_))
        || !me.family
        || !me.is_adult()
        || !party.contains(&hero)
    {
        return None;
    }
    let call = dream_call(content, heroes, hero, quest.facts(), party)?;
    if call.burden {
        return None;
    }
    match call.telling {
        Telling::Triumph => Some(Outcome::Triumph),
        Telling::Succeed => Some(Outcome::Success),
        Telling::Go => Some(Outcome::Setback),
        Telling::StayBehind => None,
    }
}

/// Who would swear board slot `slot` now: the first seated hero, in seat order, who may.
pub fn swearer(content: &Content, house: &House, slot: usize) -> Option<Oath> {
    let quest = &house.board[slot].quest;
    let party = house.party(slot);
    party.iter().find_map(|&hero| {
        may_swear(content, &house.heroes, quest, &party, hero).map(|need| Oath { by: hero, need })
    })
}

/// The card's oath button pressed: withdraw the oath, or swear one for `swearer`.
pub fn toggle(content: &Content, house: &mut House, slot: usize) {
    let next = match house.board[slot].sworn {
        Some(_) => None,
        None => swearer(content, house, slot),
    };
    house.board[slot].sworn = next;
}

/// After the board's seats change: withdraw every oath its swearer may no longer swear
/// as it was sworn — lifted off the card, or the call changed with the party.
pub fn keep_oaths(content: &Content, house: &mut House) {
    for slot in 0..house.board.len() {
        let Some(oath) = house.board[slot].sworn else {
            continue;
        };
        let party = house.party(slot);
        let quest = &house.board[slot].quest;
        if may_swear(content, &house.heroes, quest, &party, oath.by) != Some(oath.need) {
            house.board[slot].sworn = None;
        }
    }
}

/// How many of the 36 pairs fall below `need`: the ways the oath fails.
pub fn fail_ways(odds: &Forecast, need: Outcome) -> i32 {
    Outcome::ALL
        .iter()
        .filter(|&&outcome| outcome < need)
        .map(|&outcome| odds.ways(outcome))
        .sum()
}

/// The chance in 100 that board slot `slot`'s oath fails with its party as seated.
pub fn fail_percent(house: &House, slot: usize, need: Outcome) -> i32 {
    let quest = &house.board[slot].quest;
    let party = house.party(slot);
    let power = party_power(&house.heroes, &party, quest.facts(), house.patrons);
    let odds = forecast(power, quest.demand, !party.is_empty());
    percent(fail_ways(&odds, need))
}

/// The word the call tells `need` with: "triumph", "succeed", "go".
fn need_word(content: &Content, need: Outcome) -> &str {
    let tellings = &content.dream_formats.calls;
    match need {
        Outcome::Triumph => &tellings[1],
        Outcome::Success => &tellings[2],
        Outcome::Setback | Outcome::Disaster => &tellings[3],
    }
}

/// What the card says of its oath: the sworn line once sworn — the need, the mark a
/// failure leaves and the chance of it; the sheet spells out the rest — and the
/// button's label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OathReading {
    /// "Sworn by Garrick: triumph, or a mark of 2 on the name. Fails 72 in 100."
    pub sworn: Option<String>,
    /// "Swear it" or "Withdraw".
    pub button: String,
}

/// The card's oath for board slot `slot`: none when nobody is sworn and nobody may swear
/// (the button is absent then).
pub fn read_oath(content: &Content, house: &House, slot: usize) -> Option<OathReading> {
    let words = &content.words;
    let Some(oath) = house.board[slot].sworn else {
        return swearer(content, house, slot).map(|_| OathReading {
            sworn: None,
            button: words[W::QuestCardSwear].to_owned(),
        });
    };
    let weight = consequence(&house.board[slot].quest).weight.to_string();
    Some(OathReading {
        sworn: Some(fmt(
            &words[W::QuestCardSworn],
            &[
                &house.heroes[oath.by].name,
                need_word(content, oath.need),
                &weight,
                &fail_percent(house, slot, oath.need).to_string(),
            ],
        )),
        button: words[W::QuestCardWithdraw].to_owned(),
    })
}

/// The quest sheet's SWORN BY block, once sworn: the heading, the need, the chance it
/// fails, and what keeping and failing it do.
pub fn sheet_lines(content: &Content, house: &House, slot: usize) -> Option<[String; 5]> {
    let words = &content.words;
    let oath = house.board[slot].sworn?;
    let hero = &house.heroes[oath.by];
    let weight = consequence(&house.board[slot].quest).weight.to_string();
    let possessive = &content.lore.pronouns[hero.pronoun.index()].possessive;
    Some([
        fmt(&words[W::QuestSheetSwornBy], &[&hero.name.to_uppercase()]),
        fmt(
            &words[W::QuestSheetOathNeed],
            &[&hero.name, need_word(content, oath.need)],
        ),
        fmt(
            &words[W::QuestSheetOathFails],
            &[&fail_percent(house, slot, oath.need).to_string()],
        ),
        fmt(
            &words[W::QuestSheetOathKept],
            &[&weight, &hero.name, &weight],
        ),
        fmt(
            &words[W::QuestSheetOathFailed],
            &[
                &weight,
                &hero.name,
                &weight,
                &weight,
                &crate::constants::MARK_DRAIN.to_string(),
                possessive,
            ],
        ),
    ])
}

/// Judge the oath at step 5b of the resolution (DESIGN decision 10), on the living and
/// the dead alike: kept, `+weight` renown to the house and the swearer; failed,
/// `-weight` to each (the swearer's floored at 0) and a mark of `weight` on the
/// swearer.
pub fn judge(f: &Afield<'_>, house: &mut House, oath: Oath, out: &mut Vec<String>) {
    let weight = consequence(f.quest).weight;
    let words = &f.content.words;
    let kept = f.outcome >= oath.need;
    let hero = &mut house.heroes[oath.by];
    let name = hero.name.clone();
    let object = f.content.lore.pronouns[hero.pronoun.index()].object.clone();
    let w = weight.to_string();
    if kept {
        hero.renown += weight;
        house.add_renown(weight);
        out.push(fmt(&words[W::OathKept], &[&name, &w, &object, &w]));
        return;
    }
    hero.renown = (hero.renown - weight).max(0);
    hero.marks.push(Mark {
        title: f.quest.title.clone(),
        place: f.quest.place,
        year: f.year,
        by: oath.by,
        weight,
    });
    house.add_renown(-weight);
    out.push(fmt(&words[W::OathFailed], &[&name, &w, &object, &w, &w]));
}
