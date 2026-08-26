//! The claim the tuning drawer exists to keep: **an APPLY restarts the beat,
//! and what follows is a fresh run at the new constants.**
//!
//! DESIGN §8a settles the determinism interaction by saying that replay state
//! is a pure function of (beat state, assignments, constants), so a constant
//! changed mid-run would make the recording a lie, and v1's resolution is to
//! apply at a beat boundary. That resolution is only worth anything if it is
//! measured, and this is the measurement: a scripted session opens the drawer,
//! loads a preset, applies it, plays the beat out, and the result is compared
//! **byte for byte** against a run that started at those constants in the first
//! place. If they ever differ, an apply is carrying something across the
//! boundary that a stamp does not describe.
//!
//! The same run is the one the drawer's own floors and its capture come from:
//! there is one scripted drawer session, and everything about the drawer is
//! read off it.
//!
//! The contracts a `?constants=` link is refused by are `links.rs`: a different
//! subject that shares only the compact form.

use jidousha::prelude::*;
use jidousha::testing::{BackendTextureId, FrameRecord, FrameRecorder, InputScript};

use crate::beats::CHAIN;
use crate::checks::{Checks, fail};
use crate::constants::{Field, Tuning};
use crate::flow::{Flow, Preview, Stage, StartAt};
use crate::model::Social;
use crate::presets;
use crate::{layout, scaling, sprites, verify};

/// Which beat the drawer session is played on, and which preset it applies.
///
/// Beat 2 is the one with a killing in it, so a preset that moves `K_kill` and
/// `desperation_fall` changes the outcome rather than only the numbers on the
/// sheets — which is what makes the comparison below an instrument instead of a
/// tautology. `replay_identity`'s last check holds that reason to account: it
/// fails if this beat resolves the same at the preset and at the shipped set.
pub const BEAT: usize = 1;
/// The preset the session loads: cheap lives, so beat 2 resolves differently.
pub const PRESET: &str = "CUTTHROAT";

/// What the scripted drawer session saw, at the four moments worth a look.
pub struct DrawerRun {
    /// The set the session started at.
    pub started_at: Tuning,
    /// The set it applied.
    pub applied: Tuning,
    /// The flow with the drawer open and nothing pending.
    pub opened: Flow,
    /// The active constants at that moment — which must still be the start set.
    pub opened_active: Tuning,
    /// The flow with a preset loaded and **not yet applied**.
    pub pending: Flow,
    /// The active constants at *that* moment — still the start set, because
    /// nothing changes mid-beat.
    pub pending_active: Tuning,
    /// The roster while the preset was pending, to show the beat did not move.
    pub pending_social: Social,
    /// The preview at that moment, for the drawer's floors.
    pub pending_preview: Preview,
    /// The frame the drawer was photographed on.
    pub pending_frame: Option<FrameRecord>,
    /// Which backend texture the font landed on, for the floors.
    pub font: BackendTextureId,
    /// The flow just after APPLY: the beat has restarted.
    pub applied_flow: Flow,
    /// The active constants after it — the preset.
    pub applied_active: Tuning,
    /// The roster after it: the beat's authored state again.
    pub applied_social: Social,
    /// The party the restart left behind, by name.
    pub applied_party: Vec<&'static str>,
    /// The social state the beat left when it was played out afterwards.
    pub after: Social,
    /// The narration it produced.
    pub report: Vec<String>,
}

/// A scripted click, in the same three ticks `verify.rs` spends on one.
fn click(script: InputScript, tick: &mut u64, at: Vec2, viewport: PhysicalSize) -> InputScript {
    let screen = verify::headless_camera(viewport).world_to_screen(at);
    let next = script
        .pointer_at(*tick, screen)
        .click(PointerButton::Primary, *tick + 1);
    *tick += 3;
    next
}

