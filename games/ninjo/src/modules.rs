//! The module registry and the per-module disable flags (GDD §5, §9).
//!
//! **All modules are disableable from day one** (GDD §1), and the definition
//! of modular is mechanical: `verify` runs the whole suite with each module
//! individually off, and green is the claim (GDD §9's module-off matrix).
//! That matrix is built here and iterated by `verify::module_matrix`.
//!
//! **The table below is empty, and that is the point.** Wave 0b is foundation:
//! the clock, the grid, the people, the stores, the lens. Not one row of GDD
//! §5's registry has been built yet, so [`MODULES`] has no rows and the matrix
//! runs exactly one pass — the everything-on baseline. The machinery is landed
//! *now* so that wave 1.1's autonomy module lands into a harness that already
//! works, instead of arriving with the harness as a second thing to get right
//! in the same session.
//!
//! Adding a module is adding a row to [`MODULES`] and reading
//! [`ModuleSet::enabled`] wherever the module's systems and data are
//! installed. Nothing else changes: the matrix walks the table, the stamp
//! walks the table, and the drawer's report line walks the table.

use jidousha::prelude::*;

/// Which side of the MVP line a module falls (GDD §5's tier column).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// In the MVP.
    Mvp,
    /// After it.
    Post,
}

impl Tier {
    /// Both tiers, in registry order.
    pub const ALL: &'static [Tier] = &[Tier::Mvp, Tier::Post];

    /// The tier's name, as the registry table spells it.
    pub fn name(self) -> &'static str {
        match self {
            Tier::Mvp => "mvp",
            Tier::Post => "post",
        }
    }
}

/// One row of GDD §5's registry.
#[derive(Clone, Copy, Debug)]
pub struct ModuleSpec {
    /// The id the registry, the stamp and a report name it by. ASCII,
    /// lowercase, matching GDD §5's first column exactly.
    pub id: &'static str,
    /// Which side of the MVP line.
    pub tier: Tier,
    /// The wave it lands in, as GDD §8 numbers them.
    pub wave: &'static str,
    /// What it degrades to when it is off — the sentence the module-off
    /// matrix is asserting is true.
    pub degrades_to: &'static str,
}

/// Every module this build has.
///
/// **Empty in wave 0b**: foundation is not a module, and no module has been
/// built. GDD §5's table is the schedule — autonomy, needs, petitions,
/// resolution, settlement and the events-director land in wave 1, asks in
/// wave 2 — and each arrives as one row here.
pub const MODULES: &[ModuleSpec] = &[];

/// Which modules are on.
///
/// A bitmask over [`MODULES`] by index, held as a resource so the matrix can
/// plant one before `Startup` exactly as it plants a `Tuning`. Sixty-four
/// modules is more than GDD §5 will ever hold; the registry is a small closed
/// table by design, and a set that overflowed it would be a design failure
/// long before it was a type failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModuleSet {
    off: u64,
}

impl Resource for ModuleSet {}

impl Default for ModuleSet {
    fn default() -> Self {
        Self::ALL
    }
}

impl ModuleSet {
    /// Everything on — what the game ships with and what a played run uses.
    pub const ALL: Self = Self { off: 0 };

    /// The same set with the module at `index` in [`MODULES`] switched off.
    ///
    /// An index past the table is the identity: the matrix walks the table, so
    /// it cannot produce one, and a set parsed from somewhere else naming a
    /// module this build does not have should mean "this build does not have
    /// it" rather than a panic.
    pub fn without(self, index: usize) -> Self {
        if index >= MODULES.len() || index >= 64 {
            return self;
        }
        Self {
            off: self.off | (1u64 << index),
        }
    }

    /// Whether the module with this id is on.
    ///
    /// An id the registry does not have is **off**: a system asking about a
    /// module that does not exist must not run, and answering "on" would make
    /// a typo into a silently enabled feature.
    pub fn enabled(&self, id: &str) -> bool {
        match MODULES.iter().position(|spec| spec.id == id) {
            Some(index) if index < 64 => self.off & (1u64 << index) == 0,
            _ => false,
        }
    }

