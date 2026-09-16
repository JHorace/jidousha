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
//! Events carry **world-time + place + class** (DESIGN §5), and wave 0a's
//! attention machinery is built on exactly those addresses: the class table in
//! `attention.rs` says what each class does to the player, the feed is a view
//! of `Sim::events`, and **an event whose configured mode is pause-and-focus
//! stops the clock from here** — a deterministic simulation transition in the
//! tick the event fires, not a click nobody made, so a replay pauses the same
//! way twice.

use jidousha::prelude::*;

use crate::asks::{Postings, Rates};
use crate::attention::{Attention, EventClass, Mode, Pause};
use crate::clock::{Clock, stamp};
use crate::constants::Tuning;
use crate::grid::{Grid, LOCATIONS, Tile};
use crate::lens::Lens;
use crate::modules::ModuleSet;
use crate::path::Route;
use crate::people::{self, Character};
use crate::sprites::Art;
use crate::stores::{self, Regarded, Shared};
use crate::traits::TaskType;

/// **The player**, as an event's party — the one actor in this world who is
/// not a character (wave 1.2).
///
/// A posting is made and withdrawn by nobody who lives in Kawaza, and every
/// event carries a party, so the player needs an index. It names nobody in
/// the roster, which is exactly what makes [`Event::text`] read it as *you*.
pub const PLAYER: usize = usize::MAX;

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
    /// **What this occurrence paid**, in gold, into the treasury. Zero for
    /// everything that moved no money.
    ///
    /// On the event rather than recomputed from the quest board, so a window
    /// of the transcript can be added up without a second walk over sites
    /// that may since have been claimed by somebody else — the one-source rule
    /// applied to money (GDD §4.1).
    pub gold: i64,
    /// The mechanical narration after the address.
    pub note: String,
    /// **The arithmetic behind it**, where this occurrence is a decision
    /// somebody made — every term of the sum the scorer returned, kept as it
    /// was returned (`autonomy::Reckoning`).
    ///
    /// On the event for the reason [`Event::gold`] is on the event: the sum
    /// cannot be recomputed later without lying. A board that has since been
    /// claimed, a wage that has since been stepped and a regard that has
    /// since drifted would all produce a different total from the one this
    /// person actually decided on, and a breakdown that disagrees with the
    /// decision it explains is worse than no breakdown at all.
    ///
    /// **It is a record, not an input.** Nothing in the simulation reads it,
    /// no arithmetic depends on it, and no transcript prints it — a run that
    /// never opens a breakdown is byte-identical to one that opens every one.
    /// `None` for every occurrence that is not somebody deciding something.
    pub judged: Option<crate::autonomy::Reckoning>,
}

impl Event {
    /// What happened, as a sentence: who, and what they did. The feed draws
    /// the world-time and the place in their own columns, so they are not in
    /// here (`attention::place_tag` is the place).
    ///
    /// A party index that names nobody is **you** — [`PLAYER`], which is how
    /// a posting made by the player reads as a sentence about the player
    /// without anything here knowing what a posting is.
    ///
    /// Takes a [`Lens`] rather than the `Sim`, because the feed is a screen:
    /// a line naming a party the player has never seen is exactly the thing
    /// the lens exists to be able to withhold (GDD §3).
    pub fn text(&self, lens: &Lens<'_>) -> String {
        format!(
            "{} {}",
            lens.party(self.party).map_or("you", |party| party.name),
            self.note
        )
    }

    /// The same with its world-time in front — one line, for a transcript or
    /// a report that has no columns to put a stamp in.
    pub fn line(&self, lens: &Lens<'_>) -> String {
        format!("{} - {}", stamp(self.minute), self.text(lens))
    }
}

/// A job at a site: what kind of work it is, what it pays, and how long it
/// holds a party.
#[derive(Clone, Copy, Debug)]
pub struct Quest {
    /// Its name, for the log.
    pub name: &'static str,
    /// **What kind of work it is** (`CAST.md` §2). The scorer reads the
    /// aptitude row whose id is this type and the wants whose `favors` field
    /// covers it; resolution (wave 1.4) reads the same row.
    pub task: TaskType,
    /// What it pays into the treasury on completion.
    pub pot: i64,
    /// How long the work takes, in world-minutes.
    pub duration: i64,
}

/// What has become of one authored job — the third column of a board row.
///
/// **Per job, not per site.** S1 counted claims and handed out the front of
/// the list, which was enough while a site was a single dispatch target; the
/// job board makes each row its own target, so each row carries its own state
/// and a claim names the job it took (UI.md §3c, DESIGN §5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobState {
    /// Nobody has taken it. The only state a row can be ordered from.
    Open,
    /// Claimed at dispatch by this party, and being travelled to or worked.
    Claimed {
        /// Which party holds it — a party is a person, so this is who.
        by: usize,
    },
    /// Worked, paid, and finished by this party.
    Done {
        /// Who did it — so a spent board still says who spent it.
        by: usize,
    },
}

impl JobState {
    /// The party holding or having held this job, if anybody does.
    pub fn holder(self) -> Option<usize> {
        match self {
            JobState::Open => None,
            JobState::Claimed { by } | JobState::Done { by } => Some(by),
        }
    }
}

/// One quest site's board: its authored jobs, in order, and what has become of
/// each. A job is claimed at dispatch — two parties cannot take the same one —
/// and the site runs dry when every row is spent (S1 keeps the simpler choice;
/// DESIGN §10 leaves it open).
#[derive(Clone, Debug)]
pub struct Site {
    /// Which location this site is.
    pub location: usize,
    /// The authored jobs, front first.
    pub quests: Vec<Quest>,
    /// What has become of each of them, one entry per job.
    ///
    /// INVARIANT: the same length as `quests` — a job with no state is a row
    /// the board could not draw and dispatch could not judge.
    pub states: Vec<JobState>,
    /// **Which settlement industry's standing slots these are**, if they are
    /// any (wave 1.3).
    ///
    /// The one fact that tells a completed job which of GDD §4.1's ports its
    /// gold moves through — a site's pot mints into the treasury, an
    /// industry's wage mints into the worker's wallet — and the one that makes
    /// a finished slot open again instead of staying spent. `None` for every
    /// authored quest site, which is why nothing else here changed.
    pub industry: Option<usize>,
}

impl Site {
    /// The job at this slot, if the site has one there.
    pub fn quest(&self, slot: usize) -> Option<&Quest> {
        self.quests.get(slot)
    }

    /// What has become of the job at this slot, if the site has one there.
    pub fn state(&self, slot: usize) -> Option<JobState> {
        self.states.get(slot).copied()
    }

    /// Whether this slot is an open job — the one question dispatch asks.
    pub fn is_open(&self, slot: usize) -> bool {
        self.state(slot) == Some(JobState::Open)
    }

