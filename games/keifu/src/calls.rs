//! Dream calls (SPEC §9.6): which quests "call" a dreamer, and how the call is told
//! on the quest card ("Dream: Garrick, Ysolde") and the quest sheet ("Garrick's
//! dream: Win a triumph at the Barrow. He must triumph.").
//!
//! A call asks `moment::stage_met` — the function witnessing asks — against moments
//! that have not happened: the year turning, a fictitious triumph at the Door, the
//! quest with the dreamer added, and the quest without them. So the card can only
//! name a dreamer the roll could move. The card and sheet are W4's surfaces; these
//! are the readings they will show.

use crate::content::Content;
use crate::dream::Dream;
use crate::hero::{Hero, HeroId};
use crate::ids::{Outcome, Place};
use crate::moment::{Moment, QuestMoment, stage_met};
use crate::power::QuestFacts;
use crate::text::{capitalized, fmt};
use crate::words::W;

/// How a call is told (`dreams.json` `call_tellings`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Telling {
    /// The quest moves the dream only if the dreamer stays behind.
    StayBehind,
    /// It needs a triumph.
    Triumph,
    /// It needs a success.
    Succeed,
    /// Going is enough.
    Go,
}

/// A call on one dreamer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Call {
    /// The call is from the dream they carry, not their own.
    pub burden: bool,
    /// How it is told.
    pub telling: Telling,
}

/// Whether `quest`, with `party` seated, calls `hero` (SPEC §9.6): the own dream
/// first, then the burden.
pub fn dream_call(
    content: &Content,
    heroes: &[Hero],
    hero: HeroId,
    quest: QuestFacts<'_>,
    party: &[HeroId],
) -> Option<Call> {
    let me = &heroes[hero];
    let own = me
        .dream
        .as_ref()
        .and_then(|d| call_of(content, heroes, hero, d, quest, party));
    if let Some(telling) = own {
        return Some(Call {
            burden: false,
            telling,
        });
    }
    let telling = me
        .burden
        .as_ref()
        .and_then(|d| call_of(content, heroes, hero, d, quest, party))?;
    Some(Call {
        burden: true,
        telling,
    })
}

/// One dream's call, steps 1-5 of SPEC §9.6.
fn call_of(
    content: &Content,
    heroes: &[Hero],
    hero: HeroId,
    dream: &Dream,
    quest: QuestFacts<'_>,
    party: &[HeroId],
) -> Option<Telling> {
    // 1. No current stage, no call.
    if dream.is_fulfilled() {
        return None;
    }
    let stage = dream.current;
    let met = |moment: Moment<'_>| stage_met(content, heroes, hero, dream, stage, &moment);
    // 2. Met by the year turning: it completes on its own.
    if met(Moment::YearTurn) {
        return None;
    }
    // 3. Not going, and met by a quest anywhere: a triumph at the Door, alone, no tags.
    let going = party.contains(&hero);
    let alone = [hero];
    let anywhere = QuestMoment {
        place: Place::SealedDoor,
        tags: &[],
        outcome: Outcome::Triumph,
        party: &alone,
    };
    if !going && met(Moment::Quest(anywhere)) {
        return None;
    }
    // 4. Present, with the dreamer added: the first outcome that meets it is needed.
    let mut with = party.to_vec();
    if !going {
        with.push(hero);
    }
    let needed = Outcome::ALL.iter().copied().find(|&outcome| {
        met(Moment::Quest(QuestMoment {
            place: quest.place,
            tags: quest.tags,
            outcome,
            party: &with,
        }))
    });
    if needed.is_some() || going {
        return needed.map(|outcome| match outcome {
            Outcome::Triumph => Telling::Triumph,
            Outcome::Success => Telling::Succeed,
            Outcome::Setback | Outcome::Disaster => Telling::Go,
        });
    }
    // 5. Absent, the party as it is: an "apart" call.
    Outcome::ALL
        .iter()
        .copied()
        .any(|outcome| {
            met(Moment::Quest(QuestMoment {
                place: quest.place,
                tags: quest.tags,
                outcome,
                party,
            }))
        })
        .then_some(Telling::StayBehind)
}

/// Every living adult the quest calls with `party` seated, in creation order.
pub fn called(
    content: &Content,
    heroes: &[Hero],
    quest: QuestFacts<'_>,
    party: &[HeroId],
) -> Vec<(HeroId, Call)> {
    (0..heroes.len())
        .filter(|&id| heroes[id].is_living() && heroes[id].is_adult())
        .filter_map(|id| dream_call(content, heroes, id, quest, party).map(|call| (id, call)))
        .collect()
}

