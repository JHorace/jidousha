//! **Resolution** — how a job turns out, and what it pays (GDD §5's
//! resolution row, wave 1.4).
//!
//! # One question each
//!
//! - [`odds`] is the fit-to-odds curve: what a job's chances are for somebody
//!   at a fit. **The roll reads it, and so does every surface that shows
//!   odds** — the board row, the candidate picker and the work list print
//!   [`Odds::word`] over this one function, so a word on screen and the roll
//!   behind the job cannot disagree.
//! - [`resolve`] is the roll: `(worker, job, occurrence) -> tier`. Its
//!   randomness is **addressed by the occurrence** — the scenario seed, the
//!   world-minute the work completes, and the job's identity — and never by
//!   call order or frame (`FINDINGS.md` G-016's rule). Two speed scripts that
//!   finish the same job at the same world-minute roll the same number,
//!   because nothing about how the clock got there is an input.
//! - [`payout`] is the money: what each port moves when a job resolves at a
//!   tier. **The scorer's money term reads it too** (`autonomy::pay_for`), so
//!   what pulls somebody toward a job is what that job would actually pay
//!   them — the wage if it was posted, their share if it was their own idea —
//!   and never a pot that is not theirs (closes `FINDINGS.md` G-035).
//!
//! # Three tiers, and what each one is worth today
//!
//! *Went well* and *done* pay the same — honestly. What going well adds is the
//! record ([`Resolved`] on `Sim::resolved`), which GDD §4.3's bonds ("repeated
//! shared success") will read when something that reads it lands; until then
//! [`went_well_means`] says so, derived from the registry the way a trait
//! chip's dormancy clause is. *Failed* mints nothing: no pot, no share — but a
//! **posted** job's wage is paid in full anyway (wave 1.2's rule: the promise
//! was the posting's), and the job goes back on its board, open, pot intact.
//! Failure is economic only. It writes no desperation and hurts nobody; the
//! sting arrives through the purse and the needs interval like everything
//! else.
//!
//! # What is never rolled
//!
//! An industry shift (`Site::industry`). Settlement's standing slots are not
//! pot-bearing site jobs, and whether fit should matter at an industry is an
//! open question in the GDD's ledger, not something this module decides by
//! sharing `work_done` with them. [`rolls_at`] is the one predicate.
//!
//! # Degrades to
//!
//! With the module off every job succeeds — the landed stub, *headcount and
//! duration always succeed* — and the seed reaches nothing. The pay is
//! unchanged: the share is a wealth port, not a roll.

use jidousha::prelude::Rng;

use crate::constants::Tuning;
use crate::sim::{JobId, Sim, Site};

/// The module id, as `modules::MODULES` and every stamp spell it.
pub const MODULE: &str = "resolution";

/// How a job turned out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// Done, and done well. Pays as [`Tier::Done`] today; the record is the
    /// difference.
    WentWell,
    /// Done.
    Done,
    /// Botched. Mints nothing, and the job goes back on the board.
    Failed,
}

impl Tier {
    /// Every tier, best first.
    pub const ALL: &'static [Tier] = &[Tier::WentWell, Tier::Done, Tier::Failed];

    /// The tier's name, as a transcript and a report spell it.
    pub fn name(self) -> &'static str {
        match self {
            Tier::WentWell => "went well",
            Tier::Done => "done",
            Tier::Failed => "failed",
        }
    }

    /// Whether the work was done — what mints a pot.
    pub fn succeeded(self) -> bool {
        !matches!(self, Tier::Failed)
    }
}

/// **A job's chances**, in whole percent: how often it fails, how often it
/// goes well, and the rest is plain done.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Odds {
    /// Percent of rolls that fail.
    pub fail: i64,
    /// Percent of rolls that go well.
    pub well: i64,
}

/// **The odds in a word** — what a board row, a candidate row and a work row
/// print beside fit. Three, by failure chance, at the drawer's thresholds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OddsWord {
    /// Fails no more often than `odds_safe` percent.
    Safe,
    /// Between the two thresholds.
    Chancy,
    /// Fails at least `odds_risky` percent of the time.
    Risky,
}

impl OddsWord {
    /// Every word, safest first.
    pub const ALL: &'static [OddsWord] = &[OddsWord::Safe, OddsWord::Chancy, OddsWord::Risky];

