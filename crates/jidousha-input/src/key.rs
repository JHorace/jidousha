//! The keyboard vocabulary: physical keys, and their stable wire codes.
//!
//! Key types: `Key`.
//! Depends on: nothing. Must never depend on: `winit`, or any platform crate —
//! the translation table from platform codes to these lives on the other side
//! of the seam (ADR-0004, input.md §6).
//! INVARIANT: a key's wire code never changes. Codes are written into
//! recordings, and a recording made last month must replay today, so a variant
//! keeps its number forever — new keys take unused ones (input.md §5).

use core::fmt;

/// Defines `Key` once, and derives everything that must not drift from it.
///
/// The enum, the wire codes both ways, the printable name, and the list of
/// every key are four views of one table. Written by hand they would be four
/// lists to keep in step; here, adding a line adds the key everywhere.
macro_rules! keys {
    ($($name:ident = $code:literal),* $(,)?) => {
        /// A physical key, by position on the keyboard rather than by the
        /// letter printed on it.
        ///
        /// WASD is WASD on AZERTY: the key left of `S` is [`Key::A`] whatever
        /// the layout says it types. This is what games want, and one key model
        /// is one way to do it (input.md §2). Typing a name into a text box is
        /// a different problem, deliberately not solved here.
        ///
        /// The set covers letters, digits, arrows, the common editing and
        /// modifier keys, F1–F12, and ASCII punctuation. Keys outside it — the
        /// numpad, media keys, anything a particular keyboard invents — are
        /// dropped at the platform boundary rather than guessed at. That is a
        /// documented boundary of the v1 enum, not a silent failure: extending
        /// this list is an ordinary additive change.
        ///
        /// `Super` is the Windows key, the Command key, and the Meta key: one
        /// physical position, three names, depending on whose keyboard it is.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Key {
            $(
                #[doc = concat!("The `", stringify!($name), "` key.")]
                $name,
            )*
        }

        impl Key {
            /// Every key, in declaration order.
            ///
            /// Exists for tests and for tooling that enumerates the vocabulary;
            /// game code names the key it means.
            pub const ALL: &'static [Key] = &[$(Key::$name),*];

            /// This key's wire code, as written into recordings.
            #[must_use]
            pub fn code(self) -> u16 {
                match self {
                    $(Key::$name => $code),*
                }
            }

            /// The key a wire code names, or `None` if this build has never
            /// heard of it — a recording from a newer engine, most likely.
            #[must_use]
            pub fn find_by_code(code: u16) -> Option<Key> {
                match code {
                    $($code => Some(Key::$name),)*
                    _ => None,
                }
            }

            /// The variant's name, for messages and for `input_echo` (I1).
            #[must_use]
            pub fn name(self) -> &'static str {
                match self {
                    $(Key::$name => stringify!($name)),*
                }
            }
        }
    };
}

keys! {
    // Letters, 1..=26, in alphabetical order.
    A = 1, B = 2, C = 3, D = 4, E = 5, F = 6, G = 7, H = 8, I = 9,
    J = 10, K = 11, L = 12, M = 13, N = 14, O = 15, P = 16, Q = 17, R = 18,
    S = 19, T = 20, U = 21, V = 22, W = 23, X = 24, Y = 25, Z = 26,

    // The digit row. Not the numpad, which v1 does not carry.
    Digit0 = 30, Digit1 = 31, Digit2 = 32, Digit3 = 33, Digit4 = 34,
    Digit5 = 35, Digit6 = 36, Digit7 = 37, Digit8 = 38, Digit9 = 39,

    ArrowUp = 40, ArrowDown = 41, ArrowLeft = 42, ArrowRight = 43,

    Space = 50, Enter = 51, Escape = 52, Tab = 53, Backspace = 54,
    Delete = 55, Insert = 56, Home = 57, End = 58, PageUp = 59, PageDown = 60,

    ShiftLeft = 70, ShiftRight = 71, ControlLeft = 72, ControlRight = 73,
    AltLeft = 74, AltRight = 75, SuperLeft = 76, SuperRight = 77, CapsLock = 78,

    F1 = 80, F2 = 81, F3 = 82, F4 = 83, F5 = 84, F6 = 85,
    F7 = 86, F8 = 87, F9 = 88, F10 = 89, F11 = 90, F12 = 91,

    Minus = 100, Equal = 101, BracketLeft = 102, BracketRight = 103,
    Backslash = 104, Semicolon = 105, Quote = 106, Backquote = 107,
    Comma = 108, Period = 109, Slash = 110,
}

impl fmt::Display for Key {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_key_round_trips_through_its_wire_code() {
        for &key in Key::ALL {
            assert_eq!(Key::find_by_code(key.code()), Some(key), "{key}");
        }
    }

    #[test]
    fn no_two_keys_share_a_wire_code() {
        // The codes are hand-assigned, which is what makes them stable and also
        // what makes a collision possible to typo into existence.
        let codes: BTreeSet<u16> = Key::ALL.iter().map(|key| key.code()).collect();
        assert_eq!(codes.len(), Key::ALL.len());
    }

    #[test]
    fn an_unknown_wire_code_is_not_a_key() {
        assert_eq!(Key::find_by_code(0), None);
        assert_eq!(Key::find_by_code(u16::MAX), None);
    }

    #[test]
    fn a_key_prints_as_its_variant_name() {
        assert_eq!(Key::ArrowLeft.to_string(), "ArrowLeft");
        assert_eq!(Key::Digit7.to_string(), "Digit7");
    }
}
