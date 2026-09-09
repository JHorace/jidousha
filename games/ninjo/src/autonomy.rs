//! **The scorer** — autonomy's one decision function (GDD §5, wave 1.1).
//!
//! # One function, one question
//!
//! Characters live their own lives; the world is one the player overrides, not
//! a menu waiting for input. [`choose`] is the whole of that: candidate
//! actions in, one chosen action and the words for why out. **Nothing else in
//! the game decides what somebody does**, and no surface computes a second
//! answer — the character panel, the roster and the `action-started` event all
//! read the string this function wrote onto the party.
//!
//! # A second caller is a parameter
//!
//! [`choose`] takes the candidates rather than building them, because wave 2's
//! asks inject a heavily-weighted candidate into *this* function. An ask is
//! one more [`Action`] in the slice; the compliance ladder is what the scorer
//! does with it. Building the list is [`candidates`], and it is the caller's.
//!
//! # No term branches on a trait id
//!
//! Every term multiplies a row in: desperation opens the sum, a want's
//! `pressure` applies where its `favors` field covers the candidate's task
//! type, an aptitude is the row whose id *is* the task type, the pot pulls by
//! `pot_affinity`, and regard is weighed by the bond and grudge multipliers the
//! carrier happens to hold. The neutrality rule (`traits.rs`) is what makes
//! that safe: a row that does not own a field holds the value that drops out.
//! Ask "is this character greedy" anywhere below and the vocabulary stops
//! being data.
//!
//! # Degrades to
//!
//! With the module off nothing is scheduled at all (`Sim::opening`), so the
//! world is wave 0b's: everyone idles at home until the player says otherwise.

use jidousha::prelude::Key;

use crate::constants::Tuning;
use crate::grid::Grid;
use crate::sim::{self, Activity, Sim};
use crate::stores::Regarded;
use crate::traits::{self, TaskType};

/// The module id, as `modules::MODULES` and every stamp spell it.
pub const MODULE: &str = "autonomy";

/// How long a world-day is, in world-minutes — the unit `alive_days` and the
/// clock readout both count in.
pub const DAY: u64 = 1440;

/// One thing a character could do next.
///
/// Wave 1.1's three. An ask (wave 2) is a fourth variant and a fourth arm of
/// [`weigh`]; nothing else changes, which is the point of the shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Claim **this job** at this site and go.
    ///
    /// One candidate per open job, not one per site: a site's rows are six
    /// different pieces of work, and weighing only the front of the list made
    /// a crafter standing behind a fight job stay home. What the player sees
    /// on the board and what the scorer weighs are now the same thing.
    SeekWork {
        /// Which job, by site and row (`sim::JobId`).
        job: sim::JobId,
    },
    /// Walk to another character's home tile and stay a while.
    Socialize {
        /// Whose doorstep.
        toward: usize,
    },
    /// **Answer a posting** by taking this job for its wage (wave 1.2).
    ///
    /// The fourth variant this file was written expecting: an ask is one more
    /// candidate in the slice, weighed beside everything else the character
    /// could do, and agreeing goes out through the same dispatch. What makes
    /// it heavy is its terms, not a branch (`answers::terms`).
    Answer {
        /// Which posting, by its id on the ledger.
        posting: usize,
        /// Which job it would be answered with.
        job: sim::JobId,
    },
    /// Stay home. The floor every other candidate has to beat.
    Idle,
}

/// **What produced a term** — the row, or the fact.
///
/// The attribution a breakdown prints, and the whole reason it cannot rot:
/// where a trait row is what made a term, the cause carries the row's *id*
/// and the name is read off the row at the moment it is shown. Rename
/// `laborer` and the breakdown renames its line with nothing else edited,
/// which a hand-written string per trait could not do — and which
/// `traits::vocabulary` is not what asserts it: `compliance::attribution_is_derived`
/// renames a row in a staged vocabulary and reads the line back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cause {
    /// A fact or a state of the world: desperation, a wage, being asked by
    /// name. Nothing about the carrier's vocabulary produced it.
    Fact(String),
    /// These trait rows produced it, named by their own display names.
    ///
    /// Never empty — a term with no row behind it is a [`Cause::Fact`].
    Rows(Vec<traits::TraitId>),
}

impl Cause {
    /// The attribution, as a breakdown prints it — **read off the rows now**,
    /// never stored.
    pub fn phrase(&self) -> String {
        match self {
            Cause::Fact(what) => what.clone(),
            Cause::Rows(rows) => rows
                .iter()
                .map(|id| id.def().name)
                .collect::<Vec<_>>()
                .join(" and "),
        }
    }

    /// A fact, said in a `&'static str`'s worth of words.
    pub fn fact(what: &str) -> Self {
        Cause::Fact(what.to_owned())
    }
}

/// One term of a candidate's sum: what it was worth, what produced it, and
/// the words for it.
///
/// The reason a character gives is the largest positive term's own sentence —
/// the verdict-plus-reasons shape giri proved, with the arithmetic beside the
/// words so a surface can show either and neither can lie. **Since the
/// legibility session the arithmetic is shown**: `cause` is what lets a
/// breakdown say which of this person's traits moved which number, and
/// `words` still collapses the lot to one sentence for the places that have
/// room for one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Term {
    /// What the term is, as a report names it.
    pub what: &'static str,
    /// What it added (or, negative, took away).
    pub value: i64,
    /// **What produced it** — the rows, or the fact.
    pub cause: Cause,
    /// The half-sentence a reason is built from.
    pub because: String,
}

impl Term {
    /// One line of a breakdown: what it was worth, and what produced it.
    ///
    /// One formatter, so the board's band and the feed's band cannot print
    /// one sum two ways.
    pub fn line(&self) -> String {
        format!(
            "{}{} {} - {}",
            if self.value < 0 { "" } else { "+" },
            self.value,
            self.what,
            self.cause.phrase()
        )
    }
}

