//! The house (SPEC §3.1): its run seed, calendar, heroes, roster and renown.
//!
//! One `House` resource holds the whole run. It is founded from the content, the
//! summer is prepared (reseating the roster, §5.1), and "begin another house" is
//! an in-process reset: a new seed drawn from the current generator, recorded,
//! and the household founded again.

use jidousha::prelude::*;

use crate::calendar::Calendar;
use crate::constants::{HOUSE_RENOWN_AT_START, ROSTER_SEATS, YARD_SPOTS};
use crate::content::Content;
use crate::hero::{Fate, Hero, HeroId};
use crate::household::found;
use crate::ids::Place;
use crate::text::WritingMemory;

/// The whole run's state (SPEC §3.1), as far as W1 builds it.
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
    /// Per place, the heroes who fell there.
    pub fallen: Vec<Vec<HeroId>>,
    /// Per-pool memory of the last line written.
    pub writing: WritingMemory,
    /// House tales, by title.
    pub tales: Vec<String>,
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
    /// Found a house on `seed` and prepare its first summer.
    pub fn found(content: &Content, seed: u64) -> Result<Self, String> {
        let founded = found(content)?;
        let mut house = Self {
            seed,
            calendar: Calendar::start(),
            heroes: founded.heroes,
            roster: [None; ROSTER_SEATS],
            renown: HOUSE_RENOWN_AT_START,
            fallen: founded.fallen,
            writing: WritingMemory::default(),
            tales: Vec::new(),
        };
        house.prepare_summer();
        Ok(house)
    }

    /// Prepare a summer (SPEC §5.1). The board is W5; this wave reseats the roster.
    pub fn prepare_summer(&mut self) {
        self.reseat();
    }

    /// Reseat: clear the roster, push every living adult into the first free seat in
    /// creation order (SPEC §5.1).
    pub fn reseat(&mut self) {
        self.roster = [None; ROSTER_SEATS];
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

    /// The heroes who fell at `place`.
    pub fn fallen_at(&self, place: Place) -> &[HeroId] {
        &self.fallen[place.index()]
    }
}

/// Begin another house: draw a new seed from the current generator, reseed it,
/// and found the household again (SPEC §2.1, §22.1). Nothing reads a clock.
pub fn begin_another_house(world: &mut World, content: &Content) -> Result<u64, String> {
    let seed = draw_seed(world.resource_mut::<Rng>());
    world.insert_resource(Rng::from_seed(seed));
    let house = House::found(content, seed)?;
    world.insert_resource(house);
    Ok(seed)
}