/// The card's "Dream: Garrick, Ysolde", or `None` when nobody is called (SPEC §5.4).
pub fn dreamers_line(
    content: &Content,
    heroes: &[Hero],
    quest: QuestFacts<'_>,
    party: &[HeroId],
) -> Option<String> {
    let names: Vec<&str> = called(content, heroes, quest, party)
        .iter()
        .map(|(id, _)| heroes[*id].name.as_str())
        .collect();
    if names.is_empty() {
        return None;
    }
    // SPEC-GAPS KG-22: "comma-separated" is the lore's name-list separator.
    Some(fmt(
        &content.words[W::QuestCardDreamers],
        &[&names.join(&content.lore.name_list_separator)],
    ))
}

/// The quest sheet's line for one call: "<Name>'s dream|burden: <current task>.
/// <He> must <telling>." (SPEC §5.4, `ui.quest_sheet.dream_call`).
pub fn call_line(content: &Content, heroes: &[Hero], hero: HeroId, call: Call) -> String {
    let words = &content.words;
    let me = &heroes[hero];
    let dream = if call.burden {
        me.burden.as_ref()
    } else {
        me.dream.as_ref()
    };
    let task = dream
        .and_then(|d| d.stages.get(d.current))
        .map_or("", |stage| stage.task.as_str());
    // SPEC-GAPS KG-13: the word is the sheet's heading, lowered whole.
    let which = if call.burden {
        W::SheetBurden
    } else {
        W::SheetDream
    };
    let tellings = &content.dream_formats.calls;
    let telling = match call.telling {
        Telling::StayBehind => &tellings[0],
        Telling::Triumph => &tellings[1],
        Telling::Succeed => &tellings[2],
        Telling::Go => &tellings[3],
    };
    let he = capitalized(&content.lore.pronouns[me.pronoun.index()].subject);
    fmt(
        &words[W::QuestSheetDreamCall],
        &[&me.name, &words[which].to_lowercase(), task, &he, telling],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bonds::form;
    use crate::content::QuestTemplate;
    use crate::ids::{BondKind, DreamKind};
    use crate::testkit::{founded, id};

    fn template<'c>(content: &'c Content, title: &str) -> &'c QuestTemplate {
        content
            .quest_templates
            .iter()
            .find(|t| t.title == title)
            .expect("a template by that title")
    }

    #[test]
    fn garrick_seated_on_grave_goods_is_called_to_triumph_and_ysolde_to_a_road_she_has_not_walked()
    {
        let (content, heroes) = founded();
        let (garrick, ysolde) = (id(&heroes, "Garrick"), id(&heroes, "Ysolde"));
        let grave = QuestFacts::of(template(&content, "Grave goods"));
        let call = dream_call(&content, &heroes, garrick, grave, &[garrick]);
        assert_eq!(
            call,
            Some(Call {
                burden: false,
                telling: Telling::Triumph
            })
        );
        assert_eq!(
            dreamers_line(&content, &heroes, grave, &[garrick]).as_deref(),
            Some("Dream: Garrick, Ysolde")
        );
        assert_eq!(
            call_line(
                &content,
                &heroes,
                garrick,
                Call {
                    burden: false,
                    telling: Telling::Triumph
                }
            ),
            "Garrick's dream: Win a triumph at the Barrow. He must triumph."
        );
        assert_eq!(
            dream_call(&content, &heroes, ysolde, grave, &[garrick]),
            Some(Call {
                burden: false,
                telling: Telling::Go
            })
        );
        // Unseated, Garrick is still called: the Barrow is where his dream is.
        assert_eq!(
            dream_call(&content, &heroes, garrick, grave, &[]),
            Some(Call {
                burden: false,
                telling: Telling::Triumph
            })
        );
        // At the Court, where Ysolde has walked and Garrick's dream is not: nobody.
        let court = QuestFacts::of(template(&content, "The long feast"));
        assert_eq!(dreamers_line(&content, &heroes, court, &[garrick]), None);
    }

    #[test]
    fn carrying_a_blade_to_a_triumph_calls_only_the_quest_the_dreamer_sits_on() {
        let (content, mut heroes) = founded();
        let brannoc = id(&heroes, "Brannoc");
        if let Some(dream) = heroes[brannoc].dream.as_mut() {
            dream.advance_to_stage(2);
        }
        let grave = QuestFacts::of(template(&content, "Grave goods"));
        assert_eq!(dream_call(&content, &heroes, brannoc, grave, &[]), None);
        assert_eq!(
            dream_call(&content, &heroes, brannoc, grave, &[brannoc]),
            Some(Call {
                burden: false,
                telling: Telling::Triumph
            })
        );
    }

    #[test]
    fn a_teacher_is_called_to_stay_behind_when_the_student_goes_alone() {
        let (content, mut heroes) = founded();
        let (odo, wren) = (id(&heroes, "Odo"), id(&heroes, "Wren"));
        form(&mut heroes, wren, odo, BondKind::Mentor, 1);
        if let Some(bond) = heroes[odo].bonds.iter_mut().find(|b| b.other == wren) {
            bond.taught = true;
        }
        if let Some(dream) = heroes[odo].dream.as_mut() {
            dream.advance_to_stage(2);
        }
        let grave = QuestFacts::of(template(&content, "Grave goods"));
        let alone = Some(Call {
            burden: false,
            telling: Telling::StayBehind,
        });
        assert_eq!(dream_call(&content, &heroes, odo, grave, &[wren]), alone);
        assert_eq!(
            call_line(
                &content,
                &heroes,
                odo,
                Call {
                    burden: false,
                    telling: Telling::StayBehind
                }
            ),
            "Odo's dream: See a student succeed without you. He must stay behind."
        );
        // Beside the student, the student's own success is no longer "alone".
        assert_eq!(
            dream_call(&content, &heroes, odo, grave, &[wren, odo]),
            None
        );
        // With no student seated, nothing calls.
        assert_eq!(dream_call(&content, &heroes, odo, grave, &[]), None);
        // Stage two wants him beside the student: it needs only that he goes.
        if let Some(dream) = heroes[odo].dream.as_mut() {
            dream.current = 1;
        }
        assert_eq!(
            dream_call(&content, &heroes, odo, grave, &[wren]),
            Some(Call {
                burden: false,
                telling: Telling::Go
            })
        );
    }

    #[test]
    fn a_stage_the_year_will_finish_calls_no_quest_and_a_stage_met_by_success_is_told_so() {
        let (content, mut heroes) = founded();
        let (brannoc, maren) = (id(&heroes, "Brannoc"), id(&heroes, "Maren"));
        heroes[brannoc].dream = Some(
            crate::dream::Dream::build(&content, DreamKind::SeeAChildGrown, None, None)
                .expect("builds"),
        );
        let grave = QuestFacts::of(template(&content, "Grave goods"));
        assert_eq!(
            dream_call(&content, &heroes, brannoc, grave, &[brannoc]),
            None,
            "Brannoc is wed already"
        );
        let bell = QuestFacts::of(template(&content, "The bell under the tide"));
        // Maren's "Return to the Drowned Coast" needs only that she goes.
        assert_eq!(
            dream_call(&content, &heroes, maren, bell, &[]),
            Some(Call {
                burden: false,
                telling: Telling::Go
            })
        );
        if let Some(dream) = heroes[maren].dream.as_mut() {
            dream.advance_to_stage(2);
        }
        assert_eq!(
            dream_call(&content, &heroes, maren, bell, &[]),
            Some(Call {
                burden: false,
                telling: Telling::Succeed
            })
        );
        // The undead at the Barrow are not the water that took her mother.
        let lamps = QuestFacts::of(template(&content, "The lamps in the Barrow"));
        assert_eq!(dream_call(&content, &heroes, maren, lamps, &[maren]), None);
    }

    #[test]
    fn a_burden_calls_only_when_the_own_dream_does_not_and_is_told_as_a_burden() {
        let (content, mut heroes) = founded();
        let (pip, maren, garrick) = (
            id(&heroes, "Pip"),
            id(&heroes, "Maren"),
            id(&heroes, "Garrick"),
        );
        heroes[pip].age = 12;
        let mut carried = heroes[garrick].dream.clone();
        if let Some(dream) = carried.as_mut() {
            dream.owner = Some(garrick);
        }
        heroes[pip].burden = carried;
        let grave = QuestFacts::of(template(&content, "Grave goods"));
        let call = dream_call(&content, &heroes, pip, grave, &[pip]);
        assert_eq!(
            call,
            Some(Call {
                burden: true,
                telling: Telling::Triumph
            })
        );
        assert_eq!(
            call_line(
                &content,
                &heroes,
                pip,
                Call {
                    burden: true,
                    telling: Telling::Triumph
                }
            ),
            "Pip's burden: Win a triumph at the Barrow. He must triumph."
        );
        // At the Coast his own dream calls, and the burden is not asked.
        let bell = QuestFacts::of(template(&content, "The bell under the tide"));
        assert_eq!(
            dream_call(&content, &heroes, pip, bell, &[pip]),
            Some(Call {
                burden: false,
                telling: Telling::Go
            })
        );
        // A child is never on the card.
        heroes[pip].age = 10;
        let names: Vec<HeroId> = called(&content, &heroes, grave, &[])
            .iter()
            .map(|(id, _)| *id)
            .collect();
        assert!(!names.contains(&pip));
        assert!(!names.contains(&maren));
    }
}
