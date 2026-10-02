//! **The petitions battery** — what GDD §9 owes the module wave 1.5 built.
//!
//! Every expectation a shipped literal, never derived from `tuning`: the
//! mutation round runs [`judge_at`] at moved constants, and a check that
//! recomputed its expectation from the constant under test could not see it
//! move (`make-game` §A.6).
//!
//! - [`judge_at`] — staged, cheap: the cadence, the window, the roll, and the
//!   arithmetic of a gift, a satisfaction and a failure.
//! - [`one_source`] — the fired consequence is the card's reference, and a
//!   mutated deadline, `{n}` or consequence moves the card and the cliff
//!   together.
//! - [`replay`] and [`invariance`] — the same seed and orders voice the same
//!   petitions with the same words, at every speed.
//! - [`pipe`] — the escalation pipe, end to end, both ways.
//! - [`sweeps`] — the idle settlement re-judged at the petition horizon, and
//!   the attention differential re-passed.
//! - [`module_off`] and [`conservation`] — the degrades-to sentence, and the
//!   ledger over the three new ports and `gives-away`.

use crate::attention::EventClass;
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::modules::ModuleSet;
use crate::petitions::{self, DAY, Reward, Template};
use crate::pleas::{self, Credit, Found, Status};
use crate::sim::Sim;
use crate::stores::Regarded;

/// The roster index of each person a staged case is about.
const BOB: usize = 0;
const STEVE: usize = 1;
const TIM: usize = 3;
const HANA: usize = 6;
const LUDO: usize = 7;

/// A staged world: everything on, the whole camp present, nothing run.
fn staged(tuning: &Tuning) -> Sim {
    let mut sim = Sim::opening(tuning, ModuleSet::ALL);
    sim.everybody_here();
    sim
}

/// The template with this id — the table's own row, by reference.
fn row(id: &str) -> &'static Template {
    match petitions::find(id) {
        Some(template) => template,
        None => crate::checks::fail(
            "the petitions battery names a template the table does not have",
            &format!("{id:?} is not a row of petitions::TEMPLATES"),
        ),
    }
}

/// How many of the first [`ROLL_DRAWS`] checks at seed zero pass the shipped
/// roll — a literal, so a moved `plea_odds` moves it.
pub const ROLL_PASSES: usize = 57;
/// How many addresses the roll is drawn at.
pub const ROLL_DRAWS: u64 = 400;

/// **The petitions module, judged at a stated constants set** — every
/// expectation a shipped literal.
pub fn judge_at(checks: &mut Checks, tuning: &Tuning) {
    // --- the cadence, the window, the stagger -------------------------------
    for (what, got, want) in [
        (
            "a petition check's interval",
            pleas::interval(tuning),
            360u64,
        ),
        (
            "the fourth character's first check",
            pleas::first_check(tuning, TIM),
            1512,
        ),
        ("the thin-days window", pleas::thin_window(tuning), 4320),
    ] {
        checks.require(
            got == want,
            "the petition clock is not what the shipped set says",
            format!("{what} is {got} world-minutes and the shipped set says {want}"),
        );
    }
    // --- the roll ------------------------------------------------------------
    let passes = (0..ROLL_DRAWS)
        .filter(|minute| petitions::roll(0, *minute, 0, 0) < tuning.plea_odds)
        .count();
    checks.require(
        passes == ROLL_PASSES,
        "the petition roll does not pass as often as the shipped odds say",
        format!(
            "{passes} of {ROLL_DRAWS} staged checks pass and the shipped plea_odds passes \
             {ROLL_PASSES}"
        ),
    );
    // --- a gift, and the satisfaction it buys --------------------------------
    let mut gifted = staged(tuning);
    gifted.treasury = 200;
    let raised = pleas::raise(
        &mut gifted,
        tuning,
        1500,
        LUDO,
        row("collectors-visit"),
        Found::default(),
    );
    let given = raised.map(|id| pleas::give(&mut gifted, tuning, 1500, id));
    let ludo = &gifted.people[LUDO];
    let met = raised
        .and_then(|id| gifted.petitions.get(id))
        .map(|petition| petition.status);
    checks.require(
        given == Some(Ok(30))
            && met
                == Some(Status::Met {
                    at: 1500,
                    credit: Credit::Player,
                })
            && ludo.wallet == 32
            && ludo.desperation == 3
            && gifted.treasury == 170
            && gifted.shared.regard(LUDO, Regarded::Player) == 4
            && gifted.ports.transferred_gifts == 30,
        "a gift did not satisfy the petition it answered by the shipped arithmetic",
        format!(
            "the gift returned {given:?} and left the petition {met:?}, Ludo holding {}g at \
             desperation {} and regard {} toward you, the treasury {}g; the shipped set says \
             Ok(30), met by you at 1500, 32g, desperation 3, regard 4, 170g",
            ludo.wallet,
            ludo.desperation,
            gifted.shared.regard(LUDO, Regarded::Player),
            gifted.treasury
        ),
    );
    // --- a failure, the consequence it declared, and the chain ---------------
    let mut failed = staged(tuning);
    let first = pleas::raise(
        &mut failed,
        tuning,
        1500,
        BOB,
        row("collectors-visit"),
        Found::default(),
    );
    if let Some(id) = first {
        pleas::deadline(&mut failed, tuning, 1500 + 6 * DAY, id);
    }
    let bob = &failed.people[BOB];
    let chained = bob
        .active_petition
        .and_then(|id| failed.petitions.get(id))
        .map(|petition| petition.template.id);
    checks.require(
        bob.wallet == 0
            && failed.ports.burned_consequences == 6
            && bob.desperation == 6
            && failed.shared.regard(BOB, Regarded::Player) == -2
            && bob.source.contains("cleaned out by the collector")
            && chained == Some("collectors-visit-again"),
        "a failed petition did not fire the consequence it declared",
        format!(
            "after the cliff Bob holds {}g ({}g burned) at desperation {}, regard {} toward \
             you, says {:?} and carries {chained:?}; the shipped set says 0g, 6g burned, 6, -2, \
             cleaned out by the collector, and the chain's next",
            bob.wallet,
            failed.ports.burned_consequences,
            bob.desperation,
            failed.shared.regard(BOB, Regarded::Player),
            bob.source
        ),
    );
    // --- the thin-days window ------------------------------------------------
    let mut thin = staged(tuning);
    for minute in [0, DAY, 2 * DAY] {
        thin.emit_need(minute, TIM, thin.people[TIM].home, "staged".to_owned());
    }
    let template = row("thin-days");
    let inside = pleas::holds(&thin, tuning, 3 * DAY, TIM, template).is_some();
    let outside = pleas::holds(&thin, tuning, 4 * DAY + 1, TIM, template).is_some();
    checks.require(
        inside && !outside,
        "the thin-days window does not count what the shipped set says",
        format!(
            "three shortfalls a day apart hold thin-days at day 3: {inside}, and at day 4 and a \
             minute: {outside}; the shipped three-day window says true then false"
        ),
    );
    // --- the chip's explanation is derived ---------------------------------
    let said = petitions::explain(row("collectors-visit").consequence, tuning, None);
    checks.require(
        said == BROKE_EXPLAINED,
        "the broke chip does not say what the vocabulary row and the shipped set make it",
        format!("it says {said:?} and the shipped set says {BROKE_EXPLAINED:?}"),
    );
}

