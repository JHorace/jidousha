//! **Needs**: what living in Kawaza costs, and what it does to somebody who
//! cannot pay (GDD §5's needs module, wave 1.3).
//!
//! # One list, one port, one consequence
//!
//! The needs list is data ([`NEEDS`], GDD §6's `kind, interval, base cost`)
//! and v1 has one row: **coin upkeep, per interval, trait-modulated**. The
//! cost of a row for one person is `traits::upkeep_of` over that row's base —
//! written at wave 1.1 and read by nothing until now, which is why this module
//! adds no arithmetic of its own to a trait's meaning. Its base is a *drawer
//! field on the row*, so a second need is a row here and a constant there and
//! this file does not change.
//!
//! The burn is GDD §4.1's BURN port and the only one that takes gold out of a
//! wallet. **Nobody is ever overdrawn**: what is in the purse is what is
//! taken, and what was owed and could not be found is the shortfall.
//!
//! # The shortfall is the escalation pipe's first rung
//!
//! Going short **presses on desperation** — which already opens the scorer's
//! sum (`autonomy::weigh`), so a sliding person takes worse work with no
//! change to the scorer at all — and **rewrites the person's source line to
//! the event** (GDD §3). The second half is the one that keeps the first
//! honest: two people at desperation five have to be two different problems,
//! and a number that rose for the same printed reason would make them one.
//! So a rewrite composes onto `Character::origin` and never replaces it.
//!
//! # Ambient, and therefore quiet
//!
//! A burn that was met emits nothing, for the reason regard's drift emits
//! nothing: nothing *happened* to anybody, and the feed is for things that
//! did (GDD §3's vocabulary is explicit — a need is "a field, managed by
//! policy, never by per-person attention"). A burn that was **not** met is an
//! occurrence with a time, a place and a class, like everything else.

use crate::constants::{Field, Tuning};
use crate::lens::Lens;
use crate::sim::Sim;
use crate::traits;

/// The module id, as GDD §5's registry spells it.
pub const MODULE: &str = "needs";

/// **What one shortfall presses desperation by.**
///
/// One, and a code constant rather than a drawer row for the reason
/// `asks::RATE_STEP` is one: it is the size of a step, not a number anybody
/// tunes, and the drawer row that *is* the economy's lever is the base cost
/// the step is a consequence of.
pub const PRESS: i64 = 1;

/// **How desperate the `desperate` chip counts from.**
///
/// Presentation only — it changes what the chip *says* and never what the
/// scorer *does*, which is why it is a constant here rather than a term
/// anywhere (`asks::RELUCTANT_MARGIN` is the same kind of number, for the same
/// reason). Six of a possible ten: past the highest the roster is authored at,
/// so nobody is on the chip before anything has happened to them.
pub const DESPERATE_AT: i64 = 6;

/// One row of the needs list — GDD §6's `kind, interval, base cost`.
#[derive(Clone, Copy, Debug)]
pub struct NeedSpec {
    /// The id a stamp, a report and the class table name it by.
    pub id: &'static str,
    /// What a surface calls it.
    pub label: &'static str,
    /// **The drawer row this need's interval is read from**, in world-hours.
    ///
    /// A field like [`NeedSpec::base`], so the list stays data and both halves
    /// of a need stay tunable: GDD §6 gives the needs list a kind, an interval
    /// and a base cost, and the two that are numbers are both rows of the one
    /// place numbers live.
    pub every: Field,
    /// **The drawer row this need's base cost is read from**, before the
    /// carrier's motivators multiply it.
    ///
    /// A field rather than a number, so the list stays data *and* the cost
    /// stays tunable — GDD §9 asks that a mutated upkeep constant break an
    /// economy band, and this is how the row and that constant are the same
    /// thing.
    pub base: Field,
}

/// **The needs list.** One row at v1 (GDD §6), and a second is a row here
/// plus a constant in `constants.rs`.
pub const NEEDS: &[NeedSpec] = &[NeedSpec {
    id: "coin",
    label: "upkeep",
    every: Field::UpkeepHours,
    base: Field::UpkeepCoin,
}];