    /// Every module that is off, by id, in registry order.
    pub fn disabled(&self) -> Vec<&'static str> {
        MODULES
            .iter()
            .enumerate()
            .filter(|(index, _)| *index < 64 && self.off & (1u64 << index) != 0)
            .map(|(_, spec)| spec.id)
            .collect()
    }

    /// The set as a stamp carries it (GDD §9: stamps carry seed, constants,
    /// variant, module set).
    ///
    /// `modules:none` says the registry is empty, which is a different fact
    /// from `modules:all` and is the one wave 0b's recordings should carry —
    /// a run with no modules built is not the same run as one with all of them
    /// on, and a stamp that said `all` would make those two indistinguishable
    /// the day the first row lands.
    pub fn stamp(&self) -> String {
        if MODULES.is_empty() {
            return "modules:none".to_owned();
        }
        let off = self.disabled();
        if off.is_empty() {
            return "modules:all".to_owned();
        }
        format!("modules:all-except:{}", off.join("+"))
    }
}

/// The module-off matrix, as the list of sets a run is asked for (GDD §9).
///
/// The everything-on baseline first, then one set per module with that module
/// alone switched off. With an empty registry that is one pass — which is the
/// harness running, not the harness skipping: `verify` asserts it produced the
/// baseline and that every set it produced verifies.
pub fn matrix() -> Vec<(String, ModuleSet)> {
    let mut out = vec![("everything on".to_owned(), ModuleSet::ALL)];
    for (index, spec) in MODULES.iter().enumerate() {
        out.push((format!("{} off", spec.id), ModuleSet::ALL.without(index)));
    }
    out
}

/// The registry's own validation: ids are stamp-shaped and unique, and every
/// row says what it degrades to.
pub fn registry(checks: &mut crate::checks::Checks) {
    for tier in Tier::ALL.iter().copied() {
        checks.require(
            !tier.name().is_empty() && tier.name().chars().all(|g| g.is_ascii_lowercase()),
            "a module tier's name is not stamp-shaped ASCII",
            format!("{tier:?} is named {:?}", tier.name()),
        );
    }
    for (index, spec) in MODULES.iter().enumerate() {
        checks.require(
            !spec.id.is_empty()
                && spec
                    .id
                    .chars()
                    .all(|glyph| glyph.is_ascii_lowercase() || glyph == '-'),
            "a module id is not stamp-shaped ASCII",
            format!("MODULES[{index}] is named {:?}", spec.id),
        );
        checks.require(
            MODULES.iter().filter(|other| other.id == spec.id).count() == 1,
            "two modules share an id",
            format!("{:?} appears more than once in MODULES", spec.id),
        );
        checks.require(
            !spec.degrades_to.is_empty(),
            "a module does not say what it degrades to",
            format!(
                "{:?} is in the registry with no degrades-to line; the module-off matrix \
                 asserts that sentence and cannot assert an empty one",
                spec.id
            ),
        );
        checks.require(
            !ModuleSet::ALL.without(index).enabled(spec.id),
            "switching a module off does not switch it off",
            format!(
                "{:?} still reads enabled in a set built by `without({index})`",
                spec.id
            ),
        );
        checks.require(
            ModuleSet::ALL.enabled(spec.id),
            "a module is off in the everything-on set",
            format!("{:?} reads disabled in ModuleSet::ALL", spec.id),
        );
        let _ = spec.tier;
        let _ = spec.wave;
    }
    checks.require(
        matrix().len() == MODULES.len() + 1,
        "the module-off matrix does not have one pass per module plus a baseline",
        format!(
            "the matrix is {} passes over {} modules",
            matrix().len(),
            MODULES.len()
        ),
    );
    checks.require(
        matrix()
            .first()
            .is_some_and(|(_, set)| *set == ModuleSet::ALL),
        "the module-off matrix does not open with the everything-on baseline",
        format!(
            "the first pass is {:?}",
            matrix().first().map(|(name, _)| name)
        ),
    );
}
