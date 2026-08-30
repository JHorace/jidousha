//! The world that moves: parties, quests, the one scheduler, and the events
//! it emits (DESIGN §4, §5).
//!
//! **One scheduler, not per-system timers** — the invariant the whole phase
//! exists to build. Every occurrence (a tile entry, a work completion) has a
//! **world-time address** assigned when it is scheduled, independent of the
//! speed schedule; a tick that carries the clock across several world-minutes
//! fires everything due in the span, in world-time order (address, then
//! scheduling sequence). Same orders at the same world-times ⇒ identical
//! event sequence with identical world-time stamps, under any speed script —
//! the speed-invariance sweep in `sweep.rs` is that claim run every verify.
//!
//! Party positional state is discrete (DESIGN §3): a party is always on a
//! tile — resident, or following the route stored at dispatch, one scheduled
//! entry at a time, each at that terrain's cost. Smooth between-tile motion is
//! derived at draw time (`screens.rs`) and never written back here.
//!
//! Events carry **world-time + place + class** (DESIGN §5): S1 shows them as
//! a timestamped log, and S2 builds attention on exactly these addresses, so
//! the class and the tile ride every entry even though only the log reads
//! them yet.

use jidousha::prelude::*;

use crate::clock::{Clock, stamp};
use crate::constants::Tuning;
use crate::grid::{Grid, LOCATIONS, TOWN, Tile};
use crate::path::Route;
use crate::sprites::Art;

/// The five S1 event classes — the seed of S2's attention vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventClass {
    /// A party left the town for a site.
    Departed,
    /// A party reached its site.
    Arrived,
    /// Work began (same world-minute as the arrival, its own address).
    WorkBegan,
    /// The quest resolved — the stub success — and the pot paid.
    QuestComplete,
    /// The party is home.
    Returned,
}

impl EventClass {
    /// The class's name, for transcripts and the log.
    pub fn name(self) -> &'static str {
        match self {
            EventClass::Departed => "departed",
            EventClass::Arrived => "arrived",
            EventClass::WorkBegan => "work-began",
            EventClass::QuestComplete => "quest-complete",
            EventClass::Returned => "returned",
        }
    }
}

/// One thing that happened, at a world-time, at a place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    /// The world-minute it happened at.
    pub minute: u64,
    /// What kind of thing it was.
    pub class: EventClass,
    /// Which party.
    pub party: usize,
    /// The tile it happened on — every event has a place.
    pub tile: Tile,
    /// The named location standing on that tile, when one is.
    pub location: Option<usize>,
    /// The mechanical narration after the address.
    pub note: String,
}

impl Event {
    /// The log line: timestamp, party, note — mechanical narration.
    pub fn line(&self, sim: &Sim) -> String {
        format!(
            "{} - {} {}",
            stamp(self.minute),
            sim.parties
                .get(self.party)
                .map_or("someone", |party| party.name),
            self.note
        )
    }
}

/// A job at a site: what it pays, and how long it holds a party.
#[derive(Clone, Copy, Debug)]
pub struct Quest {
    /// Its name, for the log.
    pub name: &'static str,
    /// What it pays into the treasury on completion.
    pub pot: i64,
    /// How long the work takes, in world-minutes.
    pub duration: i64,
}

/// One quest site's state: its authored quests, in order, and how many have
/// been claimed. A quest is claimed at dispatch — two parties cannot take the
/// same one — and the site runs dry when the list is spent (S1 keeps the
/// simpler choice; DESIGN §10 leaves it open).
#[derive(Clone, Debug)]
pub struct Site {
    /// Which location this site is.
    pub location: usize,
    /// The authored quests, front first.
    pub quests: Vec<Quest>,
    /// How many have been claimed by a dispatch.
    pub claimed: usize,
}

impl Site {
    /// The next open quest, if the site is not dry.
    pub fn open(&self) -> Option<&Quest> {
        self.quests.get(self.claimed)
    }
}