/// **The arithmetic behind one decision, kept** — what a breakdown shows
/// (UI.md §3e).
///
/// The scorer already returned every term; until the legibility session
/// everything but the loudest one was thrown away, and "why did Ludo go
/// there" was unanswerable ten minutes later. This is that return value,
/// recorded where the decision was recorded, and it is **the same
/// [`Judged`]** the decision was made from rather than a second reckoning of
/// it: a breakdown that could disagree with the decision is the failure this
/// surface is most able to cause.
///
/// It is a record and not an input. Nothing in the simulation reads it, no
/// arithmetic depends on it, and a run that never opens a breakdown is
/// byte-identical to one that opens every one of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reckoning {
    /// What was chosen, in words, **as it read at the moment of choosing** —
    /// the board it names may have been claimed by somebody else since.
    pub chose: String,
    /// What it scored.
    pub score: i64,
    /// Every term of that score, in the order they were weighed.
    pub terms: Vec<Term>,
    /// What beat it, and by what score — filled where this is a reading of an
    /// offer that was refused, and `None` for a decision that was taken.
    pub beaten_by: Option<(String, i64)>,
}

impl Reckoning {
    /// The sum, added up from the terms rather than trusted: a total that
    /// does not equal its own terms is the one thing a breakdown must not be
    /// able to print (`verify::the_breakdown_is_the_judgement`).
    pub fn total(&self) -> i64 {
        self.terms.iter().map(|term| term.value).sum()
    }
}

/// What the scorer decided, and why.
#[derive(Clone, Debug)]
pub struct Judged {
    /// What they will do.
    pub action: Action,
    /// What it scored.
    pub score: i64,
    /// Every term of that score, in the order they were weighed.
    pub terms: Vec<Term>,
    /// The words — what the event carries and the panel shows.
    pub reason: String,
}

/// World-minutes between one character's rescorings.
pub fn interval(tuning: &Tuning) -> u64 {
    u64::try_from(tuning.scorer_hours.max(1)).unwrap_or(1) * 60
}

/// When character `who` is first weighed: one interval in, staggered by their
/// roster index so ten people do not all decide in the same tick.
///
/// Deterministic by construction — the stagger is an index, never a roll.
pub fn first_score(tuning: &Tuning, who: usize) -> u64 {
    let stagger = u64::try_from(tuning.scorer_stagger.max(0)).unwrap_or(0);
    interval(tuning) + stagger * u64::try_from(who).unwrap_or(0)
}

/// How long a job's rest lasts, in world-minutes.
pub fn rest_minutes(tuning: &Tuning) -> u64 {
    u64::try_from(tuning.rest_hours.max(0)).unwrap_or(0) * 60
}

/// The window the alive sweep gives everybody to take a job, in world-minutes.
pub fn alive_window(tuning: &Tuning) -> u64 {
    u64::try_from(tuning.alive_days.max(0)).unwrap_or(0) * DAY
}

/// How long a visit lasts, in world-minutes.
pub fn visit_minutes(tuning: &Tuning) -> u64 {
    u64::try_from(tuning.visit_minutes.max(0)).unwrap_or(0)
}

/// The candidates open to `who` right now.
///
/// **Built by the caller, weighed by the scorer.** Every **open job** at every
/// site, every other character standing at their own door, and staying home.
/// Wave 2 appends its ask to whatever this returns.
///
/// One candidate per job is what makes an aptitude mean something: the board
/// leans by fiction, so the Deep Cave's front row is a haul and its fourth is
/// the shoring, and a site-shaped candidate offered the crafter the haul or
/// nothing.
pub fn candidates(sim: &Sim, who: usize) -> Vec<Action> {
    let mut out = vec![Action::Idle];
    for (site, spec) in sim.sites.iter().enumerate() {
        for slot in spec.open_slots() {
            out.push(Action::SeekWork {
                job: sim::JobId { site, slot },
            });
        }
    }
    for (other, party) in sim.parties.iter().enumerate() {
        if other != who && party.activity == Activity::Idle {
            out.push(Action::Socialize { toward: other });
        }
    }
    out
}

