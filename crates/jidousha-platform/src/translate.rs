//! The only place winit's input vocabulary and the engine's both appear.
//!
//! Key types: `key`, `button`, `scroll_lines`, `key_event`.
//! Depends on: `winit`, `jidousha-input`.
//! INVARIANT (ADR-0004, input.md §6 CONTRACT): no winit type leaves this
//! module. Everything above it speaks `InputEvent`, which is why the edge rules,
//! the focus policy and the snapshot codec are all testable on a machine with no
//! window — and on wasm CI, where there is no window to be had.
//! INVARIANT: translation is total and lossy in one direction only. A key this
//! build does not name is *dropped*, never guessed at; that is a documented
//! boundary of the `Key` enum rather than a silent failure (input.md §2).

use jidousha_input::{InputEvent, Key, PointerButton, PointerId};
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::keyboard::{KeyCode, PhysicalKey};

/// How many pixels of scroll make one line.
///
/// Only browsers and precision touchpads report scroll in pixels; a wheel on
/// Linux or Windows reports lines directly and needs no conversion at all. So
/// this number has exactly one job: make a *browser* wheel notch feel like a
/// native one. Browsers report a notch as 100 pixels and winit reports a native
/// notch as one line, so one line is a hundred pixels and the same flick of the
/// same finger means the same thing on all three targets.
///
/// DELIBERATE: a heuristic, and knowingly so. The honest version asks the
/// platform for its real line height, which winit does not expose. Whatever
/// this produces is what gets *recorded*, so a later improvement changes new
/// recordings and leaves old ones replaying exactly as they did (input.md §3).
const PIXELS_PER_LINE: f32 = 100.0;

/// The engine's key for a winit physical key code, if this build has one.
///
/// Physical, never logical: the key left of `S` is [`Key::A`] whatever an AZERTY
/// keyboard types with it (input.md §2). The `match` is written out rather than
/// computed because the two enums are two vocabularies that happen to agree —
/// any cleverness here would be a coincidence waiting to break.
pub(crate) fn key(code: KeyCode) -> Option<Key> {
    Some(match code {
        KeyCode::KeyA => Key::A,
        KeyCode::KeyB => Key::B,
        KeyCode::KeyC => Key::C,
        KeyCode::KeyD => Key::D,
        KeyCode::KeyE => Key::E,
        KeyCode::KeyF => Key::F,
        KeyCode::KeyG => Key::G,
        KeyCode::KeyH => Key::H,
        KeyCode::KeyI => Key::I,
        KeyCode::KeyJ => Key::J,
        KeyCode::KeyK => Key::K,
        KeyCode::KeyL => Key::L,
        KeyCode::KeyM => Key::M,
        KeyCode::KeyN => Key::N,
        KeyCode::KeyO => Key::O,
        KeyCode::KeyP => Key::P,
        KeyCode::KeyQ => Key::Q,
        KeyCode::KeyR => Key::R,
        KeyCode::KeyS => Key::S,
        KeyCode::KeyT => Key::T,
        KeyCode::KeyU => Key::U,
        KeyCode::KeyV => Key::V,
        KeyCode::KeyW => Key::W,
        KeyCode::KeyX => Key::X,
        KeyCode::KeyY => Key::Y,
        KeyCode::KeyZ => Key::Z,

        KeyCode::Digit0 => Key::Digit0,
        KeyCode::Digit1 => Key::Digit1,
        KeyCode::Digit2 => Key::Digit2,
        KeyCode::Digit3 => Key::Digit3,
        KeyCode::Digit4 => Key::Digit4,
        KeyCode::Digit5 => Key::Digit5,
        KeyCode::Digit6 => Key::Digit6,
        KeyCode::Digit7 => Key::Digit7,
        KeyCode::Digit8 => Key::Digit8,
        KeyCode::Digit9 => Key::Digit9,

        KeyCode::ArrowUp => Key::ArrowUp,
        KeyCode::ArrowDown => Key::ArrowDown,
        KeyCode::ArrowLeft => Key::ArrowLeft,
        KeyCode::ArrowRight => Key::ArrowRight,

        KeyCode::Space => Key::Space,
        KeyCode::Enter => Key::Enter,
        KeyCode::Escape => Key::Escape,
        KeyCode::Tab => Key::Tab,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::Insert => Key::Insert,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,

        KeyCode::ShiftLeft => Key::ShiftLeft,
        KeyCode::ShiftRight => Key::ShiftRight,
        KeyCode::ControlLeft => Key::ControlLeft,
        KeyCode::ControlRight => Key::ControlRight,
        KeyCode::AltLeft => Key::AltLeft,
        KeyCode::AltRight => Key::AltRight,
        KeyCode::SuperLeft => Key::SuperLeft,
        KeyCode::SuperRight => Key::SuperRight,
        KeyCode::CapsLock => Key::CapsLock,

        KeyCode::F1 => Key::F1,
        KeyCode::F2 => Key::F2,
        KeyCode::F3 => Key::F3,
        KeyCode::F4 => Key::F4,
        KeyCode::F5 => Key::F5,
        KeyCode::F6 => Key::F6,
        KeyCode::F7 => Key::F7,
        KeyCode::F8 => Key::F8,
        KeyCode::F9 => Key::F9,
        KeyCode::F10 => Key::F10,
        KeyCode::F11 => Key::F11,
        KeyCode::F12 => Key::F12,

        KeyCode::Minus => Key::Minus,
        KeyCode::Equal => Key::Equal,
        KeyCode::BracketLeft => Key::BracketLeft,
        KeyCode::BracketRight => Key::BracketRight,
        KeyCode::Backslash => Key::Backslash,
        KeyCode::Semicolon => Key::Semicolon,
        KeyCode::Quote => Key::Quote,
        KeyCode::Backquote => Key::Backquote,
        KeyCode::Comma => Key::Comma,
        KeyCode::Period => Key::Period,
        KeyCode::Slash => Key::Slash,

        // The numpad, media keys, and everything a particular keyboard invents.
        // Dropped rather than guessed at — see the module header.
        _ => return None,
    })
}