/// Play the drawer: open it, load a preset, apply it, then play the beat out.
pub fn drawer_run(record: bool) -> DrawerRun {
    let viewport = verify::HEADLESS_VIEWPORT;
    let Some(spec) = CHAIN.get(BEAT) else {
        fail(
            "the drawer session names a beat the chain does not have",
            &format!("beat {BEAT} of {}", CHAIN.len()),
        );
    };
    let Some(preset) = presets::find(PRESET) else {
        fail(
            "the drawer session names a preset the table does not have",
            &format!("{PRESET:?} is not in presets::PRESETS"),
        );
    };
    let Some(row) = Field::ALL
        .iter()
        .position(|field| preset.tuning.field(*field) != Tuning::SHIPPED.field(*field))
    else {
        fail(
            "the drawer session's preset is the shipped set",
            "a preset that changes nothing cannot show that an apply changes anything",
        );
    };

    let mut script = InputScript::new();
    let mut tick = 3;
    // Open the drawer.
    script = click(script, &mut tick, layout::tune_button().center(), viewport);
    let opened_at = tick;
    tick += 1;
    // Point at a row, so the hint line has a tenant, then step it once by hand:
    // the steppers and the presets are two ways in and both are exercised.
    script = script.pointer_at(
        tick,
        verify::headless_camera(viewport).world_to_screen(layout::tuner_row(row).center()),
    );
    tick += 1;
    script = click(
        script,
        &mut tick,
        layout::tuner_plus(row).center(),
        viewport,
    );
    // Then a preset, which replaces the pending set outright.
    let preset_index = presets::PRESETS
        .iter()
        .position(|entry| entry.name == PRESET)
        .unwrap_or(0);
    script = click(
        script,
        &mut tick,
        layout::tuner_preset(preset_index).center(),
        viewport,
    );
    // Hold the pointer on the row again so the drawer's hint row is populated
    // on the frame that is photographed.
    script = script.pointer_at(
        tick,
        verify::headless_camera(viewport).world_to_screen(layout::tuner_row(row).center()),
    );
    let pending_at = tick;
    tick += 2;
    script = click(script, &mut tick, layout::tuner_apply().center(), viewport);
    let applied_at = tick;
    tick += 1;
    // Close it, and play the beat out exactly as `verify.rs` plays it: take the
    // quest, stage the intended party, send.
    script = click(script, &mut tick, layout::tune_button().center(), viewport);
    script = click(script, &mut tick, layout::quest_card(0).center(), viewport);
    for name in spec.send {
        if let Some(index) = spec.index_of(name) {
            script = click(
                script,
                &mut tick,
                layout::party_card(index).center(),
                viewport,
            );
        }
    }
    script = click(script, &mut tick, layout::send_button().center(), viewport);
    let report_at = tick;

    let mut sim = headless(crate::config(), crate::register);
    sim.world_mut().insert_resource(Tuning::SHIPPED);
    sim.world_mut().insert_resource(StartAt(BEAT));
    sim.world_mut().insert_resource(scaling::Surface(viewport));
    let mut recorder = record.then(|| FrameRecorder::new(viewport));
    let mut run = DrawerRun {
        started_at: Tuning::SHIPPED,
        applied: preset.tuning,
        opened: Flow::default(),
        opened_active: Tuning::SHIPPED,
        pending: Flow::default(),
        pending_active: Tuning::SHIPPED,
        pending_social: Social::default(),
        pending_preview: Preview::default(),
        pending_frame: None,
        font: recorder
            .as_ref()
            .map_or(BackendTextureId(0), FrameRecorder::font_texture),
        applied_flow: Flow::default(),
        applied_active: Tuning::SHIPPED,
        applied_social: Social::default(),
        applied_party: Vec::new(),
        after: Social::default(),
        report: Vec::new(),
    };

    for tick in 1..=report_at + 1 {
        sim.world_mut()
            .insert_resource(Input::new(script.snapshot_at(tick)));
        sim.tick();
        if tick == 1
            && let Some(recorder) = recorder.as_mut()
        {
            let assets = sim.world_mut().resource_mut::<Assets>();
            if let Some(failure) = sprites::settle(assets).first() {
                fail(
                    "giri's art did not load for the drawer session",
                    &crate::checks::one_line(&failure.message()),
                );
            }
            let _ = recorder;
        }
        if let Some(recorder) = recorder.as_mut() {
            recorder.settle_assets(&mut sim, tick);
        }
        let frame = recorder.as_mut().map(|recorder| recorder.draw(&mut sim));
        let flow = sim.world().resource::<Flow>().clone();
        let active = *sim.world().resource::<Tuning>();
        if tick == opened_at {
            run.opened = flow.clone();
            run.opened_active = active;
        }
        if tick == pending_at {
            run.pending = flow.clone();
            run.pending_active = active;
            run.pending_social = Social::read(&sim.world().view());
            run.pending_preview = sim.world().resource::<Preview>().clone();
            run.pending_frame = frame;
        }
        if tick == applied_at {
            run.applied_flow = flow.clone();
            run.applied_active = active;
            let social = Social::read(&sim.world().view());
            run.applied_party = flow
                .party
                .iter()
                .map(|entity| social.name(*entity))
                .collect();
            run.applied_social = social;
        }
        if tick == report_at {
            run.after = Social::read(&sim.world().view());
            run.report = flow.report.clone();
        }
    }
    run
}