impl NeedSpec {
    /// How many world-hours this need falls due on, at a stated constants set.
    pub fn interval_hours(&self, tuning: &Tuning) -> i64 {
        tuning.field(self.every)
    }
}

/// How many world-minutes lie between two burns of this need — never zero,
/// because an interval of nothing would be an occurrence that fell due at the
/// minute it was scheduled, forever.
pub fn interval(tuning: &Tuning, need: &NeedSpec) -> u64 {
    u64::try_from(tuning.field(need.every)).unwrap_or(1).max(1) * 60
}

/// **What this need costs this person, one interval**, with their motivators
/// on it (`traits::upkeep_of`).
///
/// The one answer: the burn takes it, the `short` chip predicts it, the camp
/// line adds it up and the settlement panel prints it. A surface that worked
/// out its own upkeep would be the second answer GDD §1 refuses.
pub fn cost_of(tuning: &Tuning, need: &NeedSpec, carried: &[traits::TraitId]) -> i64 {
    traits::upkeep_of(tuning.field(need.base), carried)
}

/// **What one person owes this interval**, over every need there is.
pub fn owed_by(tuning: &Tuning, carried: &[traits::TraitId]) -> i64 {
    NEEDS
        .iter()
        .map(|need| cost_of(tuning, need, carried))
        .sum()
}

/// **What the settlement burns in a world-day**, derived from the present
/// roster and never estimated (the camp line's own rule).
///
/// Every present person's cost for every need, times the number of times that
/// need falls due in a day. The reader is the state-of-the-camp line, and the
/// figure is exact by construction: it is the same `cost_of` the burn takes.
pub fn burn_per_day(lens: &Lens<'_>, tuning: &Tuning) -> i64 {
    if !lens.needs_on() {
        return 0;
    }
    let mut out = 0;
    for need in NEEDS {
        let a_day = crate::autonomy::DAY / interval(tuning, need);
        let per_interval: i64 = lens
            .roll()
            .into_iter()
            .map(|who| cost_of(tuning, need, lens.traits(who)))
            .sum();
        out += per_interval * i64::try_from(a_day).unwrap_or(0);
    }
    out
}

/// **Whether this person cannot meet their upkeep out of what they hold** —
/// the `short` chip's question, and the same comparison [`burn`] makes.
///
/// One derivation, so the chip counts the set the simulation is about to act
/// on rather than a set that resembles it.
pub fn is_short(lens: &Lens<'_>, tuning: &Tuning, who: usize) -> bool {
    lens.needs_on() && lens.present(who) && lens.wallet(who) < owed_by(tuning, lens.traits(who))
}

/// Whether the need has pressed this person past the chip's threshold.
pub fn is_desperate(lens: &Lens<'_>, who: usize) -> bool {
    lens.present(who) && lens.desperation(who) >= DESPERATE_AT
}

/// **The first burn of each need**, as the scenario schedules them.
pub fn first_burn(tuning: &Tuning, need: &NeedSpec) -> u64 {
    interval(tuning, need)
}

