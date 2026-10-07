//! How a match ends, what the player keeps, and what the status line promises.
//!
//! Key types: `Outcome`, `Placing`, `PadState`, `Standing`. Key functions: `kept_on`,
//! `settle`, `standing`, `pad_state`.
//! Depends on: `contact`, `items`. Never depended on outside the game.
//! INVARIANT: `kept_on` is the one outcome function: the status line prints
//! `kept_on(Extracted, kit)` before the player decides, and the result screen prints
//! `kept_on(outcome, kit)` after, so what is promised is what is kept.

use jidousha::prelude::*;

use crate::contact::PlayerSnap;
use crate::items::Kit;

/// The tick the hole uncovers: the match's last two zones are a fight over it.
pub const HOLE_OPENS: u64 = 5700;
/// The tick the extraction pad opens.
pub const PAD_OPENS: u64 = 2700;
/// How near body and ball must be to the pad's centre to extract.
pub const PAD_RADIUS: f32 = 1.2;
/// How long the human must stand on the pad, in ticks.
pub const EXTRACT_TICKS: u64 = 120;
/// How near a resting ball must be to the hole's centre to drop in.
pub const HOLE_RADIUS: f32 = 0.35;
/// The tick the match is called for whoever is nearest the pin.
pub const MATCH_END: u64 = 9000;

/// How the human's match ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The human's ball dropped in the hole.
    HoledOut,
    /// Every rival is gone.
    LastStanding,
    /// Time ran out with the human's ball nearest the pin.
    ClosestToPin,
    /// The human left through the pad.
    Extracted,
    /// The zone took the human.
    Eliminated,
    /// A rival holed out first.
    Lost {
        /// Who did.
        to: usize,
    },
}

/// Where the human placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placing {
    /// First.
    Won,
    /// Eliminated, in this place of six.
    Out {
        /// The place.
        place: usize,
    },
    /// Left by extracting, with this many rivals still in.
    Left {
        /// How many.
        still_in: usize,
    },
    /// A rival holed out first.
    Lost {
        /// Who did.
        to: usize,
    },
}

/// Whether the extraction pad can be used.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadState {
    /// Not yet.
    Closed {
        /// The tick it opens.
        opens_at: u64,
    },
    /// Usable now.
    Open {
        /// The tick the zone leaves it, if it ever does.
        closes_at: Option<u64>,
    },
    /// The zone has left it.
    Gone,
}

/// The pad's state at `tick`, given when the zone leaves it.
pub fn pad_state(closes_at: Option<u64>, tick: u64) -> PadState {
    if tick < PAD_OPENS {
        PadState::Closed {
            opens_at: PAD_OPENS,
        }
    } else if closes_at.is_some_and(|closes| tick >= closes) {
        PadState::Gone
    } else {
        PadState::Open { closes_at }
    }
}

/// What the human keeps when the match ends as `outcome`: the one outcome function.
pub fn kept_on(outcome: Outcome, kit: &Kit) -> Kit {
    match outcome {
        Outcome::Eliminated | Outcome::Lost { .. } => Kit::default(),
        Outcome::HoledOut | Outcome::LastStanding | Outcome::ClosestToPin | Outcome::Extracted => {
            *kit
        }
    }
}

/// A kit as words: "heavy ball, helmet", or "nothing".
pub fn kept_text(kit: &Kit) -> String {
    let names: Vec<&str> = [kit.ball, kit.body]
        .into_iter()
        .flatten()
        .map(|item| item.name())
        .collect();
    if names.is_empty() {
        "nothing".to_owned()
    } else {
        names.join(", ")
    }
}

/// The always-visible match status: how close each way of ending is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Standing {
    /// How far the human's ball is from the hole.
    pub hole_distance: f32,
    /// Whether the hole is uncovered.
    pub hole_open: bool,
    /// How many rivals are still in the match.
    pub rivals_left: usize,
    /// Where the human's ball ranks by distance to the pin, among those in.
    pub pin_rank: usize,
    /// The pad.
    pub pad: PadState,
}

/// The standing of golfer `me` at `tick`.
pub fn standing(snaps: &[PlayerSnap], me: usize, hole: Vec2, tick: u64, pad: PadState) -> Standing {
    let mine = snaps
        .iter()
        .find(|snap| snap.index == me)
        .map_or(f32::MAX, |snap| (snap.ball - hole).length());
    let rivals: Vec<&PlayerSnap> = snaps
        .iter()
        .filter(|snap| snap.index != me && !snap.extracted)
        .collect();
    let nearer = rivals
        .iter()
        .filter(|snap| (snap.ball - hole).length() < mine)
        .count();
    Standing {
        hole_distance: mine,
        hole_open: tick >= HOLE_OPENS,
        rivals_left: rivals.len(),
        pin_rank: nearer + 1,
        pad,
    }
}