/// What a party is doing. The positional state is discrete: `tile` on
/// [`Party`] is always the tile it is on, and travel is a stored route plus
/// an index — the next tile to enter — with the entry's world-time held by
/// the scheduler, echoed here for the draw-time interpolation.
#[derive(Clone, Debug, PartialEq)]
pub enum Activity {
    /// Resident in the town, dispatchable.
    Idle,
    /// Following `route` toward the quest site.
    Outbound {
        /// The route computed at dispatch.
        route: Route,
        /// Index into `route.tiles` of the next tile to enter.
        index: usize,
        /// The world-minute the current tile was entered.
        entered_at: u64,
        /// The world-minute the next tile is entered.
        next_at: u64,
    },
    /// Working the quest at the site.
    Working {
        /// The world-minute the work completes.
        until: u64,
    },
    /// Following the stored route home.
    Homebound {
        /// The home route, also computed at dispatch — terrain is static.
        route: Route,
        /// Index of the next tile to enter.
        index: usize,
        /// The world-minute the current tile was entered.
        entered_at: u64,
        /// The world-minute the next tile is entered.
        next_at: u64,
    },
}

/// One party.
#[derive(Clone, Debug)]
pub struct Party {
    /// Its name, as the log and the strip print it.
    pub name: &'static str,
    /// Its token's portrait.
    pub token: Art,
    /// The tile it is on — always exactly one.
    pub tile: Tile,
    /// What it is doing.
    pub activity: Activity,
    /// The quest it holds, as (site index, quest index).
    pub quest: Option<(usize, usize)>,
    /// The route home, computed at dispatch and held for the return leg.
    pub home: Option<Route>,
}

impl Party {
    /// The strip's one-line status. Kept short on purpose: the chip is 286
    /// reference pixels wide and the floors hold every row inside it.
    pub fn status(&self) -> String {
        let place = |location: usize| {
            LOCATIONS
                .get(location)
                .map_or("somewhere", |spec| spec.name)
        };
        match (&self.activity, self.quest) {
            (Activity::Idle, _) => format!("idle in {}", place(TOWN)),
            (Activity::Outbound { .. }, Some((site, _))) => {
                format!("-> {}", place(site_location(site)))
            }
            (Activity::Outbound { .. }, None) => "-> somewhere".to_owned(),
            (Activity::Working { .. }, Some((site, _))) => {
                format!("working {}", place(site_location(site)))
            }
            (Activity::Working { .. }, None) => "working".to_owned(),
            (Activity::Homebound { .. }, _) => format!("<- {}", place(TOWN)),
        }
    }
}

/// One scheduled occurrence: a world-time address, a tie-breaking sequence
/// number assigned at scheduling, and what falls due.
#[derive(Clone, Debug)]
struct Occurrence {
    at: u64,
    seq: u64,
    kind: Occ,
}

/// What an occurrence is.
#[derive(Clone, Copy, Debug)]
enum Occ {
    /// A party enters the next tile of its stored route.
    TileEntry {
        /// Which party.
        party: usize,
    },
    /// A party's work completes.
    WorkDone {
        /// Which party.
        party: usize,
    },
}

/// The moving world, held as one resource.
#[derive(Clone, Debug, Default)]
pub struct Sim {
    /// Every party, in authored order.
    pub parties: Vec<Party>,
    /// Every quest site, in `LOCATIONS` order (sans the town).
    pub sites: Vec<Site>,
    /// Pots accumulate here. No spending in S1 (DESIGN §5).
    pub treasury: i64,
    /// Everything that has happened, in firing order — the log's source and
    /// the sweep's transcript.
    pub events: Vec<Event>,
    queue: Vec<Occurrence>,
    next_seq: u64,
}

impl Resource for Sim {}

/// The `LOCATIONS` index a site index names (sites skip the town).
pub fn site_location(site: usize) -> usize {
    site + 1
}