    /// Every open slot, front first — what the scorer weighs one at a time and
    /// what the marker's count counts.
    pub fn open_slots(&self) -> impl Iterator<Item = usize> + '_ {
        self.states
            .iter()
            .enumerate()
            .filter(|(_, state)| **state == JobState::Open)
            .map(|(slot, _)| slot)
    }

    /// How many jobs are still open.
    pub fn open_count(&self) -> usize {
        self.open_slots().count()
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

/// **One job on a board**, addressed the way an order addresses it: which
/// site, and which of its rows.
///
/// One spelling of a pair that travels together everywhere — the order, the
/// claim, the errand, the scorer's candidate and the panel's row all mean the
/// same two numbers, and two spellings of it is how they would come to
/// disagree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JobId {
    /// Which site (sites skip the town; `site_location` is the round trip).
    pub site: usize,
    /// Which of its jobs, front first.
    pub slot: usize,
}

/// What a party is out to do.
///
/// **One field, two shapes** — a paid job at a site, or a visit to somebody's
/// doorstep — because both ride the same journey: a route out, a stay, a route
/// home. A second travel path for the scorer's own errands is the failure this
/// enum exists to make impossible.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Errand {
    /// A job at a site.
    Job(JobId),
    /// A call on another character, by registry index.
    Visit {
        /// Whose doorstep.
        toward: usize,
    },
}

/// Why a journey is being started, and how it was reached.
///
/// One struct rather than two arguments, because the pair travels together
/// everywhere: the words go onto the party and onto the event that opened the
/// errand, and `chosen` is what decides which event closes it — `action-done`
/// for something they thought of themselves, and nothing but the return for
/// an ask they agreed to, which `ask-agreed` already opened.
///
/// **There is no third constructor.** Wave 1.1 had `ordered()` for the
/// player's own dispatch; wave 1.2 replaced ordering with asking, so the two
/// ways an errand begins are the scorer's own idea and a posting somebody
/// took.
#[derive(Clone, Debug)]
pub struct Motive {
    /// The words — the scorer's verdict, or the player's order.
    pub reason: String,
    /// Whether the scorer chose this, as against the player ordering it.
    pub chosen: bool,
}

impl Motive {
    /// They decided it themselves, for this reason.
    pub fn chose(reason: String) -> Self {
        Self {
            reason,
            chosen: true,
        }
    }

    /// **They were asked, and they agreed** (wave 1.2).
    ///
    /// `chosen` is **false**, and the field still means what it meant: it is
    /// what decides whether an `action-done` closes the journey, and an
    /// answered ask is bracketed by `ask-agreed` instead. One opener and one
    /// closer per errand, never two of either.
    pub fn asked(reason: String) -> Self {
        Self {
            reason,
            chosen: false,
        }
    }
}

/// Everything a journey needs beyond where it is going: what it is for, why,
/// and the sentence the departure lands in the feed as.
#[derive(Clone, Debug)]
struct Departure {
    errand: Errand,
    motive: Motive,
    note: String,
}

/// One party.
///
/// **One party per character** (wave 1.1): the roster and this list are the
/// same length and the same order, so party `i` is character `i`'s
/// one-person band. That is what lets the scorer send somebody out through
/// the *player's* dispatch loop rather than through a second one — GDD §5's
/// parties module (wave 4) is what puts more than one name in a band.
#[derive(Clone, Debug)]
pub struct Party {
    /// Its name, as the log and the strip print it — the member's own, since
    /// a one-person party is a person.
    pub name: &'static str,
    /// The character who fields it, by registry index. They stand at their
    /// home tile whenever this party is idle.
    pub member: usize,
    /// Its token's portrait — the member's own, so a face on the road and a
    /// face at a doorstep are the same person.
    pub token: Art,
    /// The tile it is on — always exactly one.
    pub tile: Tile,
    /// What it is doing.
    pub activity: Activity,
    /// What it is out to do, if it is out.
    pub errand: Option<Errand>,
    /// The route home, computed at dispatch and held for the return leg.
    pub home: Option<Route>,
    /// **Why they are doing it, in words.** Written when the errand begins,
    /// by whoever began it — the scorer's own verdict, or the player. The
    /// character panel, the roster and the `action-started` event all read
    /// this one string, so no surface can give a second answer.
    pub reason: String,
    /// Whether the scorer chose this errand, as against the player ordering
    /// it. What decides whether an `action-done` fires on the return.
    pub chosen: bool,
    /// The world-minute before which work weighs less on this character — the
    /// rest term's whole state, set when they finish a job.
    pub rested_until: u64,
    /// **The posting this errand answers**, if it answers one (wave 1.2).
    ///
    /// Written when an ask is agreed to and cleared when the wage is paid, so
    /// the money that moves on completion is the money the posting promised
    /// and not the standing rate as it stands by then.
    pub posting: Option<usize>,
}

impl Party {
    /// The strip's one-line status. Kept short on purpose: the chip is 176
    /// reference pixels wide and the floors hold every row inside it.
    pub fn status(&self, names: &[&str]) -> String {
        let place = |location: usize| {
            plain(
                LOCATIONS
                    .get(location)
                    .map_or("somewhere", |spec| spec.name),
            )
        };
        let who = |index: usize| names.get(index).copied().unwrap_or("someone");
        match (&self.activity, self.errand) {
            (Activity::Idle, _) => "at home".to_owned(),
            (Activity::Outbound { .. }, Some(Errand::Job(job))) => {
                format!("-> {}", place(site_location(job.site)))
            }
            (Activity::Outbound { .. }, Some(Errand::Visit { toward })) => {
                format!("-> {}'s", who(toward))
            }
            (Activity::Outbound { .. }, None) => "-> somewhere".to_owned(),
            (Activity::Working { .. }, Some(Errand::Job(job))) => {
                format!("at {}", place(site_location(job.site)))
            }
            (Activity::Working { .. }, Some(Errand::Visit { toward })) => {
                format!("with {}", who(toward))
            }
            (Activity::Working { .. }, None) => "working".to_owned(),
            (Activity::Homebound { .. }, _) => "<- home".to_owned(),
        }
    }

    /// The job this party holds, if its errand is one.
    pub fn job(&self) -> Option<JobId> {
        match self.errand {
            Some(Errand::Job(job)) => Some(job),
            _ => None,
        }
    }

    /// The quest this party is out on, if it is out on one.
    pub fn quest<'a>(&self, sites: &'a [Site]) -> Option<&'a Quest> {
        let job = self.job()?;
        sites.get(job.site)?.quest(job.slot)
    }
}

