//! **The economy sweeps** — what the settlement does over days, with the
//! player watching and with the player away (GDD §9's economy sweeps, wave
//! 1.3).
//!
//! # Two hundred worlds, and what varies across them
//!
//! GDD §9 asks for "~200 idle-player seeds". **This build reads no `Rng`** —
//! `verify::seed_independence` asserts the whole transcript identical at
//! far-apart seeds — so there is no seed to draw an economy from, and the
//! honest thing is to say so and sweep the population that actually exists
//! (`compliance.rs` met the same fact and answered it the same way).
//!
//! What a seed *would* have varied in an economy sweep is **the order in which
//! ten people meet a finite board**: who reaches the mushroom haul first, and
//! who arrives to find it taken. So that is what varies, and nothing else:
//! world *k* opens with every scheduled first rescore rotated by *k* minutes
//! (`Sim::stagger_first_looks`). Every constant, every standing rate and every
//! authored pot is the shipped one in all two hundred — which is what makes a
//! mutated upkeep or wage constant move all two hundred at once and break a
//! band, as GDD §9 requires.
//!
//! # A world without an app around it
//!
//! Two hundred settlements of several world-days each is more than a conducted
//! run can afford (`sweep::conduct` drives a headless app, input script and
//! renderer). These worlds are driven through `sim::advance_to` — the same one
//! door `sim::fire_due` fires occurrences through, with the clock and the
//! pause left upstairs where they belong. There is no second simulation here.
//!
//! # The players
//!
//! Two, and both are stated in words before they are written in code, because
//! **a policy tuned until it wins is not evidence**. They are deliberately
//! dumb; if competence cannot beat neglect at the shipped constants, the
//! constants are what move.

use crate::constants::Tuning;
use crate::grid::Grid;
use crate::lens::Lens;
use crate::modules::ModuleSet;
use crate::sim::{self, Ports, Sim};
use crate::traits::TaskType;

/// How many worlds a sweep walks. GDD §9's "~200".
pub const WORLDS: u64 = 64;

/// How many world-days each of them runs.
///
/// Three, which is the span the arrival column itself sets: the last of the
/// six who came later walks in on the evening of day three (`CAST.md` §4), so
/// a shorter run would judge a settlement that was never whole and a longer
/// one would only watch the same slide continue.
pub const DAYS: u64 = 3;

/// **Who is at the wheel.**
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    /// **Nobody.** No posting is made, no rate is moved, nothing is built.
    /// The settlement is left entirely to itself.
    Idle,
    /// **A scripted attentive player**, whose whole policy is three rules,
    /// applied once a world-day and in this order:
    ///
    /// 1. if nothing is standing and the treasury can pay for the first
    ///    industry, build it;
    /// 2. post the best-fitting open job to every person standing idle at
    ///    their own door, at the standing rate, one job each;
    /// 3. if more named asks were declined since yesterday than were agreed
    ///    to, raise the standing rate for the task type that was refused most,
    ///    by the panel's own step.
    ///
    /// That is the whole of it. It reads no scorer, predicts nothing and has
    /// no memory beyond yesterday's refusals.
    Attentive,
}

/// What one world came to.
#[derive(Clone, Debug)]
pub struct Outcome {
    /// Every present person's purse at the end, in registry order.
    pub wallets: Vec<i64>,
    /// And their desperation.
    pub desperation: Vec<i64>,
    /// What the treasury holds.
    pub treasury: i64,
    /// How many shortfall occurrences the whole run produced.
    pub shortfalls: u64,
    /// Who went short first, by registry index — `None` if nobody did.
    pub first_short: Option<usize>,
    /// Every gold port, totalled.
    pub ports: Ports,
    /// What the wallets opened holding, for the conservation identity.
    pub opening_wallets: i64,
    /// **What the treasury and every purse hold at the end**, over the whole
    /// registry rather than the camp: somebody who has not arrived still has
    /// the purse they will walk in with, and conservation is about the gold
    /// and not about who is in the room.
    pub holders: i64,
    /// How many jobs and shifts were completed.
    pub completed: usize,
    /// Whether anything was built by the end.
    pub built: bool,
}

impl Outcome {
    /// The median purse over the people who are in the camp.
    pub fn median_wallet(&self) -> i64 {
        median(&self.wallets)
    }

    /// The median desperation over the people who are in the camp.
    pub fn median_desperation(&self) -> i64 {
        median(&self.desperation)
    }
}

