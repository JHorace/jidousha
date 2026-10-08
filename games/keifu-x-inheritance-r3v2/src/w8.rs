//! W8's oracle, staged and refused: Garrick's death page, read and chosen on through the
//! screen by the scripted pointer.
//!
//! **The oracle** (MODULES.md W8), on every recorded seed: year 1, stay home, into the
//! winter; the founding household aged to Garrick's death — Garrick alone made 93, so
//! the turning's +1 brings him to 94, where old age is certain (CONSTANTS §10); the
//! winter let pass; "Go on" to his page. It reads "The house is one fewer", "In memory
//! of Garrick Thorne", and offers, in this order, "Maren, daughter", "Pip, grandson",
//! "Odo, friend", "Ysolde, of the house", "Brannoc, of the house", "Wren, of the house"
//! and "No one. Let it lie." under "WHO IS HIS HEIR? One choice. It cannot be unmade."
//! **The refusal**: the pointer tries to leave — "Go on", "Skip ahead", a leaf beyond —
//! and the turning stays on the page, undecided, in year 1; it chooses Maren; the page
//! tells her taking the dream up, and the year turns. **The stirred variant**: a wanderer
//! arrived the year before and Ysolde carrying a burden, Brannoc a fulfilled one and a
//! blade: the marks fall by the rule — "(not the dream)" only on those who carry a
//! burden still undone, "(lays one aside)" on an heirloom holder who could take it.
//!
//! INVARIANT: every expectation is a shipped literal copied by hand from MODULES.md,
//! SPEC.md or the content — never computed by the code under test.

use jidousha::prelude::*;

use crate::checks::Checks;
use crate::dream::Dream;
use crate::hero::Heirloom;
use crate::house::House;
use crate::ids::DreamKind;
use crate::screen::{Page, Target, UiState, ink};
use crate::scripted::lines_in;
use crate::summer::SHEET;
use crate::telling_view::PANEL;
use crate::verify::{hero_named, page_of, point_at, session};
use crate::w5::recorded;
use crate::w7::stay_home_into_winter;

/// MODULES.md W8: Garrick's heirs, in order, and the button that chooses no one.
pub const ORACLE_HEIRS: [&str; 7] = [
    "Maren, daughter",
    "Pip, grandson",
    "Odo, friend",
    "Ysolde, of the house",
    "Brannoc, of the house",
    "Wren, of the house",
    "No one. Let it lie.",
];
/// The death page's heading and title; under them, the dead's epitaph (W9 — it took the
/// place of session 8's stand-in, the condition line "Died in year 1, aged 94").
pub const ORACLE_PAGE: [&str; 2] = ["The house is one fewer", "In memory of Garrick Thorne"];
pub const ORACLE_PROMPT: &str = "WHO IS HIS HEIR? One choice. It cannot be unmade.";
/// `lines.heir.takes_dream` for Maren, told about the dream's owner.
pub const ORACLE_TAKEN: &str =
    "Maren takes the dream up where Garrick left it: win a triumph at the Barrow.";

/// Each heir button on the page, in order: what it chooses, and what it reads.
pub fn heir_labels(page: &Page) -> Vec<(Target, String)> {
    page.targets
        .iter()
        .filter(|(_, t)| matches!(t, Target::Heir(..)))
        .map(|(rect, t)| {
            let label = lines_in(page, *rect).join(" ");
            (*t, label)
        })
        .collect()
}

/// Year 1 left at home, into the winter, Garrick aged to his death; nothing let pass.
pub fn stage_garricks_winter(sim: &mut HeadlessSim) {
    stay_home_into_winter(sim);
    let garrick = hero_named(sim, "Garrick");
    sim.world_mut().resource_mut::<House>().heroes[garrick].age = 93;
}

