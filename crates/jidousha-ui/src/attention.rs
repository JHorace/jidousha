//! The attention architecture: a data-registered class table, a config
//! that is simulation state, a feed that is a view over a log, and one
//! sentence for why the world stopped.
//!
//! Key types: `ClassSpec`, `Mode`, `Attention`, `Pause`, `FeedEntry`.
//! Key functions: `find_class`, `feed`, `reason_line`, `class_faults`.
//! Depends on: `jidousha-core` (for `Color`), `floors` (for `Breach`).
//! INVARIANT: the table is the behaviour. Nothing here branches on a class;
//! a screen asks the row what colour to draw and the scheduler asks the
//! config what the class does to the clock, and both walk the one table a
//! game registers. The feed holds indices into the game's log and nothing
//! else, so there is no state in which it and the log could disagree.

use jidousha_core::{Color, message};

use crate::floors::Breach;

/// What a class of event does to the player's attention — the whole
/// vocabulary, per class, player-configurable.
///
/// ```
/// use jidousha_ui::Mode;
///
/// assert_eq!(Mode::ALL.len(), 3);
/// assert_eq!(Mode::PauseAndFocus.name(), "pause");
/// ```
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
    /// Every mode, in the order a config panel offers them.
    pub const ALL: &'static [Mode] = &[Mode::Ignore, Mode::Log, Mode::PauseAndFocus];

    /// The name a stamp, a report and a config panel use.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Mode::Ignore => "ignore",
            Mode::Log => "log",
            Mode::PauseAndFocus => "pause",
        }
    }

    /// What it does, in one line — a config panel's own hint.
    #[must_use]
    pub fn meaning(self) -> &'static str {
        match self {
            Mode::Ignore => "not even in the feed",
            Mode::Log => "it lands in the feed",
            Mode::PauseAndFocus => "the world stops for it",
        }
    }
}

/// One row of a class table: what a class is called, how its chip is drawn,
/// and what it does to the world before the player says otherwise.
///
/// A game registers one `&'static [ClassSpec<..>]`, one row per variant of
/// its own class enum, and adds a class by adding a row — nothing that
/// draws, filters or stamps changes.
///
/// ```
/// use jidousha_core::Color;
/// use jidousha_ui::{ClassSpec, Mode, find_class};
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// enum Class { Departed, Refused }
/// #[derive(Clone, Copy, Debug, PartialEq)]
/// enum Art { Tower, Skull }
/// const CLASSES: &[ClassSpec<Class, Art>] = &[
///     ClassSpec { class: Class::Departed, id: "departed", color: Color::WHITE, icon: Art::Tower, default_mode: Mode::Ignore },
///     ClassSpec { class: Class::Refused, id: "refused", color: Color::MAGENTA, icon: Art::Skull, default_mode: Mode::PauseAndFocus },
/// ];
/// let row = find_class(CLASSES, Class::Refused).unwrap();
/// assert_eq!(row.id, "refused");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ClassSpec<Class, Icon> {
    /// Which class this row defines.
    pub class: Class,
    /// The id a transcript, a stamp and a config panel name it by. ASCII,
    /// lowercase, dashes.
    pub id: &'static str,
    /// The colour its chip is drawn in.
    pub color: Color,
    /// The picture its chip carries — a second channel, so the chip is not
    /// colour alone.
    pub icon: Icon,
    /// What it does to the world before the player says otherwise.
    pub default_mode: Mode,
}

/// This class's row of `table`, or `None` for a class the table has no row
/// for — an authoring fault `class_faults` reports.
#[must_use]
pub fn find_class<C: PartialEq, I>(
    table: &'static [ClassSpec<C, I>],
    class: C,
) -> Option<&'static ClassSpec<C, I>> {
    table.iter().find(|spec| spec.class == class)
}

/// What each class currently does to the world: one mode per row of the
/// table, in table order.
///
/// **Simulation state**, to be owned by the game's world rather than by a
/// screen: a change to it is a recorded input that changes what the world
/// does, and a replay that did not carry the config would reproduce the
/// orders and not the pauses.
///
/// ```
/// use jidousha_core::Color;
/// use jidousha_ui::{Attention, ClassSpec, Mode};
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// enum Class { Departed, Refused }
/// #[derive(Clone, Copy, Debug, PartialEq)]
/// enum Art { Tower, Skull }
/// const CLASSES: &[ClassSpec<Class, Art>] = &[
///     ClassSpec { class: Class::Departed, id: "departed", color: Color::WHITE, icon: Art::Tower, default_mode: Mode::Ignore },
///     ClassSpec { class: Class::Refused, id: "refused", color: Color::MAGENTA, icon: Art::Skull, default_mode: Mode::PauseAndFocus },
/// ];
/// let mut attention = Attention::opening(CLASSES);
/// assert_eq!(attention.mode(Class::Departed), Mode::Ignore);
/// attention.set(Class::Departed, Mode::Log);
/// assert_eq!(attention.stamp(), "attention:departed=log,refused=pause");
/// ```
#[derive(Debug)]
pub struct Attention<Class: 'static, Icon: 'static> {
    table: &'static [ClassSpec<Class, Icon>],
    modes: Vec<Mode>,
}