/// Everything the drawer session claims, asserted.
pub fn judge(checks: &mut Checks, run: &DrawerRun) {
    // --- the handle opens it, and opening it changes nothing else ----------
    checks.require(
        run.opened.tuner.open,
        "the TUNE handle did not open the tuning drawer",
        format!(
            "after clicking it the drawer was {:?}",
            run.opened.tuner.open
        ),
    );
    checks.require(
        run.opened_active == run.started_at,
        "opening the tuning drawer changed the constants in effect",
        format!(
            "the drawer opened and the active set became {}",
            run.opened_active.stamp()
        ),
    );

    // --- a pending set is pending: nothing about the beat has moved --------
    checks.require(
        run.pending.tuner.pending == run.applied,
        "clicking a preset did not load it into the pending set",
        format!(
            "the pending set is {} and {PRESET} is {}",
            run.pending.tuner.pending.stamp(),
            run.applied.stamp()
        ),
    );
    checks.require(
        run.pending_active == run.started_at,
        "a pending constant reached the simulation before APPLY",
        format!(
            "a preset was loaded and the active set became {}; the pending copy is UI state \
             and the active set is simulation state (DESIGN §8a)",
            run.pending_active.stamp()
        ),
    );
    let authored = CHAIN.get(BEAT).map(|spec| spec.roster.len()).unwrap_or(0);
    checks.require(
        run.pending_social.members.len() == authored
            && run.pending_social.members.iter().all(|member| member.alive),
        "loading a preset disturbed the beat it was loaded during",
        format!(
            "the roster is {} of {authored} characters",
            run.pending_social.members.len()
        ),
    );

    // --- APPLY: the swap and the restart, which are one action -------------
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
        run.applied_flow.beat == BEAT
            && run.applied_flow.stage == Stage::Board
            && run.applied_party.is_empty()
            && run.applied_flow.taken.is_none(),
        "APPLY did not restart the current beat",
        format!(
            "after APPLY the game is on beat {} at {:?} with the party {:?} and quest {:?}; \
             DESIGN §8a applies constants at a beat boundary by restarting the beat",
            run.applied_flow.beat + 1,
            run.applied_flow.stage,
            run.applied_party,
            run.applied_flow.taken
        ),
    );
    let restored = CHAIN.get(BEAT).is_some_and(|spec| {
        spec.roster.iter().all(|character| {
            run.applied_social
                .members
                .iter()
                .any(|member| member.name == character.name && member.alive)
        })
    });
    checks.require(
        restored,
        "APPLY restarted the beat without restoring its authored roster",
        format!(
            "the roster after the restart is {:?}",
            run.applied_social
                .members
                .iter()
                .map(|member| member.name)
                .collect::<Vec<_>>()
        ),
    );

    // --- the stamp, which is what makes the restart a record ---------------
    let log = &run.applied_flow.log;
    checks.require(
        log.iter().any(|line| line.contains(&run.applied.stamp())),
        "the applied constants did not reach the log with their stamp",
        format!(
            "the log after the apply is {log:?} and the stamp is {:?}; a restart nothing \
             recorded the constants of is a recording that cannot be reproduced (DESIGN §8a)",
            run.applied.stamp()
        ),
    );
    // And the sentence on top of it, which is what the toast says too: the
    // stamp is the record and this is what a person reads.
    checks.require(
        log.first()
            .is_some_and(|line| line.contains("this beat restarts")),
        "the applied constants were logged without saying the beat restarted",
        format!("the newest log line is {:?}", log.first()),
    );
    checks.require(
        run.applied_flow.toast.is_some(),
        "APPLY raised no toast, so a restart happens with nothing said",
        format!("the toast after APPLY was {:?}", run.applied_flow.toast),
    );

    // --- and the claim itself ----------------------------------------------
    replay_identity(checks, run);
}

/// **The claim**: after an apply, the rest of the beat is a fresh run at the new
/// constants — the same world state and the same narration, exactly.
///
/// Asserted against a run that never opened a drawer at all, and asserted to
/// *differ* from a run at the shipped set, because a comparison that would pass
/// with the preset ignored is not a comparison.
fn replay_identity(checks: &mut Checks, run: &DrawerRun) {
    let fresh = verify::play(BEAT, run.applied, false);
    let shipped = verify::play(BEAT, run.started_at, false);
    checks.require(
        run.report == fresh.report,
        "a beat played after an APPLY is not the beat played at those constants",
        format!(
            "after applying {PRESET} the narration was {:?}; a run started at {PRESET} \
             narrates {:?}. A recording is a pure function of (beat state, assignments, \
             constants) and an apply starts a new one at the restart (DESIGN §8a)",
            run.report, fresh.report
        ),
    );
    checks.require(
        sheet(&run.after) == sheet(&fresh.after),
        "a beat played after an APPLY left different state than the same beat played fresh",
        format!(
            "after the apply the roster is {:?} and a fresh run at {PRESET} leaves {:?}",
            sheet(&run.after),
            sheet(&fresh.after)
        ),
    );
    checks.require(
        sheet(&fresh.after) != sheet(&shipped.after) || fresh.report != shipped.report,
        "the drawer session applies a preset that changes nothing about this beat",
        format!(
            "beat {} resolves the same at {PRESET} and at the shipped set, so the comparison \
             above would pass with the apply ignored; pick a beat or a preset that moves the \
             outcome",
            BEAT + 1
        ),
    );
}

/// A roster as its sheets read, for comparing two runs.
fn sheet(social: &Social) -> Vec<(&'static str, bool, i32, i32, i32, String)> {
    social
        .members
        .iter()
        .map(|member| {
            (
                member.name,
                member.alive,
                member.desperation,
                member.wealth,
                member.clean_jobs,
                crate::party::marks_line(member),
            )
        })
        .collect()
}
