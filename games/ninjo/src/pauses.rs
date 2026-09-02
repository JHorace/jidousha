//! **Auto-pause is a simulation transition**, and this is the battery that
//! says so (GDD §3, wave 0a; DESIGN §6, §7).
//!
//! Four claims, each run as a conducted session driven through the real UI —
//! the config is set by clicking radios in the config drawer, exactly as a
//! player sets it, so what is under test is the played path and not a field
//! poked from a check:
//!
//! 1. **The transition.** A pause-class event fires and the clock is at speed
//!    0 in the same tick, at the event's own world-minute, with the reason
//!    naming the entry that caused it.
//! 2. **The replay.** The same recorded inputs produce the same pauses at the
//!    same world-minutes, twice.
//! 3. **The config is what decides.** The same script *without* the config
//!    change never stops, and runs the authored timeline to the end. The one
//!    difference between the two runs is a recorded click.
//! 4. **The invariance.** The whole speed sweep again, under a config that
//!    stops the world once per completion, with a resume after each — and the
//!    transcripts are identical to the sweep's own, to the world-minute.
//!    A pause stretches wall time and moves no world-time address.

use jidousha::prelude::*;

use crate::attention::{EventClass, Mode};
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::sweep::{Act, Conducted, Directive, Session, When, conduct, transcript};
use crate::{layout, sweep};

/// The class these sessions ask the world to stop for.
///
/// A completion, because it is the only class this build has that a player
/// might plausibly want to be stopped by — and because the class table opens
/// it on `log`, so the pause is unambiguously the config's doing.
const STOPS_FOR: EventClass = EventClass::QuestComplete;

/// The clicks that set one class's mode, through the config drawer.
///
/// Three of them: the handle, the radio, and a click that shuts the drawer
/// again. Tick-addressed, so they all land before the clock is started and the
/// world runs its whole life under one config.
fn set_mode(class: EventClass, mode: Mode) -> Vec<Directive> {
    let slot = Mode::ALL
        .iter()
        .position(|entry| *entry == mode)
        .unwrap_or(0);
    [
        layout::modes_button().center(),
        layout::modes_radio(class.index(), slot).center(),
        layout::modes_button().center(),
    ]
    .into_iter()
    .enumerate()
    .map(|(step, at)| Directive {
        when: When::Tick(12 + step as u64 * 4),
        what: Act::ClickUi(at),
    })
    .collect()
}

/// One dispatch, and nothing to resume it: the world runs until it stops
/// itself, and then holds.
fn stopping_script() -> Vec<Directive> {
    let mut script = set_mode(STOPS_FOR, Mode::PauseAndFocus);
    // **At 1x**, because the claim below is that the clock stops *at the
    // event's own world-minute* — and at 4x a tick carries the clock a minute
    // and a half past it, which is the coarse-clock behaviour the invariance
    // sweep is for and not this one.
    script.push(Directive {
        when: When::Tick(28),
        what: Act::Tap(Key::Digit1),
    });
    script.extend(sweep::order(12, 1, 1)); // Steve to the Deep Cave
    script
}

/// The same run with no config click in it.
fn running_script() -> Vec<Directive> {
    vec![
        Directive {
            when: When::Tick(28),
            what: Act::Tap(Key::Digit1),
        },
        sweep::order(12, 1, 1)[0],
        sweep::order(12, 1, 1)[1],
    ]
}