/// A location's name without its article — what a 176-pixel chip has room for.
///
/// A derivation and not a second name: `LOCATIONS` still holds one display
/// name per place, and this is that name with four characters of grammar
/// taken off it.
pub fn plain(name: &'static str) -> &'static str {
    name.strip_prefix("the ").unwrap_or(name)
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
    /// Regard drifts toward its fact-set baseline, and reschedules itself
    /// (GDD §4.2). Ambient: it emits no event, because nothing *happened* to
    /// anybody — the feed is for things that did.
    Drift,
    /// **One character weighs what to do next** (GDD §5's autonomy module),
    /// and reschedules itself. Ambient in the same sense drift is: the
    /// occurrence is the cadence, and only a decision that moves somebody
    /// puts anything on the feed.
    Rescore {
        /// Whose turn it is.
        who: usize,
    },
    /// **One need's interval falls due** (GDD §5's needs module, wave 1.3),
    /// and reschedules itself. Ambient in the sense drift is: a burn that was
    /// met is nothing that happened to anybody, and only a shortfall reaches
    /// the feed.
    Upkeep {
        /// Which row of `needs::NEEDS`.
        need: usize,
    },
    /// **Somebody who came later arrives in the camp** (`CAST.md` §4's
    /// arrival column, wave 1.3).
    ///
    /// One per character whose `present_from` is not zero, addressed at that
    /// world-minute like every other occurrence — which is the whole of what
    /// makes the staged start speed-invariant.
    Arrival {
        /// Who has arrived.
        who: usize,
    },
}

/// **Every port gold moves through, totalled** (GDD §4.1's exhaustive list).
///
/// Conservation is the claim this exists to make checkable: the treasury plus
/// every wallet, less what the wallets opened holding, is exactly what was
/// minted less what was burned — and a movement that belonged to no named port
/// would show up as the difference. Transfers are between holders and cancel,
/// so the total is kept for a report rather than for the identity.
///
/// **A record, not an input.** Nothing in the simulation reads it and no
/// arithmetic depends on it, exactly as `Event::judged` is a record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ports {
    /// MINT: site pots, into the treasury on resolution.
    pub minted_pots: i64,
    /// MINT: industry wages, into worker wallets (and the levy beside them).
    pub minted_wages: i64,
    /// BURN: upkeep, out of wallets, trait-modulated.
    pub burned_upkeep: i64,
    /// BURN: industry construction, out of the treasury.
    pub burned_building: i64,
    /// TRANSFER: a posting's wage, treasury to wallet. Between holders, so it
    /// cancels in the identity.
    pub transferred: i64,
}

impl Ports {
    /// What the holders should add up to, given what the wallets opened with.
    pub fn expected(&self, opening_wallets: i64) -> i64 {
        opening_wallets + self.minted_pots + self.minted_wages
            - self.burned_upkeep
            - self.burned_building
    }

    /// The ledger on one line, for a report.
    pub fn line(&self) -> String {
        format!(
            "minted {}g in pots and {}g in wages, burned {}g in upkeep and {}g in building, \
             transferred {}g in posted wages",
            self.minted_pots,
            self.minted_wages,
            self.burned_upkeep,
            self.burned_building,
            self.transferred
        )
    }
}

/// The moving world, held as one resource.
#[derive(Clone, Debug, Default)]
pub struct Sim {
    /// Every party, in registry order — one per character.
    pub parties: Vec<Party>,
    /// Every quest site, in `LOCATIONS` order (sans the town).
    pub sites: Vec<Site>,
    /// Pots accumulate here, wages and buildings leave from here.
    pub treasury: i64,
    /// **What has moved, by port** (GDD §4.1) — the conservation test's
    /// ground truth and the only place a flow figure may honestly come from.
    pub ports: Ports,
    /// Everything that has happened, in firing order — the log's source and
    /// the sweep's transcript.
    pub events: Vec<Event>,
    /// Everyone in the settlement, in registry order (GDD §3).
    pub people: Vec<Character>,
    /// The shared state every module couples through: regard, bonds and
    /// grudges, marks (GDD §4). **In sim state, because replay is the
    /// contract** — see `stores.rs`.
    pub shared: Shared,
    /// What each class of event does to the player's attention (GDD §3).
    ///
    /// **Sim state, not UI state**: the player changes it through recorded
    /// input, and it changes what the world does — a replay that carried the
    /// orders and not this would reproduce the journeys and not the pauses.
    pub attention: Attention,
    /// **The player's ledger** — every posting ever made (GDD's postings
    /// section, wave 1.2).
    ///
    /// Sim state and recorded input, like the auto-pause config: making a
    /// posting and withdrawing one are the two new inputs a recording
    /// carries, and the ledger drawer is a view of this and nothing else.
    pub postings: Postings,
    /// **What the settlement pays per task type** — the standing rates, the
    /// policy lever the mass moves by.
    pub rates: Rates,
    /// **What the settlement has built, and what it pays for a shift of it**
    /// (GDD §5's settlement module, wave 1.3).
    ///
    /// Sim state and recorded input, like the rates beside it: building and
    /// stepping a wage are the two new inputs a recording carries, and the
    /// settlement panel is a view of this and nothing else.
    pub settlement: crate::settlement::Settlement,
    /// Which modules this scenario opened with (GDD §5).
    ///
    /// Sim state for the same reason the config is: it changes what the world
    /// does, it rides every stamp, and a recording that did not carry it
    /// would be a recording of an unknown build.
    pub modules: ModuleSet,
    /// Why the world is stopped, when it stopped itself. Cleared by the
    /// player's next speed input (`sim::acknowledge_pause`).
    pub paused_by: Option<Pause>,
    /// How many times the world has stopped itself — an assertable count, so
    /// a check can say "this run auto-paused four times" rather than "it
    /// paused at some point".
    pub pauses: u64,
    queue: Vec<Occurrence>,
    next_seq: u64,
}

impl Resource for Sim {}

/// **Where each site stands**, by `LOCATIONS` index, in `Sim::sites` order.
///
/// The four authored quest sites are `LOCATIONS[1..]`, in order — which is
/// what this was arithmetic for until wave 1.3 — and the settlement's standing
/// slots are the fifth entry, at the camp itself: the first building is the
/// camp's own fire with a roof over it (`CAST.md` §1), so it wants no marker
/// of its own and the camp's marker is where it is reached.
///
/// A table rather than `site + 1`, so a site that is not the next location is
/// a row here and not a special case at thirty-five call sites.
pub const SITE_LOCATIONS: &[usize] = &[1, 2, 3, 4, crate::grid::TOWN];

/// The `LOCATIONS` index a site index names.
pub fn site_location(site: usize) -> usize {
    SITE_LOCATIONS
        .get(site)
        .copied()
        .unwrap_or(crate::grid::TOWN)
}

/// One party per character, standing at their own doorstep.
///
/// **The party list and the roster are the same list, twice** — same length,
/// same order — which is what makes "the scorer sends somebody out through the
/// player's dispatch loop" a fact about indices rather than a hope.
fn authored_parties(people: &[Character]) -> Vec<Party> {
    people
        .iter()
        .enumerate()
        .map(|(index, person)| Party {
            name: person.name,
            member: index,
            token: person.icon,
            tile: person.home,
            activity: Activity::Idle,
            errand: None,
            home: None,
            reason: String::new(),
            chosen: false,
            rested_until: 0,
            posting: None,
        })
        .collect()
}