/// The stirring: a wanderer arrived, Ysolde carrying an undone burden, Brannoc a done one
/// and a blade; the wanderer carries one undone when `burdened`. Returns the wanderer.
pub fn stir(sim: &mut HeadlessSim, burdened: bool) -> usize {
    let content = match crate::content::load() {
        Ok(content) => content,
        Err(error) => crate::checks::fail("the content did not load", &error.to_string()),
    };
    let (ysolde, brannoc) = (hero_named(sim, "Ysolde"), hero_named(sim, "Brannoc"));
    let mut rng = sim.world().resource::<Rng>().clone();
    let house = sim.world_mut().resource_mut::<House>();
    let _ = crate::wanderer::arrive(&content, house, &mut rng);
    let wanderer = house.heroes.len() - 1;
    let undone = |kind| Dream::build(&content, kind, None, None).ok();
    house.heroes[ysolde].burden = undone(DreamKind::SeeTheSea);
    let mut done = undone(DreamKind::RoofOfTheWorld);
    if let Some(d) = done.as_mut() {
        d.advance_to_stage(3);
    }
    house.heroes[brannoc].burden = done;
    house.heroes[brannoc].heirloom = Some(Heirloom {
        name: "Emberwake".to_owned(),
        sprite: "reward-sword".to_owned(),
        aptitude: crate::ids::Aptitude::Might,
        bonus: 2,
        provenance: "Forged by Brannoc Hale in year 1.".to_owned(),
    });
    if burdened {
        house.heroes[wanderer].burden = undone(DreamKind::KnownAtCourt);
    }
    house.open_hearth();
    sim.world_mut().insert_resource(rng);
    wanderer
}

/// Every leaf of turning page `index`, in order, as the panel reads it (the view put on
/// each in turn, then returned to the leaf it was on).
pub fn page_leaves(sim: &mut HeadlessSim, index: usize) -> Vec<Vec<String>> {
    let at = sim.world().resource::<UiState>().leaf;
    let wanted: Vec<usize> = {
        let house = sim.world().resource::<House>();
        let Some(passage) = &house.passage else {
            return Vec::new();
        };
        let leaves = crate::turning_view::leaves(passage, &house.heroes);
        (0..leaves.len())
            .filter(|&l| leaves[l].page == index)
            .collect()
    };
    let mut out = Vec::new();
    for leaf in wanted {
        let mut ui = *sim.world().resource::<UiState>();
        ui.leaf = leaf;
        crate::verify::set_ui(sim, ui);
        out.push(lines_in(&page_of(sim), PANEL));
    }
    let mut ui = *sim.world().resource::<UiState>();
    ui.leaf = at;
    crate::verify::set_ui(sim, ui);
    out
}

/// After the winter is let pass: "Go on" to the first leaf that offers heirs.
pub fn go_to_the_choice(sim: &mut HeadlessSim) {
    let mut guard = 0;
    while heir_labels(&page_of(sim)).is_empty() && guard < 20 {
        point_at(sim, Target::GoOn, true);
        guard += 1;
    }
}

