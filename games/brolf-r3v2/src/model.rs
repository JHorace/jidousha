//! The numbers the design fixes, and the plain types every system shares:
//! powers, items, a golfer's kit and what it does, and how a match can end.
//!
//! Everything here is data. The functions that *decide* — where a ball stops,
//! who a club hits, what an extraction keeps — are `rules.rs`, and they read
//! these numbers rather than carrying their own (DESIGN.md, "Constants").

use jidousha::prelude::*;

/// The window, and the recorder's viewport.
pub const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);
/// Half the camera's height, in world units.
pub const HALF_H: f32 = 18.0;
/// Half its width, derived from the window's shape.
pub const HALF_W: f32 = HALF_H * WINDOW.aspect();

/// The course: everything that plays happens inside it.
pub const COURSE: Rect = Rect {
    min: Vec2::new(-30.0, -11.0),
    max: Vec2::new(30.0, 16.0),
};
/// The cup, and the zone's centre.
pub const CUP: Vec2 = Vec2::new(0.0, 2.0);
/// How close a ball must stop to the cup to sink.
pub const CUP_RADIUS: f32 = 0.6;
/// The tick the cup opens; before it, every ball rolls over it. A deviation
/// from DESIGN.md (which leaves the cup open from tick 1): see FINDINGS.md G-073.
pub const CUP_OPENS: u64 = 7200;
/// A ball faster than this rolls over the cup.
pub const SINK_SPEED: f32 = 8.0;
/// The extraction gate's centre.
pub const GATE_CENTER: Vec2 = Vec2::new(26.0, -8.0);
/// The extraction gate's size.
pub const GATE_SIZE: Vec2 = Vec2::new(4.0, 4.0);

/// The extraction gate.
pub fn gate() -> Rect {
    Rect::from_center_size(GATE_CENTER, GATE_SIZE)
}

/// A golfer's radius.
pub const GOLFER_RADIUS: f32 = 0.8;
/// A ball's radius.
pub const BALL_RADIUS: f32 = 0.35;
/// Walking speed, units per second.
pub const WALK_SPEED: f32 = 9.0;
/// How close to its ball a golfer must stand to shoot it.
pub const SHOT_REACH: f32 = 1.5;
/// How fast the aim turns while an arrow is held, in degrees per second.
pub const AIM_RATE_DEGREES: f32 = 120.0;
/// The same, as an angle per second.
pub const AIM_RATE: Radians = Radians::from_degrees(AIM_RATE_DEGREES);
/// How far a club reaches, before equipment.
pub const CLUB_REACH: f32 = 2.5;
/// How near a pickup must be for the HUD to read it out.
pub const READ_RANGE: f32 = 4.0;
/// How near a pickup must be to take it.
pub const TAKE_RANGE: f32 = 1.5;

/// How fast a rolling ball slows, units per second per second.
pub const BALL_DECEL: f32 = 16.0;
/// Below this speed a ball has stopped.
pub const REST_SPEED: f32 = 0.05;

/// Ticks a club disables its target for, before equipment.
pub const CLUB_STUN: u32 = 180;
/// The speed a struck ball leaves at.
pub const STRIKE_SPEED: f32 = 20.0;
/// Ticks after a club or strike before the striker may shoot or swing again.
pub const SWING_COOLDOWN: u32 = 60;

/// The zone's keyframes: (tick, radius), linear between rows, 0 after the last.
pub const ZONE_TABLE: [(u64, f32); 9] = [
    (0, 34.0),
    (1800, 34.0),
    (3600, 22.0),
    (5400, 22.0),
    (7200, 12.0),
    (8400, 12.0),
    (9600, 4.0),
    (10800, 4.0),
    (11400, 0.0),
];
/// Ticks a golfer (or its ball) may spend outside the zone before elimination.
pub const GRACE_TICKS: u32 = 300;

/// Ticks a golfer must hold the extract key in the gate with its ball.
pub const EXTRACT_HOLD: u32 = 120;
/// How many items an extraction keeps.
pub const BAG_LIMIT: usize = 2;
/// Points for holing out or being the last one standing, on top of the kit.
pub const WIN_BONUS: u32 = 5;
/// The tick the course closes.
pub const MATCH_END_TICK: u64 = 12_000;
/// The seed the window and every verify run use.
pub const SEED: u64 = 7;

