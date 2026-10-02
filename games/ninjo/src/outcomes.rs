//! **The resolution battery** — the distribution sweep, the one-function
//! claims and the photographed session (GDD §9, wave 1.4).
//!
//! `resolution.rs` is the module: the curve, the roll and the payout. This is
//! what holds it to account, in its own file for the reason `compliance.rs`
//! is the asks module's: the record and the rules are one subject and the
//! checks over them are another, and the two together were longer than a file
//! is allowed to be.
//!
//! Every expectation here is a **shipped literal**. The mutation round runs
//! [`judge_at`] at moved constants to see whether they are measured at all,
//! and a check that computed its expectation from the constant under test
//! could not see that constant move.

use crate::constants::Tuning;
use crate::resolution::{
    Odds, Tier, fit_cell, odds, odds_for, payout, resolve, roll, rolls_at, settle, tier_of,
};
use crate::sim::{JobId, Sim};

/// The seeds the staged distribution sweep walks — the population a roll is
/// drawn over, sixty-four of them like the economy sweep's worlds.
pub const SWEEP_SEEDS: u64 = 64;

/// The world-minutes each staged job is rolled at, per seed — four
/// occurrences a day apart, so the sweep is over addresses and not one
/// address repeated.
pub const SWEEP_MINUTES: &[u64] = &[180, 1620, 3060, 4500];

/// **The staged sweep's tally at a strong fit** (fit 2): `(failed, went well,
/// done)` over every authored site job × [`SWEEP_SEEDS`] ×
/// [`SWEEP_MINUTES`] — 6144 rolls.
pub const STRONG_TALLY: (usize, usize, usize) = (674, 1491, 3979);

/// And at a poor fit (fit 0), over the same addresses.
pub const POOR_TALLY: (usize, usize, usize) = (2165, 0, 3979);

/// **The bands** (GDD §9's distribution sweep): a strong fit fails less often
/// than this, in percent of the rolls…
pub const STRONG_FAIL_CEILING: usize = 15;

/// …a poor fit more often than this…
pub const POOR_FAIL_FLOOR: usize = 28;

/// …and a strong fit goes well at least this often — what the went-well tier
/// is worth having at all.
pub const STRONG_WELL_FLOOR: usize = 18;

/// **The staged distribution sweep**: every authored site job, rolled at
/// every sweep seed and minute, at fit `fit` — `(failed, went well, done)`.
///
/// The roll is the game's own (`roll`, `tier_of`, `odds`), over the occurrence
/// addresses a played world would produce; nothing is sampled twice.
pub fn tally(tuning: &Tuning, fit: i64) -> (usize, usize, usize) {
    let sim = Sim::opening(tuning, crate::modules::ModuleSet::ALL);
    let at = odds(tuning, fit);
    let mut out = (0, 0, 0);
    for seed in 0..SWEEP_SEEDS {
        for minute in SWEEP_MINUTES.iter().copied() {
            for (site, board) in sim.sites.iter().enumerate() {
                if !rolls_at(board) {
                    continue;
                }
                for slot in 0..board.quests.len() {
                    match tier_of(at, roll(seed, minute, JobId { site, slot })) {
                        Tier::Failed => out.0 += 1,
                        Tier::WentWell => out.1 += 1,
                        Tier::Done => out.2 += 1,
                    }
                }
            }
        }
    }
    out
}

/// A count as a whole percent of a total, rounded down.
fn percent(part: usize, whole: usize) -> usize {
    (part * 100).checked_div(whole).unwrap_or(0)
}