/// The oracle on every recorded seed, its refusal, and the stirred variant. Returns the
/// summary and the vector.
pub fn check_oracle(checks: &mut Checks) -> (String, Vec<String>) {
    let mut vector = Vec::new();
    for (index, seed) in recorded().into_iter().enumerate() {
        let mut sim = session(seed);
        stage_garricks_winter(&mut sim);
        point_at(&mut sim, Target::LetWinterPass, true);
        go_to_the_choice(&mut sim);
        let page = page_of(&sim);
        let panel = lines_in(&page, PANEL);
        let labels: Vec<String> = heir_labels(&page).into_iter().map(|(_, l)| l).collect();
        let first = page_leaves(&mut sim, 1)
            .into_iter()
            .next()
            .unwrap_or_default();
        let garrick = hero_named(&sim, "Garrick");
        let epitaph = sim.world().resource::<House>().heroes[garrick]
            .epitaph
            .clone();
        checks.require(
            first.len() >= 3
                && first[..2] == ORACLE_PAGE
                && Some(&first[2]) == epitaph.as_ref()
                && panel.contains(&ORACLE_PROMPT.to_owned()),
            "Garrick's death page does not read \"The house is one fewer\", \"In memory of Garrick Thorne\", his epitaph, and the heir prompt",
            format!("seed {seed:#x}: first leaf {first:?}; the choice's {panel:?}"),
        );
        checks.require(
            labels == ORACLE_HEIRS,
            "Garrick's death page does not offer Maren, Pip, Odo, then Ysolde, Brannoc and Wren, then no one",
            format!("seed {seed:#x}: {labels:?}"),
        );
        refusal(checks, &mut sim, seed);
        if seed == recorded()[0] {
            vector.push(format!("W8 vector, Garrick's death page: {first:?}"));
            vector.push(format!("W8 vector, its heirs: {labels:?}"));
        }
        stirred(checks, seed, index % 2 == 0, &mut vector);
    }
    (
        format!(
            "W8 oracle: when Garrick dies of old age his page offers Maren (daughter), Pip (grandson), Odo (friend), then Ysolde, Brannoc and Wren, then no one, unmarked; the year refuses to turn until one is chosen; stirred, the marks fall on burdens only — on {} recorded seeds",
            recorded().len()
        ),
        vector,
    )
}

/// The year refuses to turn while the page waits: "Go on", "Skip ahead" and a leaf
/// beyond all leave it where it is; then Maren is chosen and the year turns.
fn refusal(checks: &mut Checks, sim: &mut HeadlessSim, seed: u64) {
    let leaf = sim.world().resource::<UiState>().leaf;
    let go_on_greyed = page_of(sim)
        .rows
        .iter()
        .any(|r| (r.text == "Go on" || r.text == "Summer comes") && r.style.color == ink::GONE);
    let mut tries = vec![Target::GoOn, Target::Skip];
    let last = page_of(sim)
        .targets
        .iter()
        .filter_map(|(_, t)| match t {
            Target::Leaf(n) => Some(*n),
            _ => None,
        })
        .max();
    if let Some(n) = last.filter(|&n| n > leaf) {
        tries.push(Target::Leaf(n));
    }
    for target in tries {
        point_at(sim, target, true);
        let house = sim.world().resource::<House>();
        let now = sim.world().resource::<UiState>().leaf;
        let waiting = house
            .passage
            .as_ref()
            .is_some_and(|p| p.first_undecided().is_some());
        checks.require(
            waiting
                && now == leaf
                && house.calendar.current_year() == 1
                && house.calendar.is_winter(),
            "the turning let the pointer past a death page with no heir chosen",
            format!("seed {seed:#x}: pressed {target:?}, leaf {leaf} -> {now}, waiting {waiting}"),
        );
    }
    checks.require(
        go_on_greyed,
        "\"Go on\" is not greyed on a death page that waits",
        format!("seed {seed:#x}"),
    );
    // Pointing at an heir opens their sheet in the dock.
    let maren = hero_named(sim, "Maren");
    let target = heir_labels(&page_of(sim))
        .into_iter()
        .map(|(t, _)| t)
        .find(|t| *t == Target::Heir(1, Some(maren)));
    let Some(target) = target else {
        checks.require(
            false,
            "Maren's heir button is not on page 2",
            format!("seed {seed:#x}"),
        );
        return;
    };
    point_at(sim, target, false);
    let dock = lines_in(&page_of(sim), SHEET);
    checks.require(
        dock.first().map(String::as_str) == Some("Maren Thorne"),
        "pointing at an heir does not open their sheet in the dock",
        format!("seed {seed:#x}: {dock:?}"),
    );
    point_at(sim, target, true);
    let now = sim.world().resource::<UiState>().leaf;
    let on_page = {
        let house = sim.world().resource::<House>();
        house.passage.as_ref().is_some_and(|passage| {
            let leaves = crate::turning_view::leaves(passage, &house.heroes);
            leaves.get(now).is_some_and(|l| l.page == 1)
        })
    };
    let panel: Vec<String> = page_leaves(sim, 1).concat();
    let house = sim.world().resource::<House>();
    let held = house.heroes[maren]
        .heirloom
        .as_ref()
        .map(|h| h.name.clone());
    checks.require(
        panel.contains(&ORACLE_TAKEN.to_owned())
            && heir_labels(&page_of(sim)).is_empty()
            && on_page
            && held.as_deref() == Some("Thornfall"),
        "choosing Maren does not stay on the page, tell her taking the dream up and give her Thornfall",
        format!("seed {seed:#x}: {panel:?}, leaf {leaf} -> {now}, holds {held:?}"),
    );
    crate::play::read_to_summer(sim);
    let house = sim.world().resource::<House>();
    checks.require(
        house.passage.is_none()
            && house.calendar.current_year() == 2
            && !house.calendar.is_winter(),
        "with the heir chosen the year does not turn",
        format!("seed {seed:#x}: year {}", house.calendar.current_year()),
    );
}