    /// The word itself. ASCII, and short enough for the narrowest cell that
    /// prints it (`floors` measures).
    pub fn name(self) -> &'static str {
        match self {
            OddsWord::Safe => "safe",
            OddsWord::Chancy => "chancy",
            OddsWord::Risky => "risky",
        }
    }
}

impl Odds {
    /// Percent of rolls that are plain done.
    pub fn done(self) -> i64 {
        100 - self.fail - self.well
    }

    /// **The word for these odds** — derived from this value and the drawer's
    /// two thresholds, never from fit directly, so a surface that prints it is
    /// printing the roll's own odds.
    ///
    /// `safe` is checked first, so thresholds a player has stepped across
    /// each other still give one word rather than two.
    pub fn word(self, tuning: &Tuning) -> OddsWord {
        if self.fail <= tuning.odds_safe {
            OddsWord::Safe
        } else if self.fail >= tuning.odds_risky {
            OddsWord::Risky
        } else {
            OddsWord::Chancy
        }
    }
}

/// **The fit-to-odds curve** — a function of fit alone (no difficulty stat
/// this wave; GDD §10 names the seam).
///
/// Failure falls by `fail_fit` a point from `fail_base`; going well rises by
/// `well_fit` a point from nothing; both are held inside a hundred, and going
/// well never eats into failure.
pub fn odds(tuning: &Tuning, fit: i64) -> Odds {
    let fail = (tuning.fail_base - fit * tuning.fail_fit).clamp(0, 100);
    let well = (fit * tuning.well_fit).clamp(0, 100 - fail);
    Odds { fail, well }
}

/// **This person's odds at this job** — their fit for the job's task type
/// (`traits::competence_at`, the scorer's own aptitude term), through
/// [`odds`]. The one call every odds-word on screen makes, and the one the
/// roll makes.
pub fn odds_for(sim: &Sim, tuning: &Tuning, who: usize, job: JobId) -> Option<Odds> {
    let quest = sim.sites.get(job.site)?.quest(job.slot)?;
    let person = sim.people.get(who)?;
    Some(odds(
        tuning,
        crate::traits::competence_at(quest.task, &person.traits),
    ))
}

/// **Whether work at this site is rolled at all** — a quest site's job is,
/// an industry's shift is not (read off `Site::industry`, as wave 1.3 read
/// its port).
pub fn rolls_at(site: &Site) -> bool {
    site.industry.is_none()
}

/// Whether this build rolls jobs — the module is on.
pub fn live(sim: &Sim) -> bool {
    sim.modules.enabled(MODULE)
}

/// **The occurrence's address, as one number**: seed, world-minute and job.
///
/// A mix rather than a sum, so nearby addresses — the same job a minute
/// later, the next slot along, the next seed — land far apart. The game mixes
/// before seeding because `docs/api/` does not say whether `Rng::from_seed`
/// scrambles nearby seeds itself (`FINDINGS.md` G-045).
fn address(seed: u64, minute: u64, job: JobId) -> u64 {
    fn mix(mut z: u64) -> u64 {
        z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    let place = ((job.site as u64) << 32) | job.slot as u64;
    mix(mix(mix(seed) ^ minute) ^ place)
}

/// **The roll** for one occurrence: a number in `0..100`, addressed by the
/// seed, the world-minute and the job — and by nothing else.
pub fn roll(seed: u64, minute: u64, job: JobId) -> i64 {
    i64::from(Rng::from_seed(address(seed, minute, job)).below(100))
}

/// **The tier a roll lands in, at these odds** — failure first, then going
/// well, then done. Split out so the distribution sweep can walk it over
/// staged odds without a world.
pub fn tier_of(odds: Odds, rolled: i64) -> Tier {
    if rolled < odds.fail {
        Tier::Failed
    } else if rolled < odds.fail + odds.well {
        Tier::WentWell
    } else {
        Tier::Done
    }
}

/// **How this job went** — the one resolution function.
///
/// The module off, or a shift, is the stub: done. Otherwise the worker's odds
/// at this job, and the occurrence's roll.
pub fn resolve(sim: &Sim, tuning: &Tuning, at: u64, who: usize, job: JobId) -> Tier {
    let rolled = sim.sites.get(job.site).is_some_and(rolls_at);
    if !live(sim) || !rolled {
        return Tier::Done;
    }
    match odds_for(sim, tuning, who, job) {
        Some(odds) => tier_of(odds, roll(sim.seed, at, job)),
        None => Tier::Done,
    }
}

/// **What each port moves when a job resolves** (GDD §4.1).
///
/// One value per port so the conservation ledger can be written from it
/// field by field, and [`Payout::to_worker`] is the whole of what the person
/// gets — the figure the scorer weighs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Payout {
    /// TRANSFER: a posting's wage, treasury to wallet — in full, even on a
    /// failure.
    pub wage: i64,
    /// MINT: a self-chosen job's share of the pot, into the worker's wallet,
    /// on success.
    pub share: i64,
    /// MINT: the rest of the pot, into the treasury, on success.
    pub pot: i64,
    /// MINT: an industry shift's wage, into the worker's wallet.
    pub shift: i64,
    /// MINT: an industry shift's levy, into the treasury.
    pub levy: i64,
}