/// What one candidate is worth to `who`, and every term of it.
///
/// Integer arithmetic throughout, like every other number in this game: a
/// choice has to be exactly reproducible on replay, and a float would make the
/// transcript a claim about rounding.
pub fn weigh(sim: &Sim, tuning: &Tuning, now: u64, who: usize, action: Action) -> Vec<Term> {
    let Some(person) = sim.people.get(who) else {
        return Vec::new();
    };
    let mut terms: Vec<Term> = Vec::new();
    match action {
        Action::Idle => terms.push(Term {
            what: "idle",
            value: tuning.idle_floor,
            cause: Cause::fact("the floor every errand has to beat"),
            because: "nothing worth leaving for".to_owned(),
        }),
        Action::SeekWork { job } => {
            let Some(quest) = sim
                .sites
                .get(job.site)
                .and_then(|site| site.quest(job.slot))
                .copied()
            else {
                return terms;
            };
            let where_to = crate::grid::LOCATIONS[sim::site_location(job.site)].name;
            // Desperation opens the sum, as it opened giri's willingness.
            terms.push(Term {
                what: "need",
                value: person.desperation * tuning.need_weight,
                cause: Cause::fact(&format!("desperation {}", person.desperation)),
                because: "needs the money".to_owned(),
            });
            // A want's pressure, where its `favors` field covers this work.
            let pressure = traits::pressure_toward(quest.task, &person.traits);
            if pressure != 0 {
                let rows = traits::wanting(quest.task, &person.traits);
                let named = Cause::Rows(rows.clone()).phrase();
                terms.push(Term {
                    what: "want",
                    value: pressure * tuning.want_weight,
                    cause: Cause::Rows(rows),
                    because: format!("{named}, and this is {} work", quest.task.id()),
                });
            }
            // The aptitude row whose id is the task's id.
            let apt = traits::competence_at(quest.task, &person.traits);
            if apt != 0 {
                terms.push(Term {
                    what: "aptitude",
                    value: apt * tuning.apt_weight,
                    cause: Cause::Rows(vec![quest.task.aptitude()]),
                    because: format!("good at {} work", quest.task.id()),
                });
            }
            // The pot's pull, per ten gold, by the carrier's own affinity.
            let pull = traits::pot_pull_of(&person.traits);
            if pull != 0 {
                terms.push(Term {
                    what: "pot",
                    value: pull * quest.pot * tuning.pot_weight / 10,
                    cause: Cause::Rows(traits::drawn_by_a_pot(&person.traits)),
                    because: format!("the pot is {}g", quest.pot),
                });
            }
            // The rest term: nobody works forever.
            if now < sim.parties.get(who).map_or(0, |party| party.rested_until) {
                terms.push(Term {
                    what: "rest",
                    value: -tuning.rest_weight,
                    cause: Cause::fact("not stopped since the last job"),
                    because: "not stopped since the last job".to_owned(),
                });
            }
            let _ = where_to;
        }
        // **The ask's own terms**, which are the asks module's (GDD §5: no
        // module reads another's interior — the posting is shared sim state
        // like the quest board, and the arithmetic over it lives with the
        // module that owns the record).
        Action::Answer { posting, job } => {
            let Some(posting) = sim.postings.get(posting) else {
                return terms;
            };
            return crate::answers::terms(sim, tuning, now, who, posting, job);
        }
        Action::Socialize { toward } => {
            let felt = traits::weighted_regard(
                sim.shared.regard(who, Regarded::Person(toward)),
                &person.traits,
            );
            let host = sim.people.get(toward).map_or("somebody", |who| who.name);
            let raw = sim.shared.regard(who, Regarded::Person(toward));
            terms.push(Term {
                what: "regard",
                value: felt * tuning.regard_weight,
                cause: cause_of_regard(raw, &person.traits, &format!("what they make of {host}")),
                because: format!("thinks well of {host}"),
            });
        }
    }
    terms
}

/// **Which rows moved a regard term** — the carrier's own multipliers, on the
/// side of the ledger this regard is on.
///
/// Derived from the fields, like everything else: a row whose bond (or grudge,
/// where the regard is negative) multiplier is not the neutral one is a row
/// that changed this number, and a term with no such row behind it is the
/// bare fact of what somebody thinks. Nothing here asks which trait it is.
pub fn cause_of_regard(raw: i64, carried: &[traits::TraitId], fact: &str) -> Cause {
    let rows = traits::weighing_regard(raw, carried);
    if rows.is_empty() {
        Cause::fact(fact)
    } else {
        Cause::Rows(rows)
    }
}

/// **What an action is, in words** — what a breakdown's heading says, and
/// what the line naming the candidate that beat an offer says.
///
/// Reads the world it is given and decides nothing. A [`Reckoning`] stores
/// the sentence rather than the action, because a job named at the minute it
/// was weighed may be somebody else's by the time anybody reads the record.
pub fn describe(sim: &Sim, action: Action) -> String {
    let job_name = |job: sim::JobId| {
        let where_ = crate::grid::LOCATIONS[sim::site_location(job.site)].name;
        let what = sim
            .sites
            .get(job.site)
            .and_then(|site| site.quest(job.slot))
            .map_or("work", |quest| quest.name);
        format!("{what} at {where_}")
    };
    match action {
        Action::Idle => "staying home".to_owned(),
        Action::SeekWork { job } => job_name(job),
        Action::Answer { job, .. } => format!("the posting for {}", job_name(job)),
        Action::Socialize { toward } => format!(
            "visiting {}",
            sim.people.get(toward).map_or("somebody", |who| who.name)
        ),
    }
}

/// **The one decision function**: the best of these candidates, and the words.
///
/// Ties go to the earlier candidate, and [`candidates`] puts [`Action::Idle`]
/// first — so a world where nothing is compelling is a world where everybody
/// stays home, deterministically.
pub fn choose(sim: &Sim, tuning: &Tuning, now: u64, who: usize, open: &[Action]) -> Judged {
    let mut best = Judged {
        action: Action::Idle,
        score: tuning.idle_floor,
        terms: Vec::new(),
        reason: "nothing worth leaving for".to_owned(),
    };
    let mut first = true;
    for action in open.iter().copied() {
        let terms = weigh(sim, tuning, now, who, action);
        let score: i64 = terms.iter().map(|term| term.value).sum();
        if first || score > best.score {
            best = Judged {
                action,
                score,
                reason: words(action, &terms),
                terms,
            };
            first = false;
        }
    }
    best
}

/// The words: the **first** strictly-largest positive term's own sentence.
///
/// Public because a job row's preview says why in the same words the decision
/// would (`answers::read`) — one sentence-maker, so a row cannot describe a
/// choice differently from the feed line that reports it.
///
/// First rather than last, so a tie reads as the term the sum opened with —
/// desperation, as it did in giri — and so the sentence is a function of the
/// term order rather than of an iterator's tie-breaking.
pub fn words(action: Action, terms: &[Term]) -> String {
    let mut loudest: Option<&Term> = None;
    for term in terms.iter().filter(|term| term.value > 0) {
        if loudest.is_none_or(|best| term.value > best.value) {
            loudest = Some(term);
        }
    }
    match loudest {
        Some(term) => term.because.clone(),
        None => match action {
            Action::Idle => "nothing worth leaving for".to_owned(),
            _ => "nothing better to do".to_owned(),
        },
    }
}

/// **The scorer's own return, as a record** — one constructor, called
/// wherever a decision is kept, so a breakdown is the decision rather than a
/// second reading of it.
pub fn reckon(sim: &Sim, judged: &Judged) -> Reckoning {
    Reckoning {
        chose: describe(sim, judged.action),
        score: judged.score,
        terms: judged.terms.clone(),
        beaten_by: None,
    }
}

