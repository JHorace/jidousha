//! What a golfer asks to do in a tick, and what the match records happened.
//!
//! An `Intent` is the whole interface between a brain — the keyboard, an
//! NPC, a `--verify` player — and `sim::step`; an `Event` is a line of the
//! log a check counts from.

use jidousha::prelude::*;

use crate::rules::{Ending, Item};

/// What one golfer asks to do this tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Intent {
    /// Which way to walk; length at most one.
    pub walk: Vec2,
    /// Turn the aim: -1 anticlockwise on screen .. 1 clockwise.
    pub turn: f32,
    /// Change power: -1 .. 1.
    pub power: f32,
    /// Commit the shot.
    pub shoot: bool,
    /// Club or strike, whichever `contact_for` offers.
    pub contact: bool,
    /// Take the pickup underfoot.
    pub take: bool,
    /// Extract from the pad underfoot.
    pub extract: bool,
}

/// Something that happened, for the log a check reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// `who` hit their ball.
    Shot {
        who: usize,
        from: Vec2,
        planned: Vec2,
    },
    /// `who` sank cup `cup`.
    Sunk { who: usize, cup: usize },
    /// `by` clubbed `target`, taking `stolen` tokens.
    Club {
        by: usize,
        target: usize,
        stolen: u32,
    },
    /// `by` struck `owner`'s ball.
    Strike { by: usize, owner: usize },
    /// `who` took `item`.
    Took { who: usize, item: Item },
    /// `who` was eliminated; `bounty` went to `to`.
    Eliminated {
        who: usize,
        to: Option<usize>,
        bounty: u32,
    },
    /// `who` ended `ending`.
    Ended { who: usize, ending: Ending },
}
