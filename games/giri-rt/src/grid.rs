//! The world grid: one authored tile map, and the sim reads it (DESIGN §3).
//!
//! **One grid, two readers.** The `Grid` resource built from [`MAP`] is the
//! only terrain there is: the simulation consults it for passability and
//! movement cost, and the renderer draws the same data (terrain kind → tile
//! colour). There is no decorative copy and no render-side duplicate, so the
//! map cannot lie about terrain — `verify.rs` asserts every drawn tile against
//! this grid.
//!
//! **The map is an ASCII literal** — agent-writable, diffable, and the
//! recommended authoring form. One character per tile, one row per line;
//! `Grid::parse` refuses a ragged or unknown map loudly rather than guessing.
//!
//! **S1's mechanical surface of the grid is exactly passability and movement
//! cost** (owner decision, DESIGN §3): nothing else reads terrain. No fog, no
//! encounters, no territory.

use jidousha::prelude::*;

use crate::constants::Tuning;
use crate::theme;

/// One tile's edge, in world units. The map's world rect is
/// `width * TILE` by `height * TILE`, with tile (0,0)'s corner at the origin.
pub const TILE: f32 = 16.0;

/// A tile coordinate: `x` right, `y` down, matching world space.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tile {
    /// Column, from the map's left edge.
    pub x: i32,
    /// Row, from the map's top edge.
    pub y: i32,
}

impl Tile {
    /// A coordinate.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// The centre of this tile, in world units.
    pub fn center(self) -> Vec2 {
        Vec2::new((self.x as f32 + 0.5) * TILE, (self.y as f32 + 0.5) * TILE)
    }

    /// The world rectangle this tile covers.
    pub fn rect(self) -> Rect {
        Rect::from_min_size(
            Vec2::new(self.x as f32 * TILE, self.y as f32 * TILE),
            Vec2::splat(TILE),
        )
    }
}

/// The terrain kinds — a small, closed, data-defined set (DESIGN §3).
///
/// Each kind carries a passable flag and a movement cost in world-minutes
/// (a named drawer constant). Water and peak are the impassable pair; they are
/// two kinds because they are two different pictures on the map, and one fact
/// in the sim: [`Terrain::cost`] answers `None` for both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terrain {
    /// The fast lane: the routes the world was built around.
    Road,
    /// Open ground.
    Plains,
    /// Slow going under trees.
    Forest,
    /// Broken ground: passable, and barely worth it.
    Rough,
    /// Impassable water.
    Water,
    /// Impassable mountains.
    Peak,
}

impl Terrain {
    /// Every kind, in declaration order.
    pub const ALL: &'static [Terrain] = &[
        Terrain::Road,
        Terrain::Plains,
        Terrain::Forest,
        Terrain::Rough,
        Terrain::Water,
        Terrain::Peak,
    ];

    /// The map literal's character for this kind.
    pub const fn glyph(self) -> char {
        match self {
            Terrain::Road => '=',
            Terrain::Plains => '.',
            Terrain::Forest => 'f',
            Terrain::Rough => 'r',
            Terrain::Water => '~',
            Terrain::Peak => '^',
        }
    }

    /// The kind a map character names, if it names one.
    pub fn from_glyph(glyph: char) -> Option<Terrain> {
        Terrain::ALL
            .iter()
            .copied()
            .find(|kind| kind.glyph() == glyph)
    }

    /// Whether a party may stand on this kind at all.
    pub const fn passable(self) -> bool {
        !matches!(self, Terrain::Water | Terrain::Peak)
    }

    /// What entering a tile of this kind costs, in world-minutes — `None` for
    /// the impassable kinds. The whole of what the sim reads off terrain.
    pub fn cost(self, tuning: &Tuning) -> Option<i64> {
        match self {
            Terrain::Road => Some(tuning.road_cost),
            Terrain::Plains => Some(tuning.plains_cost),
            Terrain::Forest => Some(tuning.forest_cost),
            Terrain::Rough => Some(tuning.rough_cost),
            Terrain::Water | Terrain::Peak => None,
        }
    }

    /// What the renderer fills this kind's tiles with — the second reader of
    /// the same data (`theme.rs` owns the colours).
    pub fn color(self) -> Color {
        match self {
            Terrain::Road => theme::ROAD,
            Terrain::Plains => theme::PLAINS,
            Terrain::Forest => theme::FOREST,
            Terrain::Rough => theme::ROUGH,
            Terrain::Water => theme::WATER,
            Terrain::Peak => theme::PEAK,
        }
    }
}

