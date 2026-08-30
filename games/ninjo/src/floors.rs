//! The readability floors, as assertions rather than as advice — giri's
//! UI.md §7 machinery, carried into the fork whole (the fork's UI.md says
//! which surfaces they now bind: clock, chips, labels, tokens, log).
//!
//! Stated **at reference scale**: UI space is 960x540 and one UI unit is one
//! reference pixel, so a floor and the number UI.md writes are the same
//! number. The map camera never changes that — the chrome rides `UiMap` and
//! is a constant size on screen at any zoom.

use jidousha::prelude::*;

use crate::camera::UiMap;
use crate::checks::{Checks, greater, near};
use crate::clock::Clock;
use crate::constants::Tuning;
use crate::flow::Flow;
use crate::grid::LOCATIONS;
use crate::sim::Sim;
use crate::sweep::Conducted;
use crate::ui::Panel;
use crate::{camera, layout, screens, theme, verify};

/// Whether `bounds` sits inside `area`, to within a hundredth of a unit.
pub fn inside(area: Rect, bounds: Rect) -> bool {
    const SLACK: f32 = 0.01;
    bounds.min.x >= area.min.x - SLACK
        && bounds.min.y >= area.min.y - SLACK
        && bounds.max.x <= area.max.x + SLACK
        && bounds.max.y <= area.max.y + SLACK
}

/// Every rectangle a click does something in, with the name a message uses.
pub fn targets() -> Vec<(String, Rect)> {
    let mut out: Vec<(String, Rect)> = Vec::new();
    for (index, label) in screens::chip_labels().into_iter().enumerate() {
        out.push((format!("the {label} chip"), layout::speed_chip(index)));
    }
    out.push(("the log drawer's handle".to_owned(), layout::log_button()));
    out.push((
        "the tuning drawer's handle".to_owned(),
        layout::tune_button(),
    ));
    for index in 0..Sim::opening().parties.len() {
        out.push((format!("party chip {index}"), layout::party_chip(index)));
    }
    out
}

/// Every rectangle the tuning drawer answers a click in — a set of its own,
/// because the drawer covers the screen and the overlap floor is about
/// siblings.
pub fn tuner_targets() -> Vec<(String, Rect)> {
    let mut out: Vec<(String, Rect)> = Vec::new();
    for (index, preset) in crate::presets::PRESETS.iter().enumerate() {
        out.push((
            format!("the {} preset", preset.name),
            layout::tuner_preset(index),
        ));
    }
    for (index, field) in crate::constants::Field::ALL.iter().copied().enumerate() {
        out.push((format!("{}'s -", field.name()), layout::tuner_minus(index)));
        out.push((format!("{}'s +", field.name()), layout::tuner_plus(index)));
    }
    out.push(("the APPLY verb".to_owned(), layout::tuner_apply()));
    out
}

/// The floors that are questions about the layout alone.
pub fn layout_floors(checks: &mut Checks) {
    for (what, rect) in targets() {
        let size = rect.size();
        checks.require(
            !greater(theme::MIN_TARGET, size.x) && !greater(theme::MIN_TARGET, size.y),
            "a clickable target is smaller than the readability floor allows",
            format!(
                "{what} is {:.0}x{:.0} reference pixels and the floor is {}x{}",
                size.x,
                size.y,
                theme::MIN_TARGET,
                theme::MIN_TARGET
            ),
        );
        checks.require(
            inside(layout::design(), rect),
            "a clickable target is partly off the UI rect",
            format!(
                "{what} is {rect:?} and the UI rect is {:?}",
                layout::design()
            ),
        );
    }
    let interactive = targets();
    for (index, (what, rect)) in interactive.iter().enumerate() {
        for (other_what, other) in interactive.iter().skip(index + 1) {
            checks.require(
                !rect.overlaps(*other),
                "two interactive rectangles overlap",
                format!("{what} at {rect:?} overlaps {other_what} at {other:?}"),
            );
        }
    }
    // The chips and handles live in the top bar; the party chips in the
    // strip. A control outside its band is a control over the map.
    for index in 0..layout::CHIPS {
        checks.require(
            inside(layout::topbar(), layout::speed_chip(index)),
            "a speed chip is outside the status bar",
            format!("chip {index} at {:?}", layout::speed_chip(index)),
        );
    }
    for index in 0..Sim::opening().parties.len() {
        checks.require(
            inside(layout::party_strip(), layout::party_chip(index)),
            "a party chip runs off the strip it belongs to",
            format!("chip {index} at {:?}", layout::party_chip(index)),
        );
    }
    // The site markers, in world units: at the reference camera one world
    // unit is one reference pixel, so the marker floor is the target floor.
    for spec in LOCATIONS {
        let marker = layout::marker_rect(spec.tile);
        checks.require(
            !greater(theme::MIN_TARGET, marker.size().x)
                && !greater(theme::MIN_TARGET, marker.size().y),
            "a site marker is smaller than the target floor at reference zoom",
            format!("{}'s marker is {:?}", spec.name, marker),
        );
    }
}