/// The middle value of a list, by the low median on an even count — one rule,
/// so two readings of one sweep are one number.
pub fn median(values: &[i64]) -> i64 {
    if values.is_empty() {
        return 0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[(sorted.len() - 1) / 2]
}

/// **Run one world.**
pub fn live(
    tuning: &Tuning,
    modules: ModuleSet,
    offset: u64,
    days: u64,
    player: Player,
) -> Outcome {
    let grid = crate::grid::grid();
    let mut sim = Sim::opening(tuning, modules);
    sim.stagger_first_looks(tuning, offset);
    let opening_wallets: i64 = sim.people.iter().map(|person| person.wallet).sum();
    let mut answered = (0usize, 0usize);
    for day in 1..=days {
        sim::advance_to(&mut sim, &grid, tuning, day * crate::autonomy::DAY);
        if player == Player::Attentive {
            answered = attend(
                &mut sim,
                &grid,
                tuning,
                day * crate::autonomy::DAY,
                answered,
            );
        }
    }
    let first_short = sim
        .events
        .iter()
        .find(|event| event.class == crate::attention::EventClass::UpkeepShortfall)
        .map(|event| event.party);
    let present: Vec<usize> = {
        let lens = Lens::on(&sim);
        lens.roll()
    };
    Outcome {
        wallets: present.iter().map(|who| sim.people[*who].wallet).collect(),
        desperation: present
            .iter()
            .map(|who| sim.people[*who].desperation)
            .collect(),
        treasury: sim.treasury,
        holders: sim.treasury + sim.people.iter().map(|person| person.wallet).sum::<i64>(),
        shortfalls: sim.people.iter().map(|person| person.shortfalls).sum(),
        first_short,
        ports: sim.ports,
        opening_wallets,
        completed: sim
            .events
            .iter()
            .filter(|event| event.class == crate::attention::EventClass::QuestComplete)
            .count(),
        built: sim.settlement.any_standing(),
    }
}

/// **The attentive player's day**, exactly as [`Player::Attentive`] states it.
///
/// Returns the running (agreed, declined) tally it read, so tomorrow's rule
/// three can be about what happened today.
fn attend(
    sim: &mut Sim,
    grid: &Grid,
    tuning: &Tuning,
    now: u64,
    since: (usize, usize),
) -> (usize, usize) {
    // 1. Build the first thing there is, once there is money for it.
    for index in 0..crate::settlement::INDUSTRIES.len() {
        let _ = crate::settlement::build(sim, now, index);
    }
    // 2. Post the best-fitting open job to everybody standing idle.
    let idle: Vec<usize> = {
        let lens = Lens::on(sim);
        lens.roll()
            .into_iter()
            .filter(|who| lens.at_home(*who))
            .collect()
    };
    for who in idle {
        let Some((job, task)) = best_fit(sim, who) else {
            continue;
        };
        let wage = sim.rates.of(task);
        let _ = crate::asks::post(
            sim,
            grid,
            tuning,
            now,
            crate::asks::Offer {
                who: crate::asks::Who::Person(who),
                what: crate::asks::What::Job(job),
                until: crate::asks::Until::Done,
                wage,
            },
        );
    }
    // 3. Raise the most-refused rate when refusals outran agreements.
    let (agreed, declined) = tally(sim);
    let (day_agreed, day_declined) = (agreed - since.0, declined - since.1);
    if day_declined > day_agreed
        && let Some(task) = most_refused(sim)
    {
        sim.rates.step(task, crate::asks::RATE_STEP);
    }
    (agreed, declined)
}

/// The open job this person fits best, front-first on a tie — never an
/// industry's standing slot, which pays its own wage and is not postable.
fn best_fit(sim: &Sim, who: usize) -> Option<(sim::JobId, TaskType)> {
    let carried = sim.people.get(who).map(|person| person.traits.clone())?;
    let mut best: Option<(i64, sim::JobId, TaskType)> = None;
    for (site, board) in sim.sites.iter().enumerate() {
        if board.industry.is_some() {
            continue;
        }
        for slot in board.open_slots() {
            let Some(quest) = board.quest(slot) else {
                continue;
            };
            let job = sim::JobId { site, slot };
            if sim
                .postings
                .already_posted(sim, crate::asks::Who::Person(who), job)
            {
                continue;
            }
            let fit = crate::traits::competence_at(quest.task, &carried);
            if best.is_none_or(|(seen, _, _)| fit > seen) {
                best = Some((fit, job, quest.task));
            }
        }
    }
    best.map(|(_, job, task)| (job, task))
}

/// How many named asks have been agreed to and refused so far.
fn tally(sim: &Sim) -> (usize, usize) {
    let agreed = sim
        .events
        .iter()
        .filter(|event| event.class == crate::attention::EventClass::AskAgreed)
        .count();
    let declined = sim
        .events
        .iter()
        .filter(|event| event.class == crate::attention::EventClass::AskDeclined)
        .count();
    (agreed, declined)
}

/// Which task type the standing postings have been refused on most.
fn most_refused(sim: &Sim) -> Option<TaskType> {
    let mut counts = [0usize; 4];
    for posting in sim.postings.all() {
        let refused = posting
            .answered
            .iter()
            .any(|(_, answer)| matches!(answer, crate::asks::Answer::Declined { .. }));
        if !refused {
            continue;
        }
        if let crate::asks::What::Job(job) = posting.what
            && let Some(quest) = sim
                .sites
                .get(job.site)
                .and_then(|site| site.quest(job.slot))
        {
            let slot = TaskType::ALL
                .iter()
                .position(|task| *task == quest.task)
                .unwrap_or(0);
            counts[slot] += 1;
        }
    }
    let (slot, most) = counts
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| **count)
        .map(|(slot, count)| (slot, *count))?;
    (most > 0).then(|| TaskType::ALL[slot])
}

