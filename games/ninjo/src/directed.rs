//! **The injector's battery** — what GDD §9 owes the events-director and the
//! scenario file wave 1.6 built.
//!
//! Every expectation a shipped literal, never derived from `tuning` or from
//! the template table: the mutation round runs [`judge_at`] at moved
//! constants and [`rows_round`] over moved D1-D3 rows, and a check that
//! recomputed its expectation from the thing under test could not see it
//! move (`make-game` §A.6).
//!
//! - [`scenario_equality`] — the freeplay file is the authored start moved,
//!   not redesigned: byte-identical transcripts against the pre-refactor
//!   build, with the director off either way.
//! - [`judge_at`] — the calm window, the drawn gaps and the cap, staged.
//! - [`rows_round`] — D1-D3 raised staged, and every literal of every row
//!   moved once and noticed.
//!
//! The injector *played* — the pin, the replay, the sweeps and module-off —
//! is `pressed.rs`, beside it.

use crate::attention::EventClass;
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::director;
use crate::economy::Player;
use crate::modules::ModuleSet;
use crate::petitions::{self, Template};
use crate::scenario::{self, Scenario};
use crate::sim::Sim;

/// **A world, as one number** — every event (minute, class, party, gold,
/// petition, tier, sentence), every person's end state, every petition's
/// record, the treasury and the ports, folded through FNV-1a. Two worlds
/// with one fingerprint told the same story to the byte.
pub fn fingerprint(sim: &Sim) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |text: &str| {
        for byte in text.bytes().chain(std::iter::once(b'\n')) {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for event in &sim.events {
        eat(&format!(
            "{} {} {} {} {:?} {:?} {}",
            event.minute,
            event.class.name(),
            event.party,
            event.gold,
            event.petition,
            event.tier,
            event.note
        ));
    }
    for person in &sim.people {
        eat(&format!(
            "{} {} {} {} {} {} {}",
            person.id,
            person.wallet,
            person.desperation,
            person.present,
            person.present_from,
            person.shortfalls,
            person.source
        ));
    }
    for petition in sim.petitions.all() {
        eat(&format!(
            "{} {} {} {:?} {} {:?} {}",
            petition.who,
            petition.template.id,
            petition.raised_at,
            petition.voiced_at,
            petition.deadline,
            petition.status,
            petition.text
        ));
    }
    eat(&format!("{} {}", sim.treasury, sim.ports.line()));
    hash
}

/// **The pre-refactor build's opening, fingerprinted** — taken from the
/// commit before the scenario file landed (`e9b5950`'s tree), where the
/// roster's balances were literals in `people.rs`.
pub const OPENING_FINGERPRINT: u64 = 0x9091_64cb_2ef5_7c0b;

/// **The pre-refactor build's worlds, fingerprinted** — twelve world-days
/// each, at three seeds under both players, taken from the same tree. The
/// freeplay file with the director off must reproduce every one to the byte.
pub const BEFORE: [(Player, u64, u64); 6] = [
    (Player::Idle, 0, 0xd472_1c21_eae9_d9b4),
    (Player::Idle, 7, 0x0439_527d_66c6_7439),
    (Player::Idle, 42, 0x2607_8351_8498_dc96),
    (Player::Attentive, 0, 0xd3b3_61a8_84a8_d2f2),
    (Player::Attentive, 7, 0x5975_0833_7e9d_d645),
    (Player::Attentive, 42, 0x9ebc_80b9_31ad_b5bc),
];

/// How many world-days the equality worlds run.
pub const EQUALITY_DAYS: u64 = 12;

/// **Freeplay with the director's switch thrown** — the same file, read, with
/// `director off`: an instrument, leaked once, because a scenario is
/// `'static` everywhere a world holds one.
fn freeplay_quiet() -> &'static Scenario {
    use std::sync::LazyLock;
    static QUIET: LazyLock<Scenario> = LazyLock::new(|| Scenario {
        director: false,
        ..scenario::freeplay().clone()
    });
    &QUIET
}

/// **The scenario equality** — the freeplay file at a given seed produces a
/// byte-identical transcript to the pre-refactor build at that seed with the
/// director off, both ways the director can be off: its module switched out,
/// and the file's own `director off`.
pub fn scenario_equality(checks: &mut Checks) -> String {
    let tuning = Tuning::SHIPPED;
    let opening = fingerprint(&Sim::opening(scenario::freeplay(), &tuning, ModuleSet::ALL));
    checks.require(
        opening == OPENING_FINGERPRINT,
        "the freeplay file does not open the world the authored start opened",
        format!(
            "the opening fingerprints {opening:#018x} and the pre-refactor build's was \
             {OPENING_FINGERPRINT:#018x}; the scenario file moved the start, it may not change it"
        ),
    );
    let mut matched = 0;
    for (player, seed, want) in BEFORE {
        for (how, scenario, modules) in [
            (
                "the injector switched out",
                scenario::freeplay(),
                ModuleSet::ALL.without_id(director::MODULE),
            ),
            ("the file's director off", freeplay_quiet(), ModuleSet::ALL),
        ] {
            let sim =
                crate::economy::world(scenario, &tuning, modules, seed, EQUALITY_DAYS, player);
            let got = fingerprint(&sim);
            checks.require(
                got == want,
                "the freeplay file's world is not the pre-refactor world with the director off",
                format!(
                    "{player:?} at seed {seed} over {EQUALITY_DAYS} days with {how} fingerprints \
                     {got:#018x} ({} events) and the pre-refactor build's was {want:#018x}",
                    sim.events.len()
                ),
            );
            matched += usize::from(got == want);
        }
    }
    format!(
        "scenario equality: the freeplay file opens the pre-refactor world ({OPENING_FINGERPRINT:#x}) \
         and {matched} of {} director-off worlds ({EQUALITY_DAYS} days, seeds 0/7/42, both \
         players, module off and file off) are byte-identical to it",
        BEFORE.len() * 2
    )
}

