//! The attention architecture: the event-class table, the auto-pause config,
//! and the feed that is a **view** of the simulation's event log (GDD §3,
//! wave 0a; DESIGN §6).
//!
//! # The table is the behaviour
//!
//! [`CLASSES`] is one row per event class — id, colour role, icon role, and
//! the mode the class opens on. **Nothing branches on a class id anywhere in
//! this game.** A screen asks the row what colour to draw; the scheduler asks
//! the config what a class does to the clock; both walk the same table. That
//! is what makes a wave-1 module's new class (a petition voiced, an upkeep
//! shortfall) a row here plus an enum variant, and no `match` anywhere else.
//!
//! # The feed is a view, never a list
//!
//! [`feed`] derives its entries from [`Lens::events`] every time it is asked.
//! There is no second vector, nothing copies an event anywhere, and there is
//! therefore no state in which the feed and the transcript could disagree —
//! which is the failure this whole surface exists to not have (GDD §1: a
//! surface that could disagree with the sim is the failure mode).
//! `attention::feed_is_a_view` asserts exactly that over a conducted run.
//!
//! # Auto-pause is a simulation transition
//!
//! When an event whose configured mode is [`Mode::PauseAndFocus`] fires, the
//! simulation records the pause on itself and `sim::fire_due` puts the clock
//! at speed 0 in the same tick. No synthetic input is injected: the pause is
//! a deterministic function of (recorded inputs, sim rules), so a replay
//! reproduces it exactly rather than reproducing a click nobody made. The
//! config is sim state for the same reason — a change to it is a recorded
//! input like a speed change, and a replay carries it.

use jidousha::ui::find_class;
pub use jidousha::ui::{FeedEntry, Mode};

use crate::constants::Tuning;
use crate::grid::LOCATIONS;
use crate::lens::Lens;
use crate::sim::Event;
use crate::sprites::Art;
use crate::theme;

/// What each class currently does to the world — the kit's config over this
/// game's table, held by `Sim` (ADR-0046).
pub type Attention = jidousha::ui::Attention<EventClass, Art>;

/// Why the world stopped: the kit's record over this game's classes.
///
/// Simulation state, written by [`crate::sim::Sim::emit`] and cleared by the
/// player's next speed input — so "what am I looking at" is a fact about the
/// world and not about the screen, and a replay pauses for the same reason at
/// the same world-minute.
pub type Pause = jidousha::ui::Pause<EventClass>;

/// One row of the event-class table, over this game's classes and art.
pub type ClassSpec = jidousha::ui::ClassSpec<EventClass, Art>;

