//! **The petition record, and what happens to one** (GDD §5's petitions
//! module, wave 1.5) — raised, voiced, met or failed.
//!
//! # Obligation is voicing
//!
//! A petition binds the player at **delivery**: in the camp that is an
//! audience at once, and to somebody out on the road it is the messenger the
//! asks module already rides — the word reaches the player when the petitioner
//! next arrives anywhere, the way a posting reaches somebody abroad. The
//! deadline counts from that minute. There is no accept, no decline and no
//! promise (owner, 2026-10-02): the deadline does the rest.
//!
//! # Every step is an occurrence
//!
//! A check is `Occ::Plea` on the one scheduler; a cliff is `Occ::Deadline`; a
//! walk-out's return is `Occ::Return`; a messenger's delivery lands on the
//! journey's own arrival. Satisfaction is judged after every occurrence fires
//! (`sim::advance_to`) and after a gift — never per frame — so the same seed
//! and the same orders voice the same petitions with the same words, and every
//! speed script reaches the same verdicts at the same world-minutes.
//!
//! # Incidental credit stands
//!
//! A condition objectively met is met, with full credit to whoever met it —
//! the player's arrangement or not. Whether the petitioner has to *know* is
//! the knowledge lens's question and is not built.

use crate::attention::EventClass;
use crate::constants::Tuning;
use crate::petitions::{self, Condition, DAY, Declared, Reward, Source, Template, Trigger};
use crate::sim::{Activity, Sim};
use crate::stores::{GrudgeCause, Regarded};
use crate::traits::TaskType;

/// Where a petition is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Raised, and the messenger has not reached the player yet: it binds
    /// nobody.
    Waiting,
    /// Voiced, and the deadline is running.
    Active,
    /// Met, at this minute, with this credit.
    Met {
        /// When.
        at: u64,
        /// Whose hand was in it.
        credit: Credit,
    },
    /// The deadline came and it was not met; the declared consequence fired.
    Failed {
        /// When.
        at: u64,
    },
}

impl Status {
    /// Whether it is over.
    pub fn resolved(self) -> bool {
        matches!(self, Status::Met { .. } | Status::Failed { .. })
    }
}

/// **Whose hand met it.** The regard a satisfaction moves goes to this.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Credit {
    /// The player's: a gift, a posting the condition's subject answered, or a
    /// building put up after the ask.
    Player,
    /// Nobody asked: the world met it — their own work, their own luck. Full
    /// credit all the same; nobody's regard moves for it.
    Incidental,
}

/// One petition.
#[derive(Clone, Debug)]
pub struct Petition {
    /// Its index in the record.
    pub id: usize,
    /// **The template it was raised from** — a reference into the table, so
    /// the card and the cliff read one row.
    pub template: &'static Template,
    /// Who is asking.
    pub who: usize,
    /// The `{other}` slot: who they are asking on behalf of.
    pub other: Option<usize>,
    /// The `{site}` slot, by `Sim::sites` index.
    pub site: Option<usize>,
    /// The `{n}` slot.
    pub n: i64,
    /// When it was raised.
    pub raised_at: u64,
    /// When the player was told — the minute obligation binds.
    pub voiced_at: Option<u64>,
    /// The cliff: voiced plus the template's days. Meaningless before voicing.
    pub deadline: u64,
    /// **The resolved words**, written at voicing and never again.
    pub text: String,
    /// Where it is.
    pub status: Status,
    /// What the player has given toward it, in gold.
    pub gifted: i64,
    /// **The consequence that fired**, when one did — the battery asserts it
    /// is the template's own reference and not a copy of it.
    pub fired: Option<&'static Declared>,
    /// A walk-out declared while they were out on the road, waiting for them
    /// to come home: nobody walks out of a job half done.
    pub leaving: bool,
}

impl Petition {
    /// **The consequence this card declares** — the template's reference.
    pub fn declared(&self) -> &'static Declared {
        self.template.consequence
    }

    /// Whether the deadline is running.
    pub fn active(&self) -> bool {
        self.status == Status::Active
    }

    /// World-minutes left at `now`, while it is active.
    pub fn left(&self, now: u64) -> u64 {
        self.deadline.saturating_sub(now)
    }
}