impl Payout {
    /// What the worker's purse gains — the wage, the share or the shift.
    pub fn to_worker(self) -> i64 {
        self.wage + self.share + self.shift
    }
}

/// **The payout function**: what this job pays, by port, if it resolves at
/// `tier` — given the posted wage it was taken for, if it was taken for one.
///
/// A job pays one way: **the wage if it was posted, the share if it was their
/// own idea**, and a shift pays the industry's wage. A failure mints nothing,
/// and a posted failure still pays the wage.
pub fn payout(sim: &Sim, tuning: &Tuning, job: JobId, wage: Option<i64>, tier: Tier) -> Payout {
    let Some(site) = sim.sites.get(job.site) else {
        return Payout::default();
    };
    let Some(quest) = site.quest(job.slot) else {
        return Payout::default();
    };
    if !rolls_at(site) {
        let (shift, levy) = crate::settlement::shift_pay(sim, tuning, job.site);
        return Payout {
            shift,
            levy,
            ..Payout::default()
        };
    }
    let mut out = Payout {
        wage: wage.unwrap_or(0),
        ..Payout::default()
    };
    if tier.succeeded() {
        let share = match wage {
            Some(_) => 0,
            None => share_of(tuning, quest.pot),
        };
        out.share = share;
        out.pot = quest.pot - share;
    }
    out
}

/// A self-chooser's share of a pot, rounded down, held inside the pot.
pub fn share_of(tuning: &Tuning, pot: i64) -> i64 {
    (pot * tuning.share_pct.clamp(0, 100) / 100).clamp(0, pot.max(0))
}

/// **Move it**: what [`payout`] says, through every port, onto the ledger —
/// and the regard a paid wage moves (GDD §4.2's wage-vs-expectation rule).
///
/// Reads the party's own posting, so the wage paid is the wage the posting
/// promised, and clears it.
pub fn settle(sim: &mut Sim, tuning: &Tuning, who: usize, job: JobId, tier: Tier) -> Payout {
    let posted = sim
        .parties
        .get(who)
        .and_then(|party| party.posting)
        .and_then(|id| sim.postings.get(id))
        .map(|posting| (posting.wage, posting.rate_at_posting));
    let paid = payout(sim, tuning, job, posted.map(|(wage, _)| wage), tier);
    sim.treasury += paid.pot + paid.levy - paid.wage;
    sim.ports.transferred += paid.wage;
    sim.ports.minted_shares += paid.share;
    sim.ports.minted_pots += paid.pot;
    sim.ports.minted_wages += paid.shift + paid.levy;
    if let Some(person) = sim.people.get_mut(who) {
        person.wallet += paid.to_worker();
    }
    if let Some((wage, expectation)) = posted {
        crate::answers::wage_regard(sim, tuning, who, wage, expectation);
    } else if paid.shift + paid.levy > 0
        && let Some(task) = sim
            .sites
            .get(job.site)
            .and_then(|site| site.quest(job.slot))
            .map(|quest| quest.task)
    {
        let expectation = sim.rates.of(task);
        crate::answers::wage_regard(sim, tuning, who, paid.shift + paid.levy, expectation);
    }
    if let Some(party) = sim.parties.get_mut(who) {
        party.posting = None;
    }
    paid
}

