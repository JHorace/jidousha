//! **Answering an ask** — the half of the postings module that decides and
//! pays (GDD's postings section, wave 1.2).
//!
//! `asks.rs` is the record, the rates and the hearing; this file is what
//! happens once somebody has heard something: the candidates a posting is
//! worth weighing as, the terms it is weighed by, the read a job row shows,
//! and the money that moves when the work is done.
//!
//! **There is no decision function here.** [`read`] and the answer both call
//! `autonomy::choose` over the same candidate list, which is what makes the
//! job row's verdict and the sim's answer one derivation rather than two that
//! agree today. Its own file because `asks.rs` was two subjects long, and
//! because this is the half a later wave's pressure (needs, petitions) will
//! reach into through [`drop_errand`].

use crate::autonomy::{self, Action};
use crate::constants::Tuning;
use crate::grid::Grid;
use crate::sim::{self, JobId, Sim};
use crate::stores::Regarded;

use crate::asks::{Answer, MODULE, Posting, RELUCTANT_MARGIN, Status, Until};

/// **The candidates a heard posting is worth weighing as** — one per open job
/// the posting covers.
///
/// Built by the caller and weighed by the scorer, which is the shape
/// `autonomy.rs` was written for: an ask is a fourth [`Action`], not a fork.
pub fn candidates(sim: &Sim, who: usize) -> Vec<Action> {
    if !sim.modules.enabled(MODULE) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for posting in sim.postings.all() {
        if !posting.open_to(who) {
            continue;
        }
        for (site, board) in sim.sites.iter().enumerate() {
            for slot in board.open_slots() {
                let job = JobId { site, slot };
                if posting.what.covers(sim, job) {
                    out.push(Action::Answer {
                        posting: posting.id,
                        job,
                    });
                }
            }
        }
    }
    out
}

/// What the row says the character would do, in three words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The posting wins, and comfortably.
    WouldTake,
    /// The posting wins by less than [`RELUCTANT_MARGIN`].
    Reluctant,
    /// Something else wins.
    WouldRefuse,
}

impl Verdict {
    /// The words a job row prints.
    pub fn name(self) -> &'static str {
        match self {
            Verdict::WouldTake => "would take it",
            Verdict::Reluctant => "reluctant",
            Verdict::WouldRefuse => "would refuse",
        }
    }

    /// Whether this reading means they take the work.
    pub fn takes(self) -> bool {
        matches!(self, Verdict::WouldTake | Verdict::Reluctant)
    }
}

/// **The read the job row shows, and the answer the sim gives — one
/// comparison.**
///
/// A row previews a posting that does not exist yet, so this cannot ask the
/// ledger; what it does instead is the arithmetic `autonomy::choose`
/// performs: the ask's own sum against the best of everything else this
/// character could do at this minute, over the candidate list a rescore would
/// build. The sign of that comparison **is** the decision — `choose` takes
/// the ask exactly when it outscores every rival, and a tie goes to the rival
/// because asks are appended last — so the row and the sim cannot disagree by
/// construction, and `compliance::one_function` asserts it over a played
/// world anyway.
///
/// The words are the same words, from the same maker: what they would take it
/// for, or what they would rather do.
pub fn read(
    sim: &Sim,
    tuning: &Tuning,
    now: u64,
    who: usize,
    posting: &Posting,
    job: JobId,
) -> (Verdict, String) {
    let mine = terms(sim, tuning, now, who, posting, job);
    let score: i64 = mine.iter().map(|term| term.value).sum();
    let ask = Action::Answer {
        posting: posting.id,
        job,
    };
    let mut open = autonomy::candidates(sim, who);
    open.extend(candidates(sim, who));
    open.retain(|action| *action != ask);
    let rival = autonomy::choose(sim, tuning, now, who, &open);
    if score > rival.score {
        let verdict = if score - rival.score >= RELUCTANT_MARGIN {
            Verdict::WouldTake
        } else {
            Verdict::Reluctant
        };
        return (verdict, autonomy::words(ask, &mine));
    }
    (Verdict::WouldRefuse, rival.reason)
}

/// The wants of theirs that cover a piece of work, named — the same phrase
/// the scorer's own want term uses, so one sentence describes one term
/// wherever it is read.
fn wants(person: &crate::people::Character) -> String {
    person
        .traits
        .iter()
        .map(|id| id.def())
        .filter(|def| def.pressure != 0)
        .map(|def| def.name)
        .collect::<Vec<_>>()
        .join(" and ")
}