/// One character's turn to weigh what to do, fired by the one scheduler.
///
/// **Nobody who is out is rescored**: a character on a job — the player's or
/// their own — is not asked again until they are home, so the scorer can never
/// countermand an order that is already being walked.
pub fn rescore(sim: &mut Sim, grid: &Grid, tuning: &Tuning, now: u64, who: usize) {
    if !sim.modules.enabled(MODULE) {
        return;
    }
    if sim
        .parties
        .get(who)
        .is_none_or(|party| party.activity != Activity::Idle)
    {
        return;
    }
    // **The board is read before it is weighed** (wave 1.2): an open posting
    // is heard at camp, and what somebody weighs is exactly what they have
    // heard.
    crate::asks::hear_the_board(sim, now, who);
    let mut open = candidates(sim, who);
    open.extend(crate::answers::candidates(sim, who));
    let judged = choose(sim, tuning, now, who, &open);
    let taken = match judged.action {
        Action::Answer { posting, .. } => Some(posting),
        _ => None,
    };
    let reason = judged.reason.clone();
    // What they did instead, with its arithmetic — the same sum, so a refusal
    // in the feed can be opened and answers "why not" with what won.
    let instead = reckon(sim, &judged);
    act(sim, grid, tuning, now, who, judged);
    // **A named ask they did not take is a refusal, and a refusal is said.**
    // One place, so the answer is the same whether the rescore was the
    // cadence's or an ask's own arrival.
    crate::answers::record_refusals(sim, now, who, taken, &reason, &instead);
}

/// Carry out what the scorer chose — through the player's own dispatch loop.
///
/// The `action-started` event is emitted **here and only here**: it is the
/// decision's own class, and the journey that follows tells its story in the
/// five movement classes the substrate already had. One story per movement,
/// not two.
pub fn act(sim: &mut Sim, grid: &Grid, tuning: &Tuning, now: u64, who: usize, judged: Judged) {
    let tile = sim.parties.get(who).map_or_else(
        || crate::grid::LOCATIONS[crate::grid::TOWN].tile,
        |party| party.tile,
    );
    // **The sum this decision was made from, kept beside the decision**
    // (UI.md §3e). The scorer returned every term; until the legibility
    // session everything but the loudest was thrown away here, and the feed
    // could say what somebody did and never what it came to. Built before the
    // board it names can move.
    let reckoning = reckon(sim, &judged);
    match judged.action {
        // An idle choice is not an occurrence: nothing happened to anybody,
        // and the feed is for things that did (the drift's own precedent).
        Action::Idle => {}
        Action::SeekWork { job } => {
            let name = crate::grid::LOCATIONS[sim::site_location(job.site)].name;
            let quest = sim
                .sites
                .get(job.site)
                .and_then(|site| site.quest(job.slot))
                .map_or("work", |quest| quest.name);
            sim.emit_action(
                now,
                tile,
                who,
                format!("took {quest} at {name} - {}", judged.reason),
            );
            sim.remember(reckoning);
            let _ = sim::dispatch(
                sim,
                grid,
                tuning,
                now,
                who,
                job,
                sim::Motive::chose(judged.reason.clone()),
            );
        }
        Action::Answer { .. } => {
            crate::answers::agree(sim, grid, tuning, now, who, judged, reckoning);
        }
        Action::Socialize { toward } => {
            let host = sim.people.get(toward).map_or("somebody", |who| who.name);
            sim.emit_action(
                now,
                tile,
                who,
                format!("went to see {host} - {}", judged.reason),
            );
            sim.remember(reckoning);
            let _ = sim::call_on(
                sim,
                grid,
                tuning,
                now,
                who,
                toward,
                sim::Motive::chose(judged.reason.clone()),
            );
        }
    }
}