/// Everything above, run.
pub fn judge(checks: &mut Checks) -> String {
    let tuning = Tuning::SHIPPED;
    // The Deep Cave haul resolves at minute 161 when Steve is the only person
    // the player sends anywhere — his own doorstep out, the site's work, and
    // nobody else on the board (a shipped literal, so the terrain and duration
    // constants that make it up are all measured by it). A run that stops for
    // it never comes to rest, so it is capped rather than stopped at rest.
    let stopped = at_cap(&tuning, &stopping_script());
    let again = at_cap(&tuning, &stopping_script());
    let never = conduct(&Session::plain(tuning, &running_script(), 25_000));

    // --- 1: the transition -------------------------------------------------
    let pause = stopped.sim.paused_by;
    let triggering = pause.and_then(|pause| stopped.events.get(pause.event).cloned());
    checks.require(
        pause.is_some_and(|pause| pause.class == STOPS_FOR && pause.minute == 161),
        "a pause-class event did not stop the world at its own world-minute",
        format!(
            "the world holds at minute {} and the recorded reason is {:?}; the Deep Cave \
             haul completes at 161",
            stopped.minutes, pause
        ),
    );
    checks.require(
        stopped.minutes == 161,
        "the clock did not stop in the tick the pause-class event fired",
        format!(
            "the run was capped at {} ticks with the clock reading {} world-minutes; a pause \
             recorded at 161 and a clock past it means the transition trailed the event",
            stopped.ticks, stopped.minutes
        ),
    );
    checks.require(
        triggering
            .as_ref()
            .is_some_and(|event| event.class == STOPS_FOR && event.minute == stopped.minutes),
        "the pause names an entry that is not the event that fired",
        format!("the reason points at {triggering:?}"),
    );
    checks.require(
        stopped
            .events
            .last()
            .is_some_and(|event| event.class == STOPS_FOR),
        "the world kept going after the event it was supposed to stop for",
        format!(
            "the last event of the stopped run is {:?}",
            stopped.events.last().map(|event| event.class.name())
        ),
    );
    checks.require(
        stopped.sim.pauses == 1,
        "the world did not stop exactly once for one pause-class event",
        format!("it recorded {} pauses", stopped.sim.pauses),
    );

    // --- 2: the replay -----------------------------------------------------
    checks.require(
        transcript(&stopped.events) == transcript(&again.events)
            && stopped.sim.paused_by == again.sim.paused_by
            && stopped.minutes == again.minutes
            && stopped.ticks == again.ticks,
        "the same recorded inputs did not stop the world the same way twice",
        format!(
            "the first run holds at minute {} after {} ticks with {:?}; the second holds at \
             {} after {} ticks with {:?}",
            stopped.minutes,
            stopped.ticks,
            stopped.sim.paused_by,
            again.minutes,
            again.ticks,
            again.sim.paused_by
        ),
    );

    // --- 3: the config is the whole difference -----------------------------
    checks.require(
        never.sim.pauses == 0 && never.sim.paused_by.is_none(),
        "the same run without the config click stopped anyway",
        format!(
            "it recorded {} pauses and holds {:?}; the two scripts differ by three clicks in \
             the config drawer and nothing else",
            never.sim.pauses, never.sim.paused_by
        ),
    );
    checks.require(
        never
            .events
            .iter()
            .filter(|event| event.class == STOPS_FOR)
            .count()
            >= 1
            && never.events.len() > stopped.events.len(),
        "the un-configured run did not carry on past the completion the other stopped at",
        format!(
            "it produced {} events and the stopped run produced {}",
            never.events.len(),
            stopped.events.len()
        ),
    );
    // And the two agree about everything that happened before the pause: the
    // config changes when the player is interrupted, never what occurred.
    let shared = transcript(&stopped.events);
    checks.require(
        transcript(&never.events)[..shared.len()] == shared[..],
        "configuring an auto-pause changed what happened before it",
        format!(
            "the stopped run's transcript is {:?} and the running one opens {:?}",
            shared,
            &transcript(&never.events)[..shared.len().min(never.events.len())]
        ),
    );

    // --- 4: the invariance sweep, with auto-pauses in it -------------------
    let mut counts: Vec<(&'static str, u64, u64, u64)> = Vec::new();
    let mut baseline: Option<Vec<_>> = None;
    let plain: Vec<(&'static str, Conducted)> = sweep::speed_scripts()
        .into_iter()
        .map(|(name, script)| (name, conduct(&Session::plain(tuning, &script, 60_000))))
        .collect();
    for (name, script) in sweep::speed_scripts() {
        let mut full = set_mode(STOPS_FOR, Mode::PauseAndFocus);
        full.extend(script);
        let mut session = Session::plain(tuning, &full, 90_000);
        session.resume_after = Some((sweep::resume_key(name), 20));
        let conducted = conduct(&session);
        let this = transcript(&conducted.events);
        let wanted = baseline.get_or_insert_with(|| this.clone());
        checks.require(
            &this == wanted,
            "the event transcript depends on where the world stopped itself",
            format!(
                "under {name} with auto-pauses the transcript is {this:?} and the first \
                 script's is {wanted:?}; a pause holds the clock and moves no address"
            ),
        );
        sweep::judge_orders(checks, &conducted, &format!("{name} with auto-pauses"));
        // **A pause holds the future, not the present** (GDD §3): the first
        // pause-class event of a crossed span stops the world and the rest of
        // the span still fires. At 1x the clock visits every minute, so every
        // completion gets its own stop; at 4x a tick carries 1.6 minutes and
        // two completions a minute apart share one. The count is therefore a
        // fact about the speed schedule — which is exactly why the *addresses*
        // are what invariance is asserted over, and they are, above.
        let stops = conducted.sim.pauses;
        let completions = sweep::COMPLETIONS.len() as u64;
        let wanted = if name == "all-1x" {
            stops == completions
        } else {
            (1..=completions).contains(&stops)
        };
        checks.require(
            wanted,
            "an auto-pausing run did not stop once per completion",
            format!(
                "under {name} the world stopped {stops} times and this scenario completes \
                 {completions} quests inside its window; at 1x, where the clock visits every \
                 world-minute, the two numbers are equal"
            ),
        );
        let plain_ticks = plain
            .iter()
            .find(|(other, _)| *other == name)
            .map_or(0, |(_, run)| run.ticks);
        checks.require(
            conducted.ticks > plain_ticks,
            "stopping the world four times cost no wall time at all",
            format!(
                "under {name} the auto-pausing run took {} ticks and the plain one took \
                 {plain_ticks}; a pause is supposed to stretch wall time",
                conducted.ticks
            ),
        );
        counts.push((name, conducted.sim.pauses, conducted.ticks, plain_ticks));
    }
    // The claim stated the other way round, against the sweep's own runs: the
    // auto-pausing transcripts are the *same* transcripts, not merely
    // consistent with each other.
    if let (Some(paused), Some((_, plain_run))) = (baseline.as_ref(), plain.first()) {
        let plain_transcript = transcript(&plain_run.events);
        checks.require(
            *paused == plain_transcript,
            "auto-pausing changed the world's transcript",
            format!(
                "with pauses the transcript is {paused:?}; without them it is \
                 {plain_transcript:?}"
            ),
        );
    }

    format!(
        "stopped at minute {} on {}; replay identical; unconfigured run {} events and no \
         pause; sweep {}",
        stopped.minutes,
        STOPS_FOR.name(),
        never.events.len(),
        counts
            .iter()
            .map(|(name, pauses, ticks, plain)| format!("{name} {pauses}x ({ticks}/{plain} ticks)"))
            .collect::<Vec<_>>()
            .join(" ")
    )
}

/// A session that is expected never to come to rest — capped rather than
/// stopped, because a world that has stopped itself is not at rest and never
/// will be until somebody presses a key.
fn at_cap(tuning: &Tuning, script: &[Directive]) -> Conducted {
    let mut session = Session::plain(*tuning, script, 6_000);
    session.stop_at_rest = false;
    conduct(&session)
}
