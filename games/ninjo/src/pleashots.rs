//! **The petitions, photographed** — wave 1.5's own conducted session, and
//! what each of its pictures is asserted to be.
//!
//! One session at seed [`SHOT_SEED`], at 4x, played the way the handoff asks
//! the playtest to be played: a petition voices and stops the world, the
//! player reads it and puts it in the ledger (LATER), and gives the
//! petitioner the gold it asks for (GIVE) — with the petitioner's own panel
//! photographed before and after; a second voicing is arranged (ARRANGE,
//! which opens the person's work list and posts nothing); a third is ignored
//! to its cliff, and the feed is open when `broke` fires. Every other stop the
//! petitions make is resumed by the player's own key (`resume_after`).
//!
//! The minutes and the rows are this seed's, written down: the run is
//! deterministic, and a content change that moves them is a change somebody
//! should see here first.

use jidousha::prelude::*;

use crate::attention::EventClass;
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::layout;
use crate::pleas::{Credit, Status};
use crate::sweep::{Act, Conducted, Directive, Photo, Session, When, conduct};

/// The seed the session plays at: one where two `collectors-visit` petitions
/// voice early, so one can be given to and one ignored to its cliff.
pub const SHOT_SEED: u64 = 10;
/// When Bob's `collectors-visit` is voiced at this seed — the first petition.
pub const BOB_VOICED: u64 = 1548;
/// When Steve's `thin-days` is voiced.
pub const STEVE_VOICED: u64 = 4344;
/// When the ledger is opened for its picture: Steve's has just failed and he
/// has walked out, Bob's is met, and the rest are still running — before the
/// next voicing pushes the met row off the list's ten.
pub const LEDGER_MINUTE: u64 = 10_112;
/// When Ludo's `collectors-visit` falls — and `broke` fires.
pub const LUDO_CLIFF: u64 = 11_328;
/// Bob's roster index, and his row on the roster at this hour.
const BOB: usize = 0;
/// Steve's.
const STEVE: usize = 1;
/// Ludo's.
const LUDO: usize = 7;

/// **The session.**
pub fn shot_run() -> Conducted {
    let click = |when: When, at: Vec2| Directive {
        when,
        what: Act::ClickUi(at),
    };
    let now = |at: Vec2| click(When::Now, at);
    let tap = |when: When, key: Key| Directive {
        when,
        what: Act::Tap(key),
    };
    let held = |minute: u64| When::MinuteHeld { minute, after: 8 };
    let script = vec![
        tap(When::Tick(5), Key::Digit3),
        // --- Bob asks; the player reads it, looks at him, and gives -------
        click(
            held(BOB_VOICED),
            layout::card_act(layout::voicing_card()).center(),
        ), // 2: LATER
        now(layout::roster_button().center()),
        now(layout::roster_open(BOB).center()), // 4: Bob's panel - "before"
        now(layout::pleas_button().center()),
        now(layout::plea_row(0).center()),
        now(layout::card_give(layout::plea_card()).center()), // 7: GIVE
        now(layout::roster_button().center()),
        now(layout::roster_open(BOB).center()), // 9: Bob's panel - "after"
        tap(When::Now, Key::Digit3),
        // --- Steve asks; the player arranges --------------------------------
        click(
            held(STEVE_VOICED),
            layout::card_act(layout::voicing_card()).center(),
        ),
        now(layout::card_act(layout::plea_card()).center()), // 12: ARRANGE
        tap(When::Now, Key::Digit3),
        // --- the ledger, late: met, failed and still asking -----------------
        click(When::Minute(LEDGER_MINUTE), layout::pleas_button().center()), // 14
        click(
            When::Minute(LEDGER_MINUTE + 24),
            layout::pleas_button().center(),
        ),
        // --- and the feed open when Ludo's cliff falls ----------------------
        click(
            When::Minute(LUDO_CLIFF - 60),
            layout::feed_button().center(),
        ),
    ];
    let photo = |name: &'static str, minute: u64, paused: bool, step: usize| Photo {
        name,
        minute,
        tick: 0,
        paused,
        step,
    };
    let photos = [
        photo("voicing", BOB_VOICED, true, 0),
        photo("face-before", 0, false, 4),
        photo("face-after", 0, false, 9),
        photo("arranged", 0, false, 12),
        photo("pleas", LEDGER_MINUTE + 8, false, 14),
        photo("broke", LUDO_CLIFF, true, 16),
    ];
    conduct(&Session {
        tuning: Tuning::SHIPPED,
        modules: crate::modules::ModuleSet::ALL,
        seed: Some(SHOT_SEED),
        directives: &script,
        photos: &photos,
        probe_ticks: &[],
        viewport: crate::verify::HEADLESS_VIEWPORT,
        max_ticks: 40_000,
        stop_at_rest: false,
        stop_at_minute: Some(LUDO_CLIFF + 60),
        resume_after: Some((Key::Digit3, 20)),
    })
}

