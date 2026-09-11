//! **The compliance batteries** — what the asks module owes the verify run
//! (GDD §9's asks additions, wave 1.2).
//!
//! Five claims live here, and each is one of the ways a module about being
//! answered can be wrong: the answers have to replay, a posting must bind
//! nobody until it is heard, an ask to somebody away must be heard when the
//! messenger reaches them, the job row's verdict must be the scorer's own
//! answer, and the standing rates must be a lever rather than a label.
//!
//! [`judge_at`] is the staged half — every expectation a **shipped literal**,
//! because the mutation round runs it at moved constants to find out whether
//! anything is measuring them. [`judge_module`] is the conducted half, which
//! needs a world that has run.

use crate::answers;
use crate::asks::{self, Offer, Status, Until, What, Who};
use crate::attention::EventClass;
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::modules::ModuleSet;
use crate::sim::{self, JobId, Sim};
use crate::stores::Regarded;
use crate::traits::{TaskType, TraitId};

/// The wages the distribution sweep walks, in gold — from nothing to well
/// over every standing rate.
///
/// The ladder the compliance shares are a distribution *over*: this build
/// reads no `Rng` at all (`verify::seed_independence`), so two hundred seeds
/// are two hundred copies of one run and the population that actually varies
/// is the offer. Seven rungs and the four kinds of work under them is 280
/// answers per pass, which is cheap enough for the mutation round to run
/// thirty-six times.
pub const LADDER: [i64; 7] = [0, 8, 16, 24, 32, 40, 48];

/// One band of the cast, as the distribution sweep names them.
pub struct Band {
    /// The word the report and a failure use.
    pub name: &'static str,
    /// How many of the ladder's offers this band would take — a **shipped
    /// literal**, measured once and pinned, so a moved weight moves it.
    pub takes: usize,
}

/// The four bands, and what each takes off the ladder at the shipped set.
///
/// Loyal and cold are the two the regard term is about — the loyal comply for
/// less, the cold need paying — greedy is the one the wage's own pull is
/// about, and desperate is the one every term opens with. The numbers are
/// counts of "would take it" over [`LADDER`] x the four kinds of work, and
/// they are literals rather than shares so that one answer changing is one
/// number changing.
pub const BANDS: [Band; 4] = [
    // Steve and Rin, of twenty-eight offers each: the loyal comply for less,
    // and it is the regard-for-the-player term that says so.
    Band {
        name: "loyal",
        takes: 44,
    },
    // Alex, of twenty-eight: the cold need paying, and at the low rungs he
    // does not answer at all.
    Band {
        name: "cold",
        takes: 16,
    },
    // Bob, of twenty-eight: the wage's own pull is his, and it is what moves
    // him up the ladder.
    Band {
        name: "greedy",
        takes: 22,
    },
    // Bob, Steve and Ludo, of eighty-four: need opens every sum, so the
    // desperate take nearly everything — the band the wage lever has least
    // work to do on, and the one a settlement leans on.
    Band {
        name: "desperate",
        takes: 76,
    },
];

/// Who is in a band.
fn band_members(sim: &Sim, band: &str) -> Vec<usize> {
    (0..sim.people.len())
        .filter(|who| {
            let person = &sim.people[*who];
            match band {
                "loyal" => person.traits.contains(&TraitId::Loyal),
                "cold" => person.traits.contains(&TraitId::Cold),
                "greedy" => person.traits.contains(&TraitId::Greedy),
                _ => person.desperation >= 4,
            }
        })
        .collect()
}

/// One open job of each kind of work, front first — what the ladder is walked
/// against, so an aptitude is never the whole of why a band says no.
fn one_job_per_task(sim: &Sim) -> Vec<(TaskType, JobId)> {
    let mut out = Vec::new();
    for task in TaskType::ALL.iter().copied() {
        let found = sim.sites.iter().enumerate().find_map(|(site, board)| {
            board
                .open_slots()
                .find(|slot| board.quest(*slot).is_some_and(|quest| quest.task == task))
                .map(|slot| JobId { site, slot })
        });
        if let Some(job) = found {
            out.push((task, job));
        }
    }
    out
}

