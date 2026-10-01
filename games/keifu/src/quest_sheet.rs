//! The quest sheet (SPEC §5.4, point at a card): the premise, the trouble's stakes,
//! each call, the full power breakdown, the dice, each outcome with its odds and
//! consequence, the unanswered cost — and the place's history, its own panel.
//!
//! Like the card, a pure reading: the breakdown is `power_lines::party_lines`
//! (which asserts it adds up to the card's `party_power`), the odds the same forecast.

use crate::calls::{call_line, called};
use crate::constants::{DICE_MIDPOINT, SETBACK_MARGIN, TRIUMPH_MARGIN, TRIUMPH_RENOWN};
use crate::content::Content;
use crate::forecast::{forecast, percent};
use crate::hero::HeroId;
use crate::house::House;
use crate::ids::Outcome;
use crate::power::party_power;
use crate::power_lines::party_lines;
use crate::text::{count_words, fmt, signed};
use crate::words::W;

/// How a sheet line is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    /// The quest's title.
    Title,
    /// "NEEDS MIGHT 10", "UNANSWERED: ...".
    Heading,
    /// Ordinary prose.
    Body,
    /// The place, the dice, a power line under its member.
    Note,
}

/// One line of the sheet, and the number set beside it, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetLine {
    /// What it says.
    pub text: String,
    /// How it is set.
    pub ink: Ink,
    /// "+1", "92%", in a column at the right.
    pub value: Option<String>,
}

/// The sheet and the history panel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestSheet {
    /// Top to bottom.
    pub lines: Vec<SheetLine>,
    /// The place's history, top to bottom.
    pub history: Vec<String>,
}

fn line(ink: Ink, text: impl Into<String>, value: Option<String>) -> SheetLine {
    SheetLine {
        text: text.into(),
        ink,
        value,
    }
}

/// The sheet for board slot `quest` with `party` seated.
pub fn quest_sheet(content: &Content, house: &House, quest: usize, party: &[HeroId]) -> QuestSheet {
    let words = &content.words;
    let q = &house.board[quest].quest;
    let template = &content.quest_templates[q.template];
    let place = &content.lore.places[q.place.index()];
    let heroes = &house.heroes;
    let mut out = vec![
        line(Ink::Note, place.title.clone(), None),
        line(Ink::Title, template.title.clone(), None),
        line(Ink::Body, template.premise.clone(), None),
    ];
    if q.trouble > 0 {
        let fewer = if q.seats < q.calm_seats {
            fmt(
                &words[W::QuestSheetFewerSeats],
                &[&q.seats.to_string(), &q.calm_seats.to_string()],
            )
        } else {
            String::new()
        };
        let trouble_line = fmt(
            &place.trouble_line,
            &[&content.lore.year_counts[q.trouble as usize]],
        );
        out.push(line(
            Ink::Body,
            fmt(
                &words[W::QuestSheetTroubleStakes],
                &[
                    &trouble_line,
                    &fewer,
                    &q.trouble.to_string(),
                    &(q.danger - q.calm_danger).to_string(),
                    &(q.renown - q.calm_danger).to_string(),
                ],
            ),
            None,
        ));
    }
    for (id, call) in called(content, heroes, q.facts(), party) {
        out.push(line(Ink::Body, call_line(content, heroes, id, call), None));
    }
    let aptitude = content.lore.aptitudes[q.aptitude.index()].to_uppercase();
    out.push(line(
        Ink::Heading,
        fmt(
            &words[W::QuestSheetNeeds],
            &[&aptitude, &q.demand.to_string()],
        ),
        None,
    ));
    let power = party_power(heroes, party, q.facts(), house.patrons);
    if party.is_empty() {
        out.push(line(Ink::Body, &words[W::QuestSheetNoOne], None));
    } else {
        out.push(line(
            Ink::Heading,
            &words[W::QuestSheetYouBring],
            Some(power.to_string()),
        ));
        for power_line in party_lines(content, heroes, party, q.facts(), house.patrons) {
            let (ink, value) = if power_line.head {
                (Ink::Body, None)
            } else {
                (Ink::Note, Some(signed(power_line.value)))
            };
            out.push(line(ink, power_line.text, value));
        }
    }
    out.push(line(
        Ink::Note,
        fmt(&words[W::QuestSheetDice], &[&DICE_MIDPOINT.to_string()]),
        None,
    ));
    // SPEC-GAPS KG-25: on the sheet each outcome carries its own band's odds.
    let odds = forecast(power, q.demand, !party.is_empty());
    let shown =
        |outcome: Outcome| (!party.is_empty()).then(|| format!("{}%", percent(odds.ways(outcome))));
    out.push(line(
        Ink::Body,
        fmt(
            &words[W::QuestSheetTriumph],
            &[
                &TRIUMPH_MARGIN.to_string(),
                &(q.renown + TRIUMPH_RENOWN).to_string(),
            ],
        ),
        shown(Outcome::Triumph),
    ));
    out.push(line(
        Ink::Body,
        fmt(&words[W::QuestSheetSuccess], &[&q.renown.to_string()]),
        shown(Outcome::Success),
    ));
    out.push(line(
        Ink::Body,
        fmt(&words[W::QuestSheetSetback], &[&SETBACK_MARGIN.to_string()]),
        shown(Outcome::Setback),
    ));
    out.push(line(
        Ink::Body,
        fmt(
            &words[W::QuestSheetDisaster],
            &[&q.danger.to_string(), &q.death_percent().to_string()],
        ),
        shown(Outcome::Disaster),
    ));
    out.push(line(
        Ink::Heading,
        fmt(
            &words[W::QuestSheetUnanswered],
            &[&q.unanswered_cost(house.renown).to_string()],
        ),
        None,
    ));
    QuestSheet {
        lines: out,
        history: place_history(content, house, quest),
    }
}

