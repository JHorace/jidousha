//! The three players the check plays with: one that does nothing, a golfer, and a full player.
//!
//! Key types: `Kind`, `Controller`, `Report`.
//! Depends on: `contact`, `intent`, `shots`, `text`, `world`, `zone`. Never depended on
//! outside the game's check.
//! INVARIANT: a player sends input events, never states, through the same snapshot
//! builder a real keyboard goes through, and decides with the NPCs' own functions.

use jidousha::prelude::*;
use jidousha::testing::{InputEvent, SnapshotBuilder};

use crate::contact::{contact_target, snap_players};
use crate::intent::{Intent, npc_intent};
use crate::items::Item;
use crate::shots::{MAX_SHOT, aim_point};
use crate::world::{Ball, Course, Persona, Pickup, Player};
use crate::zone::zone_at;

/// Which player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Does nothing, ever: proves the game can be lost.
    Idle,
    /// Walks to its ball and hits it where the zone will be: a first-try player.
    Golfer,
    /// The golfer, who also swings at whoever is in reach and extracts once holding two items.
    Full,
}

/// What the player learned about itself: the three numbers a run prints.
#[derive(Clone, Copy, Debug, Default)]
pub struct Report {
    /// Shots taken.
    pub shots: u32,
    /// Shots that landed inside the zone as it was when the ball landed.
    pub in_zone_at_arrival: u32,
    /// Sum of the planned aims' distances from the hole.
    pub aimed_from_hole: f32,
    /// Sum of the distances between where shots landed and where they were planned to.
    pub landed_from_plan: f32,
    /// Sum of the planned shots' lengths.
    pub planned_length: f32,
}

impl Report {
    /// Mean distance of the planned aims from the hole.
    pub fn mean_aimed_from_hole(&self) -> f32 {
        self.aimed_from_hole / self.shots.max(1) as f32
    }
    /// Mean distance between landing and plan.
    pub fn mean_landed_from_plan(&self) -> f32 {
        self.landed_from_plan / self.shots.max(1) as f32
    }
    /// Mean planned shot length.
    pub fn mean_length(&self) -> f32 {
        self.planned_length / self.shots.max(1) as f32
    }
}

/// A player's state between ticks.
pub struct Controller {
    kind: Kind,
    keys: [bool; 4],
    click: u8,
    space: bool,
    extract: bool,
    plan: Option<Vec2>,
    seen_shots: u32,
    /// What it has learned about itself.
    pub report: Report,
}

const KEYS: [Key; 4] = [Key::W, Key::A, Key::S, Key::D];

impl Controller {
    /// A fresh player of `kind`.
    pub fn new(kind: Kind) -> Self {
        Controller {
            kind,
            keys: [false; 4],
            click: 0,
            space: false,
            extract: false,
            plan: None,
            seen_shots: 0,
            report: Report::default(),
        }
    }

    /// Look at the world after a tick and record the events for the next one.
    pub fn step(&mut self, view: &WorldView<'_>, keyboard: &mut SnapshotBuilder, camera: &Camera) {
        if self.kind == Kind::Idle || view.find_resource::<Course>().is_none() {
            return;
        }
        let snaps = snap_players(view);
        let Some(me) = snaps.iter().find(|snap| snap.index == 0).copied() else {
            return;
        };
        let course = *view.resource::<Course>();
        let tick = view.resource::<Time>().tick + 1;
        self.observe(view, &course);

        let pickups: Vec<(Vec2, Item)> = view
            .query::<(&Transform, &Pickup)>()
            .map(|(_, transform, pickup)| (transform.pos, pickup.0))
            .collect();
        let mut brain = me;
        brain.persona = if self.kind == Kind::Full && me.kit.count() >= 2 {
            Persona::Extractor
        } else {
            Persona::Golfer
        };
        let mut intent = npc_intent(&brain, &snaps, &pickups, &course, tick);
        if self.kind == Kind::Full && contact_target(&me, &snaps, tick).is_some() {
            intent.swing = true;
        }
        self.send(intent, &me, keyboard, camera);
    }

    /// Record the shot taken on the last tick, if there was one.
    fn observe(&mut self, view: &WorldView<'_>, course: &Course) {
        let Some(player) = view
            .query::<&Player>()
            .find(|(_, p)| p.index == 0)
            .map(|(_, p)| *p)
        else {
            return;
        };
        if player.shots == self.seen_shots {
            return;
        }
        self.seen_shots = player.shots;
        let flight = view
            .query::<&Ball>()
            .find(|(_, ball)| ball.owner == 0)
            .and_then(|(_, ball)| ball.flight);
        let (Some(plan), Some(flight)) = (self.plan, flight) else {
            return;
        };
        self.report.shots += 1;
        self.report.aimed_from_hole += (plan - course.hole).length();
        self.report.landed_from_plan += (flight.to - plan).length();
        self.report.planned_length += (plan - flight.from).length();
        if zone_at(&course.schedule, flight.start + flight.ticks).contains(flight.to) {
            self.report.in_zone_at_arrival += 1;
        }
    }

    /// Turn an intent into the events a keyboard and mouse would send.
    fn send(
        &mut self,
        intent: Intent,
        me: &crate::contact::PlayerSnap,
        keyboard: &mut SnapshotBuilder,
        camera: &Camera,
    ) {
        let want = [
            intent.walk.y < -0.3,
            intent.walk.x < -0.3,
            intent.walk.y > 0.3,
            intent.walk.x > 0.3,
        ];
        for (k, key) in KEYS.iter().enumerate() {
            if want[k] != self.keys[k] {
                keyboard.record(if want[k] {
                    InputEvent::KeyPressed(*key)
                } else {
                    InputEvent::KeyReleased(*key)
                });
                self.keys[k] = want[k];
            }
        }
        match self.click {
            1 => {
                keyboard.record(InputEvent::ButtonPressed {
                    id: PointerId::PRIMARY,
                    button: PointerButton::Primary,
                });
                self.click = 2;
            }
            2 => {
                keyboard.record(InputEvent::ButtonReleased {
                    id: PointerId::PRIMARY,
                    button: PointerButton::Primary,
                });
                self.click = 0;
            }
            _ => {
                if let Some(at) = intent.shoot {
                    keyboard.record(InputEvent::PointerMoved {
                        id: PointerId::PRIMARY,
                        screen: camera.world_to_screen(at),
                    });
                    let reach = MAX_SHOT * me.effects.shot_reach;
                    self.plan = Some(aim_point(me.ball, at, reach));
                    self.click = 1;
                }
            }
        }
        self.space = edge(keyboard, Key::Space, intent.swing, self.space);
        self.extract = edge(keyboard, Key::E, intent.extract, self.extract);
    }
}

/// Press a key on the tick its want turns on, release it on the next.
fn edge(keyboard: &mut SnapshotBuilder, key: Key, want: bool, down: bool) -> bool {
    if want && !down {
        keyboard.record(InputEvent::KeyPressed(key));
        true
    } else if down {
        keyboard.record(InputEvent::KeyReleased(key));
        false
    } else {
        false
    }
}
