//! **Settlement**: the camp's capacity, and the standing work it opens (GDD
//! §5's settlement module, wave 1.3).
//!
//! # An industry is standing job slots
//!
//! "Passive" is a word about the player's chair and not about the world: an
//! industry is **capacity**, and what fills it is the scorer, one shift at a
//! time, exactly as it fills a site's board. So an industry's slots *are* a
//! [`crate::sim::Site`] — one more entry of `Sim::sites`, standing at the camp
//! rather than out on the map — and everything that already walks a board
//! walks this one: the candidate list, the dispatch, the journey, the
//! completion. **No term was added to the scorer**, because a shift is a job
//! and a job is a thing the scorer already knows how to want.
//!
//! What makes a slot *standing* is what happens when it is finished: it opens
//! again ([`reopen`]). A site runs dry and an industry does not, which is the
//! whole of why the settlement can limp without the player once it has one.
//!
//! # The three levers, and the two ports
//!
//! **Build** is the treasury's first real sink (GDD §4.1's BURN port); the
//! **wage** is what a shift pays, and it is the same policy family as the
//! standing rates — the industry's row is where camp work is priced, the rates
//! panel is where field work is; the **levy** is what the settlement takes out
//! of a worked shift, shipped at nothing, because passive income is upgraded
//! into and not started with.
//!
//! A completed shift moves gold through GDD §4.1's *industry* ports and not a
//! site's: the wage is **minted into the worker's wallet** (an industry makes
//! what it pays), and the levy is minted into the treasury beside it. A site's
//! pot goes the other way, into the treasury, and that difference is the whole
//! of what `Site::industry` is for.

use crate::attention::EventClass;
use crate::constants::Tuning;
use crate::sim::{JobId, Quest, Sim};
use crate::traits::TaskType;

/// The module id, as GDD §5's registry spells it.
pub const MODULE: &str = "settlement";

/// What one tap of the settlement panel's wage stepper moves a wage by, in
/// gold.
///
/// The standing rates' own step (`asks::RATE_STEP`), because the per-industry
/// wage and the standing rates are one policy family (GDD's postings section)
/// and a lever that moved camp work by a different amount than field work
/// would be two policies wearing one word.
pub const WAGE_STEP: i64 = crate::asks::RATE_STEP;

/// The most a shift's wage may be set to — the rates panel's own ceiling.
pub const WAGE_MAX: i64 = crate::asks::RATE_MAX;

/// One industry the settlement can build.
///
/// Content, the way `asks::RATES` and the event-class table are content: the
/// settlement panel is the one way to move what is movable here, and a drawer
/// row beside it would be the second way this repo's first convention refuses.
/// The levy is the exception, and it is a drawer row precisely because it is
/// settlement-wide policy with no per-industry reading.
#[derive(Clone, Copy, Debug)]
pub struct IndustrySpec {
    /// The id a stamp and a report name it by. ASCII, lowercase.
    pub id: &'static str,
    /// What the panel and the feed call it.
    pub name: &'static str,
    /// What one of its standing job slots is called on a board.
    pub shift: &'static str,
    /// What kind of work a shift is — the aptitude row the scorer reads, and
    /// the standing rate its wage is judged against.
    pub task: TaskType,
    /// What building it costs the treasury, in gold.
    pub cost: i64,
    /// How many standing job slots it opens.
    pub slots: usize,
    /// What a shift pays before the player steps it, in gold.
    ///
    /// The craft standing rate, so the opening settlement pays camp work
    /// exactly what it says it pays for that kind of work and the first step
    /// in either direction is the player's own decision rather than a
    /// correction of the authoring.
    pub wage: i64,
    /// How long a shift holds somebody, in world-minutes.
    pub minutes: i64,
    /// Which entry of `Sim::sites` holds its standing slots.
    pub site: usize,
}