/// The authored quests, one site per non-town location.
///
/// Every quest carries a task type (`CAST.md` §2) from wave 1.1 on: the
/// scorer weighs the aptitude row whose id is that type, and resolution
/// (wave 1.4) will read the same row.
///
/// **Six per site since wave 1.1**, where S1 authored one or two. S1's board
/// was written for a player who dispatches three parties by hand; a world
/// where ten people go looking for work empties a seven-job board before the
/// first day is out, and a settlement with nothing to do is not a settlement
/// the scorer can be judged on. Sites still run dry (DESIGN §10's open
/// question, still answered the simpler way) — they just take a while. Each
/// site leans toward the work its fiction implies and carries at least one job
/// of every type, so no aptitude in the vocabulary is a chip with nothing to
/// do.
fn authored_sites() -> Vec<Site> {
    let quests: [(&str, Vec<Quest>); 4] = [
        (
            "watchtower",
            vec![
                Quest {
                    name: "the beacon watch",
                    task: TaskType::Scout,
                    pot: 60,
                    duration: 120,
                },
                Quest {
                    name: "the second watch",
                    task: TaskType::Scout,
                    pot: 50,
                    duration: 120,
                },
                Quest {
                    name: "the ridge patrol",
                    task: TaskType::Fight,
                    pot: 55,
                    duration: 100,
                },
                Quest {
                    name: "the signal repair",
                    task: TaskType::Craft,
                    pot: 45,
                    duration: 80,
                },
                Quest {
                    name: "the long look",
                    task: TaskType::Scout,
                    pot: 50,
                    duration: 140,
                },
                Quest {
                    name: "the tower stores",
                    task: TaskType::Labor,
                    pot: 35,
                    duration: 90,
                },
            ],
        ),
        (
            "deep-cave",
            vec![
                Quest {
                    name: "the mushroom haul",
                    task: TaskType::Labor,
                    pot: 40,
                    duration: 90,
                },
                Quest {
                    name: "the deep survey",
                    task: TaskType::Scout,
                    pot: 55,
                    duration: 150,
                },
                Quest {
                    name: "the ore run",
                    task: TaskType::Labor,
                    pot: 45,
                    duration: 110,
                },
                Quest {
                    name: "the shoring",
                    task: TaskType::Craft,
                    pot: 50,
                    duration: 100,
                },
                Quest {
                    name: "the dark crawl",
                    task: TaskType::Fight,
                    pot: 60,
                    duration: 130,
                },
                Quest {
                    name: "the second haul",
                    task: TaskType::Labor,
                    pot: 35,
                    duration: 90,
                },
            ],
        ),
        (
            "old-crypt",
            vec![
                Quest {
                    name: "the crypt seal",
                    task: TaskType::Craft,
                    pot: 45,
                    duration: 100,
                },
                Quest {
                    name: "the second seal",
                    task: TaskType::Fight,
                    pot: 45,
                    duration: 100,
                },
                Quest {
                    name: "the grave count",
                    task: TaskType::Labor,
                    pot: 35,
                    duration: 80,
                },
                Quest {
                    name: "the stone mend",
                    task: TaskType::Craft,
                    pot: 50,
                    duration: 110,
                },
                Quest {
                    name: "the night watch",
                    task: TaskType::Fight,
                    pot: 55,
                    duration: 120,
                },
                Quest {
                    name: "the far survey",
                    task: TaskType::Scout,
                    pot: 45,
                    duration: 100,
                },
            ],
        ),
        (
            "black-vault",
            vec![
                Quest {
                    name: "the vault ledger",
                    task: TaskType::Fight,
                    pot: 80,
                    duration: 180,
                },
                Quest {
                    name: "the vault door",
                    task: TaskType::Craft,
                    pot: 70,
                    duration: 150,
                },
                Quest {
                    name: "the deep pry",
                    task: TaskType::Labor,
                    pot: 60,
                    duration: 140,
                },
                Quest {
                    name: "the long approach",
                    task: TaskType::Scout,
                    pot: 65,
                    duration: 160,
                },
                Quest {
                    name: "the second ledger",
                    task: TaskType::Fight,
                    pot: 70,
                    duration: 170,
                },
                Quest {
                    name: "the last seal",
                    task: TaskType::Craft,
                    pot: 60,
                    duration: 150,
                },
            ],
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
            let states = vec![JobState::Open; quests.len()];
            Site {
                location,
                quests,
                states,
                industry: None,
            }
        })
        // **And the settlement's own sites, one per industry, empty** (wave
        // 1.3). They stand at the camp and hold no work until somebody builds
        // them: a camp is a camp until the first building goes up, and an
        // industry with rows before it was built would be a board the scorer
        // could take work off for free.
        .chain(
            crate::settlement::INDUSTRIES
                .iter()
                .enumerate()
                .map(|(index, spec)| Site {
                    location: site_location(spec.site),
                    quests: Vec::new(),
                    states: Vec::new(),
                    industry: Some(index),
                }),
        )
        .collect()
}

/// **The authored relationship preset** (`CAST.md` §5): the seeded bonds,
/// grudges and regard, written through the store APIs at scenario open.
///
/// The other preset is *flat* — every edge zero and no facts — and which one a
/// scenario opens on is `bonds_preset`, a drawer row like every other constant
/// and so on every stamp. Nothing here writes a vector directly: a seed is the
/// same `adjust_regard` / `record_grudge` any later wave calls, so a seeded
/// world is a world that could have got there by living.
fn seed_relationships(sim: &mut Sim, tuning: &Tuning) {
    if tuning.bonds_preset == 0 {
        return;
    }
    let who = |id: &str| sim.people.iter().position(|person| person.id == id);
    let (
        Some(bob),
        Some(steve),
        Some(alex),
        Some(tim),
        Some(rin),
        Some(goro),
        Some(hana),
        Some(ludo),
        Some(ines),
        Some(odd),
    ) = (
        who("bob"),
        who("steve"),
        who("alex"),
        who("tim"),
        who("rin"),
        who("goro"),
        who("hana"),
        who("ludo"),
        who("ines"),
        who("odd"),
    )
    else {
        return;
    };
    // Regard first: a bond's floor and a grudge's ceiling hold whatever is
    // written after them, and writing the warmth before the fact is what makes
    // the fact's re-hold visible rather than vacuous.
    for (from, to, delta) in [
        (steve, Regarded::Person(bob), 3),
        (rin, Regarded::Person(steve), 2),
        (ludo, Regarded::Person(tim), 2),
        (tim, Regarded::Person(bob), -2),
        (ines, Regarded::Person(odd), -2),
        (hana, Regarded::Person(goro), 4),
        (goro, Regarded::Person(hana), 4),
        (bob, Regarded::Person(steve), 3),
    ] {
        sim.shared.adjust_regard(tuning, from, to, delta);
    }
    // Toward the player: the four founders came with the guildmaster; the six
    // who joined later have to be earned.
    for founder in [bob, steve, alex, tim] {
        sim.shared
            .adjust_regard(tuning, founder, Regarded::Player, 2);
    }
    // The facts. Bonds are written through the shared-success door, which is
    // the only door there is: two successes at high mutual regard.
    for _ in 0..tuning.bond_after.max(1) {
        sim.shared.record_shared_success(tuning, hana, goro);
        sim.shared.record_shared_success(tuning, bob, steve);
    }
    // The same job twice and only one of them paid: Goro holds it, and Odd
    // does not know.
    sim.shared.record_grudge(
        tuning,
        goro,
        Regarded::Person(odd),
        stores::GrudgeCause::Betrayal,
    );
}