/// **What a posting is worth to somebody**, term by term.
///
/// The four the design names, over the work's own terms: the wage's pull (the
/// pot term, by pot affinity **and** desperation — a wage is a pot the player
/// fills), a regard-for-the-player term (the loyal comply for less; the cold
/// need paying), the fit (the task's aptitude row), and the targeted bonus,
/// which is being asked by name rather than reading a notice. **No term
/// branches on a trait id**, exactly as in the rest of the scorer.
///
/// The job's own pot is **not** among them: the pot is the player's, and the
/// worker's money is the wage. Weighing both would pay the same gold twice.
pub fn terms(
    sim: &Sim,
    tuning: &Tuning,
    now: u64,
    who: usize,
    posting: &Posting,
    job: JobId,
) -> Vec<autonomy::Term> {
    let mut out = Vec::new();
    let Some(person) = sim.people.get(who) else {
        return out;
    };
    let Some(quest) = sim
        .sites
        .get(job.site)
        .and_then(|site| site.quest(job.slot))
        .copied()
    else {
        return out;
    };
    out.push(autonomy::Term {
        what: "need",
        value: person.desperation * tuning.need_weight,
        because: "needs the money".to_owned(),
    });
    let pressure = crate::traits::pressure_toward(quest.task, &person.traits);
    if pressure != 0 {
        out.push(autonomy::Term {
            what: "want",
            value: pressure * tuning.want_weight,
            because: format!("{}, and this is {} work", wants(person), quest.task.id()),
        });
    }
    let apt = crate::traits::competence_at(quest.task, &person.traits);
    if apt != 0 {
        out.push(autonomy::Term {
            what: "aptitude",
            value: apt * tuning.apt_weight,
            because: format!("good at {} work", quest.task.id()),
        });
    }
    // The wage: a pot the player fills, felt by affinity and by need.
    let pull = crate::traits::pot_pull_of(&person.traits) + person.desperation;
    if pull != 0 && posting.wage != 0 {
        out.push(autonomy::Term {
            what: "wage",
            value: pull * posting.wage * tuning.pot_weight / 10,
            because: format!("the wage is {}g", posting.wage),
        });
    }
    // What they think of the player. The loyal comply for less and the cold
    // need paying, and both of those are the carrier's own multipliers.
    let felt =
        crate::traits::weighted_regard(sim.shared.regard(who, Regarded::Player), &person.traits);
    if felt != 0 {
        out.push(autonomy::Term {
            what: "regard",
            value: felt * tuning.regard_weight,
            because: if felt > 0 {
                "would do it for you".to_owned()
            } else {
                "owes you nothing".to_owned()
            },
        });
    }
    if posting.targets(who) {
        out.push(autonomy::Term {
            what: "asked",
            value: tuning.ask_targeted,
            because: "was asked by name".to_owned(),
        });
    }
    if now < sim.parties.get(who).map_or(0, |party| party.rested_until) {
        out.push(autonomy::Term {
            what: "rest",
            value: -tuning.rest_weight,
            because: "not stopped since the last job".to_owned(),
        });
    }
    out
}

/// **They agreed**: the posting is answered, the job is theirs, and the
/// journey is the ordinary one.
///
/// Called from `autonomy::act`, which is where every errand begins — an ask
/// taken is a dispatch like any other, and there is no second travel path.
pub fn agree(
    sim: &mut Sim,
    grid: &Grid,
    tuning: &Tuning,
    now: u64,
    who: usize,
    judged: autonomy::Judged,
) {
    let Action::Answer { posting: id, job } = judged.action else {
        return;
    };
    let reason = judged.reason;
    let Some(posting) = sim.postings.get(id) else {
        return;
    };
    let wage = posting.wage;
    let until = posting.until;
    let name = sim
        .sites
        .get(job.site)
        .and_then(|site| site.quest(job.slot))
        .map_or("that job", |quest| quest.name);
    let tile = sim
        .parties
        .get(who)
        .map_or(crate::grid::LOCATIONS[crate::grid::TOWN].tile, |party| {
            party.tile
        });
    sim.emit_ask(
        now,
        crate::attention::EventClass::AskAgreed,
        who,
        tile,
        format!("took {name} for {wage}g - {reason}"),
    );
    let dispatched = sim::dispatch(sim, grid, tuning, now, who, job, sim::Motive::asked(reason));
    if dispatched.is_err() {
        // The row went while they were deciding. The posting stands, and the
        // failure is said rather than swallowed (CLAUDE.md: no silent
        // failure) — the next rescore weighs whatever is left.
        sim.emit_ask(
            now,
            crate::attention::EventClass::AskDropped,
            who,
            tile,
            format!("could not start {name} - the row went while they answered"),
        );
        return;
    }
    if let Some(party) = sim.parties.get_mut(who) {
        party.posting = Some(id);
    }
    if let Some(posting) = sim.postings.get_mut(id) {
        posting.answered.push((who, Answer::Agreed { job }));
        if until == Until::Done {
            posting.status = Status::Filled;
        }
    }
}

