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
use crate::lens::Lens;
use crate::sim::Sim;
use crate::sweep::{Conducted, Shot};
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

/// The base screen's controls with a site's job board up instead of the faces
/// list.
///
/// The board and the faces list share the left of the screen and are never
/// open together (`flow.rs`: opening either shuts the other), so this is the
/// other set of siblings — the character panel, the strip and the bar are in
/// both, because a dispatch has the board and the panel up at once.
pub fn board_targets() -> Vec<(String, Rect)> {
    let mut out: Vec<(String, Rect)> = base_targets();
    out.push(("the job board's close".to_owned(), layout::board_close()));
    out.push(("the board's fit chip".to_owned(), layout::board_fit_chip()));
    for slot in 0..layout::BOARD_ROWS {
        out.push((format!("job row {slot}"), layout::board_row(slot)));
    }
    // The board's own two controls (wave 1.2): what the next tap offers, and
    // whom it offers it to.
    out.push((
        "the board's wage down".to_owned(),
        layout::board_wage_down(),
    ));
    out.push(("the board's wage up".to_owned(), layout::board_wage_up()));
    out.push(("the board's who toggle".to_owned(), layout::board_to()));
    out
}

/// Every rectangle the postings ledger answers a click in: a withdrawal per
/// standing posting, and the standing rates' own steppers and postings.
pub fn ledger_targets() -> Vec<(String, Rect)> {
    let mut out = Vec::new();
    for row in 0..layout::LEDGER_ROWS {
        out.push((
            format!("ledger row {row}'s withdraw"),
            layout::ledger_withdraw(row),
        ));
    }
    for (row, task) in crate::traits::TaskType::ALL.iter().copied().enumerate() {
        out.push((
            format!("the {} rate's -", task.id()),
            layout::rates_down(row),
        ));
        out.push((format!("the {} rate's +", task.id()), layout::rates_up(row)));
        out.push((
            format!("the {} standing posting", task.id()),
            layout::rates_post(row),
        ));
    }
    out
}

/// Every rectangle a click does something in on the base screen, with the
/// name a message uses.
///
/// **Everything that can be up at once**, including the faces list and the
/// character panel: a drawer shuts both (`Flow::close_everything`), so these
/// are exactly the controls that share a screen, and the overlap floor is
/// about siblings.
pub fn targets() -> Vec<(String, Rect)> {
    let mut out: Vec<(String, Rect)> = base_targets();
    for index in 0..layout::FACE_ROWS {
        out.push((format!("face row {index}"), layout::faces_row(index)));
    }
    out
}

/// The controls that are on the base screen whichever of the two left-hand
/// surfaces is up: the bar, the meters, the character panel and the strip.
fn base_targets() -> Vec<(String, Rect)> {
    let mut out: Vec<(String, Rect)> = Vec::new();
    for (index, label) in screens::chip_labels().into_iter().enumerate() {
        out.push((format!("the {label} chip"), layout::speed_chip(index)));
    }
    out.push(("the feed drawer's handle".to_owned(), layout::feed_button()));
    out.push((
        "the ledger drawer's handle".to_owned(),
        layout::ledger_button(),
    ));
    out.push((
        "the roster drawer's handle".to_owned(),
        layout::roster_button(),
    ));
    out.push((
        "the tuning drawer's handle".to_owned(),
        layout::tune_button(),
    ));
    out.push((
        "the auto-pause drawer's handle".to_owned(),
        layout::modes_button(),
    ));
    for (index, spec) in crate::meters::METERS.iter().enumerate() {
        out.push((
            format!("the {} meter chip", spec.id),
            layout::meter_chip(index),
        ));
    }
    out.push((
        "the character panel's close".to_owned(),
        layout::person_close(),
    ));
    for slot in 0..layout::SHEET_CHIPS {
        out.push((
            format!("the sheet's trait chip {slot}"),
            layout::sheet_chip(slot),
        ));
    }
    out
}

/// Every rectangle the feed drawer answers a click in.
pub fn feed_targets() -> Vec<(String, Rect)> {
    let mut out = vec![(
        "the show-ignored toggle".to_owned(),
        layout::feed_ignored_toggle(),
    )];
    for index in 0..layout::FEED_ROWS {
        out.push((format!("feed row {index}"), layout::feed_row(index)));
    }
    out
}