/// The stirred page: a wanderer, burdens, a blade.
fn stirred(checks: &mut Checks, seed: u64, burdened: bool, vector: &mut Vec<String>) {
    let mut sim = session(seed);
    stage_garricks_winter(&mut sim);
    let wanderer = stir(&mut sim, burdened);
    point_at(&mut sim, Target::LetWinterPass, true);
    go_to_the_choice(&mut sim);
    let labels: Vec<String> = heir_labels(&page_of(&sim))
        .into_iter()
        .map(|(_, l)| l)
        .collect();
    let name = sim.world().resource::<House>().heroes[wanderer]
        .name
        .clone();
    let mark = if burdened { " (not the dream)" } else { "" };
    let want = [
        "Maren, daughter".to_owned(),
        "Pip, grandson".to_owned(),
        "Odo, friend".to_owned(),
        "Ysolde, of the house (not the dream)".to_owned(),
        "Brannoc, of the house (lays one aside)".to_owned(),
        "Wren, of the house".to_owned(),
        format!("{name}, of the house{mark}"),
        "No one. Let it lie.".to_owned(),
    ];
    checks.require(
        labels == want,
        "stirred, Garrick's heirs are not marked by the rule: \"(not the dream)\" on an undone burden only, \"(lays one aside)\" on an heirloom",
        format!("seed {seed:#x}: {labels:?}"),
    );
    if seed == recorded()[0] || seed == recorded()[1] {
        vector.push(format!(
            "W8 vector, stirred (the wanderer burdened: {burdened}): {labels:?}"
        ));
    }
}

/// A turned year with a page of every kind, for the floors and the pictures: Maren and
/// Brannoc wed the year before, Pip eleven, Garrick and Ysolde aged to their deaths and
/// Maren to forty; the winter let pass until a child is born (the run's generator moved
/// on a draw between tries), every heir the first offered. The turning is left open on its
/// first leaf.
pub fn stage_turned_year(sim: &mut HeadlessSim) {
    stay_home_into_winter(sim);
    let [maren, brannoc, pip, garrick, ysolde] =
        ["Maren", "Brannoc", "Pip", "Garrick", "Ysolde"].map(|name| hero_named(sim, name));
    {
        let house = sim.world_mut().resource_mut::<House>();
        crate::bonds::form(
            &mut house.heroes,
            maren,
            brannoc,
            crate::ids::BondKind::Spouse,
            0,
        );
        house.heroes[pip].age = 11;
        house.heroes[garrick].age = 93;
        house.heroes[ysolde].age = 93;
        house.heroes[maren].age = 39;
    }
    let house = sim.world().resource::<House>().clone();
    let rng = sim.world().resource::<Rng>().clone();
    for tries in 0..40 {
        sim.world_mut().insert_resource(house.clone());
        let mut moved = rng.clone();
        for _ in 0..tries {
            let _ = moved.next_u32();
        }
        sim.world_mut().insert_resource(moved);
        point_at(sim, Target::LetWinterPass, true);
        let born = sim
            .world()
            .resource::<House>()
            .passage
            .as_ref()
            .is_some_and(|p| {
                p.pages
                    .iter()
                    .any(|pg| pg.kind == crate::passage::PageKind::Birth)
            });
        if born {
            break;
        }
    }
    // Every heir the first offered, so every page can be turned to.
    let content = match crate::content::load() {
        Ok(content) => content,
        Err(error) => crate::checks::fail("the content did not load", &error.to_string()),
    };
    crate::play::choose_every_heir(
        &content,
        sim.world_mut().resource_mut::<House>(),
        crate::play::first_heir,
    );
    point_at(sim, Target::Leaf(0), true);
}