/// The classes of thing that happen. One variant per row of [`CLASSES`].
///
/// A wave-1 module adds its classes here and there — a variant and a row —
/// and nothing else in the game changes, because nothing else in the game
/// asks which class this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventClass {
    /// A party left the town for a site.
    Departed,
    /// A party reached its site.
    Arrived,
    /// Work began (same world-minute as the arrival, its own address).
    WorkBegan,
    /// The quest resolved — the stub success — and the pot paid.
    QuestComplete,
    /// The party is home.
    Returned,
    /// **A character decided something for themselves** and set out to do it
    /// (GDD §5's autonomy module, wave 1.1). The reason is on the note: this
    /// is the class that says *why* anybody did anything.
    ActionStarted,
    /// The thing they chose is finished and they are home again.
    ActionDone,
    /// **The player posted something** (the asks module, wave 1.2) — an entry
    /// on the ledger, binding nobody until it is heard.
    PostingMade,
    /// Somebody heard a posting: in camp at once, on the road when the
    /// messenger reached them.
    AskHeard,
    /// Somebody took a posted job, for its wage, for a stated reason.
    AskAgreed,
    /// Somebody the player asked **by name** weighed it and did something
    /// else, and this is why. An open posting has no such event: nobody
    /// refuses a notice on a board, it just goes unfilled.
    AskDeclined,
    /// Somebody abandoned a posting's job because something pressed harder.
    ///
    /// **The seam, not the behaviour** (wave 1.2): nothing in this build
    /// presses harder than work, so nothing in a played run emits this.
    /// Needs (1.3) and petitions are the pressure it is waiting for, and
    /// `answers::drop_errand` is the door they come through.
    AskDropped,
    /// The player took a posting down.
    PostingWithdrawn,
    /// **Somebody could not meet their upkeep this interval** (the needs
    /// module, wave 1.3) — the first rung of the escalation pipe, and the one
    /// occurrence in this game that rewrites the person it is about: the note
    /// carries the new source line, so the feed says *why* this person is in
    /// trouble rather than only that they are.
    UpkeepShortfall,
    /// **Somebody arrived in the camp** (`CAST.md` §4's six who came later).
    Joined,
    /// **The player built something** — the treasury's first real sink, and
    /// the beat where a camp becomes a settlement (`CAST.md` §1).
    Built,
    /// And the standing job slots that building opened.
    IndustryOpened,
    /// **A posted job was botched** (the resolution module, wave 1.4): the
    /// wage was paid anyway, nothing was minted, and the job is back on its
    /// board. A contract the player wrote going wrong is the player's to see.
    TaskFailed,
    /// **A job somebody chose for themselves was botched**: no pay, nothing
    /// minted, and the job is back on its board.
    TaskFailedOwn,
    /// **Somebody put a petition to the player** (the petitions module, wave
    /// 1.5): an ask, a deadline and a declared consequence, binding from this
    /// minute. The event carries the record, and the voicing overlay draws its
    /// card.
    PetitionVoiced,
    /// A petition was met — by the player's hand or by the world's — and the
    /// petitioner's circumstances are better for it.
    PetitionSatisfied,
    /// A petition's deadline came unmet, and the consequence its card declared
    /// fired.
    PetitionFailed,
    /// **The director fired** (the events-director, wave 1.6): an event from
    /// outside the camp reached somebody, or the world was quiet and it
    /// passed. Bookkeeping — what the player sees is the petition it voices.
    Event,
}

