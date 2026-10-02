//! The board and its seats (SPEC §5.1, §5.3): the posted quests, the slots heroes
//! are dragged between, and the rules a drop obeys.
//!
//! Seating is slot-to-slot: the roster's twelve and each posted quest's `seats`.
//! A drop onto an empty slot moves the hero; onto an occupied one swaps (the
//! displaced hero goes where the dragged one came from); anywhere else returns
//! them. After every drop, anyone sitting on a quest they refuse goes back to the
//! first free roster seat. The Update system calls `drop_hero`; nothing else moves
//! a hero between seats.

use jidousha::prelude::Rng;

use crate::content::Content;
use crate::fear::refuses;
use crate::generation::generate;
use crate::hero::HeroId;
use crate::house::House;
use crate::quest::Quest;
use crate::screen::Target;

/// A posted quest and who sits on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Posted {
    /// The quest.
    pub quest: Quest,
    /// Exactly `quest.seats` slots.
    pub seats: Vec<Option<HeroId>>,
}

/// Where a hero can sit in summer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// One of the household's twelve seats.
    Roster(usize),
    /// Seat `seat` of board slot `quest`.
    Quest {
        /// The board slot.
        quest: usize,
        /// The seat on it.
        seat: usize,
    },
}

/// A place's record (SPEC §3.1 `places[7]`, less the fallen, which `House::fallen` holds).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlaceRecord {
    /// Quests answered there.
    pub visits: i32,
    /// Of which triumphs.
    pub triumphs: i32,
    /// Of which disasters.
    pub disasters: i32,
    /// Trouble, 0..2.
    pub trouble: i32,
}

impl House {
    /// Post the summer's board (SPEC §5.2, `generation.rs`): the kept, eased board
    /// in place order, each quest with its seats empty; the per-place template memory
    /// and what generation decided are kept on the house.
    pub fn post_board(&mut self, content: &Content, rng: &mut Rng) {
        let (quests, memory, report) = generate(content, self, rng);
        self.templates_last = memory;
        self.board = quests
            .into_iter()
            .map(|quest| Posted {
                seats: vec![None; quest.seats as usize],
                quest,
            })
            .collect();
        self.board_report = Some(report);
    }

    /// Who sits in `slot`.
    pub fn hero_in(&self, slot: Slot) -> Option<HeroId> {
        match slot {
            Slot::Roster(seat) => self.roster[seat],
            Slot::Quest { quest, seat } => self.board[quest].seats[seat],
        }
    }

    /// Where `hero` sits, if anywhere (the yard is not a slot).
    pub fn slot_of(&self, hero: HeroId) -> Option<Slot> {
        if let Some(seat) = self.roster.iter().position(|s| *s == Some(hero)) {
            return Some(Slot::Roster(seat));
        }
        self.board.iter().enumerate().find_map(|(quest, posted)| {
            posted
                .seats
                .iter()
                .position(|s| *s == Some(hero))
                .map(|seat| Slot::Quest { quest, seat })
        })
    }

    /// The party on board slot `quest`: seated heroes in seat order.
    pub fn party(&self, quest: usize) -> Vec<HeroId> {
        self.board[quest].seats.iter().flatten().copied().collect()
    }

    fn put(&mut self, slot: Slot, hero: Option<HeroId>) {
        match slot {
            Slot::Roster(seat) => self.roster[seat] = hero,
            Slot::Quest { quest, seat } => self.board[quest].seats[seat] = hero,
        }
    }

    /// Release the hero dragged from `from` over `onto` (SPEC §5.3): onto an empty
    /// slot moves them, onto an occupied one swaps, onto nothing returns them. Then
    /// anyone on a quest they refuse goes back to the roster. Returns who was sent back.
    pub fn drop_hero(&mut self, from: Slot, onto: Option<Slot>) -> Vec<HeroId> {
        let Some(hero) = self.hero_in(from) else {
            panic!(
                "[keifu] a drag began on {from:?}, which is empty\n  likely cause: the \
                 board changed under a drag\n  fix: begin drags only on a hero's card"
            );
        };
        if let Some(onto) = onto.filter(|onto| *onto != from) {
            let displaced = self.hero_in(onto);
            self.put(onto, Some(hero));
            self.put(from, displaced);
        }
        self.send_back_refusers()
    }

    /// Everyone sitting on a quest they refuse goes to the first free roster seat
    /// (SPEC §5.3, `scene/scenes/summer.jai:153-170`): board order, then seat order.
    pub fn send_back_refusers(&mut self) -> Vec<HeroId> {
        let mut sent = Vec::new();
        for quest in 0..self.board.len() {
            for seat in 0..self.board[quest].seats.len() {
                let Some(hero) = self.board[quest].seats[seat] else {
                    continue;
                };
                if !refuses(&self.heroes[hero], &self.board[quest].quest.tags) {
                    continue;
                }
                let Some(free) = self.roster.iter().position(Option::is_none) else {
                    panic!(
                        "[keifu] {} refuses a quest and the roster has no free seat\n  \
                         likely cause: more than twelve living adults\n  fix: SPEC §17.3 \
                         caps the household at twelve",
                        self.heroes[hero].name
                    );
                };
                self.board[quest].seats[seat] = None;
                self.roster[free] = Some(hero);
                sent.push(hero);
            }
        }
        sent
    }