/// The world's one terrain store, held as a resource.
#[derive(Clone, Debug)]
pub struct Grid {
    /// Tiles across.
    pub width: i32,
    /// Tiles down.
    pub height: i32,
    tiles: Vec<Terrain>,
}

impl Resource for Grid {}

impl Grid {
    /// Read a map literal, refusing anything malformed by name.
    pub fn parse(text: &str) -> Result<Grid, String> {
        let rows: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();
        let Some(first) = rows.first() else {
            return Err("the map has no rows".to_owned());
        };
        let width = first.chars().count();
        let mut tiles = Vec::with_capacity(width * rows.len());
        for (y, row) in rows.iter().enumerate() {
            if row.chars().count() != width {
                return Err(format!(
                    "map row {y} is {} tiles wide and row 0 is {width} - the map is rectangular",
                    row.chars().count()
                ));
            }
            for (x, glyph) in row.chars().enumerate() {
                let Some(kind) = Terrain::from_glyph(glyph) else {
                    return Err(format!(
                        "map row {y} column {x} is {glyph:?}, which is not a terrain glyph"
                    ));
                };
                tiles.push(kind);
            }
        }
        Ok(Grid {
            width: i32::try_from(width).map_err(|_| "the map is impossibly wide".to_owned())?,
            height: i32::try_from(rows.len())
                .map_err(|_| "the map is impossibly tall".to_owned())?,
            tiles,
        })
    }

    /// Whether `tile` is on the map at all.
    pub fn contains(&self, tile: Tile) -> bool {
        (0..self.width).contains(&tile.x) && (0..self.height).contains(&tile.y)
    }

    /// The terrain at `tile`, or `None` off the map's edge.
    pub fn find(&self, tile: Tile) -> Option<Terrain> {
        if !self.contains(tile) {
            return None;
        }
        self.tiles
            .get(usize::try_from(tile.y * self.width + tile.x).ok()?)
            .copied()
    }

    /// The terrain at `tile`, which must be on the map.
    ///
    /// # Panics
    ///
    /// Off the map's edge — an authored coordinate naming a tile the map does
    /// not have is an authoring fault, said loudly.
    pub fn get(&self, tile: Tile) -> Terrain {
        match self.find(tile) {
            Some(kind) => kind,
            None => panic!(
                "{}",
                message(
                    "a tile off the map's edge was asked for",
                    &format!(
                        "({}, {}) on a {}x{} map",
                        tile.x, tile.y, self.width, self.height
                    ),
                    "an authored location or path names a coordinate the map does not have",
                    "fix the coordinate, or grow the MAP literal",
                )
            ),
        }
    }

    /// The row-major index of `tile` — the coordinate order the pathfinder's
    /// documented tie-break is stated in (DESIGN §3).
    pub fn row_major(&self, tile: Tile) -> i64 {
        i64::from(tile.y) * i64::from(self.width) + i64::from(tile.x)
    }

    /// The world rectangle the whole map covers.
    pub fn world_rect(&self) -> Rect {
        Rect::from_min_size(
            Vec2::ZERO,
            Vec2::new(self.width as f32 * TILE, self.height as f32 * TILE),
        )
    }
}

