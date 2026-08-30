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

use std::collections::{BTreeSet, VecDeque};

use jidousha_core::math::Vec2;
use jidousha_input::{
    FingerId, InputEvent, InputSnapshot, Key, MAX_TOUCHES, PointerButton, PointerId, TouchPhase,
};

/// The fingers a generated stream draws from.
///
/// One more than the format has slots for, so the fifth finger — the one the
/// engine drops on purpose — actually happens.
pub const FINGERS: usize = MAX_TOUCHES + 2;

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

/// One finger the model is following, from the tick it lands to the tick its
/// end is reported.
///
/// DELIBERATE: a **queue of owed phases** rather than the builder's "the phase
/// to report, plus an end that may be owed behind it". The two say the same
/// thing and they say it differently, which is the only reason a reference
/// model is worth having: a queue makes "each phase is reported once, in
/// order" true by construction rather than by argument.
#[derive(Clone, Debug)]
struct Finger {
    /// The platform's name for it.
    finger: FingerId,
    /// Which slot it took when it landed.
    slot: u8,
    /// Where it was last heard from.
    screen: Vec2,
    /// Phases not yet reported. A finger with nothing owed is simply down, and
    /// a down finger reports `Moved`.
    owed: VecDeque<TouchPhase>,
}

impl Finger {
    /// Whether this finger's end has already been decided.
    fn ending(&self) -> bool {
        self.owed.iter().any(|phase| phase.is_final())
    }
}