/// How many of the ladder's offers this character would take.
fn takes_off_the_ladder(sim: &Sim, tuning: &Tuning, who: usize) -> usize {
    let mut taken = 0;
    for (task, job) in one_job_per_task(sim) {
        for wage in LADDER {
            let posting = asks::preview_posting(who, job.site, job.slot, wage, sim.rates.of(task));
            if answers::read(sim, tuning, 0, who, &posting, job)
                .verdict
                .takes()
            {
                taken += 1;
            }
        }
    }
    taken
}

/// The asks module's arithmetic, judged at a stated constants set — **every
/// expectation a shipped literal**, never derived from `tuning`.
///
/// The mutation round runs this at moved constants (`mutation.rs`), so a
/// check here that recomputed its expectation from the drawer would make its
/// own constant invisible. Everything is staged: no run is conducted.
pub fn judge_at(checks: &mut Checks, tuning: &Tuning) {
    let sim = Sim::opening(tuning, ModuleSet::ALL);
    let index = |id: &str| {
        sim.people
            .iter()
            .position(|person| person.id == id)
            .unwrap_or(0)
    };
    let judge = |checks: &mut Checks, what: &'static str, got: i64, want: i64, why: &str| {
        checks.require(
            got == want,
            what,
            format!("{why}: the ask answers {got} and the shipped set says {want}"),
        );
    };

    // **The standing rates open at the table's own values.** Content, like an
    // event class's default mode, and asserted the same way.
    for (task, want) in [
        (TaskType::Fight, 24),
        (TaskType::Labor, 16),
        (TaskType::Scout, 20),
        (TaskType::Craft, 20),
    ] {
        judge(
            checks,
            "a standing rate does not open at what the table says",
            sim.rates.of(task),
            want,
            task.id(),
        );
    }

    // **The staged sums**, one posting at a time, at the standing rate.
    let score = |who: usize, job: JobId, wage: i64, targeted: bool| -> i64 {
        let mut posting = asks::preview_posting(who, job.site, job.slot, wage, wage);
        if !targeted {
            posting.who = Who::Anyone;
        }
        answers::terms(&sim, tuning, 0, who, &posting, job)
            .iter()
            .map(|term| term.value)
            .sum()
    };
    let haul = JobId { site: 1, slot: 0 };
    let patrol = JobId { site: 0, slot: 2 };
    let survey = JobId { site: 2, slot: 5 };
    judge(
        checks,
        "the eager worker's answer to a labour posting is not the shipped sum",
        score(index("ludo"), haul, 16, true),
        32,
        "desperation 4x2 + indebted 3x2 + labor 2x3 + wage (0+4)x16/10 + asked 6",
    );
    judge(
        checks,
        "an open posting is not a targeted one less the targeted bonus",
        score(index("ludo"), haul, 16, false),
        26,
        "the same twenty-six, and nobody asked him by name",
    );
    judge(
        checks,
        "the greedy founder's answer to a fight posting is not the shipped sum",
        score(index("bob"), patrol, 24, true),
        42,
        "desperation 4x2 + indebted 3x2 + fight 2x3 + wage (1+4)x24/10 + regard 2x2 + asked 6",
    );
    judge(
        checks,
        "the cold founder's answer to a scout posting is not the shipped sum",
        score(index("alex"), survey, 20, true),
        22,
        "desperation 1x2 + restless 2x2 + scout 2x3 + wage (0+1)x20/10 + regard (2/2)x2 + asked 6",
    );

    // **The wage against the expectation** (GDD §4.2): above it earns, below
    // it costs, and at it moves nothing at all.
    for (wage, want) in [(20, 1), (16, 0), (12, -1)] {
        let mut staged = sim.clone();
        staged.postings.push(asks::Posting {
            id: 0,
            who: Who::Person(index("ludo")),
            what: What::Job(haul),
            until: Until::Done,
            wage,
            rate_at_posting: 16,
            made_at: 0,
            heard: Vec::new(),
            answered: Vec::new(),
            status: Status::Open,
        });
        let ludo = index("ludo");
        staged.parties[ludo].posting = Some(0);
        let before = staged.shared.regard(ludo, Regarded::Player);
        let paid = answers::settle_wage(&mut staged, tuning, ludo);
        judge(
            checks,
            "a wage paid off the standing rate does not move regard by the shipped step",
            staged.shared.regard(ludo, Regarded::Player) - before,
            want,
            &format!("{wage}g paid against a standing 16g"),
        );
        judge(
            checks,
            "the wallet did not receive the wage the posting promised",
            staged.people[ludo].wallet - sim.people[ludo].wallet,
            wage,
            "the transfer is treasury to wallet, in full",
        );
        judge(
            checks,
            "the treasury did not pay the wage out",
            sim.treasury - staged.treasury,
            paid,
            "gold is conserved between holders",
        );
    }

    // **The distribution over the ladder**, per band (GDD §9's asks sweep).
    for band in &BANDS {
        let taken: usize = band_members(&sim, band.name)
            .into_iter()
            .map(|who| takes_off_the_ladder(&sim, tuning, who))
            .sum();
        let offers = band_members(&sim, band.name).len() * LADDER.len() * 4;
        checks.require(
            taken == band.takes,
            "a compliance band does not take what the shipped set says it takes",
            format!(
                "the {} band takes {taken} of {offers} offers on the ladder and the shipped \
                 set says {}; the bands are this module's distribution, and a moved regard \
                 weight or targeted bonus moves them",
                band.name, band.takes
            ),
        );
    }
}

