//! Staged scenes: the decision rows a played match reaches by chance, set up
//! on purpose and asked directly.
//!
//! Each builds the real game headless, ticks once so `Startup` has run, then
//! sets every piece of state the scene depends on — all NPCs idle unless the
//! scene needs one to act — and drives the player through the keyboard.
//! Expectations are shipped literals (300 ticks of grace, 180 of stun, 2.25
//! and 9.0 units of knock), never arithmetic over the constant under test.

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder};

use crate::checks::{Checks, within};
use crate::draw::palette;
use crate::events::Intent;
use crate::players::Keyboard;
use crate::rules::{self, Contact, Item};
use crate::sim::{Brain, Match};
use crate::text::{hint_line, status_lines};
use crate::zone::{inside_at, shrink_window, zone_at};
use crate::{WINDOW, config, register};

/// A headless game playing `game`, after its first tick.
pub fn staged(game: Match) -> HeadlessSim {
    let mut sim = headless(config(), register);
    sim.world_mut().insert_resource(game);
    sim.tick();
    for golfer in &mut sim.world_mut().resource_mut::<Match>().golfers[1..] {
        golfer.brain = Brain::Idle;
    }
    sim
}

/// One tick with the player doing `intent`.
pub fn tick_with(sim: &mut HeadlessSim, keyboard: &mut Keyboard, intent: Intent) {
    let input = keyboard.input_for(intent);
    sim.world_mut().insert_resource(input);
    sim.tick();
}

/// Whether some quad of `tint` covers `at`.
pub fn tinted_at(frame: &FrameRecord, at: Vec2, tint: Color) -> bool {
    frame.covering(at).iter().any(|quad| quad.tint == tint)
}

/// Row 1, grace: a ball left outside the zone is out on exactly the 300th
/// tick after the first tick outside, and the countdown is drawn meanwhile.
pub fn grace(checks: &mut Checks) -> String {
    let mut game = Match::new(11);
    // Into the second circle's hold, where the zone stands still for 15 s.
    game.tick = shrink_window(0).1 + 60;
    let mut sim = staged(game);
    let mut keyboard = Keyboard::default();
    let mut recorder = FrameRecorder::new(WINDOW);
    let (zone, first_out) = {
        let game = sim.world_mut().resource_mut::<Match>();
        let zone = zone_at(&game.schedule, game.tick);
        game.golfers[0].pos = zone.center;
        let side = if zone.center.x > 0.0 { -1.0 } else { 1.0 };
        game.golfers[0].ball.pos = zone.center + Vec2::new(side * (zone.radius + 2.0), 0.0);
        (zone, game.tick + 1)
    };
    let ball = sim.world().resource::<Match>().golfers[0].ball.pos;
    checks.require(
        !inside_at(&sim.world().resource::<Match>().schedule, ball, first_out),
        "the grace scene did not put the ball outside",
        format!("ball {ball:?}, zone {zone:?}"),
    );
    tick_with(&mut sim, &mut keyboard, Intent::default());
    let frame = recorder.draw(&mut sim);
    let status = status_lines(sim.world().resource::<Match>())[0].clone();
    checks.require(
        status.contains("OUT 5.0s"),
        "the status bar does not count the grace down from 5.0s on the first tick out",
        format!("status line: {status:?}"),
    );
    let out_drawn = frame.quads().iter().any(|quad| quad.tint == palette::OUT);
    checks.require(
        out_drawn,
        "no grace countdown drawn over the golfer on the first tick out",
        format!("{} quads drawn, none in the OUT colour", frame.quad_count()),
    );
    let mut ended_on = None;
    for _ in 0..400 {
        tick_with(&mut sim, &mut keyboard, Intent::default());
        let game = sim.world().resource::<Match>();
        if let Some((_, _, tick)) = game.golfers[0].ending {
            ended_on = Some(tick);
            break;
        }
    }
    let want = first_out + 300;
    checks.require(
        ended_on == Some(want),
        "a ball left outside was not eliminated on the tick the grace says",
        format!("first outside on tick {first_out}, eliminated on {ended_on:?}, want {want}"),
    );
    format!("ball out on tick {first_out}, eliminated on {ended_on:?} (want {want})")
}

