//! **The injector, played** — the half of its battery that runs worlds
//! rather than staging one (`directed.rs` is the other half).
//!
//! Every expectation a shipped literal.
//!
//! - [`pinned`] — the test scenario's pin fires at its minute, exactly, at
//!   every speed.
//! - [`replay`] — same seed, same firings; another seed, other firings.
//! - [`sweeps`] — the idle sweep re-judged with the director on: the calm
//!   window in every world, the cap never exceeded, Steve still first, and
//!   the attention differential with and without the director.
//! - [`module_off`] — injector off is the quiet world; petitions off silences
//!   the injector too.

use crate::attention::EventClass;
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::directed::{EQUALITY_DAYS, fingerprint};
use crate::director;
use crate::economy::Player;
use crate::modules::ModuleSet;
use crate::petitions::{self, Source};
use crate::pleas::Status;
use crate::scenario::{self, Scenario};
use crate::sim::Sim;

/// Roster indices the cases name.
const STEVE: usize = 1;

/// The minute the test scenario pins D1 at — the file's own literal.
pub const PIN_MINUTE: u64 = 90;

/// The test scenario.
pub fn pinned_scenario() -> &'static Scenario {
    match scenario::find("pinned-collector") {
        Some(found) => found,
        None => crate::checks::fail(
            "the pinned test scenario is missing",
            "scenario::FILES must carry scenarios/pinned-collector.txt",
        ),
    }
}

/// **The pin fires at its minute, exactly, at every speed** — the test
/// scenario conducted under the three speed scripts, the player resuming
/// from the voicing's stop by the key the script is running at.
pub fn pinned(checks: &mut Checks) -> String {
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
                tap(When::Minute(60), Key::Space),
                tap(
                    When::MinuteHeld {
                        minute: 60,
                        after: 40,
                    },
                    Key::Space,
                ),
                tap(When::Minute(80), Key::Digit3),
            ],
        ),
    ];
    let mut runs = Vec::new();
    for (name, script) in &scripts {
        let mut session = Session::plain(Tuning::SHIPPED, script, 40_000);
        session.scenario = pinned_scenario();
        session.stop_at_minute = Some(240);
        session.resume_after = Some((Key::Space, 2));
        runs.push((*name, conduct(&session)));
    }
    let lines = |run: &crate::sweep::Conducted| -> Vec<(u64, &'static str, usize, String)> {
        run.events
            .iter()
            .map(|event| {
                (
                    event.minute,
                    event.class.name(),
                    event.party,
                    event.note.clone(),
                )
            })
            .collect()
    };
    let base = lines(&runs[0].1);
    for (name, run) in &runs {
        let fired: Vec<(u64, String)> = run
            .events
            .iter()
            .filter(|event| event.class == EventClass::Event)
            .map(|event| (event.minute, event.note.clone()))
            .collect();
        let petition = run.sim.petitions.all().first().map(|petition| {
            (
                petition.who,
                petition.template.id,
                petition.raised_at,
                petition.voiced_at,
            )
        });
        checks.require(
            fired
                == [(
                    PIN_MINUTE,
                    "is reached by an event from outside the camp (the-collector-comes, pinned \
                     at d1 01:30)"
                        .to_owned(),
                )]
                && petition == Some((0, "the-collector-comes", PIN_MINUTE, Some(PIN_MINUTE)))
                && run.sim.paused_by.is_none(),
            "the pinned firing did not fire at its minute, exactly",
            format!(
                "{name}: the director's lines are {fired:?} and the first petition is \
                 {petition:?}; the file pins the-collector-comes for Bob at minute {PIN_MINUTE}, \
                 and with the director off nothing else may fire"
            ),
        );
        checks.require(
            lines(run) == base,
            "the pinned firing is not speed-invariant",
            format!("{name}'s transcript parts from {}'s", runs[0].0),
        );
    }
    format!(
        "pinned: the-collector-comes for Bob at minute {PIN_MINUTE} under 3 speed scripts, \
         voiced the same minute, {} identical events each",
        base.len()
    )
}

/// The director's own record of a world: every firing's minute and line, and
/// every director petition's who, template, minutes and words.
fn directed(sim: &Sim) -> Vec<String> {
    let mut out: Vec<String> = sim
        .events
        .iter()
        .filter(|event| event.class == EventClass::Event)
        .map(|event| format!("{} {} {}", event.minute, event.party, event.note))
        .collect();
    out.extend(
        sim.petitions
            .all()
            .iter()
            .filter(|petition| petition.template.source == Source::Director)
            .map(|petition| {
                format!(
                    "{} {} {} {:?} {}",
                    petition.who,
                    petition.template.id,
                    petition.raised_at,
                    petition.voiced_at,
                    petition.text
                )
            }),
    );
    out
}

