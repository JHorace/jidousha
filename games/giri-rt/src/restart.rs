//! The claim the tuning drawer exists to keep: **an APPLY restarts the
//! scenario, and what follows is a fresh run at the new constants.**
//!
//! Constants are simulation inputs; giri applied them at a beat boundary and
//! the fork's boundary is the scenario. That resolution is only worth
//! anything if it is measured, and this is the measurement: a scripted
//! session opens the drawer, steps a row, loads a preset, applies it, runs
//! the world, and the result is compared against a run that started at those
//! constants in the first place. If they ever differ, an apply is carrying
//! something across the boundary that a stamp does not describe.
//!
//! The same session is the one the drawer's floors and its capture come
//! from: one scripted drawer session, everything about the drawer read off
//! it.

use jidousha::prelude::*;
use jidousha::testing::BackendTextureId;

use crate::checks::{Checks, fail};
use crate::clock::Clock;
use crate::constants::Tuning;
use crate::flow::Flow;
use crate::sim::Sim;
use crate::sweep::{Act, Directive, Photo, Session, Shot, When, conduct, order, transcript};
use crate::{layout, presets, verify};

/// The preset the session applies. `MIRE` moves the off-road costs, so the
/// Deep Cave dispatch below resolves at different world-times than the
/// shipped set's — which is what makes the comparison an instrument instead
/// of a tautology.
pub const PRESET: &str = "MIRE";

/// The scripted session's probe ticks.
const OPENED_AT: u64 = 8;
const PENDING_AT: u64 = 18;
const APPLIED_AT: u64 = 22;

/// What the scripted drawer session saw.
pub struct DrawerRun {
    /// The set the session started at.
    pub started_at: Tuning,
    /// The set it applied.
    pub applied: Tuning,
    /// The flow with the drawer open and nothing pending.
    pub opened_flow: Flow,
    /// The active constants at that moment — still the start set.
    pub opened_active: Tuning,
    /// The flow with the preset loaded and **not yet applied**.
    pub pending_flow: Flow,
    /// The active constants at *that* moment — still the start set.
    pub pending_active: Tuning,
    /// The sim at that moment, to show nothing moved.
    pub pending_sim: Sim,
    /// The clock at that moment.
    pub pending_clock: Clock,
    /// The flow just after APPLY: the scenario has restarted.
    pub applied_flow: Flow,
    /// The active constants after it — the preset.
    pub applied_active: Tuning,
    /// The sim after it: the opening state again.
    pub applied_sim: Sim,
    /// The clock after it: minute zero, holding.
    pub applied_clock: Clock,
    /// The whole post-apply transcript — the dispatch's events.
    pub events: Vec<crate::sim::Event>,
    /// The drawer's photograph, pending state on screen.
    pub shot: Option<Shot>,
    /// The font of the recorded run.
    pub font: BackendTextureId,
}

/// The order the post-apply half runs: OX to the Deep Cave at minute 6 — a
/// route through plains and forest, which is exactly what `MIRE` moves.
fn post_apply_order() -> [Directive; 2] {
    order(6, 0, 1)
}

/// Play the drawer: open it, step a row, load the preset, apply, close, run
/// the world at 1x, dispatch.
pub fn drawer_run() -> DrawerRun {
    let Some(preset) = presets::find(PRESET) else {
        fail(
            "the drawer session names a preset the table does not have",
            &format!("{PRESET:?} is not in presets::PRESETS"),
        );
    };
    let preset_index = presets::PRESETS
        .iter()
        .position(|entry| entry.name == PRESET)
        .unwrap_or(0);
    let mut script: Vec<Directive> = vec![
        // Open the drawer (t5 move, t6 press; probe at 8).
        Directive {
            when: When::Tick(5),
            what: Act::ClickUi(layout::tune_button().center()),
        },
        // Point at the first row, so the hint band has a tenant.
        Directive {
            when: When::Tick(9),
            what: Act::PointUi(layout::tuner_row(0).center()),
        },
        // Step it once by hand — the steppers and the presets are two ways in
        // and both are exercised.
        Directive {
            when: When::Tick(11),
            what: Act::ClickUi(layout::tuner_plus(0).center()),
        },
        // Then the preset, which replaces the pending set outright.
        Directive {
            when: When::Tick(15),
            what: Act::ClickUi(layout::tuner_preset(preset_index).center()),
        },
        // APPLY (t19 move, t20 press; probe at 22).
        Directive {
            when: When::Tick(19),
            what: Act::ClickUi(layout::tuner_apply().center()),
        },
        // Close the drawer, start the clock, and play.
        Directive {
            when: When::Tick(23),
            what: Act::ClickUi(layout::tune_button().center()),
        },
        Directive {
            when: When::Tick(27),
            what: Act::Tap(Key::Digit1),
        },
    ];
    script.extend(post_apply_order());
    let photos = [Photo {
        name: "tuning",
        minute: 0,
        tick: PENDING_AT,
    }];
    let probe_ticks = [OPENED_AT, PENDING_AT, APPLIED_AT];
    let conducted = conduct(&Session {
        tuning: Tuning::SHIPPED,
        seed: None,
        directives: &script,
        photos: &photos,
        probe_ticks: &probe_ticks,
        viewport: verify::HEADLESS_VIEWPORT,
        max_ticks: 60_000,
        stop_at_rest: true,
    });

    let take = |tick: u64| {
        conducted.probe(tick).cloned().unwrap_or_else(|| {
            fail(
                "a drawer probe was never taken",
                &format!("tick {tick} of the scripted drawer session"),
            )
        })
    };
    let (_, opened_flow, opened_active, _, _) = take(OPENED_AT);
    let (_, pending_flow, pending_active, pending_sim, pending_clock) = take(PENDING_AT);
    let (_, applied_flow, applied_active, applied_sim, applied_clock) = take(APPLIED_AT);
    let shot = conducted.photo("tuning").map(|shot| Shot {
        name: shot.name,
        frame: shot.frame.clone(),
        sim: shot.sim.clone(),
        clock: shot.clock,
        flow: shot.flow.clone(),
    });
    DrawerRun {
        started_at: Tuning::SHIPPED,
        applied: preset.tuning,
        opened_flow,
        opened_active,
        pending_flow,
        pending_active,
        pending_sim,
        pending_clock,
        applied_flow,
        applied_active,
        applied_sim,
        applied_clock,
        events: conducted.events,
        shot,
        font: conducted.font,
    }
}