/// **The petition ledger** — every petition ever raised, oldest first.
///
/// Sim state: a replay carries it, and the ledger drawer is a view of it and
/// nothing else. Private with named writes, the way `asks::Postings` is.
#[derive(Clone, Debug, Default)]
pub struct Petitions {
    list: Vec<Petition>,
}

impl Petitions {
    /// Every petition, oldest first.
    pub fn all(&self) -> &[Petition] {
        &self.list
    }

    /// One petition, by id.
    pub fn get(&self, id: usize) -> Option<&Petition> {
        self.list.get(id)
    }

    fn get_mut(&mut self, id: usize) -> Option<&mut Petition> {
        self.list.get_mut(id)
    }
}

/// Whether the module is on in this world.
fn on(sim: &Sim) -> bool {
    sim.modules.enabled(petitions::MODULE)
}

/// World-minutes between one character's petition checks.
pub fn interval(tuning: &Tuning) -> u64 {
    u64::try_from(tuning.plea_hours.max(1)).unwrap_or(1) * 60
}

/// When character `who` is first checked: the first template window, staggered
/// by roster index exactly as the scorer is.
pub fn first_check(tuning: &Tuning, who: usize) -> u64 {
    let stagger = u64::try_from(tuning.scorer_stagger.max(0)).unwrap_or(0);
    petitions::first_window() + stagger * u64::try_from(who).unwrap_or(0)
}

/// The thin-days window, in world-minutes.
pub fn thin_window(tuning: &Tuning) -> u64 {
    u64::try_from(tuning.thin_window.max(0)).unwrap_or(0) * DAY
}

/// **What a trigger finds** — the `{other}` and `{site}` it was raised with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Found {
    /// The `{other}` slot.
    pub other: Option<usize>,
    /// The `{site}` slot.
    pub site: Option<usize>,
}

/// **Whether a template's trigger holds** for this person now, and what it
/// found — the state predicate of GDD §6's trigger. The window and the roll
/// are [`check`]'s; this is the third part.
pub fn holds(
    sim: &Sim,
    tuning: &Tuning,
    now: u64,
    who: usize,
    template: &Template,
) -> Option<Found> {
    let person = sim.people.get(who)?;
    if let Some(motivator) = template.motivator()
        && !person.traits.contains(&motivator)
    {
        return None;
    }
    let here_for = |days: u64| now >= person.present_from + days * DAY;
    match template.trigger {
        Trigger::Chained => None,
        Trigger::PurseBelow => (person.wallet < template.n).then_some(Found::default()),
        Trigger::NoWorkFor { task, days } => {
            let recent = person
                .memory
                .last_worked(task)
                .is_some_and(|at| at + days * DAY > now);
            // **The bench needs a settlement that can be built**: with the
            // module off there is no industry to ask for (`CAST.md` §6).
            let buildable =
                task != TaskType::Craft || sim.modules.enabled(crate::settlement::MODULE);
            (!recent && here_for(days) && buildable).then(|| Found {
                other: None,
                site: richest(sim, task),
            })
        }
        Trigger::NotOutFor { days } => {
            let recent = person
                .memory
                .last_out
                .is_some_and(|at| at + days * DAY > now);
            let new = unvisited(sim, who)?;
            (!recent && here_for(days)).then_some(Found {
                other: None,
                site: Some(new),
            })
        }
        Trigger::SomeoneDesperate => {
            let lens = crate::lens::Lens::on(sim);
            lens.roll()
                .into_iter()
                .filter(|other| *other != who && crate::needs::is_desperate(&lens, *other))
                .max_by_key(|other| (lens.desperation(*other), std::cmp::Reverse(*other)))
                .map(|other| Found {
                    other: Some(other),
                    site: None,
                })
        }
        Trigger::Shortfalls { count } => {
            // **A shortfall answered is a shortfall spent**: the window opens
            // no earlier than the last time this template's petition for them
            // was met or failed, or the same three shortfalls would voice it
            // again at the very next check.
            let spent = sim
                .petitions
                .all()
                .iter()
                .filter(|petition| petition.who == who && petition.template.id == template.id)
                .filter_map(|petition| match petition.status {
                    Status::Met { at, .. } | Status::Failed { at } => Some(at + 1),
                    _ => None,
                })
                .max()
                .unwrap_or(0);
            let since = now.saturating_sub(thin_window(tuning)).max(spent);
            let short = sim
                .events
                .iter()
                .filter(|event| {
                    event.class == EventClass::UpkeepShortfall
                        && event.party == who
                        && event.minute >= since
                })
                .count();
            (short >= count).then_some(Found::default())
        }
    }
    .filter(|_| person.present)
}

