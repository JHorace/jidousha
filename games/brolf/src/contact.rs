//! Contact: who a swing reaches and what it does, decided in one function.
//!
//! Key types: `PlayerSnap`, `Contact`. Key functions: `contact_target`, `snap_players`.
//! Depends on: `items`, `shots`, `world`. Never depended on outside the game.
//! INVARIANT: `contact_target` is read by the reach cue, the NPCs and the swing system;
//! a club never removes a player, only dazes one.

use jidousha::prelude::*;

use crate::items::{Effects, Item, Kit};
use crate::shots::{STRIKE_DISTANCE, strike_landing};
use crate::world::{Ball, Persona, Player};

/// How far a swing reaches, before equipment.
pub const SWING_REACH: f32 = 1.5;
/// How many ticks must pass between swings, before equipment.
pub const SWING_COOLDOWN: u64 = 120;
/// How many ticks a club dazes for, before equipment.
pub const DAZE_TICKS: u64 = 180;

/// One golfer, as plain values every rule function can take.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerSnap {
    /// Which golfer: 0 is the human.
    pub index: usize,
    /// How they play.
    pub persona: Persona,
    /// Where their body is.
    pub pos: Vec2,
    /// Where their ball is.
    pub ball: Vec2,
    /// Whether the ball is at rest.
    pub ball_resting: bool,
    /// The tick their daze ends.
    pub dazed_until: u64,
    /// The first tick they may swing again.
    pub swing_ready_at: u64,
    /// Whether they are standing on the pad, counting down.
    pub extracting: bool,
    /// Whether they have left the match by extracting.
    pub extracted: bool,
    /// What they hold.
    pub kit: Kit,
    /// What they hold, as multipliers.
    pub effects: Effects,
}

impl PlayerSnap {
    /// Whether they are dazed at `tick`.
    pub fn dazed(&self, tick: u64) -> bool {
        tick < self.dazed_until
    }
}

/// What a swing does.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Contact {
    /// Club a rival's body: daze them, and rob them if they can be robbed.
    Club {
        /// Who is clubbed.
        who: usize,
        /// How long they are dazed for.
        daze_ticks: u64,
        /// What they drop at the clubber's feet.
        drops: Option<Item>,
    },
    /// Strike a rival's resting ball away.
    Strike {
        /// Whose ball.
        whose: usize,
        /// Where it will land.
        lands_at: Vec2,
    },
}

/// What `me` would do by swinging now: the one contact-resolution function.
///
/// The nearest rival body in reach wins; failing that, the nearest rival resting
/// ball in the same reach. Ties go to the lower player index.
pub fn contact_target(me: &PlayerSnap, all: &[PlayerSnap], tick: u64) -> Option<Contact> {
    if me.dazed(tick) || me.extracting || tick < me.swing_ready_at {
        return None;
    }
    let reach = SWING_REACH * me.effects.swing_reach;
    let mut body: Option<(f32, &PlayerSnap)> = None;
    let mut ball: Option<(f32, &PlayerSnap)> = None;
    for rival in all.iter().filter(|p| p.index != me.index) {
        let to_body = (rival.pos - me.pos).length();
        if to_body <= reach && body.is_none_or(|(best, _)| to_body < best) {
            body = Some((to_body, rival));
        }
        let to_ball = (rival.ball - me.pos).length();
        if rival.ball_resting && to_ball <= reach && ball.is_none_or(|(best, _)| to_ball < best) {
            ball = Some((to_ball, rival));
        }
    }
    if let Some((_, rival)) = body {
        let drops = if rival.effects.drops_when_clubbed {
            rival.kit.most_recent()
        } else {
            None
        };
        return Some(Contact::Club {
            who: rival.index,
            daze_ticks: (DAZE_TICKS as f32 * rival.effects.daze_taken).round() as u64,
            drops,
        });
    }
    ball.map(|(_, rival)| Contact::Strike {
        whose: rival.index,
        lands_at: strike_landing(
            me.pos,
            rival.ball,
            STRIKE_DISTANCE * me.effects.strike_given * rival.effects.strike_taken,
        ),
    })
}

