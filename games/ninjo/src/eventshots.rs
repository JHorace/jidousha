//! **The injector, photographed** — wave 1.6's own conducted session, and
//! what each of its three pictures is asserted to be.
//!
//! One session in the pinned test scenario (`scenarios/pinned-collector.txt`)
//! at 1x: the opening minute with the feed open, so the notices band shows
//! the scenario's stamp; the pin firing at minute 90 and stopping the world
//! on its voicing, so the card carries the `event` chip; and the feed after
//! the player read it, with ignored classes shown, so the director's own
//! line sits beside the voicing it caused.
//!
//! The director is off in this file and the pin is the whole of what comes
//! in from outside, so every picture is of exactly one event.

use jidousha::prelude::*;

use crate::attention::EventClass;
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::layout;
use crate::petitions::Source;
use crate::sweep::{Act, Conducted, Directive, Photo, Session, When, conduct};

/// Bob's roster index — whom the file pins the collector for.
const BOB: usize = 0;

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
    let pin = crate::pressed::PIN_MINUTE;
    let script = vec![
        click(When::Tick(3), layout::feed_button().center()), // 1: the opening, feed open
        click(When::Tick(6), layout::feed_button().center()), // 2: shut it
        tap(When::Tick(8), Key::Digit1),                      // 3: run at 1x
        // --- the pin fires and stops the world; the player reads it ---------
        click(
            When::MinuteHeld {
                minute: pin,
                after: 8,
            },
            layout::card_act(layout::voicing_card()).center(),
        ), // 4: LATER
        now(layout::feed_button().center()), // 5: the feed
        now(layout::feed_ignored_toggle().center()), // 6: ignored shown
    ];
    let photo = |name: &'static str, minute: u64, paused: bool, step: usize| Photo {
        name,
        minute,
        tick: 0,
        paused,
        step,
    };
    let photos = [
        photo("opening", 0, false, 1),
        photo("event-card", pin, true, 3),
        photo("event-feed", 0, false, 6),
    ];
    conduct(&Session {
        tuning: Tuning::SHIPPED,
        modules: crate::modules::ModuleSet::ALL,
        seed: None,
        scenario: crate::pressed::pinned_scenario(),
        directives: &script,
        photos: &photos,
        probe_ticks: &[],
        viewport: crate::verify::HEADLESS_VIEWPORT,
        max_ticks: 4_000,
        stop_at_rest: false,
        stop_at_minute: Some(pin + 30),
        resume_after: None,
    })
}

/// The three pictures, by name, and what each is of.
pub const SHOTS: [(&str, &str); 3] = [
    ("opening", "the pinned test scenario's opening minute"),
    ("event-card", "a director petition's card, the event chip"),
    ("event-feed", "the feed around a firing"),
];

/// **Each picture is what it says** — asserted on the state it was taken in.
pub fn judge_shots(checks: &mut Checks, run: &Conducted) -> String {
    for (name, what) in SHOTS {
        match run.photo(name) {
            Some(shot) => {
                crate::frames::judge_chrome(checks, run, shot, what);
                crate::floors::judge_frame_floor(checks, run.font, &shot.frame, what);
                crate::floors::judge_figures(checks, run, shot, what);
            }
            None => checks.require(
                false,
                "an injector picture was never taken",
                format!("the {name} photo ({what}) is missing from the injector run"),
            ),
        }
    }
    // --- the opening minute: the scenario's stamp on screen -----------------
    if let Some(shot) = run.photo("opening") {
        let stamped =
            shot.flow.log.iter().any(|line| {
                line.contains("scenario:pinned-collector map:kawaza director:off pins:1")
            });
        checks.require(
            shot.clock.minutes == 0 && shot.flow.showing(crate::flow::Drawer::Feed) && stamped,
            "the opening picture is not the pinned scenario at minute zero with its stamp",
            format!(
                "minute {}, the drawer {:?}, and the notices read {:?}",
                shot.clock.minutes, shot.flow.drawer, shot.flow.log
            ),
        );
    }
    // --- the card: a director petition, the event chip ----------------------
    if let Some(shot) = run.photo("event-card") {
        let lens = crate::lens::Lens::on(&shot.sim);
        let petition = crate::card::voicing(&lens).and_then(|id| lens.petition(id));
        let chip = petition.and_then(|p| {
            let panel = crate::card::card(
                &lens,
                &Tuning::SHIPPED,
                shot.clock.minutes,
                p,
                layout::voicing_card(),
                false,
                false,
            );
            panel
                .runs
                .iter()
                .find(|run| run.text.starts_with("event - "))
                .map(|run| run.text.clone())
        });
        checks.require(
            shot.clock.paused
                && petition.is_some_and(|p| {
                    p.who == BOB
                        && p.template.source == Source::Director
                        && p.template.id == "the-collector-comes"
                })
                && chip.as_deref().is_some_and(|text| {
                    text == "event - from outside the camp - not their own want"
                }),
            "the event-card picture is not the pinned collector's card with its event chip",
            format!(
                "paused {}, the card is {:?}, its chip reads {chip:?}",
                shot.clock.paused,
                petition.map(|p| (p.who, p.template.id))
            ),
        );
    }
    // --- the feed around a firing --------------------------------------------
    if let Some(shot) = run.photo("event-feed") {
        let lens = crate::lens::Lens::on(&shot.sim);
        let shown = crate::attention::feed(
            &lens,
            shot.flow.show_ignored,
            crate::attention::feed_cap(&Tuning::SHIPPED),
        );
        let classes: Vec<EventClass> = shown
            .iter()
            .filter_map(|entry| shot.sim.events.get(entry.index).map(|event| event.class))
            .collect();
        checks.require(
            shot.flow.showing(crate::flow::Drawer::Feed)
                && shot.flow.show_ignored
                && classes.contains(&EventClass::Event)
                && classes.contains(&EventClass::PetitionVoiced),
            "the event-feed picture is not the feed showing the firing beside its voicing",
            format!(
                "the drawer {:?}, ignored shown {}, the rows' classes {:?}",
                shot.flow.drawer,
                shot.flow.show_ignored,
                classes.iter().map(|class| class.name()).collect::<Vec<_>>()
            ),
        );
    }
    format!(
        "injector photographed in the pinned scenario: {} ({} events, {} pauses)",
        SHOTS.map(|(name, _)| name).join(", "),
        run.events.len(),
        run.sim.pauses
    )
}