/// What the `broke` chip says at the shipped set — written down, because the
/// sentence is the derivation's output and a check that derived it again
/// could not see `plea_regard` move.
pub const BROKE_EXPLAINED: &str = "broke: their purse is emptied and the gold is gone, \
                                    desperation +2, their source line is rewritten to it; and \
                                    it sours - regard toward you -4, and a grudge if they were \
                                    failed before.";

/// T1 with a deadline of two days — a mutated datum, for [`one_source`].
static SHORT_DEADLINE: Template = Template {
    deadline_days: 2,
    ..petitions::TEMPLATES[0]
};

/// T1 asking for twelve gold rather than thirty.
static SMALL_DEBT: Template = Template {
    n: 12,
    ..petitions::TEMPLATES[0]
};

/// T1 declaring a walk-out rather than `broke`.
static WALKS_INSTEAD: Template = Template {
    consequence: &petitions::DECLARED[3],
    ..petitions::TEMPLATES[0]
};

/// **A template that pays gold** — the reward port's first exerciser, which is
/// a staged row and not content: T1 to T6 all pay in regard (`CAST.md` §6).
static PAYS_GOLD: Template = Template {
    reward: Reward::Gold(10),
    ..petitions::TEMPLATES[0]
};

/// **One source.** The consequence that fires is the card's own reference, the
/// card's "met when" line is derived from the predicate the cliff evaluates,
/// and a mutated deadline, `{n}` or consequence moves the card and the firing
/// together.
pub fn one_source(checks: &mut Checks) -> String {
    let tuning = Tuning::SHIPPED;
    // --- identity, over a world that lived -----------------------------------
    let lived = crate::economy::world(
        &tuning,
        ModuleSet::ALL,
        0,
        SWEEP_DAYS,
        crate::economy::Player::Idle,
    );
    let fired: Vec<_> = lived
        .petitions
        .all()
        .iter()
        .filter(|petition| matches!(petition.status, Status::Failed { .. }))
        .collect();
    let copies = fired
        .iter()
        .filter(|petition| {
            !petition
                .fired
                .is_some_and(|fired| std::ptr::eq(fired, petition.declared()))
        })
        .count();
    checks.require(
        !fired.is_empty() && copies == 0,
        "a fired consequence is not the reference the card declared",
        format!(
            "{} petitions failed over {SWEEP_DAYS} idle days and {copies} of them fired \
             something other than the template's own consequence reference",
            fired.len()
        ),
    );
    let failures = fired.len();
    // --- a mutated deadline moves the card and the cliff ---------------------
    let mut world = staged(&tuning);
    let id = pleas::raise(
        &mut world,
        &tuning,
        1500,
        BOB,
        &SHORT_DEADLINE,
        Found::default(),
    );
    let card_due = id
        .and_then(|id| world.petitions.get(id))
        .map(|petition| petition.deadline);
    let grid = crate::grid::grid();
    let mut quiet = world.clone();
    quiet.modules = ModuleSet::ALL.without_id(crate::autonomy::MODULE);
    crate::sim::advance_to(&mut quiet, &grid, &tuning, 1500 + 2 * DAY);
    let failed_at = id
        .and_then(|id| quiet.petitions.get(id))
        .and_then(|p| match p.status {
            Status::Failed { at } => Some(at),
            _ => None,
        });
    checks.require(
        card_due == Some(1500 + 2 * DAY) && failed_at == card_due,
        "a mutated deadline moved the card and the cliff apart",
        format!(
            "a two-day T1 voiced at 1500 prints due {card_due:?} and failed at {failed_at:?}; both \
             are {}",
            1500 + 2 * DAY
        ),
    );
    // --- a mutated {n} moves the line and the predicate ----------------------
    let mut debt = staged(&tuning);
    debt.people[BOB].wallet = 20;
    let small = pleas::raise(&mut debt, &tuning, 1500, BOB, &SMALL_DEBT, Found::default());
    let small_met = small
        .and_then(|id| debt.petitions.get(id))
        .map(|p| p.status.resolved());
    let mut owed = staged(&tuning);
    owed.people[BOB].wallet = 20;
    let full = pleas::raise(
        &mut owed,
        &tuning,
        1500,
        BOB,
        row("collectors-visit"),
        Found::default(),
    );
    let line_of = |sim: &Sim, id: Option<usize>| {
        id.and_then(|id| sim.petitions.get(id))
            .map(|petition| {
                petition
                    .template
                    .condition
                    .met_when(&crate::lens::Lens::on(sim), petition)
            })
            .unwrap_or_default()
    };
    let (small_line, full_line) = (line_of(&debt, small), line_of(&owed, full));
    let full_met = full
        .and_then(|id| owed.petitions.get(id))
        .map(|p| p.status.resolved());
    checks.require(
        small_met == Some(true)
            && full_met == Some(false)
            && small_line.contains("12g")
            && full_line.contains("30g"),
        "the card's met-when line and the predicate disagree about {n}",
        format!(
            "with 20g in his purse Bob's twelve-gold T1 is met: {small_met:?} ({small_line:?}) \
             and the thirty-gold one is not: {full_met:?} ({full_line:?})"
        ),
    );
    // --- a mutated consequence moves the chip and the firing -----------------
    let mut walked = staged(&tuning);
    let id = pleas::raise(
        &mut walked,
        &tuning,
        1500,
        BOB,
        &WALKS_INSTEAD,
        Found::default(),
    );
    if let Some(id) = id {
        pleas::deadline(&mut walked, &tuning, 1500 + 6 * DAY, id);
    }
    let fired = id
        .and_then(|id| walked.petitions.get(id))
        .and_then(|p| p.fired);
    checks.require(
        fired.is_some_and(|fired| std::ptr::eq(fired, &petitions::DECLARED[3]))
            && !walked.people[BOB].present
            && walked.people[BOB].wallet == 6,
        "a mutated consequence moved the card and the firing apart",
        format!(
            "a T1 declaring walks-out fired {fired:?}; Bob is present: {} with {}g",
            walked.people[BOB].present, walked.people[BOB].wallet
        ),
    );
    format!(
        "{failures} fired consequences over {SWEEP_DAYS} idle days, every one the card's own \
         reference; a two-day deadline printed and fell at minute {}; a twelve-gold debt met \
         where thirty was not; a walk-out declared was a walk-out fired",
        1500 + 2 * DAY
    )
}

