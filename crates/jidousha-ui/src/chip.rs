//! The one-value chip: which id is lit, or none — and the toggle every
//! tap-to-open field is.
//!
//! Key types: `Chip`. Key functions: `toggle`.
//! Depends on: nothing beyond `std`.
//! INVARIANT: a chip's state is one `Option<Id>`. There is no flag beside it
//! to keep in step, which is the whole reason it is a type: every parallel-
//! state defect the exemplar audit found was a surface that grew a `bool`
//! beside an `Option` (`games/ninjo/FINDINGS.md` G-031, G-063; ADR-0046).

/// Flip `slot` between holding `value` and holding nothing.
///
/// The tap-to-open idiom: tapping the thing that is open shuts it, tapping
/// anything else opens that instead. Written once here because it was
/// written eleven times as `field = (field != Some(x)).then_some(x)` in one
/// game, and a rule spelled eleven times is a rule one of them gets wrong.
///
/// ```
/// use jidousha_ui::toggle;
///
/// let mut board: Option<usize> = None;
/// toggle(&mut board, 3);
/// assert_eq!(board, Some(3));
/// toggle(&mut board, 5);
/// assert_eq!(board, Some(5), "another site's marker replaces the board");
/// toggle(&mut board, 5);
/// assert_eq!(board, None, "the same marker again puts it down");
/// ```
pub fn toggle<T: PartialEq>(slot: &mut Option<T>, value: T) {
    *slot = (slot.as_ref() != Some(&value)).then_some(value);
}

/// A chip whose explanation is showing, by id — or no chip.
///
/// A chip is a word on a surface — a trait, a verdict, a declared
/// consequence — that, tapped, shows a line explaining it. The line is
/// **derived at draw time** from the row the id names (`line`), never
/// stored, so a renamed row renames every explanation on screen; and the
/// state is the one id, so the same chip tapped on two surfaces is the same
/// chip, and a chip cannot be left lit for a card that has gone.
///
/// ```
/// use jidousha_ui::Chip;
///
/// #[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// enum Trait { Caring, Restless }
/// fn explain(id: Trait) -> String {
///     match id {
///         Trait::Caring => "somebody else's trouble".to_owned(),
///         Trait::Restless => "wanting to be elsewhere".to_owned(),
///     }
/// }
///
/// let mut chip = Chip::default();
/// assert_eq!(chip.line(explain), None);
/// chip.toggle(Trait::Caring);
/// assert!(chip.showing(Trait::Caring));
/// assert!(!chip.showing(Trait::Restless));
/// assert_eq!(chip.line(explain).as_deref(), Some("somebody else's trouble"));
/// chip.toggle(Trait::Caring);
/// assert_eq!(chip.lit, None, "tapped again, it goes out");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chip<Id> {
    /// The id whose explanation is showing, if one is.
    pub lit: Option<Id>,
}

impl<Id> Default for Chip<Id> {
    /// No chip lit — what every surface opens with.
    fn default() -> Self {
        Self { lit: None }
    }
}

impl<Id: Copy + PartialEq> Chip<Id> {
    /// Tap `id`: light it, or put it out if it is the one lit.
    pub fn toggle(&mut self, id: Id) {
        toggle(&mut self.lit, id);
    }

    /// Whether `id` is the chip whose explanation is showing — what a
    /// surface asks when it draws that chip gold.
    #[must_use]
    pub fn showing(self, id: Id) -> bool {
        self.lit == Some(id)
    }

    /// Put the chip out, whichever it was.
    pub fn shut(&mut self) {
        self.lit = None;
    }

    /// The explanation on screen, derived now from the lit id by `explain` —
    /// the game's own formatter over its own rows.
    #[must_use]
    pub fn line(self, explain: impl FnOnce(Id) -> String) -> Option<String> {
        self.lit.map(explain)
    }
}