/// The resolution battery, at a stated constants set — **every expectation a
/// shipped literal**, never derived from `tuning`.
pub fn judge_at(checks: &mut crate::checks::Checks, tuning: &Tuning) {
    fn judge(checks: &mut crate::checks::Checks, what: &'static str, got: String, want: &str) {
        checks.require(
            got == want,
            what,
            format!("resolution answers {got} and the shipped set says {want}"),
        );
    }
    // --- the curve, and its words ----------------------------------------
    let strong = odds(tuning, 2);
    let poor = odds(tuning, 0);
    judge(
        checks,
        "the fit-to-odds curve is not what the shipped set says",
        format!("{strong:?} {poor:?} done {} {}", strong.done(), poor.done()),
        "Odds { fail: 11, well: 24 } Odds { fail: 35, well: 0 } done 65 65",
    );
    judge(
        checks,
        "the odds-words are not what the shipped thresholds say",
        format!(
            "{} {} {}",
            strong.word(tuning).name(),
            poor.word(tuning).name(),
            Odds { fail: 22, well: 0 }.word(tuning).name()
        ),
        "safe risky chancy",
    );
    // --- the payout, every case ------------------------------------------
    let mut sim = Sim::opening(tuning, crate::modules::ModuleSet::ALL);
    sim.everybody_here();
    let haul = JobId { site: 1, slot: 0 }; // the mushroom haul, 40g
    let cases = [
        payout(&sim, tuning, haul, None, Tier::Done),
        payout(&sim, tuning, haul, None, Tier::WentWell),
        payout(&sim, tuning, haul, None, Tier::Failed),
        payout(&sim, tuning, haul, Some(16), Tier::Done),
        payout(&sim, tuning, haul, Some(16), Tier::Failed),
    ];
    judge(
        checks,
        "the payout function does not pay what the shipped share says",
        cases
            .iter()
            .map(|paid| format!("{}/{}/{}", paid.wage, paid.share, paid.pot))
            .collect::<Vec<_>>()
            .join(" "),
        "0/1/39 0/1/39 0/0/0 16/0/40 16/0/0",
    );
    // **Moved, not just computed**: settling a self-chosen failure moves no
    // gold anywhere, and a posted failure moves the wage and nothing else.
    let ludo = sim
        .people
        .iter()
        .position(|person| person.id == "ludo")
        .unwrap_or(0);
    let before = (sim.treasury, sim.people[ludo].wallet, sim.ports);
    let mut failed = sim.clone();
    let _ = settle(&mut failed, tuning, ludo, haul, Tier::Failed);
    checks.require(
        (failed.treasury, failed.people[ludo].wallet, failed.ports) == before,
        "a self-chosen failure moved gold",
        format!(
            "the treasury went {} -> {}, the purse {} -> {} and the ports {:?}; a failed job of \
             their own mints nothing and pays nothing",
            before.0, failed.treasury, before.1, failed.people[ludo].wallet, failed.ports
        ),
    );
    // --- one function: the pay the scorer weighs is the pay paid ---------
    let weighed = crate::autonomy::pay_for(&sim, tuning, haul, None);
    let mut done = sim.clone();
    let paid = settle(&mut done, tuning, ludo, haul, Tier::Done);
    let said = crate::autonomy::weigh(
        &sim,
        tuning,
        0,
        ludo,
        crate::autonomy::Action::SeekWork { job: haul },
    )
    .into_iter()
    .find(|term| term.what == "share")
    .map(|term| term.because);
    checks.require(
        weighed == paid.to_worker()
            && done.people[ludo].wallet - sim.people[ludo].wallet == weighed
            && said.as_deref() == Some(format!("their share is {weighed}g").as_str()),
        "the pay the scorer weighs is not the pay a completion pays",
        format!(
            "the scorer weighs {weighed}g and says {said:?}, the completion pays {}g into a \
             purse that moved {}g; one payout function must answer both",
            paid.to_worker(),
            done.people[ludo].wallet - sim.people[ludo].wallet
        ),
    );
    judge(
        checks,
        "a self-chooser's share is not what the shipped share says",
        format!("{weighed}"),
        "1",
    );
    // --- the staged distribution sweep -----------------------------------
    let (strong_tally, poor_tally) = (tally(tuning, 2), tally(tuning, 0));
    judge(
        checks,
        "the staged distribution sweep does not tally what the shipped curve does",
        format!("{strong_tally:?} {poor_tally:?}"),
        &format!("{STRONG_TALLY:?} {POOR_TALLY:?}"),
    );
    let whole = |tally: (usize, usize, usize)| tally.0 + tally.1 + tally.2;
    checks.require(
        percent(strong_tally.0, whole(strong_tally)) < STRONG_FAIL_CEILING
            && percent(poor_tally.0, whole(poor_tally)) > POOR_FAIL_FLOOR
            && percent(strong_tally.1, whole(strong_tally)) >= STRONG_WELL_FLOOR
            && strong_tally.1 < strong_tally.2
            && poor_tally.1 < poor_tally.2,
        "the outcome distribution is outside the bands fit is meant to make",
        format!(
            "a strong fit fails {}% and goes well {}% (bands: under {STRONG_FAIL_CEILING}%, at \
             least {STRONG_WELL_FLOOR}%), a poor fit fails {}% (band: over \
             {POOR_FAIL_FLOOR}%), and went-well against done is {} to {} and {} to {}; fit \
             must matter, and going well must be rarer than plain done everywhere",
            percent(strong_tally.0, whole(strong_tally)),
            percent(strong_tally.1, whole(strong_tally)),
            percent(poor_tally.0, whole(poor_tally)),
            strong_tally.1,
            strong_tally.2,
            poor_tally.1,
            poor_tally.2
        ),
    );
}