/// The authored site with the richest job of this kind on its board, open or
/// not — where somebody who wants a name would ask to be sent.
fn richest(sim: &Sim, task: TaskType) -> Option<usize> {
    let mut best: Option<(i64, usize)> = None;
    for (index, site) in sim.sites.iter().enumerate() {
        if site.industry.is_some() {
            continue;
        }
        for quest in &site.quests {
            if quest.task == task && best.is_none_or(|(pot, _)| quest.pot > pot) {
                best = Some((quest.pot, index));
            }
        }
    }
    best.map(|(_, site)| site)
}

/// The first authored site, in registry order, this person has never reached.
fn unvisited(sim: &Sim, who: usize) -> Option<usize> {
    let person = sim.people.get(who)?;
    (0..sim.sites.len()).find(|site| {
        sim.sites
            .get(*site)
            .is_some_and(|board| board.industry.is_none())
            && !person.memory.visited.contains(site)
    })
}

/// **One character's petition check** — `Occ::Plea`, on the one scheduler.
///
/// Walks the table in order and raises the first template whose window is
/// open, whose trigger holds and, where it rolls, whose seeded roll passes.
/// Nobody who already carries a petition is checked: one active petition per
/// character, held by [`raise`] as well.
pub fn check(sim: &mut Sim, tuning: &Tuning, now: u64, who: usize) {
    if !on(sim) {
        return;
    }
    let Some(person) = sim.people.get(who) else {
        return;
    };
    if !person.present || person.active_petition.is_some() {
        return;
    }
    for (index, template) in petitions::TEMPLATES.iter().enumerate() {
        if !template.checked() || now < template.opens_at() {
            continue;
        }
        let Some(found) = holds(sim, tuning, now, who, template) else {
            continue;
        };
        if template.rolled && petitions::roll(sim.seed, now, who, index) >= tuning.plea_odds {
            continue;
        }
        raise(sim, tuning, now, who, template, found);
        return;
    }
}

/// **Raise a petition** — the one door into the record.
///
/// Refuses anybody who already carries one (one active petition per
/// character, enforced in the data path rather than by the check that usually
/// calls this), and voices it at once to somebody standing in the camp. To
/// somebody out on the road it waits for the messenger.
pub fn raise(
    sim: &mut Sim,
    tuning: &Tuning,
    now: u64,
    who: usize,
    template: &'static Template,
    found: Found,
) -> Option<usize> {
    if !on(sim) || template.source == Source::Director {
        return None;
    }
    let person = sim.people.get(who)?;
    if person.active_petition.is_some() || !person.present {
        return None;
    }
    let id = sim.petitions.list.len();
    sim.petitions.list.push(Petition {
        id,
        template,
        who,
        other: found.other,
        site: found.site,
        n: template.n,
        raised_at: now,
        voiced_at: None,
        deadline: 0,
        text: String::new(),
        status: Status::Waiting,
        gifted: 0,
        fired: None,
        leaving: false,
    });
    if let Some(person) = sim.people.get_mut(who) {
        person.active_petition = Some(id);
    }
    let home = sim
        .parties
        .get(who)
        .is_some_and(|party| party.activity == Activity::Idle);
    if home {
        voice(sim, tuning, now, id);
    }
    Some(id)
}

