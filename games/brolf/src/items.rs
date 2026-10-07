//! The four pieces of equipment, what each does, and the one function that says so.
//!
//! Key types: `Item`, `Slot`, `Kit`, `Effects`.
//! Depends on: nothing. Must never be depended on by: nothing outside the game.
//! INVARIANT: `Item::effects` is the only place an item's numbers live; the label a
//! player reads (`Item::describe`) and the sim both read it, so they cannot disagree.

/// A piece of equipment a golfer can carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    /// A ball that shrugs off strikes and carries less far.
    HeavyBall,
    /// A ball that carries further and is knocked further.
    LightBall,
    /// A helmet: shorter dazes, never robbed, slower on foot.
    Helmet,
    /// A club that reaches further and strikes harder, and swings slowly.
    BigClub,
}

/// Which of the two slots an item fills.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    /// The ball slot.
    Ball,
    /// The body slot.
    Body,
}

/// What a golfer holds: one ball mod, one body mod, and which was taken last.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Kit {
    /// The ball slot.
    pub ball: Option<Item>,
    /// The body slot.
    pub body: Option<Item>,
    /// The slot filled most recently: the one a club robs.
    pub last_taken: Option<Slot>,
}

/// Multipliers an item (or a kit) applies; `NEUTRAL` is the golfer with nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Effects {
    /// Times the maximum shot length.
    pub shot_reach: f32,
    /// Times the distance a strike on this golfer's ball travels.
    pub strike_taken: f32,
    /// Times the distance this golfer's strikes send a ball.
    pub strike_given: f32,
    /// Times the daze this golfer suffers.
    pub daze_taken: f32,
    /// Times walking speed.
    pub walk: f32,
    /// Times the swing's reach.
    pub swing_reach: f32,
    /// Times the wait between swings.
    pub swing_cooldown: f32,
    /// Whether a club robs this golfer of an item.
    pub drops_when_clubbed: bool,
}

impl Effects {
    /// A golfer with no equipment.
    pub const NEUTRAL: Effects = Effects {
        shot_reach: 1.0,
        strike_taken: 1.0,
        strike_given: 1.0,
        daze_taken: 1.0,
        walk: 1.0,
        swing_reach: 1.0,
        swing_cooldown: 1.0,
        drops_when_clubbed: true,
    };

    /// The multipliers of two golfers' worth of equipment combined.
    fn then(self, other: Effects) -> Effects {
        Effects {
            shot_reach: self.shot_reach * other.shot_reach,
            strike_taken: self.strike_taken * other.strike_taken,
            strike_given: self.strike_given * other.strike_given,
            daze_taken: self.daze_taken * other.daze_taken,
            walk: self.walk * other.walk,
            swing_reach: self.swing_reach * other.swing_reach,
            swing_cooldown: self.swing_cooldown * other.swing_cooldown,
            drops_when_clubbed: self.drops_when_clubbed && other.drops_when_clubbed,
        }
    }
}

impl Item {
    /// Every item, in the order the pickup ring cycles through them.
    pub const ALL: [Item; 4] = [
        Item::HeavyBall,
        Item::LightBall,
        Item::Helmet,
        Item::BigClub,
    ];

    /// What this item does: the one equipment-effect function.
    pub fn effects(self) -> Effects {
        let n = Effects::NEUTRAL;
        match self {
            Item::HeavyBall => Effects {
                strike_taken: 0.5,
                shot_reach: 0.75,
                ..n
            },
            Item::LightBall => Effects {
                shot_reach: 1.25,
                strike_taken: 1.5,
                ..n
            },
            Item::Helmet => Effects {
                daze_taken: 0.5,
                walk: 0.85,
                drops_when_clubbed: false,
                ..n
            },
            Item::BigClub => Effects {
                swing_reach: 1.5,
                strike_given: 1.5,
                swing_cooldown: 1.5,
                ..n
            },
        }
    }

    /// The slot this item fills.
    pub fn slot(self) -> Slot {
        match self {
            Item::HeavyBall | Item::LightBall => Slot::Ball,
            Item::Helmet | Item::BigClub => Slot::Body,
        }
    }