impl Sim {
    /// The scenario at its opening state.
    ///
    /// Takes the shipped constants because the drift cadence, the scorer's
    /// cadence and the relationship preset are all among them, and the first
    /// of each occurrence is scheduled here — with a world-time address, like
    /// everything else, so it is speed-invariant for free.
    ///
    /// Takes the module set because **autonomy off means nothing is
    /// scheduled**: the world it degrades to is the wave-0b world, where
    /// everyone idles at home until the player says otherwise, and the queue
    /// says so rather than a flag being consulted at every firing.
    pub fn opening(tuning: &Tuning, modules: ModuleSet) -> Self {
        let people = people::roster();
        let mut sim = Self {
            parties: authored_parties(&people),
            sites: authored_sites(),
            treasury: 0,
            ports: Ports::default(),
            events: Vec::new(),
            people,
            postings: Postings::default(),
            rates: Rates::opening(),
            settlement: crate::settlement::Settlement::opening(),
            shared: Shared::opening(),
            attention: Attention::opening(),
            modules,
            paused_by: None,
            pauses: 0,
            queue: Vec::new(),
            next_seq: 0,
        };
        seed_relationships(&mut sim, tuning);
        sim.schedule(stores::drift_interval(tuning), Occ::Drift);
        if modules.enabled(crate::autonomy::MODULE) {
            // **Only the people who are here.** The six who came later are
            // scheduled by their own arrival, which is what makes presence a
            // fact about the world rather than a filter every reader has to
            // remember.
            for who in 0..sim.parties.len() {
                if sim.people.get(who).is_some_and(|person| person.present) {
                    sim.schedule(
                        crate::autonomy::first_score(tuning, who),
                        Occ::Rescore { who },
                    );
                }
            }
        }
        // **The staged start** (`CAST.md` §4): one occurrence per person who
        // is not a founder, at the world-minute the roster authors.
        for who in 0..sim.people.len() {
            if let Some(person) = sim.people.get(who)
                && !person.present
            {
                sim.schedule(person.present_from, Occ::Arrival { who });
            }
        }
        // **And the needs**, one occurrence per row of the list. With the
        // module off nothing is scheduled at all, which is the degrades-to
        // sentence said by the queue rather than by a flag consulted at every
        // firing (`autonomy`'s own precedent).
        if modules.enabled(crate::needs::MODULE) {
            for row in 0..crate::needs::NEEDS.len() {
                if let Some(need) = crate::needs::NEEDS.get(row) {
                    sim.schedule(
                        crate::needs::first_burn(tuning, need),
                        Occ::Upkeep { need: row },
                    );
                }
            }
        }
        sim
    }

    /// **Stage the whole camp as arrived** — an instrument, not a game path.
    ///
    /// The staged start is content (`CAST.md` §4) and most of the batteries
    /// are about something else: whether one figure is drawn per person,
    /// whether a job row's verdict equals the world's answer, whether a trait
    /// sum is the arithmetic the vocabulary says. Those claims are about the
    /// *whole cast*, and a world where six of them are not born yet would
    /// judge them over four people and call it a pass. The batteries whose
    /// subject **is** the staged start conduct real runs and never call this.
    pub fn everybody_here(&mut self) {
        for person in &mut self.people {
            person.present = true;
        }
    }

    /// **Re-deal when each person first looks up** — the economy sweep's whole
    /// population (GDD §9, wave 1.3).
    ///
    /// An instrument, not a game path: this build reads no `Rng`, so an
    /// "idle-player seed" has nothing to draw an economy from. What a seed
    /// would have varied is the order in which ten people meet a finite
    /// board — who reaches the mushroom haul first and who finds it taken —
    /// so the sweep varies exactly that and nothing the player controls, by
    /// rotating every scheduled first rescore by `by` minutes. Every constant,
    /// rate and authored pot is the shipped one in all of them.
    pub fn stagger_first_looks(&mut self, tuning: &Tuning, by: u64) {
        let cadence = crate::autonomy::interval(tuning).max(1);
        for occurrence in &mut self.queue {
            if let Occ::Rescore { who } = occurrence.kind {
                // **Per roster place**, so the rotation re-orders rather than
                // shifting everybody equally — a world where all ten look up a
                // minute later is the same world, and the thing a seed would
                // have varied is who looks up *first*. Held inside one cadence
                // so nobody's first look is pushed past their second.
                occurrence.at += by.saturating_mul(who as u64 + 1) % cadence;
            }
        }
    }

    /// Whether everything is home and nothing a party is waiting on is
    /// scheduled — the world at rest (the sweep's stopping condition).
    ///
    /// **Ambient occurrences do not count.** Regard drift, the scorer's
    /// cadence and the upkeep interval all reschedule themselves forever, and
    /// an arrival is a date on the calendar rather than a journey anybody is
    /// waiting on — so a queue that had to be empty would mean the world was
    /// never at rest once anybody could think. At rest means nobody is abroad
    /// and nothing is going to move them *before the next ambient occurrence*
    /// — which, with autonomy on, is a lull rather than an ending, and the
    /// sweep says so by stopping at a world-minute instead.
    pub fn at_rest(&self) -> bool {
        self.queue.iter().all(|occurrence| {
            matches!(
                occurrence.kind,
                Occ::Drift | Occ::Rescore { .. } | Occ::Upkeep { .. } | Occ::Arrival { .. }
            )
        }) && self
            .parties
            .iter()
            .all(|party| party.activity == Activity::Idle)
    }