/// **One interval of one need falls due** — the BURN port (GDD §4.1).
///
/// Walked in registry order over everybody who is *present*, because somebody
/// who has not arrived is not eating the camp's food. What is in the purse is
/// what is taken: a wallet never goes negative, which is the limp-floor's
/// arithmetic half — the settlement gets poorer and nobody is removed from it.
///
/// Returns how many people went short, for the batteries to read.
pub fn burn(sim: &mut Sim, tuning: &Tuning, now: u64, row: usize) -> usize {
    let Some(need) = NEEDS.get(row) else {
        return 0;
    };
    if !sim.modules.enabled(MODULE) {
        return 0;
    }
    let mut short = 0;
    for who in 0..sim.people.len() {
        let Some(person) = sim.people.get(who) else {
            continue;
        };
        if !person.present {
            continue;
        }
        let cost = cost_of(tuning, need, &person.traits);
        if cost <= 0 {
            continue;
        }
        let held = person.wallet;
        if held >= cost {
            if let Some(person) = sim.people.get_mut(who) {
                person.wallet -= cost;
            }
            sim.ports.burned_upkeep += cost;
            continue;
        }
        // **Short.** What they had is taken, what they could not find is the
        // shortfall, and the shortfall is what presses.
        let owed = cost - held;
        let line = {
            let Some(person) = sim.people.get_mut(who) else {
                continue;
            };
            person.wallet = 0;
            person.shortfalls += 1;
            crate::people::press(person, PRESS);
            person.source = rewritten(person.origin, now, person.shortfalls);
            person.source.clone()
        };
        sim.ports.burned_upkeep += held;
        let tile = sim.parties.get(who).map_or_else(
            || crate::grid::LOCATIONS[crate::grid::TOWN].tile,
            |party| party.tile,
        );
        sim.emit_need(
            now,
            who,
            tile,
            format!("could not find {owed}g for {} - {line}", need.label),
        );
        short += 1;
    }
    short
}

/// **The source line a shortfall writes** (GDD §3).
///
/// It composes onto the line they were generated with rather than replacing
/// it, because the whole job of this sentence is to make two identical
/// desperations two different problems — and a line that said only what
/// happened would say the same thing about everybody it happened to.
///
/// What it does **not** carry is the amount: the occurrence's own note says
/// how much could not be found, and the feed prints the two together, so a
/// line that repeated it would say one number twice on one row.
pub fn rewritten(origin: &str, now: u64, times: u64) -> String {
    format!(
        "{origin} - and went short on day {} ({times} in all)",
        now / crate::autonomy::DAY + 1
    )
}

/// **The state of the camp, in one line** (UI.md §3a, wave 1.3).
///
/// Every figure derived from the same per-character truths the shortfall logic
/// uses: who is present is the lens's roll, who is short is [`is_short`], the
/// treasury is the sim's own and the burn is [`burn_per_day`] over the present
/// roster. **There is no income or net figure**, because this build records no
/// flow window to derive one from and a guessed one would be the surface that
/// disagrees with the sim (GDD §1); the handoff's instruction is to omit it
/// rather than estimate it, and this is that omission, on purpose.
pub fn camp_line(lens: &Lens<'_>, tuning: &Tuning) -> String {
    let here = lens.roll().len();
    let short = lens
        .roll()
        .into_iter()
        .filter(|who| is_short(lens, tuning, *who))
        .count();
    // **Short on purpose.** It stands on the meters band, right of the chips,
    // and what is left of that band is 348 reference pixels — thirty-seven
    // glyphs of the five-by-seven face. The longest this can run is a camp of
    // ten with a five-figure treasury, which is thirty-six, so the line never
    // clips and never has to be read twice. The settlement panel prints the
    // same string with the idle count after it (UI.md §3g), because two
    // wordings of one subject are two things that can drift.
    format!(
        "{here} here - {short} short - {}g - {}g/d",
        lens.treasury(),
        burn_per_day(lens, tuning)
    )
}