/// Everything the drawer session claims, asserted.
pub fn judge(checks: &mut Checks, run: &DrawerRun) {
    // --- the handle opens it, and opening it changes nothing ----------------
    checks.require(
        run.opened_flow.tuner.open,
        "the TUNE handle did not open the tuning drawer",
        format!(
            "after clicking it the drawer was {:?}",
            run.opened_flow.tuner.open
        ),
    );
    checks.require(
        run.opened_active == run.started_at,
        "opening the tuning drawer changed the constants in effect",
        format!("the active set became {}", run.opened_active.stamp()),
    );

    // --- a pending set is pending: nothing about the world has moved --------
    checks.require(
        run.pending_flow.tuner.pending == run.applied,
        "clicking a preset did not load it into the pending set",
        format!(
            "the pending set is {} and {PRESET} is {}",
            run.pending_flow.tuner.pending.stamp(),
            run.applied.stamp()
        ),
    );
    checks.require(
        run.pending_active == run.started_at,
        "a pending constant reached the simulation before APPLY",
        format!(
            "a preset was loaded and the active set became {}; the pending copy is UI state",
            run.pending_active.stamp()
        ),
    );
    checks.require(
        run.pending_sim.events.is_empty() && run.pending_clock.minutes == 0,
        "loading a preset disturbed the scenario it was loaded during",
        format!(
            "{} events and minute {} while the world should be holding at zero",
            run.pending_sim.events.len(),
            run.pending_clock.minutes
        ),
    );

    // --- APPLY: the swap and the restart, which are one action --------------
    checks.require(
        run.applied_active == run.applied,
        "APPLY did not put the pending set into effect",
        format!(
            "the active set after APPLY is {} and the pending set was {}",
            run.applied_active.stamp(),
            run.applied.stamp()
        ),
    );
    checks.require(
        run.applied_clock.minutes == 0
            && run.applied_sim.at_rest()
            && run.applied_sim.treasury == 0
            && run.applied_sim.events.is_empty(),
        "APPLY did not restart the scenario",
        format!(
            "after APPLY the clock reads {} with {} events and {}g held; an apply restarts \
             the scenario so the whole run is a pure function of the stamped constants",
            run.applied_clock.minutes,
            run.applied_sim.events.len(),
            run.applied_sim.treasury
        ),
    );
    checks.require(
        run.applied_flow
            .log
            .iter()
            .any(|line| line.contains(&run.applied.stamp())),
        "the applied constants did not reach the log with their stamp",
        format!(
            "the log after the apply is {:?} and the stamp is {:?}",
            run.applied_flow.log,
            run.applied.stamp()
        ),
    );
    checks.require(
        run.applied_flow.toast.is_some(),
        "APPLY raised no toast, so a restart happens with nothing said",
        format!("the toast after APPLY was {:?}", run.applied_flow.toast),
    );

    // --- and the claim itself ------------------------------------------------
    replay_identity(checks, run);
}

/// **The claim**: after an apply, the run is a fresh run at the new
/// constants — the same transcript, exactly. Asserted against a run that
/// never opened a drawer, and asserted to *differ* from a run at the shipped
/// set, because a comparison that would pass with the preset ignored is not
/// a comparison.
fn replay_identity(checks: &mut Checks, run: &DrawerRun) {
    let mut fresh_script: Vec<Directive> = vec![Directive {
        when: When::Tick(2),
        what: Act::Tap(Key::Digit1),
    }];
    fresh_script.extend(post_apply_order());
    let fresh = conduct(&Session::plain(run.applied, &fresh_script, 60_000));
    let shipped = conduct(&Session::plain(run.started_at, &fresh_script, 60_000));
    checks.require(
        transcript(&run.events) == transcript(&fresh.events),
        "a scenario played after an APPLY is not the scenario played at those constants",
        format!(
            "after applying {PRESET} the transcript is {:?}; a run started at {PRESET} \
             produces {:?}",
            transcript(&run.events),
            transcript(&fresh.events)
        ),
    );
    checks.require(
        transcript(&fresh.events) != transcript(&shipped.events),
        "the drawer session applies a preset that changes nothing about this scenario",
        format!(
            "the Deep Cave dispatch resolves identically at {PRESET} and at the shipped set \
             ({:?}), so the comparison above would pass with the apply ignored",
            transcript(&fresh.events)
        ),
    );
    // The dispatch itself landed: five events, starting with the departure
    // at minute 6.
    checks.require(
        run.events.first().is_some_and(|event| {
            event.minute == 6 && event.class == crate::sim::EventClass::Departed
        }) && run.events.len() == 5,
        "the post-apply dispatch did not run its whole loop",
        format!("the post-apply transcript is {:?}", transcript(&run.events)),
    );
}
