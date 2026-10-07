//! The game as the shell sees it: a match, and what the player is in the
//! middle of choosing. Keys go in through `press`; the NPC acts through
//! `npc_step`. Neither knows about the engine, so a check can drive both.

use crate::cards::Side;
use crate::npc;
use crate::rules::{Core, ItemId, Outcome};
use jidousha::prelude::Key;

/// How many ticks the NPC takes to act, so a person can follow it.
pub const NPC_DELAY: u32 = 40;

/// Which key picks which hand slot.
pub const SLOT_KEYS: [Key; 7] = [
    Key::Digit1,
    Key::Digit2,
    Key::Digit3,
    Key::Digit4,
    Key::Digit5,
    Key::Digit6,
    Key::Digit7,
];

/// A targeted card on its way onto the stack: the legal items, and which one
/// the cursor is on.
#[derive(Clone, Debug)]
pub struct Targeting {
    pub hand_index: usize,
    pub legal: Vec<ItemId>,
    pub cursor: usize,
}

impl Targeting {
    pub fn current(&self) -> ItemId {
        self.legal[self.cursor]
    }
}

#[derive(Clone, Debug, Default)]
pub struct Ui {
    pub selected: Option<usize>,
    pub targeting: Option<Targeting>,
    /// The last refusal, in words, for the status line.
    pub message: String,
    pub npc_wait: u32,
}

pub struct Game {
    pub core: Core,
    pub ui: Ui,
    pub seed: u64,
}
impl jidousha::prelude::Resource for Game {}

impl Game {
    pub fn new(seed: u64) -> Game {
        Game {
            core: Core::new(seed),
            ui: Ui::default(),
            seed,
        }
    }

    /// One key press from the player.
    pub fn press(&mut self, key: Key) {
        if self.core.outcome.is_some() {
            if matches!(key, Key::Enter | Key::Space) {
                *self = Game::new(self.seed + 1);
            }
            return;
        }
        if self.ui.targeting.is_some() {
            self.press_while_targeting(key);
            return;
        }
        let hand = self.core.hands[Side::You.index()].len();
        if let Some(slot) = SLOT_KEYS.iter().position(|&k| k == key) {
            if slot < hand {
                self.ui.selected = Some(slot);
                self.ui.message.clear();
            }
            return;
        }
        match key {
            Key::ArrowLeft | Key::ArrowRight if hand > 0 => {
                let at = self.ui.selected.unwrap_or(0);
                let next = if key == Key::ArrowRight {
                    (at + 1) % hand
                } else {
                    (at + hand - 1) % hand
                };
                self.ui.selected = Some(next);
            }
            Key::Enter => self.play_selected(),
            Key::Space => match self.core.try_pass(Side::You) {
                Ok(()) => self.ui.message.clear(),
                Err(error) => self.ui.message = error.to_string(),
            },
            _ => {}
        }
        self.clamp_selection();
    }

    fn clamp_selection(&mut self) {
        let hand = self.core.hands[Side::You.index()].len();
        if self.ui.selected.is_some_and(|s| s >= hand) {
            self.ui.selected = None;
        }
    }

    fn play_selected(&mut self) {
        let Some(hand_index) = self.ui.selected else {
            self.ui.message = "select a card first (keys 1-7)".to_owned();
            return;
        };
        match self.core.options(Side::You, hand_index) {
            Err(error) => self.ui.message = error.to_string(),
            Ok(None) => self.commit(hand_index, None),
            Ok(Some(mut legal)) => {
                // Top of the stack first: the cursor starts on the item that resolves next.
                legal.reverse();
                self.ui.message.clear();
                self.ui.targeting = Some(Targeting {
                    hand_index,
                    legal,
                    cursor: 0,
                });
            }
        }
    }

    fn commit(&mut self, hand_index: usize, target: Option<ItemId>) {
        match self.core.try_play(Side::You, hand_index, target) {
            Ok(_) => {
                self.ui.selected = None;
                self.ui.targeting = None;
                self.ui.message.clear();
            }
            Err(error) => self.ui.message = error.to_string(),
        }
    }

    fn press_while_targeting(&mut self, key: Key) {
        let Some(targeting) = self.ui.targeting.as_mut() else {
            return;
        };
        let n = targeting.legal.len();
        match key {
            Key::ArrowUp => targeting.cursor = (targeting.cursor + n - 1) % n,
            Key::ArrowDown => targeting.cursor = (targeting.cursor + 1) % n,
            Key::Escape => self.ui.targeting = None,
            Key::Enter => {
                let (hand_index, target) = (targeting.hand_index, targeting.current());
                self.commit(hand_index, Some(target));
            }
            _ => {}
        }
    }

    /// One tick of the NPC's turn to think; it acts when the delay is up.
    pub fn npc_step(&mut self) {
        if self.core.priority != Side::Npc || self.core.outcome.is_some() {
            self.ui.npc_wait = 0;
            return;
        }
        self.ui.npc_wait += 1;
        if self.ui.npc_wait < NPC_DELAY {
            return;
        }
        self.ui.npc_wait = 0;
        let action = npc::choose(&self.core);
        if let Err(error) = self.core.try_act(Side::Npc, action) {
            panic!("the NPC chose an illegal action: {error}");
        }
        // A stack change under the cursor invalidates a half-made choice.
        self.ui.targeting = None;
        self.clamp_selection();
    }

    pub fn result_line(&self) -> Option<&'static str> {
        match self.core.outcome? {
            Outcome::Won(Side::You) => Some("YOU WIN"),
            Outcome::Won(Side::Npc) => Some("YOU LOSE"),
            Outcome::Draw => Some("DRAW"),
        }
    }
}
