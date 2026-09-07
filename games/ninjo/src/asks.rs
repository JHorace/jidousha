//! **Asks** — the postings module (GDD's postings section, wave 1.2).
//!
//! # The player never orders; the player posts
//!
//! A posting is an entry on the player's ledger: **who** (a named character,
//! or anyone), **what** (a job, a site, or a task type), **until** (done, or
//! withdrawn), and a **wage** that defaults to the standing rate for the
//! work's task type. Three usages of one mechanism: targeted is the contract,
//! open is the bounty, standing is the repeat-until-told-otherwise order.
//!
//! # Nothing binds until it is heard
//!
//! A posting binds nobody until it is heard (the petitions rule, mirrored).
//! A targeted posting to somebody in camp is heard at once — and, because
//! hearing an ask is being asked, they weigh it there and then, through the
//! one scorer. To somebody away it rides a messenger and is heard when they
//! next arrive somewhere; they answer at their next rescore, at home.
//! Withdrawal is instant on the ledger, so a posting withdrawn before it was
//! heard is a posting nobody ever answers.
//!
//! # Deciding is the scorer's, not this module's
//!
//! There is no second decision function here. [`candidates`] hands
//! `autonomy::choose` one more [`crate::autonomy::Action`] per heard posting,
//! `autonomy::weigh` scores it beside everything else the character could do,
//! and agreeing is the ordinary dispatch loop. What this module owns is the
//! record, the hearing, the answer's bookkeeping, and the money.
//!
//! # Degrades to
//!
//! With the module off there is no ledger and no posting: the site panel is
//! read-only, the standing rates panel does not open, and the world is the
//! wave-1.1 world — pure observation of people who decide for themselves.

use crate::autonomy;
use crate::constants::Tuning;
use crate::grid::Grid;
use crate::sim::{self, Activity, JobId, Sim};
use crate::traits::TaskType;

/// The module id, as `modules::MODULES` and every stamp spell it.
pub const MODULE: &str = "asks";

/// What one tap of the standing-rates panel moves a rate by, in gold.
///
/// A code constant and not a drawer row, for the reason wave 0a gave for the
/// auto-pause defaults: the panel is a live write *and* a recorded input,
/// where a drawer row would be a restart, and two ways to move one number is
/// the second way this repo's first convention refuses. Four gold, because a
/// step has to be felt: the wage's pull is per ten gold, so a step of one
/// would be a control that usually does nothing.
pub const RATE_STEP: i64 = 4;

/// The most a standing rate may be set to, in gold — the panel's own ceiling,
/// and the same range the tuning drawer offers every other number.
pub const RATE_MAX: i64 = 60;

/// How much better than its rivals a posting has to score before the row
/// stops calling the character **reluctant**.
///
/// Presentation only: it changes what the preview *says*, never what the
/// scorer *does*, which is why it is a constant here rather than a term
/// anywhere. The verdict's two other readings are facts about the sum.
pub const RELUCTANT_MARGIN: i64 = 6;

/// One row of the standing-rate table: a task type, and what the settlement
/// pays for it before the player says otherwise.
#[derive(Clone, Copy, Debug)]
pub struct RateSpec {
    /// Which kind of work.
    pub task: TaskType,
    /// What it opens at, in gold per fulfilment.
    pub opening: i64,
}

/// **The standing rates** — the lever for the mass (the GDD's postings
/// section): raise fight pay and the fighters drift to the crypt without
/// anybody being named.
///
/// Opening values are table data, exactly as an event class's default mode
/// is: the panel is the one way to move a rate, and a drawer row beside it
/// would be a second way to do a thing that already has one (wave 0a's
/// recorded decision, GDD §3). They lean the way the fiction does — fighting
/// pays most, camp labour least — and every one of them leaves a margin on
/// every authored pot, because the player is a contractor and the margin is
/// the income (GDD §4.1).
pub const RATES: &[RateSpec] = &[
    RateSpec {
        task: TaskType::Fight,
        opening: 24,
    },
    RateSpec {
        task: TaskType::Labor,
        opening: 16,
    },
    RateSpec {
        task: TaskType::Scout,
        opening: 20,
    },
    RateSpec {
        task: TaskType::Craft,
        opening: 20,
    },
];

/// What the settlement pays per task type right now.
///
/// **Simulation state**, for the reason the auto-pause config is: the player
/// changes it through a recorded input and it changes what the world does, so
/// a replay that did not carry it would reproduce the postings and not the
/// answers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rates {
    per_task: Vec<i64>,
}