/// The authored parties: at least two, because simultaneity is the point
/// (DESIGN §10) — this scenario fields three.
fn authored_parties() -> Vec<Party> {
    let home = LOCATIONS[TOWN].tile;
    [
        ("OX", Art::PortraitBob),
        ("OWL", Art::PortraitAlex),
        ("CRANE", Art::PortraitTim),
    ]
    .into_iter()
    .map(|(name, token)| Party {
        name,
        token,
        tile: home,
        activity: Activity::Idle,
        quest: None,
        home: None,
    })
    .collect()
}

/// The authored quests, one site per non-town location.
fn authored_sites() -> Vec<Site> {
    let quests: [(&str, Vec<Quest>); 4] = [
        (
            "watchtower",
            vec![
                Quest {
                    name: "the beacon watch",
                    pot: 60,
                    duration: 120,
                },
                Quest {
                    name: "the second watch",
                    pot: 50,
                    duration: 120,
                },
            ],
        ),
        (
            "deep-cave",
            vec![
                Quest {
                    name: "the mushroom haul",
                    pot: 40,
                    duration: 90,
                },
                Quest {
                    name: "the deep survey",
                    pot: 55,
                    duration: 150,
                },
            ],
        ),
        (
            "old-crypt",
            vec![
                Quest {
                    name: "the crypt seal",
                    pot: 45,
                    duration: 100,
                },
                Quest {
                    name: "the second seal",
                    pot: 45,
                    duration: 100,
                },
            ],
        ),
        (
            "black-vault",
            vec![Quest {
                name: "the vault ledger",
                pot: 80,
                duration: 180,
            }],
        ),
    ];
    quests
        .into_iter()
        .enumerate()
        .map(|(index, (id, quests))| {
            // The site table and LOCATIONS stay one list: the id here is a
            // cross-check, asserted at build time rather than trusted.
            let location = site_location(index);
            assert_eq!(
                LOCATIONS[location].id, id,
                "sim::authored_sites and grid::LOCATIONS disagree about site order"
            );
            Site {
                location,
                quests,
                claimed: 0,
            }
        })
        .collect()
}

impl Sim {
    /// The scenario at its opening state.
    pub fn opening() -> Self {
        Self {
            parties: authored_parties(),
            sites: authored_sites(),
            treasury: 0,
            events: Vec::new(),
            queue: Vec::new(),
            next_seq: 0,
        }
    }

    /// Whether everything is home and nothing is scheduled — the world at
    /// rest (the sweep's stopping condition).
    pub fn at_rest(&self) -> bool {
        self.queue.is_empty()
            && self
                .parties
                .iter()
                .all(|party| party.activity == Activity::Idle)
    }

    /// Put one occurrence on the queue.
    fn schedule(&mut self, at: u64, kind: Occ) {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.queue.push(Occurrence { at, seq, kind });
    }

    /// Record one event.
    fn emit(&mut self, minute: u64, class: EventClass, party: usize, tile: Tile, note: String) {
        self.events.push(Event {
            minute,
            class,
            party,
            tile,
            location: crate::grid::location_at(tile),
            note,
        });
    }
}

/// Why a dispatch was refused — surfaced as a toast, never silent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The party is not idle in town.
    NotIdle,
    /// The site has no open quest.
    Dry,
    /// No passable route exists (authoring fault; said loudly anyway).
    Unreachable,
}

impl Refusal {
    /// The toast's sentence.
    pub fn message(&self, party: &str, site: &str) -> String {
        match self {
            Refusal::NotIdle => format!("{party} is out - only an idle party can be sent"),
            Refusal::Dry => format!("{site} has no open quest left"),
            Refusal::Unreachable => {
                format!("no route reaches {site} - the map should not allow this")
            }
        }
    }
}