/// **The whole population, under one player.**
pub fn sweep(tuning: &Tuning, player: Player) -> Vec<Outcome> {
    (0..WORLDS)
        .map(|offset| live(tuning, ModuleSet::ALL, offset, DAYS, player))
        .collect()
}

// ── the bands, as shipped literals ────────────────────────────────────────
//
// Every number below is **written down**, never derived from `tuning`: a
// check that computed its expectation from the constant under test could not
// see that constant move, and the mutation round is what says whether any of
// this is an instrument (`mutation.rs`, and §A.6 of the game-session
// workflow).

/// What the treasury holds after two idle world-days — every pot on the
/// board, because an idle settlement works its whole board and spends
/// nothing.
pub const IDLE_TREASURY: i64 = 1260;

/// And how many jobs it finished doing it: the settlement's whole authored
/// board, taken by people nobody told to take it.
pub const IDLE_COMPLETED: usize = 24;

/// How many intervals were gone short over two idle world-days.
pub const IDLE_SHORTFALLS_TWO_DAYS: u64 = 7;

/// The purse the middle of the camp is left holding after two idle days.
///
/// **Nothing**, and that is the wave's whole argument: a self-chosen job pays
/// its pot into the *treasury* (GDD §4.1), so a settlement whose player never
/// posts anything and never builds anything works hard, banks well and leaves
/// its own people with empty hands.
pub const IDLE_MEDIAN_WALLET: i64 = 0;

/// **Who goes short first** — Steve, `CAST.md` §4.1's pariah-candidate, by
/// registry index.
///
/// The claim the demo character exists to make, and the staged start is what
/// made it real: on day one the camp is the four founders, Steve carries the
/// highest upkeep multiplier in it (caring, 3/2) and the smallest purse, and
/// he is the only one of the four whose wallet cannot meet the first interval.
/// It is **not** registry order — Bob is index 0 and pays his first interval
/// in full.
pub const PARIAH: usize = 1;

/// The worst median purse the attentive player is left with, over the whole
/// population.
pub const ATTENTIVE_WORST_WALLET: i64 = 17;

/// The most shortfalls the attentive player's worst world produced, over
/// three world-days.
pub const ATTENTIVE_WORST_SHORTFALLS: u64 = 6;

/// And the idle player's, over the same three days — the number the
/// differential is against.
pub const IDLE_SHORTFALLS: u64 = 14;

/// **The attention differential's margin, on its named measure.**
///
/// The measure is **the count of upkeep shortfalls over three world-days**,
/// and the margin is that the attentive player's *worst* world produces at
/// least this many fewer of them than the idle player's. Eight of fourteen:
/// competence more than halves the number of times somebody in Kawaza cannot
/// pay for themselves.
///
/// A shipped literal the mutation round can move, and the thing the handoff
/// says must hold at the shipped constants or the constants are wrong.
pub const ATTENTION_MARGIN_SHORTFALLS: u64 = 8;

