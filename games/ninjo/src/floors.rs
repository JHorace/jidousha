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
use crate::flow::{Drawer, Flow};
use crate::grid::LOCATIONS;
use crate::lens::Lens;
use crate::sim::Sim;
use crate::sweep::{Conducted, Shot};
use crate::ui::Panel;
use crate::{camera, layout, panels, screens, theme, tuning, verify};

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
        // **The row's own two controls**, both targets of their own beside it
        // rather than inside it: the candidate list for that job, and the
        // arithmetic behind its verdict.
        out.push((format!("job row {slot}'s who?"), layout::board_who(slot)));
        out.push((format!("job row {slot}'s why"), layout::board_why(slot)));
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

/// The base screen's controls with a job's **candidate picker** up.
///
/// The picker draws instead of the board and over the whole left column
/// (UI.md §3c), so the board's own rows and footer controls are not on this
/// screen at all — a third set of siblings, and the overlap floor is about
/// siblings. The character panel and the bar are in it, because the picker is
/// opened from a board that may already have somebody selected.
pub fn picker_targets() -> Vec<(String, Rect)> {
    let mut out: Vec<(String, Rect)> = base_targets();
    out.push((
        "the candidate picker's close".to_owned(),
        layout::board_close(),
    ));
    out.push(("the board's fit chip".to_owned(), layout::board_fit_chip()));
    for row in 0..layout::PICKER_ROWS {
        out.push((format!("candidate row {row}"), layout::picker_row(row)));
    }
    out
}

/// The base screen's controls with the **work list** up instead of a board.
///
/// The list stands in the board's own rectangle and draws instead of it
/// (UI.md §3f), so the board's rows and footer controls are not on this
/// screen at all — a fourth set of siblings, and the overlap floor is about
/// siblings. The character panel is in it, because the list is the
/// selection's own surface and the selection is what the panel is.
pub fn worklist_targets() -> Vec<(String, Rect)> {
    let mut out: Vec<(String, Rect)> = base_targets();
    out.push(("the work list's close".to_owned(), layout::board_close()));
    out.push(("the board's fit chip".to_owned(), layout::board_fit_chip()));
    for row in 0..layout::WORK_ROWS {
        out.push((format!("work row {row}"), layout::worklist_row(row)));
    }
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
    out.extend(handle_targets());
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
    out.push(("the sheet's work chip".to_owned(), layout::sheet_work()));
    for slot in 0..layout::SHEET_CHIPS {
        out.push((
            format!("the sheet's trait chip {slot}"),
            layout::sheet_chip(slot),
        ));
    }
    out
}

