//! **The settlement panel** — what the camp is, and what the player can build
//! and pay for (UI.md §3g, wave 1.3).
//!
//! The camp's own marker opens it, the way a site's marker opens that site's
//! board, and it stands in the same column and draws instead of it: the left
//! of the screen is one surface at a time and the character panel has the
//! other.
//!
//! **Two decisions live here** (the wave's decision-surface table): whether to
//! build an industry — which needs its cost, what it opens, the treasury, and
//! who is idle and who is short — and what to pay a shift of it, which needs
//! the standing rate for that kind of work and what upkeep costs a person.
//! Every one of those figures is the simulation's own: the treasury is
//! `Lens::treasury`, the cost and the slots are the industry's row, who is
//! short is `needs::is_short` (the chip's own predicate, and the burn's), and
//! the rate is `Rates::of`. Nothing here computes anything twice.

use crate::attention::CHIP;
use crate::constants::Tuning;
use crate::flow::Flow;
use crate::lens::Lens;
use crate::panels::clipped;
use crate::settlement::{self, INDUSTRIES};
use crate::ui::{self, Panel};
use crate::{layout, theme};

/// **What the camp is called right now** — the phasing arc's first beat, said
/// in the one place the player can cause it (`CAST.md` §1, GDD §1).
pub fn standing_as(lens: &Lens<'_>) -> &'static str {
    if lens.settlement().any_standing() {
        "a settlement"
    } else {
        "a camp"
    }
}

/// **Whether a shift can go wrong, in words — derived, never written** (wave
/// 1.4's honest dormancy).
///
/// A shift is not rolled: `resolution::rolls_at` says no for an industry's
/// site, and that predicate is what this reads, so the day a shift gains
/// odds the line changes because the rule did. With the module off nothing is
/// rolled anywhere, and the line says so in the trait chips' own words.
pub fn shift_odds(lens: &Lens<'_>) -> String {
    let rolled = INDUSTRIES
        .iter()
        .filter_map(|spec| lens.site(spec.site))
        .any(crate::resolution::rolls_at);
    if !lens.modules().enabled(crate::resolution::MODULE) {
        crate::traits::Consumer::Resolution.absence().to_owned()
    } else if rolled {
        "a shift can go wrong, as a job can".to_owned()
    } else {
        "a shift always pays, whatever the fit - only a job at a site is rolled".to_owned()
    }
}

/// **The settlement panel.**
pub fn settlement_panel(flow: &Flow, lens: &Lens<'_>, tuning: &Tuning) -> Panel {
    let mut panel = Panel::default();
    let town = crate::grid::LOCATIONS[crate::grid::TOWN].name;
    panel.text(ui::row(
        layout::works_title(),
        clipped(
            &format!("{town} - {}", standing_as(lens)),
            layout::WORKS_HEAD_W,
        ),
        theme::SMALL,
        theme::INK,
    ));
    // **The three facts a build decision turns on**, from the same functions
    // the meters band's chips and the state-of-the-camp line are made of: what
    // is held, who is free to work and who cannot pay for themselves.
    let idle = lens
        .roll()
        .into_iter()
        .filter(|who| lens.at_home(*who))
        .count();
    panel.text(ui::row(
        layout::works_state(),
        clipped(
            &format!("{} - {idle} idle", crate::needs::camp_line(lens, tuning)),
            layout::WORKS_HEAD_W,
        ),
        theme::SMALL,
        theme::GOLD,
    ));
    let close = layout::works_close();
    panel.text(ui::row(
        crate::ui::centered(close, "X", theme::BODY, close.min.y + 10.0),
        "X",
        theme::BODY,
        theme::INK,
    ));

    for (index, spec) in INDUSTRIES.iter().take(layout::WORKS_ROWS).enumerate() {
        let at = layout::works_row(index).min;
        let standing = lens.settlement().standing(index);
        let art = spec.task.aptitude().icon();
        panel.icon(ui::icon(
            at + layout::works::TASK_ICON,
            art,
            art.scale_across(CHIP),
        ));
        panel.text(ui::row(
            at + layout::works::NAME,
            clipped(spec.name, layout::works::NAME_W),
            theme::SMALL,
            if standing { theme::INK } else { theme::DIM },
        ));
        // **What it costs and what it opens** — the two halves of the build
        // decision, and after it is built the second half becomes what is
        // free, because a slot nobody is on is the thing to act on.
        let free = settlement::free_slots(lens, index);
        panel.text(ui::row(
            at + layout::works::COST,
            clipped(
                &if standing {
                    format!("{} slots, {free} free", spec.slots)
                } else {
                    format!("{}g - {} slots", spec.cost, spec.slots)
                },
                layout::works::COST_W,
            ),
            theme::SMALL,
            if standing { theme::DIM } else { theme::GOLD },
        ));
        // **Who is working it now.** Named, never counted: this game's whole
        // argument is that a settlement is people, and "2 of 3" is the bare
        // number the chips are forbidden to be.
        let hands = settlement::hands_on(lens, index);
        let who = if !standing {
            "nothing stands here yet".to_owned()
        } else if hands.is_empty() {
            "nobody is on it yet".to_owned()
        } else {
            format!(
                "on it: {}",
                hands
                    .iter()
                    .map(|who| lens.name(*who))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        panel.text(ui::row(
            at + layout::works::HANDS,
            clipped(&who, layout::works::HANDS_W),
            theme::SMALL,
            if hands.is_empty() {
                theme::FAINT
            } else {
                theme::REGARD
            },
        ));
        let wage = lens.settlement().wage(index);
        let rate = lens.standing_rate(spec.task);
        // **What the wage means**, which is the standing rate for that kind of
        // work: the one expectation `answers::wage_regard` judges a payment
        // against, so the row says what the simulation is about to do.
        let against = match wage.cmp(&rate) {
            std::cmp::Ordering::Greater => {
                format!(
                    "{wage}g a shift - over the {}g {} rate",
                    rate,
                    spec.task.id()
                )
            }
            std::cmp::Ordering::Less => {
                format!(
                    "{wage}g a shift - under the {}g {} rate",
                    rate,
                    spec.task.id()
                )
            }
            std::cmp::Ordering::Equal => {
                format!("{wage}g a shift - the {}g {} rate", rate, spec.task.id())
            }
        };
        panel.text(ui::row(
            at + layout::works::AGAINST,
            clipped(&against, layout::works::AGAINST_W),
            theme::SMALL,
            theme::DIM,
        ));
        // The controls: BUILD while it is not standing, and the wage stepper
        // always, because what camp work pays is a decision before and after.
        let build = layout::works_build(index);
        let affordable = lens.treasury() >= spec.cost;
        let (verb, tone) = if standing {
            ("STANDING", theme::FAINT)
        } else if affordable && lens.settlement_on() {
            ("BUILD", theme::GOLD)
        } else {
            ("BUILD", theme::FAINT)
        };
        panel.text(ui::row(
            crate::ui::centered(build, verb, theme::SMALL, build.min.y + 10.0),
            verb,
            theme::SMALL,
            tone,
        ));
        for (rect, glyph) in [
            (layout::works_wage_down(index), "-"),
            (layout::works_wage_up(index), "+"),
        ] {
            panel.text(ui::row(
                crate::ui::centered(rect, glyph, theme::BODY, rect.min.y + 8.0),
                glyph,
                theme::BODY,
                theme::INK,
            ));
        }
        let value = layout::works_wage_value(index);
        let shown = format!("{wage}g");
        panel.text(ui::row(
            crate::ui::centered(value, &shown, theme::SMALL, value.min.y + 10.0),
            shown,
            theme::SMALL,
            theme::GOLD,
        ));
    }

    let hint = if !lens.settlement_on() {
        "the settlement module is off - there is nothing to build, and needs rest entirely on \
         the work the sites authored"
            .to_owned()
    } else if lens.settlement().any_standing() {
        format!(
            "a shift is standing work: nobody is posted to it, a finished shift opens again, \
             and {}",
            shift_odds(lens)
        )
    } else {
        format!(
            "building is the treasury's first real sink, and the first building is the beat \
             where {town} stops being a camp"
        )
    };
    panel.hint(
        layout::works_hint(),
        layout::WORKS_HINT_W,
        &hint,
        theme::text(theme::SMALL, theme::FAINT),
        theme::LEADING,
    );
    let _ = flow;
    panel
}