/// The engine's button for a winit mouse button, if it has one.
///
/// Named by role: the operating system has already swapped left and right for a
/// left-handed user by the time winit sees it, so `Left` really is "the button
/// you click with" (input.md §3).
pub(crate) fn button(button: MouseButton) -> Option<PointerButton> {
    Some(match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Middle,
        // Back, Forward, and whatever `Other` names. v1 has three buttons.
        _ => return None,
    })
}

/// Scroll for one wheel event, in lines, positive toward the end of a document.
///
/// Two things happen here. Pixel deltas become lines, and the sign is flipped:
/// winit's positive means "the content moves down", which is scrolling *back*,
/// and a game asking for `scroll` means the direction Page Down goes.
pub(crate) fn scroll_lines(delta: MouseScrollDelta) -> f32 {
    match delta {
        // Horizontal scroll is dropped: `PointerState::scroll` is one number,
        // and v1 has no game that wants two (input.md §3).
        MouseScrollDelta::LineDelta(_, lines) => -lines,
        MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => -(y as f32) / PIXELS_PER_LINE,
    }
}

/// Where a cursor moved to, in pixels from the window's top-left.
pub(crate) fn pointer_moved(position: PhysicalPosition<f64>) -> InputEvent {
    InputEvent::PointerMoved {
        id: PointerId::PRIMARY,
        screen: jidousha_core::math::Vec2::new(position.x as f32, position.y as f32),
    }
}

/// The engine event for one winit key event, if there is one to make.
///
/// Two winit events are deliberately dropped here, and both would otherwise
/// produce edges no player made:
///
/// - **Auto-repeat.** Holding a key makes the operating system send a press
///   every few tens of milliseconds. Passing those on would fire
///   `just_pressed` about thirty times a second for a held key, which is the
///   single most common way this translation goes wrong.
/// - **Synthetic events.** winit reports the keys that were down when a window
///   gained or lost focus. The engine already synthesizes its own release edges
///   on focus loss, above this seam and under test (input.md §4), so accepting
///   winit's too would double every release — and on the way back in, would
///   re-press keys the player is no longer holding.
pub(crate) fn key_event(
    physical_key: PhysicalKey,
    state: ElementState,
    repeat: bool,
    is_synthetic: bool,
) -> Option<InputEvent> {
    if repeat || is_synthetic {
        return None;
    }
    let PhysicalKey::Code(code) = physical_key else {
        // `Unidentified`: a key winit itself could not name. Nothing to map to.
        return None;
    };
    let key = key(code)?;
    Some(match state {
        ElementState::Pressed => InputEvent::KeyPressed(key),
        ElementState::Released => InputEvent::KeyReleased(key),
    })
}

