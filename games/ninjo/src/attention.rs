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

use jidousha::prelude::*;

use crate::constants::Tuning;
use crate::grid::LOCATIONS;
use crate::lens::Lens;
use crate::sim::Event;
use crate::sprites::Art;
use crate::theme;

/// What a class of event does to the player's attention.
///
/// The Paradox convention, and the whole vocabulary: three modes, per class,
/// player-configurable (DESIGN §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Not even in the feed. The map already shows it.
    Ignore,
    /// It lands in the feed and the world keeps running.
    Log,
    /// The world stops, and the feed says why.
    PauseAndFocus,
}

impl Mode {
    /// Every mode, in the order the config panel offers them.
    pub const ALL: &'static [Mode] = &[Mode::Ignore, Mode::Log, Mode::PauseAndFocus];

    /// The name a stamp, a report and the config panel use.
    pub fn name(self) -> &'static str {
        match self {
            Mode::Ignore => "ignore",
            Mode::Log => "log",
            Mode::PauseAndFocus => "pause",
        }
    }

    /// What it does, in one line — the config panel's own hint.
    pub fn meaning(self) -> &'static str {
        match self {
            Mode::Ignore => "not even in the feed",
            Mode::Log => "it lands in the feed",
            Mode::PauseAndFocus => "the world stops for it",
        }
    }
}

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
}

/// One row of the event-class table: what a class is called, how it is drawn,
/// and what it does to the world by default.
#[derive(Clone, Copy, Debug)]
pub struct ClassSpec {
    /// Which class this row defines.
    pub class: EventClass,
    /// The id a transcript, a stamp and the config panel name it by. ASCII,
    /// lowercase.
    pub id: &'static str,
    /// The colour role its chip is drawn in.
    pub color: Color,
    /// The icon role its chip carries — a second channel, so the chip is not
    /// colour alone (UI.md §1).
    pub icon: Art,
    /// What it does to the world before the player says otherwise.
    pub default_mode: Mode,
}

/// The event-class table (GDD §3's wave 0a spec, at the mockup's defaults).
///
/// **The defaults are the mockup's, played and approved**: movement is
/// `ignore` because the map already shows motion, and a completion is `log`
/// because it is worth knowing and not worth stopping for. The
/// petition/consequence family that opens on `pause-and-focus` arrives with
/// the petitions module (wave 1.3) — no class this build has is one, so a
/// shipped scenario never auto-pauses until the player asks for it in the
/// config panel. That is the mockup's answer, not an oversight.
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
    /// A linear walk over a five-row table, like `Art::index`, so the enum and
    /// the table cannot drift the way parallel indices do. A class with no row
    /// is an authoring fault the vocabulary check catches; here it reads as the
    /// first row rather than panicking in a draw system.
    pub fn spec(self) -> &'static ClassSpec {
        CLASSES
            .iter()
            .find(|spec| spec.class == self)
            .unwrap_or(&CLASSES[0])
    }

    /// Its index in the table — what [`Attention`] stores modes by.
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

/// What each class currently does to the world.
///
/// **Simulation state** (`Sim` owns one), because a change to it is a recorded
/// input that changes what the world does: a replay that did not carry the
/// config would reproduce the orders and not the pauses. Held as one mode per
/// row of [`CLASSES`], in table order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attention {
    modes: Vec<Mode>,
}

impl Default for Attention {
    fn default() -> Self {
        Self::opening()
    }
}

impl Attention {
    /// The table's own defaults — what a scenario opens on.
    pub fn opening() -> Self {
        Self {
            modes: CLASSES.iter().map(|spec| spec.default_mode).collect(),
        }
    }

    /// What this class does right now.
    pub fn mode(&self, class: EventClass) -> Mode {
        self.modes
            .get(class.index())
            .copied()
            .unwrap_or(class.spec().default_mode)
    }

    /// Set what a class does. The one write, so a screen cannot invent a
    /// fourth mode or a class the table does not have.
    pub fn set(&mut self, class: EventClass, mode: Mode) {
        if let Some(slot) = self.modes.get_mut(class.index()) {
            *slot = mode;
        }
    }