impl Default for Rates {
    fn default() -> Self {
        Self::opening()
    }
}

impl Rates {
    /// The table's own values — what a scenario opens on.
    pub fn opening() -> Self {
        Self {
            per_task: RATES.iter().map(|spec| spec.opening).collect(),
        }
    }

    /// The standing rate for this kind of work.
    pub fn of(&self, task: TaskType) -> i64 {
        self.per_task.get(index_of(task)).copied().unwrap_or(0)
    }

    /// Move one rate by `delta`, held inside the panel's range. The one
    /// write, so nothing can set a rate the panel could not have.
    pub fn step(&mut self, task: TaskType, delta: i64) -> i64 {
        let slot = index_of(task);
        let held = (self.of(task) + delta).clamp(0, RATE_MAX);
        if let Some(rate) = self.per_task.get_mut(slot) {
            *rate = held;
        }
        held
    }

    /// The rates as a stamp carries them: `rates:fight=24,labor=16,...`.
    pub fn stamp(&self) -> String {
        let body = RATES
            .iter()
            .map(|spec| format!("{}={}", spec.task.id(), self.of(spec.task)))
            .collect::<Vec<_>>()
            .join(",");
        format!("rates:{body}")
    }
}

/// A task type's row in the rate table.
fn index_of(task: TaskType) -> usize {
    RATES
        .iter()
        .position(|spec| spec.task == task)
        .unwrap_or_default()
}

/// Who a posting is addressed to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    /// Anybody who hears it — the bounty.
    Anyone,
    /// One named character — the contract.
    Person(usize),
}

/// What a posting asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum What {
    /// This job, by site and row.
    Job(JobId),
    /// Any open job at this site.
    Site(usize),
    /// Any open job of this kind, anywhere.
    Task(TaskType),
}

impl What {
    /// Whether this posting covers that job — the one matcher, so the ledger,
    /// the scorer's candidates and the answer all mean the same thing.
    pub fn covers(self, sim: &Sim, job: JobId) -> bool {
        let Some(quest) = sim
            .sites
            .get(job.site)
            .and_then(|site| site.quest(job.slot))
        else {
            return false;
        };
        match self {
            What::Job(named) => named == job,
            What::Site(site) => site == job.site,
            What::Task(task) => quest.task == task,
        }
    }

    /// The same through a lens, for the surfaces.
    pub fn label_through(self, lens: &crate::lens::Lens<'_>) -> String {
        match self {
            What::Job(job) => lens
                .site(job.site)
                .and_then(|board| board.quest(job.slot))
                .map_or_else(|| "that job".to_owned(), |quest| quest.name.to_owned()),
            What::Site(site) => format!(
                "any job at {}",
                crate::grid::LOCATIONS[sim::site_location(site)].name
            ),
            What::Task(task) => format!("any {} work", task.id()),
        }
    }

    /// How a ledger row and a feed line name it.
    pub fn label(self, sim: &Sim) -> String {
        match self {
            What::Job(job) => sim
                .sites
                .get(job.site)
                .and_then(|site| site.quest(job.slot))
                .map_or_else(|| "that job".to_owned(), |quest| quest.name.to_owned()),
            What::Site(site) => format!(
                "any job at {}",
                crate::grid::LOCATIONS[sim::site_location(site)].name
            ),
            What::Task(task) => format!("any {} work", task.id()),
        }
    }
}

/// How long a posting stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Until {
    /// One fulfilment closes it.
    Done,
    /// It stands, and keeps recruiting, until the player takes it down.
    Withdrawn,
}

impl Until {
    /// The word a ledger row prints.
    pub fn name(self) -> &'static str {
        match self {
            Until::Done => "done",
            Until::Withdrawn => "withdrawn",
        }
    }
}

/// What has become of a posting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Standing, and answerable.
    Open,
    /// Somebody agreed and the work is theirs (a `done` posting closes here).
    Filled,
    /// The player took it down.
    Withdrawn,
}

impl Status {
    /// The word a ledger row prints.
    pub fn name(self) -> &'static str {
        match self {
            Status::Open => "open",
            Status::Filled => "filled",
            Status::Withdrawn => "withdrawn",
        }
    }
}

/// What somebody said, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    /// They took it, and this is the job they took.
    Agreed {
        /// The row they claimed.
        job: JobId,
    },
    /// They did not, and this is the scorer's own reason for what they did
    /// instead.
    Declined {
        /// The words.
        reason: String,
    },
}

