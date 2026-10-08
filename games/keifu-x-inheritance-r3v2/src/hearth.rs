//! The hearth (SPEC §3.1 `hearth`, §11.1-§11.2): the winter's seats, and opening them.
//!
//! Twelve winter seats (SPEC-GAPS KG-46: the help calls them ten) — the fire's two, the
//! training yard's learner and teacher, the garden's two, the long table's two, and two
//! benches of a child and a teacher — and the yard's six, where the children wait.
//! With the roster (the hall, in winter) they are every slot a hero can be dragged
//! between in winter. **Any hero can be put in any seat** (§11.2); eligibility is
//! judged only when the winter resolves, by the plans in `plans.rs`, which the seat
//! previews read too.

use crate::constants::{BENCHES, FIRE_SEATS, GARDEN_SEATS, TALE_SEATS, YARD_SPOTS};
use crate::hero::HeroId;
use crate::house::House;

/// One seat of the hearth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seat {
    /// By the fire, `0..FIRE_SEATS`.
    Fire(usize),
    /// The training yard's learner.
    Learner,
    /// The training yard's teacher.
    Teacher,
    /// The garden, `0..GARDEN_SEATS`.
    Garden(usize),
    /// The long table, `0..TALE_SEATS`.
    Table(usize),
    /// Bench `b`'s child.
    BenchChild(usize),
    /// Bench `b`'s teacher.
    BenchTeacher(usize),
    /// The yard, `0..YARD_SPOTS`: children's holding seats.
    Yard(usize),
}

/// Every hearth seat: the twelve winter seats in resolution order, then the yard.
pub const SEATS: usize = FIRE_SEATS + 2 + GARDEN_SEATS + TALE_SEATS + 2 * BENCHES + YARD_SPOTS;

impl Seat {
    /// Every seat, in the order `index` numbers them.
    pub fn all() -> [Seat; SEATS] {
        let mut out = [Seat::Learner; SEATS];
        let mut at = 0;
        let mut push = |seat: Seat| {
            out[at] = seat;
            at += 1;
        };
        (0..FIRE_SEATS).for_each(|i| push(Seat::Fire(i)));
        push(Seat::Learner);
        push(Seat::Teacher);
        (0..GARDEN_SEATS).for_each(|i| push(Seat::Garden(i)));
        (0..TALE_SEATS).for_each(|i| push(Seat::Table(i)));
        for bench in 0..BENCHES {
            push(Seat::BenchChild(bench));
            push(Seat::BenchTeacher(bench));
        }
        (0..YARD_SPOTS).for_each(|i| push(Seat::Yard(i)));
        out
    }

    /// Where this seat is in `Hearth`'s array.
    pub fn index(self) -> usize {
        match Seat::all().iter().position(|seat| *seat == self) {
            Some(index) => index,
            None => panic!(
                "[keifu_x_inheritance_r3v2] {self:?} is not a hearth seat\n  likely cause: an index past \
                 CONSTANTS §9's seat counts\n  fix: name seats from Seat::all()"
            ),
        }
    }
}

/// A group of seats on the hearth screen, each with its help (`ui.winter.*_help`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// The hall: whoever is left there does nothing.
    Hall,
    /// By the fire.
    Fire,
    /// The training yard.
    Training,
    /// The garden.
    Garden,
    /// The long table.
    Table,
    /// The benches in the yard.
    Benches,
}

/// Who sits where at the hearth.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Hearth {
    seats: [Option<HeroId>; SEATS],
}

impl Hearth {
    /// Who sits in `seat`.
    pub fn at(&self, seat: Seat) -> Option<HeroId> {
        self.seats[seat.index()]
    }

    /// Put `hero` (or nobody) in `seat`.
    pub fn put(&mut self, seat: Seat, hero: Option<HeroId>) {
        self.seats[seat.index()] = hero;
    }

    /// Where `hero` sits at the hearth, if anywhere.
    pub fn seat_of(&self, hero: HeroId) -> Option<Seat> {
        Seat::all()
            .into_iter()
            .find(|seat| self.at(*seat) == Some(hero))
    }

    /// Empty every seat.
    pub fn clear(&mut self) {
        self.seats = [None; SEATS];
    }
}

impl House {
    /// Open the hearth (SPEC §11.1): reseat the household (the roster is the hall),
    /// put the first six living children in the yard, then move up to two wounded
    /// heroes, in roster seat order, from the hall to the fire.
    pub fn open_hearth(&mut self) {
        self.reseat();
        for (spot, child) in self.yard().into_iter().enumerate() {
            self.hearth.put(Seat::Yard(spot), Some(child));
        }
        let mut fire = 0;
        for seat in 0..self.roster.len() {
            let Some(hero) = self.roster[seat] else {
                continue;
            };
            if fire < FIRE_SEATS && self.heroes[hero].wounded {
                self.roster[seat] = None;
                self.hearth.put(Seat::Fire(fire), Some(hero));
                fire += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::Slot;
    use crate::testkit::{house, id};

    #[test]
    fn the_hearth_has_ten_winter_seats_and_six_in_the_yard_each_numbered_once() {
        let all = Seat::all();
        assert_eq!(all.len(), 16 + 2);
        for (index, seat) in all.iter().enumerate() {
            assert_eq!(seat.index(), index, "{seat:?}");
        }
        assert_eq!(
            all[..10],
            [
                Seat::Fire(0),
                Seat::Fire(1),
                Seat::Learner,
                Seat::Teacher,
                Seat::Garden(0),
                Seat::Garden(1),
                Seat::Table(0),
                Seat::Table(1),
                Seat::BenchChild(0),
                Seat::BenchTeacher(0),
            ]
        );
    }

    #[test]
    fn opening_the_hearth_seats_the_children_in_the_yard_and_the_first_two_wounded_by_the_fire() {
        let (_, mut house) = house();
        let names = ["Maren", "Ysolde", "Brannoc"];
        for name in names {
            let hero = id(&house.heroes, name);
            house.heroes[hero].wounded = true;
        }
        let garrick = id(&house.heroes, "Garrick");
        house.hearth.put(Seat::Table(0), Some(garrick));
        house.open_hearth();
        let at = |seat| house.hearth.at(seat);
        // Maren and Ysolde are the first wounded in roster order; Brannoc waits in the hall.
        assert_eq!(at(Seat::Fire(0)), Some(id(&house.heroes, "Maren")));
        assert_eq!(at(Seat::Fire(1)), Some(id(&house.heroes, "Ysolde")));
        let brannoc = id(&house.heroes, "Brannoc");
        assert!(matches!(house.slot_of(brannoc), Some(Slot::Roster(_))));
        assert_eq!(at(Seat::Yard(0)), Some(id(&house.heroes, "Pip")));
        assert_eq!(at(Seat::Yard(1)), Some(id(&house.heroes, "Wren")));
        assert_eq!(at(Seat::Yard(2)), None);
        // The rest of the hearth is empty, and the hall holds only who is left.
        assert_eq!(at(Seat::Table(0)), None);
        assert_eq!(house.slot_of(garrick), Some(Slot::Roster(0)));
        assert_eq!(house.roster.iter().flatten().count(), 3);
    }

    #[test]
    fn reseating_for_summer_empties_every_hearth_seat() {
        let (_, mut house) = house();
        house.open_hearth();
        house.reseat();
        assert!(
            Seat::all()
                .iter()
                .all(|seat| house.hearth.at(*seat).is_none())
        );
    }
}