/// Every rectangle the auto-pause config drawer answers a click in.
pub fn modes_targets() -> Vec<(String, Rect)> {
    let mut out = Vec::new();
    for (row, class) in crate::attention::EventClass::all().into_iter().enumerate() {
        for (slot, mode) in crate::attention::Mode::ALL.iter().copied().enumerate() {
            out.push((
                format!("{}'s {} radio", class.name(), mode.name()),
                layout::modes_radio(row, slot),
            ));
        }
    }
    out
}

/// Every rectangle the roster drawer answers a click in: a row's name, and
/// every trait chip on it.
pub fn roster_targets() -> Vec<(String, Rect)> {
    let mut out = Vec::new();
    for row in 0..layout::ROSTER_ROWS {
        out.push((format!("roster row {row}"), layout::roster_open(row)));
        for slot in 0..layout::SHEET_CHIPS {
            out.push((
                format!("roster row {row}'s trait chip {slot}"),
                layout::roster_chip(row, slot),
            ));
        }
    }
    out
}

/// The controls that share this screen — whichever surface is up.
///
/// One function, because "a row of text may not lie across a control it is not
/// the label of" is a question about *what is on screen together*, and a
/// drawer covers everything under it.
pub fn controls_for(flow: &Flow) -> Vec<(String, Rect)> {
    if flow.tuner.open {
        return tuner_targets();
    }
    if flow.feed_open {
        return feed_targets();
    }
    if flow.modes_open {
        return modes_targets();
    }
    if flow.roster_open {
        return roster_targets();
    }
    if flow.ledger_open {
        return ledger_targets();
    }
    if flow.board.is_some() {
        return board_targets();
    }
    targets()
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
    // **Both base screens**: the one the faces list is up on and the one the
    // job board is up on. They share most of their controls and differ on the
    // left, and each has to hold the floors on its own.
    for set in [targets(), board_targets()] {
        for (what, rect) in &set {
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
                inside(layout::design(), *rect),
                "a clickable target is partly off the UI rect",
                format!(
                    "{what} is {rect:?} and the UI rect is {:?}",
                    layout::design()
                ),
            );
        }
        for (index, (what, rect)) in set.iter().enumerate() {
            for (other_what, other) in set.iter().skip(index + 1) {
                checks.require(
                    !rect.overlaps(*other),
                    "two interactive rectangles overlap",
                    format!("{what} at {rect:?} overlaps {other_what} at {other:?}"),
                );
            }
        }
    }
    // Every job row is inside the panel that holds it, and the board has a row
    // for every job a site is authored with — a job with no row is a job that
    // cannot be ordered, now that the row *is* the order.
    for slot in 0..layout::BOARD_ROWS {
        checks.require(
            inside(layout::board_panel(), layout::board_row(slot)),
            "a job row runs off the board that holds it",
            format!(
                "row {slot} is {:?} and the board is {:?}",
                layout::board_row(slot),
                layout::board_panel()
            ),
        );
    }
    for site in Sim::opening(&Tuning::SHIPPED, crate::modules::ModuleSet::ALL).sites {
        checks.require(
            site.quests.len() <= layout::BOARD_ROWS,
            "a site has more jobs than its board has rows",
            format!(
                "{} is authored with {} jobs and the board draws {}; a job with no row is a \
                 job nobody can be sent to",
                LOCATIONS[site.location].name,
                site.quests.len(),
                layout::BOARD_ROWS
            ),
        );
    }
    // The chips and handles live in the top bar. A control outside its band
    // is a control over the map.
    for index in 0..layout::CHIPS {
        checks.require(
            inside(layout::topbar(), layout::speed_chip(index)),
            "a speed chip is outside the status bar",
            format!("chip {index} at {:?}", layout::speed_chip(index)),
        );
    }
    // **A trait's explanation fits the two bands that print it**, at the
    // longest the vocabulary can produce (UI.md §3, the dormancy clause).
    // Derived from the data rather than eyeballed once: the sentence grew
    // when the clause landed, and it will grow again when a wave retires one.
    let longest = crate::traits::TRAITS
        .iter()
        .map(|def| crate::traits::explain(def.id, crate::modules::ModuleSet::ALL))
        .max_by_key(String::len)
        .unwrap_or_default();
    let rows = |width: f32| {
        crate::ui::wrap(&longest, crate::ui::columns(width, theme::SMALL))
            .lines()
            .count()
    };
    let sheet_rows = rows(layout::sheet::PROSE_W);
    let sheet_end = layout::person_panel().min.y
        + layout::sheet::EXPLAIN.y
        + sheet_rows as f32 * (theme::SMALL + 2.0);
    checks.require(
        !greater(sheet_end, layout::person_panel().max.y),
        "a trait's explanation runs off the character panel that holds it",
        format!(
            "{longest:?} wraps to {sheet_rows} rows ending at {sheet_end:.0} and the panel \
             ends at {:.0}",
            layout::person_panel().max.y
        ),
    );
    checks.require(
        rows(layout::ROSTER_EXPLAIN_W) <= layout::ROSTER_EXPLAIN_ROWS,
        "a trait's explanation does not fit the roster's explanation band",
        format!(
            "{longest:?} wraps to {} rows and the band holds {}; the rows below it start at \
             {:.0}",
            rows(layout::ROSTER_EXPLAIN_W),
            layout::ROSTER_EXPLAIN_ROWS,
            layout::roster_open(0).min.y
        ),
    );

    // **The breakdown band holds the widest sum this game can produce**
    // (UI.md §3e): every term of an answered posting, its total, and the
    // candidate that beat it. Counted off the scorer rather than remembered.
    let widest = {
        let sim = Sim::opening(&Tuning::SHIPPED, crate::modules::ModuleSet::ALL);
        let mut most = 0usize;
        for who in 0..sim.people.len() {
            for site in 0..sim.sites.len() {
                for slot in sim.sites[site].open_slots() {
                    let job = crate::sim::JobId { site, slot };
                    let posting = crate::asks::preview_posting(who, site, slot, 20, 20);
                    most = most.max(
                        crate::answers::terms(&sim, &Tuning::SHIPPED, 0, who, &posting, job).len(),
                    );
                }
            }
        }
        most
    };
    checks.require(
        widest + 2 <= layout::BREAKDOWN_CELLS,
        "the breakdown band has no room for the widest sum the scorer can produce",
        format!(
            "an answered posting weighs up to {widest} terms and the band holds \
             {} cells, which has to carry the terms, the total and the candidate that beat it",
            layout::BREAKDOWN_CELLS
        ),
    );
    // **And every line it can print fits a cell.** Walked over the vocabulary
    // rather than over the lines that happen to be on screen: a want covered
    // by two of somebody's motivators names both rows, and the day a third
    // one is authored the sentence gets longer without anybody noticing. A
    // clipped term is a term whose attribution is the half that goes.
    {
        let tuning = Tuning::SHIPPED;
        let sim = Sim::opening(&tuning, crate::modules::ModuleSet::ALL);
        let style = theme::text(theme::SMALL, theme::INK);
        let mut lines: Vec<String> = Vec::new();
        for who in 0..sim.people.len() {
            for action in crate::autonomy::candidates(&sim, who) {
                lines.extend(
                    crate::autonomy::weigh(&sim, &tuning, 0, who, action)
                        .iter()
                        .map(crate::autonomy::Term::line),
                );
            }
            for site in 0..sim.sites.len() {
                for slot in sim.sites[site].open_slots() {
                    let job = crate::sim::JobId { site, slot };
                    let posting = crate::asks::preview_posting(who, site, slot, 20, 20);
                    lines.extend(
                        crate::answers::terms(&sim, &tuning, 0, who, &posting, job)
                            .iter()
                            .map(crate::autonomy::Term::line),
                    );
                }
            }
        }
        if let Some(longest) = lines.into_iter().max_by(|a, b| {
            style
                .width_of(a)
                .partial_cmp(&style.width_of(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            checks.require(
                !greater(style.width_of(&longest), layout::BREAKDOWN_CELL_W),
                "a term of a breakdown does not fit the cell that prints it",
                format!(
                    "{longest:?} is {:.0} reference pixels wide and a cell is {:.0}; what a \
                     clip takes off a term line is the attribution, which is the half the \
                     band exists for",
                    style.width_of(&longest),
                    layout::BREAKDOWN_CELL_W
                ),
            );
        }
    }
    for index in 0..layout::BREAKDOWN_CELLS {
        let cell = Rect::from_min_size(
            layout::breakdown_cell(index),
            Vec2::new(layout::BREAKDOWN_CELL_W, theme::SMALL),
        );
        checks.require(
            inside(layout::breakdown_panel(), cell),
            "a breakdown cell runs off the band that holds it",
            format!(
                "cell {index} is {cell:?} and the band is {:?}",
                layout::breakdown_panel()
            ),
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

/// The floors over every drawer's own controls: the tuning drawer's steppers,
/// the feed's rows and toggle, and the config's radios.
pub fn drawer_floors(checks: &mut Checks) {
    for (drawer, controls, handle) in [
        (
            layout::tuner_panel(),
            tuner_targets(),
            layout::tune_button(),
        ),
        (layout::feed_panel(), feed_targets(), layout::feed_button()),
        (
            layout::feed_panel(),
            modes_targets(),
            layout::modes_button(),
        ),
        (
            layout::roster_panel(),
            roster_targets(),
            layout::roster_button(),
        ),
        (
            layout::ledger_panel(),
            ledger_targets(),
            layout::ledger_button(),
        ),
    ] {
        drawer_floor(checks, drawer, &controls, handle);
    }
}

/// One drawer's controls against the floors.
fn drawer_floor(checks: &mut Checks, drawer: Rect, controls: &[(String, Rect)], handle: Rect) {
    for (what, rect) in controls {
        let (what, rect) = (what.clone(), *rect);
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
        !handle.overlaps(drawer),
        "a drawer covers its own handle",
        format!("the handle is {handle:?} and the drawer is {drawer:?}"),
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

/// **The floor governs the map's words, and the zoom no longer yields to
/// them** (UI.md §4; `FINDINGS.md` G-022, closed by the legibility session).
///
/// The chrome rides `UiMap` and is a constant size on screen at any zoom, so
/// the readability floors bind it whatever the camera does. **Map-space text
/// does not**: a name under a figure is drawn at `theme::SMALL` *world*
/// units, and what a reader gets is that height scaled by how much world the
/// camera is showing. At the default camera the two are the same number,
/// which is why every floor in this game is stated in reference pixels.
///
/// Wave 1.2's rider asked for the default camera to step out one level, and
/// the floor **refused** it: a name reads at 12.0 pixels at the default and
/// 10.7 one notch out, so stepping out drew an illegible name and the zoom
/// was the thing that had to yield. That was the floor used as a veto, and it
/// was the wrong way round — the name is what should yield, because a name
/// that is not drawn costs the reader nothing and a name drawn at 10.7 pixels
/// costs them the map.
///
/// So the rule is now: **below the floor, no map word is drawn at all**
/// (`screens::content`). This function states the two numbers the rule turns
/// on — where the words survive and where they stop — and asserts the
/// arithmetic that makes the second one a real boundary rather than a
/// coincidence: one notch out is under the floor, so a player who zooms out
/// loses the names and keeps the picture, and the selected character's name,
/// which is chrome, survives either way.
///
/// **What is no longer asserted here is the default camera.** Where it should
/// sit is a play judgement and the owner's, and the rule is what makes it a
/// judgement they can actually make.
pub fn map_legibility(checks: &mut Checks) -> String {
    let at = |height: f32| theme::SMALL * layout::DESIGN_H / height;
    let now = at(camera::DEFAULT_H);
    let stepped = at(camera::DEFAULT_H * camera::SCROLL_STEP);
    checks.require(
        !greater(theme::MIN_TEXT - 0.01, now),
        "the map draws no words at the camera the game opens at",
        format!(
            "a name under a figure reads at {now:.1} reference pixels at the default camera \
             height of {:.0} and the floor is {:.0}, so the opening screen would be a map \
             with nothing named on it",
            camera::DEFAULT_H,
            theme::MIN_TEXT
        ),
    );
    checks.require(
        greater(theme::MIN_TEXT, stepped),
        "the label rule has no boundary inside one notch of the wheel",
        format!(
            "stepped out one notch a name reads at {stepped:.1} reference pixels, which is \
             not under the {:.0}-pixel floor; the drop rule would then never fire within a \
             notch of the default and the zoomed-out screen state asserts nothing",
            theme::MIN_TEXT
        ),
    );
    format!(
        "map labels {now:.1}px at the default zoom and {stepped:.1}px one notch out, where the \
         floor drops them"
    )
}

/// The screen states the content floors judge, built from a conducted run.
///
/// Every surface this build has, in the state that puts the most on it: the
/// opening screen, the feed after a full run (with a pause reason showing and
/// the ignored classes revealed), the auto-pause config, a character's panel
/// beside a drilled meter chip, and the strip carrying a toast.
pub fn content_states(baseline: &Conducted) -> Vec<(&'static str, Flow, Sim, Clock, Camera)> {
    let opening = (
        "the opening screen",
        Flow::default(),
        Sim::opening(&Tuning::SHIPPED, crate::modules::ModuleSet::ALL),
        Clock::opening(),
    );
    // The end of the conducted run: notices written, everything home.
    let mut played = Flow::default();
    played.note("d1 11:42 - running at 4x".to_owned());
    played.note("Steve is out - only an idle party takes orders".to_owned());
    let mut ended_clock = Clock::opening();
    ended_clock.minutes = baseline.minutes;
    // A world that stopped itself, with the feed open on the reason: the
    // loudest the feed gets, and the state the pause screenshot is taken in.
    let mut paused_sim = baseline.sim.clone();
    paused_sim.attention.set(
        crate::attention::EventClass::QuestComplete,
        crate::attention::Mode::PauseAndFocus,
    );
    if let Some(event) = paused_sim
        .events
        .iter()
        .position(|event| event.class == crate::attention::EventClass::QuestComplete)
    {
        paused_sim.paused_by = Some(crate::attention::Pause {
            event,
            class: crate::attention::EventClass::QuestComplete,
            minute: paused_sim.events[event].minute,
        });
        paused_sim.pauses = 1;
    }
    let mut feed_open = played.clone();
    feed_open.feed_open = true;
    feed_open.show_ignored = true;
    let mut modes_open = played.clone();
    modes_open.modes_open = true;
    // The roster, with a chip's explanation up: the loudest that surface gets.
    let mut roster_open = played.clone();
    roster_open.roster_open = true;
    roster_open.explained = crate::traits::TRAITS
        .iter()
        .max_by_key(|def| {
            crate::traits::explain(def.id, crate::modules::ModuleSet::ALL)
                .chars()
                .count()
        })
        .map(|def| def.id);
    // A character selected and a chip drilled: the two panels that share the
    // base screen, both up at once.
    let mut looked_at = played.clone();
    looked_at.selected = Some(baseline.sim.people.len().saturating_sub(1));
    looked_at.drilled = Some(0);
    // With the longest explanation a trait has, open on the sheet: a panel
    // that fits its widest state fits every other one.
    looked_at.explained = crate::traits::TRAITS
        .iter()
        .max_by_key(|def| {
            crate::traits::explain(def.id, crate::modules::ModuleSet::ALL)
                .chars()
                .count()
        })
        .map(|def| def.id);
    // A toast up, a party picked — the strip's loudest state.
    let mut toasted = played.clone();
    toasted.selected = Some(0);
    toasted.toast = Some(crate::flow::Toast {
        text: crate::sim::Refusal::NotIdle.message("Steve", "the Black Vault", "the vault door"),
        until: u64::MAX,
    });
    // **The job board, on the site that has been worked hardest**, with
    // somebody selected so the fit column and the travel line are both up:
    // the loudest that surface gets, which is the state its floors are owed
    // against.
    let mut board = played.clone();
    board.selected = Some(0);
    board.board = Some(
        (0..baseline.sim.sites.len())
            .max_by_key(|site| {
                baseline.sim.sites[*site]
                    .states
                    .iter()
                    .filter(|state| **state != crate::sim::JobState::Open)
                    .count()
            })
            .unwrap_or(0),
    );
    // And the same board read by nobody: no fit column, no travel line.
    let mut unread = board.clone();
    unread.selected = None;
    // **The board at its loudest** (wave 1.2): a wage stepped off its
    // standing rate, the fit chip explaining itself, and a verdict with its
    // reason on every open row.
    let mut offering = board.clone();
    offering.fit_explained = true;
    offering.offer = Some(crate::asks::RATE_MAX);
    offering.post_open = true;
    // **The ledger**, on a world that has been asked things: every row of it
    // is a posting somebody heard, agreed to or refused.
    let mut ledger = played.clone();
    ledger.ledger_open = true;
    // **The two states the legibility session added** (UI.md §3e): a job row's
    // arithmetic open on the board, and a decision's arithmetic open under
    // the feed — the band's two placements, both judged.
    let mut explained = board.clone();
    explained.breakdown = Some(crate::flow::Breakdown::Job(0));
    let mut explained_entry = feed_open.clone();
    explained_entry.breakdown = baseline
        .sim
        .events
        .iter()
        .position(|event| event.judged.is_some())
        .map(crate::flow::Breakdown::Entry);
    let at = verify::run_camera(verify::HEADLESS_VIEWPORT);
    // **And the map one notch further out than the default**, which the label
    // rule now permits: below the floor the map's words drop out rather than
    // shrinking, and the one that survives is the selected character's, which
    // is chrome (UI.md §4).
    let out = Camera {
        height: camera::DEFAULT_H * camera::SCROLL_STEP,
        ..at
    };
    let picked = Flow {
        selected: Some(0),
        ..Flow::default()
    };
    vec![
        (opening.0, opening.1, opening.2, opening.3, at),
        (
            "the ended run with the feed open, mid-pause",
            feed_open,
            paused_sim.clone(),
            ended_clock,
            at,
        ),
        (
            "the auto-pause config",
            modes_open,
            paused_sim.clone(),
            ended_clock,
            at,
        ),
        (
            "the roster with an explanation open",
            roster_open,
            baseline.sim.clone(),
            ended_clock,
            at,
        ),
        (
            "a character looked at, beside a drilled chip",
            looked_at,
            baseline.sim.clone(),
            ended_clock,
            at,
        ),
        (
            "a toast up and somebody picked",
            toasted,
            baseline.sim.clone(),
            ended_clock,
            at,
        ),
        (
            "a site's job board, read by a selected character",
            board,
            baseline.sim.clone(),
            ended_clock,
            at,
        ),
        (
            "a job row's arithmetic, open under the board",
            explained,
            baseline.sim.clone(),
            ended_clock,
            at,
        ),
        (
            "a decision's arithmetic, open under the feed",
            explained_entry,
            baseline.sim.clone(),
            ended_clock,
            at,
        ),
        (
            "the job board with a wage stepped and the fit chip open",
            offering,
            baseline.sim.clone(),
            ended_clock,
            at,
        ),
        (
            "the postings ledger, with the standing rates beside it",
            ledger,
            baseline.sim.clone(),
            ended_clock,
            at,
        ),
        (
            "a site's job board with nobody selected",
            unread,
            baseline.sim.clone(),
            ended_clock,
            at,
        ),
        (
            "the settlement one notch of zoom out, with somebody picked",
            picked,
            Sim::opening(&Tuning::SHIPPED, crate::modules::ModuleSet::ALL),
            Clock::opening(),
            out,
        ),
    ]
}

/// The content floors: every row of every screen state at or above the text
/// floor, inside its rect, and never lying across a control it is not the
/// label of.
pub fn content_floors(checks: &mut Checks, baseline: &Conducted) {
    let tuning = Tuning::SHIPPED;
    let grid = crate::grid::grid();
    for (what, flow, sim, clock, camera) in content_states(baseline) {
        let panel = screens::content(
            &flow,
            &Lens::on(&sim),
            &grid,
            &clock,
            &tuning,
            screens::reading(&clock, &tuning, screens::TICK),
            &camera,
        );
        judge_panel(checks, &panel, what, &controls_for(&flow));
        judge_cast(checks, &panel, &Lens::on(&sim), &clock, what);
    }
}

/// One panel against the floors.
pub fn judge_panel(checks: &mut Checks, panel: &Panel, what: &str, controls: &[(String, Rect)]) {
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
        for (control, target) in controls {
            if !text.bounds().overlaps(*target) {
                continue;
            }
            checks.require(
                inside(*target, text.bounds()),
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
    let grid = crate::grid::grid();
    for (what, flow) in tuning_states {
        let panel = screens::content(
            flow,
            &Lens::on(&drawer.applied_sim),
            &grid,
            &Clock::opening(),
            &drawer.applied_active,
            screens::reading(&Clock::opening(), &drawer.applied_active, screens::TICK),
            &verify::run_camera(verify::HEADLESS_VIEWPORT),
        );
        judge_panel(checks, &panel, what, &controls_for(flow));
    }
    // The refused-link state, staged: the longest refusal in the hint row.
    let mut refused = drawer.pending_flow.clone();
    refused.tuner.fault = crate::links::refusals().into_iter().max_by_key(String::len);
    let panel = screens::content(
        &refused,
        &Lens::on(&drawer.applied_sim),
        &grid,
        &Clock::opening(),
        &drawer.pending_active,
        screens::reading(&Clock::opening(), &drawer.pending_active, screens::TICK),
        &verify::run_camera(verify::HEADLESS_VIEWPORT),
    );
    judge_panel(
        checks,
        &panel,
        "the drawer with a refused link",
        &controls_for(&refused),
    );
    if let Some(shot) = &drawer.shot {
        judge_frame_floor(checks, drawer.font, &shot.frame, "the tuning drawer");
    }
}

/// **One person, one figure** (UI.md §6): every figure-weight picture on a
/// photographed frame is one the screen said it would draw, and nobody is
/// drawn twice.
///
/// The floor the double-drawn cast slipped through (`FINDINGS.md` G-023).
/// `frames::judge_chrome` asks one direction — every row and icon the `Panel`
/// says is somewhere on the frame — and wave 1.1's party tokens were drawn
/// straight through `ctx.sprite`, outside the `Panel` UI.md §6 judges, so a
/// second figure for every person was invisible to every check this game had.
/// This asks the other direction at the one weight a person is drawn at: a
/// figure-sized quad the screen cannot account for is somebody drawn twice,
/// and two quads on one corner is somebody drawn on top of themselves.
pub fn judge_figures(checks: &mut Checks, run: &Conducted, shot: &Shot, what: &str) {
    let camera = shot.camera;
    let map = UiMap::for_camera(&camera);
    let view = camera.visible_bounds();
    let panel = screens::content(
        &shot.flow,
        &Lens::on(&shot.sim),
        &crate::grid::grid(),
        &shot.clock,
        &Tuning::SHIPPED,
        screens::reading(&shot.clock, &Tuning::SHIPPED, screens::TICK),
        &shot.camera,
    );
    judge_cast(checks, &panel, &Lens::on(&shot.sim), &shot.clock, what);
    // Every corner the screen says a figure-weight picture stands at: map
    // content where it is, chrome through the same mapping it is drawn with.
    let mut wanted: Vec<Vec2> = panel
        .world_icons
        .iter()
        .filter(|icon| icon.bounds().overlaps(view) && near(icon.bounds().size().x, layout::HOME))
        .map(|icon| icon.at)
        .collect();
    wanted.extend(
        panel
            .icons
            .iter()
            .filter(|icon| near(icon.bounds().size().x * map.scale, layout::HOME))
            .map(|icon| map.to_world(icon.at)),
    );
    let drawn: Vec<Rect> = shot
        .frame
        .quads()
        .iter()
        .filter(|quad| quad.texture != run.font)
        .map(|quad| quad.bounds())
        .filter(|bounds| near(bounds.size().x, layout::HOME) && near(bounds.size().y, layout::HOME))
        .collect();
    // Counted by corner rather than one apiece: a person working at a site
    // stands on that site's marker, so one corner legitimately carries two
    // pictures, and what the frame owes is the number the screen named.
    for at in &wanted {
        let named = wanted
            .iter()
            .filter(|other| near(other.x, at.x) && near(other.y, at.y))
            .count();
        let copies = drawn
            .iter()
            .filter(|bounds| near(bounds.min.x, at.x) && near(bounds.min.y, at.y))
            .count();
        checks.require(
            copies == named,
            "the map draws a different number of figures than the screen says stand there",
            format!(
                "{what}: {copies} figure-sized quads land on ({:.1}, {:.1}) and the screen says \
                 {named}",
                at.x, at.y
            ),
        );
    }
    let stray: Vec<Rect> = drawn
        .iter()
        .filter(|bounds| {
            !wanted
                .iter()
                .any(|at| near(bounds.min.x, at.x) && near(bounds.min.y, at.y))
        })
        .copied()
        .collect();
    checks.require(
        stray.is_empty(),
        "the map draws a figure the screen does not say it draws",
        format!(
            "{what}: {} figure-sized quad(s) landed at corners the screen never named - {:?}; \
             the screen says {} of them and the frame carries {}",
            stray.len(),
            stray
                .iter()
                .map(|bounds| (bounds.min.x, bounds.min.y))
                .collect::<Vec<_>>(),
            wanted.len(),
            drawn.len()
        ),
    );
}

/// **One person, one figure, one place** (UI.md §3b): the cast on one screen,
/// counted and placed.
///
/// The panel half of the figure floor, which needs no photograph and so binds
/// every screen state `content_floors` judges rather than only the two the run
/// stops to photograph. Three questions, and the first two are the ones the
/// double-drawn cast would have failed at any offset:
///
/// - every person has **exactly one** figure on the map — found by their own
///   portrait, which no two of the cast share
///   (`library::portraits_are_tellable_apart`);
/// - that figure is at `screens::where_drawn`, so a person is drawn where the
///   one answer says they are and nowhere else;
/// - a figure standing alone is drawn **exactly** where its person stands, so
///   a nudge has to have somebody else's figure as its reason;
/// - **no two figures stand closer than [`FIGURES_APART`]**, so ten figures
///   stacked on one tile do not pass a count. This is the guarantee
///   `screens::where_drawn`'s placement exists to make, stated here as its own
///   number rather than read off the drawer's, so a nudge that stopped
///   separating people fails this floor instead of moving it.
///
/// Note what it does **not** say: that no two figures overlap. A figure is 32
/// world units and a tile is 16, so two people at neighbouring doorsteps
/// legitimately overlap and always have. Whether the settlement is too crowded
/// to read is a judgement about the map, and it is the owner's
/// (`FINDINGS.md` G-022).
///
/// How far apart two cast figures have to be drawn to read as two people: a
/// shipped literal — half the 32-unit figure — deliberately not derived from
/// the nudge it judges (`make-game` §A.6: an instrument that computes its
/// expectation from the number under test cannot see that number move).
pub const FIGURES_APART: f32 = 16.0;

pub fn judge_cast(checks: &mut Checks, panel: &Panel, lens: &Lens<'_>, clock: &Clock, what: &str) {
    let now = screens::reading(clock, &Tuning::SHIPPED, screens::TICK);
    let mut figures: Vec<(usize, Rect)> = Vec::new();
    for (index, person) in lens.people().iter().enumerate() {
        let mine: Vec<Rect> = panel
            .world_icons
            .iter()
            .filter(|icon| icon.art == person.icon)
            .map(crate::ui::IconRun::bounds)
            .collect();
        checks.require(
            mine.len() == 1,
            "a character is not drawn on the map exactly once",
            format!(
                "{what}: {} figures carry {}'s portrait and one person is one figure",
                mine.len(),
                lens.name(index)
            ),
        );
        let Some(place) = screens::stands_at(lens, index, now) else {
            checks.require(
                false,
                "a character is standing nowhere at all",
                format!("{what}: {} has no place on the map", lens.name(index)),
            );
            continue;
        };
        let Some(wanted) = screens::where_drawn(lens, index, now) else {
            checks.require(
                false,
                "a character is drawn nowhere at all",
                format!("{what}: {} has no place on the map", lens.name(index)),
            );
            continue;
        };
        if let Some(drawn) = mine.first() {
            checks.require(
                near(drawn.min.x, wanted.min.x) && near(drawn.min.y, wanted.min.y),
                "a character's figure is not where the one answer puts them",
                format!(
                    "{what}: {} is drawn at ({:.1}, {:.1}) and `where_drawn` says ({:.1}, {:.1})",
                    lens.name(index),
                    drawn.min.x,
                    drawn.min.y,
                    wanted.min.x,
                    wanted.min.y
                ),
            );
            // **Drawn exactly on their place unless somebody is there.** The
            // sharp half, and the one wave 1.1's nudge could not have passed
            // at any roster index but zero: it moved every person, alone or
            // not, and by the ninth it moved them fifty-one units — three
            // tiles from the tile their party was on. A figure that stands
            // somewhere its person does not has to have another figure as its
            // reason.
            let alone = (0..lens.people().len())
                .filter(|other| *other != index)
                .filter_map(|other| screens::stands_at(lens, other, now))
                .all(|theirs| !greater(FIGURES_APART, theirs.distance(place)));
            checks.require(
                !alone || (near(drawn.center().x, place.x) && near(drawn.center().y, place.y)),
                "a character standing on their own is not drawn where they stand",
                format!(
                    "{what}: {} stands at ({:.1}, {:.1}) with nobody within {FIGURES_APART:.0} \
                     units and is drawn at ({:.1}, {:.1})",
                    lens.name(index),
                    place.x,
                    place.y,
                    drawn.center().x,
                    drawn.center().y
                ),
            );
            figures.push((index, *drawn));
        }
    }
    for (slot, (index, mine)) in figures.iter().enumerate() {
        for (other, theirs) in figures.iter().skip(slot + 1) {
            let apart = mine.center().distance(theirs.center());
            checks.require(
                !greater(FIGURES_APART, apart),
                "two characters are drawn too close together to read as two people",
                format!(
                    "{what}: {} at ({:.1}, {:.1}) and {} at ({:.1}, {:.1}) are {apart:.1} units \
                     apart and the floor is {FIGURES_APART:.0}",
                    lens.name(*index),
                    mine.center().x,
                    mine.center().y,
                    lens.name(*other),
                    theirs.center().x,
                    theirs.center().y
                ),
            );
        }
    }
}