/// One posting, as the data format records it (the GDD's posting record).
#[derive(Clone, Debug)]
pub struct Posting {
    /// Its id — its index in the ledger, which is also the order it was made.
    pub id: usize,
    /// Who it is addressed to.
    pub who: Who,
    /// What it asks for.
    pub what: What,
    /// How long it stands.
    pub until: Until,
    /// What it pays per fulfilment, in gold.
    pub wage: i64,
    /// The standing rate for its work at the moment it was made — the
    /// expectation the payment is judged against (GDD §4.2).
    pub rate_at_posting: i64,
    /// The world-minute it was made at.
    pub made_at: u64,
    /// Who has heard it, and when.
    pub heard: Vec<(usize, u64)>,
    /// Who has answered it, and what they said.
    pub answered: Vec<(usize, Answer)>,
    /// What has become of it.
    pub status: Status,
}

impl Posting {
    /// Whether this character has heard it.
    pub fn heard_by(&self, who: usize) -> Option<u64> {
        self.heard
            .iter()
            .find(|(index, _)| *index == who)
            .map(|(_, minute)| *minute)
    }

    /// Whether it is addressed to this character by name.
    pub fn targets(&self, who: usize) -> bool {
        self.who == Who::Person(who)
    }

    /// Whether this character may weigh it: it is standing, they have heard
    /// it, and it is either theirs by name or open to anybody.
    pub fn open_to(&self, who: usize) -> bool {
        self.status == Status::Open
            && self.heard_by(who).is_some()
            && matches!(self.who, Who::Anyone | Who::Person(_) if self.who == Who::Anyone || self.targets(who))
    }

    /// **What is being asked for**, without who it is addressed to — what a
    /// feed line says when the sentence's own subject is the person it was
    /// addressed to, and repeating their name would be a stutter.
    pub fn terms_line(&self, lens: &crate::lens::Lens<'_>) -> String {
        format!(
            "{} - {}g - until {}",
            self.what.label_through(lens),
            self.wage,
            self.until.name()
        )
    }

    /// The one-line summary a ledger row and a feed sentence share.
    ///
    /// Takes a [`Lens`] rather than the `Sim`, because a ledger row is a
    /// screen: the names on it are read the way every other surface reads
    /// them (UI.md §6), and the sim's own callers hand it `Lens::on`.
    pub fn line(&self, lens: &crate::lens::Lens<'_>) -> String {
        let who = match self.who {
            Who::Anyone => "anyone".to_owned(),
            Who::Person(index) => lens.name(index).to_owned(),
        };
        format!(
            "{who} - {} - {}g - until {}",
            self.what.label_through(lens),
            self.wage,
            self.until.name()
        )
    }
}

/// The player's ledger: every posting ever made, in the order they were made.
///
/// **Sim state, and the only copy there is** — the ledger drawer is a view of
/// this vector (`ledger.rs`), so a posting the player can see and a posting
/// the scorer can weigh cannot be two different things.
#[derive(Clone, Debug, Default)]
pub struct Postings {
    rows: Vec<Posting>,
}

impl Postings {
    /// Every posting, oldest first.
    pub fn all(&self) -> &[Posting] {
        &self.rows
    }

    /// One posting by id.
    pub fn get(&self, id: usize) -> Option<&Posting> {
        self.rows.get(id)
    }

    /// Put a new posting on the ledger and hand back its id — the one write
    /// that makes a posting exist.
    pub fn push(&mut self, posting: Posting) -> usize {
        let id = self.rows.len();
        self.rows.push(posting);
        id
    }

    /// One posting, to write an answer, a hearing or a status onto.
    ///
    /// The ledger's rows are only ever changed through the functions in this
    /// module and `answers.rs`; this is how those functions reach a row,
    /// rather than the vector being public to the whole game.
    pub fn get_mut(&mut self, id: usize) -> Option<&mut Posting> {
        self.rows.get_mut(id)
    }

    /// Every posting matching a predicate, by id — how a caller walks the
    /// ledger before writing to it, without holding a borrow while it does.
    pub fn ids_where(&self, mut wanted: impl FnMut(&Posting) -> bool) -> Vec<usize> {
        self.rows
            .iter()
            .filter(|posting| wanted(posting))
            .map(|posting| posting.id)
            .collect()
    }

    /// Whether this job already has a standing posting on it — what stops one
    /// tap from making the same ask twice.
    pub fn already_posted(&self, sim: &Sim, who: Who, job: JobId) -> bool {
        self.rows.iter().any(|posting| {
            posting.status == Status::Open && posting.who == who && posting.what.covers(sim, job)
        })
    }
}

