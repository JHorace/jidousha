//! The three tiers of memory the panel reports, and how bytes are written.
//!
//! Key types: `Engine`; `linear_bytes`, `megabytes`.
//! Depends on: `jidousha-render-core` (`BackendStats`). Must never depend on:
//! a crate, or anything that can reach a tick.
//! INVARIANT: three tiers, and each one is only ever compared with itself. The
//! process tier (`process.rs`, native) is what the operating system charges
//! this program; the wasm tier below is the module's own linear memory; the
//! engine tier is what the renderer and the world are holding. They do not add
//! up and are never presented as if they did — the renderer's textures live on
//! a GPU, which no resident set size can see (frame-pacing.md §7).
//! INVARIANT: the engine tier is **counted at the seam, never sampled**. Every
//! number in it is a running total something already maintains, so the tier
//! that catches unbounded growth costs a frame nothing to read.

use jidousha_render_core::BackendStats;

/// How many bytes a wasm memory page is.
///
/// 64KiB, fixed by the WebAssembly specification — the one number in this file
/// that is a fact about a format rather than about a machine.
#[cfg(target_arch = "wasm32")]
const PAGE_BYTES: u64 = 65_536;

/// The wasm module's own linear memory, in bytes.
///
/// **Not `performance.memory`**, which is Chrome-only, reports the whole tab
/// rather than this module, and is quantised for fingerprinting reasons. The
/// page count is what the module itself can see and it is exact: linear memory
/// only ever grows, so this is the high-water mark of everything the engine and
/// its allocator have ever asked for.
#[cfg(target_arch = "wasm32")]
pub(crate) fn linear_bytes() -> Option<u64> {
    Some(core::arch::wasm32::memory_size(0) as u64 * PAGE_BYTES)
}

/// Native builds have no linear memory; the process tier is the reading there.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn linear_bytes() -> Option<u64> {
    None
}

/// What the engine itself is holding, counted rather than sampled.
///
/// The **actionable** tier: a resident set size that climbs is a fact with no
/// address in it, and these counters have addresses. A texture total that grows
/// is art nobody unloaded; an entity count that grows is a spawner with no
/// reaper; a quad count that grows is a Draw system accumulating rather than
/// describing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Engine {
    /// What the render backend reports it is holding.
    pub(crate) backend: BackendStats,
    /// Live entities in the world, read at draw time.
    pub(crate) entities: usize,
    /// Component values across every store, read at draw time.
    pub(crate) components: usize,
    /// Quads the game submitted this frame — the overlay's own excluded, since
    /// they are appended to a copy the world never sees.
    pub(crate) quads: usize,
}

/// A byte count as the panel writes it.
///
/// Mebibytes with one decimal, always, rather than a unit that changes with the
/// magnitude: a panel a person is watching for *growth* must not switch from KB
/// to MB half way up, because the number would appear to fall.
pub(crate) fn megabytes(bytes: u64) -> String {
    format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_are_written_in_mebibytes_with_one_decimal() {
        assert_eq!(megabytes(0), "0.0MB");
        assert_eq!(megabytes(1024 * 1024), "1.0MB");
        assert_eq!(megabytes(1024 * 1024 * 3 / 2), "1.5MB");
    }

    #[test]
    fn a_small_allocation_stays_in_the_same_unit_rather_than_becoming_kilobytes() {
        // Why the unit never changes: this panel is watched for growth, and a
        // reading that switched units would make a rising number look like a
        // falling one at exactly the threshold somebody was watching.
        assert_eq!(megabytes(4096), "0.0MB");
        assert_eq!(megabytes(1024 * 1024 / 10), "0.1MB");
    }

    #[test]
    fn a_gigabyte_is_written_out_rather_than_abbreviated() {
        // Same reason: 1024.0MB and 1.0GB on consecutive samples would read as
        // a thousand-fold drop.
        assert_eq!(megabytes(1024 * 1024 * 1024), "1024.0MB");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_native_build_has_no_linear_memory_to_report() {
        // And says so, rather than reporting the process tier twice under two
        // names — the two are different questions and only one of them has an
        // answer on each target.
        assert_eq!(linear_bytes(), None);
    }
}
