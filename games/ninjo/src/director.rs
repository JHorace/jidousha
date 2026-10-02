//! **The events-director — the minimal injector** (GDD §5, wave 1.6): the
//! world moves without you.
//!
//! # What it is, and what it is not
//!
//! The director is the anti-aimlessness lever, and its MVP shape was decided
//! with the capsule (2026-08-29): a **minimal injector** with three canned,
//! petition-flavoured templates — `CAST.md` §6's D1-D3 — so the MVP gate
//! measures the character loop under mild pressure rather than in a vacuum.
//! Pressure curves and storyteller pacing are the full director, post-MVP,
//! and nothing here reaches for them.
//!
//! **It speaks only through the petition machinery.** A firing raises a
//! petition with `pleas::raise`, the one door into the record, and from there
//! it is a petition like any other: voiced at once or by the messenger, the
//! same card, the same ledger, the same cliff. The only difference the player
//! can see is the source chip, which says `event` (`petitions::SOURCES`).
//!
//! # Every firing is an occurrence
//!
//! `Occ::Director` on the one scheduler. The calm window ends with an
//! unarmed occurrence at `calm_days` (nothing fires before it, in any world);
//! from there each firing draws the next at **its own address** — the seed,
//! the world-minute and a salt ([`draw`]), never a frame and never call
//! order — between half and one and a half `director_hours` ([`gap`]). The
//! target, the template, the `{site}` and the `{n}` are drawn the same way,
//! so the same seed and orders raise the same petitions at the same minutes
//! under every speed script (`FINDINGS.md` G-016's rule, G-045's mix).
//!
//! # A firing reaches somebody, or passes
//!
//! It reaches **somebody eligible**: present, carrying no petition (the
//! one-active-petition rule, as ever), and holding a trait the template's
//! trigger names ([`reach`], roster order, template order). It passes — a
//! feed line at `ignore`, because the world being quiet is not an event —
//! when nobody is, and when `director_max` director petitions are already
//! unresolved.
//!
//! **A scenario's pins** fire the same way at their scripted minutes,
//! exactly then: past the calm window, past the cap and whether or not the
//! scenario lets the director speak on its own — they are the scenario's
//! script, and the director is what happens between them.

use crate::constants::Tuning;
use crate::modules::ModuleSet;
use crate::petitions::{self, DAY, SiteSlot, Source, Template};
use crate::pleas::{self, Found};
use crate::sim::{PLAYER, Sim};

/// The module id, as GDD §5's registry spells it.
pub const MODULE: &str = "events-director";

/// **Whether the injector runs in a world with these modules** — its own row
/// on, and petitions on: it speaks only through them, so with petitions off
/// it is silent too (asserted by `pressed::module_off`, not assumed).
pub fn runs(modules: ModuleSet) -> bool {
    modules.enabled(MODULE) && modules.enabled(petitions::MODULE)
}

/// **The end of the calm window** — the world-minute before which the
/// director fires nothing.
pub fn calm_until(tuning: &Tuning) -> u64 {
    u64::try_from(tuning.calm_days.max(0)).unwrap_or(0) * DAY
}

/// What a draw is for — the salt that keeps two draws at one minute apart.
#[derive(Clone, Copy, Debug)]
enum Salt {
    Gap = 1,
    Pick = 2,
    Site = 3,
    Spread = 4,
}

/// **One draw, addressed by the occurrence** — the seed, the world-minute and
/// what it is for, mixed before seeding (G-045), and nothing else.
fn draw(seed: u64, minute: u64, salt: Salt, below: u64) -> u64 {
    fn mix(mut z: u64) -> u64 {
        z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    const DIRECTOR: u64 = 0x6469_7265_6374_6f72;
    if below <= 1 {
        return 0;
    }
    let address = mix(mix(mix(seed ^ DIRECTOR) ^ minute) ^ salt as u64);
    let limit = u32::try_from(below).unwrap_or(u32::MAX);
    u64::from(jidousha::prelude::Rng::from_seed(address).below(limit))
}

/// **The gap to the next firing**, drawn at the firing's address: between
/// half and one and a half `director_hours`, so the mean is the drawer row.
pub fn gap(tuning: &Tuning, seed: u64, at: u64) -> u64 {
    let period = u64::try_from(tuning.director_hours.max(1)).unwrap_or(1) * 60;
    (period / 2 + draw(seed, at, Salt::Gap, period + 1)).max(1)
}

/// The director's rows of the template table, in table order.
pub fn rows() -> Vec<&'static Template> {
    petitions::TEMPLATES
        .iter()
        .filter(|template| template.source == Source::Director)
        .collect()
}

/// **How many director-sourced petitions stand unresolved** — raised and not
/// yet met or failed, voiced or still on the messenger's road.
pub fn active(sim: &Sim) -> usize {
    sim.petitions
        .all()
        .iter()
        .filter(|petition| petition.template.source == Source::Director)
        .filter(|petition| !petition.status.resolved())
        .count()
}

/// The cap, as a count.
pub fn cap(tuning: &Tuning) -> usize {
    usize::try_from(tuning.director_max.max(0)).unwrap_or(0)
}