/// **The industries.** One at the MVP (GDD §5: "one generic industry at
/// MVP"), and it is the camp's own fire with a roof over it — `CAST.md` §1's
/// first standing building, and the beat where a camp becomes a settlement.
pub const INDUSTRIES: &[IndustrySpec] = &[IndustrySpec {
    id: "camp-works",
    name: "the camp works",
    shift: "a shift at the works",
    task: TaskType::Craft,
    cost: 180,
    slots: 3,
    wage: 20,
    minutes: 180,
    site: 4,
}];

/// **What the settlement has built, and what it pays for it.**
///
/// Simulation state, for the reason the standing rates are: the player writes
/// it through recorded input and it changes what the world does, so a replay
/// that did not carry it would reproduce the postings and not the shifts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settlement {
    standing: Vec<bool>,
    wages: Vec<i64>,
}

impl Default for Settlement {
    fn default() -> Self {
        Self::opening()
    }
}

impl Settlement {
    /// A camp: nothing built, every wage at its authored opening.
    pub fn opening() -> Self {
        Self {
            standing: INDUSTRIES.iter().map(|_| false).collect(),
            wages: INDUSTRIES.iter().map(|spec| spec.wage).collect(),
        }
    }

    /// Whether industry `index` is standing.
    pub fn standing(&self, index: usize) -> bool {
        self.standing.get(index).copied().unwrap_or(false)
    }

    /// What a shift at industry `index` pays.
    pub fn wage(&self, index: usize) -> i64 {
        self.wages.get(index).copied().unwrap_or(0)
    }

    /// Whether anything at all has been built — the camp/settlement beat.
    pub fn any_standing(&self) -> bool {
        self.standing.iter().any(|built| *built)
    }