/// The scorer, judged at a stated constants set — **every expectation a
/// shipped literal**, never derived from `tuning`.
///
/// The mutation round runs this battery at moved constants to see whether the
/// constants are being measured at all (`mutation.rs`), so a check here that
/// recomputed its expectation from `tuning` would make its own constant
/// invisible. Everything is staged: no run is conducted, so this is cheap
/// enough to run thirty-four times.
pub fn judge_at(checks: &mut crate::checks::Checks, tuning: &Tuning) {
    fn judge(
        checks: &mut crate::checks::Checks,
        what: &'static str,
        got: i64,
        want: i64,
        why: &str,
    ) {
        checks.require(
            got == want,
            what,
            format!("{why}: the scorer answers {got} and the shipped set says {want}"),
        );
    }

    // The cadences, as world-minutes.
    judge(
        checks,
        "the scorer's cadence is not what the shipped set says",
        i64::try_from(interval(tuning)).unwrap_or(-1),
        240,
        "four world-hours between rescorings",
    );
    judge(
        checks,
        "the scorer's stagger is not what the shipped set says",
        i64::try_from(first_score(tuning, 3)).unwrap_or(-1),
        312,
        "the fourth of the roster is first weighed at 240 + 3 x 24",
    );
    judge(
        checks,
        "rest does not last what the shipped set says",
        i64::try_from(rest_minutes(tuning)).unwrap_or(-1),
        360,
        "six world-hours of rest after a job",
    );
    judge(
        checks,
        "a visit does not last what the shipped set says",
        i64::try_from(visit_minutes(tuning)).unwrap_or(-1),
        45,
        "forty-five world-minutes on somebody's doorstep",
    );
    judge(
        checks,
        "the alive sweep's window is not what the shipped set says",
        i64::try_from(alive_window(tuning)).unwrap_or(-1),
        4320,
        "three world-days of 1440 minutes",
    );

    let sim = Sim::opening(tuning, crate::modules::ModuleSet::ALL);
    let index = |id: &str| {
        sim.people
            .iter()
            .position(|person| person.id == id)
            .unwrap_or(0)
    };
    let (bob, steve, ludo, goro, odd, hana) = (
        index("bob"),
        index("steve"),
        index("ludo"),
        index("goro"),
        index("odd"),
        index("hana"),
    );
    let score = |sim: &Sim, now: u64, who: usize, action: Action| -> i64 {
        weigh(sim, tuning, now, who, action)
            .iter()
            .map(|term| term.value)
            .sum()
    };

    // **The terms, one staged sum at a time.** Ludo at the Deep Cave's labour
    // haul is the demo character `CAST.md` §4.1 names for this module: need 4
    // x 2, the indebted want 3 x 2 because its `favors` is any paid work, and
    // the labour aptitude 2 x 3.
    judge(
        checks,
        "the eager worker's pull toward open labour is not the shipped sum",
        score(
            &sim,
            0,
            ludo,
            Action::SeekWork {
                job: sim::JobId { site: 1, slot: 0 },
            },
        ),
        20,
        "desperation 4x2 + indebted 3x2 + labor 2x3",
    );
    // Bob at the Black Vault adds the pot's own pull, which only the greedy
    // feel: 80 gold at one affinity and a weight of one is eight.
    judge(
        checks,
        "the pot does not pull the greedy by the shipped weight",
        score(
            &sim,
            0,
            bob,
            Action::SeekWork {
                job: sim::JobId { site: 3, slot: 0 },
            },
        ),
        28,
        "desperation 4x2 + indebted 3x2 + fight 2x3 + pot 1x80x1/10",
    );
    // A want whose `favors` names another task adds nothing: Odd wants fight
    // work and the Deep Cave's haul is labour.
    judge(
        checks,
        "a want applied to work its favors field does not name",
        score(
            &sim,
            0,
            odd,
            Action::SeekWork {
                job: sim::JobId { site: 1, slot: 0 },
            },
        ),
        6,
        "desperation 3x2 and nothing else - renown wants fight work",
    );
    // Idling is the floor every candidate has to beat.
    judge(
        checks,
        "idling does not score the shipped floor",
        score(&sim, 0, ludo, Action::Idle),
        3,
        "the idle floor, and nothing else is added to it",
    );
    // The rest term, staged: somebody who just finished a job.
    let mut rested = sim.clone();
    rested.parties[ludo].rested_until = 100;
    judge(
        checks,
        "the rest term does not cost what the shipped set says",
        score(
            &rested,
            0,
            ludo,
            Action::SeekWork {
                job: sim::JobId { site: 1, slot: 0 },
            },
        ),
        8,
        "the same twenty, less the rest weight of twelve",
    );
    judge(
        checks,
        "the rest term outlasts the rest it is counting",
        score(
            &rested,
            100,
            ludo,
            Action::SeekWork {
                job: sim::JobId { site: 1, slot: 0 },
            },
        ),
        20,
        "at the minute rest ends the term is gone",
    );

    // **The authored preset** (`CAST.md` §5), read back through the stores:
    // the seeded warmth, the sibling bond, and the rivals' grudge.
    judge(
        checks,
        "the authored preset did not seed the regard CAST.md wants",
        sim.shared.regard(steve, Regarded::Person(bob)),
        3,
        "steve -> bob, small and positive",
    );
    checks.require(
        sim.shared.facts(hana, Regarded::Person(goro)).bond
            && sim.shared.facts(goro, Regarded::Person(odd)).grudge,
        "the authored preset did not seed the facts CAST.md wants",
        format!(
            "hana -> goro holds {:?} and goro -> odd holds {:?}; the siblings bond and the \
             rivals do not",
            sim.shared.facts(hana, Regarded::Person(goro)),
            sim.shared.facts(goro, Regarded::Person(odd))
        ),
    );
    // Socialising is regard, as the visitor's own personality weighs it:
    // Steve is loyal, so his warmth for Bob counts double before the weight.
    judge(
        checks,
        "a visit is not worth the regard the visitor feels for their host",
        score(&sim, 0, steve, Action::Socialize { toward: bob }),
        12,
        "regard 3, doubled by loyal, times the regard weight of 2",
    );

    // **The scorer passes on work that does not suit.** Alex is the cold
    // scout with the lowest desperation in the band, and a board of nothing
    // but fight work is a board he stays home for — which is what makes
    // "everybody took the first thing offered" a fact about a generous board
    // rather than about a scorer that cannot say no.
    let alex = index("alex");
    let mut fight_only = sim.clone();
    for site in &mut fight_only.sites {
        site.quests.retain(|quest| quest.task == TaskType::Fight);
        site.states = vec![crate::sim::JobState::Open; site.quests.len()];
    }
    checks.require(
        matches!(
            choose(&fight_only, tuning, 0, alex, &candidates(&fight_only, alex)).action,
            Action::Idle
        ),
        "the scorer takes work nobody in the sum wanted",
        format!(
            "Alex chose {:?} off a board of fight work only; his desperation is the lowest \
             in the band and no term of his covers a fight task",
            choose(&fight_only, tuning, 0, alex, &candidates(&fight_only, alex)).action
        ),
    );

    // **The crafter takes the craft job standing behind a fight job.** The
    // whole of what per-job candidates bought: with one candidate per *site*
    // the scorer only ever saw the front of the list, so a board whose first
    // open row was fight work offered Ines nothing and she stayed home while
    // the job she is the best in the band at stood open behind it. Staged over
    // a board spent everywhere else, so the choice is between those two rows
    // and nothing else.
    let ines = index("ines");
    let mut behind = sim.clone();
    for site in &mut behind.sites {
        site.states = vec![crate::sim::JobState::Done { by: 0 }; site.quests.len()];
    }
    behind.sites[0].quests = vec![
        crate::sim::Quest {
            name: "the ridge patrol",
            task: TaskType::Fight,
            pot: 55,
            duration: 100,
        },
        crate::sim::Quest {
            name: "the signal repair",
            task: TaskType::Craft,
            pot: 45,
            duration: 80,
        },
    ];
    behind.sites[0].states = vec![crate::sim::JobState::Open; 2];
    let picked = choose(&behind, tuning, 0, ines, &candidates(&behind, ines));
    checks.require(
        picked.action
            == Action::SeekWork {
                job: sim::JobId { site: 0, slot: 1 },
            },
        "the scorer cannot reach a job standing behind another one",
        format!(
            "Ines chose {:?} off a board whose first open row is fight work and whose second \
             is the signal repair; she is the band's crafter, and a candidate list built per \
             site would only ever have offered her the patrol",
            picked.action
        ),
    );
    // And the fit the board's row shows her is the aptitude the sum weighed —
    // one function, two readers (`Lens::competence`).
    judge(
        checks,
        "the fit a board row shows is not the aptitude the scorer weighs",
        crate::traits::competence_at(TaskType::Craft, &behind.people[ines].traits),
        2,
        "Ines's craft aptitude, which the panel prints and the sum multiplies",
    );

    // A completed visit's warmth, through the one function the scheduler uses.
    let mut visited = sim.clone();
    let before = (
        visited.shared.regard(ludo, Regarded::Person(hana)),
        visited.shared.regard(hana, Regarded::Person(ludo)),
    );
    crate::sim::settle_visit(&mut visited, tuning, ludo, hana);
    judge(
        checks,
        "a visit does not warm the visitor by what the shipped set says",
        visited.shared.regard(ludo, Regarded::Person(hana)) - before.0,
        1,
        "one point of regard for the call",
    );
    judge(
        checks,
        "a visit's warmth is not symmetric where the shipped set says it is",
        visited.shared.regard(hana, Regarded::Person(ludo)) - before.1,
        1,
        "the shipped set makes a visit mutual",
    );

    // **Nobody who is out is rescored**, and nobody who is idle chooses to
    // idle while paid work stands open — the two claims the cadence rests on.
    let mut busy = sim.clone();
    busy.parties[ludo].activity = Activity::Working { until: 999 };
    let open = candidates(&busy, ludo);
    checks.require(
        !open
            .iter()
            .any(|action| matches!(action, Action::Socialize { toward } if *toward == ludo)),
        "the scorer offered somebody their own doorstep",
        "candidates() must not propose visiting yourself".to_owned(),
    );
    let chosen = choose(&sim, tuning, 0, ludo, &candidates(&sim, ludo));
    checks.require(
        matches!(chosen.action, Action::SeekWork { .. }) && !chosen.reason.is_empty(),
        "the eager worker does not take open work, or takes it for no stated reason",
        format!(
            "Ludo chose {:?} scoring {} because {:?}, out of {:?}; CAST.md §4.1 names him \
             the character the scorer is most visibly alive on",
            chosen.action,
            chosen.score,
            chosen.reason,
            chosen
                .terms
                .iter()
                .map(|term| (term.what, term.value))
                .collect::<Vec<_>>()
        ),
    );
    // The verdict's own arithmetic: the score is the sum of the terms it
    // reports, so a surface showing either cannot disagree with the sim.
    judge(
        checks,
        "a verdict's score is not the sum of the terms it reports",
        chosen.terms.iter().map(|term| term.value).sum::<i64>(),
        chosen.score,
        "the reasons and the number are one derivation",
    );
    // The task type a quest names is the aptitude row the scorer reads: the
    // Deep Cave's open haul is labour, and `TaskType::of_aptitude` is what
    // says so both ways.
    checks.require(
        sim.sites
            .get(1)
            .and_then(|site| site.quest(0))
            .is_some_and(|quest| TaskType::of_aptitude(quest.task.aptitude()) == Some(quest.task)),
        "a quest's task type does not round-trip through its aptitude row",
        "CAST.md §2 makes an aptitude's id the task's id".to_owned(),
    );
}