/// The event-class table (GDD §3's wave 0a spec, at the mockup's defaults).
///
/// **The defaults are the mockup's, played and approved**: movement is
/// `ignore` because the map already shows motion, and a completion is `log`
/// because it is worth knowing and not worth stopping for. The
/// petition/consequence family that opens on `pause-and-focus` arrives with
/// the petitions module (wave 1.5); the one class that stops a shipped
/// scenario today is wave 1.2's `ask-declined`, and **none of wave 1.3's four
/// joins it** — pressure that stopped the world every time somebody went short
/// would stop it every day, and the chips and the camp line are where that
/// pressure is meant to be read.
///
/// Wave 1.1 adds the scorer's two: `action-started` opens on `log`, because
/// the reason a character left is the one thing the feed exists to carry, and
/// `action-done` on `ignore`, because the completion and the return already
/// said it.
pub const CLASSES: &[ClassSpec] = &[
    ClassSpec {
        class: EventClass::Departed,
        id: "departed",
        color: theme::DIM,
        icon: Art::QuestTower,
        default_mode: Mode::Ignore,
    },
    ClassSpec {
        class: EventClass::Arrived,
        id: "arrived",
        color: theme::DIM,
        icon: Art::QuestCave,
        default_mode: Mode::Ignore,
    },
    ClassSpec {
        class: EventClass::WorkBegan,
        id: "work-began",
        color: theme::DIM,
        icon: Art::QuestCrypt,
        default_mode: Mode::Ignore,
    },
    ClassSpec {
        class: EventClass::QuestComplete,
        id: "quest-complete",
        color: theme::GOLD,
        icon: Art::Coin,
        default_mode: Mode::Log,
    },
    ClassSpec {
        class: EventClass::Returned,
        id: "returned",
        color: theme::REGARD,
        icon: Art::Heart,
        default_mode: Mode::Ignore,
    },
    // **Autonomy's two classes** (wave 1.1). A self-dispatch reuses the five
    // movement classes above for the journey — one story per movement, not
    // two — and these two carry the *decision*: the start is worth reading
    // because it is the only place the reason appears, and the end is not,
    // because the completion and the return already spoke.
    ClassSpec {
        class: EventClass::ActionStarted,
        id: "action-started",
        color: theme::INK,
        icon: Art::Scout,
        default_mode: Mode::Log,
    },
    ClassSpec {
        class: EventClass::ActionDone,
        id: "action-done",
        color: theme::DIM,
        icon: Art::Craft,
        default_mode: Mode::Ignore,
    },
    // **The asks module's six** (wave 1.2). The player's own two — a posting
    // made and a posting withdrawn — are `ignore`, because the ledger already
    // holds them and an event for the thing you just did is a feed that
    // repeats you back at yourself. The three that are somebody else's answer
    // are worth reading, and **a refusal by name is the first class in this
    // game that stops the world**: being told no, with a reason, is the whole
    // of what this wave added to the loop, and a player who missed it would
    // be a player who never found out that asking can fail.
    ClassSpec {
        class: EventClass::PostingMade,
        id: "posting-made",
        color: theme::INK,
        icon: Art::Eye,
        default_mode: Mode::Ignore,
    },
    ClassSpec {
        class: EventClass::AskHeard,
        id: "ask-heard",
        color: theme::DIM,
        icon: Art::Renown,
        default_mode: Mode::Log,
    },
    ClassSpec {
        class: EventClass::AskAgreed,
        id: "ask-agreed",
        color: theme::REGARD,
        icon: Art::Indebted,
        default_mode: Mode::Log,
    },
    ClassSpec {
        class: EventClass::AskDeclined,
        id: "ask-declined",
        color: theme::EMBER,
        icon: Art::Skull,
        default_mode: Mode::PauseAndFocus,
    },
    ClassSpec {
        class: EventClass::AskDropped,
        id: "ask-dropped",
        color: theme::EMBER,
        icon: Art::Restless,
        default_mode: Mode::Log,
    },
    ClassSpec {
        class: EventClass::PostingWithdrawn,
        id: "posting-withdrawn",
        color: theme::DIM,
        icon: Art::Maker,
        default_mode: Mode::Ignore,
    },
    // **Needs and settlement's four** (wave 1.3). All four are `log`: none of
    // them is a thing the player has to answer *now*, and the class that stops
    // the world is still the refusal by name. A shortfall is the loudest of
    // the four and it is still not a pause, because pressure that stopped the
    // world every time somebody went short would stop it four times a day —
    // the chips and the camp line are where that pressure is meant to be read,
    // and the config panel is where a player who wants to be stopped says so.
    ClassSpec {
        class: EventClass::UpkeepShortfall,
        id: "upkeep-shortfall",
        color: theme::EMBER,
        icon: Art::Flame,
        default_mode: Mode::Log,
    },
    ClassSpec {
        class: EventClass::Joined,
        id: "joined",
        color: theme::REGARD,
        icon: Art::Caring,
        default_mode: Mode::Log,
    },
    ClassSpec {
        class: EventClass::Built,
        id: "built",
        color: theme::GOLD,
        icon: Art::QuestVault,
        default_mode: Mode::Log,
    },
    ClassSpec {
        class: EventClass::IndustryOpened,
        id: "industry-opened",
        color: theme::INK,
        icon: Art::Labor,
        default_mode: Mode::Log,
    },
    // **Resolution's two** (wave 1.4). A completion still lands as
    // `quest-complete`, now carrying its tier (*went well* or *done*), at the
    // same `log` — a job that went well is worth knowing and pays as done. A
    // failure is two classes because it is two different things to the
    // player: a **posted** job going wrong is a contract they wrote, and it
    // stops the world on the targeted decline's precedent; one somebody chose
    // for themselves is their own bad day, and it lands in the feed. The
    // second id is `own-job-failed` rather than `task-failed-own` because the
    // feed's class column holds fourteen glyphs and the photograph showed the
    // fifteenth running into the place.
    ClassSpec {
        class: EventClass::TaskFailed,
        id: "task-failed",
        color: theme::EMBER,
        icon: Art::Skull,
        default_mode: Mode::PauseAndFocus,
    },
    ClassSpec {
        class: EventClass::TaskFailedOwn,
        id: "own-job-failed",
        color: theme::EMBER,
        icon: Art::Flame,
        default_mode: Mode::Log,
    },
    // **The petitions' three** (wave 1.5), at the approved mockup's defaults
    // (2026-10-02). A voicing **stops the world**: the capsule's primary
    // screen, where the card is the warning and the deadline starts. A
    // satisfaction is `log` — the camp got better and the feed says so. A
    // failure stops the world too, on the posted-failure precedent: a
    // consequence the card declared, firing, is the player's to see. All three
    // are the config panel's to override.
    ClassSpec {
        class: EventClass::PetitionVoiced,
        id: "petition-voiced",
        color: theme::GOLD,
        icon: Art::Eye,
        default_mode: Mode::PauseAndFocus,
    },
    ClassSpec {
        class: EventClass::PetitionSatisfied,
        id: "petition-satisfied",
        color: theme::REGARD,
        icon: Art::Heart,
        default_mode: Mode::Log,
    },
    ClassSpec {
        class: EventClass::PetitionFailed,
        id: "petition-failed",
        color: theme::EMBER,
        icon: Art::Skull,
        default_mode: Mode::PauseAndFocus,
    },
    // **The director's one** (wave 1.6), and it opens on `ignore`: the firing
    // is bookkeeping, and what the player sees is the petition it voices,
    // which stops the world as every voicing does. A firing that found nobody
    // is the world being quiet, which is not an event either. Its id is the
    // word the source table shows for the class — `event` — so the feed
    // never says `director` to a player who is never told there is one.
    ClassSpec {
        class: EventClass::Event,
        id: "event",
        color: theme::DIM,
        icon: Art::Restless,
        default_mode: Mode::Ignore,
    },
];