    /// The settlement as a stamp carries it: `works:camp-works=20g standing`.
    pub fn stamp(&self) -> String {
        let body = INDUSTRIES
            .iter()
            .enumerate()
            .map(|(index, spec)| {
                format!(
                    "{}={}g {}",
                    spec.id,
                    self.wage(index),
                    if self.standing(index) {
                        "standing"
                    } else {
                        "unbuilt"
                    }
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("works:{body}")
    }
}

/// Why a build was refused — surfaced as a toast, never silent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The settlement module is switched off, so there is nothing to build.
    NoSettlement,
    /// It is already standing.
    Standing,
    /// The treasury does not hold what it costs.
    Poor {
        /// What is held.
        held: i64,
        /// What it costs.
        cost: i64,
    },
}

impl Refusal {
    /// The toast's sentence — what happened, and what would fix it.
    pub fn message(&self, name: &str) -> String {
        match self {
            Refusal::NoSettlement => {
                format!("{name} cannot be built - the settlement module is off")
            }
            Refusal::Standing => format!("{name} is already standing"),
            Refusal::Poor { held, cost } => {
                format!("{name} costs {cost}g and the treasury holds {held}g")
            }
        }
    }
}

/// **Which industry this site's standing slots belong to**, if any.
///
/// The one question that tells a completed job which of GDD §4.1's ports its
/// gold moves through, and it is a fact about the site rather than a branch on
/// a name.
pub fn industry_at(sim: &Sim, site: usize) -> Option<usize> {
    sim.sites.get(site).and_then(|site| site.industry)
}

/// **Build it** — the treasury's first real sink (GDD §4.1's BURN port).
///
/// Debits exactly the industry's cost, stands it up, opens exactly its slots,
/// and says both: `built` names the beat and `industry-opened` names the work.
/// Nothing else writes `Sim::settlement`, so what the panel shows and what the
/// scorer can take are the same fact.
pub fn build(sim: &mut Sim, now: u64, index: usize) -> Result<(), Refusal> {
    let Some(spec) = INDUSTRIES.get(index) else {
        return Err(Refusal::NoSettlement);
    };
    if !sim.modules.enabled(MODULE) {
        return Err(Refusal::NoSettlement);
    }
    if sim.settlement.standing(index) {
        return Err(Refusal::Standing);
    }
    if sim.treasury < spec.cost {
        return Err(Refusal::Poor {
            held: sim.treasury,
            cost: spec.cost,
        });
    }
    sim.treasury -= spec.cost;
    sim.ports.burned_building += spec.cost;
    sim.settlement.stand(index);
    open_slots(sim, index);
    let tile = crate::grid::LOCATIONS[crate::grid::TOWN].tile;
    let first = !sim
        .settlement
        .standing
        .iter()
        .enumerate()
        .any(|(other, built)| other != index && *built);
    let treasury = sim.treasury;
    sim.emit_settlement(
        now,
        EventClass::Built,
        tile,
        if first {
            format!(
                "built {} for {}g ({treasury}g held) - Kawaza is a settlement now, not a camp",
                spec.name, spec.cost
            )
        } else {
            format!("built {} for {}g ({treasury}g held)", spec.name, spec.cost)
        },
    );
    sim.emit_settlement(
        now,
        EventClass::IndustryOpened,
        tile,
        format!(
            "opened {} standing {} slots at {} - {}g a shift",
            spec.slots,
            spec.task.id(),
            spec.name,
            sim.settlement.wage(index)
        ),
    );
    Ok(())
}

/// Fill the industry's site with its standing slots, all open.
fn open_slots(sim: &mut Sim, index: usize) {
    let Some(spec) = INDUSTRIES.get(index) else {
        return;
    };
    let wage = sim.settlement.wage(index);
    let Some(site) = sim.sites.get_mut(spec.site) else {
        return;
    };
    site.quests = (0..spec.slots)
        .map(|_| Quest {
            name: spec.shift,
            task: spec.task,
            pot: wage,
            duration: spec.minutes,
        })
        .collect();
    site.states = (0..spec.slots)
        .map(|_| crate::sim::JobState::Open)
        .collect();
}

impl Settlement {
    /// Stand an industry up. Private to the module's own `build`.
    fn stand(&mut self, index: usize) {
        if let Some(built) = self.standing.get_mut(index) {
            *built = true;
        }
    }
}

/// **Step an industry's wage**, held inside the panel's range — the one write,
/// so nothing can set a wage the panel could not have.
///
/// The wage is the slot's own pot, because the scorer weighs a pot and a shift
/// is a job: writing it onto the rows is what makes the lever reach the
/// decision without a term of its own. Returns what the wage now is.
pub fn step_wage(sim: &mut Sim, index: usize, delta: i64) -> i64 {
    let held = (sim.settlement.wage(index) + delta).clamp(0, WAGE_MAX);
    if let Some(wage) = sim.settlement.wages.get_mut(index) {
        *wage = held;
    }
    if let Some(spec) = INDUSTRIES.get(index)
        && let Some(site) = sim.sites.get_mut(spec.site)
    {
        for quest in &mut site.quests {
            quest.pot = held;
        }
    }
    held
}

/// **A shift is finished: pay it** (GDD §4.1's industry ports).
///
/// The wage is *minted* into the worker's wallet — an industry makes what it
/// pays, which is what separates this port from a posting's wage coming out of
/// the treasury — and the levy is minted into the treasury beside it. The
/// regard the paying moves is `answers::wage_regard`, the same
/// wage-vs-expectation rule the standing rates use (GDD §4.2), read against
/// the standing rate for the shift's own kind of work.
///
/// Returns `(paid, levied)` for the completion's own sentence.
pub fn settle_shift(sim: &mut Sim, tuning: &Tuning, who: usize, site: usize) -> (i64, i64) {
    let Some(index) = industry_at(sim, site) else {
        return (0, 0);
    };
    let Some(spec) = INDUSTRIES.get(index) else {
        return (0, 0);
    };
    let wage = sim.settlement.wage(index);
    let levy = tuning.industry_levy.clamp(0, wage);
    let paid = wage - levy;
    if let Some(person) = sim.people.get_mut(who) {
        person.wallet += paid;
    }
    sim.treasury += levy;
    let expectation = sim.rates.of(spec.task);
    crate::answers::wage_regard(sim, tuning, who, wage, expectation);
    (paid, levy)
}

/// **A standing slot is standing again.**
///
/// What makes an industry different from a site: a site's row is spent when it
/// is done and this one is not, so the settlement never runs dry and the
/// scorer always has somewhere to send a person who needs paying.
pub fn reopen(sim: &mut Sim, job: JobId) {
    if industry_at(sim, job.site).is_none() {
        return;
    }
    if let Some(state) = sim
        .sites
        .get_mut(job.site)
        .and_then(|site| site.states.get_mut(job.slot))
    {
        *state = crate::sim::JobState::Open;
    }
}

/// **Who is working this industry right now**, in registry order — what the
/// panel's row says and what a build decision turns on.
pub fn hands_on(lens: &crate::lens::Lens<'_>, index: usize) -> Vec<usize> {
    let Some(spec) = INDUSTRIES.get(index) else {
        return Vec::new();
    };
    let Some(site) = lens.site(spec.site) else {
        return Vec::new();
    };
    site.states
        .iter()
        .filter_map(|state| match state {
            crate::sim::JobState::Claimed { by } => Some(*by),
            _ => None,
        })
        .collect()
}

/// How many of an industry's slots nobody has taken.
pub fn free_slots(lens: &crate::lens::Lens<'_>, index: usize) -> usize {
    INDUSTRIES
        .get(index)
        .and_then(|spec| lens.site(spec.site))
        .map_or(0, crate::sim::Site::open_count)
}

/// The registry's own validation: the industry table is authorable data and
/// the site it names is the one it gets.
pub fn registry(checks: &mut crate::checks::Checks, tuning: &Tuning) {
    let sim = Sim::opening(tuning, crate::modules::ModuleSet::ALL);
    for (index, spec) in INDUSTRIES.iter().enumerate() {
        checks.require(
            !spec.id.is_empty()
                && spec
                    .id
                    .chars()
                    .all(|glyph| glyph.is_ascii_lowercase() || glyph == '-'),
            "an industry id is not stamp-shaped ASCII",
            format!("INDUSTRIES[{index}] is named {:?}", spec.id),
        );
        checks.require(
            spec.slots > 0 && spec.slots <= crate::layout::BOARD_ROWS,
            "an industry opens more standing slots than a board has rows",
            format!(
                "{:?} opens {} slots and the board holds {}",
                spec.id,
                spec.slots,
                crate::layout::BOARD_ROWS
            ),
        );
        checks.require(
            spec.cost > 0 && spec.minutes > 0,
            "an industry is free to build or takes no time to work",
            format!(
                "{:?} costs {}g a shift of {} minutes",
                spec.id, spec.cost, spec.minutes
            ),
        );
        checks.require(
            spec.wage == sim.rates.of(spec.task),
            "an industry does not open at the standing rate for its own work",
            format!(
                "{:?} opens at {}g and {} work stands at {}g; the per-industry wage and the \
                 standing rates are one policy family, so the opening settlement pays camp \
                 work what it says it pays",
                spec.id,
                spec.wage,
                spec.task.id(),
                sim.rates.of(spec.task)
            ),
        );
        checks.require(
            industry_at(&sim, spec.site) == Some(index),
            "an industry's site is not the site that says it is that industry",
            format!(
                "{:?} names site {} and that site reports industry {:?}",
                spec.id,
                spec.site,
                industry_at(&sim, spec.site)
            ),
        );
        checks.require(
            sim.sites
                .get(spec.site)
                .is_some_and(|site| site.quests.is_empty()),
            "an industry opens the scenario already standing",
            format!(
                "{:?}'s site carries work before anybody built it; the camp opens as a camp",
                spec.id
            ),
        );
        checks.require(
            crate::sim::site_location(spec.site) == crate::grid::TOWN,
            "an industry does not stand at the camp",
            format!(
                "{:?} stands at {:?}; the first building is the camp's own (CAST.md §1)",
                spec.id,
                crate::grid::LOCATIONS[crate::sim::site_location(spec.site)].name
            ),
        );
    }
}