/// **One resolution, kept** — the record bonds will read (GDD §4.3's
/// "repeated shared success"), and the whole of what *went well* is worth
/// today.
///
/// A record, not an input: nothing in the simulation reads it yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// The world-minute the work finished.
    pub minute: u64,
    /// Who worked it.
    pub who: usize,
    /// Which job.
    pub job: JobId,
    /// How it went.
    pub tier: Tier,
    /// Whether it was a posting (as against their own idea).
    pub posted: bool,
}

/// **The fit cell** — `fit 2 safe` — the one formatter the board row, the
/// candidate picker and the work list print beside a job (wave 1.4).
///
/// The word is [`Odds::word`] over the odds the caller read through
/// `Lens::odds`, which is [`odds_for`] — the roll's own function — so the
/// word on a row is the roll's odds and never a second formula over fit.
/// `None` odds (the module off, or a shift) print the fit alone: nothing is
/// rolled, so there are no odds to name.
pub fn fit_cell(fit: i64, odds: Option<Odds>, tuning: &Tuning) -> String {
    match odds {
        Some(odds) => format!("fit {fit} {}", odds.word(tuning).name()),
        None => format!("fit {fit}"),
    }
}

/// **The fit cell a row should show, read straight off the simulation** —
/// what the batteries compare a drawn cell against.
///
/// Built from [`live`], [`rolls_at`] and [`odds_for`] — the three the roll
/// itself consults — rather than from `Lens::odds`, so a surface that stopped
/// reading the roll's odds would disagree with this and be caught.
pub fn cell_for(sim: &Sim, tuning: &Tuning, who: usize, job: JobId) -> String {
    let rolled = live(sim) && sim.sites.get(job.site).is_some_and(rolls_at);
    let fit = sim
        .sites
        .get(job.site)
        .and_then(|site| site.quest(job.slot))
        .zip(sim.people.get(who))
        .map_or(0, |(quest, person)| {
            crate::traits::competence_at(quest.task, &person.traits)
        });
    let odds = if rolled {
        odds_for(sim, tuning, who, job)
    } else {
        None
    };
    fit_cell(fit, odds, tuning)
}

/// **What fit means, derived** — the fit chip's explanation, on the board, the
/// picker and the work list (wave 1.2's clarity rider, re-derived by wave 1.4).
///
/// Wave 1.2 wrote this as a sentence — "every job succeeds until resolution
/// lands" — and it would have had to be hand-edited to retire itself
/// (`FINDINGS.md` G-046). It is now built from the registry and the curve: with
/// the module live it says what the odds *are*, at every fit the vocabulary
/// can produce, through [`odds`] and [`Odds::word`]; with it off it says the
/// trait chips' own dormancy clause (`traits::Consumer::Resolution`). Moving a
/// curve constant moves this sentence.
pub fn fit_means(tuning: &Tuning, modules: crate::modules::ModuleSet) -> String {
    let head = "fit sways answers";
    if !modules.enabled(MODULE) {
        return format!(
            "{head}, and {}.",
            crate::traits::Consumer::Resolution.absence()
        );
    }
    let mut fits: Vec<i64> = crate::traits::TaskType::ALL
        .iter()
        .map(|task| task.aptitude().def().aptitude)
        .chain(std::iter::once(0))
        .collect();
    fits.sort_unstable_by(|a, b| b.cmp(a));
    fits.dedup();
    let each = fits
        .iter()
        .map(|fit| {
            let odds = odds(tuning, *fit);
            let well = if odds.well > 0 {
                format!(", {}% well", odds.well)
            } else {
                String::new()
            };
            format!(
                "fit {fit} {}: {}% fail{well}",
                odds.word(tuning).name(),
                odds.fail
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "{head} and sets odds - {each}. {}.",
        went_well_means(modules)
    )
}

/// The module whose arrival gives *went well* a mechanical reader — the
/// shared-success door into a bond (GDD §4.3; the parties wave).
pub const WENT_WELL_READER: &str = "parties";

/// **What going well is worth, in words, derived from the registry** — the
/// dormancy-honesty pattern the trait chips use (`traits::explain`).
///
/// While nothing that reads the record is registered and on, the sentence
/// says it pays as done; the day the reader lands, this changes because the
/// registry did.
pub fn went_well_means(modules: crate::modules::ModuleSet) -> &'static str {
    if modules.enabled(WENT_WELL_READER) {
        "well counts toward a bond"
    } else {
        "well pays as done today"
    }
}
