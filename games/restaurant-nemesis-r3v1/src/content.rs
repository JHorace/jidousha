//! The words: themes, nemesis titles, diner names, posts. Silly on purpose,
//! written for this game, ASCII only (the built-in font draws nothing else).

/// What an order can go wrong about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    /// Sauces, packets, the little cups.
    Condiment,
    /// Hot things hot, cold things cold.
    Temperature,
    /// How long it takes.
    Wait,
    /// How much is on the plate.
    Portion,
}

/// Every theme, in the order ties are broken in.
pub const THEMES: [Theme; 4] = [
    Theme::Condiment,
    Theme::Temperature,
    Theme::Wait,
    Theme::Portion,
];

impl Theme {
    /// The word on an order row.
    pub fn name(self) -> &'static str {
        match self {
            Theme::Condiment => "CONDIMENT",
            Theme::Temperature => "TEMP",
            Theme::Wait => "WAIT",
            Theme::Portion => "PORTION",
        }
    }

    /// Where this theme sits in `THEMES`.
    pub fn index(self) -> usize {
        match self {
            Theme::Condiment => 0,
            Theme::Temperature => 1,
            Theme::Wait => 2,
            Theme::Portion => 3,
        }
    }

    /// The four titles a nemesis of this theme can take.
    pub fn titles(self) -> [&'static str; 4] {
        match self {
            Theme::Condiment => [
                "Mustard Monster",
                "The Ketchup Count",
                "Relish Wraith",
                "Mayo Marauder",
            ],
            Theme::Temperature => [
                "The Lukewarm Baron",
                "Frostbite Fran",
                "Captain Scald",
                "The Tepid Terror",
            ],
            Theme::Wait => [
                "Clockwatcher Clive",
                "The Impatient Duke",
                "Tapping Tessa",
                "Sir Waits-a-Lot",
            ],
            Theme::Portion => [
                "The Crumb Countess",
                "Big Plate Pete",
                "Lord Morsel",
                "The Ravenous Critic",
            ],
        }
    }

    /// What a nemesis of this theme posts after being failed.
    pub fn complaint(self) -> &'static str {
        match self {
            Theme::Condiment => "ONE packet. ONE. for FRIES.",
            Theme::Temperature => "my soup was room temperature. which room?",
            Theme::Wait => "aged a full year waiting for toast",
            Theme::Portion => "needed a microscope to find the steak",
        }
    }
}

/// First names for ordinary diners.
pub const DINERS: [&str; 16] = [
    "Ada", "Bram", "Cleo", "Dov", "Edie", "Finn", "Gus", "Hana", "Ivo", "Juno", "Kit", "Lux", "Mo",
    "Nell", "Otto", "Pia",
];