/// **The posting a job row is previewing** — the one a tap would make.
///
/// Not on the ledger and never written to it: it exists so the row's read and
/// the answer are literally the same shape of thing, weighed by the same
/// `answers::terms`. Its id is one no ledger row can have, so a preview that
/// leaked into the scorer would score nothing rather than score somebody
/// else's ask.
pub fn preview_posting(who: usize, site: usize, slot: usize, wage: i64, rate: i64) -> Posting {
    Posting {
        id: usize::MAX,
        who: Who::Person(who),
        what: What::Job(JobId { site, slot }),
        until: Until::Done,
        wage,
        rate_at_posting: rate,
        made_at: 0,
        heard: Vec::new(),
        answered: Vec::new(),
        status: Status::Open,
    }
}

/// **What the fit column means, and what it does not mean yet** — the board's
/// own chip explanation (wave 1.2's clarity rider).
///
/// Written like every other chip line: honest about the wave it is in. Fit
/// weighs into whether somebody agrees today; whether the *work* goes well is
/// resolution's, and resolution is wave 1.4.
pub fn fit_means() -> String {
    "fit: their aptitude for this work. It sways whether they agree; every job succeeds \
     until resolution lands."
        .to_owned()
}

/// **What is being asked for** — the four fields of a posting the player
/// chooses, as one value.
///
/// One struct rather than four arguments because they travel together
/// everywhere: the board's tap, the standing-posting button and every staged
/// case in the batteries all name exactly these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Offer {
    /// Who it is addressed to.
    pub who: Who,
    /// What it asks for.
    pub what: What,
    /// How long it stands.
    pub until: Until,
    /// What it pays per fulfilment.
    pub wage: i64,
}

/// **Make a posting** — the player's whole verb (the GDD's postings section).
///
/// The wage defaults to the standing rate at the caller; the record keeps the
/// rate it was made against, because that is the expectation the payment is
/// judged by however the rates move afterwards. Hearing follows immediately:
/// in camp it is at once, away it rides a messenger.
pub fn post(sim: &mut Sim, grid: &Grid, tuning: &Tuning, now: u64, offer: Offer) -> Option<usize> {
    let Offer {
        who,
        what,
        until,
        wage,
    } = offer;
    if !sim.modules.enabled(MODULE) {
        return None;
    }
    let rate = rate_for(sim, what);
    let id = sim.postings.all().len();
    let posting = Posting {
        id,
        who,
        what,
        until,
        wage,
        rate_at_posting: rate,
        made_at: now,
        heard: Vec::new(),
        answered: Vec::new(),
        status: Status::Open,
    };
    let line = posting.line(&crate::lens::Lens::on(sim));
    sim.postings.push(posting);
    let tile = crate::grid::LOCATIONS[crate::grid::TOWN].tile;
    sim.emit_ask(
        now,
        crate::attention::EventClass::PostingMade,
        sim::PLAYER,
        tile,
        format!("posted: {line}"),
    );
    match who {
        // A named ask is carried to the person it names: in camp that is at
        // once, and abroad it is the messenger's ride.
        Who::Person(index) => hear(sim, grid, tuning, now, index, id),
        // A notice on the board is heard by whoever comes to read it, which
        // is everybody at home on their next rescore.
        Who::Anyone => {}
    }
    Some(id)
}

/// **Withdraw a posting** — instant on the ledger, so a posting taken down
/// before it was heard is one nobody ever answers.
pub fn withdraw(sim: &mut Sim, now: u64, id: usize) {
    if !sim.modules.enabled(MODULE) {
        return;
    }
    let Some(posting) = sim.postings.get_mut(id) else {
        return;
    };
    if posting.status != Status::Open {
        return;
    }
    posting.status = Status::Withdrawn;
    let Some(line) = sim
        .postings
        .get(id)
        .map(|posting| posting.terms_line(&crate::lens::Lens::on(sim)))
    else {
        return;
    };
    let tile = crate::grid::LOCATIONS[crate::grid::TOWN].tile;
    sim.emit_ask(
        now,
        crate::attention::EventClass::PostingWithdrawn,
        sim::PLAYER,
        tile,
        format!("withdrew: {line}"),
    );
}