/// On the open turning, the first leaf of the first page of `kind`.
pub fn turn_to_kind(sim: &mut HeadlessSim, kind: crate::passage::PageKind) {
    let leaf = {
        let house = sim.world().resource::<House>();
        let Some(passage) = &house.passage else {
            crate::checks::fail(
                "no turning is open",
                &format!("looking for a {kind:?} page"),
            );
        };
        let Some(page) = passage.pages.iter().position(|p| p.kind == kind) else {
            crate::checks::fail("the turning has no page of a kind", &format!("{kind:?}"));
        };
        crate::turning_view::first_leaf_of(
            &crate::turning_view::leaves(passage, &house.heroes),
            page,
        )
    };
    let mut ui = *sim.world().resource::<UiState>();
    ui.leaf = leaf;
    crate::verify::set_ui(sim, ui);
}

/// A death page long enough to run over several leaves, at every length from two to
/// nine times Garrick's: "Skip ahead" from the winter's page lands on its last leaf, where
/// the choice is drawn, and every heir button sits inside the page's text, clear of the
/// navigation — whatever the last leaf's lines leave of it.
pub fn check_long_page(checks: &mut Checks) -> String {
    let mut most = 0;
    for times in 2..=9 {
        let mut sim = session(recorded()[0]);
        stage_garricks_winter(&mut sim);
        point_at(&mut sim, Target::LetWinterPass, true);
        {
            let house = sim.world_mut().resource_mut::<House>();
            if let Some(page) = house.passage.as_mut().and_then(|p| p.pages.get_mut(1)) {
                page.lines = (0..times).flat_map(|_| page.lines.clone()).collect();
            }
        }
        point_at(&mut sim, Target::Leaf(0), true);
        point_at(&mut sim, Target::Skip, true);
        let (leaf, last, first) = {
            let house = sim.world().resource::<House>();
            let leaves = house
                .passage
                .as_ref()
                .map(|passage| crate::turning_view::leaves(passage, &house.heroes))
                .unwrap_or_default();
            (
                sim.world().resource::<UiState>().leaf,
                crate::turning_view::last_leaf_of(&leaves, 1),
                crate::turning_view::first_leaf_of(&leaves, 1),
            )
        };
        let page = page_of(&sim);
        let area = crate::telling_view::text_area();
        let buttons: Vec<Rect> = page
            .targets
            .iter()
            .filter(|(_, t)| matches!(t, Target::Heir(..)))
            .map(|(r, _)| *r)
            .collect();
        let inside = buttons.iter().all(|r| area.contains_rect(*r));
        checks.require(
            last > first && leaf == last && buttons.len() == 7 && inside,
            "on a long death page \"Skip ahead\" does not land on the leaf with the choice, or the choice spills out of the page",
            format!("{times} times: leaf {leaf}, the page's leaves {first}..={last}, {} buttons, inside {inside}", buttons.len()),
        );
        most = most.max(last - first + 1);
    }
    format!(
        "W8 long death pages: two to nine times Garrick's, up to {most} leaves; \"Skip ahead\" lands on the last, the choice inside the page"
    )
}