/// The naive model: physical state, rebuilt from scratch, with the edge rules
/// stated as plainly as they can be.
pub struct Reference {
    /// Fingers on the glass, in the order they landed.
    fingers: Vec<Finger>,
    /// The slot whose finger is driving the primary pointer, if any.
    mirror: Option<u8>,
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
            fingers: Vec::new(),
            mirror: None,
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
            InputEvent::Touched {
                finger,
                phase,
                screen,
            } => self.touch(finger, phase, screen),
            InputEvent::Scrolled { lines, .. } => self.scroll += lines,
            InputEvent::FocusLost => {
                self.focused = false;
                // Everything down is released for the player, so they do not
                // come back to a character still running left.
                self.released.extend(std::mem::take(&mut self.down));
                self.buttons_released
                    .extend(std::mem::take(&mut self.buttons_down));
                // Fingers are cancelled, not lifted: what the window knows is
                // that it stopped being told about them.
                for index in 0..self.fingers.len() {
                    if !self.fingers[index].ending() {
                        self.fingers[index].owed.push_back(TouchPhase::Cancelled);
                    }
                }
                self.mirror = None;
            }
            InputEvent::FocusGained => self.focused = true,
        }
    }

    /// One touch event, and the mirror it may move.
    fn touch(&mut self, finger: FingerId, phase: TouchPhase, screen: Vec2) {
        let known = self.fingers.iter().position(|held| held.finger == finger);
        let index = match (phase, known) {
            // A finger that lands twice is one finger, and a slot it cannot
            // have is a finger the engine never hears about again.
            (TouchPhase::Began, Some(_)) => return,
            (TouchPhase::Began, None) => {
                let taken: BTreeSet<u8> = self.fingers.iter().map(|held| held.slot).collect();
                let Some(slot) = (0..MAX_TOUCHES as u8).find(|slot| !taken.contains(slot)) else {
                    return;
                };
                self.fingers.push(Finger {
                    finger,
                    slot,
                    screen,
                    owed: VecDeque::from([TouchPhase::Began]),
                });
                self.fingers.len() - 1
            }
            // Anything about a finger that is not down, or that is already on
            // its way out, is not an event this engine has anything to say
            // about.
            (_, None) => return,
            (_, Some(index)) if self.fingers[index].ending() => return,
            (_, Some(index)) => index,
        };
        self.fingers[index].screen = screen;
        if phase.is_final() {
            self.fingers[index].owed.push_back(phase);
        }

        // The mirror: first active touch wins, and does not hand over.
        let slot = self.fingers[index].slot;
        match phase {
            TouchPhase::Began if self.mirror.is_none() => {
                self.mirror = Some(slot);
                self.screen = screen;
                self.buttons_pressed.insert(PointerButton::Primary);
                self.buttons_down.insert(PointerButton::Primary);
            }
            _ if self.mirror != Some(slot) => {}
            TouchPhase::Began | TouchPhase::Moved => self.screen = screen,
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.screen = screen;
                self.buttons_released.insert(PointerButton::Primary);
                self.buttons_down.remove(&PointerButton::Primary);
                self.mirror = None;
            }
        }
    }

    /// The touches of a first tick, spending one owed phase each.
    fn spend_touches(&mut self) -> Vec<(u8, TouchPhase, Vec2)> {
        let mut out = Vec::new();
        for held in &mut self.fingers {
            let phase = held.owed.pop_front().unwrap_or(TouchPhase::Moved);
            out.push((held.slot, phase, held.screen));
        }
        self.fingers.retain(|held| {
            !out.iter()
                .any(|(slot, phase, _)| *slot == held.slot && phase.is_final())
        });
        out.sort_by_key(|(slot, _, _)| *slot);
        out
    }

    /// The touches of a catch-up tick: every finger still being followed,
    /// carrying no edge.
    fn touch_state(&self) -> Vec<(u8, TouchPhase, Vec2)> {
        let mut out: Vec<(u8, TouchPhase, Vec2)> = self
            .fingers
            .iter()
            .map(|held| (held.slot, TouchPhase::Moved, held.screen))
            .collect();
        out.sort_by_key(|(slot, _, _)| *slot);
        out
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
            touches: self.spend_touches(),
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
            touches: self.touch_state(),
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
    /// Every touch this tick: slot, phase, position.
    pub touches: Vec<(u8, TouchPhase, Vec2)>,
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
            touches: snapshot
                .touches()
                .iter()
                .map(|touch| (touch.id.slot(), touch.phase, touch.screen))
                .collect(),
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
            0..=24 => Step::Event(InputEvent::KeyPressed(pick_key(&mut rng))),
            25..=44 => Step::Event(InputEvent::KeyReleased(pick_key(&mut rng))),
            45..=48 => Step::Event(InputEvent::ButtonPressed {
                id: PointerId::PRIMARY,
                button: pick_button(&mut rng),
            }),
            49..=52 => Step::Event(InputEvent::ButtonReleased {
                id: PointerId::PRIMARY,
                button: pick_button(&mut rng),
            }),
            53..=56 => Step::Event(InputEvent::PointerMoved {
                id: PointerId::PRIMARY,
                screen: Vec2::new(rng.below(800) as f32, rng.below(600) as f32),
            }),
            57..=59 => Step::Event(InputEvent::Scrolled {
                id: PointerId::PRIMARY,
                lines: rng.below(5) as f32 - 2.0,
            }),
            // Touches, weighted so that a small pool of fingers lands, moves,
            // lifts and is cancelled in every wrong order as well as the right
            // one: a move for a finger that is not down, a second landing of a
            // finger already down, a fifth finger with no slot to take.
            60..=71 => Step::Event(InputEvent::Touched {
                finger: pick_finger(&mut rng),
                phase: pick_phase(&mut rng),
                screen: Vec2::new(rng.below(800) as f32, rng.below(600) as f32),
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

fn pick_finger(rng: &mut Rng) -> FingerId {
    FingerId::from_platform(u64::from(rng.below(FINGERS as u32)))
}

fn pick_phase(rng: &mut Rng) -> TouchPhase {
    // Weighted toward landing and moving: an even draw would spend most of the
    // stream ending fingers that are not down, which the model and the builder
    // both ignore and which therefore checks nothing.
    match rng.below(10) {
        0..=3 => TouchPhase::Began,
        4..=7 => TouchPhase::Moved,
        8 => TouchPhase::Ended,
        _ => TouchPhase::Cancelled,
    }
}