/// How wide a class chip's icon is drawn, in reference pixels (UI.md §3's
/// sixteen-unit chip).
pub const CHIP: f32 = 16.0;

impl EventClass {
    /// Every class, in table order — what the config panel lists.
    pub fn all() -> Vec<EventClass> {
        CLASSES.iter().map(|spec| spec.class).collect()
    }

    /// This class's row.
    ///
    /// A linear walk over the table, like `Art::index`, so the enum and the
    /// table cannot drift the way parallel indices do. A class with no row is
    /// an authoring fault the vocabulary check catches; here it reads as the
    /// first row rather than panicking in a draw system.
    pub fn spec(self) -> &'static ClassSpec {
        find_class(CLASSES, self).unwrap_or(&CLASSES[0])
    }

    /// Its index in the table — the config panel's row for it.
    pub fn index(self) -> usize {
        CLASSES
            .iter()
            .position(|spec| spec.class == self)
            .unwrap_or(0)
    }

    /// The class's name, for transcripts, the feed and the config panel.
    pub fn name(self) -> &'static str {
        self.spec().id
    }
}

/// The feed: the sim's event log, newest first, filtered by the config and
/// bounded by `feed_cap` — the kit's view, read through the lens.
///
/// **Derived on every call.** The entries are indices into the log, so there
/// is nothing here that could be stale, out of order, or missing a line the
/// transcript has.
pub fn feed(lens: &Lens<'_>, show_ignored: bool, cap: usize) -> Vec<FeedEntry> {
    jidousha::ui::feed(
        lens.events().iter().map(|event| event.class),
        lens.attention(),
        show_ignored,
        cap,
    )
}

/// The place tag on a feed row: the named location, or the bare tile when the
/// event happened on the road.
pub fn place_tag(event: &Event) -> String {
    match event.location.and_then(|index| LOCATIONS.get(index)) {
        Some(spec) => spec.name.to_owned(),
        None => format!("({}, {})", event.tile.x, event.tile.y),
    }
}

/// The pause reason, as the banner and the feed's header both say it — one
/// sentence, one source.
pub fn reason_line(lens: &Lens<'_>) -> Option<String> {
    let pause = lens.pause()?;
    let event = lens.events().get(pause.event)?;
    Some(jidousha::ui::reason_line(
        pause.class.name(),
        &place_tag(event),
        &event.text(lens),
    ))
}

/// Engine ticks per tenth of a wall-second, at the engine's fixed sixty.
///
/// The pulse is presentation and so is measured in wall time; the drawer's
/// range is small, so the constant is stated in tenths and multiplied here.
pub const TICKS_PER_TENTH: u64 = 6;

/// How many ticks a click-to-focus pulse marker lasts.
pub fn pulse_ticks(tuning: &Tuning) -> u64 {
    u64::try_from(tuning.pulse_tenths.max(0)).unwrap_or(0) * TICKS_PER_TENTH
}