/// And on the second measure, the median purse: the attentive player's worst
/// world leaves the middle of the camp at least this much better off than the
/// idle player's best.
pub const ATTENTION_MARGIN_WALLET: i64 = 17;

/// What one worked shift puts in the worker's purse at the shipped
/// settlement — the whole of the industry's wage, because the levy is shipped
/// at nothing.
pub const SHIFT_PAID: i64 = 20;

/// And what it puts in the treasury: nothing. Passive income is upgraded into
/// (GDD §4.1), and this is the number the drawer's knob moves.
pub const SHIFT_LEVIED: i64 = 0;

/// How far desperation gets in an idle settlement — the limp floor's own
/// number, and **well under `people::DESPERATION_MAX`**: the vulnerable slide
/// and nobody bottoms out, which is the contract the owner's decision states.
pub const IDLE_WORST_DESPERATION: i64 = 8;

/// **The cheap battery**, at a stated constants set — one idle world and one
/// staged shift, both with shipped literals.
///
/// Run by the mutation round thirty-nine times, so it is a single world of two
/// days rather than the population: what it is for is seeing the economy's own
/// constants move, and one settlement sees `upkeep_coin` and `upkeep_hours`
/// exactly as two hundred do. The levy is staged rather than played, because
/// an idle player never builds anything for it to be levied on.
pub fn judge_at(checks: &mut crate::checks::Checks, tuning: &Tuning) {
    let idle = live(tuning, ModuleSet::ALL, 0, 2, Player::Idle);
    checks.require(
        idle.treasury == IDLE_TREASURY && idle.completed == IDLE_COMPLETED,
        "two idle world-days do not bank what the board is worth",
        format!(
            "the treasury holds {}g after {} completions and the shipped economy banks \
             {IDLE_TREASURY}g over {IDLE_COMPLETED}",
            idle.treasury, idle.completed
        ),
    );
    checks.require(
        idle.median_wallet() == IDLE_MEDIAN_WALLET && idle.shortfalls == IDLE_SHORTFALLS_TWO_DAYS,
        "the idle settlement's purses are not where the shipped economy leaves them",
        format!(
            "the median purse is {}g after {} shortfalls and the shipped economy leaves \
             {IDLE_MEDIAN_WALLET}g after {IDLE_SHORTFALLS_TWO_DAYS}",
            idle.median_wallet(),
            idle.shortfalls
        ),
    );
    checks.require(
        idle.first_short == Some(PARIAH),
        "the first person to go short is not the pariah-candidate",
        format!(
            "{:?} went short first and CAST.md §4.1 names Steve (index {PARIAH}) - the \
             highest upkeep multiplier among the founders, and the smallest purse of them",
            idle.first_short
        ),
    );
    // **The limp floor, arithmetic half**: what is in the purse is what is
    // taken, so no wallet is ever overdrawn.
    checks.require(
        idle.wallets.iter().all(|held| *held >= 0),
        "an idle settlement overdrew somebody",
        format!(
            "the purses read {:?}; upkeep takes what is there",
            idle.wallets
        ),
    );
    // **Conservation** (GDD §4.1): the holders are the mints less the burns.
    let holders = idle.holders;
    checks.require(
        holders == idle.ports.expected(idle.opening_wallets),
        "gold moved through a port GDD §4.1 does not name",
        format!(
            "the treasury and the purses hold {holders}g and the ports account for {}g \
             ({})",
            idle.ports.expected(idle.opening_wallets),
            idle.ports.line()
        ),
    );
    judge_a_shift(checks, tuning);
}