/// **The petition is put to the player** — obligation binds here.
fn voice(sim: &mut Sim, tuning: &Tuning, now: u64, id: usize) {
    let Some(petition) = sim.petitions.get(id) else {
        return;
    };
    if petition.status != Status::Waiting {
        return;
    }
    let days = petition.template.deadline_days;
    let deadline = now + days * DAY;
    let (who, tid) = (petition.who, petition.template.id);
    if let Some(petition) = sim.petitions.get_mut(id) {
        petition.voiced_at = Some(now);
        petition.deadline = deadline;
        petition.status = Status::Active;
    }
    let text = sim.petitions.get(id).map_or_else(String::new, |petition| {
        petitions::resolve(
            petition.template.text,
            &crate::lens::Lens::on(sim),
            petition,
        )
    });
    if let Some(petition) = sim.petitions.get_mut(id) {
        petition.text = text;
    }
    sim.schedule_deadline(deadline, id);
    let kind = sim
        .petitions
        .get(id)
        .map_or("sours", |petition| petition.declared().kind.id);
    let tile = tile_of(sim, who);
    sim.emit_petition(
        now,
        EventClass::PetitionVoiced,
        who,
        tile,
        format!(
            "petitioned you ({tid}) - due {}, or {kind}",
            crate::clock::stamp(deadline)
        ),
        id,
    );
    // **Met already** is met: a far-road asked by somebody already walking to
    // somewhere new is answered by the walk (incidental credit stands).
    settle_met(sim, tuning, now);
}

/// **The messenger arrives**: a petition raised while its petitioner was out
/// is voiced at the world-minute they reach wherever they were going — the
/// channel orders leave by, run the other way.
pub fn deliver(sim: &mut Sim, tuning: &Tuning, now: u64, who: usize) {
    if !on(sim) {
        return;
    }
    let waiting: Vec<usize> = sim
        .petitions
        .all()
        .iter()
        .filter(|petition| petition.who == who && petition.status == Status::Waiting)
        .map(|petition| petition.id)
        .collect();
    for id in waiting {
        voice(sim, tuning, now, id);
    }
}

/// **Home from the road**: the messenger catches them at their own door, and
/// a walk-out declared while they were out takes effect now.
pub fn came_home(sim: &mut Sim, tuning: &Tuning, now: u64, who: usize) {
    deliver(sim, tuning, now, who);
    let pending: Vec<(usize, u64)> = sim
        .petitions
        .all()
        .iter()
        .filter(|petition| petition.who == who && petition.leaving)
        .map(|petition| (petition.id, petition.declared().days))
        .collect();
    for (id, days) in pending {
        if let Some(petition) = sim.petitions.get_mut(id) {
            petition.leaving = false;
        }
        leave(sim, now, who, days);
    }
}

/// **Judge every running petition against the world as it now stands** —
/// called after every occurrence fires and after every gift.
pub fn settle_met(sim: &mut Sim, tuning: &Tuning, now: u64) {
    if !on(sim) {
        return;
    }
    let met: Vec<usize> = sim
        .petitions
        .all()
        .iter()
        .filter(|petition| {
            petition.active()
                && !petition.template.condition.at_deadline()
                && petition.template.condition.met(sim, petition)
        })
        .map(|petition| petition.id)
        .collect();
    for id in met {
        satisfy(sim, tuning, now, id);
    }
}

/// **The cliff** — `Occ::Deadline`. The predicate the card's "met when" line
/// was derived from is evaluated one last time; met is met, and otherwise the
/// declared consequence fires.
pub fn deadline(sim: &mut Sim, tuning: &Tuning, now: u64, id: usize) {
    let Some(petition) = sim.petitions.get(id) else {
        return;
    };
    if !petition.active() {
        return;
    }
    if petition.template.condition.met(sim, petition) {
        satisfy(sim, tuning, now, id);
    } else {
        fail(sim, tuning, now, id);
    }
}