/// **The scorer's own verify battery**: the four claims wave 1.1 owes beyond
/// the sweep (GDD §9).
///
/// Conducted runs, so these are claims about the played world rather than
/// about staged state: a replay, the alive sweep, the relationship-preset
/// flip, and the one-dispatch-path assertion.
pub fn judge_module(
    checks: &mut crate::checks::Checks,
    baseline: &crate::sweep::Conducted,
) -> String {
    let tuning = Tuning::SHIPPED;
    let mut notes: Vec<String> = Vec::new();

    // --- 1: the choices replay ---------------------------------------------
    // The same seed and the same orders, twice: the same characters take the
    // same actions at the same world-minutes **for the same stated reasons**.
    // `sweep::transcript` carries the sentence, and the sentence is where the
    // reason lives, so this is a claim about the words as well as the moves.
    let script = crate::sweep::speed_scripts().remove(0).1;
    let again = crate::sweep::conduct(&crate::sweep::Session::plain(tuning, &script, 60_000));
    let (first, second) = (
        crate::sweep::transcript(&baseline.events),
        crate::sweep::transcript(&again.events),
    );
    checks.require(
        first == second,
        "the scorer does not choose the same way twice",
        format!(
            "the first run's transcript is {first:?} and the replay's is {second:?}; a choice \
             is a function of (seed, orders, constants) and nothing else"
        ),
    );
    let reasons = baseline
        .events
        .iter()
        .filter(|event| event.class == crate::attention::EventClass::ActionStarted)
        .count();
    checks.require(
        reasons > 0
            && baseline
                .events
                .iter()
                .filter(|event| event.class == crate::attention::EventClass::ActionStarted)
                .all(|event| event.note.contains(" - ")),
        "a character decided something and the transcript does not say why",
        format!(
            "{reasons} action-started events, and one of them carries no reason after its \
             verb"
        ),
    );
    notes.push(format!("{reasons} decisions, replayed identically"));

    // --- 2: alive -----------------------------------------------------------
    notes.push(judge_alive(checks, &tuning));

    // --- 3: the relationship preset flips a choice --------------------------
    judge_presets(checks, &tuning);

    // --- 4: one dispatch path -----------------------------------------------
    judge_one_path(checks, baseline);
    judge_named_claims(checks, baseline);

    notes.join("; ")
}