/// The floors over the tuning drawer's own controls.
pub fn tuner_floors(checks: &mut Checks) {
    let drawer = layout::tuner_panel();
    for (what, rect) in tuner_targets() {
        let size = rect.size();
        checks.require(
            !greater(theme::MIN_TARGET, size.x) && !greater(theme::MIN_TARGET, size.y),
            "a tuning control is smaller than the readability floor allows",
            format!(
                "{what} is {:.0}x{:.0} reference pixels and the floor is {}x{}",
                size.x,
                size.y,
                theme::MIN_TARGET,
                theme::MIN_TARGET
            ),
        );
        checks.require(
            inside(drawer, rect),
            "a tuning control is outside the drawer that holds it",
            format!("{what} is {rect:?} and the drawer is {drawer:?}"),
        );
    }
    let controls = tuner_targets();
    for (index, (what, rect)) in controls.iter().enumerate() {
        for (other_what, other) in controls.iter().skip(index + 1) {
            checks.require(
                !rect.overlaps(*other),
                "two tuning controls overlap",
                format!("{what} at {rect:?} overlaps {other_what} at {other:?}"),
            );
        }
    }
    checks.require(
        !layout::tune_button().overlaps(drawer),
        "the tuning drawer covers its own handle",
        format!(
            "the handle is {:?} and the drawer is {drawer:?}",
            layout::tune_button()
        ),
    );
}

/// The UI-mapping contract — giri's scaling contract, restated over a camera
/// that moves: the chrome rect fits inside the view uniformly, centred, at
/// every viewport and every legal zoom, and reads scale 1 at the reference
/// surface and default zoom.
pub fn uimap_contract(checks: &mut Checks) -> String {
    let mut notes = Vec::new();
    for (what, viewport) in [
        ("reference", verify::HEADLESS_VIEWPORT),
        ("narrow", verify::NARROW_VIEWPORT),
        ("short", PhysicalSize::new(1280, 300)),
        ("tiny", PhysicalSize::new(200, 160)),
    ] {
        for height in [camera::MIN_H, camera::DEFAULT_H, camera::MAX_H] {
            let camera = Camera {
                height,
                ..camera::camera_for(viewport)
            };
            let map = UiMap::for_camera(&camera);
            let view = camera.visible_bounds();
            let chrome = map.to_world_rect(layout::design());
            checks.require(
                inside(view, chrome),
                "the chrome does not fit inside the camera's view",
                format!(
                    "at {what} ({}x{}) height {height}: the chrome maps to {chrome:?} and the \
                     view is {view:?}",
                    viewport.width, viewport.height
                ),
            );
            checks.require(
                near(chrome.min.x - view.min.x, view.max.x - chrome.max.x)
                    && near(chrome.min.y - view.min.y, view.max.y - chrome.max.y),
                "the chrome is not centred in the view",
                format!(
                    "at {what} height {height}: spare span {:.2}/{:.2} across and {:.2}/{:.2} \
                     down",
                    chrome.min.x - view.min.x,
                    view.max.x - chrome.max.x,
                    chrome.min.y - view.min.y,
                    view.max.y - chrome.max.y
                ),
            );
            // The round trip a hit-test rides.
            let probe = Vec2::new(123.0, 456.0);
            let back = map.ui_of(map.to_world(probe));
            checks.require(
                (back - probe).length() < 0.01,
                "the UI mapping does not round-trip",
                format!("{probe:?} maps back to {back:?} at {what} height {height}"),
            );
        }
        let default_map = UiMap::for_camera(&camera::camera_for(viewport));
        notes.push(format!(
            "{what} {}x{} = {:.3}x",
            viewport.width, viewport.height, default_map.scale
        ));
    }
    let reference = UiMap::for_camera(&camera::camera_for(verify::HEADLESS_VIEWPORT));
    checks.require(
        near(reference.scale, 1.0),
        "the reference surface at default zoom is no longer reference scale",
        format!(
            "the UI scale there is {:.4}, and every floor is stated at 1.0",
            reference.scale
        ),
    );
    notes.join(", ")
}