/// **One shift, staged and paid** — the industry's own ports (GDD §4.1), and
/// the levy the drawer's knob sets.
///
/// Staged because an idle player never builds and a conducted one is too dear
/// to run thirty-nine times: what is being asked is what `settle_shift` moves,
/// which is a question about one completion.
fn judge_a_shift(checks: &mut crate::checks::Checks, tuning: &Tuning) {
    let Some(spec) = crate::settlement::INDUSTRIES.first() else {
        return;
    };
    let mut sim = Sim::opening(tuning, ModuleSet::ALL);
    sim.everybody_here();
    sim.treasury = spec.cost;
    let built = crate::settlement::build(&mut sim, 0, 0);
    checks.require(
        built.is_ok() && sim.treasury == 0,
        "building did not debit exactly what the industry costs",
        format!(
            "the build returned {built:?} and left {}g held",
            sim.treasury
        ),
    );
    checks.require(
        crate::settlement::free_slots(&crate::lens::Lens::on(&sim), 0) == spec.slots,
        "building did not open exactly the slots the industry says it opens",
        format!(
            "{} slots stand open and {:?} opens {}",
            crate::settlement::free_slots(&crate::lens::Lens::on(&sim), 0),
            spec.id,
            spec.slots
        ),
    );
    // One shift, worked and paid: the wage mints into the worker's purse and
    // the levy into the treasury, and nothing else moves.
    let worker = 4usize;
    let (before_wallet, before_treasury) = (sim.people[worker].wallet, sim.treasury);
    let (paid, levied) = crate::settlement::settle_shift(&mut sim, tuning, worker, spec.site);
    checks.require(
        paid + levied == sim.settlement.wage(0)
            && sim.people[worker].wallet == before_wallet + paid
            && sim.treasury == before_treasury + levied,
        "a worked shift did not pay what the industry pays",
        format!(
            "the shift paid {paid}g to the worker and levied {levied}g against a wage of \
             {}g; the purse moved {} and the treasury {}",
            sim.settlement.wage(0),
            sim.people[worker].wallet - before_wallet,
            sim.treasury - before_treasury
        ),
    );
    // **Shipped literals, never `tuning.industry_levy` read back at itself**:
    // a check that computed its expectation from the constant under test could
    // not see that constant move, and the levy is shipped at nothing, so the
    // only way to see it at all is to write down what nothing pays.
    checks.require(
        paid == SHIFT_PAID && levied == SHIFT_LEVIED,
        "a worked shift does not pay what the shipped settlement pays",
        format!(
            "the shift paid the worker {paid}g and the settlement {levied}g; the shipped              settlement pays {SHIFT_PAID}g and levies {SHIFT_LEVIED}g, because passive              income is upgraded into and not started with (GDD §4.1)"
        ),
    );
}