/// How many world-days the petition-horizon sweeps run: long enough for a
/// voicing on day two to reach its cliff, its chain to reach its own, and a
/// walk-out declared there to come home.
pub const SWEEP_DAYS: u64 = 12;

/// How many worlds each player's petition-horizon sweep walks.
pub const SWEEP_WORLDS: u64 = 16;

/// One petition, as a replay compares it: who, which template, when it was
/// raised, voiced and due, what it said, and how it ended.
type Record = (usize, &'static str, u64, Option<u64>, u64, String, Status);

/// The record of every petition a world raised.
fn records(sim: &Sim) -> Vec<Record> {
    sim.petitions
        .all()
        .iter()
        .map(|petition| {
            (
                petition.who,
                petition.template.id,
                petition.raised_at,
                petition.voiced_at,
                petition.deadline,
                petition.text.clone(),
                petition.status,
            )
        })
        .collect()
}

/// **The same seed and orders voice the same petitions with the same words.**
pub fn replay(checks: &mut Checks) -> String {
    let tuning = Tuning::SHIPPED;
    let once = crate::economy::world(
        &tuning,
        ModuleSet::ALL,
        3,
        SWEEP_DAYS,
        crate::economy::Player::Idle,
    );
    let twice = crate::economy::world(
        &tuning,
        ModuleSet::ALL,
        3,
        SWEEP_DAYS,
        crate::economy::Player::Idle,
    );
    let (a, b) = (records(&once), records(&twice));
    let lines = |sim: &Sim| {
        let lens = crate::lens::Lens::on(sim);
        sim.events
            .iter()
            .map(|event| event.line(&lens))
            .collect::<Vec<_>>()
    };
    checks.require(
        !a.is_empty() && a == b && lines(&once) == lines(&twice),
        "the same seed did not voice the same petitions twice",
        format!(
            "{} petitions the first time and {} the second; a replay is the contract",
            a.len(),
            b.len()
        ),
    );
    format!("{} petitions replayed word for word at seed 3", a.len())
}

/// The world-minute the invariance sweep runs to: past a voicing, its cliff,
/// a walk-out and its return.
pub const INVARIANCE_UNTIL: u64 = 16_000;

/// A full transcript, every event to the end of the run, with its sentence.
fn whole(events: &[crate::sim::Event]) -> Vec<(u64, &'static str, usize, String, Option<usize>)> {
    events
        .iter()
        .map(|event| {
            (
                event.minute,
                event.class.name(),
                event.party,
                event.note.clone(),
                event.petition,
            )
        })
        .collect()
}

/// **Speed-invariance over the three new sources** — the petition check, the
/// cliff and the return — under three speed scripts, each resumed by the
/// player from every stop the petitions make.
pub fn invariance(checks: &mut Checks) -> String {
    use crate::sweep::{Act, Directive, Session, When, conduct};
    use jidousha::prelude::Key;
    let tap = |when: When, key: Key| Directive {
        when,
        what: Act::Tap(key),
    };
    let scripts: Vec<(&str, Vec<Directive>)> = vec![
        ("all-1x", vec![tap(When::Tick(5), Key::Digit1)]),
        ("all-4x", vec![tap(When::Tick(5), Key::Digit3)]),
        (
            "mixed-pause",
            vec![
                tap(When::Tick(5), Key::Digit2),
                tap(When::Minute(3_000), Key::Space),
                tap(
                    When::MinuteHeld {
                        minute: 3_000,
                        after: 120,
                    },
                    Key::Space,
                ),
                tap(When::Minute(6_000), Key::Digit3),
            ],
        ),
    ];
    let mut runs = Vec::new();
    for (name, script) in &scripts {
        let mut session = Session::plain(Tuning::SHIPPED, script, 200_000);
        session.stop_at_minute = Some(INVARIANCE_UNTIL);
        session.resume_after = Some((Key::Space, 2));
        let run = conduct(&session);
        runs.push((*name, run));
    }
    let (first, rest) = runs.split_at(1);
    let base = whole(&first[0].1.events);
    let count = |class: EventClass| {
        first[0]
            .1
            .events
            .iter()
            .filter(|event| event.class == class)
            .count()
    };
    let (voiced, failed, satisfied) = (
        count(EventClass::PetitionVoiced),
        count(EventClass::PetitionFailed),
        count(EventClass::PetitionSatisfied),
    );
    let returned = first[0]
        .1
        .events
        .iter()
        .filter(|event| event.class == EventClass::Joined && event.note.starts_with("came back"))
        .count();
    for (name, run) in rest {
        let theirs = whole(&run.events);
        let parted = base
            .iter()
            .zip(theirs.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(base.len().min(theirs.len()));
        checks.require(
            theirs == base,
            "a petition source is not addressed in world-time",
            format!(
                "{name}'s transcript parts from {}'s at entry {parted} of {}: {:?} against {:?}",
                first[0].0,
                base.len(),
                theirs.get(parted),
                base.get(parted)
            ),
        );
        checks.require(
            records(&run.sim) == records(&first[0].1.sim),
            "the same world-time voiced different petitions at different speeds",
            format!("{name}'s petition record differs from {}'s", first[0].0),
        );
    }
    // **A sweep over sources that never fire passes vacuously**, so all three
    // are asserted to have fired inside the window.
    checks.require(
        voiced >= 1 && failed >= 1 && returned >= 1,
        "the invariance window does not reach all three petition sources",
        format!(
            "{voiced} voicings, {failed} cliffs that fired and {returned} returns inside \
             {INVARIANCE_UNTIL} minutes; every one of the three has to fire for the sweep to say \
             anything"
        ),
    );
    format!(
        "3 speed scripts over {INVARIANCE_UNTIL} world-minutes, {} events each, {voiced} \
         voicings, {satisfied} met, {failed} failed and {returned} returns at identical minutes",
        base.len()
    )
}

/// **The escalation pipe, end to end** (wave 1.5's own script): Tim — who
/// carries no motivator, so the only template that can speak for him is the
/// shortfall's — is left with an empty purse. He goes short, the pressure
/// rises, `thin-days` is voiced, and then two runs part. Left alone, it
/// fails: he walks out, and comes back. Arranged, the player posts him paid
/// work: it is met, his desperation comes down a step and his source line is
/// rewritten to it.
pub fn pipe(checks: &mut Checks) -> String {
    let tuning = Tuning::SHIPPED;
    let grid = crate::grid::grid();
    let open = |tuning: &Tuning| {
        let mut sim = Sim::opening(tuning, ModuleSet::ALL);
        sim.people[TIM].wallet = 0;
        sim
    };
    let tims = |sim: &Sim| -> Vec<(u64, &'static str)> {
        sim.events
            .iter()
            .filter(|event| {
                event.party == TIM
                    && matches!(
                        event.class,
                        EventClass::UpkeepShortfall
                            | EventClass::PetitionVoiced
                            | EventClass::PetitionSatisfied
                            | EventClass::PetitionFailed
                            | EventClass::Joined
                    )
            })
            .map(|event| (event.minute, event.class.name()))
            .collect()
    };
    // --- left alone ----------------------------------------------------------
    let mut alone = open(&tuning);
    crate::sim::advance_to(&mut alone, &grid, &tuning, PIPE_UNTIL);
    let left = tims(&alone);
    checks.require(
        left == PIPE_ALONE,
        "the escalation pipe does not run shortfall to walk-out to return",
        format!("Tim's transcript left alone is {left:?} and the shipped pipe is {PIPE_ALONE:?}"),
    );
    // --- arranged ------------------------------------------------------------
    let mut arranged = open(&tuning);
    crate::sim::advance_to(&mut arranged, &grid, &tuning, PIPE_VOICED);
    let before = arranged.people[TIM].desperation;
    // The arranged twin puts the mushroom haul back on its board at the
    // voicing — the state a botched job leaves, which this seed rolled none
    // of — and the player posts it to Tim by name, at the standing rate.
    let haul = crate::sim::JobId { site: 1, slot: 0 };
    if let Some(state) = arranged.sites[haul.site].states.get_mut(haul.slot) {
        *state = crate::sim::JobState::Open;
    }
    let rate = arranged.rates.of(crate::traits::TaskType::Labor);
    crate::asks::post(
        &mut arranged,
        &grid,
        &tuning,
        PIPE_VOICED,
        crate::asks::Offer {
            who: crate::asks::Who::Person(TIM),
            what: crate::asks::What::Job(haul),
            until: crate::asks::Until::Done,
            wage: rate,
        },
    );
    crate::sim::advance_to(&mut arranged, &grid, &tuning, PIPE_VOICED + DAY);
    let met = tims(&arranged);
    let tim = &arranged.people[TIM];
    let credit = arranged
        .petitions
        .all()
        .iter()
        .find(|petition| petition.who == TIM)
        .map(|petition| petition.status);
    checks.require(
        met == PIPE_ARRANGED
            && tim.desperation == before - 1
            && tim.source.contains("found paying work")
            && matches!(
                credit,
                Some(Status::Met {
                    credit: Credit::Player,
                    ..
                })
            ),
        "the arranged pipe did not come down a step with the source line rewritten",
        format!(
            "Tim's transcript arranged is {met:?} (the shipped pipe is {PIPE_ARRANGED:?}); he \
             went from desperation {before} to {} and says {:?}; the petition ended {credit:?}",
            tim.desperation, tim.source
        ),
    );
    format!(
        "Tim alone: {} occurrences, voiced at {PIPE_VOICED}, walked out and came back; arranged: \
         met by your posting, desperation {before} -> {}",
        left.len(),
        tim.desperation
    )
}

/// When Tim's `thin-days` is voiced in the pipe.
pub const PIPE_VOICED: u64 = 4392;
/// How far the left-alone run goes: past his return.
pub const PIPE_UNTIL: u64 = 16_000;
/// **Tim's pipe, left alone** — shipped literals.
pub const PIPE_ALONE: &[(u64, &str)] = &[
    (1440, "upkeep-shortfall"),
    (2880, "upkeep-shortfall"),
    (4320, "upkeep-shortfall"),
    (4392, "petition-voiced"),
    (5760, "upkeep-shortfall"),
    (7200, "upkeep-shortfall"),
    (8640, "upkeep-shortfall"),
    (10080, "upkeep-shortfall"),
    (10152, "petition-failed"),
    (14472, "joined"),
    (15840, "upkeep-shortfall"),
];
/// **Tim's pipe, arranged.**
pub const PIPE_ARRANGED: &[(u64, &str)] = &[
    (1440, "upkeep-shortfall"),
    (2880, "upkeep-shortfall"),
    (4320, "upkeep-shortfall"),
    (4392, "petition-voiced"),
    (4557, "petition-satisfied"),
];

/// What one world's petitions came to, at the petition horizon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    /// Voiced.
    pub voiced: usize,
    /// Met.
    pub met: usize,
    /// Failed.
    pub failed: usize,
    /// Walk-outs that fired.
    pub walked: usize,
    /// Walk-outs that came home again.
    pub returned: usize,
    /// Grudges the cast holds against the player.
    pub grudges: usize,
}

/// Count a world's petitions.
pub fn tally(sim: &Sim) -> Tally {
    let all = sim.petitions.all();
    let walked = all
        .iter()
        .filter(|petition| {
            matches!(petition.status, Status::Failed { .. }) && petition.declared().kind.leaves
        })
        .count();
    Tally {
        voiced: all
            .iter()
            .filter(|petition| petition.voiced_at.is_some())
            .count(),
        met: all
            .iter()
            .filter(|petition| matches!(petition.status, Status::Met { .. }))
            .count(),
        failed: all
            .iter()
            .filter(|petition| matches!(petition.status, Status::Failed { .. }))
            .count(),
        walked,
        returned: sim
            .events
            .iter()
            .filter(|event| {
                event.class == EventClass::Joined && event.note.starts_with("came back")
            })
            .count(),
        grudges: (0..sim.people.len())
            .filter(|who| sim.shared.facts(*who, Regarded::Player).grudge)
            .count(),
    }
}

/// The span a list of counts lands in.
fn span(values: impl Iterator<Item = usize>) -> (usize, usize) {
    values.fold((usize::MAX, 0), |(low, high), value| {
        (low.min(value), high.max(value))
    })
}

/// **The idle settlement's petitions, at the horizon** — `(lowest, highest)`
/// over the sweep's worlds, shipped literals.
pub const IDLE_VOICED: (usize, usize) = (13, 17);
/// The failures' band.
pub const IDLE_FAILED: (usize, usize) = (11, 13);
/// The walk-outs' band.
pub const IDLE_WALKED: (usize, usize) = (5, 10);
/// The grudges' band.
pub const IDLE_GRUDGES: (usize, usize) = (2, 3);
/// And what an idle player's camp meets: nothing — nobody's work answers a
/// petition once the board is spent (`FINDINGS.md` G-050).
pub const IDLE_MET: (usize, usize) = (0, 0);
/// The attentive player's worst world, in failed petitions.
pub const ATTENTIVE_WORST_FAILED: usize = 7;
/// **The differential's petition margin**: the attentive player's worst
/// world fails at least this many fewer petitions than neglect's best.
pub const PETITION_MARGIN: usize = 4;
/// And walks this many fewer people out of the camp.
pub const WALKOUT_MARGIN: usize = 3;
/// The first minute anybody in idle world zero reaches the desperation
/// ceiling — **with petitions on and with them off alike** (the diagnosis).
pub const CEILING_MINUTE: u64 = 7200;

/// The first minute anybody's desperation reads the ceiling, walking the
/// world day by day — `None` if nobody gets there.
fn first_ceiling(tuning: &Tuning, modules: ModuleSet) -> Option<u64> {
    let grid = crate::grid::grid();
    let mut sim = Sim::opening(tuning, modules);
    let mut minute = 0;
    while minute <= SWEEP_DAYS * DAY {
        crate::sim::advance_to(&mut sim, &grid, tuning, minute);
        if sim
            .people
            .iter()
            .any(|person| person.desperation >= crate::people::DESPERATION_MAX)
        {
            return Some(minute);
        }
        minute += 60;
    }
    None
}

/// **The idle sweep re-judged, and the attention differential re-passed**,
/// at the petition horizon.
pub fn sweeps(checks: &mut Checks) -> String {
    let tuning = Tuning::SHIPPED;
    let live = |player: crate::economy::Player| -> Vec<Sim> {
        (0..SWEEP_WORLDS)
            .map(|world| crate::economy::world(&tuning, ModuleSet::ALL, world, SWEEP_DAYS, player))
            .collect()
    };
    let idle = live(crate::economy::Player::Idle);
    let attentive = live(crate::economy::Player::Attentive);
    let idles: Vec<Tally> = idle.iter().map(tally).collect();
    let attends: Vec<Tally> = attentive.iter().map(tally).collect();
    let bands = (
        span(idles.iter().map(|t| t.voiced)),
        span(idles.iter().map(|t| t.failed)),
        span(idles.iter().map(|t| t.walked)),
        span(idles.iter().map(|t| t.grudges)),
    );
    let idle_met = span(idles.iter().map(|t| t.met));
    checks.require(
        bands == (IDLE_VOICED, IDLE_FAILED, IDLE_WALKED, IDLE_GRUDGES) && idle_met == IDLE_MET,
        "the idle settlement's petitions do not land in the bands the shipped set states",
        format!(
            "over {SWEEP_WORLDS} idle worlds of {SWEEP_DAYS} days: voiced {:?}, failed {:?}, \
             walked out {:?}, grudges {:?}, met {idle_met:?}; the shipped bands are \
             {IDLE_VOICED:?}, {IDLE_FAILED:?}, {IDLE_WALKED:?}, {IDLE_GRUDGES:?}, {IDLE_MET:?}",
            bands.0, bands.1, bands.2, bands.3
        ),
    );
    // --- the limp floor, at the petition horizon -----------------------------
    for (world, sim) in idle.iter().chain(attentive.iter()).enumerate() {
        let tally = tally(sim);
        let away = sim
            .people
            .iter()
            .filter(|person| !person.present && person.present_from <= SWEEP_DAYS * DAY);
        let owed_back = tally.walked.saturating_sub(away.count());
        checks.require(
            sim.people.iter().all(|person| person.wallet >= 0) && tally.returned >= owed_back,
            "a petition consequence broke the limp floor",
            format!(
                "world {world}: a purse is overdrawn ({:?}), or {} walk-outs fired and only {} \
                 came back with nobody else still away",
                sim.people
                    .iter()
                    .map(|person| person.wallet)
                    .collect::<Vec<_>>(),
                tally.walked,
                tally.returned
            ),
        );
        let opening: i64 = crate::people::roster()
            .iter()
            .map(|person| person.wallet)
            .sum();
        let holders = sim.treasury + sim.people.iter().map(|person| person.wallet).sum::<i64>();
        checks.require(
            holders == sim.ports.expected(opening),
            "gold moved through a port GDD §4.1 does not name, at the petition horizon",
            format!(
                "world {world}: the holders hold {holders}g and the ports account for {}g ({})",
                sim.ports.expected(opening),
                sim.ports.line()
            ),
        );
    }
    // **The ceiling, diagnosed** (`FINDINGS.md` G-050): an idle camp reaches
    // the desperation ceiling on its shortfalls alone, before any petition's
    // cliff falls — so the minute is the same with the module off.
    let (on, off) = (
        first_ceiling(&tuning, ModuleSet::ALL),
        first_ceiling(&tuning, ModuleSet::ALL.without_id(petitions::MODULE)),
    );
    checks.require(
        on == Some(CEILING_MINUTE) && off == on,
        "the idle ceiling is not the shortfalls' alone",
        format!(
            "idle world zero first reads the ceiling at {on:?} with petitions on and {off:?} \
             with them off; the shipped diagnosis is minute {CEILING_MINUTE} both ways"
        ),
    );
    // --- the attention differential, re-passed -------------------------------
    let worst = attends.iter().map(|t| t.failed).max().unwrap_or(usize::MAX);
    let fewest = idles.iter().map(|t| t.failed).min().unwrap_or(0);
    let worst_walked = attends.iter().map(|t| t.walked).max().unwrap_or(usize::MAX);
    let fewest_walked = idles.iter().map(|t| t.walked).min().unwrap_or(0);
    checks.require(
        worst == ATTENTIVE_WORST_FAILED
            && worst + PETITION_MARGIN <= fewest
            && worst_walked + WALKOUT_MARGIN <= fewest_walked,
        "attention does not beat neglect on petitions by the shipped margin",
        format!(
            "the attentive player's worst world failed {worst} petitions and walked {worst_walked} \
             people out, against neglect's best {fewest} and {fewest_walked}; the shipped worst \
             is {ATTENTIVE_WORST_FAILED} and the margins {PETITION_MARGIN} and {WALKOUT_MARGIN}"
        ),
    );
    let met = span(attends.iter().map(|t| t.met));
    format!(
        "{SWEEP_WORLDS} worlds x 2 players x {SWEEP_DAYS} days: idle voices {:?}, fails {:?}, \
         walks {:?} out and holds {:?} grudges, meets none; attentive's worst fails {worst} and \
         walks {worst_walked} out, meeting {met:?}; the idle ceiling falls at minute \
         {CEILING_MINUTE} with petitions on or off",
        bands.0, bands.1, bands.2, bands.3
    )
}

/// **Petitions off** — the degrades-to sentence, as a fact: nothing is voiced
/// and nothing fires, regard toward the player moves only through asks, wages
/// and drift, and nothing lowers desperation.
pub fn module_off(checks: &mut Checks) -> String {
    let tuning = Tuning::SHIPPED;
    let off = ModuleSet::ALL.without_id(petitions::MODULE);
    let attended = crate::economy::world(
        &tuning,
        off,
        0,
        SWEEP_DAYS,
        crate::economy::Player::Attentive,
    );
    let idle = crate::economy::world(&tuning, off, 0, SWEEP_DAYS, crate::economy::Player::Idle);
    for (who, sim) in [("attentive", &attended), ("idle", &idle)] {
        let petition_events = sim
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event.class,
                    EventClass::PetitionVoiced
                        | EventClass::PetitionSatisfied
                        | EventClass::PetitionFailed
                )
            })
            .count();
        let ports = sim.ports.transferred_gifts
            + sim.ports.transferred_rewards
            + sim.ports.transferred_given
            + sim.ports.burned_consequences;
        let grudges = (0..sim.people.len())
            .filter(|person| sim.shared.facts(*person, Regarded::Player).grudge)
            .count();
        checks.require(
            sim.petitions.all().is_empty()
                && petition_events == 0
                && ports == 0
                && grudges == 0
                && sim
                    .people
                    .iter()
                    .all(|person| person.present || person.present_from > 0),
            "with petitions off somebody still asked, or something still fired",
            format!(
                "the {who} world holds {} petitions, {petition_events} petition events, {ports}g \
                 through the petition ports, {grudges} grudges against you; the module's \
                 degrades-to sentence says none of any",
                sim.petitions.all().len()
            ),
        );
    }
    // **Nothing lowers desperation** with the module off: every person's
    // reading only ever rose from where the roster opened them.
    let fell = idle
        .people
        .iter()
        .zip(crate::people::roster().iter())
        .filter(|(now, was)| now.desperation < was.desperation)
        .count();
    checks.require(
        fell == 0,
        "with petitions off something lowered desperation",
        format!("{fell} people end the idle world below the desperation they opened at"),
    );
    format!(
        "petitions off: no petition, no consequence and no grudge over {SWEEP_DAYS} attentive \
         and idle days; nothing lowered desperation"
    )
}