/// **The five handles**, off the one list of drawers there is.
///
/// They are on every screen this game has, drawer or no drawer: the top bar
/// stands above the drawers and a handle answers a click from inside one
/// (UI.md §3), so they share a screen with whatever else is up.
fn handle_targets() -> Vec<(String, Rect)> {
    Drawer::ALL
        .into_iter()
        .map(|drawer| (format!("the {} handle", drawer.label()), drawer.handle()))
        .collect()
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
    // **A match over the one drawer**, the same value `screens::content`
    // draws from and `flow::read_input` routes clicks with: three readers,
    // one field, so the controls this floor judges are the controls that are
    // actually on screen.
    if let Some(drawer) = flow.drawer {
        let mut out = match drawer {
            Drawer::Tune => tuner_targets(),
            Drawer::Roster => roster_targets(),
            Drawer::Ledger => ledger_targets(),
            Drawer::Feed => feed_targets(),
            Drawer::Modes => modes_targets(),
        };
        // The handles stand above every drawer and stay live inside one, so
        // they share the screen with whatever the drawer carries.
        out.extend(handle_targets());
        return out;
    }
    if flow.listing.is_some() {
        return worklist_targets();
    }
    if flow.board.is_some() {
        if flow.picking.is_some() {
            return picker_targets();
        }
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
    // **All three base screens**: the one the faces list is up on, the one
    // the job board is up on, and the one a job's candidate picker is up on.
    // They share most of their controls and differ on the left, and each has
    // to hold the floors on its own.
    for set in [targets(), board_targets(), picker_targets()] {
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
    // **Every cell of a work row holds the content the scenario authors**
    // (UI.md §3f). A list across every board is the one surface where a clip
    // takes the word that says *which* job a tap is about, so the cells are
    // measured against the authored names rather than eyeballed against the
    // ones that happened to be longest the day they were typed.
    {
        let opening = crate::sim::Sim::opening(&Tuning::SHIPPED, crate::modules::ModuleSet::ALL);
        let width = |text: &str| theme::text(theme::SMALL, theme::INK).width_of(text);
        let mut cells: Vec<(String, String, f32)> = Vec::new();
        for site in &opening.sites {
            for quest in &site.quests {
                cells.push((
                    "a job's name".to_owned(),
                    quest.name.to_owned(),
                    layout::work::NAME_W,
                ));
                cells.push((
                    "a job's pot".to_owned(),
                    format!("{}g", quest.pot),
                    layout::work::POT_W,
                ));
                cells.push((
                    "a job's duration".to_owned(),
                    format!("{} min", quest.duration),
                    layout::work::DURATION_W,
                ));
            }
        }
        for location in LOCATIONS {
            cells.push((
                "a site's name".to_owned(),
                location.name.to_owned(),
                layout::work::WHERE_W,
            ));
        }
        for (what, text, cell) in cells {
            checks.require(
                !greater(width(&text), cell),
                "a work row's cell is narrower than the content the scenario authors",
                format!(
                    "{what}, {text:?}, is {:.0} reference pixels wide and its cell is {cell:.0}",
                    width(&text)
                ),
            );
        }
        // **And both header lines fit the band that prints them**, at the
        // longest name in the cast and the fullest the settlement can be: the
        // count and the wage are what the answers below were read at and they
        // are on no other surface while this one is up.
        let style = theme::text(theme::SMALL, theme::INK);
        let jobs: usize = opening.sites.iter().map(|site| site.quests.len()).sum();
        let header = [
            format!(
                "the work open to {}",
                crate::people::roster()
                    .iter()
                    .map(|person| person.name)
                    .max_by_key(|name| name.len())
                    .unwrap_or_default()
            ),
            format!(
                "{} of {jobs} open - at the standing rate",
                layout::WORK_ROWS
            ),
        ];
        for line in header {
            checks.require(
                !greater(style.width_of(&line), layout::WORKLIST_HEAD_W),
                "a work list header line does not fit the band that prints it",
                format!(
                    "{line:?} is {:.0} reference pixels wide and the header band is {:.0}",
                    style.width_of(&line),
                    layout::WORKLIST_HEAD_W
                ),
            );
        }
        // And the whole row lands inside the panel that holds it.
        for row in 0..layout::WORK_ROWS {
            checks.require(
                inside(layout::worklist_panel(), layout::worklist_row(row)),
                "a work row is outside the list that holds it",
                format!(
                    "row {row} is {:?} and the list is {:?}",
                    layout::worklist_row(row),
                    layout::worklist_panel()
                ),
            );
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
    // **The candidate picker holds the whole cast, in the panel that holds
    // it** (UI.md §3c). A candidate with no row is a person the board cannot
    // name, which is the defect this surface exists to close — so the cast is
    // counted against the rows rather than remembered as ten.
    for row in 0..layout::PICKER_ROWS {
        checks.require(
            inside(layout::picker_panel(), layout::picker_row(row)),
            "a candidate row runs off the picker that holds it",
            format!(
                "row {row} is {:?} and the picker is {:?}",
                layout::picker_row(row),
                layout::picker_panel()
            ),
        );
    }
    {
        let cast = Sim::opening(&Tuning::SHIPPED, crate::modules::ModuleSet::ALL)
            .people
            .len();
        checks.require(
            cast <= layout::PICKER_ROWS,
            "the cast is larger than the candidate picker has rows",
            format!(
                "the registry holds {cast} people and the picker draws {}; everyone appears                  on that list, so a person with no row is somebody the board cannot name",
                layout::PICKER_ROWS
            ),
        );
    }
    // **Both of the picker's header lines fit the band they run in**, at the
    // longest job name the scenario authors and the widest wage the steppers
    // reach: the job says what the list is for and the wage says what its
    // answers were read at, and the wage appears nowhere else while the
    // picker is covering the board's footer.
    {
        let style = theme::text(theme::SMALL, theme::INK);
        let sim = Sim::opening(&Tuning::SHIPPED, crate::modules::ModuleSet::ALL);
        let longest = sim
            .sites
            .iter()
            .flat_map(|site| site.quests.iter())
            .map(|quest| format!("who for {}?", quest.name))
            .chain(std::iter::once(format!(
                "{}g offered - best fit first",
                crate::asks::RATE_MAX
            )))
            .max_by(|a, b| {
                style
                    .width_of(a)
                    .partial_cmp(&style.width_of(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or_default();
        checks.require(
            !greater(style.width_of(&longest), layout::PICKER_HEAD_W),
            "a candidate picker's header line does not fit the band that prints it",
            format!(
                "{longest:?} is {:.0} reference pixels wide and the header band is {:.0}; the                  wage is what the answers below were read at and it is on no other surface                  while this one is up",
                style.width_of(&longest),
                layout::PICKER_HEAD_W
            ),
        );
    }
    // **And the longest thing the picker's footer can say fits the band**: it
    // prints the fit chip's own sentence, which is the board's, and that
    // sentence grew once already when the dormancy clause landed.
    {
        let longest = [
            crate::asks::fit_means(),
            format!(
                "sorted by fit - tap somebody to name them for {}, then tap its row to post it",
                Sim::opening(&Tuning::SHIPPED, crate::modules::ModuleSet::ALL)
                    .sites
                    .iter()
                    .flat_map(|site| site.quests.iter())
                    .map(|quest| quest.name)
                    .max_by_key(|name| name.len())
                    .unwrap_or_default()
            ),
        ]
        .into_iter()
        .max_by_key(String::len)
        .unwrap_or_default();
        let rows = crate::ui::wrap(
            &longest,
            crate::ui::columns(layout::PICKER_HINT_W, theme::SMALL),
        )
        .lines()
        .count();
        let end = layout::picker_hint().y + rows as f32 * (theme::SMALL + 2.0);
        checks.require(
            !greater(end, layout::picker_panel().max.y),
            "the candidate picker's footer runs off the panel that holds it",
            format!(
                "{longest:?} wraps to {rows} rows ending at {end:.0} and the picker ends at                  {:.0}",
                layout::picker_panel().max.y
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
    // **The character panel's flowed rows, at the longest each of them can
    // be** (UI.md §3a). The source, the activity line, the home row and a
    // tapped chip's explanation each start where the one above ended, so the
    // question is not whether one offset is right but whether the whole flow
    // fits — counted off the data the rows are built from, over every
    // character the registry holds.
    let sheet_rows = rows(layout::sheet::PROSE_W);
    let flow_rows = layout::sheet::LEAD_ROWS + sheet_rows;
    let sheet_end = layout::person_panel().min.y
        + layout::sheet::SOURCE.y
        + flow_rows as f32 * (theme::SMALL + 2.0)
        + 4.0 * layout::sheet::FLOW_GAP;
    checks.require(
        !greater(sheet_end, layout::person_panel().max.y),
        "the character panel's flowed rows run off the panel that holds them",
        format!(
            "the widest flow is {flow_rows} rows ({} above the explanation, and the longest \
             explanation, {longest:?}, wraps to {sheet_rows}) ending at {sheet_end:.0} and the \
             panel ends at {:.0}",
            layout::sheet::LEAD_ROWS,
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

/// **The tuning drawer's right column fits the drawer, with room to grow.**
///
/// The stamp is `Tuning::readout` and it grows a row every other constant
/// `constants.rs` gains; the prose band under it is measured down from the
/// stamp (`tuning::prose_top`), so growth moves the band rather than colliding
/// with it — until the band runs off the drawer's foot, which is what this
/// asserts, at the longest prose the drawer can print and
/// `tuning::STAMP_HEADROOM` rows before it is a problem.
///
/// **The floor fails while there is still room**, so the wave that adds the
/// constant is told to re-lay this column instead of finding out from a
/// screenshot the way the owner did (`FINDINGS.md` G-028).
pub fn tuner_right_column(checks: &mut Checks) {
    let prose = crate::ui::columns(layout::tuner_prose_width(), theme::SMALL);
    let rows = |text: &str| crate::ui::wrap(text, prose).lines().count();
    // The two tallest states the band takes. The hint and the note share it
    // and only one of them is ever up (`tuning::drawer`), so the worst case is
    // whichever is taller: the longest single hint, or the resting line with
    // the APPLY note under it.
    let longest_hint = crate::constants::Field::ALL
        .iter()
        .map(|field| format!("{} - {}", field.name(), field.meaning()))
        .chain(crate::links::refusals())
        .map(|line| rows(&line))
        .max()
        .unwrap_or(1);
    let resting = rows(tuning::RESTING_HINT) + rows(tuning::APPLY_NOTE);
    let prose_rows = longest_hint.max(resting);
    let stamp_rows = tuning::stamp_text(&Tuning::SHIPPED, 0).lines().count();
    for extra in 0..=tuning::STAMP_HEADROOM {
        let end =
            tuning::prose_top(stamp_rows + extra) + prose_rows as f32 * (theme::SMALL + 2.0) + 4.0;
        checks.require(
            !greater(end, layout::tuner_panel().max.y),
            "the tuning drawer's right column runs off the drawer",
            format!(
                "at {stamp_rows} rows of stamp plus {extra} of headroom the prose band starts \
                 at {:.0}, runs {prose_rows} rows and ends at {end:.0}; the drawer ends at \
                 {:.0}. Re-lay the right column before adding the constant",
                tuning::prose_top(stamp_rows + extra),
                layout::tuner_panel().max.y
            ),
        );
    }
}

/// **The two new floors, demonstrated failing on the screens they were
/// written for.**
///
/// A floor nobody has seen fail is a floor nobody knows is connected. Both of
/// these were written because a screen the owner photographed on 2026-09-11
/// was wrong and every check passed, so both are staged here on that screen —
/// the tuning drawer's right column as it was laid out before this session,
/// and two drawers' content in one frame — and the assertion is that judging
/// those screens *reports*, by the name of the claim.
///
/// The judge runs into a throwaway `Checks`, so a floor doing its job here
/// does not fail the run.
pub fn floors_bite(checks: &mut Checks) -> String {
    let sim = Sim::opening(&Tuning::SHIPPED, crate::modules::ModuleSet::ALL);
    let lens = Lens::on(&sim);
    let flow = Flow {
        drawer: Some(Drawer::Tune),
        ..Flow::default()
    };

    // --- the right column, at the offset it had before this session --------
    //
    // The stamp flowed down from the label and the prose band began at a
    // typed 350; at thirty-six constants the stamp's last rows reached 362,
    // and "seed 0" was drawn through "point at a constant for what it does".
    const PRE_FIX_HINT_Y: f32 = 350.0;
    let prose = crate::ui::columns(layout::tuner_prose_width(), theme::SMALL);
    let mut before = Panel::default();
    before.block(
        layout::tuner_stamp() + Vec2::new(0.0, 14.0),
        &tuning::stamp_text(&Tuning::SHIPPED, 0),
        theme::SMALL,
        theme::INK,
    );
    before.block(
        Vec2::new(layout::tuner_stamp().x, PRE_FIX_HINT_Y),
        &crate::ui::wrap(tuning::RESTING_HINT, prose),
        theme::SMALL,
        theme::FAINT,
    );
    let mut staged = Checks::default();
    judge_panel(
        &mut staged,
        &before,
        "the tuning drawer's right column at its pre-fix offset",
        &[],
    );
    let overlap_bites = staged.reported("two rows of chrome text overlap");
    checks.require(
        overlap_bites,
        "the chrome-overlap floor does not fail on the screen it was written for",
        format!(
            "the stamp laid out from {:?} and the prose band at y {PRE_FIX_HINT_Y} is the \
             owner's 2026-09-11 screenshot, and judging it reported {} problem(s)",
            layout::tuner_stamp(),
            staged.failures()
        ),
    );

    // --- two drawers' content in one frame ---------------------------------
    //
    // The state itself is now unrepresentable — `Flow::drawer` is one value —
    // so it is staged by absorbing two drawers' panels, which is what the
    // five open-flags made `screens::content` do.
    let mut both = panels::roster_drawer(&flow, &lens);
    both.absorb(tuning::drawer(&flow, &Tuning::SHIPPED));
    let mut staged = Checks::default();
    judge_panel(&mut staged, &both, "TUNE drawn over an open ROSTER", &[]);
    let count_bites = staged.reported("two drawers' content is in one frame");
    checks.require(
        count_bites,
        "the one-drawer floor does not fail on the screen it was written for",
        format!(
            "a frame carrying both the roster's and the tuning drawer's content reported {} \
             problem(s), and none of them was the drawer count",
            staged.failures()
        ),
    );
    format!(
        "chrome overlap bites on the pre-fix right column ({} problems), the drawer count \
         bites on TUNE over ROSTER ({} problems)",
        overlap_bites as usize, count_bites as usize
    )
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
    feed_open.drawer = Some(Drawer::Feed);
    feed_open.show_ignored = true;
    let mut modes_open = played.clone();
    modes_open.drawer = Some(Drawer::Modes);
    // The roster, with a chip's explanation up: the loudest that surface gets.
    let mut roster_open = played.clone();
    roster_open.drawer = Some(Drawer::Roster);
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
    // **The candidate picker, read by nobody** — which is the state it exists
    // for: an open board over a character's own sprite, and a job that can
    // still be aimed at that character. Its front open row, so the list is
    // one the sim can answer.
    let mut picking = board.clone();
    picking.selected = None;
    let picked_site = picking.board.unwrap_or(0);
    picking.picking = baseline
        .sim
        .sites
        .get(picked_site)
        .and_then(|site| site.open_slots().next())
        .map(|slot| crate::sim::JobId {
            site: picked_site,
            slot,
        });
    // And the same list with somebody already named on it and the fit chip
    // explaining itself — the loudest that surface gets.
    let mut picking_named = picking.clone();
    picking_named.selected = Some(0);
    picking_named.fit_explained = true;
    // **The work list, on the character the settlement suits worst and the
    // one it suits best** — the two ends of the fit spread, so the surface is
    // judged with a refusal on it and with an agreement on it. Its loudest
    // state has the fit chip explaining itself as well.
    let mut listed = played.clone();
    listed.selected = Some(0);
    listed.listing = Some(0);
    let mut listed_last = played.clone();
    let last = baseline.sim.people.len().saturating_sub(1);
    listed_last.selected = Some(last);
    listed_last.listing = Some(last);
    listed_last.fit_explained = true;
    // **The ledger**, on a world that has been asked things: every row of it
    // is a posting somebody heard, agreed to or refused.
    let mut ledger = played.clone();
    ledger.drawer = Some(Drawer::Ledger);
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
            "the work list open on the first of the cast",
            listed,
            baseline.sim.clone(),
            ended_clock,
            at,
        ),
        (
            "the work list at its loudest, with the fit chip explaining itself",
            listed_last,
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
            "a job's candidate picker, read with nobody selected",
            picking,
            baseline.sim.clone(),
            ended_clock,
            at,
        ),
        (
            "a job's candidate picker, with somebody named and the fit chip open",
            picking_named,
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
        // **The character panel's flowed rows land inside it** (UI.md §3a).
        // `layout_floors` budgets the flow at `sheet::LEAD_ROWS`; this is
        // what turns that budget into an assertion, over the sheets a played
        // world actually produces.
        if let Some(who) = flow.selected {
            let sheet = panels::person_panel(&flow, &Lens::on(&sim), who);
            for run in &sheet.runs {
                checks.require(
                    inside(layout::person_panel(), run.bounds()),
                    "a row of the character panel runs off the panel that holds it",
                    format!(
                        "{what}: {:?} occupies {:?} and the panel is {:?}",
                        run.text,
                        run.bounds(),
                        layout::person_panel()
                    ),
                );
            }
        }
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
    // **No two rows of chrome on one band collide** — the floor the tuning
    // drawer's right column needed and did not have.
    //
    // `judge_panel` asked chrome text against *controls* and map labels
    // against *each other*, and never chrome against chrome; so the stamp
    // growing a row every other constant walked into the prose band beside it
    // and no check said a word (`FINDINGS.md` G-028). **On one band**, because
    // the layers are what make an overlay legitimate: the breakdown band is
    // drawn over the feed drawer's footer with its own ground behind it and
    // that is deliberate (UI.md §3e), while two rows on the same band are two
    // rows drawn through each other.
    let mut chrome: Vec<&crate::ui::TextRun> = panel.runs.iter().collect();
    chrome.sort_by_key(|run| run.layer);
    for (index, text) in chrome.iter().enumerate() {
        for other in chrome.iter().skip(index + 1) {
            if other.layer != text.layer {
                continue;
            }
            checks.require(
                !text.bounds().overlaps(other.bounds()),
                "two rows of chrome text overlap",
                format!(
                    "{what}: {:?} at {:?} and {:?} at {:?}, both on band {}",
                    text.text,
                    text.bounds(),
                    other.text,
                    other.bounds(),
                    text.layer
                ),
            );
        }
    }
    // **At most one drawer's content in the frame.**
    //
    // With `Flow::drawer` a single `Option<Drawer>` this is unrepresentable,
    // and the floor says it anyway: the next surface to grow an open-flag of
    // its own should fail here rather than be found in a screenshot. It
    // counts the drawers by the head row each of them draws — `Drawer::title`,
    // the one string the drawer prints and this reads, so the two cannot
    // drift apart into a floor that sees nothing.
    let drawers: Vec<&'static str> = Drawer::ALL
        .into_iter()
        .filter(|drawer| panel.runs.iter().any(|run| run.text == drawer.title()))
        .map(Drawer::label)
        .collect();
    checks.require(
        drawers.len() <= 1,
        "two drawers' content is in one frame",
        format!("{what}: {drawers:?} are all drawing, and a drawer covers the screen"),
    );
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
    // **The frame half of this floor is a question about the map, and a
    // drawer covers the map.**
    //
    // It counts quads at the figure weight, which is thirty-two — and
    // thirty-two is also the target floor, so the tuning drawer's seventy-two
    // stepper buttons are seventy-two figure-sized squares that no cast
    // member stands at. The screen half above still runs on every frame; this
    // half is asked of the frames where the map is the thing being looked at.
    if shot.flow.drawer.is_some() {
        return;
    }
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
