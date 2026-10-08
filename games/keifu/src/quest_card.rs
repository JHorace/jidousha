//! The quest card as information (SPEC §5.4): what the player reads at the moment
//! of choosing whom to send.
//!
//! `read_card` is a pure reading of one posted quest for a given party — the seated
//! one, or the one a drag is previewing (`preview_party`). Every number on it comes
//! from the functions the roll and the resolution will read: `party_power`, the
//! forecast, the quest's own stakes, the dream calls and the fear line.

use crate::board::Slot;
use crate::calls::dreamers_line;
use crate::content::Content;
use crate::fear::{fears, refuses};
use crate::forecast::{Forecast, card_percentages, forecast};
use crate::hero::HeroId;
use crate::house::House;
use crate::power::{fear_line, party_power, you_bring};
use crate::quest::Source;
use crate::text::fmt;
use crate::words::W;

/// Everything one quest card says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CardReading {
    /// "The Barrow".
    pub place: String,
    /// "Grave goods".
    pub title: String,
    /// Each tag's title, and whether a seated or watched hero fears it.
    pub tags: Vec<(String, bool)>,
    /// "Danger".
    pub danger_word: String,
    /// One pip per point of danger.
    pub danger: i32,
    /// "Needs Might 10".
    pub needs: String,
    /// "you bring 12", when anyone is going.
    pub you_bring: Option<String>,
    /// The 36 pairs, for the odds bar.
    pub forecast: Forecast,
    /// "Succeed 92%", "triumph 42%", "Setback 8%", "disaster 0%", when anyone is going.
    pub odds: Option<[String; 4]>,
    /// With nobody going: the place's trouble line, or "No one is going. Room for 2."
    pub idle: Option<String>,
    /// With nobody going, what leaving it does: "Left: danger 3-4, room 1-2"
    /// (`outlook::telegraph`; not in lineage).
    pub telegraph: Option<String>,
    /// "Renown +2".
    pub renown: String,
    /// "unanswered -1", at the house's renown now.
    pub unanswered: String,
    /// "Dream: Garrick, Ysolde", or on a ghost's quest "The ghost of Garrick".
    pub dream: Option<String>,
    /// "Ysolde will not go." for a watched hero who refuses, else the fear line.
    pub warning: Option<String>,
}

/// The seats board slot `quest` shows while `hand` (a hero, and where they would
/// land) is held: everyone seated but the hero in hand, who has left their seat;
/// and the hero in hand too, where they would land, if that is an empty seat on
/// this quest and they would go (SPEC §5.4 "live preview": a card with room;
/// SPEC-GAPS KG-28: the whole card reads this party).
/// With nothing in hand it is the seats as they are.
pub fn preview_seats(
    house: &House,
    quest: usize,
    hand: Option<(HeroId, Option<Slot>)>,
) -> Vec<Option<HeroId>> {
    let mut seats = house.board[quest].seats.clone();
    let Some((hero, landing)) = hand else {
        return seats;
    };
    for seat in seats.iter_mut().filter(|s| **s == Some(hero)) {
        *seat = None;
    }
    if let Some(Slot::Quest { quest: q, seat }) = landing
        && q == quest
        && seats[seat].is_none()
        && !refuses(&house.heroes[hero], &house.board[quest].quest.tags)
    {
        seats[seat] = Some(hero);
    }
    seats
}

