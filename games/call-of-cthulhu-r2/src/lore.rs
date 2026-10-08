//! The beings, their cults and their lore: everything the game says, as data.
//!
//! Written for this game, on Lovecraft's public-domain mythos. Every string is
//! printable ASCII — the built-in font draws anything else as a box, and no
//! assertion over drawn quads can tell (the verify run checks every string
//! here).

/// Which being is on the line. The index into `BEINGS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Being {
    Dagon,
    Nyarlathotep,
    YogSothoth,
}

impl Being {
    /// Every being, in table order.
    pub const ALL: [Being; 3] = [Being::Dagon, Being::Nyarlathotep, Being::YogSothoth];

    /// This being's row of the table.
    pub fn lore(self) -> &'static BeingLore {
        &BEINGS[self.index()]
    }

    /// Where this being sits in `ALL` and in every per-being array.
    pub fn index(self) -> usize {
        match self {
            Being::Dagon => 0,
            Being::Nyarlathotep => 1,
            Being::YogSothoth => 2,
        }
    }
}

/// How many facts each being has.
pub const FACTS: usize = 4;

/// The three kinds of answer every question offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnswerKind {
    /// Drawn from the being's lore: ends the call sooner.
    Lore,
    /// Merely wrong: the call goes on.
    Wrong,
    /// An insult: angers it.
    Insult,
}

impl AnswerKind {
    /// Every kind, in the order the content table lists a question's answers.
    pub const ALL: [AnswerKind; 3] = [AnswerKind::Lore, AnswerKind::Wrong, AnswerKind::Insult];
}

/// One fact about a being, the question it asks about it, and the answers.
pub struct Fact {
    /// What the morning's study teaches, as a short title.
    pub title: &'static str,
    /// The fact itself, as the lore panel shows it once known.
    pub known: &'static str,
    /// What the being asks about it.
    pub question: &'static str,
    /// The lore answer, the wrong answer, the insult — in `AnswerKind::ALL`
    /// order. The screen shuffles them by the seed.
    pub answers: [&'static str; 3],
}

/// One being's row.
pub struct BeingLore {
    pub name: &'static str,
    pub cult: &'static str,
    /// Sanity per exchange before temper and composure.
    pub base_drain: i32,
    /// How it opens a call.
    pub greeting: &'static str,
    pub facts: [Fact; FACTS],
}

/// The table.
pub const BEINGS: [BeingLore; 3] = [
    BeingLore {
        name: "Father Dagon",
        cult: "the Esoteric Order of Dagon",
        base_drain: 2,
        greeting: "The line is wet. Something breathes through water.",
        facts: [
            Fact {
                title: "the tithe of Innsmouth",
                known: "Innsmouth pays a tithe of gold; he takes it as a promise, not wealth.",
                question: "WHAT DOES THE TOWN OWE THE DEEP?",
                answers: [
                    "A promise, paid in gold.",
                    "Nothing. They are fishermen.",
                    "A good scrubbing, by the smell.",
                ],
            },
            Fact {
                title: "Devil Reef",
                known: "Devil Reef, a mile out, is where his children come up to the air.",
                question: "WHERE DO MY CHILDREN BREATHE?",
                answers: [
                    "On Devil Reef, at low tide.",
                    "In the harbour, by the boats.",
                    "In a fishbowl, I would think.",
                ],
            },
            Fact {
                title: "the patient tide",
                known: "He counts in tides, not days; to him a promise kept late is still kept.",
                question: "HOW LONG IS ONE OF YOUR YEARS?",
                answers: [
                    "Two tides a day, and you count them all.",
                    "Twelve months, the same as yours.",
                    "Too long, if this call is anything to go by.",
                ],
            },
            Fact {
                title: "Mother Hydra",
                known: "Mother Hydra keeps the deep with him; he speaks of her before himself.",
                question: "WHO KEEPS THE DEEP BESIDE ME?",
                answers: [
                    "Mother Hydra.",
                    "The coastguard.",
                    "Nobody, from the sound of you.",
                ],
            },
        ],
    },
    BeingLore {
        name: "Nyarlathotep",
        cult: "the Church of Starry Wisdom",
        base_drain: 3,
        greeting: "A smooth voice, smiling. It knows your name already.",
        facts: [
            Fact {
                title: "the thousand masks",
                known: "He wears a thousand forms; naming one as the true face pleases him.",
                question: "WHICH OF MY FACES IS THE TRUE ONE?",
                answers: [
                    "None, and all. You have a thousand.",
                    "The one in the photograph.",
                    "The ugly one, surely.",
                ],
            },
            Fact {
                title: "the Shining Trapezohedron",
                known: "The Shining Trapezohedron, kept in the dark, is the window he uses.",
                question: "WHAT DO THEY KEEP IN THE STEEPLE, IN THE DARK?",
                answers: [
                    "The Shining Trapezohedron.",
                    "The church bells.",
                    "Your fan mail.",
                ],
            },
            Fact {
                title: "the messenger",
                known: "He is the messenger of the Outer Gods, and proud of the office.",
                question: "AND WHOSE VOICE AM I?",
                answers: [
                    "The messenger of the Outer Gods.",
                    "A salesman's, I think.",
                    "An annoying one.",
                ],
            },
            Fact {
                title: "the Black Pharaoh",
                known: "In Egypt he ruled as the Black Pharaoh, and likes to be remembered for it.",
                question: "WHAT DID THEY CALL ME BY THE NILE?",
                answers: ["The Black Pharaoh.", "A tourist.", "A mistake."],
            },
        ],
    },
    BeingLore {
        name: "Yog-Sothoth",
        cult: "the Whateley household",
        base_drain: 4,
        greeting: "Static, then many voices in one. The gate is open.",
        facts: [
            Fact {
                title: "the gate and the key",
                known: "He is the gate, and the key, and the guardian of the gate.",
                question: "WHAT AM I?",
                answers: [
                    "The gate, the key, and the guardian.",
                    "A long-distance caller.",
                    "A wrong number.",
                ],
            },
            Fact {
                title: "the spheres",
                known: "He appears as a heap of iridescent spheres, ever shifting.",
                question: "WHAT SHAPE DO I WEAR WHEN I AM SEEN?",
                answers: [
                    "A heap of shining spheres.",
                    "A tall man in grey.",
                    "A bad case of hiccups.",
                ],
            },
            Fact {
                title: "Sentinel Hill",
                known: "The Whateleys called him from the stones on Sentinel Hill, at Dunwich.",
                question: "FROM WHICH HILL WAS I CALLED?",
                answers: [
                    "Sentinel Hill, at Dunwich.",
                    "Beacon Hill, in Boston.",
                    "The nearest anthill.",
                ],
            },
            Fact {
                title: "all times at once",
                known: "Past, present and future are one in him; he does not wait.",
                question: "WHEN WILL I COME THROUGH?",
                answers: [
                    "You already have, and always will.",
                    "Next Tuesday.",
                    "When you learn some manners.",
                ],
            },
        ],
    },
];
