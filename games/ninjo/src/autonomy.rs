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
    /// Claim the open job at this site and go.
    SeekWork {
        /// Which site.
        site: usize,
    },
    /// Walk to another character's home tile and stay a while.
    Socialize {
        /// Whose doorstep.
        toward: usize,
    },
    /// Stay home. The floor every other candidate has to beat.
    Idle,
}

/// One term of a candidate's sum: what it was worth, and the words for it.
///
/// The reason a character gives is the largest positive term's own sentence —
/// the verdict-plus-reasons shape giri proved, with the arithmetic beside the
/// words so a surface can show either and neither can lie.
#[derive(Clone, Debug)]
pub struct Term {
    /// What the term is, as a report names it.
    pub what: &'static str,
    /// What it added (or, negative, took away).
    pub value: i64,
    /// The half-sentence a reason is built from.
    pub because: String,
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
/// **Built by the caller, weighed by the scorer.** Every site with an open
/// quest, every other character standing at their own door, and staying home.
/// Wave 2 appends its ask to whatever this returns.
pub fn candidates(sim: &Sim, who: usize) -> Vec<Action> {
    let mut out = vec![Action::Idle];
    for (site, spec) in sim.sites.iter().enumerate() {
        if spec.open().is_some() {
            out.push(Action::SeekWork { site });
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
            because: "nothing worth leaving for".to_owned(),
        }),
        Action::SeekWork { site } => {
            let Some(quest) = sim.sites.get(site).and_then(|site| site.open()).copied() else {
                return terms;
            };
            let where_to = crate::grid::LOCATIONS[sim::site_location(site)].name;
            // Desperation opens the sum, as it opened giri's willingness.
            terms.push(Term {
                what: "need",
                value: person.desperation * tuning.need_weight,
                because: "needs the money".to_owned(),
            });
            // A want's pressure, where its `favors` field covers this work.
            let pressure = traits::pressure_toward(quest.task, &person.traits);
            if pressure != 0 {
                let named = person
                    .traits
                    .iter()
                    .map(|id| id.def())
                    .filter(|def| def.favors.covers(quest.task))
                    .map(|def| def.name)
                    .collect::<Vec<_>>()
                    .join(" and ");
                terms.push(Term {
                    what: "want",
                    value: pressure * tuning.want_weight,
                    because: format!("{named}, and this is {} work", quest.task.id()),
                });
            }
            // The aptitude row whose id is the task's id.
            let apt = traits::competence_at(quest.task, &person.traits);
            if apt != 0 {
                terms.push(Term {
                    what: "aptitude",
                    value: apt * tuning.apt_weight,
                    because: format!("it is {} work, and they are good at it", quest.task.id()),
                });
            }
            // The pot's pull, per ten gold, by the carrier's own affinity.
            let pull = traits::pot_pull_of(&person.traits);
            if pull != 0 {
                terms.push(Term {
                    what: "pot",
                    value: pull * quest.pot * tuning.pot_weight / 10,
                    because: format!("the pot is {}g, and they can feel it", quest.pot),
                });
            }
            // The rest term: nobody works forever.
            if now < sim.parties.get(who).map_or(0, |party| party.rested_until) {
                terms.push(Term {
                    what: "rest",
                    value: -tuning.rest_weight,
                    because: "has not stopped since the last job".to_owned(),
                });
            }
            let _ = where_to;
        }
        Action::Socialize { toward } => {
            let felt = traits::weighted_regard(
                sim.shared.regard(who, Regarded::Person(toward)),
                &person.traits,
            );
            let host = sim.people.get(toward).map_or("somebody", |who| who.name);
            terms.push(Term {
                what: "regard",
                value: felt * tuning.regard_weight,
                because: format!("thinks well of {host}"),
            });
        }
    }
    terms
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
                reason: reason_for(action, &terms),
                terms,
            };
            first = false;
        }
    }
    best
}

/// The words: the **first** strictly-largest positive term's own sentence.
///
/// First rather than last, so a tie reads as the term the sum opened with —
/// desperation, as it did in giri — and so the sentence is a function of the
/// term order rather than of an iterator's tie-breaking.
fn reason_for(action: Action, terms: &[Term]) -> String {
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
    let open = candidates(sim, who);
    let judged = choose(sim, tuning, now, who, &open);
    act(sim, grid, tuning, now, who, judged);
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
    match judged.action {
        // An idle choice is not an occurrence: nothing happened to anybody,
        // and the feed is for things that did (the drift's own precedent).
        Action::Idle => {}
        Action::SeekWork { site } => {
            let name = crate::grid::LOCATIONS[sim::site_location(site)].name;
            let quest = sim
                .sites
                .get(site)
                .and_then(|site| site.open())
                .map_or("work", |quest| quest.name);
            sim.emit_action(
                now,
                tile,
                who,
                format!("took {quest} at {name} - {}", judged.reason),
            );
            let _ = sim::dispatch(
                sim,
                grid,
                tuning,
                now,
                who,
                site,
                sim::Motive::chose(judged.reason.clone()),
            );
        }
        Action::Socialize { toward } => {
            let host = sim.people.get(toward).map_or("somebody", |who| who.name);
            sim.emit_action(
                now,
                tile,
                who,
                format!("went to see {host} - {}", judged.reason),
            );
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
        score(&sim, 0, ludo, Action::SeekWork { site: 1 }),
        20,
        "desperation 4x2 + indebted 3x2 + labor 2x3",
    );
    // Bob at the Black Vault adds the pot's own pull, which only the greedy
    // feel: 80 gold at one affinity and a weight of one is eight.
    judge(
        checks,
        "the pot does not pull the greedy by the shipped weight",
        score(&sim, 0, bob, Action::SeekWork { site: 3 }),
        28,
        "desperation 4x2 + indebted 3x2 + fight 2x3 + pot 1x80x1/10",
    );
    // A want whose `favors` names another task adds nothing: Odd wants fight
    // work and the Deep Cave's haul is labour.
    judge(
        checks,
        "a want applied to work its favors field does not name",
        score(&sim, 0, odd, Action::SeekWork { site: 1 }),
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
        score(&rested, 0, ludo, Action::SeekWork { site: 1 }),
        8,
        "the same twenty, less the rest weight of twelve",
    );
    judge(
        checks,
        "the rest term outlasts the rest it is counting",
        score(&rested, 100, ludo, Action::SeekWork { site: 1 }),
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
        site.claimed = 0;
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
            .and_then(|site| site.open())
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
            site.claimed = site.quests.len();
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
