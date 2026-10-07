//! The beings and what they know: five facts each, one question to a fact, and the three
//! answers to every question — the true one, a guess, and an insult. Authored for this
//! game; the mythos is Lovecraft's, the words are not.
//!
//! INVARIANT: every string is printable ASCII, so the font draws it as written; a fact
//! and its question and answers are in step (`FACTS[b][f]` is what `QUESTIONS[b][f]` asks).

/// How many beings call.
pub const BEINGS: usize = 3;
/// How many facts each being has, and so how many questions.
pub const FACTS_PER_BEING: usize = 5;

/// One of the three callers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Being {
    /// The Gate and the Key.
    YogSothoth,
    /// The Deep.
    Dagon,
    /// The Crawling Chaos.
    Nyarlathotep,
}

impl Being {
    /// Every being, in table order.
    pub const ALL: [Being; BEINGS] = [Being::YogSothoth, Being::Dagon, Being::Nyarlathotep];

    /// This being's row in every table.
    pub fn index(self) -> usize {
        match self {
            Being::YogSothoth => 0,
            Being::Dagon => 1,
            Being::Nyarlathotep => 2,
        }
    }

    /// The being whose row is `index`, if there is one.
    pub fn find(index: usize) -> Option<Being> {
        Being::ALL.get(index).copied()
    }

    /// What the caller id says.
    pub fn name(self) -> &'static str {
        ["Yog-Sothoth", "Dagon", "Nyarlathotep"][self.index()]
    }

    /// What the cult that serves it calls itself.
    pub fn cult(self) -> &'static str {
        [
            "the Keepers of the Gate",
            "the Order of the Deep Ones",
            "the Church of Starry Wisdom",
        ][self.index()]
    }

    /// How many exchanges its call lasts if every answer is a wash.
    pub fn call_length(self) -> i32 {
        [4, 6, 4][self.index()]
    }

    /// What one exchange costs in sanity before Composure.
    pub fn drain(self) -> i32 {
        [2, 1, 2][self.index()]
    }

    /// The temper at which it is in wrath, and every exchange costs double.
    pub fn wrath_at(self) -> i32 {
        [2, 4, 3][self.index()]
    }

    /// How many exchanges a wrong answer adds to its call.
    pub fn guess_penalty(self) -> i32 {
        [1, 1, 1][self.index()]
    }
}

/// A fact: the thing the player learns, and the question it answers.
pub struct Fact {
    /// What the morning calls it.
    pub title: &'static str,
    /// What the lore panel says once it is known.
    pub lore: &'static str,
    /// What the being asks.
    pub question: &'static str,
    /// The true answer.
    pub right: &'static str,
    /// The plausible wrong one.
    pub guess: &'static str,
    /// The one that angers it.
    pub insult: &'static str,
    /// What it says when it is answered rightly.
    pub pleased: &'static str,
}