/// The authored world (DESIGN §3): a 48x27 map whose terrain makes routing
/// visible.
///
/// Reading it: `=` road, `.` plains, `f` forest, `r` rough, `~` water,
/// `^` peak. The authored facts the verify scripts lean on:
///
/// - **The road beats the overland shortcut.** Ebisu (7,14) to the Watchtower
///   (36,4) is 39 tiles by the straightest overland line — through the
///   forest — and 47 tiles by road (east along the spine, north up the x=40
///   branch, west along y=4). At the shipped costs the longer road is far
///   cheaper, and the pathfinder takes it.
/// - **A barrier forces a detour.** The peak ridge (x 31..43, y 15..20)
///   stands between Ebisu and the Black Vault (40,21); the road wraps around
///   its east end via x=44.
/// - The Deep Cave (12,7) has no road: five road tiles east and seven
///   forest tiles north is the cheap way in — 59 minutes of mostly slog.
/// - The lake (x 18..24, y 8..12) breaks the forest interior; the rough
///   south-west is passable and barely worth crossing.
pub const MAP: &str = "\
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
^..............................................^
^.......ffffffffffffffffffffffffffffffff.......^
^.......ffffffffffffffffffffffffffffffff.......^
^.......ffffffffffffffffffffffffffff=====......^
^.......ffffffffffffffffffffffffffff....=......^
^.......ffffffffffffffffffffffffffff....=......^
^.......ffffffffffffffffffffffffffff....=......^
^.......ffffffffff~~~~~~~fffffffffff....=......^
^.......ffffffffff~~~~~~~fffffffffff....=......^
^.......ffffffffff~~~~~~~fffffffffff....=......^
^.......ffffffffff~~~~~~~fffffffffff....=......^
^.......ffffffffff~~~~~~~fffffffffff....=......^
^.......ffffffffffffffffffffffffffff....=......^
^......======================================..^
^..............................^^^^^^^^^^^^^=..^
^rrrrrrrrrrrrrrrr..............^^^^^^^^^^^^^=..^
^rrrrrrrrrrrrrrrr..............^^^^^^^^^^^^^=..^
^rrrrrrrrrrrrrrrr..............^^^^^^^^^^^^^=..^
^rrrrrrrrrrrrrrrr..............^^^^^^^^^^^^^=..^
^rrrrrrrrrrrrrrrr..............^^^^^^^^^^^^^=..^
^rrrrrrrrrrrrrrrr.......................=====..^
^rrrrrrrrrrrrrrrr..............................^
^rrrrrrrrrrrrrrrr..............................^
^rrrrrrrrrrrrrrrr..............................^
^rrrrrrrrrrrrrrrr..............................^
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^";

/// Build the one grid.
///
/// # Panics
///
/// On a malformed [`MAP`] — an authoring fault, caught at startup rather than
/// drawn wrong.
pub fn grid() -> Grid {
    match Grid::parse(MAP) {
        Ok(grid) => grid,
        Err(why) => panic!(
            "{}",
            message(
                "the authored map does not parse",
                &why,
                "an edit to grid::MAP broke its shape or used an unknown glyph",
                "fix the MAP literal; the glyphs are = . f r ~ ^",
            )
        ),
    }
}

/// Which marker icon a location draws (the roles come from giri's library).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconRole {
    /// The town — home base.
    Town,
    /// A cave site.
    Cave,
    /// A crypt site.
    Crypt,
    /// A tower site.
    Tower,
    /// A vault site.
    Vault,
}

/// A named tile (DESIGN §3): locations are what give tiles names.
#[derive(Clone, Copy, Debug)]
pub struct LocationSpec {
    /// The id a link or a log line names it by.
    pub id: &'static str,
    /// The display name.
    pub name: &'static str,
    /// Which tile it is.
    pub tile: Tile,
    /// Which marker it draws.
    pub icon: IconRole,
}

/// Index of the town in [`LOCATIONS`] — where parties live and pots land.
pub const TOWN: usize = 0;

/// Every named tile: the town, then the quest sites.
pub const LOCATIONS: &[LocationSpec] = &[
    LocationSpec {
        id: "ebisu",
        name: "Ebisu",
        tile: Tile::new(7, 14),
        icon: IconRole::Town,
    },
    LocationSpec {
        id: "watchtower",
        name: "the Watchtower",
        tile: Tile::new(36, 4),
        icon: IconRole::Tower,
    },
    LocationSpec {
        id: "deep-cave",
        name: "the Deep Cave",
        tile: Tile::new(12, 7),
        icon: IconRole::Cave,
    },
    LocationSpec {
        id: "old-crypt",
        name: "the Old Crypt",
        tile: Tile::new(25, 19),
        icon: IconRole::Crypt,
    },
    LocationSpec {
        id: "black-vault",
        name: "the Black Vault",
        tile: Tile::new(40, 21),
        icon: IconRole::Vault,
    },
];

/// The location standing on `tile`, if one is.
pub fn location_at(tile: Tile) -> Option<usize> {
    LOCATIONS.iter().position(|spec| spec.tile == tile)
}