/// Dispatch: the entire order vocabulary (DESIGN §5).
///
/// The route out **and** the route home are computed here, once, and stored —
/// terrain is static in S1. The departure event and the first tile entry are
/// addressed from `now`, the world-minute the order was given; paused or not,
/// the address is the clock's.
pub fn dispatch(
    sim: &mut Sim,
    grid: &Grid,
    tuning: &Tuning,
    now: u64,
    party_index: usize,
    site_index: usize,
) -> Result<(), Refusal> {
    let town = LOCATIONS[TOWN].tile;
    let Some(site) = sim.sites.get(site_index) else {
        return Err(Refusal::Dry);
    };
    let Some(quest) = site.open().copied() else {
        return Err(Refusal::Dry);
    };
    let goal = LOCATIONS[site.location].tile;
    {
        let Some(party) = sim.parties.get(party_index) else {
            return Err(Refusal::NotIdle);
        };
        if party.activity != Activity::Idle {
            return Err(Refusal::NotIdle);
        }
    }
    let Some(out) = crate::path::route(grid, tuning, town, goal) else {
        return Err(Refusal::Unreachable);
    };
    let Some(home) = crate::path::route(grid, tuning, goal, town) else {
        return Err(Refusal::Unreachable);
    };
    let Some(first) = out.tiles.first().copied() else {
        return Err(Refusal::Unreachable);
    };
    let first_cost = grid.get(first).cost(tuning).unwrap_or(0);

    sim.sites[site_index].claimed += 1;
    let site_name = LOCATIONS[site_location(site_index)].name;
    // Kept under eighty characters: a log row is one row, and the drawer is
    // ninety-nine characters wide at the text floor.
    let note = format!(
        "departed for {} - {} tiles, {} min ({}, {}g)",
        site_name,
        out.tiles.len(),
        out.cost,
        quest.name,
        quest.pot,
    );
    let next_at = now + u64::try_from(first_cost).unwrap_or(0);
    {
        let party = &mut sim.parties[party_index];
        party.quest = Some((site_index, sim.sites[site_index].claimed - 1));
        party.home = Some(home);
        party.activity = Activity::Outbound {
            route: out,
            index: 0,
            entered_at: now,
            next_at,
        };
    }
    sim.emit(now, EventClass::Departed, party_index, town, note);
    sim.schedule(next_at, Occ::TileEntry { party: party_index });
    Ok(())
}

/// Fire everything due at or before this world-minute, in world-time order.
///
/// The system behind the scheduling invariant (DESIGN §4): occurrences fire
/// by (address, scheduling sequence), never by party order or queue position,
/// and a handler that schedules a follow-up due in the same span sees it
/// fired in the same call — the loop pops one minimum at a time.
pub fn fire_due(world: &mut World) {
    let now = world.resource::<Clock>().minutes;
    let tuning = *world.resource::<Tuning>();
    // The grid is read, never written: cloning a 48x27 byte table per firing
    // span would also be fine, but a split borrow through two resources is not
    // available, so take the cheap copy only when something is actually due.
    loop {
        let due = {
            let sim = world.resource::<Sim>();
            let mut best: Option<(usize, u64, u64)> = None;
            for (slot, occurrence) in sim.queue.iter().enumerate() {
                if occurrence.at > now {
                    continue;
                }
                let better =
                    best.is_none_or(|(_, at, seq)| (occurrence.at, occurrence.seq) < (at, seq));
                if better {
                    best = Some((slot, occurrence.at, occurrence.seq));
                }
            }
            best.map(|(slot, _, _)| slot)
        };
        let Some(slot) = due else { return };
        let grid = world.resource::<Grid>().clone();
        let sim = world.resource_mut::<Sim>();
        let occurrence = sim.queue.remove(slot);
        match occurrence.kind {
            Occ::TileEntry { party } => tile_entry(sim, &grid, &tuning, occurrence.at, party),
            Occ::WorkDone { party } => work_done(sim, &grid, &tuning, occurrence.at, party),
        }
    }
}

