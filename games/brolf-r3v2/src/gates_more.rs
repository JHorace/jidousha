//! Decision rows 3 and 4 — equipment and extraction — the staged end screens,
//! and the contracts no played session reaches (DESIGN.md, Gates).

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, find_bounds};

use crate::checks::{Checks, Driver, glyphs_at};
use crate::draw::{BANNER_SIZE, BANNER_Y, HUD_AT};
use crate::gates::{edit_golfer, freeze, place_ball};
use crate::hud::{hud_lines_of, result_lines};
use crate::model::*;
use crate::players::Player;
use crate::rules::*;
use crate::world::{Match, Pickup};

/// Line 6 of the HUD now.
fn line6(driver: &Driver) -> String {
    hud_lines_of(&driver.snap())[5].clone()
}

/// Row 3: the pickup line says what an item does, and the sim then does it.
pub fn equipment(checks: &mut Checks) -> String {
    let mut driver = Driver::new();
    driver.idle();
    freeze(&mut driver, &[1, 2, 3]);
    let Some(&(at, _)) = driver
        .snap()
        .pickups
        .iter()
        .find(|(_, item)| *item == Item::Driver)
    else {
        checks.require(
            false,
            "decision 3 equipment: no Driver lies on the course",
            String::new(),
        );
        return "decision 3 equipment: not run".to_owned();
    };
    edit_golfer(&mut driver, 0, |t, _| t.pos = at - Vec2::new(1.0, 0.0));
    place_ball(&mut driver, 0, at + Vec2::new(0.0, -1.0), Vec2::ZERO);
    driver.idle();
    let frame = driver.draw();
    let offer = line6(&driver);
    let drawn = glyphs_at(&frame, driver.font(), HUD_AT.y + 5.0);
    checks.require(
        offer == "F: take Driver - shot speed x1.4 | replaces nothing"
            && drawn == offer.chars().count(),
        "decision 3 equipment: the pickup line does not say what the Driver does",
        format!("line 6 {offer:?}, {drawn} glyphs drawn"),
    );
    driver.tap(Key::F);
    let view = driver.snap();
    let kit = view.golfer(0).map_or(Kit::default(), |g| g.kit);
    let gone = !view.pickups.iter().any(|(p, _)| p.distance(at) < 1e-3);
    checks.require(
        kit.club == Some(Item::Driver) && gone && effects(&kit).shot_scale == 1.4,
        "decision 3 equipment: F did not take the Driver",
        format!(
            "kit {kit:?}, pickup gone {gone}, shot scale {}",
            effects(&kit).shot_scale
        ),
    );
    edit_golfer(&mut driver, 0, |t, g| {
        t.pos = Vec2::new(-29.0, -8.0);
        g.aim = Radians::ZERO;
        g.power = Power::Chip;
    });
    place_ball(&mut driver, 0, Vec2::new(-28.0, -8.0), Vec2::ZERO);
    driver.idle();
    driver.tap(Key::Space);
    for _ in 0..600 {
        if driver.snap().ball(0).is_some_and(|b| b.vel == Vec2::ZERO) {
            break;
        }
        driver.idle();
    }
    let rest = driver.snap().ball(0).map_or(Vec2::ZERO, |b| b.pos);
    let predicted = roll_out(
        Vec2::new(-28.0, -8.0),
        Vec2::new(25.2, 0.0),
        Seconds(1.0 / 60.0),
        0,
    )
    .rest;
    let roll = rest.x + 28.0;
    checks.require(
        rest.distance(predicted) < 1e-3 && (roll - 19.63).abs() < 0.02,
        "decision 3 equipment: a CHIP with the Driver did not roll as stated",
        format!("rested at {rest:?} ({roll:.2} along), predicted {predicted:?}; 19.63 expected"),
    );
    let feet = driver.snap().golfer(0).map_or(Vec2::ZERO, |g| g.pos);
    let world = driver.world_mut();
    let pickup = world.spawn();
    world.insert(pickup, Transform::at(feet));
    world.insert(
        pickup,
        Pickup {
            item: Item::LongClub,
        },
    );
    driver.idle();
    let swap = line6(&driver);
    driver.tap(Key::F);
    let view = driver.snap();
    let club = view.golfer(0).and_then(|g| g.kit.club);
    let left = view
        .pickups
        .iter()
        .any(|(p, item)| *item == Item::Driver && p.distance(feet) < 1e-3);
    checks.require(
        swap.ends_with("| replaces Driver") && club == Some(Item::LongClub) && left,
        "decision 3 equipment: a second club did not replace the first and leave it behind",
        format!("line 6 {swap:?}; club {club:?}; Driver left at the feet {left}"),
    );
    format!("decision 3 equipment: {offer:?}; CHIP with Driver rolled {roll:.2}; swap {swap:?}")
}