/// **Whether the player's hand is in it** — a gift, a posting its subject
/// answered since it was voiced, or a building put up since that the
/// condition is about or that the meeting work was done at.
fn players_hand(sim: &Sim, petition: &Petition) -> bool {
    if petition.gifted > 0 {
        return true;
    }
    let Some(since) = petition.voiced_at else {
        return false;
    };
    let subject = petition.template.condition.subject(petition);
    let worked = sim
        .people
        .get(subject)
        .is_some_and(|person| person.memory.worked_since(since).any(|done| done.posted));
    let posted_now = sim
        .parties
        .get(subject)
        .is_some_and(|party| party.posting.is_some() && party.activity != Activity::Idle);
    // **A building put up since is the player's hand** where the condition
    // is the bench itself, or where the work that met it is a shift at what
    // was built: only the player builds, and work that exists because the
    // player made it exist is work the player arranged.
    let built = sim
        .events
        .iter()
        .any(|event| event.class == EventClass::Built && event.minute >= since);
    let shifted = sim
        .people
        .get(subject)
        .is_some_and(|person| person.memory.worked_since(since).any(|done| done.shift));
    let raised_work = built && (petition.template.condition == Condition::Bench || shifted);
    worked || posted_now || raised_work
}

/// **Met** — the escalation pipe's other end (`FINDINGS.md` G-033).
///
/// Desperation falls by the drawer's step and the source line is rewritten
/// to the event; regard rises toward whoever's hand was in it, which is the
/// player or nobody in this build (no character acts on another's petition
/// yet); a reward, where the template carries one, moves out of the
/// petitioner's purse to the satisfier; and a `next` link raises its next.
fn satisfy(sim: &mut Sim, tuning: &Tuning, now: u64, id: usize) {
    let Some(petition) = sim.petitions.get(id).cloned() else {
        return;
    };
    let credit = if players_hand(sim, &petition) {
        Credit::Player
    } else {
        Credit::Incidental
    };
    if let Some(record) = sim.petitions.get_mut(id) {
        record.status = Status::Met { at: now, credit };
    }
    let lens_name = |sim: &Sim, who: usize| sim.people.get(who).map_or("somebody", |p| p.name);
    let relieved = petition.template.relieved.replace(
        "{other}",
        petition.other.map_or("somebody", |who| lens_name(sim, who)),
    );
    let (before, after) = {
        let Some(person) = sim.people.get_mut(petition.who) else {
            return;
        };
        person.active_petition = None;
        let before = person.desperation;
        crate::people::press(person, -tuning.plea_relief);
        person.source = rewritten(person.origin, &relieved, now);
        (before, person.desperation)
    };
    if credit == Credit::Player {
        sim.shared
            .adjust_regard(tuning, petition.who, Regarded::Player, tuning.plea_regard);
    }
    let mut paid = 0;
    if let Reward::Gold(owed) = petition.template.reward
        && credit == Credit::Player
        && let Some(person) = sim.people.get_mut(petition.who)
    {
        paid = owed.clamp(0, person.wallet.max(0));
        person.wallet -= paid;
        sim.treasury += paid;
        sim.ports.transferred_rewards += paid;
    }
    let tile = tile_of(sim, petition.who);
    let credited = match credit {
        Credit::Player if petition.gifted > 0 => format!("your gift of {}g", petition.gifted),
        Credit::Player => "your doing".to_owned(),
        Credit::Incidental => "nobody's doing but the world's".to_owned(),
    };
    let reward = if paid > 0 {
        format!(", {paid}g to you")
    } else {
        String::new()
    };
    sim.emit_petition(
        now,
        EventClass::PetitionSatisfied,
        petition.who,
        tile,
        format!(
            "{relieved} - {} met, {credited}; desperation {before} -> {after}{reward}",
            petition.template.id
        ),
        id,
    );
    if let Some(next) = petition.template.next_on_met.and_then(petitions::find) {
        raise(
            sim,
            tuning,
            now,
            petition.who,
            next,
            Found {
                other: petition.other,
                site: petition.site,
            },
        );
    }
}