/// Read board slot `quest` for `party`, with `watched` the hero pointed at or in hand.
pub fn read_card(
    content: &Content,
    house: &House,
    quest: usize,
    party: &[HeroId],
    watched: Option<HeroId>,
) -> CardReading {
    let words = &content.words;
    let q = &house.board[quest].quest;
    let place = &content.lore.places[q.place.index()];
    let heroes = &house.heroes;
    let power = party_power(heroes, party, q.facts(), house.patrons);
    let anyone = !party.is_empty();
    let odds_of = forecast(power, q.demand, anyone);
    let tags = q
        .tags
        .iter()
        .map(|&tag| {
            let feared = party
                .iter()
                .chain(watched.iter())
                .any(|&h| fears(&heroes[h], &[tag]));
            (content.lore.tags[tag.index()].title.clone(), feared)
        })
        .collect();
    let odds = anyone.then(|| {
        let [succeed, triumph, setback, disaster] = card_percentages(&odds_of);
        [
            (W::QuestCardSucceed, succeed),
            (W::QuestCardTriumph, triumph),
            (W::QuestCardSetback, setback),
            (W::QuestCardDisaster, disaster),
        ]
        .map(|(word, value)| fmt(&words[word], &[&value.to_string()]))
    });
    let idle = (!anyone).then(|| {
        if q.trouble > 0 {
            fmt(
                &place.trouble_line,
                &[&content.lore.year_counts[q.trouble as usize]],
            )
        } else {
            // SPEC-GAPS KG-29: one line, joined by a space, as SPEC §5.4 quotes it.
            format!(
                "{} {}",
                &words[W::QuestCardNoOne],
                fmt(&words[W::QuestCardRoomFor], &[&q.seats.to_string()])
            )
        }
    });
    let refusing = watched.filter(|&h| refuses(&heroes[h], &q.tags));
    let warning = match refusing {
        Some(h) => Some(fmt(&words[W::QuestCardWillNotGo], &[&heroes[h].name])),
        None => fear_line(content, heroes, party, &q.tags),
    };
    CardReading {
        place: place.title.clone(),
        title: q.title.clone(),
        tags,
        danger_word: words[W::QuestCardDanger].to_owned(),
        danger: q.danger,
        needs: fmt(
            &words[W::QuestCardNeeds],
            &[
                &content.lore.aptitudes[q.aptitude.index()],
                &q.demand.to_string(),
            ],
        ),
        you_bring: anyone.then(|| you_bring(content, power)),
        forecast: odds_of,
        odds,
        idle,
        telegraph: if anyone {
            None
        } else {
            crate::outlook::telegraph(content, house, quest)
                .map(|t| crate::outlook::card_line(content, &t))
        },
        renown: fmt(&words[W::QuestCardRenown], &[&q.renown.to_string()]),
        unanswered: fmt(
            &words[W::QuestCardUnanswered],
            &[&q.unanswered_cost(house.renown).to_string()],
        ),
        // SPEC §5.4: "either 'The ghost of <name>' or 'Dream: <names>'"; a ghost's
        // quest shows the ghost line (SPEC-GAPS KG-33).
        dream: match q.source {
            Source::Ghost(dead) => Some(fmt(&words[W::QuestCardGhost], &[&heroes[dead].name])),
            Source::Template(_) | Source::Door(_) => {
                dreamers_line(content, heroes, q.facts(), party)
            }
        },
        warning,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{house, id};

    fn preview_party(
        house: &House,
        quest: usize,
        hand: Option<(HeroId, Option<Slot>)>,
    ) -> Vec<HeroId> {
        preview_seats(house, quest, hand)
            .into_iter()
            .flatten()
            .collect()
    }

    #[test]
    fn garrick_and_brannoc_on_grave_goods_need_might_and_bring_twelve() {
        let (content, mut house) = house();
        let (garrick, brannoc) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Brannoc"));
        let empty = read_card(&content, &house, 0, &[], None);
        assert_eq!(
            (empty.place.as_str(), empty.title.as_str()),
            ("The Barrow", "Grave goods")
        );
        assert_eq!(empty.idle.as_deref(), Some("No one is going. Room for 2."));
        assert_eq!((empty.you_bring.clone(), empty.odds.clone()), (None, None));
        assert_eq!(
            (empty.renown.as_str(), empty.unanswered.as_str()),
            ("Renown +2", "unanswered -1")
        );
        house.board[0].seats = vec![Some(garrick), Some(brannoc)];
        house.board[0].quest.demand = 10;
        let card = read_card(&content, &house, 0, &house.party(0), None);
        assert_eq!(card.needs, "Needs Might 10");
        assert_eq!(card.you_bring.as_deref(), Some("you bring 12"));
        // CONSTANTS §3 at +2: 0, 6, 20, 10 of 36.
        assert_eq!(
            card.odds,
            Some(["Succeed 83%", "triumph 28%", "Setback 17%", "disaster 0%"].map(String::from))
        );
        assert_eq!(card.tags, [("Dark".to_owned(), false)]);
        assert_eq!(card.idle, None);
        assert_eq!(card.danger, 2);
    }

    #[test]
    fn a_watched_hero_who_refuses_is_named_and_their_fear_lights_the_tag() {
        let (content, mut house) = house();
        let ysolde = id(&house.heroes, "Ysolde");
        let card = read_card(&content, &house, 0, &[], Some(ysolde));
        assert_eq!(card.tags, [("Dark".to_owned(), true)]);
        assert_eq!(card.warning, None);
        house.heroes[ysolde].fear.broken = true;
        let card = read_card(&content, &house, 0, &[], Some(ysolde));
        assert_eq!(card.warning.as_deref(), Some("Ysolde will not go."));
    }

    #[test]
    fn a_troubled_quest_with_nobody_going_shows_its_trouble_line() {
        let (content, mut house) = house();
        house.board[1].quest.trouble = 2;
        let card = read_card(&content, &house, 1, &[], None);
        assert_eq!(
            card.idle.as_deref(),
            Some("The tide has had the Drowned Coast to itself for two years.")
        );
    }

    #[test]
    fn the_hand_leaves_its_seat_and_previews_where_it_would_land() {
        let (_, mut house) = house();
        let (garrick, brannoc, ysolde) = (
            id(&house.heroes, "Garrick"),
            id(&house.heroes, "Brannoc"),
            id(&house.heroes, "Ysolde"),
        );
        let from = house.slot_of(garrick).expect("Garrick is seated");
        house.drop_hero(from, Some(Slot::Quest { quest: 0, seat: 0 }));
        let card = Some(crate::screen::Target::Quest(0));
        let land = house.landing(brannoc, card);
        assert_eq!(land, Some(Slot::Quest { quest: 0, seat: 1 }));
        assert_eq!(
            preview_party(&house, 0, Some((brannoc, land))),
            [garrick, brannoc]
        );
        assert_eq!(
            preview_party(&house, 1, Some((brannoc, land))),
            Vec::<HeroId>::new()
        );
        // Garrick lifted from his own seat leaves it; over his card he lands back on it.
        assert_eq!(
            preview_party(&house, 0, Some((garrick, None))),
            Vec::<HeroId>::new()
        );
        let back = house.landing(garrick, card);
        assert_eq!(back, Some(Slot::Quest { quest: 0, seat: 0 }));
        assert_eq!(preview_party(&house, 0, Some((garrick, back))), [garrick]);
        // A full card has no room: over it, or over a seated hero (a swap), no preview.
        let from = house.slot_of(brannoc).expect("Brannoc is seated");
        house.drop_hero(from, Some(Slot::Quest { quest: 0, seat: 1 }));
        assert_eq!(house.landing(ysolde, card), None);
        let swap = house.landing(ysolde, Some(crate::screen::Target::Hero(brannoc)));
        assert_eq!(swap, Some(Slot::Quest { quest: 0, seat: 1 }));
        assert_eq!(
            preview_party(&house, 0, Some((ysolde, swap))),
            [garrick, brannoc]
        );
        // A refuser is never previewed.
        house.reseat();
        house.heroes[ysolde].fear.broken = true;
        let land = house.landing(ysolde, card);
        assert_eq!(
            preview_party(&house, 0, Some((ysolde, land))),
            Vec::<HeroId>::new()
        );
    }
}