/// **Alive**: with the player idle, everybody takes work.
///
/// The economy sweep's opening half (GDD §9). The plan asks for ~200 seeds;
/// this build has **no `Rng` read at all** — `verify::seed_independence`
/// asserts the whole transcript is identical at seeds far apart — so the other
/// hundred and ninety-odd are the same run, and eight far-apart seeds are what
/// is worth the wall time until randomness lands.
fn judge_alive(checks: &mut crate::checks::Checks, tuning: &Tuning) -> String {
    let seeds = [0u64, 1, 7, 99, 1_000, 65_535, 7_777_777, 4_294_967_291];
    let window = alive_window(tuning);
    let mut summary = String::new();
    for seed in seeds {
        let mut session = crate::sweep::Session::plain(*tuning, &[], 40_000);
        session.seed = Some(seed);
        session.stop_at_minute = Some(window);
        // The player never touches the world: the clock is started and that
        // is all. An idle player is the condition the claim is about.
        let start = [crate::sweep::Directive {
            when: crate::sweep::When::Tick(5),
            what: crate::sweep::Act::Tap(Key::Digit3),
        }];
        session.directives = &start;
        let run = crate::sweep::conduct(&session);
        // Who took paid work, and when they first did.
        let people = run.sim.people.len();
        let mut first_job: Vec<Option<u64>> = vec![None; people];
        let mut jobs: Vec<usize> = vec![0; people];
        for event in &run.events {
            if event.class != crate::attention::EventClass::Departed
                || !event.note.starts_with("departed for")
            {
                continue;
            }
            if let Some(slot) = first_job.get_mut(event.party) {
                slot.get_or_insert(event.minute);
            }
            if let Some(count) = jobs.get_mut(event.party) {
                *count += 1;
            }
        }
        let idle: Vec<&str> = (0..people)
            .filter(|who| first_job[*who].is_none())
            .map(|who| run.sim.people[who].name)
            .collect();
        checks.require(
            idle.is_empty(),
            "somebody never took a job in a world where nobody was told to",
            format!(
                "at seed {seed}, {idle:?} took no paid work in {} world-days; the settlement \
                 must limp without the player (GDD §1)",
                tuning.alive_days
            ),
        );
        // **Nobody is dispatched while already out**: the scorer never
        // countermands a journey, and the dispatch loop refuses one anyway.
        let mut out: Vec<bool> = vec![false; people];
        for event in &run.events {
            let who = event.party;
            match event.class {
                crate::attention::EventClass::Departed => {
                    checks.require(
                        !out[who],
                        "somebody was dispatched while they were already out",
                        format!(
                            "at seed {seed}, {} departed at minute {} without having come \
                             home",
                            run.sim.people[who].name, event.minute
                        ),
                    );
                    out[who] = true;
                }
                crate::attention::EventClass::Returned => out[who] = false,
                _ => {}
            }
        }
        // **The eager worker** (`CAST.md` §4.1): Ludo takes work at the very
        // first moment he is asked to think about it. Indebted favours any
        // paid work and he has no pride to spend, so there is no board he
        // waits out — which is what "the character the scorer is most visibly
        // alive on" means once the cadence is staggered by roster index and
        // *who goes first* is a fact about the roster's order rather than
        // about anybody's appetite.
        let ludo = run
            .sim
            .people
            .iter()
            .position(|person| person.id == "ludo")
            .unwrap_or(0);
        let most = jobs.iter().copied().max().unwrap_or(0);
        checks.require(
            first_job.get(ludo).copied().flatten() == Some(first_score(tuning, ludo)),
            "the eager worker did not take work the first time he was asked to think",
            format!(
                "at seed {seed}, Ludo first departed for work at {:?} and he is first \
                 weighed at minute {}",
                first_job.get(ludo).copied().flatten(),
                first_score(tuning, ludo)
            ),
        );
        // How many of the band waited a round before taking anything - a
        // number the report carries rather than an assertion, because at the
        // shipped weights a board of six jobs a site suits everybody and
        // nobody waits. That the scorer *can* say no is staged in
        // [`judge_at`], where a board can be made to hold nothing anybody
        // wants.
        let patient = (0..people)
            .filter(|who| first_job[*who] > Some(first_score(tuning, *who)))
            .count();
        if summary.is_empty() {
            summary = format!(
                "alive sweep: {} seeds x {} world-days, everybody worked, busiest {most} jobs, {} waited",
                seeds.len(),
                tuning.alive_days,
                patient
            );
        }
    }
    summary
}