/// A party enters the next tile of its stored route.
///
/// Read pass then write pass: what the party is doing is copied out first,
/// because emitting an event and scheduling a follow-up both borrow the whole
/// `Sim` the party lives in.
fn tile_entry(sim: &mut Sim, grid: &Grid, tuning: &Tuning, at: u64, index: usize) {
    let (route, step, homebound, quest) = {
        let Some(party) = sim.parties.get(index) else {
            return;
        };
        match &party.activity {
            Activity::Outbound {
                route, index: step, ..
            } => (route.clone(), *step, false, party.quest),
            Activity::Homebound {
                route, index: step, ..
            } => (route.clone(), *step, true, party.quest),
            // A stray occurrence for a party no longer travelling is a
            // scheduler fault; S1 has no path that produces one, and a silent
            // skip would hide it — but a panic in release is forbidden, so it
            // is recorded where a check will read it.
            _ => {
                let tile = party.tile;
                sim.emit(
                    at,
                    EventClass::Arrived,
                    index,
                    tile,
                    "was scheduled to move while not travelling - a scheduler fault".to_owned(),
                );
                return;
            }
        }
    };
    let Some(tile) = route.tiles.get(step).copied() else {
        return;
    };
    sim.parties[index].tile = tile;
    let next_step = step + 1;
    if let Some(next) = route.tiles.get(next_step).copied() {
        // Still on the road: schedule the next entry at this terrain's cost.
        let cost = grid.get(next).cost(tuning).unwrap_or(0);
        let next_at = at + u64::try_from(cost).unwrap_or(0);
        match &mut sim.parties[index].activity {
            Activity::Outbound {
                index: step,
                entered_at,
                next_at: slot,
                ..
            }
            | Activity::Homebound {
                index: step,
                entered_at,
                next_at: slot,
                ..
            } => {
                *step = next_step;
                *entered_at = at;
                *slot = next_at;
            }
            _ => {}
        }
        sim.schedule(next_at, Occ::TileEntry { party: index });
        return;
    }
    // Journey's end.
    if homebound {
        let party = &mut sim.parties[index];
        party.activity = Activity::Idle;
        party.quest = None;
        party.home = None;
        sim.emit(
            at,
            EventClass::Returned,
            index,
            tile,
            format!("returned to {}", LOCATIONS[TOWN].name),
        );
        return;
    }
    let quest_data = quest.and_then(|(site, slot)| {
        sim.sites
            .get(site)
            .and_then(|site| site.quests.get(slot))
            .copied()
    });
    let Some(quest_data) = quest_data else { return };
    let until = at + u64::try_from(quest_data.duration).unwrap_or(0);
    sim.parties[index].activity = Activity::Working { until };
    let site_name = quest
        .map(|(site, _)| LOCATIONS[site_location(site)].name)
        .unwrap_or("the site");
    sim.emit(
        at,
        EventClass::Arrived,
        index,
        tile,
        format!("arrived at {site_name}"),
    );
    sim.emit(
        at,
        EventClass::WorkBegan,
        index,
        tile,
        format!(
            "began {} - {} minutes of work",
            quest_data.name, quest_data.duration
        ),
    );
    sim.schedule(until, Occ::WorkDone { party: index });
}

/// A party's work completes: the stub success — the pot pays, and the party
/// turns for home on the route stored at dispatch.
fn work_done(sim: &mut Sim, grid: &Grid, tuning: &Tuning, at: u64, index: usize) {
    let (quest, home, tile) = {
        let Some(party) = sim.parties.get(index) else {
            return;
        };
        let quest = party.quest.and_then(|(site, slot)| {
            sim.sites
                .get(site)
                .and_then(|site| site.quests.get(slot))
                .copied()
        });
        (quest, party.home.clone(), party.tile)
    };
    let Some(quest) = quest else { return };
    let Some(home) = home else { return };
    let Some(first) = home.tiles.first().copied() else {
        return;
    };
    let cost = grid.get(first).cost(tuning).unwrap_or(0);
    let next_at = at + u64::try_from(cost).unwrap_or(0);
    sim.parties[index].activity = Activity::Homebound {
        route: home,
        index: 0,
        entered_at: at,
        next_at,
    };
    sim.treasury += quest.pot;
    let treasury = sim.treasury;
    sim.emit(
        at,
        EventClass::QuestComplete,
        index,
        tile,
        format!(
            "completed {} - {}g into the treasury ({}g held) - turning for home",
            quest.name, quest.pot, treasury
        ),
    );
    sim.schedule(next_at, Occ::TileEntry { party: index });
}