    /// Everyone's name, in registry order — what a status line needs to say
    /// whose doorstep somebody is standing on.
    pub fn names(&self) -> Vec<&'static str> {
        self.people.iter().map(|person| person.name).collect()
    }

    /// Put one occurrence on the queue.
    fn schedule(&mut self, at: u64, kind: Occ) {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.queue.push(Occurrence { at, seq, kind });
    }

    /// Record one event — and, when its class says so, stop the world.
    ///
    /// **The auto-pause lives here** because this is where an event becomes a
    /// fact: the mode comes off the config (which came off the class table),
    /// nothing in this function knows which class it is looking at, and the
    /// clock is put at speed 0 by `fire_due` in the same tick. The *first*
    /// pause-class event of a crossed span is the one that stops the world;
    /// the rest of the span still fires, because their world-times have
    /// already arrived and a pause holds the future, not the present.
    fn emit(&mut self, minute: u64, class: EventClass, party: usize, tile: Tile, note: String) {
        self.emit_paying(minute, class, party, tile, 0, note);
    }

    /// The same, for an occurrence that moved gold.
    fn emit_paying(
        &mut self,
        minute: u64,
        class: EventClass,
        party: usize,
        tile: Tile,
        gold: i64,
        note: String,
    ) {
        self.events.push(Event {
            minute,
            class,
            party,
            tile,
            location: crate::grid::location_at(tile),
            gold,
            note,
            judged: None,
        });
        if self.attention.mode(class) == Mode::PauseAndFocus && self.paused_by.is_none() {
            self.paused_by = Some(Pause {
                event: self.events.len() - 1,
                class,
                minute,
            });
            self.pauses += 1;
        }
    }

    /// Record one of the asks module's occurrences (wave 1.2).
    ///
    /// Public for the reason `emit_action` is: the decision belongs to the
    /// module and the emission belongs here, because this is the one door
    /// past which an event can stop the world.
    pub fn emit_ask(
        &mut self,
        minute: u64,
        class: EventClass,
        party: usize,
        tile: Tile,
        note: String,
    ) {
        self.emit(minute, class, party, tile, note);
    }

    /// Record a needs-module occurrence — today the upkeep shortfall (wave
    /// 1.3).
    ///
    /// Public for the reason [`Sim::emit_ask`] is: the arithmetic belongs to
    /// the module and the emission belongs here, because this is the one door
    /// past which an event can stop the world.
    pub fn emit_need(&mut self, minute: u64, who: usize, tile: Tile, note: String) {
        self.emit(minute, EventClass::UpkeepShortfall, who, tile, note);
    }

    /// Record a settlement occurrence — a building, or the slots it opened.
    ///
    /// Its party is [`PLAYER`], because building is the player's own act and
    /// nobody in Kawaza signs for it — the same index a posting is made under,
    /// which is what makes the feed read it as *you*.
    pub fn emit_settlement(&mut self, minute: u64, class: EventClass, tile: Tile, note: String) {
        self.emit(minute, class, PLAYER, tile, note);
    }

    /// Record somebody arriving in the camp (`CAST.md` §4's arrival column).
    pub fn emit_joined(&mut self, minute: u64, who: usize, tile: Tile, note: String) {
        self.emit(minute, EventClass::Joined, who, tile, note);
    }

    /// Record an `action-started` — the scorer's own class, and the one place
    /// a *reason* reaches the feed.
    ///
    /// Public because the decision belongs to `autonomy.rs` and the emission
    /// belongs here: the class table is where emission is declared (GDD §3),
    /// and a module that wrote straight into `events` would be a second door
    /// past the auto-pause.
    pub fn emit_action(&mut self, minute: u64, tile: Tile, who: usize, note: String) {
        self.emit(minute, EventClass::ActionStarted, who, tile, note);
    }

    /// **Keep the arithmetic behind the occurrence just emitted** (UI.md §3e).
    ///
    /// Called immediately after the emit that reports a decision, with the
    /// [`autonomy::Reckoning`] the scorer's own [`autonomy::Judged`] became —
    /// so the feed's breakdown and the decision are one derivation and not
    /// two. It writes onto the last event rather than taking a parameter on
    /// four emitters, because only the two decision classes have a sum and a
    /// parameter every emitter had to pass `None` for is a parameter nobody
    /// reads.
    ///
    /// A call with no event to attach to is a caller that emitted nothing,
    /// which is a bug in the caller rather than a state of the world — said
    /// loudly, never swallowed (CLAUDE.md: no silent failure).
    pub fn remember(&mut self, reckoning: crate::autonomy::Reckoning) {
        match self.events.last_mut() {
            Some(event) => event.judged = Some(reckoning),
            None => crate::checks::fail(
                "a decision's arithmetic was recorded against no occurrence",
                "`Sim::remember` was called with an empty event log; it attaches to the \
                 occurrence just emitted, so the emit that should have preceded it did not \
                 happen",
            ),
        }
    }
}

/// The player has seen why the world stopped: clear the reason.
///
/// Called by the one speed-input path (`flow::apply_speed`), so resuming and
/// forgetting are one action — a reason that outlived its pause would make the
/// banner lie, and a reason that stayed would stop the next auto-pause from
/// being recorded.
pub fn acknowledge_pause(sim: &mut Sim) {
    sim.paused_by = None;
}

/// Why a dispatch was refused — surfaced as a toast, never silent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The party is not idle at home.
    NotIdle,
    /// The site has no job at that slot at all.
    Dry,
    /// The job is there and somebody already has it, or it is finished.
    Taken,
    /// No passable route exists (authoring fault; said loudly anyway).
    Unreachable,
}

impl Refusal {
    /// The toast's sentence.
    ///
    /// Every refusal names the thing it is about, which is why the job's name
    /// travels with the party's and the site's: a board row that bounced with
    /// "that is taken" and no name is a row the player has to guess at.
    pub fn message(&self, party: &str, site: &str, job: &str) -> String {
        match self {
            Refusal::NotIdle => format!("{party} is out - only an idle party can be sent"),
            Refusal::Dry => format!("{site} has no open job left"),
            Refusal::Taken => format!("{job} is taken already - pick an open job"),
            Refusal::Unreachable => {
                format!("no route reaches {site} - the map should not allow this")
            }
        }
    }
}

/// **The one journey.** A party leaves the tile it is standing on, walks a
/// route computed here, stays, and walks a stored route home.
///
/// Player dispatch and the scorer's own errands both come through this
/// function and there is no other way to move: that is the "one loop, two
/// callers" the wave plan asks for, held by there being exactly one place that
/// writes [`Activity::Outbound`].
fn begin_journey(
    sim: &mut Sim,
    grid: &Grid,
    tuning: &Tuning,
    now: u64,
    party_index: usize,
    goal: Tile,
    setting_out: Departure,
) -> Result<(), Refusal> {
    let Some(party) = sim.parties.get(party_index) else {
        return Err(Refusal::NotIdle);
    };
    if party.activity != Activity::Idle {
        return Err(Refusal::NotIdle);
    }
    let from = party.tile;
    let Some(out) = crate::path::route(grid, tuning, from, goal) else {
        return Err(Refusal::Unreachable);
    };
    let Some(home) = crate::path::route(grid, tuning, goal, from) else {
        return Err(Refusal::Unreachable);
    };
    let Some(first) = out.tiles.first().copied() else {
        return Err(Refusal::Unreachable);
    };
    let first_cost = grid.get(first).cost(tuning).unwrap_or(0);
    let next_at = now + u64::try_from(first_cost).unwrap_or(0);
    {
        let party = &mut sim.parties[party_index];
        party.errand = Some(setting_out.errand);
        party.home = Some(home);
        party.reason = setting_out.motive.reason;
        party.chosen = setting_out.motive.chosen;
        party.activity = Activity::Outbound {
            route: out,
            index: 0,
            entered_at: now,
            next_at,
        };
    }
    sim.emit(
        now,
        EventClass::Departed,
        party_index,
        from,
        setting_out.note,
    );
    sim.schedule(next_at, Occ::TileEntry { party: party_index });
    Ok(())
}

