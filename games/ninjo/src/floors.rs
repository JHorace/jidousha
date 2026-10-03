//! The readability floors, as assertions rather than as advice — giri's
//! UI.md §7 machinery, carried into the fork whole (the fork's UI.md says
//! which surfaces they now bind: clock, chips, labels, tokens, log).
//!
//! Stated **at reference scale**: UI space is 960x540 and one UI unit is one
//! reference pixel, so a floor and the number UI.md writes are the same
//! number. The map camera never changes that — the chrome rides `UiMap` and
//! is a constant size on screen at any zoom.

use jidousha::prelude::*;
use jidousha::ui::{Floors, inside};

use crate::camera::UiMap;
use crate::checks::{Checks, greater, near};
use crate::clock::Clock;
use crate::constants::Tuning;
use crate::flow::{Drawer, Flow};
use crate::grid::LOCATIONS;
use crate::lens::Lens;
use crate::sim::Sim;
use crate::sweep::{Conducted, Shot};
use crate::ui::{self, Panel};
use crate::{camera, layout, panels, screens, theme, tuning, verify};

/// The numbers this game's floors are stated against (UI.md §7): the text
/// floor, the UI rect, and the map.
pub fn floors() -> Floors {
    Floors {
        min_text: theme::MIN_TEXT,
        chrome: layout::design(),
        world: crate::grid::grid().world_rect(),
    }
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

/// The base screen's controls with the **settlement panel** up instead of a
/// board (UI.md §3g).
///
/// It stands in the board's own rectangle and draws instead of it, for the
/// reason the picker and the work list do, so the board's rows and footer are
/// not on this screen at all — a fifth set of siblings, and the overlap floor
/// is about siblings. Its own controls are the close and, per industry row,
/// the BUILD verb and the two halves of a wage stepper.
pub fn works_targets() -> Vec<(String, Rect)> {
    let mut out: Vec<(String, Rect)> = base_targets();
    out.push((
        "the settlement panel's close".to_owned(),
        layout::works_close(),
    ));
    for index in 0..layout::WORKS_ROWS {
        out.push((
            format!("industry row {index}'s BUILD"),
            layout::works_build(index),
        ));
        out.push((
            format!("industry row {index}'s wage down"),
            layout::works_wage_down(index),
        ));
        out.push((
            format!("industry row {index}'s wage up"),
            layout::works_wage_up(index),
        ));
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

/// **Every rectangle the petition ledger answers a click in** (wave 1.5): a
/// row per petition it can show, and the card's chip, ARRANGE and GIVE.
pub fn pleas_targets() -> Vec<(String, Rect)> {
    let mut out = Vec::new();
    for row in 0..layout::PLEA_ROWS {
        out.push((format!("petition row {row}"), layout::plea_row(row)));
    }
    let card = layout::plea_card();
    out.push((
        "the card's consequence chip".to_owned(),
        layout::card_chip(card),
    ));
    out.push(("the card's ARRANGE".to_owned(), layout::card_act(card)));
    out.push(("the card's GIVE".to_owned(), layout::card_give(card)));
    out
}

/// **Every rectangle a click does something in while the world is stopped for
/// a voicing**: the card's chip, LATER, and the speed chips that resume.
pub fn voicing_targets() -> Vec<(String, Rect)> {
    let mut out = Vec::new();
    for (index, label) in screens::chip_labels().into_iter().enumerate() {
        out.push((format!("the {label} chip"), layout::speed_chip(index)));
    }
    let card = layout::voicing_card();
    out.push((
        "the overlay's consequence chip".to_owned(),
        layout::card_chip(card),
    ));
    out.push(("the overlay's LATER".to_owned(), layout::card_act(card)));
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
pub fn controls_for(flow: &Flow, lens: &Lens<'_>) -> Vec<(String, Rect)> {
    // **The voicing overlay is the screen while it is up** (UI.md §3h), and
    // it is the sim's own pause that says so — the controls on screen are
    // the card's and the speed chips, whatever the flow holds under it.
    if crate::card::voicing(lens).is_some() {
        return voicing_targets();
    }
    // **A match over the one drawer**, the same value `screens::content`
    // draws from and `flow::read_input` routes clicks with: three readers,
    // one field, so the controls this floor judges are the controls that are
    // actually on screen.
    if let Some(drawer) = flow.drawer {
        let mut out = match drawer {
            Drawer::Tune => tuner_targets(),
            Drawer::Roster => roster_targets(),
            Drawer::Ledger => ledger_targets(),
            Drawer::Pleas => pleas_targets(),
            Drawer::Feed => feed_targets(),
            Drawer::Modes => modes_targets(),
        };
        // The handles stand above every drawer and stay live inside one, so
        // they share the screen with whatever the drawer carries.
        out.extend(handle_targets());
        return out;
    }
    if flow.works {
        return works_targets();
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

/// **The config drawer has room for the class after the ones it has**
/// (`FINDINGS.md` G-047, closed by wave 1.5's re-lay).
///
/// Asked about the *next* class rather than the last, so it fails one class
/// early — while the drawer still draws — which is the floor G-047 asked for:
/// the twenty-first class arrived with nowhere to be configured and nothing
/// said so in advance.
pub fn modes_have_room(checks: &mut Checks) {
    let next = crate::attention::CLASSES.len();
    let drawer = layout::modes_panel();
    for slot in 0..crate::attention::Mode::ALL.len() {
        let radio = layout::modes_radio(next, slot);
        checks.require(
            inside(drawer, radio) && !greater(radio.max.y, layout::modes_footer().y),
            "the auto-pause config has no room for another class",
            format!(
                "class {next} would put its radio {slot} at {radio:?}, outside the drawer \
                 {drawer:?} or under the footer at {:.0}. Re-lay the config before adding the \
                 class",
                layout::modes_footer().y
            ),
        );
    }
}

/// The floors that are questions about the layout alone.
pub fn layout_floors(checks: &mut Checks) {
    // **All three base screens**: the one the faces list is up on, the one
    // the job board is up on, and the one a job's candidate picker is up on.
    // They share most of their controls and differ on the left, and each has
    // to hold the floors on its own.
    for set in [
        targets(),
        board_targets(),
        picker_targets(),
        worklist_targets(),
        works_targets(),
    ] {
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
        let opening = crate::sim::Sim::opening(
            crate::scenario::freeplay(),
            &Tuning::SHIPPED,
            crate::modules::ModuleSet::ALL,
        );
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
        // The site's name as the row prints it — without its article since
        // wave 1.4 (`sim::plain`), which is what the odds-word cost the cell.
        for location in LOCATIONS {
            cells.push((
                "a site's name".to_owned(),
                crate::sim::plain(location.name).to_owned(),
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
                crate::scenario::freeplay()
                    .cast()
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
        let cast = Sim::opening(
            crate::scenario::freeplay(),
            &Tuning::SHIPPED,
            crate::modules::ModuleSet::ALL,
        )
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
        let sim = Sim::opening(
            crate::scenario::freeplay(),
            &Tuning::SHIPPED,
            crate::modules::ModuleSet::ALL,
        );
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
            crate::resolution::fit_means(&Tuning::SHIPPED, crate::modules::ModuleSet::ALL),
            crate::resolution::fit_means(
                &Tuning::SHIPPED,
                crate::modules::ModuleSet::ALL.without_id(crate::resolution::MODULE),
            ),
            format!(
                "sorted by fit - tap somebody to name them for {}, then tap its row to post it",
                Sim::opening(
                    crate::scenario::freeplay(),
                    &Tuning::SHIPPED,
                    crate::modules::ModuleSet::ALL
                )
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
        let rows = jidousha::ui::wrap(
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
    for site in Sim::opening(
        crate::scenario::freeplay(),
        &Tuning::SHIPPED,
        crate::modules::ModuleSet::ALL,
    )
    .sites
    {
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
        jidousha::ui::wrap(&longest, crate::ui::columns(width, theme::SMALL))
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
        let sim = Sim::opening(
            crate::scenario::freeplay(),
            &Tuning::SHIPPED,
            crate::modules::ModuleSet::ALL,
        );
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
        let mut sim = Sim::opening(
            crate::scenario::freeplay(),
            &tuning,
            crate::modules::ModuleSet::ALL,
        );
        sim.everybody_here();
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
            layout::modes_panel(),
            modes_targets(),
            layout::modes_button(),
        ),
        (
            layout::feed_panel(),
            pleas_targets(),
            layout::pleas_button(),
        ),
        (
            layout::voicing_panel(),
            voicing_targets()
                .into_iter()
                .filter(|(what, _)| what.starts_with("the overlay"))
                .collect(),
            layout::pleas_button(),
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

/// **The odds-words bind like every other word on a row** (wave 1.4).
///
/// Legible: every word and every tier name is lowercase ASCII and no two are
/// alike. Non-overlapping: at the shipped thresholds the three words' bands
/// over the failure chance are each non-empty and in order — every failure
/// percent from 0 to 100 reads exactly one word (by construction) and the
/// word never gets *safer* as the chance of failure rises. And they fit: the
/// widest cell any row can print — the highest fit there is beside the
/// longest word — fits the board's, the picker's and the work list's fit
/// cells alike. (That the word drawn *is* the sim's odds is
/// `resolution::judge_one_function`'s, and every row battery's.)
pub fn odds_words(checks: &mut Checks) {
    use crate::resolution::{Odds, OddsWord, Tier};
    let names: Vec<&str> = OddsWord::ALL
        .iter()
        .map(|word| word.name())
        .chain(Tier::ALL.iter().map(|tier| tier.name()))
        .collect();
    for name in &names {
        checks.require(
            !name.is_empty()
                && name
                    .chars()
                    .all(|glyph| glyph.is_ascii_lowercase() || glyph == ' '),
            "an odds-word or a tier name is not lowercase ASCII",
            format!("{name:?}"),
        );
        checks.require(
            names.iter().filter(|other| *other == name).count() == 1,
            "two odds-words or tier names are the same word",
            format!("{name:?} appears more than once"),
        );
    }
    let tuning = Tuning::SHIPPED;
    let read: Vec<usize> = (0..=100)
        .map(|fail| {
            let word = Odds { fail, well: 0 }.word(&tuning);
            OddsWord::ALL
                .iter()
                .position(|each| *each == word)
                .unwrap_or(0)
        })
        .collect();
    checks.require(
        read.windows(2).all(|pair| pair[0] <= pair[1])
            && (0..OddsWord::ALL.len()).all(|index| read.contains(&index)),
        "the odds-words' bands overlap or leave a word out",
        format!(
            "over failure chances 0..100 the words run {:?} at safe <= {} and risky >= {}; \
             each word must own a band and the bands must run safe, chancy, risky",
            read.iter()
                .map(|index| OddsWord::ALL[*index].name())
                .collect::<std::collections::BTreeSet<_>>(),
            tuning.odds_safe,
            tuning.odds_risky
        ),
    );
    let top = crate::traits::TaskType::ALL
        .iter()
        .map(|task| task.aptitude().def().aptitude)
        .max()
        .unwrap_or(0);
    let style = theme::text(theme::SMALL, theme::INK);
    let widest = OddsWord::ALL
        .iter()
        .map(|word| format!("fit {top} {}", word.name()))
        .max_by(|a, b| {
            style
                .width_of(a)
                .partial_cmp(&style.width_of(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or_default();
    // And the name cell the odds-word took forty pixels from still holds the
    // longest name in the cast.
    for person in crate::scenario::freeplay().cast() {
        checks.require(
            !greater(style.width_of(person.name), layout::cand::NAME_W),
            "a candidate row's name cell is narrower than a name in the cast",
            format!(
                "{} is {:.0} wide and the cell is {:.0}",
                person.name,
                style.width_of(person.name),
                layout::cand::NAME_W
            ),
        );
    }
    for (surface, cell) in [
        ("a job row", layout::job::FIT_W),
        ("a candidate row", layout::cand::FIT_W),
        ("a work row", layout::work::FIT_W),
    ] {
        checks.require(
            !greater(style.width_of(&widest), cell),
            "an odds-word does not fit the fit cell that prints it",
            format!(
                "{widest:?} is {:.0} wide and {surface}'s fit cell is {cell:.0}",
                style.width_of(&widest)
            ),
        );
    }
}

/// **The tuning drawer's prose band and stamp both fit, with room to grow.**
///
/// Two claims, since wave 1.6 swapped the two blocks (`FINDINGS.md` G-059).
/// The stamp, in the header band, packs into its three rows at its tallest —
/// every constant moved, the widest seed and the longest scenario id — with no
/// row wider than the band. And the prose band, at the fourth column's foot
/// (`layout::tuner_foot_for`), fits its tallest state — the longest hovered
/// meaning, every refused link, the resting line with the APPLY note — **under
/// one more constant than the game has**: the floor fails while there is still
/// room, so the wave that adds the constant is told to re-lay the column
/// instead of finding out from a screenshot the way the owner did
/// (`FINDINGS.md` G-028).
pub fn tuner_right_column(checks: &mut Checks) {
    let style = theme::text(theme::SMALL, theme::INK);
    let constants = crate::constants::Field::ALL.len();
    // --- the stamp, at its tallest ------------------------------------------
    let mut every = Tuning::SHIPPED;
    for field in crate::constants::Field::ALL.iter().copied() {
        every = every.with(field, Tuning::SHIPPED.field(field) + 1);
    }
    let longest = crate::scenario::all()
        .iter()
        .map(|scenario| scenario.id.as_str())
        .max_by_key(|id| id.len())
        .unwrap_or("");
    let tallest = tuning::stamp_text(&every, u64::from(u32::MAX), longest);
    let widest = tallest
        .lines()
        .map(|line| style.width_of(line))
        .fold(0.0f32, f32::max);
    let stamp_rows = tallest.lines().count();
    checks.require(
        stamp_rows <= layout::TUNER_STAMP_ROWS
            && !greater(widest, layout::TUNER_STAMP_W)
            && tallest.contains("more"),
        "the tuning drawer's stamp does not pack into its band",
        format!(
            "a set that moves every constant stamps {stamp_rows} rows, the widest {widest:.0} \
             wide, against a band of {} rows and {:.0} wide; it must count what it could not \
             name: {tallest:?}",
            layout::TUNER_STAMP_ROWS,
            layout::TUNER_STAMP_W
        ),
    );
    let stamp_end = layout::tuner_stamp().y + stamp_rows as f32 * (theme::SMALL + 2.0);
    checks.require(
        !greater(stamp_end, layout::tuner_name(0).y - 6.0),
        "the tuning drawer's stamp runs into the stepper rows",
        format!(
            "the stamp ends at {stamp_end:.0} and the first stepper name is at {:.0}",
            layout::tuner_name(0).y
        ),
    );
    // --- the prose band, under one more constant ------------------------------
    let prose = crate::ui::columns(layout::TUNER_HINT_W, theme::SMALL);
    let rows = |text: &str| jidousha::ui::wrap(text, prose).lines().count();
    let resting = format!("{} {}", tuning::RESTING_HINT, tuning::APPLY_NOTE);
    let tallest = crate::constants::Field::ALL
        .iter()
        .map(|field| format!("{} - {}", field.name(), field.meaning()))
        .chain(crate::links::refusals())
        .chain(std::iter::once(resting))
        .map(|line| (rows(&line), line))
        .max_by_key(|(count, _)| *count);
    if let Some((count, line)) = tallest {
        let end = layout::tuner_foot_for(constants + 1).y + count as f32 * (theme::SMALL + 2.0);
        checks.require(
            !greater(end, layout::tuner_panel().max.y),
            "the tuning drawer's prose band runs off the drawer",
            format!(
                "{line:?} wraps to {count} rows at {:.0} wide and, under {} constants, ends at \
                 {end:.0}; the drawer ends at {:.0}. Re-lay the fourth column before adding \
                 the constant",
                layout::TUNER_HINT_W,
                constants + 1,
                layout::tuner_panel().max.y
            ),
        );
    }
}

/// **The tuning drawer has room for the constant after the ones it has**
/// (`FINDINGS.md` G-034 — closed by wave 1.4's fourth column).
///
/// Asked about the *next* index rather than the last one, so it fails while
/// the drawer still draws: the next constant's stepper must be inside the
/// drawer, and (`tuner_right_column`) the stamp it pushes down must still fit.
/// And every name must fit the name cell the fourth column made narrower.
pub fn tuner_has_room(checks: &mut Checks) {
    let next = crate::constants::Field::ALL.len();
    let drawer = layout::tuner_panel();
    for (what, rect) in [
        ("its -", layout::tuner_minus(next)),
        ("its +", layout::tuner_plus(next)),
        ("its row", layout::tuner_row(next)),
    ] {
        checks.require(
            inside(drawer, rect),
            "the tuning drawer has no room for another constant",
            format!(
                "constant {next} would put {what} at ({:.0}, {:.0})-({:.0}, {:.0}) and the \
                 drawer is ({:.0}, {:.0})-({:.0}, {:.0}). Re-lay the stepper columns before \
                 adding it",
                rect.min.x,
                rect.min.y,
                rect.max.x,
                rect.max.y,
                drawer.min.x,
                drawer.min.y,
                drawer.max.x,
                drawer.max.y
            ),
        );
    }
    let style = theme::text(theme::SMALL, theme::INK);
    for field in crate::constants::Field::ALL.iter().copied() {
        let wide = style.width_of(field.name());
        checks.require(
            !greater(wide + 4.0, layout::TUNER_NAME_W),
            "a constant's name does not fit its stepper row",
            format!(
                "{} is {wide:.0} wide and the name cell is {:.0} with a four-pixel gap before \
                 the - button",
                field.name(),
                layout::TUNER_NAME_W
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
    let sim = Sim::opening(
        crate::scenario::freeplay(),
        &Tuning::SHIPPED,
        crate::modules::ModuleSet::ALL,
    );
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
    let prose = crate::ui::columns(layout::TUNER_HINT_W, theme::SMALL);
    let mut before = Panel::default();
    before.block(
        layout::tuner_foot_for(36) + Vec2::new(0.0, 14.0),
        &format!("{}\nseed 0", Tuning::SHIPPED.readout()),
        theme::text(theme::SMALL, theme::INK),
        theme::LEADING,
    );
    before.block(
        Vec2::new(layout::tuner_foot_for(36).x, PRE_FIX_HINT_Y),
        &jidousha::ui::wrap(tuning::RESTING_HINT, prose),
        theme::text(theme::SMALL, theme::FAINT),
        theme::LEADING,
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
            layout::tuner_foot_for(36),
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
    let count_bites = staged.reported("two overlays' content is in one frame");
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

/// **Every floor that moved into the kit, staged and seen to bite from its
/// new home** (ADR-0046) — silent on pass.
///
/// `floors_bite` stages the two the owner's screenshots were about (the
/// chrome overlap and the drawer count); this stages the rest, each through
/// this game's own wrapper so what is proved is the wrapper feeding the kit's
/// breach into `Checks`, by the floor's name. Seven over a panel, three over
/// a photograph whose state has been doctored to claim something its frame
/// does not carry, and the frame floor over a frame that draws text under it.
pub fn moved_floors_bite(checks: &mut Checks, run: &Conducted) {
    let judge = |panel: &Panel, what: &str, controls: &[(String, Rect)]| {
        let mut staged = Checks::default();
        judge_panel(&mut staged, panel, what, controls);
        staged
    };
    fn bites(checks: &mut Checks, floor: &'static str, staged: &Checks, how: &str) {
        checks.require(
            staged.reported(floor),
            "a floor that moved into the kit no longer bites from there",
            format!(
                "{how}: judging it reported {} problem(s) and none was {floor:?}",
                staged.failures()
            ),
        );
    }
    // --- the panel floors ---------------------------------------------------
    let mut panel = Panel::default();
    panel.text(ui::row(
        Vec2::new(10.0, 60.0),
        "a tenth under",
        theme::MIN_TEXT - 0.1,
        theme::INK,
    ));
    bites(
        checks,
        "a row of text is smaller than the readability floor allows",
        &judge(&panel, "a row a tenth under the floor", &[]),
        "a row set a tenth of a pixel under the floor",
    );
    let mut panel = Panel::default();
    panel.text(ui::row(
        Vec2::new(layout::DESIGN_W - 10.0, 60.0),
        "off the right edge",
        theme::SMALL,
        theme::INK,
    ));
    bites(
        checks,
        "a row of chrome text runs off the UI rect",
        &judge(&panel, "a row starting ten pixels from the right edge", &[]),
        "a row that starts ten pixels inside the right edge",
    );
    let chip = layout::speed_chip(0);
    let mut panel = Panel::default();
    panel.text(ui::row(
        Vec2::new(chip.min.x - 40.0, chip.min.y + 10.0),
        "a row running into a chip",
        theme::SMALL,
        theme::INK,
    ));
    bites(
        checks,
        "a row of text lies across a control it is not the label of",
        &judge(
            &panel,
            "a row running into the first speed chip",
            &[("the first speed chip".to_owned(), chip)],
        ),
        "a row that starts forty pixels left of a speed chip and runs into it",
    );
    let world = crate::grid::grid().world_rect();
    let mut panel = Panel::default();
    panel.world_text(ui::row(
        world.max + Vec2::splat(10.0),
        "past the world",
        theme::SMALL,
        theme::INK,
    ));
    bites(
        checks,
        "a map label runs off the world",
        &judge(&panel, "a label ten units past the map's corner", &[]),
        "a map label placed ten units past the map's far corner",
    );
    let mut panel = Panel::default();
    panel.world_text(ui::row(
        world.center(),
        "the Deep Cave",
        theme::SMALL,
        theme::INK,
    ));
    panel.world_text(ui::row(
        world.center() + Vec2::splat(4.0),
        "the Old Crypt",
        theme::SMALL,
        theme::INK,
    ));
    bites(
        checks,
        "two map labels overlap",
        &judge(&panel, "two labels four units apart", &[]),
        "two map labels four units apart",
    );
    let mut panel = Panel::default();
    panel.icon(ui::icon(
        Vec2::new(10.0, 60.0),
        crate::sprites::Art::Coin,
        1.5,
    ));
    bites(
        checks,
        "a pixel-art icon is drawn at a fractional scale",
        &judge(&panel, "a coin at one and a half", &[]),
        "a coin drawn at one and a half texels per texel",
    );
    let mut panel = Panel::default();
    // The coin is eight texels; at two texels per texel it is sixteen wide,
    // and starting ten inside the edge it ends six past it.
    panel.icon(ui::icon(
        Vec2::new(layout::DESIGN_W - 10.0, 60.0),
        crate::sprites::Art::Coin,
        2.0,
    ));
    bites(
        checks,
        "a chrome icon runs off the UI rect",
        &judge(&panel, "a coin ten pixels from the right edge", &[]),
        "a coin sixteen wide that starts ten pixels inside the right edge",
    );
    // --- the frame floors: a photograph whose state claims more than it has --
    //
    // The map photograph was taken with nobody selected; a flow that says
    // somebody is claims the character panel's rows and portrait, and the
    // frame has neither. The feed photograph was taken under a drawer, where
    // the map's words are silent; a flow with no drawer claims them.
    let doctored = |shot: &Shot, flow: Flow| Shot {
        name: shot.name,
        frame: shot.frame.clone(),
        sim: shot.sim.clone(),
        clock: shot.clock,
        flow,
        camera: shot.camera,
    };
    if let Some(shot) = run.photo("map") {
        let mut flow = shot.flow.clone();
        flow.selected = Some(0);
        let claimed = doctored(shot, flow);
        let mut staged = Checks::default();
        crate::frames::judge_chrome(&mut staged, run, &claimed, "the map, claiming a sheet");
        bites(
            checks,
            "a row of the chrome is not drawn as the string it is",
            &staged,
            "the map photograph judged against a flow that claims the character panel",
        );
        bites(
            checks,
            "an icon the screen says it draws is not on the frame",
            &staged,
            "the map photograph judged against a flow that claims a portrait",
        );
    } else {
        checks.require(false, "the map photograph was not taken", String::new());
    }
    if let Some(shot) = run.photo("feed") {
        let mut flow = shot.flow.clone();
        flow.drawer = None;
        let claimed = doctored(shot, flow);
        let mut staged = Checks::default();
        crate::frames::judge_chrome(
            &mut staged,
            run,
            &claimed,
            "the feed, claiming the map's words",
        );
        bites(
            checks,
            "a map label is not drawn as the string it is",
            &staged,
            "the feed photograph judged against a flow with no drawer up",
        );
    } else {
        checks.require(false, "the feed photograph was not taken", String::new());
    }
    // --- the frame floor: a frame that draws under it -----------------------
    fn under_the_floor(ctx: &mut DrawCtx) {
        ctx.text(
            Vec2::ZERO,
            "under",
            TextStyle {
                size: theme::MIN_TEXT - 1.0,
                ..theme::text(theme::SMALL, theme::INK)
            },
        );
    }
    let mut sim = headless(crate::config(), |app| {
        app.add_system(Draw, under_the_floor);
    });
    let mut recorder = jidousha::testing::FrameRecorder::new(verify::HEADLESS_VIEWPORT);
    let frame = recorder.draw(&mut sim);
    let mut staged = Checks::default();
    judge_frame_floor(
        &mut staged,
        recorder.font_texture(),
        &frame,
        "a frame with a glyph a pixel under the floor",
    );
    bites(
        checks,
        "a glyph was drawn below the readability floor",
        &staged,
        "a frame drawn with text one pixel under the floor",
    );
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
        Sim::opening(
            crate::scenario::freeplay(),
            &Tuning::SHIPPED,
            crate::modules::ModuleSet::ALL,
        ),
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
    roster_open.explained.lit = crate::traits::TRAITS
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
    looked_at.drilled.lit = Some(0);
    // With the longest explanation a trait has, open on the sheet: a panel
    // that fits its widest state fits every other one.
    looked_at.explained.lit = crate::traits::TRAITS
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
    explained.breakdown = Some(crate::flow::Breakdown::Job(crate::sim::JobId {
        site: explained.board.unwrap_or(0),
        slot: 0,
    }));
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
    // **The settlement panel, in both of its states** (UI.md §3g): a camp with
    // nothing built and a settlement with the works standing and somebody on
    // them, which are the two things its rows say and the two lengths they say
    // them at.
    let works = Flow {
        works: true,
        ..played.clone()
    };
    let mut standing = baseline.sim.clone();
    standing.everybody_here();
    standing.treasury = 10_000;
    let _ = crate::settlement::build(&mut standing, ended_clock.minutes, 0);
    if let Some(spec) = crate::settlement::INDUSTRIES.first()
        && let Some(site) = standing.sites.get_mut(spec.site)
        && let Some(state) = site.states.first_mut()
    {
        *state = crate::sim::JobState::Claimed { by: 0 };
    }
    // **The petitions' surfaces at their loudest** (wave 1.5): the overlay
    // with the chip's explanation open, the ledger with every kind of row and
    // its card explained, and the ledger on the card whose explanation is
    // longest — a gives-away naming the one it speaks for.
    let (pleaded, voiced, pleaded_clock, last) = petitioned_world();
    // The chip is open for its own card's id: the voiced petition on the
    // overlay, the first ledger row on the mixed ledger, the widest card on
    // its own.
    let voicing_flow = Flow {
        consequence_open: jidousha::ui::Chip { lit: Some(last) },
        ..Flow::default()
    };
    let first_row = crate::card::ledger_order(&Lens::on(&pleaded))
        .first()
        .copied();
    let pleas_flow = Flow {
        drawer: Some(Drawer::Pleas),
        consequence_open: jidousha::ui::Chip { lit: first_row },
        ..Flow::default()
    };
    let widest = pleaded
        .petitions
        .all()
        .iter()
        .find(|petition| petition.template.id == "look-after-them")
        .map(|petition| petition.id);
    let widest_flow = Flow {
        drawer: Some(Drawer::Pleas),
        consequence_open: jidousha::ui::Chip { lit: widest },
        plea: widest,
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
            "the settlement panel over a camp with nothing built",
            works.clone(),
            baseline.sim.clone(),
            ended_clock,
            at,
        ),
        (
            "the settlement panel with the works standing and somebody on them",
            works,
            standing,
            ended_clock,
            at,
        ),
        (
            "the settlement one notch of zoom out, with somebody picked",
            picked,
            Sim::opening(
                crate::scenario::freeplay(),
                &Tuning::SHIPPED,
                crate::modules::ModuleSet::ALL,
            ),
            Clock::opening(),
            out,
        ),
        (
            "the voicing overlay, its chip explained",
            voicing_flow,
            voiced,
            pleaded_clock,
            at,
        ),
        (
            "the petition ledger, mixed, its card's chip explained",
            pleas_flow,
            pleaded.clone(),
            pleaded_clock,
            at,
        ),
        (
            "the petition ledger on its widest card",
            widest_flow,
            pleaded.clone(),
            pleaded_clock,
            at,
        ),
        (
            "the petition ledger, the world stopped by a failure",
            Flow {
                drawer: Some(Drawer::Pleas),
                ..Flow::default()
            },
            stopped_by_failure(pleaded),
            pleaded_clock,
            at,
        ),
    ]
}

/// **A camp that has petitioned**, staged for the floors (wave 1.5): six
/// petitions running from six templates, one met by a gift and one failed at
/// its cliff — every shape a ledger row and a card can take — and a world
/// stopped for the last voicing, for the overlay.
pub(crate) fn petitioned_world() -> (Sim, Sim, Clock, usize) {
    use crate::pleas::{self, Found};
    let tuning = Tuning::SHIPPED;
    let mut sim = Sim::opening(
        crate::scenario::freeplay(),
        &tuning,
        crate::modules::ModuleSet::ALL,
    );
    sim.everybody_here();
    sim.treasury = 400;
    let raise = |sim: &mut Sim, who: usize, id: &str, found: Found| {
        crate::petitions::find(id)
            .and_then(|template| pleas::raise(sim, &tuning, 1500, who, template, found))
    };
    let site = |site| Found {
        other: None,
        site: Some(site),
        n: None,
    };
    let given = raise(&mut sim, 0, "collectors-visit", Found::default());
    let failed = raise(&mut sim, 7, "collectors-visit", Found::default());
    raise(&mut sim, 1, "thin-days", Found::default());
    raise(
        &mut sim,
        6,
        "look-after-them",
        Found {
            other: Some(1),
            site: None,
            n: None,
        },
    );
    raise(&mut sim, 9, "the-far-road", site(3));
    raise(&mut sim, 5, "proving-job", site(3));
    raise(&mut sim, 4, "a-proper-bench", Found::default());
    if let Some(id) = given {
        let _ = pleas::give(&mut sim, &tuning, 1600, id);
    }
    if let Some(id) = failed {
        pleas::deadline(&mut sim, &tuning, 1500 + 6 * crate::petitions::DAY, id);
    }
    // The chain's next is the last voicing: stop the world for it.
    let mut voiced = sim.clone();
    let event = voiced
        .events
        .iter()
        .rposition(|event| event.class == crate::attention::EventClass::PetitionVoiced);
    let last = event
        .and_then(|index| voiced.events[index].petition)
        .unwrap_or(0);
    if let Some(index) = event {
        voiced.paused_by = Some(crate::attention::Pause {
            event: index,
            class: crate::attention::EventClass::PetitionVoiced,
            minute: voiced.events[index].minute,
        });
    }
    let mut clock = Clock::opening();
    clock.minutes = 1500 + 2 * crate::petitions::DAY;
    (sim, voiced, clock, last)
}

/// **The staged camp, stopped by its failed petition** — the pause a player
/// sitting in the ledger meets when a cliff falls.
pub(crate) fn stopped_by_failure(mut sim: Sim) -> Sim {
    let failed = sim
        .events
        .iter()
        .rposition(|event| event.class == crate::attention::EventClass::PetitionFailed);
    if let Some(index) = failed {
        sim.paused_by = Some(crate::attention::Pause {
            event: index,
            class: crate::attention::EventClass::PetitionFailed,
            minute: sim.events[index].minute,
        });
    }
    sim
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
        judge_panel(checks, &panel, what, &controls_for(&flow, &Lens::on(&sim)));
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

/// One panel against the floors: the kit's generic floors over this game's
/// numbers (`floors()`), its control set and its drawers, then the one floor
/// that is this game's own.
pub fn judge_panel(checks: &mut Checks, panel: &Panel, what: &str, controls: &[(String, Rect)]) {
    // **At most one drawer's content in the frame.** With `Flow::drawer` a
    // single `Option<Drawer>` this is unrepresentable, and the floor says it
    // anyway: the next surface to grow an open-flag of its own should fail
    // here rather than be found in a screenshot. The kit counts the drawers
    // by the head row each of them draws — `Drawer::title`, the one string
    // the drawer prints and the floor reads, so the two cannot drift apart
    // into a floor that sees nothing.
    let drawers: Vec<(&str, &str)> = Drawer::ALL
        .into_iter()
        .map(|drawer| (drawer.label(), drawer.title()))
        .collect();
    for breach in jidousha::ui::judge_panel(panel, &floors(), controls, &drawers) {
        checks.require(false, breach.what, format!("{what}: {}", breach.detail));
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
    for breach in jidousha::ui::frame_text_floor(frame, font, theme::MIN_TEXT) {
        checks.require(false, breach.what, format!("{what}: {}", breach.detail));
    }
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
        judge_panel(
            checks,
            &panel,
            what,
            &controls_for(flow, &Lens::on(&drawer.applied_sim)),
        );
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
        &controls_for(&refused, &Lens::on(&drawer.applied_sim)),
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
        // **Somebody who has not arrived has no figure at all** (`CAST.md` §4,
        // wave 1.3) — the other half of "one person, one figure": a person who
        // is not in the camp is drawn nowhere, and the floor says so in both
        // directions so a presence bug cannot hide as a missing sprite.
        if !lens.present(index) {
            checks.require(
                mine.is_empty(),
                "somebody who has not arrived is drawn on the map",
                format!(
                    "{what}: {} figure(s) carry {}'s portrait and they do not reach Kawaza \
                     until minute {}",
                    mine.len(),
                    lens.name(index),
                    lens.arrives(index)
                ),
            );
            continue;
        }
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
            let alone = lens
                .roll()
                .into_iter()
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
