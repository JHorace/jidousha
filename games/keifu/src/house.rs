//! The house (SPEC §3.1): its run seed, calendar, heroes, roster and renown.
//!
//! One `House` resource holds the whole run. It is founded from the content, the
//! summer is prepared (reseating the roster, §5.1), and "begin another house" is
//! an in-process reset: a new seed drawn from the current generator, recorded,
//! and the household founded again.

use jidousha::prelude::*;

use crate::board::{PlaceRecord, Posted};
use crate::calendar::Calendar;
use crate::constants::{HOUSE_RENOWN_AT_START, ROSTER_SEATS, YARD_SPOTS};
use crate::content::Content;
use crate::generation::BoardReport;
use crate::ghost::Ghost;
use crate::hero::{Fate, Hero, HeroId};
use crate::household::found;
use crate::ids::Place;
use crate::telling::Telling;
use crate::text::WritingMemory;

/// The whole run's state (SPEC §3.1), as far as W1 builds it.
#[derive(Clone)]
pub struct House {
    /// The seed this run's generator was made from — explicit, recorded state.
    pub seed: u64,
    /// Year and season.
    pub calendar: Calendar,
    /// Every hero ever created, in creation order.
    pub heroes: Vec<Hero>,
    /// Twelve seats: the household in summer.
    pub roster: [Option<HeroId>; ROSTER_SEATS],
    /// House renown, floored at 0.
    pub renown: i32,
    /// Heroes ever crowned; each adds +1 to every quest (SPEC §6).
    pub patrons: i32,
    /// Per place, the heroes who fell there.
    pub fallen: Vec<Vec<HeroId>>,
    /// Per-pool memory of the last line written.
    pub writing: WritingMemory,
    /// House tales, oldest first.
    pub tales: Vec<Tale>,
    /// Blades forged this run; the next takes blade name `blades_named mod 10`.
    pub blades_named: usize,
    /// This summer's posted quests, in place order (board slots 0..).
    pub board: Vec<Posted>,
    /// Per place: visits, triumphs, disasters, trouble.
    pub places: Vec<PlaceRecord>,
    /// Ghosts of dreams left to no one, in list order (SPEC §14.4).
    pub ghosts: Vec<Ghost>,
    /// Per place, the template its quest had on the last board (SPEC §5.2).
    pub templates_last: Vec<Option<usize>>,
    /// What this summer's board generation decided, for the checks.
    pub board_report: Option<BoardReport>,
    /// This summer's telling, from set out until the player leaves it (SPEC §3.1 `tale`).
    pub telling: Option<Telling>,
    /// Heroes who died since the last turning and await their death page (SPEC §3.1).
    pub mourned: Vec<HeroId>,
    /// The house has closed: renown was spent when the telling was left (SPEC §2.1).
    pub closed: bool,
}

/// A house tale (SPEC §3.1): its title, whom it is about, and the year it was first told.
#[derive(Clone, Debug, PartialEq)]
pub struct Tale {
    /// "The tale of Pip and the sea".
    pub title: String,
    /// The dreamer it is about.
    pub about: HeroId,
    /// The year it was left.
    pub since: i32,
}

impl Resource for House {}

/// The run's seed as a check or a command line fixes it, read once at founding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunSeed(pub u64);

impl Resource for RunSeed {}

/// A seed drawn from a generator: two 32-bit draws (SPEC §22.1, "a new house
/// reseeds from a draw of the previous run's generator").
pub fn draw_seed(rng: &mut Rng) -> u64 {
    (u64::from(rng.next_u32()) << 32) | u64::from(rng.next_u32())
}

impl House {
    /// Found a house on `seed` and prepare its first summer, drawing the board's
    /// rolls from `rng` (the run's generator).
    pub fn found(content: &Content, seed: u64, rng: &mut Rng) -> Result<Self, String> {
        let founded = found(content)?;
        let mut house = Self {
            seed,
            calendar: Calendar::start(),
            heroes: founded.heroes,
            roster: [None; ROSTER_SEATS],
            renown: HOUSE_RENOWN_AT_START,
            patrons: 0,
            fallen: founded.fallen,
            writing: WritingMemory::default(),
            tales: Vec::new(),
            blades_named: 0,
            board: Vec::new(),
            places: vec![PlaceRecord::default(); Place::ALL.len()],
            ghosts: Vec::new(),
            templates_last: vec![None; Place::ALL.len()],
            board_report: None,
            telling: None,
            mourned: Vec::new(),
            closed: false,
        };
        house.prepare_summer(content, rng);
        Ok(house)
    }

    /// Prepare a summer (SPEC §5.1): post the board, then reseat the household.
    pub fn prepare_summer(&mut self, content: &Content, rng: &mut Rng) {
        self.post_board(content, rng);
        self.reseat();
    }

    /// Reseat: clear the roster and every quest seat, push every living adult into
    /// the first free roster seat in creation order (SPEC §5.1).
    pub fn reseat(&mut self) {
        self.roster = [None; ROSTER_SEATS];
        for posted in &mut self.board {
            posted.seats.iter_mut().for_each(|seat| *seat = None);
        }
        let adults: Vec<HeroId> = (0..self.heroes.len())
            .filter(|&id| self.heroes[id].is_living() && self.heroes[id].is_adult())
            .collect();
        for id in adults {
            if let Some(seat) = self.roster.iter_mut().find(|seat| seat.is_none()) {
                *seat = Some(id);
            }
        }
    }

    /// The yard: living children, the first six in creation order.
    pub fn yard(&self) -> Vec<HeroId> {
        (0..self.heroes.len())
            .filter(|&id| self.heroes[id].is_living() && !self.heroes[id].is_adult())
            .take(YARD_SPOTS)
            .collect()
    }

    /// Living and gone (dead or departed), for the family tally.
    pub fn living_and_gone(&self) -> (usize, usize) {
        let living = self
            .heroes
            .iter()
            .filter(|h| h.fate == Fate::Living)
            .count();
        (living, self.heroes.len() - living)
    }

    /// Change house renown by `delta`, floored at 0 (CONSTANTS §1: "Renown is floored at
    /// 0 by every change").
    pub fn add_renown(&mut self, delta: i32) {
        self.renown = (self.renown + delta).max(0);
    }

    /// Take `hero` out of every seat: the roster and every quest's (SPEC §7.4, the
    /// dead and the crowned are "removed from every seat").
    pub fn unseat(&mut self, hero: HeroId) {
        for seat in self.roster.iter_mut().filter(|seat| **seat == Some(hero)) {
            *seat = None;
        }
        for posted in &mut self.board {
            for seat in posted.seats.iter_mut().filter(|seat| **seat == Some(hero)) {
                *seat = None;
            }
        }
    }

    /// The heroes who fell at `place`.
    pub fn fallen_at(&self, place: Place) -> &[HeroId] {
        &self.fallen[place.index()]
    }
}

/// Begin another house: draw a new seed from the current generator, reseed it,
/// and found the household again (SPEC §2.1, §22.1). Nothing reads a clock.
pub fn begin_another_house(world: &mut World) -> Result<u64, String> {
    let seed = draw_seed(world.resource_mut::<Rng>());
    let mut rng = Rng::from_seed(seed);
    let house = House::found(world.resource::<Content>(), seed, &mut rng)?;
    world.insert_resource(rng);
    world.insert_resource(house);
    Ok(seed)
}
