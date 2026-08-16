//! The naive reference the edge rules are checked against, and the generator
//! that drives them both.
//!
//! Key types: `Step`, `Reference`, `Rng`.
//! Depends on: `jidousha_input`'s public API only.
//! INVARIANT: the reference is written to be *obviously* right, never
//! efficient — `BTreeSet`s and whole-set rebuilds, where the real builder keeps
//! sorted `Vec`s and incremental edge lists. When the two disagree, the
//! reference is the one that is easy to read (ADR-0006).
//! INVARIANT: the generator's RNG is this file's own, not the engine's. A test
//! that drew its randomness from the engine would go quiet in exactly the case
//! where the engine's RNG broke.

// Each integration test compiles the whole of this module while using part of
// it; the unused warnings are an artifact of that, not a signal about the code.
#![allow(dead_code)]

use std::collections::BTreeSet;

use jidousha_core::math::Vec2;
use jidousha_input::{InputEvent, InputSnapshot, Key, PointerButton, PointerId};

/// The keys a generated stream draws from.
///
/// A handful, not all eighty-three: collisions are the interesting case, and a
/// small alphabet makes a random stream press the same key twice in one frame
/// often enough to matter.
pub const ALPHABET: [Key; 5] = [Key::A, Key::D, Key::W, Key::S, Key::Space];

/// One thing the driver does, in order.
#[derive(Clone, Copy, Debug)]
pub enum Step {
    /// The platform reported an event.
    Event(InputEvent),
    /// A frame boundary: the loop ran this many Update ticks.
    ///
    /// Zero is the interesting one — a frame whose accumulator had not filled
    /// yet. Its events are not lost; they belong to whichever tick runs next.
    Frame { ticks: usize },
}

/// The naive model: physical state, rebuilt from scratch, with the edge rules
/// stated as plainly as they can be.
pub struct Reference {
    /// Keys physically down right now.
    down: BTreeSet<Key>,
    /// Buttons physically down right now.
    buttons_down: BTreeSet<PointerButton>,
    /// Keys that went down since the last emitted first tick.
    pressed: BTreeSet<Key>,
    /// Keys that came up since the last emitted first tick.
    released: BTreeSet<Key>,
    buttons_pressed: BTreeSet<PointerButton>,
    buttons_released: BTreeSet<PointerButton>,
    screen: Vec2,
    scroll: f32,
    focused: bool,
}

impl Reference {
    pub fn new() -> Self {
        Self {
            down: BTreeSet::new(),
            buttons_down: BTreeSet::new(),
            pressed: BTreeSet::new(),
            released: BTreeSet::new(),
            buttons_pressed: BTreeSet::new(),
            buttons_released: BTreeSet::new(),
            screen: Vec2::ZERO,
            scroll: 0.0,
            focused: true,
        }
    }

    pub fn record(&mut self, event: InputEvent) {
        match event {
            InputEvent::KeyPressed(key) => {
                self.pressed.insert(key);
                self.down.insert(key);
            }
            InputEvent::KeyReleased(key) => {
                self.released.insert(key);
                self.down.remove(&key);
            }
            InputEvent::PointerMoved { screen, .. } => self.screen = screen,
            InputEvent::ButtonPressed { button, .. } => {
                self.buttons_pressed.insert(button);
                self.buttons_down.insert(button);
            }
            InputEvent::ButtonReleased { button, .. } => {
                self.buttons_released.insert(button);
                self.buttons_down.remove(&button);
            }
            InputEvent::Scrolled { lines, .. } => self.scroll += lines,
            InputEvent::FocusLost => {
                self.focused = false;
                // Everything down is released for the player, so they do not
                // come back to a character still running left.
                self.released.extend(std::mem::take(&mut self.down));
                self.buttons_released
                    .extend(std::mem::take(&mut self.buttons_down));
            }
            InputEvent::FocusGained => self.focused = true,
        }
    }

    /// What the frame's first tick should see, spending the frame's edges.
    pub fn first_tick(&mut self) -> Expected {
        // A key tapped inside this frame is down for this one tick: press
        // without held would be a state no game expects.
        let held: BTreeSet<Key> = self.down.union(&self.pressed).copied().collect();
        let buttons_held: BTreeSet<PointerButton> = self
            .buttons_down
            .union(&self.buttons_pressed)
            .copied()
            .collect();
        let expected = Expected {
            held,
            pressed: std::mem::take(&mut self.pressed),
            released: std::mem::take(&mut self.released),
            buttons_held,
            buttons_pressed: std::mem::take(&mut self.buttons_pressed),
            buttons_released: std::mem::take(&mut self.buttons_released),
            screen: self.screen,
            scroll: self.scroll,
            focused: self.focused,
        };
        self.scroll = 0.0;
        expected
    }

