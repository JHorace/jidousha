//! **What a tap on a petition does** — the voicing overlay's clicks and the
//! petition ledger's (wave 1.5; UI.md §3h).
//!
//! Three gestures and no more. **ARRANGE navigates**: it puts the drawer down
//! and opens the surface the card's condition is acted on, with the right
//! person selected, and it posts nothing — the board's row, the work list's
//! row and the settlement panel's BUILD are still the only things that act.
//! **GIVE** is the one exception, the §4.1 petition gift, and only where the
//! condition is money-shaped. **LATER** puts the overlay down to the ledger.
//! Everything else on a card is the consequence chip, which explains.
//!
//! The record is never written from here except through `pleas::give`; what
//! is written is the UI's own state (`Flow`), and the one recorded input that
//! is not — the gift — is made at the clock's minute like a posting.

use jidousha::prelude::*;

use crate::card::{self, Arrange};
use crate::clock::{Clock, stamp};
use crate::constants::Tuning;
use crate::flow::Flow;
use crate::lens::Lens;
use crate::sim::Sim;
use crate::{layout, sim};

/// A click while the world is stopped for a voicing.
///
/// The overlay is the screen: LATER takes the card to the ledger and
/// acknowledges the pause, the chip explains, the speed chips resume the
/// world (which is the other way to acknowledge it), and anything else
/// bounces with what is waiting — a tap that silently did nothing would be
/// the silent failure this game refuses.
pub fn overlay_click(world: &mut World, at: Vec2, tick: u64) {
    let Some(id) = card::voicing(&Lens::on(world.resource::<Sim>())) else {
        return;
    };
    let origin = layout::voicing_card();
    if layout::card_act(origin).contains(at) {
        sim::acknowledge_pause(world.resource_mut::<Sim>());
        world.resource_mut::<Flow>().open_pleas_on(id);
        return;
    }
    if layout::card_chip(origin).contains(at) {
        let flow = world.resource_mut::<Flow>();
        flow.consequence_open = !flow.consequence_open;
        return;
    }
    let chips = [
        None,
        Some(crate::clock::Rate::X1),
        Some(crate::clock::Rate::X2),
        Some(crate::clock::Rate::X4),
    ];
    for (index, change) in chips.into_iter().enumerate() {
        if layout::speed_chip(index).contains(at) {
            crate::flow::apply_speed(world, tick, change);
            return;
        }
    }
    world.resource_mut::<Flow>().bounce(
        tick,
        "a petition is waiting - LATER puts it in the ledger, space resumes".to_owned(),
    );
}

/// A click inside the open petition ledger.
///
/// A row puts its card beside the list; the card's chip explains; ARRANGE
/// goes where the condition is acted on; GIVE gives. Anything else shuts the
/// drawer, which is every drawer's rule.
pub fn ledger_click(world: &mut World, at: Vec2, tick: u64) {
    let (order, focus) = {
        let sim = world.resource::<Sim>();
        let lens = Lens::on(sim);
        (
            card::ledger_order(&lens),
            card::focused(world.resource::<Flow>(), &lens),
        )
    };
    for (row, id) in order.iter().take(layout::PLEA_ROWS).enumerate() {
        if layout::plea_row(row).contains(at) {
            let flow = world.resource_mut::<Flow>();
            if flow.plea != Some(*id) {
                flow.consequence_open = false;
            }
            flow.plea = Some(*id);
            return;
        }
    }
    let Some(id) = focus else {
        world.resource_mut::<Flow>().drawer = None;
        return;
    };
    let origin = layout::plea_card();
    if layout::card_chip(origin).contains(at) {
        let flow = world.resource_mut::<Flow>();
        flow.consequence_open = !flow.consequence_open;
        return;
    }
    let (active, money) =
        world
            .resource::<Sim>()
            .petitions
            .get(id)
            .map_or((false, false), |petition| {
                (
                    petition.active(),
                    petition.template.condition.wallet_shaped(),
                )
            });
    if active && layout::card_act(origin).contains(at) {
        arrange(world, tick, id);
        return;
    }
    if active && money && layout::card_give(origin).contains(at) {
        give(world, tick, id);
        return;
    }
    if layout::card_rect(origin).contains(at) {
        // Inside the card's own words: a tap that reads, and does nothing.
        return;
    }
    world.resource_mut::<Flow>().drawer = None;
}

/// **ARRANGE** — go to where this petition's condition is acted on.
///
/// Navigation and nothing else: the drawer goes down, and the surface the
/// condition's subject is acted on comes up — their work list, the board of
/// the site the card names with them selected, or the settlement panel. Nobody
/// absent can be arranged for; that bounces with why.
pub fn arrange(world: &mut World, tick: u64, id: usize) {
    let target = {
        let sim = world.resource::<Sim>();
        let Some(petition) = sim.petitions.get(id) else {
            return;
        };
        card::arrange(petition)
    };
    let absent = |world: &World, who: usize| {
        let lens = Lens::on(world.resource::<Sim>());
        (!lens.present(who))
            .then(|| format!("{} is not in the camp to arrange for", lens.name(who)))
    };
    let flow_note = |world: &mut World, what: String| {
        let now = world.resource::<Clock>().minutes;
        world
            .resource_mut::<Flow>()
            .note(format!("{} - arranging: {what}", stamp(now)));
    };
    match target {
        Arrange::WorkOf(who) => {
            if let Some(why) = absent(world, who) {
                world.resource_mut::<Flow>().bounce(tick, why);
                return;
            }
            let flow = world.resource_mut::<Flow>();
            flow.close_everything();
            flow.selected = Some(who);
            flow.listing = Some(who);
            let name = Lens::on(world.resource::<Sim>()).name(who).to_owned();
            flow_note(world, format!("{name}'s work list"));
        }
        Arrange::Board { who, site } => {
            if let Some(why) = absent(world, who) {
                world.resource_mut::<Flow>().bounce(tick, why);
                return;
            }
            let flow = world.resource_mut::<Flow>();
            flow.close_everything();
            flow.selected = Some(who);
            flow.board = Some(site);
            let place = crate::grid::LOCATIONS[sim::site_location(site)].name;
            flow_note(world, format!("{place}'s board"));
        }
        Arrange::Works => {
            let flow = world.resource_mut::<Flow>();
            flow.close_everything();
            flow.works = true;
            flow_note(world, "the settlement panel".to_owned());
        }
    }
}

/// **GIVE** — the petition gift, at the clock's minute, a recorded input like
/// a posting. Refused gifts bounce with the two numbers.
pub fn give(world: &mut World, tick: u64, id: usize) {
    let now = world.resource::<Clock>().minutes;
    let tuning = *world.resource::<Tuning>();
    let outcome = {
        let sim = world.resource_mut::<Sim>();
        crate::pleas::give(sim, &tuning, now, id)
    };
    let name = {
        let sim = world.resource::<Sim>();
        sim.petitions
            .get(id)
            .and_then(|petition| sim.people.get(petition.who))
            .map_or("them", |person| person.name)
    };
    let flow = world.resource_mut::<Flow>();
    match outcome {
        Ok(amount) => flow.note(format!("{} - gave {name} {amount}g", stamp(now))),
        Err(refused) => flow.bounce(tick, refused.message()),
    }
}