/// **Each picture is what it says** — asserted on the state it was taken in,
/// so a photograph cannot pass by being a picture of something else.
pub fn judge_shots(checks: &mut Checks, run: &Conducted) -> String {
    let mut said = Vec::new();
    // **Every row the screen meant is on the frame, the frame keeps the
    // floors, and every figure is one person** — the three frame judges every
    // photographed surface owes (UI.md §6), over all six.
    for (name, what) in [
        ("voicing", "a petition voiced, mid-pause"),
        ("face-before", "Bob's panel before his petition was met"),
        ("face-after", "Bob's panel after his petition was met"),
        ("arranged", "where ARRANGE went"),
        ("pleas", "the petition ledger, mixed"),
        ("broke", "the feed on a broke"),
    ] {
        if let Some(shot) = run.photo(name) {
            crate::frames::judge_chrome(checks, run, shot, what);
            crate::floors::judge_frame_floor(checks, run.font, &shot.frame, what);
            crate::floors::judge_figures(checks, run, shot, what);
        }
    }
    // --- the voicing, mid-pause ----------------------------------------------
    match run.photo("voicing") {
        Some(shot) => {
            let lens = crate::lens::Lens::on(&shot.sim);
            let id = crate::card::voicing(&lens);
            let petition = id.and_then(|id| lens.petition(id));
            checks.require(
                shot.clock.paused
                    && petition
                        .is_some_and(|p| p.who == BOB && p.template.id == "collectors-visit"),
                "the voicing picture is not Bob's petition stopping the world",
                format!(
                    "the frame was taken paused: {} over {:?}",
                    shot.clock.paused,
                    petition.map(|p| (p.who, p.template.id))
                ),
            );
            said.push("voicing");
        }
        None => checks.require(false, "the voicing picture was never taken", String::new()),
    }
    // --- the face, before and after the gift ---------------------------------
    match (run.photo("face-before"), run.photo("face-after")) {
        (Some(before), Some(after)) => {
            let (a, b) = (&before.sim.people[BOB], &after.sim.people[BOB]);
            let met = after
                .sim
                .petitions
                .all()
                .iter()
                .find(|p| p.who == BOB)
                .map(|p| p.status);
            checks.require(
                before.flow.selected == Some(BOB)
                    && after.flow.selected == Some(BOB)
                    && b.desperation == a.desperation - 1
                    && b.wallet == a.wallet + 30
                    && b.source != a.source
                    && b.source.contains("paid the collector off")
                    && matches!(
                        met,
                        Some(Status::Met {
                            credit: Credit::Player,
                            ..
                        })
                    ),
                "the face pictures are not Bob before and after his petition was met",
                format!(
                    "selected {:?} then {:?}; desperation {} -> {}, purse {} -> {}, source {:?} -> \
                     {:?}; the petition ended {met:?}",
                    before.flow.selected,
                    after.flow.selected,
                    a.desperation,
                    b.desperation,
                    a.wallet,
                    b.wallet,
                    a.source,
                    b.source
                ),
            );
            said.push("face before and after");
        }
        _ => checks.require(false, "the face pictures were never taken", String::new()),
    }
    // --- ARRANGE navigated, and posted nothing -------------------------------
    match run.photo("arranged") {
        Some(shot) => {
            checks.require(
                shot.flow.listing == Some(STEVE)
                    && shot.flow.selected == Some(STEVE)
                    && shot.flow.drawer.is_none()
                    && shot.sim.postings.all().is_empty(),
                "ARRANGE did not open Steve's work list, or it posted something",
                format!(
                    "the list is {:?}, the selection {:?}, the drawer {:?} and the ledger holds {} \
                     postings",
                    shot.flow.listing,
                    shot.flow.selected,
                    shot.flow.drawer,
                    shot.sim.postings.all().len()
                ),
            );
            said.push("arranged");
        }
        None => checks.require(false, "the arranged picture was never taken", String::new()),
    }
    // --- the ledger, mixed ---------------------------------------------------
    match run.photo("pleas") {
        Some(shot) => {
            let lens = crate::lens::Lens::on(&shot.sim);
            let order = crate::card::ledger_order(&lens);
            let shown: Vec<Status> = order
                .iter()
                .take(layout::PLEA_ROWS)
                .filter_map(|id| lens.petition(*id).map(|p| p.status))
                .collect();
            let (active, met, failed) = (
                shown.iter().filter(|s| **s == Status::Active).count(),
                shown
                    .iter()
                    .filter(|s| matches!(s, Status::Met { .. }))
                    .count(),
                shown
                    .iter()
                    .filter(|s| matches!(s, Status::Failed { .. }))
                    .count(),
            );
            checks.require(
                shot.flow.showing(crate::flow::Drawer::Pleas) && active > 0 && met > 0 && failed > 0,
                "the ledger picture is not a mix of asking, met and failed",
                format!(
                    "the drawer is {:?} and its rows are {active} asking, {met} met, {failed} failed",
                    shot.flow.drawer
                ),
            );
            said.push("ledger");
        }
        None => checks.require(false, "the ledger picture was never taken", String::new()),
    }
    // --- the feed on a broke firing ------------------------------------------
    match run.photo("broke") {
        Some(shot) => {
            let pause = shot.sim.paused_by;
            let event = pause.and_then(|pause| shot.sim.events.get(pause.event));
            let petition = event
                .and_then(|event| event.petition)
                .and_then(|id| shot.sim.petitions.get(id));
            checks.require(
                shot.flow.showing(crate::flow::Drawer::Feed)
                    && event
                        .is_some_and(|e| e.class == EventClass::PetitionFailed && e.party == LUDO)
                    && petition.is_some_and(|p| p.declared().kind.id == "broke")
                    && shot.sim.people[LUDO].wallet == 0,
                "the broke picture is not the feed open on Ludo's broke",
                format!(
                    "the drawer is {:?}, the pause names {:?}, Ludo holds {}g",
                    shot.flow.drawer,
                    event.map(|e| (e.class.name(), e.party, e.note.clone())),
                    shot.sim.people[LUDO].wallet
                ),
            );
            said.push("broke");
        }
        None => checks.require(false, "the broke picture was never taken", String::new()),
    }
    let timeline = run
        .sim
        .petitions
        .all()
        .iter()
        .map(|petition| {
            format!(
                "{} {} {}{}",
                run.sim.people[petition.who].name,
                petition.template.id,
                petition
                    .voiced_at
                    .map_or_else(|| "-".to_owned(), |at| at.to_string()),
                match petition.status {
                    Status::Met { at, .. } => format!(" met {at}"),
                    Status::Failed { at } => format!(" failed {at}"),
                    _ => String::new(),
                }
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!(
        "petitions photographed at seed {SHOT_SEED}: {} ({} events, {} pauses) - {timeline}",
        said.join(", "),
        run.events.len(),
        run.sim.pauses
    )
}