/// **The played half of the distribution sweep** — every job the economy
/// sweep's worlds resolved, by the worker's fit.
///
/// The staged sweep says what the roll does over addresses; this says what
/// the camp actually got, which is the same roll over the jobs people chose
/// and were posted to. Two claims beyond the staged bands: **somebody still
/// takes poor-fit work** (desperate people must still rationally take it —
/// tuning until nobody does would make fit a wall rather than a risk), and a
/// strong fit fails less often than a poor one in the played record too.
pub fn judge_played(checks: &mut crate::checks::Checks, played: &[(i64, Tier)]) -> String {
    let count = |strong: bool, tier: Option<Tier>| {
        played
            .iter()
            .filter(|(fit, got)| (*fit > 0) == strong && tier.is_none_or(|want| *got == want))
            .count()
    };
    let (strong, poor) = (count(true, None), count(false, None));
    let strong_failed = count(true, Some(Tier::Failed));
    let poor_failed = count(false, Some(Tier::Failed));
    let well = count(true, Some(Tier::WentWell)) + count(false, Some(Tier::WentWell));
    let done = count(true, Some(Tier::Done)) + count(false, Some(Tier::Done));
    checks.require(
        poor >= PLAYED_POOR_FLOOR
            && percent(strong_failed, strong) < STRONG_FAIL_CEILING
            && percent(poor_failed, poor) > POOR_FAIL_FLOOR
            && well < done,
        "the played outcomes do not show fit mattering",
        format!(
            "over the economy sweep's worlds {strong} strong-fit jobs failed {strong_failed} \
             times and {poor} poor-fit jobs failed {poor_failed} times (bands: under \
             {STRONG_FAIL_CEILING}% and over {POOR_FAIL_FLOOR}%, and at least \
             {PLAYED_POOR_FLOOR} poor-fit jobs taken); {well} went well against {done} done"
        ),
    );
    format!(
        "played outcomes: strong fit {strong_failed} of {strong} failed ({}%), poor fit \
         {poor_failed} of {poor} ({}%), {well} went well against {done} done",
        percent(strong_failed, strong),
        percent(poor_failed, poor)
    )
}

/// The fewest poor-fit jobs the economy sweep's worlds must between them have
/// taken — the claim that desperate people still rationally take work they are
/// not fit for.
pub const PLAYED_POOR_FLOOR: usize = 1;