    /// What a second or later tick of the same frame should see.
    pub fn catch_up_tick(&self) -> Expected {
        Expected {
            held: self.down.clone(),
            pressed: BTreeSet::new(),
            released: BTreeSet::new(),
            buttons_held: self.buttons_down.clone(),
            buttons_pressed: BTreeSet::new(),
            buttons_released: BTreeSet::new(),
            screen: self.screen,
            scroll: 0.0,
            focused: self.focused,
        }
    }
}

/// One tick as the model says it should look.
#[derive(Clone, Debug, PartialEq)]
pub struct Expected {
    pub held: BTreeSet<Key>,
    pub pressed: BTreeSet<Key>,
    pub released: BTreeSet<Key>,
    pub buttons_held: BTreeSet<PointerButton>,
    pub buttons_pressed: BTreeSet<PointerButton>,
    pub buttons_released: BTreeSet<PointerButton>,
    pub screen: Vec2,
    pub scroll: f32,
    pub focused: bool,
}

impl Expected {
    /// The same view, read off a real snapshot.
    pub fn of(snapshot: &InputSnapshot) -> Self {
        let pointer = &snapshot.pointers()[0];
        Self {
            held: snapshot.held_keys().iter().copied().collect(),
            pressed: snapshot.pressed_keys().iter().copied().collect(),
            released: snapshot.released_keys().iter().copied().collect(),
            buttons_held: pointer.held_buttons().iter().copied().collect(),
            buttons_pressed: pointer.pressed_buttons().iter().copied().collect(),
            buttons_released: pointer.released_buttons().iter().copied().collect(),
            screen: pointer.screen,
            scroll: pointer.scroll,
            focused: snapshot.window_focused(),
        }
    }
}

/// The generator's own RNG — SplitMix64, short enough to read.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 32) as u32
    }

    pub fn below(&mut self, limit: u32) -> u32 {
        self.next_u32() % limit
    }
}

/// Build one driver script: events interleaved with frame boundaries.
///
/// Weighted so that presses and releases of the same small alphabet collide
/// inside single frames, taps happen, frames sometimes run no ticks at all, and
/// focus is lost often enough that the synthesized releases are exercised
/// rather than merely present.
pub fn generate(seed: u64, length: usize) -> Vec<Step> {
    let mut rng = Rng::new(seed);
    let mut steps = Vec::with_capacity(length);
    for _ in 0..length {
        let step = match rng.below(100) {
            0..=29 => Step::Event(InputEvent::KeyPressed(pick_key(&mut rng))),
            30..=54 => Step::Event(InputEvent::KeyReleased(pick_key(&mut rng))),
            55..=59 => Step::Event(InputEvent::ButtonPressed {
                id: PointerId::PRIMARY,
                button: pick_button(&mut rng),
            }),
            60..=64 => Step::Event(InputEvent::ButtonReleased {
                id: PointerId::PRIMARY,
                button: pick_button(&mut rng),
            }),
            65..=68 => Step::Event(InputEvent::PointerMoved {
                id: PointerId::PRIMARY,
                screen: Vec2::new(rng.below(800) as f32, rng.below(600) as f32),
            }),
            69..=71 => Step::Event(InputEvent::Scrolled {
                id: PointerId::PRIMARY,
                lines: rng.below(5) as f32 - 2.0,
            }),
            72..=74 => Step::Event(InputEvent::FocusLost),
            75..=77 => Step::Event(InputEvent::FocusGained),
            // Frames, including the occasional one that runs no tick at all and
            // the occasional one that runs several catch-up ticks.
            _ => Step::Frame {
                ticks: (rng.below(10) as usize).min(4),
            },
        };
        steps.push(step);
    }
    steps
}

fn pick_key(rng: &mut Rng) -> Key {
    ALPHABET[rng.below(ALPHABET.len() as u32) as usize]
}

fn pick_button(rng: &mut Rng) -> PointerButton {
    PointerButton::ALL[rng.below(PointerButton::ALL.len() as u32) as usize]
}