/// How hard a shot is struck.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Power {
    /// 10 units per second.
    Putt,
    /// 18 units per second.
    Chip,
    /// 26 units per second.
    Drive,
}

impl Power {
    /// Every power, weakest first.
    pub const ALL: [Power; 3] = [Power::Putt, Power::Chip, Power::Drive];

    /// The ball's speed off the club, before equipment.
    pub fn speed(self) -> f32 {
        match self {
            Power::Putt => 10.0,
            Power::Chip => 18.0,
            Power::Drive => 26.0,
        }
    }

    /// How the HUD names it.
    pub fn name(self) -> &'static str {
        match self {
            Power::Putt => "PUTT",
            Power::Chip => "CHIP",
            Power::Drive => "DRIVE",
        }
    }

    /// One step up (`+1`) or down (`-1`), stopping at the ends.
    pub fn stepped(self, step: i8) -> Power {
        let at = Power::ALL.iter().position(|&p| p == self).unwrap_or(1) as i32;
        let next = (at + i32::from(step)).clamp(0, 2) as usize;
        Power::ALL[next]
    }
}

/// Which slot an item goes in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    /// The club.
    Club,
    /// The ball.
    Ball,
    /// The head.
    Head,
}

/// A piece of equipment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    /// Shots leave faster.
    Driver,
    /// The club reaches further.
    LongClub,
    /// The ball cannot be struck, and shots leave slower.
    LeadBall,
    /// A club disables for less and knocks nothing loose.
    Helmet,
}

impl Item {
    /// Its name, as the HUD prints it.
    pub fn name(self) -> &'static str {
        match self {
            Item::Driver => "Driver",
            Item::LongClub => "Long Club",
            Item::LeadBall => "Lead Ball",
            Item::Helmet => "Helmet",
        }
    }

    /// The letter on its pickup square.
    pub fn letter(self) -> &'static str {
        match self {
            Item::Driver => "D",
            Item::LongClub => "R",
            Item::LeadBall => "L",
            Item::Helmet => "H",
        }
    }

    /// Its slot.
    pub fn category(self) -> Category {
        match self {
            Item::Driver | Item::LongClub => Category::Club,
            Item::LeadBall => Category::Ball,
            Item::Helmet => Category::Head,
        }
    }

    /// What it scores when kept.
    pub fn value(self) -> u32 {
        match self {
            Item::Driver | Item::LongClub => 3,
            Item::LeadBall | Item::Helmet => 2,
        }
    }
}

/// The six pickups, in placement order.
pub const PICKUPS: [Item; 6] = [
    Item::Driver,
    Item::Helmet,
    Item::LeadBall,
    Item::LongClub,
    Item::Driver,
    Item::Helmet,
];

/// What a golfer holds: one item per slot at most.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Kit {
    /// The club slot.
    pub club: Option<Item>,
    /// The ball slot.
    pub ball: Option<Item>,
    /// The head slot.
    pub head: Option<Item>,
}

impl Kit {
    /// A kit holding only `item`.
    pub fn only(item: Item) -> Kit {
        let mut kit = Kit::default();
        kit.put(item);
        kit
    }

    /// A kit holding each of `items`, later ones replacing earlier in a slot.
    pub fn of(items: &[Item]) -> Kit {
        let mut kit = Kit::default();
        for &item in items {
            kit.put(item);
        }
        kit
    }

    /// What the slot `category` holds.
    pub fn get(&self, category: Category) -> Option<Item> {
        match category {
            Category::Club => self.club,
            Category::Ball => self.ball,
            Category::Head => self.head,
        }
    }

    /// Put `item` in its slot and hand back what it replaced.
    pub fn put(&mut self, item: Item) -> Option<Item> {
        let slot = match item.category() {
            Category::Club => &mut self.club,
            Category::Ball => &mut self.ball,
            Category::Head => &mut self.head,
        };
        slot.replace(item)
    }