/// The screen states the content floors judge, built from a conducted run.
pub fn content_states(baseline: &Conducted) -> Vec<(&'static str, Flow, Sim, Clock)> {
    let opening = (
        "the opening screen",
        Flow::default(),
        Sim::opening(),
        Clock::opening(),
    );
    // The end of the conducted run: full log, everything home.
    let mut logged = Flow::default();
    for event in &baseline.events {
        let line = event.line(&baseline.sim);
        logged.note(line);
    }
    let mut log_open = logged.clone();
    log_open.log_open = true;
    let mut ended_clock = Clock::opening();
    ended_clock.minutes = baseline.minutes;
    // A toast up, a party picked — the strip's loudest state.
    let mut toasted = logged.clone();
    toasted.selected = Some(0);
    toasted.toast = Some(crate::flow::Toast {
        text: crate::sim::Refusal::NotIdle.message("CRANE", "the Black Vault"),
        until: u64::MAX,
    });
    vec![
        opening,
        (
            "the ended run with the log open",
            log_open,
            baseline.sim.clone(),
            ended_clock,
        ),
        (
            "the strip with a toast and a pick",
            toasted,
            baseline.sim.clone(),
            ended_clock,
        ),
    ]
}

/// The content floors: every row of every screen state at or above the text
/// floor, inside its rect, and never lying across a control it is not the
/// label of.
pub fn content_floors(checks: &mut Checks, baseline: &Conducted) {
    let tuning = Tuning::SHIPPED;
    for (what, flow, sim, clock) in content_states(baseline) {
        let panel = screens::content(&flow, &sim, &clock, &tuning);
        judge_panel(checks, &panel, what, flow.tuner.open);
    }
}