/// **The first firing in idle world zero**, a shipped literal: the minute
/// the calm window, the gap drawn at its end and the target together put
/// it at.
pub const FIRST_FIRING: u64 = 5885;
/// The gap drawn at the calm window's end, at seed zero.
pub const FIRST_GAP: u64 = 1565;
/// What two hundred gaps drawn at seed zero, minutes 0 to 199, add up to.
pub const GAP_SUM: u64 = 289_292;

/// **The injector, judged at a stated constants set** — every expectation a
/// shipped literal: the instrument the mutation round reads the three
/// pressure params through.
pub fn judge_at(checks: &mut Checks, tuning: &Tuning) {
    // --- the calm window and the drawn gap ----------------------------------
    let calm = director::calm_until(tuning);
    let first = director::gap(tuning, 0, calm);
    let gaps: Vec<u64> = (0..200)
        .map(|minute| director::gap(tuning, 0, minute))
        .collect();
    let sum: u64 = gaps.iter().sum();
    let (low, high) = (
        gaps.iter().copied().min().unwrap_or(0),
        gaps.iter().copied().max().unwrap_or(0),
    );
    checks.require(
        calm == 4320 && first == FIRST_GAP && sum == GAP_SUM && low >= 720 && high <= 2160,
        "the director's clock is not what the shipped set says",
        format!(
            "the calm window ends at {calm}, the first gap is {first}, two hundred gaps sum to \
             {sum} between {low} and {high}; the shipped set says 4320, {FIRST_GAP}, {GAP_SUM}, \
             inside 720 to 2160"
        ),
    );
    // --- the first firing, played -------------------------------------------
    let idle = crate::economy::world(
        scenario::freeplay(),
        tuning,
        ModuleSet::ALL,
        0,
        5,
        Player::Idle,
    );
    let firing = idle
        .events
        .iter()
        .find(|event| event.class == EventClass::Event)
        .map(|event| event.minute);
    checks.require(
        firing == Some(FIRST_FIRING),
        "the director first fires at a minute the shipped set does not say",
        format!(
            "idle world zero's first event is at {firing:?}; the shipped set says {FIRST_FIRING}"
        ),
    );
    // --- the cap ------------------------------------------------------------
    let mut staged = staged_world(tuning);
    director::fire(&mut staged, tuning, 5000);
    director::fire(&mut staged, tuning, 5001);
    director::fire(&mut staged, tuning, 5002);
    let standing = director::active(&staged);
    let passed = staged
        .events
        .iter()
        .filter(|event| event.class == EventClass::Event && event.note.contains("already in play"))
        .count();
    checks.require(
        standing == 1 && passed == 2,
        "the director's cap does not hold at the shipped set",
        format!(
            "three firings in a staged camp leave {standing} director petitions standing and \
             {passed} firings passed at the cap; the shipped director_max says 1 and 2"
        ),
    );
}

/// A staged world: freeplay, everybody present, the director's clock not yet
/// run.
fn staged_world(tuning: &Tuning) -> Sim {
    let mut sim = Sim::opening(scenario::freeplay(), tuning, ModuleSet::ALL);
    sim.everybody_here();
    sim
}

/// **One director row raised in a staged camp** — what [`rows_round`]
/// compares: who it reached, its `{n}`, its deadline, its site, its words,
/// its declared kind and days, and its "met when" line.
type Raised = (
    usize,
    i64,
    u64,
    Option<usize>,
    String,
    &'static str,
    u64,
    String,
);

/// Raise `row` in the staged camp at minute 5000, seed zero.
fn raise_staged(row: &'static Template) -> Option<Raised> {
    let tuning = Tuning::SHIPPED;
    let mut sim = staged_world(&tuning);
    director::fire_from(&mut sim, &tuning, 5000, &[row]);
    let petition = sim.petitions.all().first()?;
    let lens = crate::lens::Lens::on(&sim);
    Some((
        petition.who,
        petition.n,
        petition.deadline,
        petition.site,
        petition.text.clone(),
        petition.declared().kind.id,
        petition.declared().days,
        petition.template.condition.met_when(&lens, petition),
    ))
}