/// **The route a party would walk to a site** — the one answer to "how far".
///
/// [`dispatch`] computes it to write the departure's sentence, the journey
/// walks it, and the board's travel line previews it (`Lens::travel`). One
/// function, so a preview cannot promise a journey the sim will not make; a
/// second `route(...)` call anywhere a surface can see is the failure this
/// exists to refuse (GDD §1: one decision function per question).
pub fn route_out(
    grid: &Grid,
    tuning: &Tuning,
    sim: &Sim,
    party_index: usize,
    site_index: usize,
) -> Option<Route> {
    let site = sim.sites.get(site_index)?;
    let goal = LOCATIONS[site.location].tile;
    let from = sim.parties.get(party_index)?.tile;
    crate::path::route(grid, tuning, from, goal)
}

/// Dispatch: the player's whole order vocabulary (DESIGN §5), and the same
/// call the scorer makes when it sends somebody to work.
///
/// **The order names a job**, by (site, slot), because the board is where an
/// order is given and a board row is one job. S1 handed out the front of the
/// site's list and the player could not see which job that was; a UI that
/// names the work while dispatch quietly claims the first-open one is two
/// answers to one question.
///
/// The route out **and** the route home are computed once and stored — terrain
/// is static. The departure event and the first tile entry are addressed from
/// `now`, the world-minute the order was given; paused or not, the address is
/// the clock's.
pub fn dispatch(
    sim: &mut Sim,
    grid: &Grid,
    tuning: &Tuning,
    now: u64,
    party_index: usize,
    job: JobId,
    motive: Motive,
) -> Result<(), Refusal> {
    let JobId {
        site: site_index,
        slot,
    } = job;
    let Some(site) = sim.sites.get(site_index) else {
        return Err(Refusal::Dry);
    };
    let Some(quest) = site.quest(slot).copied() else {
        return Err(Refusal::Dry);
    };
    if !site.is_open(slot) {
        return Err(Refusal::Taken);
    }
    let goal = LOCATIONS[site.location].tile;
    let Some(party) = sim.parties.get(party_index) else {
        return Err(Refusal::NotIdle);
    };
    if party.activity != Activity::Idle {
        return Err(Refusal::NotIdle);
    }
    let Some(out) = route_out(grid, tuning, sim, party_index, site_index) else {
        return Err(Refusal::Unreachable);
    };
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
    let began = begin_journey(
        sim,
        grid,
        tuning,
        now,
        party_index,
        goal,
        Departure {
            errand: Errand::Job(job),
            motive,
            note,
        },
    );
    if began.is_ok()
        && let Some(state) = sim.sites[site_index].states.get_mut(slot)
    {
        *state = JobState::Claimed { by: party_index };
    }
    began
}

/// A call on somebody's doorstep — the scorer's other errand, through the same
/// journey the player's orders ride.
pub fn call_on(
    sim: &mut Sim,
    grid: &Grid,
    tuning: &Tuning,
    now: u64,
    party_index: usize,
    toward: usize,
    motive: Motive,
) -> Result<(), Refusal> {
    let Some(host) = sim.people.get(toward) else {
        return Err(Refusal::Unreachable);
    };
    let (goal, name) = (host.home, host.name);
    let note = format!("set out to call on {name}");
    begin_journey(
        sim,
        grid,
        tuning,
        now,
        party_index,
        goal,
        Departure {
            errand: Errand::Visit { toward },
            motive,
            note,
        },
    )
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
    // available, so take the cheap copy once.
    let grid = world.resource::<Grid>().clone();
    advance_to(world.resource_mut::<Sim>(), &grid, &tuning, now);
    // **The auto-pause, applied**: the world stopping itself is a simulation
    // transition, so it happens here, in the tick the event fired, before
    // anything draws. The clock is the speed, and speed 0 is the pause;
    // nothing synthesises an input, which is what makes a replay reproduce
    // the pause rather than reproduce a click.
    if world.resource::<Sim>().paused_by.is_some() {
        world.resource_mut::<Clock>().paused = true;
    }
}

