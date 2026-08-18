//! The instrument: an accumulator for failed checks, and everything this game
//! asks about a frame it drew.
//!
//! Nobody running `--verify` can look at the game, so these messages are the
//! only instrument there is. Two rules follow from that and are worth stating
//! because both cost a cycle to learn: a check reports the numbers it judged
//! rather than the conclusion it reached, and a failed check does not stop the
//! run — an instrument that halts at the first bad reading costs a whole cycle
//! per fault.

use std::cmp::Ordering;
use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{FrameRecord, FrameRecorder, PhysicalSize};

use crate::draw;
use crate::{BALL_RADIUS, HINT, WINNING_SCORE};
use crate::{Play, Scoreboard, Side, config, register};

/// Slack for a float comparison against a layout constant.
pub(crate) const EPSILON: f32 = 1e-3;

/// The camera's viewport, which the recorder has to be given too.
///
/// The recorder's viewport *overrides* the world's, and nothing writes it back,
/// so a check reading bounds from the `Camera` resource and quads from the
/// recorder judges against the wrong rectangle unless the two agree.
pub(crate) const VIEWPORT: PhysicalSize = PhysicalSize::new(1280, 720);

/// Every failed check, kept rather than exited on.
///
/// Nobody running this can look at the game, so the run is the only instrument
/// there is — and an instrument that stops at the first bad reading costs a
/// whole cycle per fault. Each entry prints in the engine's four-part shape,
/// and each one prints the numbers it judged rather than its conclusion.
#[derive(Default)]
pub(crate) struct Checks {
    problems: Vec<(String, String)>,
}

impl Checks {
    pub(crate) fn require(&mut self, ok: bool, what: &str, specifics: String) {
        if !ok {
            self.problems.push((what.to_string(), specifics));
        }
    }

    pub(crate) fn verdict(&self) -> ExitCode {
        if self.problems.is_empty() {
            return ExitCode::SUCCESS;
        }
        for (what, specifics) in &self.problems {
            eprintln!(
                "{}",
                message(
                    what,
                    specifics,
                    "the game's rules, its layout constants, or the engine underneath them \
                     changed",
                    "run `cargo run -p jidousha --example pong`, watch that same situation, \
                     and compare it with the numbers above",
                )
            );
        }
        ExitCode::FAILURE
    }
}

/// `a` and `b` within `EPSILON`, and false if either is NaN.
///
/// Through `partial_cmp` rather than as `!(x > y)`, because negating a float
/// comparison quietly admits NaN — and a NaN that crept into a position would
/// satisfy every bound in this file.
pub(crate) fn near(a: f32, b: f32) -> bool {
    matches!((a - b).abs().partial_cmp(&EPSILON), Some(Ordering::Less))
}

/// The score reads as two digits either side of the middle.
///
/// The "a glyph covers the middle of this cell" checks elsewhere read the same
/// layout constants the drawing does, so they say the two agree and nothing
/// about whether the layout is any good — move `SCORE_X` to 1.5 and both still
/// pass while the two digits sit on top of the centre line reading as one
/// number. This asks the frame instead: how many glyphs are in the score band,
/// which side of the middle each one is, and how much air is between them.
pub(crate) fn check_the_score_reads_as_two_numbers(
    checks: &mut Checks,
    frame: &FrameRecord,
    font: jidousha::testing::BackendTextureId,
) {
    let band = draw::SCORE_TOP..draw::SCORE_TOP + draw::SCORE_SIZE;
    let digits: Vec<Rect> = frame
        .quads()
        .iter()
        .filter(|quad| quad.texture == font)
        .map(|quad| quad.bounds())
        .filter(|bounds| band.contains(&bounds.min.y) && band.contains(&bounds.center().y))
        .collect();
    checks.require(
        digits.len() == 2,
        "the score is not two digits",
        format!(
            "{} glyph quads fall in the score's band ({:.2} to {:.2}); one number each side of \
             the middle is the whole of this game's score",
            digits.len(),
            band.start,
            band.end
        ),
    );
    let left = digits
        .iter()
        .filter(|bounds| bounds.center().x < 0.0)
        .count();
    checks.require(
        left == 1 && digits.len() - left == 1,
        "the two score digits are not one to each side",
        format!("{left} of {} are left of the middle", digits.len()),
    );
    // And there is a number's worth of air between them, so they read as two
    // scores rather than as one two-digit number.
    let gap = digits
        .iter()
        .map(|bounds| bounds.min.x.max(-bounds.max.x))
        .fold(f32::INFINITY, f32::min)
        * 2.0;
    checks.require(
        gap >= draw::SCORE_SIZE,
        "the two score digits are too close to read as two scores",
        format!(
            "{gap:.2} units of air between them, against digits {:.2} tall; they meet over the \
             centre line and read as one number",
            draw::SCORE_SIZE
        ),
    );
}

/// The camera the game is actually drawing through, with the recorder's
/// viewport in place of the world's.
pub(crate) fn camera_of(sim: &HeadlessSim) -> Camera {
    Camera {
        viewport: VIEWPORT,
        ..*sim.world().resource::<Camera>()
    }
}

