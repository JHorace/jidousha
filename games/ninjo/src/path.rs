//! Pathfinding, deterministic by construction (DESIGN §3).
//!
//! One route function: 4-connected Dijkstra over the grid's movement costs,
//! computed once at dispatch and stored — terrain is static in S1, so a route
//! never goes stale.
//!
//! **The documented rule, which the deliberate-tie test asserts** (DESIGN §3;
//! `verify.rs::path_contracts`):
//!
//! 1. The frontier pops the pending tile with the **lowest cost**, ties broken
//!    by the **lowest row-major coordinate** (`y * width + x`).
//! 2. A popped tile expands its neighbours in the order **N, E, S, W**.
//! 3. A tile's recorded route is replaced only by a **strictly cheaper** one —
//!    an equal-cost route arriving later never displaces the one that is
//!    already there.
//!
//! Together those three make the chosen route a pure function of the grid and
//! the endpoints. Nothing here iterates a hash map: the pending set is a
//! plain vector scanned by the documented order, which is O(n) per pop and
//! exactly fast enough for a 48x27 map consulted only at dispatch.

use crate::constants::Tuning;
use crate::grid::{Grid, Tile};

/// A stored route: the tiles entered, in order, ending on the goal — the
/// start tile is where the traveller already stands, so it is not in the
/// list — and the total cost in world-minutes (the sum of the entered tiles'
/// terrain costs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    /// Every tile entered, in travel order.
    pub tiles: Vec<Tile>,
    /// The whole journey's cost, in world-minutes.
    pub cost: i64,
}

/// The neighbour offsets, in the documented expansion order: N, E, S, W.
const STEPS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

/// The cheapest route from `from` to `to`, or `None` when no passable route
/// exists. `from` itself needs no cost — a party stands where it stands.
pub fn route(grid: &Grid, tuning: &Tuning, from: Tile, to: Tile) -> Option<Route> {
    if !grid.contains(from) || !grid.contains(to) {
        return None;
    }
    if grid.get(to).cost(tuning).is_none() && from != to {
        return None;
    }
    if from == to {
        return Some(Route {
            tiles: Vec::new(),
            cost: 0,
        });
    }

    /// One tile's search state.
    #[derive(Clone, Copy)]
    struct Node {
        cost: i64,
        parent: Option<Tile>,
        settled: bool,
        pending: bool,
    }
    let empty = Node {
        cost: 0,
        parent: None,
        settled: false,
        pending: false,
    };
    let size = usize::try_from(grid.width).ok()? * usize::try_from(grid.height).ok()?;
    let mut nodes = vec![empty; size];
    let index = |tile: Tile| usize::try_from(grid.row_major(tile)).ok();

    nodes[index(from)?].pending = true;
    loop {
        // Rule 1: lowest cost, then lowest row-major coordinate. The scan
        // walks the board in row-major order, so keeping the first strict
        // minimum *is* the tie-break.
        let mut best: Option<(usize, i64)> = None;
        for (at, node) in nodes.iter().enumerate() {
            if node.pending && !node.settled && best.is_none_or(|(_, cost)| node.cost < cost) {
                best = Some((at, node.cost));
            }
        }
        let Some((at, cost)) = best else {
            // The frontier ran dry without reaching `to`: unreachable.
            return None;
        };
        nodes[at].settled = true;
        let here = Tile::new(
            i32::try_from(at).ok()? % grid.width,
            i32::try_from(at).ok()? / grid.width,
        );
        if here == to {
            // Walk the parents back into travel order.
            let mut tiles = vec![here];
            let mut walk = here;
            while let Some(parent) = nodes[index(walk)?].parent {
                if parent == from {
                    break;
                }
                tiles.push(parent);
                walk = parent;
            }
            tiles.reverse();
            return Some(Route { tiles, cost });
        }
        // Rule 2: N, E, S, W.
        for (dx, dy) in STEPS {
            let next = Tile::new(here.x + dx, here.y + dy);
            let Some(kind) = grid.find(next) else {
                continue;
            };
            let Some(step) = kind.cost(tuning) else {
                continue;
            };
            let Some(slot) = index(next) else { continue };
            let node = &mut nodes[slot];
            if node.settled {
                continue;
            }
            let through = cost + step;
            // Rule 3: strictly cheaper only.
            if !node.pending || through < node.cost {
                node.cost = through;
                node.parent = Some(here);
                node.pending = true;
            }
        }
    }
}