/// **One function, shown and rolled** (the wave's decision-surface row):
/// every odds-word on a job row equals the sim's own odds for that pair, and
/// mutating a curve constant moves the row and the roll **together** — or
/// the row is reading a copy.
///
/// Staged: the whole camp present, Alex selected (the scout, so the
/// Watchtower's board holds both a strong and a poor fit for one person), the
/// board drawn by `board::site_board` exactly as the game draws it, and the
/// roll taken by [`resolve`] itself at an address whose number lands between
/// the shipped and the mutated odds.
pub fn judge_one_function(checks: &mut crate::checks::Checks) -> String {
    let grid = crate::grid::grid();
    let shipped = Tuning::SHIPPED;
    let moved = shipped.with(crate::constants::Field::FailBase, 60);
    let mut sim = Sim::opening(&shipped, crate::modules::ModuleSet::ALL);
    sim.everybody_here();
    let alex = sim
        .people
        .iter()
        .position(|person| person.id == "alex")
        .unwrap_or(0);
    let flow = crate::flow::Flow {
        selected: Some(alex),
        ..crate::flow::Flow::default()
    };
    let words = |tuning: &Tuning| -> (Vec<String>, Vec<String>) {
        let lens = crate::lens::Lens::on(&sim);
        let panel = crate::board::site_board(&flow, &lens, &grid, tuning, 0, 0);
        let mut shown = Vec::new();
        let mut want = Vec::new();
        for (slot, quest) in sim.sites[0].quests.iter().enumerate() {
            let at = crate::layout::board_row(slot).min + crate::layout::job::FIT;
            shown.push(
                panel
                    .runs
                    .iter()
                    .find(|run| run.at == at)
                    .map_or_else(String::new, |run| run.text.clone()),
            );
            let odds = odds_for(&sim, tuning, alex, JobId { site: 0, slot });
            want.push(fit_cell(
                crate::traits::competence_at(quest.task, &sim.people[alex].traits),
                odds,
                tuning,
            ));
        }
        (shown, want)
    };
    let (shown, want) = words(&shipped);
    let (shown_moved, want_moved) = words(&moved);
    checks.require(
        shown == want && shown_moved == want_moved,
        "a job row's odds-word is not the sim's odds for that pair",
        format!(
            "the Watchtower's rows read {shown:?} for Alex and the sim's odds say {want:?}; at \
             fail_base 60 they read {shown_moved:?} against {want_moved:?}"
        ),
    );
    checks.require(
        shown != shown_moved,
        "a mutated curve constant did not move the row",
        format!("the rows read {shown:?} at the shipped curve and at fail_base 60 alike"),
    );
    // The roll at the same pair, at an address that lands between the odds.
    let watch = JobId { site: 0, slot: 0 }; // the beacon watch, scout work
    let minute = (0..20_000u64).find(|minute| {
        let rolled = roll(sim.seed, *minute, watch);
        (11..35).contains(&rolled)
    });
    let tiers = minute.map(|minute| {
        (
            resolve(&sim, &shipped, minute, alex, watch),
            resolve(&sim, &moved, minute, alex, watch),
        )
    });
    checks.require(
        tiers == minute.map(|_| (Tier::WentWell, Tier::Failed)),
        "a mutated curve constant did not move the roll with the row",
        format!(
            "at minute {minute:?} Alex's beacon watch resolves {tiers:?} at the shipped curve \
             and at fail_base 60; the row moved from safe to risky, so the roll must move from \
             going well to failing"
        ),
    );
    format!(
        "odds on the row are the roll's: Alex's Watchtower row reads {shown:?}, at fail_base 60 \
         {shown_moved:?}, and the roll at minute {} moves {tiers:?}",
        minute.unwrap_or(0)
    )
}

/// The seed the resolution session is photographed at — chosen, and said so:
/// at seed 3 the labourer's posted fight job fails at minute 156, a job goes
/// well at 478 and three jobs have failed by the end of day one, which is
/// every picture this session owes. The judge asserts each of those facts
/// rather than trusting this comment.
pub const SHOT_SEED: u64 = 3;