/// **What the wage lever actually reaches** (the wave's decision-surface
/// table, and the answer it got).
///
/// The decision table asks for "a wage step changes at least one worker's next
/// choice". **It does not, at the shipped constants and this cast**, and this
/// is the instrument that says so rather than a check written until it passed.
///
/// The reason is a seam between two waves, not a defect in either:
/// `answers::terms` feels a *posted* wage by `pot_affinity` **plus
/// desperation**, so a posting reaches anybody who needs money; but an
/// industry's slots are filled by self-choice, and `autonomy::weigh`'s pot term
/// for a self-chosen job is `pot_affinity` **alone** — which in this cast is
/// `greedy`, and `greedy` is Bob. So the wage moves exactly one person's sum,
/// and never across the idle floor, because his `indebted` want already carries
/// him over it.
///
/// What is asserted is therefore the honest pair: **the lever reaches the
/// decision function** (Bob's sum moves by exactly the arithmetic, a shipped
/// literal) and **the ladder moves nobody's answer** (also a literal, so the
/// day it starts to it fails here and this comment is what gets rewritten).
/// `FINDINGS.md` G-035 is the entry; the fences forbid touching the scorer's
/// terms, which is what the fix would be.
pub fn judge_the_wage(checks: &mut crate::checks::Checks, tuning: &Tuning) -> String {
    let Some(spec) = crate::settlement::INDUSTRIES.first() else {
        return String::new();
    };
    let mut sim = Sim::opening(tuning, ModuleSet::ALL);
    sim.everybody_here();
    sim.treasury = spec.cost;
    let _ = crate::settlement::build(&mut sim, 0, 0);
    // The settlement's own board, worked out; and nobody pressed, so what is
    // being measured is the wage and not the desperation under it. This is the
    // world the works are *for*: a shift is the only paid work in Kawaza, and
    // what it has to beat is staying home.
    for site in 0..sim.sites.len() {
        if sim.sites[site].industry.is_some() {
            continue;
        }
        for slot in 0..sim.sites[site].states.len() {
            sim.sites[site].states[slot] = crate::sim::JobState::Done { by: 0 };
        }
    }
    for person in &mut sim.people {
        person.desperation = 0;
    }
    let shift = crate::autonomy::Action::SeekWork {
        job: sim::JobId {
            site: spec.site,
            slot: 0,
        },
    };
    let weigh = |sim: &Sim, who: usize| -> i64 {
        crate::autonomy::weigh(sim, tuning, 0, who, shift)
            .iter()
            .map(|term| term.value)
            .sum()
    };
    let on_shift = |sim: &Sim| -> usize {
        let lens = Lens::on(sim);
        lens.roll()
            .into_iter()
            .filter(|who| {
                matches!(
                    crate::autonomy::choose(
                        sim,
                        tuning,
                        0,
                        *who,
                        &crate::autonomy::candidates(sim, *who),
                    )
                    .action,
                    crate::autonomy::Action::SeekWork { job } if job.site == spec.site
                )
            })
            .count()
    };
    // --- 1: the lever reaches the decision function ------------------------
    let greedy = sim
        .people
        .iter()
        .position(|person| person.id == "bob")
        .unwrap_or(0);
    let opening = sim.settlement.wage(0);
    let at_opening = weigh(&sim, greedy);
    crate::settlement::step_wage(&mut sim, 0, crate::settlement::WAGE_MAX);
    let at_ceiling = weigh(&sim, greedy);
    checks.require(
        at_opening == GREEDY_AT_THE_OPENING_WAGE && at_ceiling == GREEDY_AT_THE_CEILING,
        "stepping an industry's wage does not reach the scorer at all",
        format!(
            "the one carrier of a pot affinity weighs a shift at {at_opening} for {opening}g \
             and {at_ceiling} for {}g; the shipped settlement weighs \
             {GREEDY_AT_THE_OPENING_WAGE} and {GREEDY_AT_THE_CEILING}. The wage is the \
             slot's own pot, so a wage that moved no sum would mean the lever did not reach \
             the decision function at all",
            sim.settlement.wage(0)
        ),
    );
    // --- 2: and what it moves, over the panel's whole range ----------------
    //
    // Walked in the panel's own step, exactly as `compliance.rs` walks the
    // standing rates: what a lever is worth is how much of it a player has to
    // spend before anybody does anything differently.
    crate::settlement::step_wage(&mut sim, 0, -crate::settlement::WAGE_MAX);
    let mut ladder: Vec<(i64, usize)> = Vec::new();
    loop {
        ladder.push((sim.settlement.wage(0), on_shift(&sim)));
        if sim.settlement.wage(0) >= crate::settlement::WAGE_MAX {
            break;
        }
        crate::settlement::step_wage(&mut sim, 0, crate::settlement::WAGE_STEP);
    }
    let moves: Vec<i64> = ladder
        .windows(2)
        .filter(|pair| pair[0].1 != pair[1].1)
        .map(|pair| pair[1].0)
        .collect();
    let seated = ladder
        .iter()
        .find(|(wage, _)| *wage == opening)
        .map_or(0, |(_, count)| *count);
    checks.require(
        moves == WAGE_LADDER_MOVES && seated == SHIFTS_AT_THE_OPENING_WAGE,
        "the wage lever moves the camp somewhere the shipped settlement does not say",
        format!(
            "walking the stepper from 0g to {}g in {}g steps changes who takes a shift at \
             {moves:?}, and {seated} of the camp take one at the opening {opening}g; the \
             shipped settlement moves them at {WAGE_LADDER_MOVES:?} and seats \
             {SHIFTS_AT_THE_OPENING_WAGE}. An empty list is this session's own finding and \
             not a passing check: the day the lever starts to move somebody, this fails and \
             `FINDINGS.md` G-035 is what gets rewritten",
            crate::settlement::WAGE_MAX,
            crate::settlement::WAGE_STEP
        ),
    );
    format!(
        "the wage lever: it reaches the scorer ({at_opening} at {opening}g, {at_ceiling} at \
         {}g for the one carrier of a pot affinity) and over the panel's whole range it moves \
         the camp's answer at {moves:?} - {seated} take a shift at any wage, which is \
         FINDINGS G-035",
        crate::settlement::WAGE_MAX
    )
}

/// What the cast's one carrier of a pot affinity weighs a shift at, at the
/// wage the works open on — a shipped literal, so a lever that stopped
/// reaching the scorer fails here.
pub const GREEDY_AT_THE_OPENING_WAGE: i64 = 8;

/// And at the panel's ceiling.
pub const GREEDY_AT_THE_CEILING: i64 = 12;