/// Each being's five facts, in the order they are studied.
pub const FACTS: [[Fact; FACTS_PER_BEING]; BEINGS] = [
    [
        Fact {
            title: "the Key and the Gate",
            lore: "Yog-Sothoth is the Key and the Gate, and knows where all gates are.",
            question: "Mortal. What am I to the doors of the world?",
            right: "The Key and the Gate.",
            guess: "A very old locksmith.",
            insult: "A door like any other, I suppose.",
            pleased: "Yes. That will do.",
        },
        Fact {
            title: "all time at once",
            lore: "Past and future are one to Yog-Sothoth; it speaks of Tuesday as of the Flood.",
            question: "When, mortal, are you speaking to me?",
            right: "All times at once, for you.",
            guess: "Just now, at night.",
            insult: "In your time, which I am kind enough to share.",
            pleased: "Tidy. Continue.",
        },
        Fact {
            title: "outside space and time",
            lore: "Yog-Sothoth lies coterminous with all time and space, outside them.",
            question: "Where do I stand?",
            right: "Everywhere and outside everywhere.",
            guess: "Somewhere above the clouds.",
            insult: "Nowhere I could not reach with a good telescope.",
            pleased: "Close enough to wisdom.",
        },
        Fact {
            title: "the Whateley twins",
            lore: "Its children by Lavinia Whateley were the Whateley twins; one was a thing no eye could hold.",
            question: "Who were my sons upon the Whateley girl?",
            right: "A pale child, and a thing too large to see.",
            guess: "Two boys who liked books.",
            insult: "Mortal sons. Gods do not have sons.",
            pleased: "Remembered. Rare.",
        },
        Fact {
            title: "when the stars are right",
            lore: "The Old Ones will return when the stars are right, and Yog-Sothoth opens the way.",
            question: "What must be right before the way opens?",
            right: "The stars.",
            guess: "The weather.",
            insult: "Only your permission, I imagine.",
            pleased: "Brief. Good.",
        },
    ],
    [
        Fact {
            title: "the Father below",
            lore: "Dagon is the Father of the Deep Ones and waits under the sea.",
            question: "Who is it that waits, down where the tide forgets?",
            right: "Father Dagon, below the waves.",
            guess: "A very large fish.",
            insult: "Some kind of sea monster.",
            pleased: "Ahh. You do remember.",
        },
        Fact {
            title: "the Deep Ones' kin",
            lore: "Dagon is served by the Deep Ones, who breed with the people of fishing towns.",
            question: "Who are my children in the harbour towns?",
            right: "The Deep Ones, and their kin.",
            guess: "The gulls.",
            insult: "Fishermen who were cursed, poor souls.",
            pleased: "Mm. The kin send greetings.",
        },
        Fact {
            title: "Innsmouth",
            lore: "Dagon is worshipped in Innsmouth, where the Order of Dagon keeps its hall.",
            question: "Where do they sing to me with a hall and a ring?",
            right: "Innsmouth.",
            guess: "Arkham.",
            insult: "Wherever the fishermen are drunk enough.",
            pleased: "The song goes on.",
        },
        Fact {
            title: "the harbour's gold",
            lore: "The old gold of Innsmouth is Deep One work, and each piece is a promise.",
            question: "What did the harbour take, and what did it give back?",
            right: "Fish for gold, and a promise.",
            guess: "Salt for rope.",
            insult: "Nothing worth the trouble.",
            pleased: "A fair trade, is it not?",
        },
        Fact {
            title: "slow answers",
            lore: "Dagon speaks slowly, and a hurried answer is an insult to him.",
            question: "Tell me, little one, how long should one take to answer a god?",
            right: "As long as the tide takes.",
            guess: "A minute is plenty.",
            insult: "Quickly. Gods have all the time to wait.",
            pleased: "Patience. Good.",
        },
    ],
    [
        Fact {
            title: "the messenger",
            lore: "Nyarlathotep is the Crawling Chaos and the messenger of the Outer Gods.",
            question: "What do the others send when they cannot be bothered?",
            right: "Me. The messenger.",
            guess: "A very long letter.",
            insult: "A servant who talks too much.",
            pleased: "How flattering, and how true.",
        },
        Fact {
            title: "a thousand faces",
            lore: "Nyarlathotep wears a thousand forms, and the Black Man is only one of them.",
            question: "Which of my faces is the true one?",
            right: "None. It has a thousand.",
            guess: "The tall dark man.",
            insult: "The pharaoh. It is the vainest.",
            pleased: "A clever mortal. Rare.",
        },
        Fact {
            title: "the Egyptian road",
            lore: "He walked out of Egypt as a pharaoh and was seen to hold a crowd in thrall.",
            question: "Where did I last go among men, in a dark suit?",
            right: "Out of Egypt, holding crowds.",
            guess: "To a garden party.",
            insult: "Nowhere. You only haunt old books.",
            pleased: "Ah, the old tour.",
        },
        Fact {
            title: "sincere flattery",
            lore: "Nyarlathotep loves a lie told sincerely; flattery pleases him and flat truth bores him.",
            question: "Do you think I am handsome?",
            right: "Beyond all measure.",
            guess: "You are quite tall.",
            insult: "Not particularly. But it is a good suit.",
            pleased: "You see me so well.",
        },
        Fact {
            title: "the blind dancers' flute",
            lore: "He is the soul of the Outer Gods, who dance blind and mindless to a flute.",
            question: "What plays while the gods dance?",
            right: "A thin flute, in the dark.",
            guess: "A radio.",
            insult: "Nothing that matters.",
            pleased: "You hear it too.",
        },
    ],
];

/// The being's own question and the answers for fact `fact`.
pub fn fact(being: Being, fact: usize) -> Option<&'static Fact> {
    FACTS[being.index()].get(fact)
}

/// What a being says when a guess leaves the call longer.
pub fn guess_reply(being: Being) -> &'static str {
    [
        "No. Again, and try to be less small.",
        "Mm. The tide did not say that.",
        "Hm. Almost charming. Wrong, though.",
    ][being.index()]
}

/// What a being says when it is insulted.
pub fn insult_reply(being: Being) -> &'static str {
    [
        "You presume. The gate remembers every insult.",
        "The deep is not amused. It is never amused.",
        "Oh, that was clumsy. I take such things personally.",
    ][being.index()]
}

/// What a being says when it is hung up on.
pub fn hangup_reply(being: Being) -> &'static str {
    [
        "...you will answer next time.",
        "The line goes wet. The sea has a long memory.",
        "Click. I will call again, and sooner.",
    ][being.index()]
}

/// What a being says when its call ends of itself.
pub fn goodbye(being: Being) -> &'static str {
    [
        "Enough. The line is yours again.",
        "The tide goes out. Until tomorrow.",
        "Delightful. Do hold the line... no. Goodbye.",
    ][being.index()]
}

/// What a being says when it first speaks.
pub fn greeting(being: Being) -> &'static str {
    [
        "The phone is cold in your hand. A voice like a door opening.",
        "The receiver drips. Something breathes, slowly, below.",
        "The line crackles with laughter. A very pleasant voice says your name.",
    ][being.index()]
}