    /// The config as a stamp carries it: `attention:departed=ignore,...`.
    pub fn stamp(&self) -> String {
        let body = CLASSES
            .iter()
            .map(|spec| format!("{}={}", spec.id, self.mode(spec.class).name()))
            .collect::<Vec<_>>()
            .join(",");
        format!("attention:{body}")
    }
}

/// Why the world stopped: the class that did it, and which entry of the event
/// log it was.
///
/// Simulation state, written by [`crate::sim::Sim::emit`] and cleared by the
/// player's next speed input — so "what am I looking at" is a fact about the
/// world and not about the screen, and a replay pauses for the same reason at
/// the same world-minute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pause {
    /// Which entry of `Sim::events` did it.
    pub event: usize,
    /// What class it was.
    pub class: EventClass,
    /// The world-minute it fired at.
    pub minute: u64,
}

/// One row of the feed: which event, and whether it is only here because the
/// player asked to see ignored classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeedEntry {
    /// The index into the sim's event log — the feed's whole state.
    pub index: usize,
    /// Whether its class is configured `ignore` (drawn dimmed, for auditing).
    pub ignored: bool,
}

/// The feed: the sim's event log, newest first, filtered by the config and
/// bounded by `feed_cap`.
///
/// **Derived on every call.** The entries are indices into the log, so there
/// is nothing here that could be stale, out of order, or missing a line the
/// transcript has.
pub fn feed(lens: &Lens<'_>, show_ignored: bool, cap: usize) -> Vec<FeedEntry> {
    let attention = lens.attention();
    lens.events()
        .iter()
        .enumerate()
        .rev()
        .filter_map(|(index, event)| {
            let ignored = attention.mode(event.class) == Mode::Ignore;
            (!ignored || show_ignored).then_some(FeedEntry { index, ignored })
        })
        .take(cap)
        .collect()
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
    // Class and place first, the sentence after: a long note is clipped at
    // the drawer's edge, and what must survive the clip is what stopped the
    // world and where.
    Some(format!(
        "paused: {} at {} - {}",
        pause.class.name(),
        place_tag(event),
        event.text(lens)
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
    for (index, spec) in CLASSES.iter().enumerate() {
        checks.require(
            !spec.id.is_empty()
                && spec
                    .id
                    .chars()
                    .all(|glyph| glyph.is_ascii_lowercase() || glyph == '-'),
            "an event class id is not stamp-shaped ASCII",
            format!("CLASSES[{index}] is named {:?}", spec.id),
        );
        checks.require(
            CLASSES.iter().filter(|other| other.id == spec.id).count() == 1,
            "two event classes share an id",
            format!("{:?} appears more than once in CLASSES", spec.id),
        );
        checks.require(
            CLASSES
                .iter()
                .filter(|other| other.class == spec.class)
                .count()
                == 1,
            "an event class has more than one row in the table",
            format!("{:?} appears more than once in CLASSES", spec.class),
        );
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
        // The chip is two channels: a colour that is not the panel it sits on,
        // and a picture that is square and drawn at a whole scale.
        checks.require(
            spec.color != theme::PANEL && spec.color != theme::BAR && spec.color.a > 0.99,
            "an event class chip would be invisible on the feed",
            format!(
                "{:?} is drawn {:?} and the feed's fill is {:?}",
                spec.id,
                spec.color,
                theme::PANEL
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
    let opening = Attention::opening();
    for (class, wanted) in [
        (EventClass::Departed, Mode::Ignore),
        (EventClass::Arrived, Mode::Ignore),
        (EventClass::WorkBegan, Mode::Ignore),
        (EventClass::Returned, Mode::Ignore),
        (EventClass::QuestComplete, Mode::Log),
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
    let mut set = Attention::opening();
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
    let mut sim = crate::sim::Sim::opening(tuning);
    for minute in 0..25u64 {
        sim.events.push(Event {
            minute,
            class: EventClass::QuestComplete,
            party: 0,
            tile: LOCATIONS[0].tile,
            location: Some(0),
            note: format!("a probe event at minute {minute}"),
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
    // this scenario's transcript is mostly movement.
    let shown = feed(&lens, false, cap).len();
    let all = feed(&lens, true, cap).len();
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