/// **Failed** — the declared consequence fires: the template's own reference,
/// and every consequence sours as well unless it *is* `sours`.
fn fail(sim: &mut Sim, tuning: &Tuning, now: u64, id: usize) {
    let Some(petition) = sim.petitions.get(id).cloned() else {
        return;
    };
    // **The repeat and the egregious**, GDD §4.3's grudge rule: they have
    // been failed before, or this was already the second ask of an arc.
    let repeat = sim.petitions.all().iter().any(|other| {
        other.who == petition.who && other.id != id && matches!(other.status, Status::Failed { .. })
    });
    let egregious = petition.template.trigger == Trigger::Chained;
    let declared = petition.declared();
    if let Some(record) = sim.petitions.get_mut(id) {
        record.status = Status::Failed { at: now };
        record.fired = Some(declared);
    }
    let effects = fire(sim, tuning, now, &petition, declared, repeat || egregious);
    let tile = tile_of(sim, petition.who);
    let broken = petition.template.broken.replace(
        "{other}",
        petition
            .other
            .and_then(|who| sim.people.get(who))
            .map_or("somebody", |person| person.name),
    );
    sim.emit_petition(
        now,
        EventClass::PetitionFailed,
        petition.who,
        tile,
        format!("{broken} - {} fired: {effects}", declared.kind.id),
        id,
    );
    if let Some(next) = petition.template.next_on_fail.and_then(petitions::find) {
        raise(
            sim,
            tuning,
            now,
            petition.who,
            next,
            Found {
                other: petition.other,
                site: petition.site,
            },
        );
    }
}

/// **Fire a declared consequence**, field by field — and say what it did.
///
/// The vocabulary row decides; nothing here asks which kind it is looking at.
fn fire(
    sim: &mut Sim,
    tuning: &Tuning,
    now: u64,
    petition: &Petition,
    declared: &'static Declared,
    grudge: bool,
) -> String {
    let who = petition.who;
    let kind = declared.kind;
    let mut said: Vec<String> = Vec::new();
    if let Some(person) = sim.people.get_mut(who) {
        person.active_petition = None;
    }
    if kind.burns_purse
        && let Some(person) = sim.people.get_mut(who)
    {
        let burned = person.wallet.max(0);
        person.wallet -= burned;
        sim.ports.burned_consequences += burned;
        said.push(format!("{burned}g burned"));
    }
    if kind.gives_half
        && let Some(other) = petition.other
        && other != who
    {
        let half = sim
            .people
            .get(who)
            .map_or(0, |person| person.wallet.max(0) / 2);
        if let Some(person) = sim.people.get_mut(who) {
            person.wallet -= half;
        }
        if let Some(person) = sim.people.get_mut(other) {
            person.wallet += half;
        }
        sim.ports.transferred_given += half;
        let to = sim
            .people
            .get(other)
            .map_or("somebody", |person| person.name);
        said.push(format!("{half}g to {to}"));
    }
    if declared.pressed() != 0
        && let Some(person) = sim.people.get_mut(who)
    {
        crate::people::press(person, declared.pressed());
        said.push(format!("desperation +{}", declared.pressed()));
    }
    if kind.rewrites
        && let Some(person) = sim.people.get_mut(who)
    {
        let line = petition.template.broken;
        person.source = rewritten(person.origin, line, now);
    }
    // **Sours**, always: obligation was voiced, and failing it costs regard
    // toward the player and nobody else (GDD §4.2).
    sim.shared
        .adjust_regard(tuning, who, Regarded::Player, -tuning.plea_regard);
    said.push(format!("regard -{}", tuning.plea_regard));
    if grudge {
        sim.shared
            .record_grudge(tuning, who, Regarded::Player, GrudgeCause::PetitionFailed);
        said.push("a grudge".to_owned());
    }
    if kind.leaves {
        let abroad = sim
            .parties
            .get(who)
            .is_some_and(|party| party.activity != Activity::Idle);
        if abroad {
            if let Some(record) = sim.petitions.get_mut(petition.id) {
                record.leaving = true;
            }
            said.push(format!("gone {} days once back", declared.days));
        } else {
            leave(sim, now, who, declared.days);
            said.push(format!("gone {} days, unpaid", declared.days));
        }
    }
    said.join(", ")
}