/// **The needs module, judged at a stated constants set** — every expectation
/// a shipped literal, never derived from `tuning`.
///
/// Staged: no run is conducted, so the whole battery is a few microseconds and
/// the claims are about arithmetic rather than about timing.
pub fn judge_at(checks: &mut crate::checks::Checks, tuning: &Tuning) {
    let Some(need) = NEEDS.first() else {
        return;
    };
    // **The trait side, read for the first time** (`traits::upkeep_of`, written
    // at wave 1.1 and called by nothing until now). Three sheets, three
    // multipliers, and the numbers are written down: the base is 5, Steve
    // carries `caring` at 3/2 and pays 7, Bob carries `indebted` at 5/4 and
    // pays 6, and Alex carries neither and pays the base.
    let cast = crate::people::roster();
    for (id, wanted) in [("steve", 7i64), ("bob", 6), ("alex", 5), ("tim", 5)] {
        let Some(person) = cast.iter().find(|person| person.id == id) else {
            continue;
        };
        checks.require(
            cost_of(tuning, need, &person.traits) == wanted,
            "upkeep is not what the carrier's motivators multiply it to",
            format!(
                "{id} pays {} an interval and the shipped economy asks {wanted}",
                cost_of(tuning, need, &person.traits)
            ),
        );
    }
    // **The burn, and the floor inside it**: what is in the purse is what is
    // taken, so a wallet is never overdrawn, and what could not be found is
    // what presses.
    let mut sim = crate::sim::Sim::opening(tuning, crate::modules::ModuleSet::ALL);
    sim.everybody_here();
    let opened: Vec<i64> = sim.people.iter().map(|person| person.wallet).collect();
    let desperation: Vec<i64> = sim.people.iter().map(|person| person.desperation).collect();
    let origins: Vec<&'static str> = sim.people.iter().map(|person| person.origin).collect();
    let short = burn(&mut sim, tuning, 0, 0);
    checks.require(
        sim.people.iter().all(|person| person.wallet >= 0),
        "one interval of upkeep overdrew somebody",
        format!(
            "the purses read {:?} after a burn of the shipped base; upkeep takes what is \
             there and the shortfall is the rest",
            sim.people
                .iter()
                .map(|person| person.wallet)
                .collect::<Vec<_>>()
        ),
    );
    checks.require(
        short == SHORT_ON_THE_FIRST_BURN,
        "the first interval of upkeep did not press the people it is meant to",
        format!(
            "{short} of the camp went short on the first burn and the shipped economy \
             presses {SHORT_ON_THE_FIRST_BURN}"
        ),
    );
    for (who, person) in sim.people.iter().enumerate() {
        let went_short = person.shortfalls > 0;
        checks.require(
            person.desperation == desperation[who] + i64::from(went_short) * PRESS,
            "a shortfall pressed desperation by something other than one step",
            format!(
                "{} opened at {} and reads {} after {} shortfall(s)",
                person.name, desperation[who], person.desperation, person.shortfalls
            ),
        );
        // **And the source line is rewritten to the event** (GDD §3) —
        // composed onto what they came here as, never replacing it, because
        // the whole job of this sentence is to keep two identical
        // desperations two different problems.
        checks.require(
            person.source.starts_with(origins[who])
                && (person.source != origins[who]) == went_short,
            "a shortfall did not rewrite the source line to the event",
            format!(
                "{} went short {} time(s) and their source reads {:?}; it opened {:?}",
                person.name, person.shortfalls, person.source, origins[who]
            ),
        );
        checks.require(
            person.wallet == (opened[who] - cost_of(tuning, need, &person.traits)).max(0),
            "a burn took something other than what the need costs",
            format!(
                "{} opened at {}g, owes {}g and holds {}g",
                person.name,
                opened[who],
                cost_of(tuning, need, &person.traits),
                person.wallet
            ),
        );
    }
    // **Two people at one desperation are two problems.** The claim the source
    // line exists for, asserted over the world the burn just made rather than
    // over the roster it started from.
    for (a, first) in sim.people.iter().enumerate() {
        for second in sim.people.iter().skip(a + 1) {
            checks.require(
                first.desperation != second.desperation || first.source != second.source,
                "two people at one desperation are the same problem",
                format!(
                    "{} and {} both read desperation {} and both say {:?}",
                    first.name, second.name, first.desperation, first.source
                ),
            );
        }
    }
    // **Desperation is held inside its range** — pressed past the ceiling and
    // it stops, so the scorer's opening term stays a comparison.
    let mut pressed = crate::sim::Sim::opening(tuning, crate::modules::ModuleSet::ALL);
    pressed.everybody_here();
    for person in &mut pressed.people {
        person.wallet = 0;
    }
    for interval in 0..40u64 {
        burn(&mut pressed, tuning, interval, 0);
    }
    checks.require(
        pressed
            .people
            .iter()
            .all(|person| person.desperation == crate::people::DESPERATION_MAX),
        "forty short intervals did not press everybody to the ceiling and no further",
        format!(
            "the camp reads {:?} against a ceiling of {}",
            pressed
                .people
                .iter()
                .map(|person| person.desperation)
                .collect::<Vec<_>>(),
            crate::people::DESPERATION_MAX
        ),
    );
    // **The camp line is derived, never estimated** (the handoff's own rule):
    // every figure on it is the same per-character truth the burn reads.
    let lens = crate::lens::Lens::on(&sim);
    let wanted = crate::autonomy::DAY / interval(tuning, need);
    let by_hand: i64 = lens
        .roll()
        .into_iter()
        .map(|who| cost_of(tuning, need, lens.traits(who)))
        .sum::<i64>()
        * i64::try_from(wanted).unwrap_or(0);
    checks.require(
        burn_per_day(&lens, tuning) == by_hand,
        "the camp line's daily burn is not the present roster's own upkeep",
        format!(
            "the line says {}g a day and the roster's own costs come to {by_hand}g",
            burn_per_day(&lens, tuning)
        ),
    );
    let line = camp_line(&lens, tuning);
    checks.require(
        line.contains(&format!("{by_hand}g/d"))
            && line.contains(&format!("{} here", lens.roll().len()))
            && line.contains(&format!("{}g -", lens.treasury())),
        "the camp line does not say the numbers it derived",
        format!("it reads {line:?}"),
    );
    // **And it fits the band it stands on**, at the widest the settlement can
    // make it: a line that clipped would be a glance the player has to open
    // something else to finish reading.
    let widest = format!(
        "{} here - {} short - {}g - {}g/d",
        lens.people().len(),
        lens.people().len(),
        99_999,
        999
    );
    let style = crate::theme::text(crate::theme::SMALL, crate::theme::INK);
    checks.require(
        !crate::checks::greater(style.width_of(&widest), crate::layout::camp_line_width()),
        "the state-of-the-camp line does not fit the band it stands on",
        format!(
            "its widest reading is {:.0}px and the band leaves {:.0}px right of the chips",
            style.width_of(&widest),
            crate::layout::camp_line_width()
        ),
    );
    // **And it says nothing about income**, on purpose: this build records no
    // flow window to derive one from, and a guessed figure on the glance band
    // would be the surface that disagrees with the sim (GDD §1).
    checks.require(
        !line.contains("income") && !line.contains("net"),
        "the camp line estimates a flow the simulation does not record",
        format!("it reads {line:?}; omit it rather than guess it"),
    );
}