/// How many entries the feed view holds.
pub fn feed_cap(tuning: &Tuning) -> usize {
    usize::try_from(tuning.feed_cap.max(0)).unwrap_or(0)
}

/// The class table's own validation: the claims a comment cannot hold.
pub fn vocabulary(checks: &mut crate::checks::Checks) {
    // The table's own shape — stamp-shaped ids, one row per class, a chip
    // colour that is not the feed's fill — is the kit's to judge.
    for breach in jidousha::ui::class_faults(CLASSES, theme::PANEL) {
        checks.require(false, breach.what, breach.detail);
    }
    for (index, spec) in CLASSES.iter().enumerate() {
        checks.require(
            spec.class.spec().id == spec.id && spec.class.index() == index,
            "an event class does not find its own row",
            format!(
                "{:?} is row {index} and looks up {:?} at {}",
                spec.class,
                spec.class.spec().id,
                spec.class.index()
            ),
        );
        // The chip is two channels: a colour that is not the bar it may sit
        // on either, and a picture that is square and drawn at a whole scale.
        checks.require(
            spec.color != theme::BAR,
            "an event class chip would be invisible on the bar",
            format!(
                "{:?} is drawn {:?} and the bar's fill is {:?}",
                spec.id,
                spec.color,
                theme::BAR
            ),
        );
        let texels = spec.icon.texels();
        checks.require(
            texels.width == texels.height && (CHIP as u32).is_multiple_of(texels.width),
            "an event class chip icon is not a square whole-scale picture",
            format!(
                "{:?} carries {:?}, which is {}x{} texels and the chip is {CHIP} units",
                spec.id, spec.icon, texels.width, texels.height
            ),
        );
    }
    checks.require(
        EventClass::all().len() == CLASSES.len(),
        "the class table and the class list disagree about how many classes there are",
        format!(
            "the table has {} rows and the list has {}",
            CLASSES.len(),
            EventClass::all().len()
        ),
    );
    for mode in Mode::ALL.iter().copied() {
        checks.require(
            !mode.name().is_empty() && mode.name().chars().all(|g| g.is_ascii_lowercase()),
            "a mode's name is not stamp-shaped ASCII",
            format!("{mode:?} is named {:?}", mode.name()),
        );
    }
    // The mockup's defaults, asserted as the shipped table rather than as a
    // sentence in a document: movement is ignored and a completion is logged.
    let opening = Attention::opening(CLASSES);
    for (class, wanted) in [
        (EventClass::Departed, Mode::Ignore),
        (EventClass::Arrived, Mode::Ignore),
        (EventClass::WorkBegan, Mode::Ignore),
        (EventClass::Returned, Mode::Ignore),
        (EventClass::QuestComplete, Mode::Log),
        (EventClass::ActionStarted, Mode::Log),
        (EventClass::ActionDone, Mode::Ignore),
        (EventClass::PostingMade, Mode::Ignore),
        (EventClass::AskHeard, Mode::Log),
        (EventClass::AskAgreed, Mode::Log),
        (EventClass::AskDeclined, Mode::PauseAndFocus),
        (EventClass::AskDropped, Mode::Log),
        (EventClass::PostingWithdrawn, Mode::Ignore),
        (EventClass::PetitionVoiced, Mode::PauseAndFocus),
        (EventClass::PetitionSatisfied, Mode::Log),
        (EventClass::PetitionFailed, Mode::PauseAndFocus),
        (EventClass::Event, Mode::Ignore),
    ] {
        checks.require(
            opening.mode(class) == wanted,
            "a class does not open on the mode the mockup settled",
            format!(
                "{} opens on {} and the owner-tested default is {}",
                class.name(),
                opening.mode(class).name(),
                wanted.name()
            ),
        );
    }
    // Setting one mode moves one mode.
    let mut set = Attention::opening(CLASSES);
    set.set(EventClass::Departed, Mode::PauseAndFocus);
    checks.require(
        set.mode(EventClass::Departed) == Mode::PauseAndFocus
            && set.mode(EventClass::QuestComplete) == opening.mode(EventClass::QuestComplete),
        "setting one class's mode moved another class's mode",
        format!("the config reads {}", set.stamp()),
    );
}