/// **D1-D3, raised staged** — the shipped literals the round compares.
fn shipped_raised() -> [(&'static str, Raised); 3] {
    [
        (
            "the-collector-comes",
            (
                7,
                30,
                13640,
                None,
                "Ludo: I owe 30 gold to a man who counts days. Find me work that pays before \
                 d10 11:20, or he takes it out of me."
                    .to_owned(),
                "broke",
                0,
                "Ludo's purse holds 30g, any time before d10 11:20".to_owned(),
            ),
        ),
        (
            "rival-offer",
            (
                5,
                31,
                12200,
                Some(3),
                "Goro: The Grey Banners came through the Black Vault offering 31 a week. Show \
                 me what I am worth here before d9 11:20, or I will go find out."
                    .to_owned(),
                "walks-out",
                5,
                "Goro is paid 31g or more in wages and shares, before d9 11:20".to_owned(),
            ),
        ),
        (
            "word-from-the-road",
            (
                5,
                0,
                10760,
                Some(3),
                "Goro: Travelers say the Black Vault is worth somebody's time again - good pots \
                 for whoever moves first. We should be first."
                    .to_owned(),
                "sours",
                0,
                "a job at the Black Vault is finished by anybody, before d8 11:20".to_owned(),
            ),
        ),
    ]
}

/// **D1-D3's literals, broken on purpose** — every row raised staged against
/// its shipped literal, then each literal of each row moved once and the
/// staged raise asked to notice. `N of N noticed`, or the literal is not
/// being measured.
pub fn rows_round(checks: &mut Checks) -> String {
    let shipped = shipped_raised();
    for (id, want) in &shipped {
        let got = petitions::find(id).and_then(raise_staged);
        checks.require(
            got.as_ref() == Some(want),
            "a director template does not raise what the shipped table says",
            format!("{id} raised {got:?} and the shipped literal is {want:?}"),
        );
    }
    let mut mutants: Vec<(String, Template)> = Vec::new();
    for (id, _) in &shipped {
        let Some(row) = petitions::find(id).copied() else {
            continue;
        };
        let other_kind = if row.consequence.kind.id == "sours" {
            &petitions::DECLARED[2]
        } else {
            &petitions::DECLARED[0]
        };
        let swapped: &'static [crate::traits::TraitId] = match row.trigger {
            petitions::Trigger::Carries { any, .. }
                if any.contains(&crate::traits::TraitId::Restless) =>
            {
                &[crate::traits::TraitId::Caring]
            }
            _ => &[crate::traits::TraitId::Restless],
        };
        let site = match row.trigger {
            petitions::Trigger::Carries { site, .. } => site,
            _ => petitions::SiteSlot::None,
        };
        let moved_site = match site {
            petitions::SiteSlot::None => petitions::SiteSlot::Any,
            _ => petitions::SiteSlot::None,
        };
        let other_condition = if row.condition == petitions::Condition::PaidWork {
            petitions::Condition::CraftDone
        } else {
            petitions::Condition::PaidWork
        };
        for (what, mutant) in [
            (
                "deadline +1 day",
                Template {
                    deadline_days: row.deadline_days + 1,
                    ..row
                },
            ),
            (
                "{n} +7",
                Template {
                    n: row.n + 7,
                    ..row
                },
            ),
            (
                "spread moved",
                Template {
                    spread: if row.spread == 0 { 25 } else { 0 },
                    ..row
                },
            ),
            (
                "consequence swapped",
                Template {
                    consequence: other_kind,
                    ..row
                },
            ),
            (
                "traits swapped",
                Template {
                    trigger: petitions::Trigger::Carries { any: swapped, site },
                    ..row
                },
            ),
            (
                "site slot moved",
                Template {
                    trigger: petitions::Trigger::Carries {
                        any: match row.trigger {
                            petitions::Trigger::Carries { any, .. } => any,
                            _ => &[],
                        },
                        site: moved_site,
                    },
                    ..row
                },
            ),
            (
                "condition swapped",
                Template {
                    condition: other_condition,
                    ..row
                },
            ),
            (
                "words changed",
                Template {
                    text: "{name}: Something came in from outside.",
                    ..row
                },
            ),
        ] {
            mutants.push((format!("{id} {what}"), mutant));
        }
    }
    let mut missed: Vec<String> = Vec::new();
    let total = mutants.len();
    for (name, mutant) in mutants {
        let leaked: &'static Template = Box::leak(Box::new(mutant));
        let got = raise_staged(leaked);
        let want = shipped
            .iter()
            .find(|(id, _)| *id == leaked.id)
            .map(|(_, want)| want.clone());
        if got == want {
            missed.push(name);
        }
    }
    checks.require(
        missed.is_empty(),
        "a director template's literal can be changed without the staged raise noticing",
        format!("{missed:?} raised exactly what the shipped rows raise"),
    );
    format!(
        "D1-D3 raised staged as shipped; row mutation round: {} of {total} noticed",
        total - missed.len()
    )
}