/// How many of the camp cannot meet the very first interval of upkeep, at the
/// shipped economy and with everybody staged present.
///
/// A shipped literal: Steve, Rin and Ludo open with less than one interval in
/// their purses. Never derived from the roster, because a check that counted
/// the roster would count whatever the roster happened to be.
pub const SHORT_ON_THE_FIRST_BURN: usize = 3;

/// **The needs and settlement modules, over a conducted world** — the
/// invariance sweep's extension over this wave's three new sources, the two
/// module-off passes, and the claim that a pressed person takes worse work.
pub fn judge_module(
    checks: &mut crate::checks::Checks,
    baseline: &crate::sweep::Conducted,
) -> String {
    // --- 1: the arrival is world-time addressed ----------------------------
    //
    // Rin walks into Kawaza at the minute `CAST.md` §4 gives her, and the
    // baseline's own transcript is where that is asserted
    // (`sweep::expected_events`). What is asserted here is the *other* half —
    // that it happens at the same world-minute under every speed script —
    // which `sweep::run` already compares transcript for transcript. This is
    // the presence claim the transcript cannot make: the camp the run ends
    // with is the camp the arrival column says it should be.
    let joined: Vec<(u64, usize)> = baseline
        .events
        .iter()
        .filter(|event| event.class == crate::attention::EventClass::Joined)
        .map(|event| (event.minute, event.party))
        .collect();
    let wanted: Vec<(u64, usize)> = crate::people::roster()
        .iter()
        .enumerate()
        .filter(|(_, person)| !person.present && person.present_from <= crate::sweep::RUN_UNTIL)
        .map(|(who, person)| (person.present_from, who))
        .collect();
    checks.require(
        joined == wanted,
        "the staged start did not seat the camp the arrival column describes",
        format!(
            "the run reports arrivals {joined:?} and CAST.md §4's column schedules {wanted:?} \
             inside the run"
        ),
    );
    checks.require(
        baseline
            .sim
            .people
            .iter()
            .filter(|person| person.present)
            .count()
            == crate::people::founders().len() + joined.len(),
        "the camp at the end of the run is not the founders plus the arrivals",
        format!(
            "{} people are present after {} arrivals onto a founding band of {}",
            baseline
                .sim
                .people
                .iter()
                .filter(|person| person.present)
                .count(),
            joined.len(),
            crate::people::founders().len()
        ),
    );

    // --- 2: the upkeep and its shortfall, under every speed ----------------
    //
    // The shipped interval is a world-day and the invariance window is half of
    // one, so the source is swept at an interval that fits the window. **What
    // is under test is the addressing**, not the interval: an occurrence with
    // a world-time address is speed-invariant by construction, and one that is
    // not is exactly the failure the substrate exists to prevent — so the
    // claim is made where the source actually fires.
    let pressing = Tuning::SHIPPED.with(crate::constants::Field::UpkeepHours, PRESSING_HOURS);
    let mut transcripts: Vec<(String, Vec<crate::sweep::Entry>)> = Vec::new();
    let mut purses: Vec<(String, Vec<i64>)> = Vec::new();
    for (name, script) in crate::sweep::speed_scripts() {
        let run = crate::sweep::conduct(&crate::sweep::Session::plain(pressing, &script, 60_000));
        transcripts.push((name.to_owned(), crate::sweep::transcript(&run.events)));
        purses.push((
            name.to_owned(),
            run.sim.people.iter().map(|person| person.wallet).collect(),
        ));
    }
    let (first, rest) = transcripts.split_at(1);
    for (name, theirs) in rest {
        checks.require(
            *theirs == first[0].1,
            "the upkeep interval is not addressed in world-time",
            format!(
                "{name}'s transcript differs from {}'s under a pressing upkeep; an occurrence \
                 with a world-time address fires at the same world-minute at every speed",
                first[0].0
            ),
        );
    }
    let (held, others) = purses.split_at(1);
    for (name, theirs) in others {
        checks.require(
            *theirs == held[0].1,
            "the same world-time of upkeep burned different amounts at different speeds",
            format!(
                "{name} ends holding {theirs:?} and {} ends holding {:?}",
                held[0].0, held[0].1
            ),
        );
    }
    let shortfalls = first[0]
        .1
        .iter()
        .filter(|entry| entry.1 == crate::attention::EventClass::UpkeepShortfall.name())
        .count();
    checks.require(
        shortfalls == PRESSING_SHORTFALLS,
        "the pressing upkeep did not produce the shortfalls the sweep is over",
        format!(
            "the window carries {shortfalls} shortfalls and the shipped literal is \
             {PRESSING_SHORTFALLS}; a sweep over a source that never fires passes vacuously"
        ),
    );

    // --- 3: desperation is already the scorer's opening term ---------------
    //
    // **Verified, not built** (the handoff's own instruction). Nothing in
    // `autonomy.rs` changed this wave; what changed is that somebody can now
    // be pressed. So the claim is that pressing them moves the sum.
    let tuning = Tuning::SHIPPED;
    let mut easy = crate::sim::Sim::opening(&tuning, crate::modules::ModuleSet::ALL);
    easy.everybody_here();
    let mut pressed = easy.clone();
    let who = 3usize;
    pressed.people[who].desperation = crate::people::DESPERATION_MAX;
    let job = crate::sim::JobId { site: 3, slot: 3 };
    let weigh = |sim: &crate::sim::Sim| {
        crate::autonomy::weigh(
            sim,
            &tuning,
            0,
            who,
            crate::autonomy::Action::SeekWork { job },
        )
        .iter()
        .map(|term| term.value)
        .sum::<i64>()
    };
    let (before, after) = (weigh(&easy), weigh(&pressed));
    checks.require(
        after - before
            == (crate::people::DESPERATION_MAX - easy.people[who].desperation) * tuning.need_weight,
        "pressing somebody did not move the scorer's opening term",
        format!(
            "the same job weighs {before} at desperation {} and {after} at the ceiling; \
             desperation opens the sum and nothing in the scorer changed this wave",
            easy.people[who].desperation
        ),
    );

    // --- 4: the two module-off worlds --------------------------------------
    //
    // The degrades-to sentences, as facts. With needs off nothing is burned
    // and nobody goes short; with settlement off there is nothing to build and
    // no standing slot to work.
    let needs_off = crate::modules::MODULES
        .iter()
        .position(|spec| spec.id == MODULE)
        .unwrap_or(0);
    let mut quiet =
        crate::sim::Sim::opening(&tuning, crate::modules::ModuleSet::ALL.without(needs_off));
    quiet.everybody_here();
    let opened: Vec<i64> = quiet.people.iter().map(|person| person.wallet).collect();
    let grid = crate::grid::grid();
    crate::sim::advance_to(&mut quiet, &grid, &tuning, 3 * crate::autonomy::DAY);
    let purses: Vec<i64> = quiet.people.iter().map(|person| person.wallet).collect();
    checks.require(
        quiet.ports.burned_upkeep == 0
            && quiet
                .people
                .iter()
                .all(|person| person.shortfalls == 0 && person.source == person.origin)
            && purses.iter().zip(&opened).all(|(now, was)| now >= was),
        "with needs off something was still paid for",
        format!(
            "three world-days burned {}g of upkeep, the purses went {opened:?} -> {purses:?} \
             and {} people went short; with the module off nothing is paid for and wealth \
             accumulates",
            quiet.ports.burned_upkeep,
            quiet
                .people
                .iter()
                .filter(|person| person.shortfalls > 0)
                .count()
        ),
    );
    let settlement_off = crate::modules::MODULES
        .iter()
        .position(|spec| spec.id == crate::settlement::MODULE)
        .unwrap_or(0);
    let mut camp = crate::sim::Sim::opening(
        &tuning,
        crate::modules::ModuleSet::ALL.without(settlement_off),
    );
    camp.everybody_here();
    camp.treasury = 10_000;
    let refused = crate::settlement::build(&mut camp, 0, 0);
    crate::sim::advance_to(&mut camp, &grid, &tuning, 3 * crate::autonomy::DAY);
    checks.require(
        refused == Err(crate::settlement::Refusal::NoSettlement)
            && !camp.settlement.any_standing()
            && camp.ports.minted_wages == 0
            && camp.ports.burned_building == 0,
        "with settlement off something was still built",
        format!(
            "the build returned {refused:?}, {}g was minted in wages and {}g burned in \
             building; with the module off the camp stays a camp",
            camp.ports.minted_wages, camp.ports.burned_building
        ),
    );
    let list = NEEDS
        .iter()
        .map(|need| format!("{} every {}h", need.id, need.interval_hours(&tuning)))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "needs and settlement: [{list}], {} arrival(s) in the window at the authored minutes, upkeep \
         swept under 3 speed scripts at {PRESSING_HOURS}h ({shortfalls} shortfalls, identical \
         transcripts and purses), desperation still opens the scorer's sum, and both \
         module-off worlds degrade as the registry says",
        joined.len()
    )
}

/// The interval the invariance sweep presses at, in world-hours.
///
/// Two: the shipped interval is a world-day and the sweep's window is half of
/// one, so the source would never fire inside it. What is swept is the
/// **addressing**, which is the same at any interval.
pub const PRESSING_HOURS: i64 = 2;

/// And how many shortfalls that produces inside the invariance window — a
/// shipped literal, so a sweep over a source that stopped firing fails
/// instead of passing vacuously.
pub const PRESSING_SHORTFALLS: usize = 8;