/// The engine event for a winit mouse button change, if there is one.
pub(crate) fn button_event(state: ElementState, winit_button: MouseButton) -> Option<InputEvent> {
    let button = button(winit_button)?;
    let id = PointerId::PRIMARY;
    Some(match state {
        ElementState::Pressed => InputEvent::ButtonPressed { id, button },
        ElementState::Released => InputEvent::ButtonReleased { id, button },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_the_engine_names_can_arrive_from_winit() {
        // The direction that matters: a `Key` variant nothing translates to is a
        // key a game can ask about and never receive — `held(Key::F7)` quietly
        // false forever. Checking the table covers the enum is what makes the
        // enum a promise rather than a wish.
        let reachable: Vec<Key> = ALL_CODES.iter().filter_map(|code| key(*code)).collect();
        let missing: Vec<&Key> = Key::ALL
            .iter()
            .filter(|wanted| !reachable.contains(wanted))
            .collect();
        assert!(missing.is_empty(), "no winit code maps to {missing:?}");
    }

    #[test]
    fn no_two_winit_keys_translate_to_the_same_engine_key() {
        // A duplicate would make two physical keys indistinguishable, and the
        // one written second would be dead — the sort of typo a table this long
        // invites and nothing else would notice.
        let mut seen: Vec<Key> = ALL_CODES.iter().filter_map(|code| key(*code)).collect();
        let before = seen.len();
        seen.sort();
        seen.dedup();
        assert_eq!(before, seen.len(), "two codes map to one key");
    }

    #[test]
    fn letters_are_translated_by_position_and_not_by_name() {
        // The whole point of physical keys: this must hold before any layout is
        // consulted, which is what makes WASD work on AZERTY (input.md §2).
        assert_eq!(key(KeyCode::KeyW), Some(Key::W));
        assert_eq!(key(KeyCode::KeyA), Some(Key::A));
        assert_eq!(key(KeyCode::KeyS), Some(Key::S));
        assert_eq!(key(KeyCode::KeyD), Some(Key::D));
    }

    #[test]
    fn a_key_this_build_does_not_name_is_dropped_rather_than_guessed() {
        assert_eq!(key(KeyCode::Numpad0), None);
        assert_eq!(key(KeyCode::MediaPlayPause), None);
        assert_eq!(key(KeyCode::PrintScreen), None);
    }

    #[test]
    fn the_three_buttons_map_and_the_rest_are_dropped() {
        assert_eq!(button(MouseButton::Left), Some(PointerButton::Primary));
        assert_eq!(button(MouseButton::Right), Some(PointerButton::Secondary));
        assert_eq!(button(MouseButton::Middle), Some(PointerButton::Middle));
        assert_eq!(button(MouseButton::Back), None);
        assert_eq!(button(MouseButton::Other(9)), None);
    }

    #[test]
    fn auto_repeat_does_not_reach_the_engine() {
        // The bug this exists to prevent: a held key firing `just_pressed`
        // thirty times a second, because the operating system helpfully repeats
        // it for text editors that want that.
        let held = key_event(
            PhysicalKey::Code(KeyCode::KeyD),
            ElementState::Pressed,
            true,
            false,
        );
        assert_eq!(held, None);
        let real = key_event(
            PhysicalKey::Code(KeyCode::KeyD),
            ElementState::Pressed,
            false,
            false,
        );
        assert_eq!(real, Some(InputEvent::KeyPressed(Key::D)));
    }

    #[test]
    fn winits_synthetic_focus_events_do_not_reach_the_engine() {
        // The engine synthesizes its own releases on focus loss, above this
        // seam and under test. Taking winit's as well would release every held
        // key twice on the way out and press them all again on the way back.
        for state in [ElementState::Pressed, ElementState::Released] {
            assert_eq!(
                key_event(PhysicalKey::Code(KeyCode::KeyA), state, false, true),
                None
            );
        }
    }

    #[test]
    fn an_unidentified_key_is_dropped() {
        use winit::keyboard::NativeKeyCode;
        assert_eq!(
            key_event(
                PhysicalKey::Unidentified(NativeKeyCode::Unidentified),
                ElementState::Pressed,
                false,
                false,
            ),
            None
        );
    }

    #[test]
    fn a_release_translates_to_a_release() {
        assert_eq!(
            key_event(
                PhysicalKey::Code(KeyCode::Space),
                ElementState::Released,
                false,
                false
            ),
            Some(InputEvent::KeyReleased(Key::Space)),
        );
        assert_eq!(
            button_event(ElementState::Released, MouseButton::Left),
            Some(InputEvent::ButtonReleased {
                id: PointerId::PRIMARY,
                button: PointerButton::Primary,
            }),
        );
    }

    #[test]
    fn one_wheel_notch_is_one_line_however_the_platform_reports_it() {
        // The reason `PIXELS_PER_LINE` exists: a native wheel says "one line"
        // and a browser wheel says "one hundred pixels", and the same flick has
        // to mean the same thing on both.
        let native = scroll_lines(MouseScrollDelta::LineDelta(0.0, 1.0));
        let browser = scroll_lines(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
            0.0, 100.0,
        )));
        assert!((native - browser).abs() < 1e-6, "{native} vs {browser}");
    }

    #[test]
    fn scrolling_toward_the_end_of_a_document_is_positive() {
        // winit's positive means "the content moves down", which is scrolling
        // *back*. A game asking for `scroll` means the way Page Down goes, so
        // the sign flips here and nowhere else.
        assert!(scroll_lines(MouseScrollDelta::LineDelta(0.0, -1.0)) > 0.0);
        assert!(scroll_lines(MouseScrollDelta::LineDelta(0.0, 1.0)) < 0.0);
        assert!(
            scroll_lines(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                0.0, -100.0
            ))) > 0.0
        );
    }

    #[test]
    fn horizontal_scroll_is_dropped_rather_than_folded_in() {
        // Folding it into one number would make a sideways swipe zoom the game.
        assert_eq!(scroll_lines(MouseScrollDelta::LineDelta(5.0, 0.0)), 0.0);
        assert_eq!(
            scroll_lines(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
                500.0, 0.0
            ))),
            0.0
        );
    }

    #[test]
    fn a_cursor_position_arrives_in_pixels_from_the_top_left() {
        assert_eq!(
            pointer_moved(PhysicalPosition::new(12.5, 30.0)),
            InputEvent::PointerMoved {
                id: PointerId::PRIMARY,
                screen: jidousha_core::math::Vec2::new(12.5, 30.0),
            }
        );
    }

    /// Every `KeyCode` this build translates, for the coverage tests above.
    ///
    /// Written out rather than iterated: winit's `KeyCode` has no `ALL`, and a
    /// list that is missing an entry can only make the tests *weaker*, never
    /// wrong — while the tests it feeds are what catch a missing or duplicated
    /// row in the table itself.
    const ALL_CODES: &[KeyCode] = &[
        KeyCode::KeyA,
        KeyCode::KeyB,
        KeyCode::KeyC,
        KeyCode::KeyD,
        KeyCode::KeyE,
        KeyCode::KeyF,
        KeyCode::KeyG,
        KeyCode::KeyH,
        KeyCode::KeyI,
        KeyCode::KeyJ,
        KeyCode::KeyK,
        KeyCode::KeyL,
        KeyCode::KeyM,
        KeyCode::KeyN,
        KeyCode::KeyO,
        KeyCode::KeyP,
        KeyCode::KeyQ,
        KeyCode::KeyR,
        KeyCode::KeyS,
        KeyCode::KeyT,
        KeyCode::KeyU,
        KeyCode::KeyV,
        KeyCode::KeyW,
        KeyCode::KeyX,
        KeyCode::KeyY,
        KeyCode::KeyZ,
        KeyCode::Digit0,
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
        KeyCode::Space,
        KeyCode::Enter,
        KeyCode::Escape,
        KeyCode::Tab,
        KeyCode::Backspace,
        KeyCode::Delete,
        KeyCode::Insert,
        KeyCode::Home,
        KeyCode::End,
        KeyCode::PageUp,
        KeyCode::PageDown,
        KeyCode::ShiftLeft,
        KeyCode::ShiftRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::AltLeft,
        KeyCode::AltRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
        KeyCode::CapsLock,
        KeyCode::F1,
        KeyCode::F2,
        KeyCode::F3,
        KeyCode::F4,
        KeyCode::F5,
        KeyCode::F6,
        KeyCode::F7,
        KeyCode::F8,
        KeyCode::F9,
        KeyCode::F10,
        KeyCode::F11,
        KeyCode::F12,
        KeyCode::Minus,
        KeyCode::Equal,
        KeyCode::BracketLeft,
        KeyCode::BracketRight,
        KeyCode::Backslash,
        KeyCode::Semicolon,
        KeyCode::Quote,
        KeyCode::Backquote,
        KeyCode::Comma,
        KeyCode::Period,
        KeyCode::Slash,
    ];
}