    /// Empty a slot and hand back what it held.
    pub fn take(&mut self, category: Category) -> Option<Item> {
        match category {
            Category::Club => self.club.take(),
            Category::Ball => self.ball.take(),
            Category::Head => self.head.take(),
        }
    }

    /// Everything held, in slot order Club, Ball, Head.
    pub fn items(&self) -> Vec<Item> {
        [self.club, self.ball, self.head]
            .into_iter()
            .flatten()
            .collect()
    }
}

/// What a kit does to the golfer holding it — the one reading of equipment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effects {
    /// Multiplies every shot's speed.
    pub shot_scale: f32,
    /// Multiplies the club's reach.
    pub reach_scale: f32,
    /// Multiplies how long a club disables this golfer.
    pub stun_scale: f32,
    /// This golfer's ball cannot be struck.
    pub strike_immune: bool,
    /// A club knocks this golfer's best item loose.
    pub drops_when_clubbed: bool,
}

impl Effects {
    /// Nothing held.
    pub const BARE: Effects = Effects {
        shot_scale: 1.0,
        reach_scale: 1.0,
        stun_scale: 1.0,
        strike_immune: false,
        drops_when_clubbed: true,
    };
}

/// How a match ended, for You.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndKind {
    /// Your ball sank.
    Holed,
    /// Every rival was eliminated first.
    LastStanding,
    /// You left through the gate.
    Extracted,
    /// The zone took you, or the course closed.
    Eliminated,
    /// A rival's ball sank first.
    Lost {
        /// Which golfer holed out.
        by: u8,
    },
}

/// The result: how it ended, what was kept, what it scored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// How it ended.
    pub kind: EndKind,
    /// The items kept.
    pub kept: Vec<Item>,
    /// The points.
    pub points: u32,
}

/// The four golfers' names, by index.
pub const NAMES: [&str; 4] = ["YOU", "ROOK", "PIKE", "WREN"];

/// How an NPC plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Temperament {
    /// The keyboard.
    Player,
    /// Clubs whoever comes near.
    Hunter,
    /// Extracts once it holds two items.
    Banker,
    /// Plays its own ball and nothing else.
    Putter,
}

/// Index, start, temperament and colour of each golfer.
pub const GOLFERS: [(Vec2, Temperament, Color); 4] = [
    (
        Vec2::new(-22.0, 11.0),
        Temperament::Player,
        Color::rgb(0.35, 0.80, 1.0),
    ),
    (
        Vec2::new(22.0, 11.0),
        Temperament::Hunter,
        Color::rgb(1.0, 0.50, 0.20),
    ),
    (
        Vec2::new(18.0, -7.0),
        Temperament::Banker,
        Color::rgb(0.90, 0.90, 0.30),
    ),
    (
        Vec2::new(-22.0, -7.0),
        Temperament::Putter,
        Color::rgb(0.80, 0.40, 0.90),
    ),
];

/// Where a golfer's ball starts, relative to the golfer.
pub const BALL_OFFSET: Vec2 = Vec2::new(1.0, 0.0);

/// The colours.
pub mod palette {
    use jidousha::prelude::Color;
    /// What the frame clears to.
    pub const CLEAR: Color = Color::rgb(0.05, 0.07, 0.06);
    /// The course.
    pub const COURSE_GREEN: Color = Color::rgb(0.16, 0.38, 0.18);
    /// The zone now.
    pub const ZONE_RING: Color = Color::rgb(0.95, 0.95, 0.70);
    /// The zone when you get there.
    pub const NEXT_RING: Color = Color::rgba(0.95, 0.95, 0.70, 0.45);
    /// A landing point that will be safe.
    pub const SAFE_DOT: Color = Color::rgb(0.3, 1.0, 0.4);
    /// A landing point that will not be, and a golfer outside.
    pub const DANGER: Color = Color::rgb(0.95, 0.25, 0.20);
    /// A pickup square.
    pub const PICKUP: Color = Color::rgb(0.85, 0.75, 0.30);
    /// HUD text.
    pub const HUD_TEXT: Color = Color::WHITE;
    /// Behind the result banner.
    pub const BANNER_BACK: Color = Color::rgb(0.02, 0.02, 0.03);
}