/// **Same seed and orders, the same petitions at the same minutes** — and
/// another seed, other ones: the seed reaches the director.
pub fn replay(checks: &mut Checks) -> String {
    let tuning = Tuning::SHIPPED;
    let run = |seed: u64| {
        crate::economy::world(
            scenario::freeplay(),
            &tuning,
            ModuleSet::ALL,
            seed,
            EQUALITY_DAYS,
            Player::Attentive,
        )
    };
    let (once, twice, other) = (run(3), run(3), run(4));
    let (a, b, c) = (directed(&once), directed(&twice), directed(&other));
    checks.require(
        !a.is_empty() && a == b && fingerprint(&once) == fingerprint(&twice) && a != c,
        "the director's firings are not addressed by the seed and the minute",
        format!(
            "seed 3 fired {} director lines the first time and {} the second ({} the same), and \
             seed 4 fired {}; the same seed must replay to the byte and another must differ",
            a.len(),
            b.len(),
            if a == b { "all" } else { "not all" },
            c.len()
        ),
    );
    format!(
        "director replayed: {} lines at seed 3, word for word, and seed 4 differs",
        a.len()
    )
}

/// **The most director petitions standing unresolved at any minute** — over
/// the record, by every raising minute.
fn most_standing(sim: &Sim) -> usize {
    let spans: Vec<(u64, u64)> = sim
        .petitions
        .all()
        .iter()
        .filter(|petition| petition.template.source == Source::Director)
        .map(|petition| {
            let end = match petition.status {
                Status::Met { at, .. } | Status::Failed { at } => at,
                _ => u64::MAX,
            };
            (petition.raised_at, end)
        })
        .collect();
    spans
        .iter()
        .map(|(at, _)| {
            spans
                .iter()
                .filter(|(from, to)| from <= at && at < to)
                .count()
        })
        .max()
        .unwrap_or(0)
}

/// How many director-sourced petitions a world raised.
fn raised(sim: &Sim) -> usize {
    sim.petitions
        .all()
        .iter()
        .filter(|petition| petition.template.source == Source::Director)
        .count()
}

/// The idle worlds' director petitions, `(fewest, most)` over the sweep — a
/// shipped literal.
pub const IDLE_DIRECTED: (usize, usize) = (1, 2);
/// The attentive worlds'.
pub const ATTENTIVE_DIRECTED: (usize, usize) = (3, 8);