/// **The authored sites a `{site}` slot may name** — every one for the
/// rumour of a rival, only those with open work for the word from the road.
pub fn sites_for(sim: &Sim, slot: SiteSlot) -> Vec<usize> {
    sim.sites
        .iter()
        .enumerate()
        .filter(|(_, site)| site.industry.is_none())
        .filter(|(_, site)| match slot {
            SiteSlot::None => false,
            SiteSlot::Any => true,
            SiteSlot::OpenWork => site.open_count() > 0,
        })
        .map(|(index, _)| index)
        .collect()
}

/// **Everybody an event could reach now**, as (who, template) in roster order
/// and then template order — the eligible set a firing draws from.
pub fn reach(
    sim: &Sim,
    tuning: &Tuning,
    now: u64,
    rows: &[&'static Template],
) -> Vec<(usize, &'static Template)> {
    let roll = crate::lens::Lens::on(sim).roll();
    let mut out = Vec::new();
    for who in roll {
        let free = sim
            .people
            .get(who)
            .is_some_and(|person| person.active_petition.is_none());
        if !free {
            continue;
        }
        for template in rows {
            if now >= template.opens_at() && pleas::holds(sim, tuning, now, who, template).is_some()
            {
                out.push((who, *template));
            }
        }
    }
    out
}

/// **One firing** — `Occ::Director`, armed.
pub fn fire(sim: &mut Sim, tuning: &Tuning, now: u64) {
    fire_from(sim, tuning, now, &rows());
}

/// The same over a stated set of rows — what the battery's mutation round
/// hands a mutated D1-D3 to.
pub fn fire_from(sim: &mut Sim, tuning: &Tuning, now: u64, rows: &[&'static Template]) {
    if !runs(sim.modules) {
        return;
    }
    let town = crate::grid::LOCATIONS[crate::grid::TOWN].tile;
    let standing = active(sim);
    if standing >= cap(tuning) {
        sim.emit_director(
            now,
            PLAYER,
            town,
            format!(
                "heard nothing new from outside the camp - {standing} event(s) already in play, \
                 the most there may be"
            ),
        );
        return;
    }
    let eligible = reach(sim, tuning, now, rows);
    if eligible.is_empty() {
        sim.emit_director(
            now,
            PLAYER,
            town,
            "heard nothing from outside the camp - nobody an event could reach".to_owned(),
        );
        return;
    }
    let pick = draw(sim.seed, now, Salt::Pick, eligible.len() as u64);
    let (who, template) = eligible[usize::try_from(pick).unwrap_or(0)];
    carry_in(sim, tuning, now, who, template, "");
}

/// **A scenario's pin** — `Occ::Pin`, at exactly its minute.
pub fn pinned(sim: &mut Sim, tuning: &Tuning, now: u64, pin: usize) {
    if !runs(sim.modules) {
        return;
    }
    let Some(scripted) = sim.scenario.pins.get(pin).copied() else {
        return;
    };
    let eligible: Vec<usize> = reach(sim, tuning, now, &[scripted.template])
        .into_iter()
        .map(|(who, _)| who)
        .filter(|who| scripted.who.is_none_or(|named| named == *who))
        .collect();
    let when = crate::clock::stamp(now);
    if eligible.is_empty() {
        let whom = scripted
            .who
            .and_then(|who| sim.people.get(who))
            .map_or("anybody".to_owned(), |person| person.name.to_owned());
        sim.emit_director(
            now,
            PLAYER,
            crate::grid::LOCATIONS[crate::grid::TOWN].tile,
            format!(
                "heard nothing from outside the camp - the {} pinned at {when} could not reach \
                 {whom}",
                scripted.template.id
            ),
        );
        return;
    }
    let pick = draw(sim.seed, now, Salt::Pick, eligible.len() as u64);
    let who = eligible[usize::try_from(pick).unwrap_or(0)];
    carry_in(
        sim,
        tuning,
        now,
        who,
        scripted.template,
        &format!(", pinned at {when}"),
    );
}

/// **Carry an event in** — fill its slots at this firing's address, say so,
/// and raise it through the one door.
fn carry_in(
    sim: &mut Sim,
    tuning: &Tuning,
    now: u64,
    who: usize,
    template: &'static Template,
    pinned: &str,
) {
    let site = match template.trigger {
        petitions::Trigger::Carries { site, .. } => {
            let sites = sites_for(sim, site);
            let pick = draw(sim.seed, now, Salt::Site, sites.len() as u64);
            sites.get(usize::try_from(pick).unwrap_or(0)).copied()
        }
        _ => None,
    };
    let spread = u64::try_from(template.spread.max(0)).unwrap_or(0);
    let n = template.n + i64::try_from(draw(sim.seed, now, Salt::Spread, spread + 1)).unwrap_or(0);
    let tile = sim
        .parties
        .get(who)
        .map_or(crate::grid::LOCATIONS[crate::grid::TOWN].tile, |party| {
            party.tile
        });
    sim.emit_director(
        now,
        who,
        tile,
        format!(
            "is reached by an event from outside the camp ({}{pinned})",
            template.id
        ),
    );
    let raised = pleas::raise(
        sim,
        tuning,
        now,
        who,
        template,
        Found {
            other: None,
            site,
            n: Some(n),
        },
    );
    if raised.is_none() {
        crate::checks::fail(
            "the director reached somebody and the petition was not raised",
            &format!(
                "{} was eligible for {} at minute {now} and pleas::raise refused it; reach and \
                 raise disagree about who can carry a petition",
                who, template.id
            ),
        );
    }
}