impl<C, I> Clone for Attention<C, I> {
    fn clone(&self) -> Self {
        Self {
            table: self.table,
            modes: self.modes.clone(),
        }
    }
}

impl<C, I> PartialEq for Attention<C, I> {
    /// The same table and the same modes. The table is compared by identity:
    /// a game has one, and two configs over two tables are two different
    /// things whatever their rows say.
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.table, other.table) && self.modes == other.modes
    }
}

impl<C, I> Eq for Attention<C, I> {}

impl<C: Copy + PartialEq + std::fmt::Debug, I> Attention<C, I> {
    /// The table's own defaults — what a scenario opens on.
    pub fn opening(table: &'static [ClassSpec<C, I>]) -> Self {
        Self {
            table,
            modes: table.iter().map(|spec| spec.default_mode).collect(),
        }
    }

    /// The table this config is over.
    #[must_use]
    pub fn table(&self) -> &'static [ClassSpec<C, I>] {
        self.table
    }

    fn row(&self, class: C) -> usize {
        match self.table.iter().position(|spec| spec.class == class) {
            Some(row) => row,
            None => panic!(
                "{}",
                message(
                    "an event class has no row in its class table",
                    &format!(
                        "{class:?} was asked of a table with {} rows",
                        self.table.len()
                    ),
                    "a class variant was added without its row",
                    "add a row for it to the table `Attention::opening` was built from; \
                     `class_faults` reports a table that is missing one",
                )
            ),
        }
    }

    /// What this class does right now.
    ///
    /// # Panics
    ///
    /// For a class the table has no row for — a variant added without its
    /// row, which `class_faults` catches before any world is built.
    #[must_use]
    pub fn mode(&self, class: C) -> Mode {
        self.modes[self.row(class)]
    }

    /// Set what a class does. The one write, so a screen cannot invent a
    /// fourth mode or a class the table does not have.
    ///
    /// # Panics
    ///
    /// For a class the table has no row for, as [`mode`](Self::mode) does.
    pub fn set(&mut self, class: C, mode: Mode) {
        let row = self.row(class);
        self.modes[row] = mode;
    }

    /// The config as a stamp carries it: `attention:departed=ignore,...`, in
    /// table order.
    #[must_use]
    pub fn stamp(&self) -> String {
        let body = self
            .table
            .iter()
            .zip(&self.modes)
            .map(|(spec, mode)| format!("{}={}", spec.id, mode.name()))
            .collect::<Vec<_>>()
            .join(",");
        format!("attention:{body}")
    }
}

/// Why the world stopped: which entry of the log did it, what class it was,
/// and when.
///
/// Simulation state, written where the game records the event and cleared
/// by the player's next speed input — so "what am I looking at" is a fact
/// about the world and not about the screen, and a replay pauses for the
/// same reason at the same minute.
///
/// ```
/// use jidousha_ui::Pause;
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// enum Class { Refused }
/// let pause = Pause { event: 7, class: Class::Refused, minute: 480 };
/// assert_eq!(pause.event, 7);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pause<Class> {
    /// Which entry of the game's event log did it.
    pub event: usize,
    /// What class it was.
    pub class: Class,
    /// The world-minute it fired at.
    pub minute: u64,
}

/// One row of the feed: which event, and whether it is only here because
/// the player asked to see ignored classes.
///
/// ```
/// use jidousha_ui::FeedEntry;
///
/// let row = FeedEntry { index: 3, ignored: false };
/// assert_eq!(row.index, 3);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeedEntry {
    /// The index into the game's event log — the feed's whole state.
    pub index: usize,
    /// Whether its class is configured `ignore` (drawn dimmed, for auditing).
    pub ignored: bool,
}