/// **The relationship preset changes what somebody chooses.**
///
/// Flat and authored are a drawer row (`bonds_preset`) and so ride every
/// stamp; this is the claim that the row is not decoration. Staged over a
/// world whose quest board is spent, because that is where regard is what is
/// left to weigh — with work open, work wins under either preset, which is
/// itself the right answer and not a difference worth asserting.
fn judge_presets(checks: &mut crate::checks::Checks, tuning: &Tuning) {
    let spend = |tuning: &Tuning| {
        let mut sim = Sim::opening(tuning, crate::modules::ModuleSet::ALL);
        for site in &mut sim.sites {
            site.states = vec![crate::sim::JobState::Done { by: 0 }; site.quests.len()];
        }
        sim
    };
    let authored = spend(&tuning.with(crate::constants::Field::BondsPreset, 1));
    let flat = spend(&tuning.with(crate::constants::Field::BondsPreset, 0));
    let differing: Vec<(&str, Action, Action)> = (0..authored.people.len())
        .filter_map(|who| {
            let a = choose(&authored, tuning, 0, who, &candidates(&authored, who)).action;
            let b = choose(&flat, tuning, 0, who, &candidates(&flat, who)).action;
            (a != b).then_some((authored.people[who].name, a, b))
        })
        .collect();
    checks.require(
        !differing.is_empty(),
        "the relationship preset changes nobody's mind",
        "with an empty quest board, every character chooses the same thing under the \
         authored seeds as under the flat ones; a preset that changes no choice is a row \
         nothing reads"
            .to_owned(),
    );
    // And the difference is the one the seeds are for: somebody goes to see
    // somebody they think well of, where a flat world gives them no reason to.
    checks.require(
        differing
            .iter()
            .any(|(_, a, b)| matches!(a, Action::Socialize { .. }) && *b == Action::Idle),
        "the authored preset's difference is not the one the seeds are for",
        format!(
            "the choices that differ are {differing:?}; CAST.md §5 seeds warmth, and warmth \
             is what takes somebody to another door"
        ),
    );
    // Flat is flat: no edges, no facts.
    checks.require(
        flat.shared.edges().is_empty() && flat.shared.all_facts().is_empty(),
        "the flat preset is not flat",
        format!(
            "it opens with {} edges and {} facts",
            flat.shared.edges().len(),
            flat.shared.all_facts().len()
        ),
    );
}

/// **The claim is the named job**, for a player's order and a self-dispatch
/// alike.
///
/// Every departure for paid work names a job in its sentence; the row the
/// board records as taken is that same job, taken by that same party. This is
/// the check aimed at a dispatch that showed the player one job and claimed
/// the site's first open one underneath — the failure the whole job board
/// would otherwise be a picture of.
fn judge_named_claims(checks: &mut crate::checks::Checks, run: &crate::sweep::Conducted) {
    let departures = run
        .events
        .iter()
        .filter(|event| {
            event.class == crate::attention::EventClass::Departed
                && event.note.starts_with("departed for")
        })
        .count();
    let mut taken = 0usize;
    for site in &run.sim.sites {
        for (slot, state) in site.states.iter().enumerate() {
            let Some(by) = state.holder() else { continue };
            taken += 1;
            let Some(quest) = site.quest(slot) else {
                continue;
            };
            let named = run
                .events
                .iter()
                .filter(|event| {
                    event.party == by
                        && event.class == crate::attention::EventClass::Departed
                        && event.note.contains(quest.name)
                })
                .count();
            checks.require(
                named == 1,
                "a claimed job is not the job the departure named",
                format!(
                    "{:?} at {} is held by party {by} and {named} of their departures name it; \
                     the order names a job and the claim is that job, never the site's \
                     first-open row",
                    quest.name,
                    crate::grid::LOCATIONS[site.location].name,
                ),
            );
        }
    }
    checks.require(
        taken == departures && departures > 0,
        "the board's spent rows and the departures for work are not the same list",
        format!(
            "{taken} rows are claimed or done across the four sites and {departures} \
             departures for paid work were emitted; one departure claims exactly one row"
        ),
    );
}

/// **One dispatch path**: a party the player sent and a party that sent itself
/// produce the same shape of journey.
///
/// The failure this is aimed at is a second travel loop for autonomous
/// characters. There is one, so the five movement classes come out in the same
/// order with the same kind of sentence on each, and the only difference is
/// the pair of decision events that bracket a self-dispatch.
fn judge_one_path(checks: &mut crate::checks::Checks, run: &crate::sweep::Conducted) {
    let movement = [
        crate::attention::EventClass::Departed,
        crate::attention::EventClass::Arrived,
        crate::attention::EventClass::WorkBegan,
        crate::attention::EventClass::QuestComplete,
        crate::attention::EventClass::Returned,
    ];
    let journey = |party: usize| -> Vec<(&'static str, String)> {
        run.events
            .iter()
            .filter(|event| event.party == party && movement.contains(&event.class))
            .take(movement.len())
            .map(|event| {
                (
                    event.class.name(),
                    event.note.split(' ').next().unwrap_or_default().to_owned(),
                )
            })
            .collect()
    };
    // Party 0 is ordered by the script at minute 8; the first party whose
    // journey the scorer began is whoever `chosen` was true for.
    let self_sent = run
        .events
        .iter()
        .find(|event| event.class == crate::attention::EventClass::ActionStarted)
        .map(|event| event.party);
    let Some(self_sent) = self_sent else {
        checks.require(
            false,
            "no party sent itself anywhere in the whole run",
            "the one-dispatch-path claim has nothing to compare against".to_owned(),
        );
        return;
    };
    let (ordered, chosen) = (journey(0), journey(self_sent));
    checks.require(
        ordered == chosen && !ordered.is_empty(),
        "a self-dispatched journey is not the shape a player-dispatched one is",
        format!(
            "the ordered party's journey reads {ordered:?} and the self-sent one's reads \
             {chosen:?}; there is one dispatch loop and two callers, so the five movement \
             classes and their verbs must come out the same"
        ),
    );
    // The decision events are the whole of the difference, and they bracket
    // the journey rather than replacing any part of it.
    let decisions = run
        .events
        .iter()
        .filter(|event| {
            event.party == self_sent
                && matches!(
                    event.class,
                    crate::attention::EventClass::ActionStarted
                        | crate::attention::EventClass::ActionDone
                )
        })
        .count();
    checks.require(
        decisions >= 1,
        "a self-dispatched journey carries no decision events at all",
        format!("party {self_sent} emitted {decisions} of them"),
    );
    checks.require(
        !run.events.iter().any(|event| {
            event.party == 0
                && matches!(
                    event.class,
                    crate::attention::EventClass::ActionStarted
                        | crate::attention::EventClass::ActionDone
                )
                && event.minute < 300
        }),
        "a player's order was reported as somebody's own decision",
        "the party the script ordered at minute 8 emitted an action-started before it came \
         home; a player's order is not a question anybody asked themselves"
            .to_owned(),
    );
    let _ = crate::sweep::addresses(&run.events);
}