/// The minute the camp is photographed at, a day after the string of failures.
pub const AFTERMATH_MINUTE: u64 = 1700;

/// **The resolution module's photographed session** — the four pictures a
/// person looks at to see what this wave did (UI.md §5).
///
/// Steve, the labourer, is posted to a fight job at minute 16 — a poor fit,
/// and the board said `risky` — and it goes wrong: the world stops on
/// `task-failed`, which is the first picture. The second is that job's board
/// a little later with Steve selected, the row open again and reading
/// `fit 0 risky`. The third is the feed with a job that went well in it, and
/// the fourth the camp's own panel a day after three jobs failed.
pub fn shot_run() -> crate::sweep::Conducted {
    use crate::sweep::{Act, Directive, Photo, When};
    use jidousha::prelude::{Key, Vec2};
    let click = |when: When, at: Vec2| Directive {
        when,
        what: Act::ClickUi(at),
    };
    let marker = |location: usize| {
        crate::layout::marker_rect(crate::grid::LOCATIONS[location].tile).center()
    };
    let mut script = vec![Directive {
        when: When::Tick(5),
        what: Act::Tap(Key::Digit3),
    }];
    // **The poor fit**: Steve to the Old Crypt's second seal, fight work.
    script.extend(crate::sweep::post(16, 1, 2, 1));
    // The job's board after the failure, with Steve picked back up at home.
    script.push(Directive {
        when: When::Minute(200),
        what: crate::sweep::pick_at_home(1),
    });
    script.push(Directive {
        when: When::Minute(204),
        what: Act::ClickWorld(marker(crate::sim::site_location(2))),
    });
    script.push(click(
        When::Minute(232),
        crate::layout::board_close().center(),
    ));
    // The feed, with a job that went well in it.
    script.push(click(
        When::Minute(484),
        crate::layout::feed_button().center(),
    ));
    script.push(click(
        When::Minute(520),
        crate::layout::feed_button().center(),
    ));
    // The camp a day later.
    script.push(Directive {
        when: When::Minute(AFTERMATH_MINUTE - 12),
        what: Act::ClickWorld(marker(crate::grid::TOWN)),
    });
    let photos = [
        Photo {
            name: "failed",
            minute: 100,
            tick: 0,
            paused: true,
        },
        Photo {
            name: "reopened",
            minute: 216,
            tick: 0,
            paused: false,
        },
        Photo {
            name: "wentwell",
            minute: 496,
            tick: 0,
            paused: false,
        },
        Photo {
            name: "aftermath",
            minute: AFTERMATH_MINUTE,
            tick: 0,
            paused: false,
        },
    ];
    crate::sweep::conduct(&crate::sweep::Session {
        tuning: Tuning::SHIPPED,
        modules: crate::modules::ModuleSet::ALL,
        seed: Some(SHOT_SEED),
        directives: &script,
        photos: &photos,
        probe_ticks: &[],
        viewport: crate::verify::HEADLESS_VIEWPORT,
        max_ticks: 6_000,
        stop_at_rest: false,
        stop_at_minute: Some(AFTERMATH_MINUTE + 20),
        resume_after: Some((Key::Digit3, 20)),
    })
}