/// The match-ending event for the human, if there is one, in this order: the human
/// extracted, the human eliminated, a hole-out, nobody left, time up.
pub fn settle(
    snaps: &[PlayerSnap],
    eliminated: &[usize],
    hole: Vec2,
    tick: u64,
) -> Option<(Outcome, Placing)> {
    let live: Vec<&PlayerSnap> = snaps.iter().filter(|snap| !snap.extracted).collect();
    let rivals_left = live.iter().filter(|snap| snap.index != 0).count();
    if snaps.iter().any(|snap| snap.index == 0 && snap.extracted) {
        return Some((
            Outcome::Extracted,
            Placing::Left {
                still_in: rivals_left,
            },
        ));
    }
    if eliminated.contains(&0) {
        return Some((
            Outcome::Eliminated,
            Placing::Out {
                place: rivals_left + 1,
            },
        ));
    }
    if tick >= HOLE_OPENS {
        let holed = live
            .iter()
            .filter(|snap| snap.ball_resting && (snap.ball - hole).length() <= HOLE_RADIUS)
            .map(|snap| snap.index)
            .min();
        match holed {
            Some(0) => return Some((Outcome::HoledOut, Placing::Won)),
            Some(to) => return Some((Outcome::Lost { to }, Placing::Lost { to })),
            None => {}
        }
    }
    if rivals_left == 0 {
        return Some((Outcome::LastStanding, Placing::Won));
    }
    if tick >= MATCH_END {
        let nearest = live
            .iter()
            .min_by(|a, b| {
                let da = (a.ball - hole).length();
                let db = (b.ball - hole).length();
                da.total_cmp(&db).then(a.index.cmp(&b.index))
            })
            .map(|snap| snap.index)?;
        return Some(if nearest == 0 {
            (Outcome::ClosestToPin, Placing::Won)
        } else {
            (Outcome::Lost { to: nearest }, Placing::Lost { to: nearest })
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contact::PlayerSnap;
    use crate::items::{Effects, Item};
    use crate::world::Persona;

    fn snap(index: usize, ball: Vec2) -> PlayerSnap {
        PlayerSnap {
            index,
            persona: Persona::Golfer,
            pos: Vec2::new(9.0, 9.0),
            ball,
            ball_resting: true,
            dazed_until: 0,
            swing_ready_at: 0,
            extracting: false,
            extracted: false,
            kit: Kit::default(),
            effects: Effects::NEUTRAL,
        }
    }

    #[test]
    fn kept_on_eliminated_or_lost_is_empty_and_kept_on_anything_else_is_the_kit() {
        let kit = Kit {
            ball: Some(Item::HeavyBall),
            body: Some(Item::Helmet),
            last_taken: Some(crate::items::Slot::Body),
        };
        assert_eq!(kept_on(Outcome::Eliminated, &kit), Kit::default());
        assert_eq!(kept_on(Outcome::Lost { to: 2 }, &kit), Kit::default());
        for outcome in [
            Outcome::HoledOut,
            Outcome::LastStanding,
            Outcome::ClosestToPin,
            Outcome::Extracted,
        ] {
            assert_eq!(kept_on(outcome, &kit), kit, "{outcome:?}");
        }
        assert_eq!(kept_text(&kit), "heavy ball, helmet");
        assert_eq!(kept_text(&Kit::default()), "nothing");
    }

    #[test]
    fn settle_orders_extraction_before_elimination_before_a_hole_out() {
        let hole = Vec2::new(1.0, 1.0);
        let mut human = snap(0, hole);
        human.extracted = true;
        let rival = snap(1, hole);
        let all = [human, rival];
        let tick = HOLE_OPENS + 10;
        assert_eq!(
            settle(&all, &[0], hole, tick),
            Some((Outcome::Extracted, Placing::Left { still_in: 1 }))
        );
        human.extracted = false;
        let all = [human, rival];
        assert_eq!(
            settle(&all, &[0], hole, tick),
            Some((Outcome::Eliminated, Placing::Out { place: 2 }))
        );
        assert_eq!(
            settle(&all, &[], hole, tick),
            Some((Outcome::HoledOut, Placing::Won)),
            "the lower index wins a tie for the hole"
        );
        let all = [snap(0, Vec2::new(5.0, 5.0)), rival];
        assert_eq!(
            settle(&all, &[], hole, tick),
            Some((Outcome::Lost { to: 1 }, Placing::Lost { to: 1 }))
        );
    }

    #[test]
    fn a_ball_in_the_hole_before_the_hole_opens_does_not_end_the_match() {
        let hole = Vec2::new(1.0, 1.0);
        let all = [snap(0, hole), snap(1, Vec2::new(5.0, 5.0))];
        assert_eq!(settle(&all, &[], hole, HOLE_OPENS - 1), None);
        assert_eq!(
            settle(&all, &[], hole, HOLE_OPENS),
            Some((Outcome::HoledOut, Placing::Won))
        );
    }

    #[test]
    fn the_last_golfer_standing_wins_and_time_up_goes_to_the_nearest_ball() {
        let hole = Vec2::ZERO;
        let all = [snap(0, Vec2::new(3.0, 0.0))];
        assert_eq!(
            settle(&all, &[], hole, 100),
            Some((Outcome::LastStanding, Placing::Won))
        );
        let all = [snap(0, Vec2::new(3.0, 0.0)), snap(2, Vec2::new(2.0, 0.0))];
        assert_eq!(settle(&all, &[], hole, MATCH_END - 1), None);
        assert_eq!(
            settle(&all, &[], hole, MATCH_END),
            Some((Outcome::Lost { to: 2 }, Placing::Lost { to: 2 }))
        );
    }

    #[test]
    fn the_pad_is_closed_then_open_then_gone() {
        assert_eq!(
            pad_state(Some(5000), PAD_OPENS - 1),
            PadState::Closed {
                opens_at: PAD_OPENS
            }
        );
        assert_eq!(
            pad_state(Some(5000), 4999),
            PadState::Open {
                closes_at: Some(5000)
            }
        );
        assert_eq!(pad_state(Some(5000), 5000), PadState::Gone);
    }
}
