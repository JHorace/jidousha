//! The spec's enumerations, in their canonical orders (`spec/content/README.md`).
//!
//! These are identifiers, not content: each variant's `id()` is the Jai enum name
//! the content files key by, and the index is the order every table is in. The
//! loader checks that every content table lists exactly these ids in exactly this
//! order, so a reordered file fails at startup rather than shifting an index.

/// Declares a fieldless enum with `ALL`, `index()` and `id()`.
macro_rules! ids {
    ($(#[$doc:meta])* $name:ident { $($variant:ident = $id:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $(
                #[doc = $id]
                $variant
            ),+
        }

        impl $name {
            /// Every value, in canonical order.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// The position in canonical order — the index into content tables.
            // Not every enumeration indexes a table yet.
            #[allow(dead_code)]
            pub fn index(self) -> usize {
                self as usize
            }

            /// The id the content files use.
            pub fn id(self) -> &'static str {
                match self {
                    $($name::$variant => $id),+
                }
            }

            /// The value a content id names.
            // Not every enumeration is read out of a content file yet.
            #[allow(dead_code)]
            pub fn find(id: &str) -> Option<$name> {
                Self::ALL.iter().copied().find(|value| value.id() == id)
            }
        }
    };
}

ids!(
    /// The six traits of the variant's rudimentary genetics: a gift and a flaw per
    /// aptitude (`traits` in `lore.json`).
    Trait {
        Strong = "STRONG",
        Frail = "FRAIL",
        Sharp = "SHARP",
        Dull = "DULL",
        Steadfast = "STEADFAST",
        Faint = "FAINT",
    }
);

ids!(
    /// Might, Wits, Spirit.
    Aptitude { Might = "MIGHT", Wits = "WITS", Spirit = "SPIRIT" }
);

ids!(
    /// What a quest carries and what a hero fears.
    Tag {
        Dark = "DARK",
        Undead = "UNDEAD",
        Water = "WATER",
        Beasts = "BEASTS",
        Heights = "HEIGHTS",
        Cold = "COLD",
        Fire = "FIRE",
        Crowds = "CROWDS",
    }
);

ids!(
    /// The seven places; the first six are questing places, in board order.
    Place {
        Barrow = "BARROW",
        DrownedCoast = "DROWNED_COAST",
        HighPass = "HIGH_PASS",
        Emberfall = "EMBERFALL",
        KingsCourt = "KINGS_COURT",
        Deepwood = "DEEPWOOD",
        SealedDoor = "SEALED_DOOR",
    }
);

ids!(
    /// Age phases.
    Phase {
        Child = "CHILD",
        Youth = "YOUTH",
        Prime = "PRIME",
        Veteran = "VETERAN",
        Elder = "ELDER",
    }
);

ids!(
    /// Callings.
    Vocation {
        Knight = "KNIGHT",
        Warrior = "WARRIOR",
        Ranger = "RANGER",
        Scholar = "SCHOLAR",
        Priest = "PRIEST",
        Sage = "SAGE",
    }
);

ids!(
    /// Prophecies.
    Destiny {
        Unspoken = "UNSPOKEN",
        FireWillEndYou = "FIRE_WILL_END_YOU",
        OutliveThoseYouLove = "OUTLIVE_THOSE_YOU_LOVE",
        WearACrown = "WEAR_A_CROWN",
        OpenTheSealedDoor = "OPEN_THE_SEALED_DOOR",
        ChildWillSurpassYou = "CHILD_WILL_SURPASS_YOU",
        BreakAndBeMended = "BREAK_AND_BE_MENDED",
        DieInYourBed = "DIE_IN_YOUR_BED",
        CarryTheHouse = "CARRY_THE_HOUSE",
        TeachAGreater = "TEACH_A_GREATER",
    }
);

ids!(
    /// Bond kinds. "A's bond to B is PARENT" means B is A's parent.
    BondKind {
        Companion = "COMPANION",
        Friend = "FRIEND",
        Rival = "RIVAL",
        Spouse = "SPOUSE",
        Mentor = "MENTOR",
        Student = "STUDENT",
        Parent = "PARENT",
        Child = "CHILD",
    }
);

ids!(
    /// The nine dreams.
    DreamKind {
        SeeTheSea = "SEE_THE_SEA",
        RoofOfTheWorld = "ROOF_OF_THE_WORLD",
        ForgeABlade = "FORGE_A_BLADE",
        WorthyStudent = "WORTHY_STUDENT",
        AvengeTheLost = "AVENGE_THE_LOST",
        KnownAtCourt = "KNOWN_AT_COURT",
        WalkEveryRoad = "WALK_EVERY_ROAD",
        QuietTheBarrow = "QUIET_THE_BARROW",
        SeeAChildGrown = "SEE_A_CHILD_GROWN",
    }
);

ids!(
    /// What a fulfilled dream leaves.
    LegacyKind {
        None = "NONE",
        Heirloom = "HEIRLOOM",
        Tale = "TALE",
        Blessing = "BLESSING",
    }
);

ids!(
    /// An epitaph's nine parts (SPEC §20), in `epitaph.json`'s `parts` order: each
    /// wording's coin is the part's index.
    Part {
        Origin = "ORIGIN",
        Roads = "ROADS",
        Triumph = "TRIUMPH",
        Fear = "FEAR",
        Dream = "DREAM",
        Prophecy = "PROPHECY",
        Love = "LOVE",
        End = "END",
        Left = "LEFT",
    }
);

ids!(
    /// He or she.
    Pronoun { He = "HE", She = "SHE" }
);

ids!(
    /// The twelve writing pools (`writing.json`), in file order.
    Pool {
        Wounds = "WOUNDS",
        Dreads = "DREADS",
        QuestDeaths = "QUEST_DEATHS",
        Lessons = "LESSONS",
        SleepDeaths = "SLEEP_DEATHS",
        Births = "BIRTHS",
        Arrivals = "ARRIVALS",
        Weddings = "WEDDINGS",
        TalesTold = "TALES_TOLD",
        Rests = "RESTS",
        Courages = "COURAGES",
        Friendships = "FRIENDSHIPS",
    }
);

ids!(
    /// What a hero did with a winter: the moment a winter seat offers (SPEC §9.3, §11.3).
    WinterAction {
        Rest = "REST",
        Train = "TRAIN",
        Teach = "TEACH",
        Court = "COURT",
        MindAChild = "MIND_A_CHILD",
        TellTheTale = "TELL_THE_TALE",
    }
);

ids!(
    /// How a quest went. "At least" comparisons use this order.
    Outcome {
        Disaster = "DISASTER",
        Setback = "SETBACK",
        Success = "SUCCESS",
        Triumph = "TRIUMPH",
    }
);