/// The place's history (SPEC §5.4): how often the house quested there, with its
/// triumphs and disasters, then every hero who fell there with their fate telling.
///
/// SPEC-GAPS KG-27: the fallen are listed under "not quested here yet" too — the
/// Drowned Coast took Elsbeth before the house ever quested there.
pub fn place_history(content: &Content, house: &House, quest: usize) -> Vec<String> {
    let words = &content.words;
    let place = house.board[quest].quest.place;
    let record = house.places[place.index()];
    let mut out = vec![if record.visits == 0 {
        words[W::QuestSheetNotQuested].to_owned()
    } else {
        fmt(
            &words[W::QuestSheetHistory],
            &[
                &count_words(content, record.visits as usize),
                &record.triumphs.to_string(),
                &record.disasters.to_string(),
            ],
        )
    }];
    for &id in house.fallen_at(place) {
        let hero = &house.heroes[id];
        out.push(fmt(
            &words[W::QuestSheetFallen],
            &[&hero.full_name(), &hero.fate_telling],
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{house, id};

    fn texts(sheet: &QuestSheet) -> Vec<(String, Option<String>)> {
        sheet
            .lines
            .iter()
            .map(|l| (l.text.clone(), l.value.clone()))
            .collect()
    }

    #[test]
    fn grave_goods_with_garrick_and_brannoc_reads_line_by_line() {
        let (content, mut house) = house();
        let (garrick, brannoc) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Brannoc"));
        house.board[0].quest.demand = 11;
        let sheet = quest_sheet(&content, &house, 0, &[garrick, brannoc]);
        let s = |t: &str| t.to_owned();
        let v = |t: &str| Some(t.to_owned());
        assert_eq!(
            texts(&sheet),
            [
                (s("The Barrow"), None),
                (s("Grave goods"), None),
                (
                    s("Robbers went in at dusk. Bring them out, or what is left."),
                    None
                ),
                (
                    s("Garrick's dream: Win a triumph at the Barrow. He must triumph."),
                    None
                ),
                (
                    s("Ysolde's dream: Quest at three different places. She must go."),
                    None
                ),
                (s("NEEDS MIGHT 11"), None),
                (s("You bring"), v("12")),
                (s("Garrick, Might 5"), None),
                (s("  carries Thornfall"), v("+1")),
                (s("Brannoc, Might 6"), None),
                (s("Two dice, less 7, are added to that."), None),
                // CONSTANTS §3 at +1: 0, 10, 20, 6 of 36.
                (
                    s("Beat it by 4: +3 renown. The least able learns."),
                    v("17%")
                ),
                (s("Meet it: +2 renown."), v("56%")),
                (s("Miss by up to 4: one is wounded."), v("28%")),
                (
                    s("Miss by more: -2 renown, all wounded, each dies 30 in 100."),
                    v("0%")
                ),
                (s("UNANSWERED: -1 renown, and it grows worse."), None),
            ]
        );
        assert_eq!(sheet.history, ["The house has not quested here yet."]);
    }

    #[test]
    fn nobody_going_reads_no_one_and_no_odds_and_the_coast_remembers_elsbeth() {
        let (content, house) = house();
        let sheet = quest_sheet(&content, &house, 1, &[]);
        let lines = texts(&sheet);
        assert!(lines.contains(&("No one is going.".to_owned(), None)));
        assert!(lines.iter().all(|(t, v)| v.is_none() || t == "You bring"));
        assert_eq!(
            sheet.history,
            [
                "The house has not quested here yet.",
                "Elsbeth Thorne was lost at the Drowned Coast."
            ]
        );
    }

    #[test]
    fn a_troubled_quest_tells_its_stakes_and_a_visited_place_its_history() {
        let (content, mut house) = house();
        let q = &mut house.board[0].quest;
        q.trouble = 1;
        q.calm_seats = 2;
        q.seats = 1;
        q.danger = 3;
        q.renown = 3;
        let place = q.place.index();
        house.places[place].visits = 2;
        house.places[place].triumphs = 1;
        let sheet = quest_sheet(&content, &house, 0, &[]);
        assert!(sheet.lines.iter().any(|l| l.text
            == "The Barrow's dead have walked a year unanswered. Room for 1 where there was \
                room for 2. Each who goes must bring 1 more, the danger is 1 higher, and it pays \
                1 more renown. Answer it, however it goes, and it eases."));
        assert_eq!(
            sheet.history,
            ["Quested here twice: 1 in triumph, 0 in disaster."]
        );
        assert!(
            sheet
                .lines
                .iter()
                .any(|l| l.text == "UNANSWERED: -2 renown, and it grows worse.")
        );
    }
}