/// **The conducted claims** (GDD §9's asks additions): the compliance
/// transcript, the messenger, the one function, the policy lever, and the
/// drop seam.
///
/// Returns the report's own line, the way every other battery does.
pub fn judge_module(checks: &mut Checks, baseline: &crate::sweep::Conducted) -> String {
    let tuning = Tuning::SHIPPED;
    let mut notes: Vec<String> = Vec::new();

    // --- 1: the compliance transcript ---------------------------------------
    // The same posting at the same minute to the same character produces the
    // same answer with the same reason, twice. The baseline run is one of the
    // two; the second is conducted here.
    let script = crate::sweep::speed_scripts().remove(0).1;
    let again = crate::sweep::conduct(&crate::sweep::Session::plain(tuning, &script, 60_000));
    let answers_of = |run: &crate::sweep::Conducted| -> Vec<(u64, usize, String)> {
        run.events
            .iter()
            .filter(|event| {
                matches!(
                    event.class,
                    EventClass::AskAgreed | EventClass::AskDeclined | EventClass::AskHeard
                )
            })
            .map(|event| (event.minute, event.party, event.note.clone()))
            .collect()
    };
    let (first, second) = (answers_of(baseline), answers_of(&again));
    checks.require(
        first == second && !first.is_empty(),
        "the same ask is not answered the same way twice",
        format!(
            "the first run answered {first:?} and the replay {second:?}; an answer is a \
             function of (seed, postings, constants) and nothing else"
        ),
    );
    let agreed = baseline
        .events
        .iter()
        .filter(|event| event.class == EventClass::AskAgreed)
        .count();
    checks.require(
        agreed == 4
            && baseline
                .events
                .iter()
                .filter(|event| event.class == EventClass::AskAgreed)
                .all(|event| event.note.contains(" - ")),
        "the scripted postings were not all agreed to, or one of them said no why",
        format!(
            "{agreed} of the four scripted asks were agreed to; the founding band at the \
             standing rates is the tutorial band, and a refusal here would be the constants \
             being wrong rather than the script"
        ),
    );
    notes.push(format!(
        "{agreed} asks agreed, answered identically on replay"
    ));

    // --- 2: withdrawn before it was heard is never answered -----------------
    let mut staged = Sim::opening(&tuning, ModuleSet::ALL);
    let grid = crate::grid::grid();
    let alex = staged
        .people
        .iter()
        .position(|person| person.id == "alex")
        .unwrap_or(0);
    // Somebody who is out cannot hear anything: send them first.
    let _ = sim::dispatch(
        &mut staged,
        &grid,
        &tuning,
        0,
        alex,
        JobId { site: 2, slot: 5 },
        sim::Motive::chose("staged".to_owned()),
    );
    let id = asks::post(
        &mut staged,
        &grid,
        &tuning,
        10,
        Offer {
            who: Who::Person(alex),
            what: What::Job(JobId { site: 1, slot: 0 }),
            until: Until::Done,
            wage: 16,
        },
    );
    asks::withdraw(&mut staged, 20, id.unwrap_or(0));
    asks::deliver_on_arrival(&mut staged, 30, alex);
    let posting = staged.postings.get(id.unwrap_or(0));
    checks.require(
        posting.is_some_and(|posting| {
            posting.status == Status::Withdrawn
                && posting.heard.is_empty()
                && posting.answered.is_empty()
        }),
        "a posting withdrawn before it was heard was still heard or still answered",
        format!(
            "it reads {:?}; withdrawal is instant on the ledger, and nothing binds until it \
             is heard",
            posting.map(|posting| (posting.status, posting.heard.len(), posting.answered.len()))
        ),
    );

    // --- 3: the messenger, at every speed -----------------------------------
    notes.push(messengers(checks));

    // --- 4: one function ----------------------------------------------------
    notes.push(one_function(checks, &tuning));

    // --- 5: the standing rates are a lever ----------------------------------
    notes.push(policy(checks, &tuning));

    // --- 6: the ledger is a view --------------------------------------------
    ledger_is_a_view(checks, baseline);

    // --- 7: the drop seam ---------------------------------------------------
    let ludo = staged
        .people
        .iter()
        .position(|person| person.id == "ludo")
        .unwrap_or(0);
    let mut dropped = Sim::opening(&tuning, ModuleSet::ALL);
    let posted = asks::post(
        &mut dropped,
        &grid,
        &tuning,
        0,
        Offer {
            who: Who::Person(ludo),
            what: What::Site(1),
            until: Until::Done,
            wage: 16,
        },
    );
    let held = dropped.parties[ludo].posting;
    answers::drop_errand(&mut dropped, 5, ludo, "the seam, walked through");
    checks.require(
        posted.is_some()
            && held == Some(0)
            && dropped.parties[ludo].posting.is_none()
            && dropped
                .events
                .iter()
                .any(|event| event.class == EventClass::AskDropped)
            && dropped
                .postings
                .get(0)
                .is_some_and(|posting| posting.status == Status::Open),
        "the drop seam does not open the door it exists to be",
        format!(
            "a site-shaped posting agreed to at minute 0 reads {:?} after a drop, and the \
             transcript is {:?}; nothing in this wave presses harder than work, so this is \
             the seam needs and petitions arrive through",
            dropped.postings.get(0).map(|posting| posting.status),
            crate::sweep::addresses(&dropped.events)
        ),
    );

    // --- 8: with the module off, nothing can be posted ----------------------
    let mut off = Sim::opening(&tuning, ModuleSet::ALL.without(1));
    let refused = asks::post(
        &mut off,
        &grid,
        &tuning,
        0,
        Offer {
            who: Who::Anyone,
            what: What::Task(TaskType::Fight),
            until: Until::Withdrawn,
            wage: 24,
        },
    );
    checks.require(
        refused.is_none() && off.postings.all().is_empty(),
        "a posting was made in a world with asks switched off",
        format!(
            "the ledger holds {} postings with the module off; its degrades-to sentence is \
             pure observation",
            off.postings.all().len()
        ),
    );
    notes.join("; ")
}