/// **The idle sweep re-judged with the director on** (freeplay's default),
/// and the attention differential read with it and without it.
pub fn sweeps(checks: &mut Checks) -> String {
    let tuning = Tuning::SHIPPED;
    let days = crate::petitioned::SWEEP_DAYS;
    let worlds = crate::petitioned::SWEEP_WORLDS;
    let live = |player: Player, modules: ModuleSet| -> Vec<Sim> {
        (0..worlds)
            .map(|world| {
                crate::economy::world(scenario::freeplay(), &tuning, modules, world, days, player)
            })
            .collect()
    };
    let quiet = ModuleSet::ALL.without_id(director::MODULE);
    let (idle, attentive) = (
        live(Player::Idle, ModuleSet::ALL),
        live(Player::Attentive, ModuleSet::ALL),
    );
    let calm = director::calm_until(&tuning);
    for (world, sim) in idle.iter().chain(attentive.iter()).enumerate() {
        let idle_world = world < idle.len();
        let early_event = sim
            .events
            .iter()
            .find(|event| event.class == EventClass::Event && event.minute < calm)
            .map(|event| event.minute);
        let early_raise = sim
            .petitions
            .all()
            .iter()
            .find(|petition| {
                petition.template.source == Source::Director && petition.raised_at < calm
            })
            .map(|petition| petition.raised_at);
        let fired = sim
            .events
            .iter()
            .filter(|event| event.class == EventClass::Event)
            .count();
        checks.require(
            early_event.is_none() && early_raise.is_none() && fired > 0,
            "the director broke its calm window, or never spoke",
            format!(
                "world {world}: a firing at {early_event:?} and a director petition raised at \
                 {early_raise:?} before minute {calm}, and {fired} firings in all; the calm \
                 window holds in every world and the director speaks after it"
            ),
        );
        let most = most_standing(sim);
        checks.require(
            most <= director::cap(&tuning),
            "more director petitions stood unresolved than the cap allows",
            format!(
                "world {world}: {most} stood at once and director_max is {}",
                director::cap(&tuning)
            ),
        );
        let first_short = sim
            .events
            .iter()
            .find(|event| event.class == EventClass::UpkeepShortfall)
            .map(|event| event.party);
        // **Steve still goes short first** — the idle sweep's claim
        // (`CAST.md` §4.1's pariah-candidate); an attentive player may post
        // him work before his first interval, which is the point of one.
        checks.require(
            !idle_world || first_short == Some(STEVE),
            "somebody other than Steve goes short first with the director on",
            format!("idle world {world}: the first shortfall is {first_short:?}; CAST.md s4.1 says Steve"),
        );
    }
    let band = |sims: &[Sim]| -> (usize, usize) {
        sims.iter()
            .map(raised)
            .fold((usize::MAX, 0), |(low, high), count| {
                (low.min(count), high.max(count))
            })
    };
    let (idle_band, attentive_band) = (band(&idle), band(&attentive));
    checks.require(
        idle_band == IDLE_DIRECTED && attentive_band == ATTENTIVE_DIRECTED,
        "the director's pressure does not land in the bands the shipped set states",
        format!(
            "over {worlds} worlds of {days} days the director raised {idle_band:?} petitions in \
             idle worlds and {attentive_band:?} in attentive ones; the shipped bands are \
             {IDLE_DIRECTED:?} and {ATTENTIVE_DIRECTED:?}"
        ),
    );
    // --- the differential, with and without the director ---------------------
    let margin = |idle: &[Sim], attentive: &[Sim]| -> (usize, usize, i64) {
        let worst = attentive
            .iter()
            .map(|sim| crate::petitioned::tally(sim).failed)
            .max()
            .unwrap_or(0);
        let best = idle
            .iter()
            .map(|sim| crate::petitioned::tally(sim).failed)
            .min()
            .unwrap_or(0);
        (worst, best, best as i64 - worst as i64)
    };
    let (on_worst, on_best, on) = margin(&idle, &attentive);
    let (off_worst, off_best, off) =
        margin(&live(Player::Idle, quiet), &live(Player::Attentive, quiet));
    checks.require(
        on >= off,
        "the director narrowed the attention differential",
        format!(
            "with the director on, attention's worst world fails {on_worst} petitions against \
             neglect's best {on_best} (a margin of {on}); with it off, {off_worst} against \
             {off_best} ({off}); external pressure should widen the margin, or hold it"
        ),
    );
    format!(
        "director on, {worlds} worlds x 2 players x {days} days: calm until minute {calm} in \
         every world, never more than {} standing, Steve first every time; director petitions \
         idle {idle_band:?}, attentive {attentive_band:?}; the petition margin is {on} with the \
         director and {off} without",
        director::cap(&tuning)
    )
}

/// **Module-off, both ways**: the injector off is the quiet world, and
/// petitions off silences the injector too — asserted, not assumed.
pub fn module_off(checks: &mut Checks) -> String {
    let tuning = Tuning::SHIPPED;
    let days = crate::petitioned::SWEEP_DAYS;
    for (what, modules) in [
        (
            "the injector off",
            ModuleSet::ALL.without_id(director::MODULE),
        ),
        (
            "petitions off",
            ModuleSet::ALL.without_id(petitions::MODULE),
        ),
    ] {
        checks.require(
            !director::runs(modules),
            "the injector runs in a world it should be silent in",
            format!("with {what}, director::runs says it runs"),
        );
        for player in [Player::Idle, Player::Attentive] {
            for (scenario, name) in [
                (scenario::freeplay(), "freeplay"),
                (pinned_scenario(), "pinned"),
            ] {
                let sim = crate::economy::world(scenario, &tuning, modules, 0, days, player);
                let fired = sim
                    .events
                    .iter()
                    .filter(|event| event.class == EventClass::Event)
                    .count();
                checks.require(
                    fired == 0 && raised(&sim) == 0,
                    "the injector spoke in a world it should be silent in",
                    format!(
                        "with {what}, {name} under the {player:?} player fired {fired} times and \
                         raised {} director petitions over {days} days",
                        raised(&sim)
                    ),
                );
            }
        }
    }
    format!(
        "injector off and petitions off: no firing, no pin and no director petition in freeplay \
         or the pinned scenario, under both players over {days} days; T1-T6 with the injector \
         off are the scenario equality's worlds"
    )
}