/// Every golfer in play, sorted by index: the one reader of the world.
pub fn snap_players(world: &WorldView<'_>) -> Vec<PlayerSnap> {
    let balls: Vec<(usize, Vec2, bool)> = world
        .query::<(&Transform, &Ball)>()
        .map(|(_, transform, ball)| (ball.owner, transform.pos, ball.flight.is_none()))
        .collect();
    let mut snaps: Vec<PlayerSnap> = world
        .query::<(&Transform, &Player)>()
        .filter_map(|(_, transform, player)| {
            let (_, ball, resting) = balls.iter().find(|(owner, _, _)| *owner == player.index)?;
            Some(PlayerSnap {
                index: player.index,
                persona: player.persona,
                pos: transform.pos,
                ball: *ball,
                ball_resting: *resting,
                dazed_until: player.dazed_until,
                swing_ready_at: player.swing_ready_at,
                extracting: player.extracting_since.is_some(),
                extracted: player.extracted,
                kit: player.kit,
                effects: player.kit.effects(),
            })
        })
        .collect();
    snaps.sort_by_key(|snap| snap.index);
    snaps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(index: usize, pos: Vec2, ball: Vec2) -> PlayerSnap {
        PlayerSnap {
            index,
            persona: Persona::Golfer,
            pos,
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
    fn contact_prefers_a_body_to_a_ball_and_the_nearer_of_two_bodies() {
        let me = snap(0, Vec2::ZERO, Vec2::new(0.0, 5.0));
        let near = snap(1, Vec2::new(1.0, 0.0), Vec2::new(0.5, 0.0));
        let far = snap(2, Vec2::new(1.4, 0.0), Vec2::new(9.0, 9.0));
        let all = [me, near, far];
        assert!(matches!(
            contact_target(&me, &all, 10),
            Some(Contact::Club { who: 1, .. })
        ));
        let all = [me, snap(1, Vec2::new(5.0, 5.0), Vec2::new(0.5, 0.0))];
        assert!(matches!(
            contact_target(&me, &all, 10),
            Some(Contact::Strike { whose: 1, .. })
        ));
    }

    #[test]
    fn a_swing_reaches_a_body_exactly_at_the_swing_reach_and_no_further() {
        let me = snap(0, Vec2::ZERO, Vec2::new(0.0, 5.0));
        let at = snap(1, Vec2::new(1.5, 0.0), Vec2::new(9.0, 9.0));
        let past = snap(1, Vec2::new(1.51, 0.0), Vec2::new(9.0, 9.0));
        assert!(contact_target(&me, &[me, at], 10).is_some());
        assert!(contact_target(&me, &[me, past], 10).is_none());
    }

    #[test]
    fn a_dazed_golfer_or_one_waiting_to_swing_again_reaches_nothing() {
        let mut me = snap(0, Vec2::ZERO, Vec2::new(0.0, 5.0));
        let rival = snap(1, Vec2::new(1.0, 0.0), Vec2::new(9.0, 9.0));
        me.dazed_until = 50;
        assert!(contact_target(&me, &[me, rival], 49).is_none());
        assert!(contact_target(&me, &[me, rival], 50).is_some());
        me.dazed_until = 0;
        me.swing_ready_at = 120;
        assert!(contact_target(&me, &[me, rival], 119).is_none());
    }

    #[test]
    fn a_helmet_shortens_the_daze_and_refuses_the_robbery() {
        let me = snap(0, Vec2::ZERO, Vec2::new(0.0, 5.0));
        let mut rival = snap(1, Vec2::new(1.0, 0.0), Vec2::new(9.0, 9.0));
        rival.kit.take(Item::Helmet);
        rival.effects = rival.kit.effects();
        match contact_target(&me, &[me, rival], 10) {
            Some(Contact::Club {
                daze_ticks, drops, ..
            }) => {
                assert_eq!(daze_ticks, 90);
                assert_eq!(drops, None);
            }
            other => panic!("expected a club, got {other:?}"),
        }
    }
}