/// The feed: the game's event log, newest first, filtered by the config and
/// bounded by `cap`.
///
/// `classes` is the class of every event in the log, oldest first — the one
/// thing the feed needs to know about an event. **Derived on every call**:
/// the entries are indices, so there is nothing here that could be stale,
/// out of order, or missing a line the log has.
///
/// ```
/// use jidousha_core::Color;
/// use jidousha_ui::{Attention, ClassSpec, FeedEntry, Mode, feed};
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// enum Class { Departed, Refused }
/// #[derive(Clone, Copy, Debug, PartialEq)]
/// enum Art { Tower, Skull }
/// const CLASSES: &[ClassSpec<Class, Art>] = &[
///     ClassSpec { class: Class::Departed, id: "departed", color: Color::WHITE, icon: Art::Tower, default_mode: Mode::Ignore },
///     ClassSpec { class: Class::Refused, id: "refused", color: Color::MAGENTA, icon: Art::Skull, default_mode: Mode::PauseAndFocus },
/// ];
/// let attention = Attention::opening(CLASSES);
/// let log = [Class::Departed, Class::Refused, Class::Departed];
/// assert_eq!(
///     feed(log.iter().copied(), &attention, false, 10),
///     vec![FeedEntry { index: 1, ignored: false }]
/// );
/// assert_eq!(feed(log.iter().copied(), &attention, true, 2).len(), 2, "newest two, ignored shown");
/// ```
pub fn feed<C: Copy + PartialEq + std::fmt::Debug, I, L>(
    classes: L,
    attention: &Attention<C, I>,
    show_ignored: bool,
    cap: usize,
) -> Vec<FeedEntry>
where
    L: IntoIterator<Item = C>,
    L::IntoIter: DoubleEndedIterator + ExactSizeIterator,
{
    classes
        .into_iter()
        .enumerate()
        .rev()
        .filter_map(|(index, class)| {
            let ignored = attention.mode(class) == Mode::Ignore;
            (!ignored || show_ignored).then_some(FeedEntry { index, ignored })
        })
        .take(cap)
        .collect()
}

/// The pause reason, as the banner, the feed's header and a card's note all
/// say it — one sentence, one source.
///
/// Class and place first, the sentence after: a long note is clipped at the
/// drawer's edge, and what must survive the clip is what stopped the world
/// and where.
///
/// ```
/// use jidousha_ui::reason_line;
///
/// assert_eq!(
///     reason_line("ask-declined", "the Deep Cave", "Bob would rather not"),
///     "paused: ask-declined at the Deep Cave - Bob would rather not"
/// );
/// ```
#[must_use]
pub fn reason_line(class: &str, place: &str, text: &str) -> String {
    format!("paused: {class} at {place} - {text}")
}

/// What a class table can be wrong about before any world is built: an id
/// that is not stamp-shaped, two rows sharing an id, two rows for one
/// class, and a chip whose colour would vanish into `ground`.
///
/// A class with *no* row is the one fault a table cannot show; the game's
/// own list of classes is what to walk against `find_class`.
///
/// ```
/// use jidousha_core::Color;
/// use jidousha_ui::{ClassSpec, Mode, class_faults};
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// enum Class { Departed }
/// #[derive(Clone, Copy, Debug, PartialEq)]
/// enum Art { Tower }
/// let ground = Color::rgb(0.1, 0.1, 0.1);
/// let twice: &[ClassSpec<Class, Art>] = &[
///     ClassSpec { class: Class::Departed, id: "departed", color: Color::WHITE, icon: Art::Tower, default_mode: Mode::Ignore },
///     ClassSpec { class: Class::Departed, id: "departed", color: ground, icon: Art::Tower, default_mode: Mode::Ignore },
/// ];
/// let faults = class_faults(twice, ground);
/// assert_eq!(faults.len(), 5, "{faults:?}");
/// ```
pub fn class_faults<C: PartialEq + std::fmt::Debug, I>(
    table: &[ClassSpec<C, I>],
    ground: Color,
) -> Vec<Breach> {
    let mut out = Vec::new();
    for (index, spec) in table.iter().enumerate() {
        if spec.id.is_empty()
            || !spec
                .id
                .chars()
                .all(|glyph| glyph.is_ascii_lowercase() || glyph == '-')
        {
            out.push(Breach {
                what: "an event class id is not stamp-shaped ASCII",
                detail: format!("row {index} is named {:?}", spec.id),
            });
        }
        if table.iter().filter(|other| other.id == spec.id).count() != 1 {
            out.push(Breach {
                what: "two event classes share an id",
                detail: format!("{:?} appears more than once", spec.id),
            });
        }
        if table
            .iter()
            .filter(|other| other.class == spec.class)
            .count()
            != 1
        {
            out.push(Breach {
                what: "an event class has more than one row in the table",
                detail: format!("{:?} appears more than once", spec.class),
            });
        }
        if spec.color == ground || spec.color.a <= 0.99 {
            out.push(Breach {
                what: "an event class chip would be invisible on the feed",
                detail: format!(
                    "{:?} is drawn {:?} and the feed's fill is {ground:?}",
                    spec.id, spec.color
                ),
            });
        }
    }
    out
}