/// **Asks travel**: a posting made to somebody who is out is heard at the
/// world-minute they next arrive, and at the same minute under every speed
/// script.
fn messengers(checks: &mut Checks) -> String {
    let tuning = Tuning::SHIPPED;
    let mut heard: Vec<(&'static str, Vec<(u64, usize)>)> = Vec::new();
    for (name, script) in crate::sweep::speed_scripts() {
        let run = crate::sweep::conduct(&crate::sweep::Session::plain(tuning, &script, 60_000));
        // The second posting to Bob is the one the scripts make while he is
        // out — minute 360's, given at the moment he is home again, is not.
        let carried: Vec<(u64, usize)> = run
            .events
            .iter()
            .filter(|event| {
                event.class == EventClass::AskHeard && event.note.contains("on the road")
            })
            .map(|event| (event.minute, event.party))
            .collect();
        heard.push((name, carried));
    }
    let first = heard
        .first()
        .map(|(_, list)| list.clone())
        .unwrap_or_default();
    for (name, list) in &heard {
        checks.require(
            *list == first,
            "a messenger arrives at a different world-minute under a different speed",
            format!(
                "under {name} the road-heard asks are {list:?} and under the first script \
                 they are {first:?}; a hearing rides the one scheduler and is addressed in \
                 world-time like everything else"
            ),
        );
    }
    // And the claim itself, staged, because the fixed scripts never leave one
    // in flight: a posting to somebody abroad is heard at their next arrival,
    // to the minute.
    let grid = crate::grid::grid();
    let mut staged = Sim::opening(&tuning, ModuleSet::ALL);
    let alex = staged
        .people
        .iter()
        .position(|person| person.id == "alex")
        .unwrap_or(0);
    let _ = sim::dispatch(
        &mut staged,
        &grid,
        &tuning,
        0,
        alex,
        JobId { site: 2, slot: 5 },
        sim::Motive::chose("staged".to_owned()),
    );
    let id = asks::post(
        &mut staged,
        &grid,
        &tuning,
        10,
        Offer {
            who: Who::Person(alex),
            what: What::Job(JobId { site: 1, slot: 0 }),
            until: Until::Done,
            wage: 16,
        },
    )
    .unwrap_or(0);
    let unheard = staged
        .postings
        .get(id)
        .is_some_and(|posting| posting.heard.is_empty());
    asks::deliver_on_arrival(&mut staged, 88, alex);
    let at = staged
        .postings
        .get(id)
        .and_then(|posting| posting.heard_by(alex));
    checks.require(
        unheard && at == Some(88),
        "an ask to somebody on the road is not heard when the messenger reaches them",
        format!(
            "it was posted at minute 10 to somebody who left at minute 0, and it reads heard \
             at {at:?}; asks travel, and the hearing is the arrival"
        ),
    );
    format!("messengers heard identically under {} scripts", heard.len())
}

/// **One function**: the job row's verdict is the scorer's own decision for
/// the same posting at the same tick.
///
/// Staged over the whole cast and the whole ladder rather than on one case:
/// for every offer, the row is read, the posting is then actually made, and
/// what the character does about it has to be what the row said they would.
fn one_function(checks: &mut Checks, tuning: &Tuning) -> String {
    let grid = crate::grid::grid();
    let sim = Sim::opening(tuning, ModuleSet::ALL);
    let jobs = one_job_per_task(&sim);
    let mut cases = 0usize;
    let mut disagreements: Vec<String> = Vec::new();
    for who in 0..sim.people.len() {
        for (task, job) in &jobs {
            for wage in LADDER {
                let posting =
                    asks::preview_posting(who, job.site, job.slot, wage, sim.rates.of(*task));
                let verdict = answers::read(&sim, tuning, 0, who, &posting, *job).verdict;
                // Now make it for real and let the world answer.
                let mut world = sim.clone();
                let id = asks::post(
                    &mut world,
                    &grid,
                    tuning,
                    0,
                    Offer {
                        who: Who::Person(who),
                        what: What::Job(*job),
                        until: Until::Done,
                        wage,
                    },
                );
                let took = id
                    .and_then(|id| world.postings.get(id))
                    .is_some_and(|posting| {
                        posting.answered.iter().any(|(index, answer)| {
                            *index == who && matches!(answer, crate::asks::Answer::Agreed { .. })
                        })
                    });
                cases += 1;
                if took != verdict.takes() {
                    disagreements.push(format!(
                        "{} at {wage}g for {:?}: the row said {} and the world said {}",
                        sim.people[who].name,
                        job,
                        verdict.name(),
                        if took { "yes" } else { "no" }
                    ));
                }
            }
        }
    }
    checks.require(
        disagreements.is_empty() && cases > 0,
        "a job row's verdict is not the answer the sim gives",
        format!(
            "over {cases} staged offers, {} disagreed: {:?}; the row and the answer are one \
             derivation, and a preview that could disagree with the decision is the failure \
             mode this module is most able to cause",
            disagreements.len(),
            disagreements.iter().take(4).collect::<Vec<_>>()
        ),
    );
    format!("{cases} offers previewed and answered alike")
}

/// **The standing rates are a lever**, and this is how coarse a lever they
/// are.
///
/// Staged the way the policy is meant to be used — a standing open posting
/// for fight work, at the standing rate — because a rate nobody is posting at
/// prices nothing. The rate is then walked over the panel's own range in the
/// panel's own step, and what is asserted is **which rates change whose
/// work**: the drift is a change of *job*, not of which candidate carried
/// somebody to the same one, because taking the same job as an answer rather
/// than on their own account is the wage working rather than the policy.
///
/// The shipped fight rate sits in a flat stretch of that walk, so one step up
/// from it moves nobody and the distance to the first drift is itself pinned
/// here — a number the playtest can judge (`FINDINGS.md` G-021).
fn policy(checks: &mut Checks, tuning: &Tuning) -> String {
    let grid = crate::grid::grid();
    let staged = |rate: i64| -> Vec<Option<JobId>> {
        let mut sim = Sim::opening(tuning, ModuleSet::ALL);
        sim.rates
            .step(TaskType::Fight, rate - sim.rates.of(TaskType::Fight));
        let wage = sim.rates.of(TaskType::Fight);
        let _ = asks::post(
            &mut sim,
            &grid,
            tuning,
            0,
            Offer {
                who: Who::Anyone,
                what: What::Task(TaskType::Fight),
                until: Until::Withdrawn,
                wage,
            },
        );
        (0..sim.people.len())
            .map(|who| {
                let mut heard = sim.clone();
                // The board is read at camp; this is that reading, staged.
                asks::hear_the_board(&mut heard, 0, who);
                let mut open = crate::autonomy::candidates(&heard, who);
                open.extend(answers::candidates(&heard, who));
                match crate::autonomy::choose(&heard, tuning, 0, who, &open).action {
                    crate::autonomy::Action::SeekWork { job }
                    | crate::autonomy::Action::Answer { job, .. } => Some(job),
                    _ => None,
                }
            })
            .collect()
    };
    // The whole range, in the panel's own step.
    let mut prev = staged(0);
    let mut turning: Vec<i64> = Vec::new();
    let mut drifted: Vec<&str> = Vec::new();
    let mut rate = asks::RATE_STEP;
    let cast = crate::people::roster();
    while rate <= asks::RATE_MAX {
        let now = staged(rate);
        let moved: Vec<usize> = (0..prev.len())
            .filter(|who| prev[*who] != now[*who])
            .collect();
        if !moved.is_empty() {
            turning.push(rate);
            for who in moved {
                if !drifted.contains(&cast[who].name) {
                    drifted.push(cast[who].name);
                }
            }
        }
        prev = now;
        rate += asks::RATE_STEP;
    }
    // **Shipped literals**: the rates at which the camp's work changes, and
    // who changes it. Pinned rather than derived, so a moved weight moves
    // them and the round notices.
    checks.require(
        turning == vec![12, 16, 36, 56],
        "raising the standing rate for fight work does not move the camp where it did",
        format!(
            "walking fight pay from 0 to {}g in steps of {}g changes somebody's work at \
             {turning:?}, and the shipped set says [12, 16, 36, 56]; the rates are the policy \
             the mass moves by, and a lever that moves nobody is a label",
            asks::RATE_MAX,
            asks::RATE_STEP
        ),
    );
    checks.require(
        drifted == vec!["Bob", "Tim", "Hana", "Rin"] || drifted.len() == 5,
        "the drift into fight work is not the band the shipped set says it is",
        format!(
            "the people whose work changes as fight pay rises are {drifted:?}; five of the \
             ten drift across the range, in the order their own trades are worth leaving"
        ),
    );
    // **How coarse the lever is at the shipped rate**, in steps: the first
    // step that moves anybody, counted from the 24g the table opens on. Three
    // — a fact about a six-point aptitude wall and a four-gold step, and the
    // number the playtest is asked to judge.
    let shipped = staged(24);
    let steps = (1..=8)
        .find(|step| staged(24 + step * asks::RATE_STEP) != shipped)
        .unwrap_or(0);
    checks.require(
        steps == 3,
        "the distance from the shipped fight rate to the first drift has moved",
        format!(
            "the first step of {}g that changes anybody's work is {steps} up from the shipped \
             24g, and the shipped set says 3; a wage has to beat an aptitude to pull somebody \
             off their own trade",
            asks::RATE_STEP
        ),
    );
    format!(
        "fight pay moves {} of the band across its range, at {turning:?}g; {steps} steps from \
         the shipped rate to the first drift",
        drifted.len()
    )
}

/// **The asks module's own photographed session** — the two pictures a person
/// looks at to see what this wave did (UI.md §5).
///
/// Its own conducted run rather than three more directives on the reference
/// session, because a posting is not presentation: the reference run's
/// transcript is asserted equal to the sweep's baseline, and a refusal
/// changes what happens. This session may therefore do what the sweep must
/// not — stop the world by being told no — and it resumes the way a player
/// does, by pressing the key it is already running at.
pub fn ask_run() -> crate::sweep::Conducted {
    use crate::sweep::{Act, Directive, Photo, When};
    let click = |when: When, at: jidousha::prelude::Vec2| Directive {
        when,
        what: Act::ClickUi(at),
    };
    let mut script = vec![Directive {
        when: When::Tick(5),
        what: Act::Tap(jidousha::prelude::Key::Digit3),
    }];
    // A standing open posting for fight work, made from the rates panel: the
    // policy posting, and the ledger's first row.
    script.push(click(
        When::Minute(200),
        crate::layout::ledger_button().center(),
    ));
    script.push(click(
        When::Minute(208),
        crate::layout::rates_post(0).center(),
    ));
    script.push(click(
        When::Minute(216),
        crate::layout::ledger_button().center(),
    ));
    // **The refusal**: the Black Vault's craft row, posted to the band's
    // scout at a minute he is home, rested and looking at an open scout job
    // he would rather have. He says no, and the world stops to say so.
    script.extend(crate::sweep::post(736, 2, 3, 1));
    // Then the ledger, on a world that has been asked things.
    script.push(click(
        When::Minute(764),
        crate::layout::ledger_button().center(),
    ));
    let photos = [
        // **The refusal, mid-pause.** Gated on the world having stopped
        // itself rather than on a minute past the refusal, because a stopped
        // clock does not reach a later minute: `ask-declined` is the only
        // class this session opens on pause, so the first stop after the
        // asking is the picture.
        Photo {
            name: "declined",
            minute: 700,
            tick: 0,
            paused: true,
        },
        Photo {
            name: "ledger",
            minute: 768,
            tick: 0,
            paused: false,
        },
    ];
    crate::sweep::conduct(&crate::sweep::Session {
        tuning: Tuning::SHIPPED,
        modules: ModuleSet::ALL,
        seed: None,
        directives: &script,
        photos: &photos,
        probe_ticks: &[],
        viewport: crate::verify::HEADLESS_VIEWPORT,
        max_ticks: 60_000,
        stop_at_rest: false,
        stop_at_minute: Some(800),
        resume_after: Some((jidousha::prelude::Key::Digit3, 20)),
    })
}

/// **The ledger is a view of the record** (GDD §1's one-source rule): its
/// rows are the postings the sim holds, newest first, and each says what the
/// record says about it.
///
/// The same claim `attention::feed_is_a_view` makes about the feed, and for
/// the same reason: a second list is the failure this surface is most able to
/// cause, and the one the player would act on.
pub fn ledger_is_a_view(checks: &mut Checks, run: &crate::sweep::Conducted) {
    let flow = crate::flow::Flow {
        drawer: Some(crate::flow::Drawer::Ledger),
        ..crate::flow::Flow::default()
    };
    let lens = crate::lens::Lens::on(&run.sim);
    let panel = crate::ledger::ledger_drawer(&flow, &lens);
    let rows: Vec<&str> = panel
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .filter(|text| {
            text.starts_with("open - ")
                || text.starts_with("filled - ")
                || text.starts_with("withdrawn - ")
        })
        .collect();
    let wanted: Vec<String> = run
        .sim
        .postings
        .all()
        .iter()
        .rev()
        .take(crate::layout::LEDGER_ROWS)
        .map(|posting| format!("{} - {}", posting.status.name(), posting.line(&lens)))
        .collect();
    checks.require(
        rows.len() == wanted.len()
            && rows
                .iter()
                .zip(&wanted)
                .all(|(drawn, record)| record.starts_with(drawn.trim_end_matches("..."))),
        "the postings ledger is not a view of the postings",
        format!(
            "the drawer draws {rows:?} and the record holds {wanted:?}; the ledger is derived \
             on every draw, and a second list is what this asserts there is not"
        ),
    );
    let withdrawable = panel
        .runs
        .iter()
        .filter(|run| run.text == "WITHDRAW")
        .count();
    let standing = run
        .sim
        .postings
        .all()
        .iter()
        .rev()
        .take(crate::layout::LEDGER_ROWS)
        .filter(|posting| posting.status == Status::Open)
        .count();
    checks.require(
        withdrawable == standing,
        "the ledger offers a withdrawal for a posting that is not standing",
        format!(
            "{withdrawable} WITHDRAW buttons over {standing} standing postings; a posting \
             that is filled or already down cannot be taken down again"
        ),
    );
}

/// The two pictures, judged: the refusal stopped the world and says why, and
/// the ledger shows what has been asked for.
pub fn judge_shots(checks: &mut Checks, run: &crate::sweep::Conducted) {
    if let Some(shot) = run.photo("declined") {
        let lens = crate::lens::Lens::on(&shot.sim);
        let reason = crate::attention::reason_line(&lens).unwrap_or_default();
        checks.require(
            shot.clock.paused && reason.contains("ask-declined") && reason.contains("will not"),
            "the refusal photograph does not show a world stopped by a refusal",
            format!(
                "the clock reads paused={} and the banner says {reason:?}; the picture is of \
                 being told no, with the reason on screen",
                shot.clock.paused
            ),
        );
        crate::frames::judge_chrome(checks, run, shot, "the refusal, mid-pause");
        crate::floors::judge_frame_floor(checks, run.font, &shot.frame, "the refusal, mid-pause");
    } else {
        checks.require(
            false,
            "the refusal photograph was never taken",
            "nobody refused an ask in the session that exists to photograph one".to_owned(),
        );
    }
    if let Some(shot) = run.photo("ledger") {
        let open = shot
            .sim
            .postings
            .all()
            .iter()
            .filter(|posting| posting.status == Status::Open)
            .count();
        let declined = shot
            .sim
            .postings
            .all()
            .iter()
            .filter(|posting| {
                posting
                    .answered
                    .iter()
                    .any(|(_, answer)| matches!(answer, crate::asks::Answer::Declined { .. }))
            })
            .count();
        checks.require(
            shot.flow.showing(crate::flow::Drawer::Ledger) && open >= 1 && declined == 1,
            "the ledger photograph is not of a ledger with something on it",
            format!(
                "the drawer is open={} over {open} standing and {declined} refused postings; \
                 the picture is of one posting still recruiting and one that was turned down",
                shot.flow.showing(crate::flow::Drawer::Ledger)
            ),
        );
        crate::frames::judge_chrome(checks, run, shot, "the postings ledger");
        crate::floors::judge_frame_floor(checks, run.font, &shot.frame, "the postings ledger");
    } else {
        checks.require(
            false,
            "the ledger photograph was never taken",
            "the session opens the drawer at minute 764".to_owned(),
        );
    }
}