    /// The name a player reads.
    pub fn name(self) -> &'static str {
        match self {
            Item::HeavyBall => "heavy ball",
            Item::LightBall => "light ball",
            Item::Helmet => "helmet",
            Item::BigClub => "big club",
        }
    }

    /// The three-letter tag drawn beside the item on the course.
    pub fn tag(self) -> &'static str {
        match self {
            Item::HeavyBall => "HVY",
            Item::LightBall => "LGT",
            Item::Helmet => "HLM",
            Item::BigClub => "BIG",
        }
    }

    /// What this item does, formatted from `effects()` and nothing else.
    pub fn describe(self) -> String {
        let e = self.effects();
        let n = Effects::NEUTRAL;
        let mut parts: Vec<String> = Vec::new();
        let mut factor = |value: f32, neutral: f32, label: &str| {
            if value != neutral {
                parts.push(format!("{label} x{value}"));
            }
        };
        factor(e.strike_taken, n.strike_taken, "strikes on it");
        factor(e.shot_reach, n.shot_reach, "your reach");
        factor(e.strike_given, n.strike_given, "your strikes");
        factor(e.daze_taken, n.daze_taken, "dazes on you");
        factor(e.walk, n.walk, "your walk");
        factor(e.swing_reach, n.swing_reach, "your swing reach");
        factor(e.swing_cooldown, n.swing_cooldown, "your swing wait");
        if e.drops_when_clubbed != n.drops_when_clubbed {
            parts.push("never robbed by a club".to_owned());
        }
        parts.join(", ")
    }
}

impl Kit {
    /// The component-wise product of the two slots over `Effects::NEUTRAL`.
    pub fn effects(&self) -> Effects {
        let mut total = Effects::NEUTRAL;
        for item in [self.ball, self.body].into_iter().flatten() {
            total = total.then(item.effects());
        }
        total
    }

    /// Put `item` in its slot and return what it displaced.
    pub fn take(&mut self, item: Item) -> Option<Item> {
        let slot = item.slot();
        let displaced = match slot {
            Slot::Ball => self.ball.replace(item),
            Slot::Body => self.body.replace(item),
        };
        self.last_taken = Some(slot);
        displaced
    }

    /// The item held in `slot`.
    pub fn in_slot(&self, slot: Slot) -> Option<Item> {
        match slot {
            Slot::Ball => self.ball,
            Slot::Body => self.body,
        }
    }

    /// The item taken most recently, if it is still held.
    pub fn most_recent(&self) -> Option<Item> {
        self.last_taken.and_then(|slot| self.in_slot(slot))
    }

    /// Remove and return the most recent item; the other slot becomes the recent one.
    pub fn remove_most_recent(&mut self) -> Option<Item> {
        let slot = self.last_taken?;
        let removed = match slot {
            Slot::Ball => self.ball.take(),
            Slot::Body => self.body.take(),
        };
        self.last_taken = if self.ball.is_some() {
            Some(Slot::Ball)
        } else if self.body.is_some() {
            Some(Slot::Body)
        } else {
            None
        };
        removed
    }

    /// How many items are held.
    pub fn count(&self) -> usize {
        usize::from(self.ball.is_some()) + usize::from(self.body.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_kits_effects_are_the_product_of_its_two_slots() {
        let kit = Kit {
            ball: Some(Item::LightBall),
            body: Some(Item::Helmet),
            last_taken: Some(Slot::Body),
        };
        let e = kit.effects();
        assert_eq!(e.shot_reach, 1.25);
        assert_eq!(e.strike_taken, 1.5);
        assert_eq!(e.daze_taken, 0.5);
        assert_eq!(e.walk, 0.85);
        assert!(!e.drops_when_clubbed);
        assert_eq!(Kit::default().effects(), Effects::NEUTRAL);
    }

    #[test]
    fn an_items_label_names_every_effect_it_has_and_none_it_lacks() {
        let labels = [
            "strikes on it",
            "your reach",
            "your strikes",
            "dazes on you",
            "your walk",
            "your swing reach",
            "your swing wait",
            "never robbed",
        ];
        let wanted: [(Item, &[&str]); 4] = [
            (Item::HeavyBall, &["strikes on it", "your reach"]),
            (Item::LightBall, &["strikes on it", "your reach"]),
            (Item::Helmet, &["dazes on you", "your walk", "never robbed"]),
            (
                Item::BigClub,
                &["your strikes", "your swing reach", "your swing wait"],
            ),
        ];
        for (item, has) in wanted {
            let text = item.describe();
            for label in labels {
                assert_eq!(
                    text.contains(label),
                    has.contains(&label),
                    "{item:?} described as {text:?}, label {label:?}"
                );
            }
        }
    }

    #[test]
    fn taking_an_item_returns_the_one_it_displaced() {
        let mut kit = Kit::default();
        assert_eq!(kit.take(Item::HeavyBall), None);
        assert_eq!(kit.take(Item::LightBall), Some(Item::HeavyBall));
        assert_eq!(kit.take(Item::Helmet), None);
        assert_eq!(kit.most_recent(), Some(Item::Helmet));
        assert_eq!(kit.remove_most_recent(), Some(Item::Helmet));
        assert_eq!(kit.most_recent(), Some(Item::LightBall));
    }
}