/// **Fire everything the world owes up to `now`, in world-time order** — the
/// one door an occurrence comes through.
///
/// Split out of [`fire_due`] at wave 1.3 so the economy sweep can run a world
/// forward without an app around it (`economy.rs`): two hundred settlements of
/// several world-days each is a population a conducted run cannot afford, and
/// a second loop over the queue would be a second simulation. The `World` half
/// of `fire_due` — reading the clock, taking the grid, applying the pause — is
/// all that is left up there, and this is everything that *happens*.
pub fn advance_to(sim: &mut Sim, grid: &Grid, tuning: &Tuning, now: u64) {
    loop {
        let due = {
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
        let Some(slot) = due else { break };
        let occurrence = sim.queue.remove(slot);
        match occurrence.kind {
            Occ::TileEntry { party } => tile_entry(sim, grid, tuning, occurrence.at, party),
            Occ::WorkDone { party } => work_done(sim, grid, tuning, occurrence.at, party),
            Occ::Drift => {
                sim.shared.drift(tuning);
                sim.schedule(occurrence.at + stores::drift_interval(tuning), Occ::Drift);
            }
            Occ::Rescore { who } => {
                sim.schedule(
                    occurrence.at + crate::autonomy::interval(tuning),
                    Occ::Rescore { who },
                );
                crate::autonomy::rescore(sim, grid, tuning, occurrence.at, who);
            }
            Occ::Upkeep { need } => {
                if let Some(row) = crate::needs::NEEDS.get(need) {
                    sim.schedule(
                        occurrence.at + crate::needs::interval(tuning, row),
                        Occ::Upkeep { need },
                    );
                }
                crate::needs::burn(sim, tuning, occurrence.at, need);
            }
            Occ::Arrival { who } => arrive(sim, tuning, occurrence.at, who),
        }
    }
}

/// **Somebody who came later walks into the camp** (`CAST.md` §4).
///
/// Presence is written here and nowhere else, and their first rescore is
/// scheduled from the minute they arrived — so the staggered cadence they join
/// on is theirs rather than a leftover of a world they were not in.
fn arrive(sim: &mut Sim, tuning: &Tuning, at: u64, who: usize) {
    let Some(person) = sim.people.get_mut(who) else {
        return;
    };
    if person.present {
        return;
    }
    person.present = true;
    let (home, name, origin) = (person.home, person.name, person.origin);
    if let Some(party) = sim.parties.get_mut(who) {
        party.tile = home;
    }
    sim.emit_joined(at, who, home, format!("arrived in Kawaza - {origin}"));
    let _ = name;
    if sim.modules.enabled(crate::autonomy::MODULE) {
        sim.schedule(
            at + crate::autonomy::first_score(tuning, who),
            Occ::Rescore { who },
        );
    }
}

/// A party enters the next tile of its stored route.
///
/// Read pass then write pass: what the party is doing is copied out first,
/// because emitting an event and scheduling a follow-up both borrow the whole
/// `Sim` the party lives in.
fn tile_entry(sim: &mut Sim, grid: &Grid, tuning: &Tuning, at: u64, index: usize) {
    let (route, step, homebound, errand) = {
        let Some(party) = sim.parties.get(index) else {
            return;
        };
        match &party.activity {
            Activity::Outbound {
                route, index: step, ..
            } => (route.clone(), *step, false, party.errand),
            Activity::Homebound {
                route, index: step, ..
            } => (route.clone(), *step, true, party.errand),
            // A stray occurrence for a party no longer travelling is a
            // scheduler fault; nothing produces one, and a silent skip would
            // hide it — but a panic in release is forbidden, so it is
            // recorded where a check will read it.
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
        let chosen = sim.parties[index].chosen;
        let reason = sim.parties[index].reason.clone();
        {
            let party = &mut sim.parties[index];
            party.activity = Activity::Idle;
            party.errand = None;
            party.home = None;
            party.chosen = false;
        }
        sim.emit(at, EventClass::Returned, index, tile, "is home".to_owned());
        // **The decision's own end** (wave 1.1): only what the scorer began
        // closes here, because a player's order was never a question anybody
        // asked themselves.
        if chosen {
            sim.emit(
                at,
                EventClass::ActionDone,
                index,
                tile,
                format!("is done with what they set out to do - {reason}"),
            );
        }
        return;
    }
    match errand {
        Some(Errand::Job(JobId { site, slot })) => {
            let Some(quest) = sim
                .sites
                .get(site)
                .and_then(|site| site.quest(slot))
                .copied()
            else {
                return;
            };
            let until = at + u64::try_from(quest.duration).unwrap_or(0);
            sim.parties[index].activity = Activity::Working { until };
            let site_name = LOCATIONS[site_location(site)].name;
            sim.emit(
                at,
                EventClass::Arrived,
                index,
                tile,
                format!("arrived at {site_name}"),
            );
            // **The messenger catches them where the road ends** (wave 1.2):
            // every ask addressed to somebody who was already out is heard
            // here, at this world-minute, and answered at their next rescore.
            crate::asks::deliver_on_arrival(sim, at, index);
            sim.emit(
                at,
                EventClass::WorkBegan,
                index,
                tile,
                format!(
                    "began {} - {} minutes of {} work",
                    quest.name,
                    quest.duration,
                    quest.task.id()
                ),
            );
            sim.schedule(until, Occ::WorkDone { party: index });
        }
        Some(Errand::Visit { toward }) => {
            let until = at + u64::try_from(tuning.visit_minutes.max(0)).unwrap_or(0);
            sim.parties[index].activity = Activity::Working { until };
            let host = sim.people.get(toward).map_or("somebody", |who| who.name);
            sim.emit(
                at,
                EventClass::Arrived,
                index,
                tile,
                format!("arrived at {host}'s"),
            );
            crate::asks::deliver_on_arrival(sim, at, index);
            sim.schedule(until, Occ::WorkDone { party: index });
        }
        None => {}
    }
}

/// What a completed visit is worth: a small warmth, and symmetric or not by a
/// drawer row.
///
/// Its own function because the scorer battery asserts it with shipped
/// literals and the scheduler performs it — one arithmetic, two callers, so a
/// check cannot pass over a second copy of the rule.
pub fn settle_visit(sim: &mut Sim, tuning: &Tuning, visitor: usize, host: usize) {
    sim.shared
        .adjust_regard(tuning, visitor, Regarded::Person(host), tuning.visit_regard);
    if tuning.visit_mutual != 0 {
        sim.shared
            .adjust_regard(tuning, host, Regarded::Person(visitor), tuning.visit_regard);
    }
}

/// A party's stay ends: the pot pays on a job, the warmth lands on a visit,
/// and either way the party turns for home on the route stored at the start.
fn work_done(sim: &mut Sim, grid: &Grid, tuning: &Tuning, at: u64, index: usize) {
    let (errand, home, tile, member) = {
        let Some(party) = sim.parties.get(index) else {
            return;
        };
        (party.errand, party.home.clone(), party.tile, party.member)
    };
    let Some(errand) = errand else { return };
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
    match errand {
        Errand::Job(JobId { site, slot }) => {
            let Some(quest) = sim
                .sites
                .get(site)
                .and_then(|site| site.quest(slot))
                .copied()
            else {
                return;
            };
            // **The row is finished, and the board says so.** A job whose pot
            // has paid is neither open nor still somebody's; the state moves
            // here, where the money moves, so no second pass has to work out
            // which claimed rows are over (UI.md §3c).
            if let Some(state) = sim.sites[site].states.get_mut(slot) {
                *state = JobState::Done { by: index };
            }
            // **Which port this job's gold moves through** (GDD §4.1), read
            // off the site rather than branched on a name: a quest site's pot
            // is minted into the treasury, and an industry's shift is minted
            // into the worker's wallet with the levy beside it. A shift's slot
            // then opens again, which is the whole of what makes it standing.
            let shift = crate::settlement::industry_at(sim, site).is_some();
            let (paid, levied) = if shift {
                let moved = crate::settlement::settle_shift(sim, tuning, index, site);
                sim.ports.minted_wages += moved.0 + moved.1;
                crate::settlement::reopen(sim, JobId { site, slot });
                moved
            } else {
                sim.treasury += quest.pot;
                sim.ports.minted_pots += quest.pot;
                (0, 0)
            };
            // **The wage, paid on completion** (GDD §4.1's TRANSFER port):
            // treasury to wallet, and the regard the paying moves. Nothing is
            // paid for work nobody posted, which is why this reads the
            // party's own posting rather than the board.
            let wage = crate::answers::settle_wage(sim, tuning, index);
            let treasury = sim.treasury;
            // The rest term's whole state: work weighs less on somebody who
            // just finished some (`autonomy.rs`).
            sim.parties[index].rested_until = at + crate::autonomy::rest_minutes(tuning);
            let (into_treasury, note) = if shift {
                (
                    levied,
                    format!(
                        "worked {} - {paid}g earned, {levied}g levied ({treasury}g held) - \
                         turning for home",
                        quest.name
                    ),
                )
            } else if wage > 0 {
                (
                    quest.pot,
                    format!(
                        "completed {} - {}g in, {wage}g paid out ({treasury}g held) - turning \
                         for home",
                        quest.name, quest.pot
                    ),
                )
            } else {
                (
                    quest.pot,
                    format!(
                        "completed {} - {}g into the treasury ({treasury}g held) - turning for \
                         home",
                        quest.name, quest.pot
                    ),
                )
            };
            sim.emit_paying(
                at,
                EventClass::QuestComplete,
                index,
                tile,
                into_treasury,
                note,
            );
        }
        Errand::Visit { toward } => settle_visit(sim, tuning, member, toward),
    }
    sim.schedule(next_at, Occ::TileEntry { party: index });
}