/// **They leave the camp** — the away-state `walks-out` declares.
///
/// Not a party: presence drops exactly as the staged start's `present_from`
/// holds it before an arrival, so the map, the roster, the picker and the
/// work list follow through the lens with no edit of their own, and nobody
/// absent is rescored, checked or charged upkeep.
fn leave(sim: &mut Sim, now: u64, who: usize, days: u64) {
    if let Some(person) = sim.people.get_mut(who) {
        person.present = false;
    }
    sim.schedule_return(now + days * DAY, who);
}

/// **They come back** — `Occ::Return`. Unpaid: nothing was earned while they
/// were gone.
pub fn come_back(sim: &mut Sim, now: u64, who: usize) {
    let Some(person) = sim.people.get_mut(who) else {
        return;
    };
    if person.present {
        return;
    }
    person.present = true;
    let home = person.home;
    if let Some(party) = sim.parties.get_mut(who) {
        party.tile = home;
    }
    sim.emit_joined(now, who, home, "came back to Kawaza, unpaid".to_owned());
}

/// Why a gift was refused — surfaced as a toast, never silent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GiftRefusal {
    /// The petitions module is off.
    Off,
    /// The petition is not running.
    NotActive,
    /// Its condition is not something money answers.
    NotWalletShaped,
    /// The treasury cannot pay it.
    Poor {
        /// What the treasury holds.
        held: i64,
        /// What the gift is.
        gift: i64,
    },
}

impl GiftRefusal {
    /// The sentence the disabled control and the toast both say.
    pub fn message(self) -> String {
        match self {
            GiftRefusal::Off => "petitions are off - there is nothing to give to".to_owned(),
            GiftRefusal::NotActive => "that petition is not asking any more".to_owned(),
            GiftRefusal::NotWalletShaped => "money does not answer this one".to_owned(),
            GiftRefusal::Poor { held, gift } => {
                format!("the treasury holds {held}g and the gift is {gift}g")
            }
        }
    }
}

/// **What a gift to this petition is, and whether it can be made** — the one
/// answer the GIVE control draws and the gift performs.
pub fn gift(sim: &Sim, id: usize) -> Result<i64, GiftRefusal> {
    if !on(sim) {
        return Err(GiftRefusal::Off);
    }
    let Some(petition) = sim.petitions.get(id) else {
        return Err(GiftRefusal::NotActive);
    };
    if !petition.active() {
        return Err(GiftRefusal::NotActive);
    }
    if !petition.template.condition.wallet_shaped() {
        return Err(GiftRefusal::NotWalletShaped);
    }
    let amount = petition.n;
    if sim.treasury < amount {
        return Err(GiftRefusal::Poor {
            held: sim.treasury,
            gift: amount,
        });
    }
    Ok(amount)
}

/// **Give** — the §4.1 petition-gift TRANSFER, treasury to the petitioner's
/// purse: the expensive direct answer, money instead of attention.
///
/// Debits exactly what [`gift`] says, and then the condition is judged through
/// the same predicate as ever — a gift does not satisfy a petition, the purse
/// it fills does.
pub fn give(sim: &mut Sim, tuning: &Tuning, now: u64, id: usize) -> Result<i64, GiftRefusal> {
    let amount = gift(sim, id)?;
    let who = sim.petitions.get(id).map_or(0, |petition| petition.who);
    sim.treasury -= amount;
    if let Some(person) = sim.people.get_mut(who) {
        person.wallet += amount;
    }
    sim.ports.transferred_gifts += amount;
    if let Some(petition) = sim.petitions.get_mut(id) {
        petition.gifted += amount;
    }
    settle_met(sim, tuning, now);
    Ok(amount)
}

/// **The source line a petition writes** (GDD §3), composed onto the line
/// they were generated with, like the shortfall's.
pub fn rewritten(origin: &str, event: &str, now: u64) -> String {
    format!("{origin} - {event} (day {})", now / DAY + 1)
}

/// Where somebody is standing, for an event's place.
fn tile_of(sim: &Sim, who: usize) -> crate::grid::Tile {
    sim.parties.get(who).map_or_else(
        || crate::grid::LOCATIONS[crate::grid::TOWN].tile,
        |party| party.tile,
    )
}