/// **They declined**: the scorer chose something else, and a posting made to
/// somebody by name says so in words.
///
/// An open posting simply goes unfilled — nobody refused a notice on a board,
/// and an event for every character who read one and did something else would
/// be a feed nobody could read.
pub fn decline(sim: &mut Sim, now: u64, who: usize, id: usize, reason: &str) {
    let Some(posting) = sim.postings.get(id) else {
        return;
    };
    if !posting.targets(who) || posting.status != Status::Open {
        return;
    }
    let what = posting.what.label(sim);
    let tile = sim
        .parties
        .get(who)
        .map_or(crate::grid::LOCATIONS[crate::grid::TOWN].tile, |party| {
            party.tile
        });
    sim.emit_ask(
        now,
        crate::attention::EventClass::AskDeclined,
        who,
        tile,
        format!("will not take {what} - {reason}"),
    );
    if let Some(posting) = sim.postings.get_mut(id) {
        posting.answered.push((
            who,
            Answer::Declined {
                reason: reason.to_owned(),
            },
        ));
    }
}

/// **Everything this character was asked and did not take.**
///
/// Called after every rescore: whatever they chose, a named ask they did not
/// choose is a refusal, and a refusal is said. One place, so the answer to a
/// posting is the same whether the rescore was the cadence's or the ask's own
/// arrival.
pub fn record_refusals(sim: &mut Sim, now: u64, who: usize, taken: Option<usize>, reason: &str) {
    if !sim.modules.enabled(MODULE) {
        return;
    }
    let refused = sim.postings.ids_where(|posting| {
        posting.status == Status::Open
            && posting.targets(who)
            && posting.heard_by(who).is_some()
            && Some(posting.id) != taken
            && !posting
                .answered
                .iter()
                .any(|(index, answer)| *index == who && matches!(answer, Answer::Declined { .. }))
    });
    for id in refused {
        decline(sim, now, who, id, reason);
    }
}

/// **The wage is paid on completion** (GDD §4.1's TRANSFER port), and the
/// paying moves regard by what it was measured against (GDD §4.2).
///
/// Treasury to wallet, in full: the promise was the posting's, and a
/// settlement that paid what was left rather than what was owed would be a
/// silent failure with a debtor's face. Expectation is the standing rate the
/// posting recorded — paying above it earns a little regard, below it costs a
/// little, whether or not they took it for less.
pub fn settle_wage(sim: &mut Sim, tuning: &Tuning, who: usize) -> i64 {
    let Some(id) = sim.parties.get(who).and_then(|party| party.posting) else {
        return 0;
    };
    let Some(posting) = sim.postings.get(id) else {
        return 0;
    };
    let (wage, expectation) = (posting.wage, posting.rate_at_posting);
    sim.treasury -= wage;
    if let Some(person) = sim.people.get_mut(who) {
        person.wallet += wage;
    }
    let delta = match wage.cmp(&expectation) {
        std::cmp::Ordering::Greater => tuning.wage_regard,
        std::cmp::Ordering::Less => -tuning.wage_regard,
        std::cmp::Ordering::Equal => 0,
    };
    if delta != 0 {
        sim.shared
            .adjust_regard(tuning, who, Regarded::Player, delta);
    }
    if let Some(party) = sim.parties.get_mut(who) {
        party.posting = None;
    }
    wage
}

/// **The drop seam** — a character abandons a posting's job because something
/// pressed harder.
///
/// Nothing presses harder than work in this wave, so nothing calls this with
/// a live errand: needs and petitions are what supply the pressure, and this
/// is the door they arrive through. It is a function and an event class
/// rather than a comment because a seam nobody can call is a seam nobody will
/// find (the battery walks through it on a staged world).
pub fn drop_errand(sim: &mut Sim, now: u64, who: usize, why: &str) {
    let Some(id) = sim.parties.get(who).and_then(|party| party.posting) else {
        return;
    };
    let tile = sim
        .parties
        .get(who)
        .map_or(crate::grid::LOCATIONS[crate::grid::TOWN].tile, |party| {
            party.tile
        });
    let what = sim
        .postings
        .get(id)
        .map_or_else(|| "the work".to_owned(), |posting| posting.what.label(sim));
    sim.emit_ask(
        now,
        crate::attention::EventClass::AskDropped,
        who,
        tile,
        format!("dropped {what} - {why}"),
    );
    if let Some(party) = sim.parties.get_mut(who) {
        party.posting = None;
    }
    if let Some(posting) = sim.postings.get_mut(id) {
        posting.status = Status::Open;
    }
}