    /// Where `hero`, in hand, lands if released over `over` — the one answer the
    /// release and the card's preview both read (SPEC §5.3, §5.4). A seat lands on
    /// that seat (a swap, if someone sits there); a seated hero's card on their seat;
    /// a quest card's body on its first free seat, the hand's own seat counting as
    /// free (SPEC-GAPS KG-28); anything else nowhere, and the hero returns.
    pub fn landing(&self, hero: HeroId, over: Option<Target>) -> Option<Slot> {
        match over? {
            Target::Seat(slot) => Some(slot),
            Target::Hero(id) => self.slot_of(id),
            Target::Quest(quest) => self.board[quest]
                .seats
                .iter()
                .position(|seat| seat.is_none() || *seat == Some(hero))
                .map(|seat| Slot::Quest { quest, seat }),
            Target::OpenFamily | Target::CloseFamily | Target::Dock => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{house, id};

    fn seat(quest: usize, seat: usize) -> Slot {
        Slot::Quest { quest, seat }
    }

    #[test]
    fn year_one_posts_grave_goods_and_the_bell_first_of_four_with_empty_seats() {
        let (_, house) = house();
        let titles: Vec<&str> = house.board.iter().map(|p| p.quest.title.as_str()).collect();
        assert_eq!(titles.len(), 4);
        assert_eq!(titles[..2], ["Grave goods", "The bell under the tide"]);
        assert!(house.board.iter().all(
            |p| p.seats.len() == p.quest.seats as usize && p.seats.iter().all(Option::is_none)
        ));
    }

    #[test]
    fn a_drop_on_an_empty_seat_moves_on_an_occupied_one_swaps_and_elsewhere_returns() {
        let (_, mut house) = house();
        let (garrick, odo) = (id(&house.heroes, "Garrick"), id(&house.heroes, "Odo"));
        let from = house.slot_of(garrick).expect("Garrick is seated");
        house.drop_hero(from, Some(seat(0, 0)));
        assert_eq!(house.hero_in(seat(0, 0)), Some(garrick));
        assert_eq!(
            house.hero_in(from),
            None,
            "the roster seat he left is empty"
        );
        assert_eq!(house.party(0), [garrick]);
        // Odo onto Garrick's seat: Garrick goes to the roster seat Odo came from.
        let odo_from = house.slot_of(odo).expect("Odo is seated");
        house.drop_hero(odo_from, Some(seat(0, 0)));
        assert_eq!(house.party(0), [odo]);
        assert_eq!(house.hero_in(odo_from), Some(garrick));
        // Released over nothing, or over his own seat: nothing moves.
        let before = house.clone();
        house.drop_hero(seat(0, 0), None);
        house.drop_hero(seat(0, 0), Some(seat(0, 0)));
        assert_eq!(house.board, before.board);
        assert_eq!(house.roster, before.roster);
        // Seat order is the party's order.
        house.drop_hero(odo_from, Some(seat(0, 1)));
        assert_eq!(house.party(0), [odo, garrick]);
    }

    #[test]
    fn a_hero_dropped_on_a_quest_they_refuse_goes_back_to_the_first_free_roster_seat() {
        let (_, mut house) = house();
        let (ysolde, maren) = (id(&house.heroes, "Ysolde"), id(&house.heroes, "Maren"));
        house.heroes[ysolde].fear.broken = true; // Dark: Grave goods carries it.
        let from = house.slot_of(ysolde).expect("Ysolde is seated");
        let maren_from = house.slot_of(maren).expect("Maren is seated");
        house.drop_hero(maren_from, Some(seat(1, 0)));
        let sent = house.drop_hero(from, Some(seat(0, 0)));
        assert_eq!(sent, [ysolde]);
        assert_eq!(house.party(0), Vec::<HeroId>::new());
        let first_free = Slot::Roster(1);
        assert_eq!(maren_from, first_free, "Maren sat second and left");
        assert_eq!(house.slot_of(ysolde), Some(first_free));
    }

    #[test]
    fn reseating_empties_every_quest_seat() {
        let (_, mut house) = house();
        let garrick = id(&house.heroes, "Garrick");
        let from = house.slot_of(garrick).expect("Garrick is seated");
        house.drop_hero(from, Some(seat(1, 1)));
        house.reseat();
        assert!(
            house
                .board
                .iter()
                .all(|p| p.seats.iter().all(Option::is_none))
        );
        assert_eq!(house.slot_of(garrick), Some(Slot::Roster(0)));
    }
}
