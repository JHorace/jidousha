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
use crate::hearth::Hearth;
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
    /// The winter's seats (SPEC §3.1 `hearth`); empty all summer.
    pub hearth: Hearth,
    /// This turning's pages, from the winter's passing until summer comes (SPEC §3.1
    /// `passage`).
    pub passage: Option<crate::passage::Passage>,
    /// Per place: visits, triumphs, disasters, trouble.
    pub places: Vec<PlaceRecord>,
    /// Ghosts of dreams left to no one, in list order (SPEC §14.4).
    pub ghosts: Vec<Ghost>,
    /// Per place, the template its quest had on the last board (SPEC §5.2).
    pub templates_last: Vec<Option<usize>>,
    /// The name and house bags newcomers are drawn from (SPEC §3.1 `generation`, §17.1).
    pub bags: crate::newcomers::Bags,
    /// What this summer's board generation decided, for the checks.
    pub board_report: Option<BoardReport>,
    /// This summer's telling, from set out until the player leaves it (SPEC §3.1 `tale`).
    pub telling: Option<Telling>,
    /// Heroes who died since the last turning and await their death page (SPEC §3.1).
    pub mourned: Vec<HeroId>,
    /// The house has closed: renown was spent when the telling was left (SPEC §2.1).
    pub closed: bool,
    /// The Ending, once entered (SPEC §23): after the Door, or once the house closed.
    pub ending: Option<crate::ending::Ending>,
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
    /// Found a house on `seed` and prepare its first summer, drawing from `rng` (the run's
    /// generator) in SPEC §22.2's order: the two dead founders' epitaph wordings (Elsbeth
    /// first, as `dead_at_start` lists them, each composed as it is rolled), then the
    /// board's rolls.
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
            hearth: Hearth::default(),
            passage: None,
            places: vec![PlaceRecord::default(); Place::ALL.len()],
            ghosts: Vec::new(),
            templates_last: vec![None; Place::ALL.len()],
            bags: crate::newcomers::Bags::new(content),
            board_report: None,
            telling: None,
            mourned: Vec::new(),
            closed: false,
            ending: None,
        };
        for key in &content.founding.dead_at_start {
            let Some(id) = house.heroes.iter().position(|hero| hero.key == *key) else {
                return Err(format!("dead_at_start names {key:?}, who was not founded"));
            };
            crate::epitaph::remember(content, &mut house.heroes, &mut house.writing, id, rng);
        }
        house.prepare_summer(content, rng);
        Ok(house)
    }

    /// Prepare a summer (SPEC §5.1): post the board, then reseat the household.
    pub fn prepare_summer(&mut self, content: &Content, rng: &mut Rng) {
        self.post_board(content, rng);
        self.reseat();
    }

    /// Reseat: clear the roster, every quest seat and every hearth seat, push every
    /// living adult into the first free roster seat in creation order (SPEC §5.1).
    pub fn reseat(&mut self) {
        self.roster = [None; ROSTER_SEATS];
        self.hearth.clear();
        for posted in &mut self.board {
            posted.seats.iter_mut().for_each(|seat| *seat = None);
            // The variant's (DESIGN decision 10): reseating withdraws every oath.
            posted.sworn = None;
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

    /// Take `hero` out of every seat: the roster, every quest's and the hearth's
    /// (SPEC §7.4, the dead and the crowned are "removed from every seat").
    pub fn unseat(&mut self, hero: HeroId) {
        if let Some(seat) = self.hearth.seat_of(hero) {
            self.hearth.put(seat, None);
        }
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

/// One house of the run, as the chronicle records it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Founded {
    /// The seed it was founded on.
    pub seed: u64,
    /// How it ended — the verdict's title and the year — once it has.
    pub ended: Option<(String, i32)>,
}

/// The run's transcript of houses: every house founded in this process, in order, each
/// with its seed and, once another is begun, how it ended. "Begin another house" writes
/// it; nothing else does, and a replay of the same presses on the same seed writes the
/// same chronicle (SPEC §22.1: a new house reseeds from a draw of the generator).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Chronicle(pub Vec<Founded>);

impl Resource for Chronicle {}

/// Begin another house (SPEC §2.1, §22.1, `scene/scenes/ending.jai:26-27,38-40`): the
/// ended house's verdict goes into the chronicle, a new seed is drawn from the current
/// generator, the generator is reseeded from it, and the authored household is founded
/// again — the whole game state reset, in process. Nothing reads a clock.
pub fn begin_another_house(world: &mut World) -> Result<u64, String> {
    let ended = {
        let content = world.resource::<Content>();
        let house = world.resource::<House>();
        house.ending.as_ref().map(|ending| {
            (
                ending.title(content).to_owned(),
                house.calendar.current_year(),
            )
        })
    };
    let old_seed = world.resource::<House>().seed;
    let seed = draw_seed(world.resource_mut::<Rng>());
    let mut rng = Rng::from_seed(seed);
    let house = House::found(world.resource::<Content>(), seed, &mut rng)?;
    world.insert_resource(rng);
    world.insert_resource(house);
    let mut chronicle = world
        .find_resource::<Chronicle>()
        .cloned()
        .unwrap_or_default();
    match chronicle.0.last_mut() {
        Some(last) if last.seed == old_seed => last.ended = ended,
        _ => chronicle.0.push(Founded {
            seed: old_seed,
            ended,
        }),
    }
    chronicle.0.push(Founded { seed, ended: None });
    world.insert_resource(chronicle);
    Ok(seed)
}