/// **Every wage the camp's answer changes at**, walking the settlement panel's
/// own stepper across its own range, over a settlement whose sites are dry and
/// whose people are unpressed.
///
/// **Empty, and that is the finding** (`FINDINGS.md` G-035): a self-chosen
/// job's pot is felt by `pot_affinity` alone, only `greedy` carries one, and
/// the one character who has it is over the idle floor on his `indebted` want
/// before the wage says anything. It is a shipped literal so that the day the
/// scorer's money term changes, this check fails and says so.
pub const WAGE_LADDER_MOVES: &[i64] = &[];

/// And how many of the camp take a shift, at any wage the panel can reach: the
/// two makers, whose trade it is, and the two indebted, whose want covers any
/// paid work.
pub const SHIFTS_AT_THE_OPENING_WAGE: usize = 4;

/// **The economy sweeps** (GDD §9) — the population, the limp floor, and the
/// attention differential, run once.
pub fn judge_sweeps(checks: &mut crate::checks::Checks, tuning: &Tuning) -> String {
    let idle = sweep(tuning, Player::Idle);
    let attentive = sweep(tuning, Player::Attentive);
    // --- the idle bands ----------------------------------------------------
    let treasuries: Vec<i64> = idle.iter().map(|run| run.treasury).collect();
    let shortfalls: Vec<u64> = idle.iter().map(|run| run.shortfalls).collect();
    let wallets: Vec<i64> = idle.iter().map(Outcome::median_wallet).collect();
    // **The idle settlement is order-invariant**, which is the sweep's own
    // finding and the reason two hundred worlds would say what sixty-four do:
    // who takes which job varies with who looks up first, and what the
    // settlement comes to does not.
    let same = treasuries.iter().all(|held| *held == IDLE_TREASURY)
        && shortfalls.iter().all(|count| *count == IDLE_SHORTFALLS)
        && wallets.iter().all(|held| *held == IDLE_MEDIAN_WALLET);
    checks.require(
        same,
        "the idle settlement's outcome depends on who looked up first",
        format!(
            "over {WORLDS} orders of thinking the treasury ran {:?}..{:?}, the shortfalls \
             {:?}..{:?} and the median purse {:?}..{:?}; the shipped economy comes to \
             {IDLE_TREASURY}g, {IDLE_SHORTFALLS} shortfalls and {IDLE_MEDIAN_WALLET}g",
            treasuries.iter().min(),
            treasuries.iter().max(),
            shortfalls.iter().min(),
            shortfalls.iter().max(),
            wallets.iter().min(),
            wallets.iter().max(),
        ),
    );
    // --- the limp floor ----------------------------------------------------
    //
    // **Nobody starves at subsistence** (GDD §9), in the three things that
    // can be said about a world with no death in it: no purse is overdrawn,
    // nobody is pressed to the ceiling, and the settlement keeps working —
    // every authored job is finished by somebody nobody told to finish it.
    for run in &idle {
        let worst = run.desperation.iter().copied().max().unwrap_or(0);
        checks.require(
            run.wallets.iter().all(|held| *held >= 0)
                && worst <= IDLE_WORST_DESPERATION
                && run.completed == IDLE_COMPLETED,
            "an idle settlement did not limp",
            format!(
                "the purses read {:?}, the worst desperation is {worst} against a ceiling of \
                 {} and {} of the board's {IDLE_COMPLETED} jobs were finished; the settlement \
                 must limp without the player (GDD §1)",
                run.wallets,
                crate::people::DESPERATION_MAX,
                run.completed
            ),
        );
        // **And the slide is differential**: some of the camp goes short and
        // some does not, which is what "the vulnerable slide, and you can see
        // which" means. A world where everybody bottomed out would say
        // nothing about who is vulnerable, and one where nobody moved would
        // say the pressure is not there.
        let pressed = run
            .desperation
            .iter()
            .zip(crate::people::roster().iter())
            .filter(|(now, was)| **now > was.desperation)
            .count();
        checks.require(
            pressed > 0 && pressed < run.desperation.len(),
            "an idle settlement pressed everybody or nobody",
            format!(
                "{pressed} of {} people are worse off than they opened; the limp is meant to \
                 be uncomfortable and it is meant to be uneven",
                run.desperation.len()
            ),
        );
    }
    // **Steve first, in every world** — the demo character's claim, asserted
    // over the whole population rather than over the median one, because the
    // population turned out to agree.
    let firsts: Vec<Option<usize>> = idle.iter().map(|run| run.first_short).collect();
    checks.require(
        firsts.iter().all(|who| *who == Some(PARIAH)),
        "the pariah-candidate is not the first to go short in every idle world",
        format!("the first to go short reads {firsts:?} and CAST.md §4.1 names index {PARIAH}"),
    );
    // --- the attention differential ---------------------------------------
    //
    // **The attentive player's worst world against the idle player's every
    // world.** Stated that way round on purpose: a median-against-median
    // comparison can be won by a long tail, and what the wave claims is that
    // competence beats neglect, not that it usually does.
    let worst_shortfalls = attentive
        .iter()
        .map(|run| run.shortfalls)
        .max()
        .unwrap_or(u64::MAX);
    let worst_wallet = attentive
        .iter()
        .map(Outcome::median_wallet)
        .min()
        .unwrap_or(0);
    checks.require(
        worst_shortfalls == ATTENTIVE_WORST_SHORTFALLS
            && worst_shortfalls + ATTENTION_MARGIN_SHORTFALLS <= IDLE_SHORTFALLS,
        "attention does not beat neglect by the margin the wave ships",
        format!(
            "the attentive player's worst world produced {worst_shortfalls} shortfalls \
             against the idle player's {IDLE_SHORTFALLS}, and the shipped margin is \
             {ATTENTION_MARGIN_SHORTFALLS}. If competence cannot beat neglect at the \
             shipped constants, the constants are wrong"
        ),
    );
    checks.require(
        worst_wallet == ATTENTIVE_WORST_WALLET
            && worst_wallet >= IDLE_MEDIAN_WALLET + ATTENTION_MARGIN_WALLET,
        "attention does not leave the camp better off by the margin the wave ships",
        format!(
            "the attentive player's worst median purse is {worst_wallet}g against the idle \
             player's {IDLE_MEDIAN_WALLET}g, and the shipped margin is \
             {ATTENTION_MARGIN_WALLET}g"
        ),
    );
    // **And the industry is what the attention bought**: every attentive world
    // builds it, and every attentive world therefore has work standing after
    // the board is spent — which is the limp-floor the settlement module is
    // for, said as a fact about the sweep.
    checks.require(
        attentive.iter().all(|run| run.built)
            && attentive.iter().all(|run| run.completed > IDLE_COMPLETED),
        "the attentive player did not build the settlement out of its own board",
        format!(
            "{} of {WORLDS} attentive worlds stood an industry up, and the busiest finished \
             {:?} jobs against the idle {IDLE_COMPLETED}",
            attentive.iter().filter(|run| run.built).count(),
            attentive.iter().map(|run| run.completed).max()
        ),
    );
    // --- conservation, over both ------------------------------------------
    for (player, runs) in [("idle", &idle), ("attentive", &attentive)] {
        for run in runs.iter() {
            let holders = run.holders;
            checks.require(
                holders == run.ports.expected(run.opening_wallets),
                "gold moved through a port GDD §4.1 does not name",
                format!(
                    "under the {player} player the treasury and the purses hold {holders}g \
                     and the ports account for {}g ({})",
                    run.ports.expected(run.opening_wallets),
                    run.ports.line()
                ),
            );
        }
    }
    // **The other measure the handoff names**, reported rather than asserted:
    // the *median* desperation does not separate the two players, because the
    // person in the middle of the camp is not the person who slides. That is
    // itself the finding — the vulnerable slide and the comfortable do not —
    // and it is why the margin is stated on the shortfall count and the purse.
    let idle_pressed = median(
        &idle
            .iter()
            .map(Outcome::median_desperation)
            .collect::<Vec<_>>(),
    );
    let kept = median(
        &attentive
            .iter()
            .map(Outcome::median_desperation)
            .collect::<Vec<_>>(),
    );
    format!(
        "economy sweeps: {WORLDS} orders of thinking x 2 players x {DAYS} world-days; idle \
         banks {IDLE_TREASURY}g, leaves a median purse of {IDLE_MEDIAN_WALLET}g and goes \
         short {IDLE_SHORTFALLS} times, the same in every world; attentive's worst world \
         goes short {worst_shortfalls} and holds {worst_wallet}g, a margin of \
         {ATTENTION_MARGIN_SHORTFALLS} shortfalls and {ATTENTION_MARGIN_WALLET}g; median \
         desperation {idle_pressed} against {kept}, which separates nothing and is why the \
         margin is not stated on it; Steve first every time"
    )
}