/// Nothing is drawn outside the camera.
///
/// The single highest-value check a game of shapes and text can make, and the
/// one a mis-centred banner trips. `contains_rect` is closed on all four sides,
/// because a quad flush against the camera's edge is on screen.
pub(crate) fn check_nothing_off_screen(
    checks: &mut Checks,
    sim: &HeadlessSim,
    frame: &FrameRecord,
    tick: u64,
) {
    let view = camera_of(sim).visible_bounds();
    for quad in frame.quads() {
        let bounds = quad.bounds();
        if !view.contains_rect(bounds) {
            checks.require(
                false,
                "something was drawn off screen",
                format!(
                    "on tick {tick} a quad spanning {bounds:?} went to a camera showing \
                     {view:?}; text centred with TextStyle::width_of is the usual culprit"
                ),
            );
            // One report is a diagnosis. A thousand is noise, and they would all
            // be the same fault.
            return;
        }
    }
}

/// The size of the disc drawn at `at`, or `None` if nothing ball-shaped is.
///
/// `ctx.circle` submits sixteen wedges, not one square, so nothing the size of
/// the ball is drawn anywhere and "a quad the size of the thing" is the wrong
/// question. What is true is that all sixteen share the centre as a corner and
/// all sixteen fit inside the disc's box, so the union of the quads covering
/// the centre is exactly `2r` square.
pub(crate) fn disc_at(frame: &FrameRecord, at: Vec2) -> Option<Vec2> {
    let box_of_it = Rect::from_center_size(at, Vec2::splat(BALL_RADIUS * 2.0));
    let mut union: Option<Rect> = None;
    for quad in frame.covering(at) {
        let drawn = quad.bounds();
        // Written out rather than as `Rect::contains`, which is half-open and
        // would throw away the one wedge reaching the far edge.
        let inside = drawn.min.x >= box_of_it.min.x - EPSILON
            && drawn.min.y >= box_of_it.min.y - EPSILON
            && drawn.max.x <= box_of_it.max.x + EPSILON
            && drawn.max.y <= box_of_it.max.y + EPSILON;
        if !inside {
            continue; // The table behind the ball, not the ball.
        }
        union = Some(match union {
            None => drawn,
            Some(so_far) => Rect {
                min: so_far.min.min(drawn.min),
                max: so_far.max.max(drawn.max),
            },
        });
    }
    union.map(|rect| rect.size())
}

/// Every string this game can draw, checked against what the font can show.
///
/// No assertion over drawn quads can see a wrong character: the font covers
/// space through `~` and draws everything else as a box at exactly a letter's
/// advance, so glyph counts, `width_of` centring and the off-screen check above
/// all pass identically on a stray em dash. The check has to look at the
/// string.
pub(crate) fn check_every_literal(checks: &mut Checks) {
    let mut strings = vec![
        ("the hint line", HINT.to_string()),
        ("the banner's prompt", draw::BANNER_PROMPT.to_string()),
    ];
    // Both banners, including the one a winning run never draws.
    for winner in [Side::Left, Side::Right] {
        let board = Scoreboard {
            left: WINNING_SCORE,
            right: WINNING_SCORE - 2,
            play: Play::Over { winner },
            ..Scoreboard::new()
        };
        if let Some(verdict) = draw::banner_verdict(&board) {
            strings.push(("a banner verdict", verdict));
        }
    }
    // And every score the table can ever show.
    for points in 0..=WINNING_SCORE {
        strings.push(("a score digit", points.to_string()));
    }
    for (name, text) in strings {
        checks.require(
            text.chars().all(|c| (' '..='~').contains(&c)),
            "a string the game draws is outside the font's range",
            format!(
                "{name} is {text:?}; the font covers space through '~' and draws anything else \
                 as a box of exactly a letter's size, so nothing about what was drawn can tell \
                 the difference"
            ),
        );
    }
}

/// The screens the run never reached.
///
/// The off-screen check only judges frames that were drawn, and a controller
/// good enough to finish the match is a controller that never loses it: the
/// losing banner is the one string in the game that nothing measured. Three
/// lines per screen — tick once so `Startup` has run, set the resource that
/// selects the screen, draw one frame — and the same check runs over it.
pub(crate) fn check_the_screens_never_reached(checks: &mut Checks, recorder: &mut FrameRecorder) {
    for (name, board) in [
        (
            "the losing banner",
            Scoreboard {
                left: WINNING_SCORE - 2,
                right: WINNING_SCORE,
                play: Play::Over {
                    winner: Side::Right,
                },
                ..Scoreboard::new()
            },
        ),
        (
            "the widest banner this game can draw",
            Scoreboard {
                left: WINNING_SCORE,
                right: 0,
                play: Play::Over { winner: Side::Left },
                ..Scoreboard::new()
            },
        ),
    ] {
        let mut sim = headless(config(), register);
        sim.tick(); // Startup, so the world exists.
        sim.world_mut().insert_resource(board);
        let frame = recorder.draw(&mut sim);
        let view = camera_of(&sim).visible_bounds();
        for quad in frame.quads() {
            let bounds = quad.bounds();
            if !view.contains_rect(bounds) {
                checks.require(
                    false,
                    "a screen the run never reaches is drawn off the camera",
                    format!(
                        "{name} put a quad spanning {bounds:?} on a camera showing {view:?}; \
                         no frame of the played match would have shown it"
                    ),
                );
                break;
            }
        }
    }
}