/// The standing rate a posting for this work inherits.
///
/// A posting for one job or one site takes the rate of the work it names; a
/// task-type posting takes that type's own. A site holding several kinds of
/// work takes the rate of its first open row, because that is the job the
/// posting is most likely to be answered with.
pub fn rate_for(sim: &Sim, what: What) -> i64 {
    let task = match what {
        What::Task(task) => Some(task),
        What::Job(job) => sim
            .sites
            .get(job.site)
            .and_then(|site| site.quest(job.slot))
            .map(|quest| quest.task),
        What::Site(site) => sim.sites.get(site).and_then(|board| {
            board
                .open_slots()
                .next()
                .and_then(|slot| board.quest(slot))
                .map(|quest| quest.task)
        }),
    };
    task.map_or(0, |task| sim.rates.of(task))
}

/// **Somebody hears a posting.**
///
/// The hearing is recorded on the record and said in the feed; and because
/// being asked *is* being asked, a character standing idle at home weighs it
/// there and then, through the one scorer. Somebody who is out has heard it
/// and answers when they are next asked to think.
pub fn hear(sim: &mut Sim, grid: &Grid, tuning: &Tuning, now: u64, who: usize, id: usize) {
    let Some(posting) = sim.postings.get(id) else {
        return;
    };
    if posting.status != Status::Open || posting.heard_by(who).is_some() {
        return;
    }
    if sim
        .parties
        .get(who)
        .is_none_or(|party| party.activity != Activity::Idle)
    {
        // Away: the ask rides a messenger, and the hearing lands when they
        // next arrive somewhere (`deliver_on_arrival`).
        return;
    }
    if let Some(posting) = sim.postings.get_mut(id) {
        posting.heard.push((who, now));
    }
    let Some(line) = sim
        .postings
        .get(id)
        .map(|posting| posting.terms_line(&crate::lens::Lens::on(sim)))
    else {
        return;
    };
    let tile = sim
        .parties
        .get(who)
        .map_or(crate::grid::LOCATIONS[crate::grid::TOWN].tile, |party| {
            party.tile
        });
    sim.emit_ask(
        now,
        crate::attention::EventClass::AskHeard,
        who,
        tile,
        format!("heard the ask - {line}"),
    );
    // Asked, and standing at their own door: they answer now, through the
    // same rescore the cadence would have given them later.
    autonomy::rescore(sim, grid, tuning, now, who);
}

/// **The messenger arrives**: every targeted posting waiting on this
/// character is heard at the world-minute they reach wherever they were
/// going.
///
/// Called from the one journey's arrival, so the hearing has a world-time
/// address like every other occurrence and is speed-invariant by
/// construction. They answer at their next rescore, at home — a person on a
/// job does not turn around, and the scorer never asks somebody who is out.
pub fn deliver_on_arrival(sim: &mut Sim, now: u64, who: usize) {
    if !sim.modules.enabled(MODULE) {
        return;
    }
    let waiting = sim.postings.ids_where(|posting| {
        posting.status == Status::Open && posting.targets(who) && posting.heard_by(who).is_none()
    });
    for id in waiting {
        let Some(line) = sim
            .postings
            .get(id)
            .map(|posting| posting.terms_line(&crate::lens::Lens::on(sim)))
        else {
            continue;
        };
        if let Some(posting) = sim.postings.get_mut(id) {
            posting.heard.push((who, now));
        }
        let tile = sim
            .parties
            .get(who)
            .map_or(crate::grid::LOCATIONS[crate::grid::TOWN].tile, |party| {
                party.tile
            });
        sim.emit_ask(
            now,
            crate::attention::EventClass::AskHeard,
            who,
            tile,
            format!("heard the ask on the road - {line}"),
        );
    }
}

/// **The open board, read at camp**: a character at home hears every standing
/// open posting on their next rescore.
///
/// Called by the scorer's own cadence, before the candidates are built, so
/// what somebody weighs is exactly what they have heard.
pub fn hear_the_board(sim: &mut Sim, now: u64, who: usize) {
    if !sim.modules.enabled(MODULE) {
        return;
    }
    let unheard = sim.postings.ids_where(|posting| {
        posting.status == Status::Open
            && posting.who == Who::Anyone
            && posting.heard_by(who).is_none()
    });
    for id in unheard {
        let Some(line) = sim
            .postings
            .get(id)
            .map(|posting| posting.terms_line(&crate::lens::Lens::on(sim)))
        else {
            continue;
        };
        if let Some(posting) = sim.postings.get_mut(id) {
            posting.heard.push((who, now));
        }
        let tile = sim
            .parties
            .get(who)
            .map_or(crate::grid::LOCATIONS[crate::grid::TOWN].tile, |party| {
                party.tile
            });
        sim.emit_ask(
            now,
            crate::attention::EventClass::AskHeard,
            who,
            tile,
            format!("read the board - {line}"),
        );
    }
}