/// Row 2: the cue shows first, a club stuns for exactly 180 ticks, takes a
/// token, and never removes the target.
pub fn club(checks: &mut Checks) -> String {
    let mut sim = staged(Match::new(11));
    let mut keyboard = Keyboard::default();
    let mut recorder = FrameRecorder::new(WINDOW);
    let target = {
        let game = sim.world_mut().resource_mut::<Match>();
        let at = game.golfers[0].pos + Vec2::new(1.5, 0.0);
        game.golfers[1].pos = at;
        game.golfers[1].stash = 2;
        at
    };
    tick_with(&mut sim, &mut keyboard, Intent::default());
    let frame = recorder.draw(&mut sim);
    let hint = hint_line(sim.world().resource::<Match>());
    let cue = frame
        .quads()
        .iter()
        .filter(|quad| quad.tint == palette::CUE)
        .filter(|quad| (quad.bounds().center() - target).length() < 1.3)
        .count();
    checks.require(
        cue > 0 && hint.starts_with("F: CLUB RED - stun 3.0s, take 1"),
        "the club cue is not on screen before the club",
        format!("{cue} cue quads around the target; hint {hint:?}"),
    );
    tick_with(
        &mut sim,
        &mut keyboard,
        Intent {
            contact: true,
            ..Intent::default()
        },
    );
    let clubbed_on = sim.world().resource::<Match>().tick;
    let mut stunned = 0;
    let mut freed_on = None;
    let mut removed = false;
    for _ in 0..240 {
        let game = sim.world().resource::<Match>();
        removed |= game.golfers[1].ending.is_some();
        if game.golfers[1].stun > 0 {
            stunned += 1;
        } else if freed_on.is_none() {
            freed_on = Some(game.tick);
        }
        tick_with(&mut sim, &mut keyboard, Intent::default());
    }
    let game = sim.world().resource::<Match>();
    let stash = (game.golfers[0].stash, game.golfers[1].stash);
    checks.require(
        stunned == 180 && freed_on == Some(clubbed_on + 180),
        "a club did not stun for exactly three seconds",
        format!("clubbed on {clubbed_on}, stunned {stunned} ticks, free on {freed_on:?}"),
    );
    checks.require(
        stash == (1, 1) && !removed,
        "a club did not take exactly one token, or removed the target",
        format!("stashes (player, target) {stash:?}, want (1, 1); removed: {removed}"),
    );
    format!(
        "cue then club on {clubbed_on}: stunned {stunned} ticks, free on {freed_on:?}, stash {stash:?}"
    )
}

/// How far a rival's strike moves the player's ball, holding `item`; and the
/// hint shown while standing on a HEAVY pickup, when `item` is one.
fn knock(item: Option<Item>) -> (f32, String) {
    let mut sim = staged(Match::new(11));
    let mut keyboard = Keyboard::default();
    let mut hint = String::new();
    if item.is_some() {
        {
            let game = sim.world_mut().resource_mut::<Match>();
            let Some(pickup) = game
                .pickups
                .iter()
                .find(|pickup| pickup.item == Item::Heavy)
            else {
                return (f32::NAN, "no HEAVY pickup on seed 11".to_owned());
            };
            game.golfers[0].pos = pickup.pos;
        }
        tick_with(&mut sim, &mut keyboard, Intent::default());
        hint = hint_line(sim.world().resource::<Match>());
        tick_with(
            &mut sim,
            &mut keyboard,
            Intent {
                take: true,
                ..Intent::default()
            },
        );
    }
    let from = {
        let game = sim.world_mut().resource_mut::<Match>();
        let ball = Vec2::new(0.0, 0.0);
        game.golfers[0].ball.pos = ball;
        game.golfers[0].pos = Vec2::new(0.0, 6.0);
        game.golfers[1].pos = ball + Vec2::new(-1.0, 0.0);
        game.golfers[1].brain = Brain::Brute;
        ball
    };
    for _ in 0..240 {
        tick_with(&mut sim, &mut keyboard, Intent::default());
    }
    let game = sim.world().resource::<Match>();
    let moved = (game.golfers[0].ball.pos - from).length();
    if item.is_some() && game.golfers[0].item != Some(Item::Heavy) {
        return (f32::NAN, format!("E did not take it; hint was {hint:?}"));
    }
    (moved, hint)
}

/// Row 3: taking HEAVY on a fixed seed does what its description says.
pub fn equipment(checks: &mut Checks) -> String {
    let (bare, _) = knock(None);
    let (heavy, hint) = knock(Some(Item::Heavy));
    checks.require(
        hint.contains("HEAVY: a struck ball rolls 2.25 not 9.0"),
        "the HEAVY pickup does not say what it does before it is taken",
        format!("hint on the pickup: {hint:?}"),
    );
    checks.require(
        within(bare, 9.0, 0.01) && within(heavy, 2.25, 0.01),
        "a strike does not move the ball as far as the equipment says",
        format!("bare ball rolled {bare:.3} (want 9.0), HEAVY ball rolled {heavy:.3} (want 2.25)"),
    );
    format!("strike moves a bare ball {bare:.2}, a HEAVY ball {heavy:.2}")
}

/// The other two pieces of equipment, asked their contract directly: a club
/// on a HELMET stuns for one second and takes nothing, and a full DRIVER shot
/// stops 21 units out. Literals, so a constant that moves is seen moving.
pub fn equipment_contracts(checks: &mut Checks) -> String {
    let mut game = Match::new(11);
    game.golfers[1].pos = game.golfers[0].pos + Vec2::new(1.5, 0.0);
    game.golfers[1].stash = 2;
    game.golfers[1].item = Some(Item::Helmet);
    let club = rules::contact_for(&game, 0);
    let says = rules::describe(Item::Helmet);
    checks.require(
        club == Some(Contact::Club {
            target: 1,
            stun_ticks: 60,
            steal: 0,
        }) && says.contains("stun 1.0s, lose 0 tokens"),
        "a club on a HELMET does not do what the HELMET says",
        format!("contact {club:?}; description {says:?}"),
    );
    let from = Vec2::new(-10.0, 0.0);
    let reach = (rules::landing(from, Radians::ZERO, 1.0, Some(Item::Driver)) - from).length();
    checks.require(
        within(reach, 21.0, 0.01),
        "a full DRIVER shot does not stop where the DRIVER says",
        format!("rolled {reach:.3}, want 21.0"),
    );
    format!("HELMET club {club:?}; DRIVER full shot {reach:.2}")
}