/// **Conservation over the three new ports and `gives-away`**: a gift, a
/// reward, a burn and a giving-away, each staged, and the identity after each.
pub fn conservation(checks: &mut Checks) -> String {
    let tuning = Tuning::SHIPPED;
    let holds =
        |sim: &Sim| sim.treasury + sim.people.iter().map(|person| person.wallet).sum::<i64>();
    let mut notes = Vec::new();
    // A gift, and the reward the gift's satisfaction pays.
    let mut gold = staged(&tuning);
    gold.treasury = 100;
    let opening = holds(&gold);
    let gift = pleas::raise(&mut gold, &tuning, 1500, LUDO, &PAYS_GOLD, Found::default());
    let given = gift.map(|id| pleas::give(&mut gold, &tuning, 1500, id));
    checks.require(
        given == Some(Ok(30))
            && gold.ports.transferred_gifts == 30
            && gold.ports.transferred_rewards == 10
            && gold.treasury == 80
            && gold.people[LUDO].wallet == 22
            && holds(&gold) == gold.ports.expected(opening),
        "a gift or a reward moved gold the ledger does not account for",
        format!(
            "the gift returned {given:?}; {}g in gifts and {}g in rewards; the treasury {}g and \
             Ludo {}g; holders {} against {}",
            gold.ports.transferred_gifts,
            gold.ports.transferred_rewards,
            gold.treasury,
            gold.people[LUDO].wallet,
            holds(&gold),
            gold.ports.expected(opening)
        ),
    );
    notes.push("gift 30g and reward 10g".to_owned());
    // A burn.
    let mut burn = staged(&tuning);
    let opening = holds(&burn);
    let id = pleas::raise(
        &mut burn,
        &tuning,
        1500,
        BOB,
        row("collectors-visit"),
        Found::default(),
    );
    if let Some(id) = id {
        pleas::deadline(&mut burn, &tuning, 1500 + 6 * DAY, id);
    }
    checks.require(
        holds(&burn) == burn.ports.expected(opening) && burn.ports.burned_consequences == 6,
        "broke burned gold the ledger does not account for",
        format!(
            "{}g burned; holders {} against {}",
            burn.ports.burned_consequences,
            holds(&burn),
            burn.ports.expected(opening)
        ),
    );
    notes.push(format!("broke burned {}g", burn.ports.burned_consequences));
    // A giving-away: Hana, on Steve's behalf.
    let mut give = staged(&tuning);
    give.people[HANA].wallet = 9;
    let opening = holds(&give);
    let id = pleas::raise(
        &mut give,
        &tuning,
        1500,
        HANA,
        row("look-after-them"),
        Found {
            other: Some(STEVE),
            site: None,
        },
    );
    give.people[STEVE].desperation = crate::needs::DESPERATE_AT;
    if let Some(id) = id {
        pleas::deadline(&mut give, &tuning, 1500 + 4 * DAY, id);
    }
    checks.require(
        holds(&give) == give.ports.expected(opening)
            && give.ports.transferred_given == 4
            && give.people[HANA].wallet == 5
            && give.people[STEVE].wallet == 7,
        "gives-away moved gold the ledger does not account for",
        format!(
            "{}g given; Hana {}g and Steve {}g; holders {} against {}",
            give.ports.transferred_given,
            give.people[HANA].wallet,
            give.people[STEVE].wallet,
            holds(&give),
            give.ports.expected(opening)
        ),
    );
    notes.push(format!("gives-away {}g", give.ports.transferred_given));
    format!("conserved: {}", notes.join(", "))
}