/// The two attention constants, judged against **shipped literals** — the
/// instrument the mutation round reads them through.
///
/// Derived expectations would make both constants invisible to their own
/// round: a check that recomputes `feed_cap` from `tuning` cannot see
/// `feed_cap` move.
pub fn judge_at(checks: &mut crate::checks::Checks, tuning: &Tuning) {
    checks.require(
        pulse_ticks(tuning) == 150,
        "the click-to-focus pulse does not last what the shipped set says",
        format!(
            "the pulse runs {} ticks and the shipped 25 tenths at sixty ticks a second is 150",
            pulse_ticks(tuning)
        ),
    );
    // A feed over more events than it may hold: the cap is what stops it.
    let mut sim = crate::sim::Sim::opening(
        crate::scenario::freeplay(),
        tuning,
        crate::modules::ModuleSet::ALL,
    );
    for minute in 0..25u64 {
        sim.events.push(Event {
            minute,
            class: EventClass::QuestComplete,
            party: 0,
            tile: LOCATIONS[0].tile,
            location: Some(0),
            gold: 0,
            note: format!("a probe event at minute {minute}"),
            judged: None,
            tier: None,
            petition: None,
        });
    }
    let lens = Lens::on(&sim);
    let held = feed(&lens, false, feed_cap(tuning)).len();
    checks.require(
        held == 10,
        "the feed does not hold what the shipped cap says",
        format!(
            "over twenty-five logged events the feed holds {held} entries and the shipped cap \
             is 10"
        ),
    );
    checks.require(
        feed(&lens, false, feed_cap(tuning))
            .first()
            .is_some_and(|entry| entry.index == 24),
        "the feed is not newest-first",
        format!(
            "the first entry is {:?} of twenty-five events",
            feed(&lens, false, feed_cap(tuning)).first()
        ),
    );
}

/// **The feed is a view of the transcript** (GDD §1's one-source rule): its
/// contents equal the sim's own event log, filtered by the config, at both
/// settings of the ignored toggle.
pub fn feed_is_a_view(checks: &mut crate::checks::Checks, run: &crate::sweep::Conducted) {
    let lens = Lens::on(&run.sim);
    let cap = feed_cap(&Tuning::SHIPPED);
    for show_ignored in [false, true] {
        let feed = feed(&lens, show_ignored, cap);
        // The same answer, computed the long way round from the transcript.
        let wanted: Vec<usize> = run
            .events
            .iter()
            .enumerate()
            .rev()
            .filter(|(_, event)| show_ignored || lens.attention().mode(event.class) != Mode::Ignore)
            .map(|(index, _)| index)
            .take(cap)
            .collect();
        let got: Vec<usize> = feed.iter().map(|entry| entry.index).collect();
        checks.require(
            got == wanted,
            "the feed is not the event log filtered by the config",
            format!(
                "with show-ignored {show_ignored} the feed holds events {got:?} and the \
                 transcript filtered by {} holds {wanted:?}; the feed is a view, and a second \
                 list is the failure this is asserted against",
                lens.attention().stamp()
            ),
        );
        for entry in &feed {
            let Some(event) = run.events.get(entry.index) else {
                checks.require(
                    false,
                    "a feed entry names an event the transcript does not have",
                    format!("entry {entry:?} of {} events", run.events.len()),
                );
                continue;
            };
            checks.require(
                entry.ignored == (lens.attention().mode(event.class) == Mode::Ignore),
                "a feed entry disagrees with the config about whether it is ignored",
                format!(
                    "{} reads ignored={} and the config says {}",
                    event.class.name(),
                    entry.ignored,
                    lens.attention().mode(event.class).name()
                ),
            );
        }
    }
    // Hiding the ignored classes is what the filter does, and it does it here:
    // this scenario's transcript is mostly movement. Judged **over the whole
    // log rather than over a capped view**, because a world busy enough to
    // fill the cap twice over would fill it with either filter and the claim
    // would pass without the filter doing anything.
    let shown = feed(&lens, false, usize::MAX).len();
    let all = feed(&lens, true, usize::MAX).len();
    checks.require(
        shown < all,
        "the ignored filter hides nothing in a run that is mostly movement",
        format!(
            "{shown} entries with the ignored classes hidden and {all} with them shown, over \
             {} events; the assertion above would pass with the filter removed",
            run.events.len()
        ),
    );
}