/// **The four pictures are of what they say they are**, and each passes the
/// chrome and the frame floors.
pub fn judge_shots(checks: &mut crate::checks::Checks, run: &crate::sweep::Conducted) {
    let crypt_seal = JobId { site: 2, slot: 1 };
    let missing = |checks: &mut crate::checks::Checks, name: &str| {
        checks.require(
            false,
            "a resolution photograph was never taken",
            format!("the {name} photo is missing from the resolution run"),
        );
    };
    match run.photo("failed") {
        Some(shot) => {
            let lens = crate::lens::Lens::on(&shot.sim);
            let reason = crate::attention::reason_line(&lens).unwrap_or_default();
            checks.require(
                shot.clock.paused && reason.contains("task-failed") && reason.contains("botched"),
                "the failure photograph does not show a world stopped by a posted job going \
                 wrong",
                format!(
                    "the clock reads paused={} and the banner says {reason:?}",
                    shot.clock.paused
                ),
            );
            crate::frames::judge_chrome(checks, run, shot, "a posted job failed, mid-pause");
            crate::floors::judge_frame_floor(
                checks,
                run.font,
                &shot.frame,
                "a posted job failed, mid-pause",
            );
        }
        None => missing(checks, "failed"),
    }
    match run.photo("reopened") {
        Some(shot) => {
            let lens = crate::lens::Lens::on(&shot.sim);
            let open = shot
                .sim
                .sites
                .get(crypt_seal.site)
                .is_some_and(|site| site.is_open(crypt_seal.slot));
            let panel = crate::board::site_board(
                &shot.flow,
                &lens,
                &crate::grid::grid(),
                &Tuning::SHIPPED,
                shot.clock.minutes,
                crypt_seal.site,
            );
            let at = crate::layout::board_row(crypt_seal.slot).min + crate::layout::job::FIT;
            let cell = panel
                .runs
                .iter()
                .find(|run| run.at == at)
                .map(|run| run.text.clone());
            let want = crate::resolution::cell_for(&shot.sim, &Tuning::SHIPPED, 1, crypt_seal);
            checks.require(
                shot.flow.board == Some(crypt_seal.site)
                    && shot.flow.selected == Some(1)
                    && open
                    && cell.as_deref() == Some(want.as_str())
                    && want.ends_with("risky"),
                "the reopened photograph is not the failed job back on its board",
                format!(
                    "the board reads {:?} with {:?} selected, the second seal is open={open} and \
                     its fit cell reads {cell:?} against the sim's {want:?}",
                    shot.flow.board, shot.flow.selected
                ),
            );
            crate::frames::judge_chrome(checks, run, shot, "the failed job back on its board");
            crate::floors::judge_frame_floor(
                checks,
                run.font,
                &shot.frame,
                "the failed job back on its board",
            );
        }
        None => missing(checks, "reopened"),
    }
    match run.photo("wentwell") {
        Some(shot) => {
            let lens = crate::lens::Lens::on(&shot.sim);
            let feed = crate::attention::feed(&lens, shot.flow.show_ignored, 10);
            let shown = feed.iter().any(|entry| {
                shot.sim
                    .events
                    .get(entry.index)
                    .and_then(|event| event.tier)
                    == Some(Tier::WentWell)
            });
            checks.require(
                shot.flow.showing(crate::flow::Drawer::Feed) && shown,
                "the went-well photograph has no job that went well in its feed",
                format!(
                    "the feed is open={} and carries a went-well line={shown}",
                    shot.flow.showing(crate::flow::Drawer::Feed)
                ),
            );
            crate::frames::judge_chrome(checks, run, shot, "a job that went well, in the feed");
            crate::floors::judge_frame_floor(
                checks,
                run.font,
                &shot.frame,
                "a job that went well, in the feed",
            );
        }
        None => missing(checks, "wentwell"),
    }
    match run.photo("aftermath") {
        Some(shot) => {
            let failed = shot
                .sim
                .resolved
                .iter()
                .filter(|record| record.tier == Tier::Failed)
                .count();
            checks.require(
                failed >= 3 && shot.flow.works,
                "the aftermath photograph is not the camp after a string of failures",
                format!(
                    "{failed} jobs had failed by minute {AFTERMATH_MINUTE} and the settlement \
                     panel is open={}; the picture is of the camp a day after three or more",
                    shot.flow.works
                ),
            );
            crate::frames::judge_chrome(checks, run, shot, "the camp after a string of failures");
            crate::floors::judge_frame_floor(
                checks,
                run.font,
                &shot.frame,
                "the camp after a string of failures",
            );
        }
        None => missing(checks, "aftermath"),
    }
}