/// The font quads of one banner row, as a box.
fn banner_box(
    frame: &FrameRecord,
    font: jidousha::testing::BackendTextureId,
    y: f32,
) -> Option<Rect> {
    find_bounds(frame.quads().into_iter().filter(|q| {
        q.texture == font
            && (q.bounds().min.y - y).abs() < 1e-3
            && (q.bounds().size().y - BANNER_SIZE).abs() < 1e-3
    }))
}

/// Row 4: extracting keeps exactly what the status line promised.
pub fn extraction(checks: &mut Checks, shots: &mut Vec<(String, FrameRecord)>) -> String {
    let mut driver = Driver::new();
    driver.idle();
    freeze(&mut driver, &[1, 2, 3]);
    edit_golfer(&mut driver, 0, |_, g| {
        g.kit = Kit::of(&[Item::Driver, Item::Helmet, Item::LeadBall])
    });
    let mut promise = String::new();
    let mut last_live = None;
    while driver.tick < 3000 && driver.snap().result.is_none() {
        let view = driver.snap();
        promise = hud_lines_of(&view)[2].clone();
        let held = view.golfer(0).map_or(0, |g| g.extract_ticks);
        if held > EXTRACT_HOLD - 5 {
            last_live = Some(driver.draw());
        }
        driver.play(Player::Extractor);
    }
    let ended = driver.tick;
    let reached = driver.snap().golfer(0).map_or(0, |g| g.extract_ticks);
    let result = driver.world().resource::<Match>().result.clone();
    let frame = driver.draw();
    let font = driver.font();
    let kit = Kit::of(&[Item::Driver, Item::Helmet, Item::LeadBall]);
    let Some(outcome) = result else {
        checks.require(
            false,
            "decision 4 extraction: the extractor never got out",
            format!("by tick {ended}: {promise:?}"),
        );
        return "decision 4 extraction: did not extract".to_owned();
    };
    let lines = result_lines(&outcome);
    checks.require(
        promise.starts_with("EXTRACT keeps Driver+Lead Ball = 5")
            && outcome.kind == EndKind::Extracted
            && lines == ["EXTRACTED", "kept: Driver, Lead Ball", "points: 5"].map(str::to_owned)
            && outcome.kept == kept_on_extract(&kit),
        "decision 4 extraction: the result is not what the status line promised",
        format!("promised {promise:?}; result {outcome:?}; banner {lines:?}"),
    );
    let promised: Vec<&str> = promise
        .trim_start_matches("EXTRACT keeps ")
        .split(" = ")
        .next()
        .unwrap_or("")
        .split('+')
        .collect();
    let shown: Vec<&str> = lines[1].trim_start_matches("kept: ").split(", ").collect();
    let points = promise
        .split(" = ")
        .nth(1)
        .and_then(|p| p.split(' ').next())
        .unwrap_or("");
    checks.require(
        promised == shown && lines[2] == format!("points: {points}") && reached == 120,
        "decision 4 extraction: the banner keeps something other than the status line said",
        format!(
            "promised {promised:?} for {points}; banner {shown:?} {:?}; held {reached} ticks",
            lines[2]
        ),
    );
    for (row, y) in BANNER_Y.iter().enumerate() {
        let found = banner_box(&frame, font, *y);
        let centred = found.is_some_and(|b| b.center().x.abs() < 0.05);
        checks.require(
            centred,
            "decision 4 extraction: a banner row is not drawn centred",
            format!("row {row} at y {y}: {found:?}"),
        );
    }
    if let Some(live) = last_live {
        shots.push(("extraction, holding in the gate".to_owned(), live));
    }
    shots.push(("extraction result".to_owned(), frame));
    format!(
        "decision 4 extraction: {promise:?} -> {:?} on tick {ended}",
        lines.join(" / ")
    )
}

/// Every ending's banner, staged: on screen, centred, five different screens.
pub fn staged_screens(checks: &mut Checks, shots: &mut Vec<(String, FrameRecord)>) -> String {
    let kit = Kit::of(&[Item::Driver, Item::Helmet, Item::LeadBall]);
    let kinds = [
        EndKind::Holed,
        EndKind::LastStanding,
        EndKind::Extracted,
        EndKind::Eliminated,
        EndKind::Lost { by: 1 },
    ];
    let mut firsts = Vec::new();
    for kind in kinds {
        let mut driver = Driver::new();
        driver.idle();
        let outcome = outcome_of(kind, &kit);
        driver.world_mut().resource_mut::<Match>().result = Some(outcome.clone());
        let frame = driver.draw();
        let font = driver.font();
        let lines = result_lines(&outcome);
        for (line, y) in lines.iter().zip(BANNER_Y) {
            let found = banner_box(&frame, font, y);
            checks.require(
                found.is_some_and(|b| b.center().x.abs() < 0.05)
                    && line.chars().all(|c| (' '..='~').contains(&c)),
                "staged screens: a banner row is missing, off centre or unprintable",
                format!("{kind:?}: {line:?} drawn at {found:?}"),
            );
        }
        firsts.push(lines[0].clone());
        shots.push((format!("staged {kind:?}"), frame));
    }
    let mut unique = firsts.clone();
    unique.sort();
    unique.dedup();
    checks.require(
        unique.len() == firsts.len(),
        "staged screens: two endings show the same headline",
        format!("headlines {firsts:?}"),
    );
    format!("staged screens: {firsts:?}")
}