/// One panel against the floors.
pub fn judge_panel(checks: &mut Checks, panel: &Panel, what: &str, tuner_open: bool) {
    let map_rect = crate::grid::grid().world_rect();
    for text in panel.runs.iter().chain(panel.world_runs.iter()) {
        checks.require(
            !greater(theme::MIN_TEXT, text.size),
            "a row of text is smaller than the readability floor allows",
            format!(
                "{what}: {:?} is set at {:.1} reference pixels and the floor is {:.0}",
                text.text,
                text.size,
                theme::MIN_TEXT
            ),
        );
    }
    for text in &panel.runs {
        checks.require(
            inside(layout::design(), text.bounds()),
            "a row of chrome text runs off the UI rect",
            format!("{what}: {:?} occupies {:?}", text.text, text.bounds()),
        );
        // Nothing lies across a control it is not the label of.
        let controls: Vec<(String, Rect)> = if tuner_open {
            tuner_targets()
        } else {
            targets()
        };
        for (control, target) in controls {
            if !text.bounds().overlaps(target) {
                continue;
            }
            checks.require(
                inside(target, text.bounds()),
                "a row of text lies across a control it is not the label of",
                format!(
                    "{what}: {:?} at {:?} crosses {control} at {target:?}",
                    text.text,
                    text.bounds()
                ),
            );
        }
    }
    for text in &panel.world_runs {
        checks.require(
            inside(map_rect, text.bounds()),
            "a map label runs off the world",
            format!("{what}: {:?} occupies {:?}", text.text, text.bounds()),
        );
    }
    // Map labels never collide with each other — the authored placement's
    // own floor.
    for (index, text) in panel.world_runs.iter().enumerate() {
        for other in panel.world_runs.iter().skip(index + 1) {
            checks.require(
                !text.bounds().overlaps(other.bounds()),
                "two map labels overlap",
                format!(
                    "{what}: {:?} at {:?} and {:?} at {:?}",
                    text.text,
                    text.bounds(),
                    other.text,
                    other.bounds()
                ),
            );
        }
    }
    for icon in panel.icons.iter().chain(panel.world_icons.iter()) {
        checks.require(
            near(icon.scale, icon.scale.round()),
            "a pixel-art icon is drawn at a fractional scale",
            format!(
                "{what}: {:?} is drawn at {:.2}x, and the engine samples nearest - a fraction \
                 puts a wobble in it",
                icon.art, icon.scale
            ),
        );
    }
    for icon in &panel.icons {
        checks.require(
            inside(layout::design(), icon.bounds()),
            "a chrome icon runs off the UI rect",
            format!("{what}: {:?} occupies {:?}", icon.art, icon.bounds()),
        );
    }
    // The redundancy floor: the treasury's number has its coin beside it.
    if let Some(gold) = panel
        .runs
        .iter()
        .find(|run| run.at == layout::treasury_text_at())
    {
        let coin = panel
            .icons
            .iter()
            .find(|icon| icon.art == crate::sprites::Art::Coin);
        let adjacent = coin.is_some_and(|coin| {
            let gap = gold.bounds().min.x - coin.bounds().max.x;
            !greater(gap, theme::SMALL) && !greater(-1.0, gap)
        });
        checks.require(
            adjacent,
            "the treasury's number has no coin beside it",
            format!(
                "{what}: the gold reads at {:?} and the coin icon is {:?}",
                gold.at,
                coin.map(|icon| icon.at)
            ),
        );
    }
}

/// The frame floor: no glyph drawn below the text floor, on a reference
/// frame at default zoom (where one world unit is one reference pixel).
pub fn judge_frame_floor(
    checks: &mut Checks,
    font: jidousha::testing::BackendTextureId,
    frame: &jidousha::testing::FrameRecord,
    what: &str,
) {
    let smallest = frame
        .quads()
        .iter()
        .filter(|quad| quad.texture == font)
        .map(|quad| quad.bounds().size().y)
        .fold(f32::MAX, f32::min);
    checks.require(
        smallest == f32::MAX || !greater(theme::MIN_TEXT - 0.01, smallest),
        "a glyph was drawn below the readability floor",
        format!(
            "{what}: the shortest glyph quad is {smallest:.2} reference pixels and the floor \
             is {:.0}",
            theme::MIN_TEXT
        ),
    );
}

/// The drawer's own screens against the floors — the states no played
/// scenario reaches: pending, refused-link, and just-applied.
pub fn judge_tuner_screen(checks: &mut Checks, drawer: &crate::restart::DrawerRun) {
    let tuning_states = [
        ("the drawer with a pending set", &drawer.pending_flow),
        ("the drawer after the APPLY", &drawer.applied_flow),
    ];
    for (what, flow) in tuning_states {
        let panel = screens::content(
            flow,
            &drawer.applied_sim,
            &Clock::opening(),
            &drawer.applied_active,
        );
        judge_panel(checks, &panel, what, flow.tuner.open);
    }
    // The refused-link state, staged: the longest refusal in the hint row.
    let mut refused = drawer.pending_flow.clone();
    refused.tuner.fault = crate::links::refusals().into_iter().max_by_key(String::len);
    let panel = screens::content(
        &refused,
        &drawer.applied_sim,
        &Clock::opening(),
        &drawer.pending_active,
    );
    judge_panel(checks, &panel, "the drawer with a refused link", true);
    if let Some(shot) = &drawer.shot {
        judge_frame_floor(checks, drawer.font, &shot.frame, "the tuning drawer");
    }
}