/// The contracts no played session reaches.
pub fn contracts(checks: &mut Checks) -> String {
    let dt = Seconds(1.0 / 60.0);
    let wall = ball_step(Vec2::new(29.5, 2.0), Vec2::new(30.0, 0.0), dt);
    checks.require(
        wall.pos.x <= COURSE.max.x - BALL_RADIUS && wall.vel.x < 0.0,
        "contracts: a ball at the edge did not bounce back inside",
        format!("{wall:?}"),
    );
    let near = CUP + Vec2::new(0.3, 0.0);
    let slow = roll_out(near, Vec2::new(-2.0, 0.0), dt, 7200);
    let fast = roll_out(near, Vec2::new(-12.0, 0.0), dt, 7200);
    let shut = roll_out(near, Vec2::new(-2.0, 0.0), dt, 7000);
    checks.require(
        slow.sunk && !fast.sunk && !shut.sunk,
        "contracts: the cup does not take a slow ball once open, refuse a fast one, and stay shut before",
        format!("slow {slow:?}; fast {fast:?}; before tick 7200 {shut:?}"),
    );
    let start = Vec2::new(-28.0, -8.0);
    let chip = roll_out(start, Vec2::new(18.0, 0.0), dt, 0).rest.x - start.x;
    let putt = roll_out(start, Vec2::new(10.0, 0.0), dt, 0).rest.x - start.x;
    checks.require(
        (chip - 9.98).abs() < 0.02 && (putt - 3.04).abs() < 0.02,
        "contracts: a CHIP and a PUTT do not roll the stated distances",
        format!("CHIP {chip:.3}, PUTT {putt:.3}; 9.98 and 3.04 stated"),
    );
    let shrinking = (1..=11_400u64).all(|t| zone_at(t).radius <= zone_at(t - 1).radius);
    let scanned = (0..20_000u64).find(|&t| !inside_zone(zone_at(t), GATE_CENTER));
    checks.require(
        shrinking && zone_excludes_at(GATE_CENTER) == scanned,
        "contracts: the zone grows somewhere, or the gate's closing tick is wrong",
        format!(
            "non-increasing {shrinking}; solved {:?}, scanned {scanned:?}",
            zone_excludes_at(GATE_CENTER)
        ),
    );
    let fastest = Power::Drive.speed() * 1.4 * dt.as_f32();
    checks.require(
        fastest < 2.0 * CUP_RADIUS,
        "contracts: the fastest ball can step over the cup in one tick",
        format!(
            "{fastest:.3} per tick against a cup {:.2} across",
            2.0 * CUP_RADIUS
        ),
    );
    let kept = kept_on_extract(&Kit::of(&[Item::Driver, Item::LeadBall, Item::Helmet]));
    checks.require(
        kept == [Item::Driver, Item::LeadBall],
        "contracts: extraction does not keep the two most valuable, ties by slot",
        format!("kept {kept:?}"),
    );
    let outside = clip_to_course(Vec2::new(-40.0, -20.0), Vec2::new(40.0, -20.0));
    let inner = clip_to_course(Vec2::new(-1.0, 0.0), Vec2::new(1.0, 1.0));
    let crossing = clip_to_course(Vec2::new(0.0, 0.0), Vec2::new(50.0, 0.0));
    let held = |p: Vec2| {
        p.x >= COURSE.min.x - 1e-4
            && p.x <= COURSE.max.x + 1e-4
            && p.y >= COURSE.min.y - 1e-4
            && p.y <= COURSE.max.y + 1e-4
    };
    checks.require(
        outside.is_none()
            && inner == Some((Vec2::new(-1.0, 0.0), Vec2::new(1.0, 1.0)))
            && crossing.is_some_and(|(a, b)| held(a) && held(b) && (b.x - 30.0).abs() < 1e-4),
        "contracts: clipping to the course is wrong",
        format!("outside {outside:?}, inside {inner:?}, crossing {crossing:?}"),
    );
    format!(
        "contracts: CHIP {chip:.2}, PUTT {putt:.2}, gate closes {scanned:?}, fastest step {fastest:.2}"
    )
}
