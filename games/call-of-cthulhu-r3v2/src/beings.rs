//! The three callers, as data: names, tempers of speech, and the three
//! questions each one asks, with the fact that answers each and the three
//! replies the player may give.
//!
//! Lore written for this game, drawing on the public-domain mythos. Every
//! string is printable ASCII: the built-in face draws a box for anything else.

use crate::rules::AnswerKind;

/// Which being is on the line. The index is the being's row in `BEINGS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BeingId {
    Cthulhu,
    Nyarlathotep,
    Hastur,
}

impl BeingId {
    /// Every being, in id order.
    pub const ALL: [BeingId; 3] = [BeingId::Cthulhu, BeingId::Nyarlathotep, BeingId::Hastur];

    /// This being's row in `BEINGS` and in every per-being array.
    pub fn index(self) -> usize {
        match self {
            BeingId::Cthulhu => 0,
            BeingId::Nyarlathotep => 1,
            BeingId::Hastur => 2,
        }
    }

    /// The being's data.
    pub fn being(self) -> &'static Being {
        &BEINGS[self.index()]
    }

    /// The being's name.
    pub fn name(self) -> &'static str {
        self.being().name
    }
}

/// One question a being asks, the fact that answers it, and the three replies
/// in the order they are listed on screen.
pub struct Question {
    pub ask: &'static str,
    pub fact: &'static str,
    pub replies: [(AnswerKind, &'static str); 3],
}

/// One caller.
pub struct Being {
    pub id: BeingId,
    pub name: &'static str,
    pub epithet: &'static str,
    pub opening: &'static str,
    /// Sanity an ordinary exchange costs, before temper and nerves.
    pub drain: u32,
    pub questions: [Question; 3],
}

use AnswerKind::{Anger as A, Lore as L, Wrong as W};

/// The callers, in id order.
pub const BEINGS: [Being; 3] = [
    Being {
        id: BeingId::Cthulhu,
        name: "Cthulhu",
        epithet: "the Sleeper of R'lyeh",
        opening: "The line is full of water. Something very large breathes at the other end.",
        drain: 6,
        questions: [
            Question {
                ask: "WHERE... DO I LIE, LITTLE VOICE?",
                fact: "It lies in R'lyeh, sunk beneath the Pacific, until the stars come right.",
                replies: [
                    (W, "In a cave under the Himalayas, I think."),
                    (L, "In R'lyeh, beneath the sea, until the stars are right."),
                    (A, "Nowhere. You are a story the sailors tell."),
                ],
            },
            Question {
                ask: "WHAT DO THE DREAMERS SEE WHEN I CALL THEM?",
                fact: "It speaks to artists in dreams: a city of wrong angles, a head of tentacles, wings.",
                replies: [
                    (
                        L,
                        "A city of wrong angles, and your face: the tentacles, the wings.",
                    ),
                    (A, "Nothing. Nobody dreams of you any more."),
                    (W, "Numbers. Long columns of numbers."),
                ],
            },
            Question {
                ask: "WHAT DO MY PRIESTS SING?",
                fact: "Its cult chants that in his house at R'lyeh dead Cthulhu waits dreaming.",
                replies: [
                    (W, "A sea shanty. Something about a whale."),
                    (A, "Wake up, old man. You have overslept."),
                    (L, "In his house at R'lyeh dead Cthulhu waits dreaming."),
                ],
            },
        ],
    },
    Being {
        id: BeingId::Nyarlathotep,
        name: "Nyarlathotep",
        epithet: "the Crawling Chaos",
        opening: "A pleasant voice, amused, as if it had been expecting you to pick up.",
        drain: 4,
        questions: [
            Question {
                ask: "Tell me, friend: what face am I wearing tonight?",
                fact: "He wears a thousand masks; on the line he is the tall dark man out of Egypt who shows machines.",
                replies: [
                    (
                        L,
                        "The tall dark man out of Egypt, the one who shows the machines.",
                    ),
                    (W, "A face like my uncle's. Round, kind."),
                    (A, "You have no face. You are nothing but a voice."),
                ],
            },
            Question {
                ask: "Whose errand do you suppose I am running?",
                fact: "He is the messenger of the Outer Gods and the mouth of blind Azathoth; mistake him for Cthulhu and he will not forgive it.",
                replies: [
                    (W, "The devil's, I would guess."),
                    (
                        L,
                        "Azathoth's. The blind idiot god at the centre of everything.",
                    ),
                    (A, "Cthulhu's, like all of you."),
                ],
            },
            Question {
                ask: "I put on a little show in your city tonight. What did they see?",
                fact: "He shows crowds sparks and cold flames, and afterwards nobody in the city can sleep.",
                replies: [
                    (A, "Nothing worth watching. They went home early."),
                    (W, "A juggling act, wasn't it?"),
                    (
                        L,
                        "Sparks and cold fire, and afterwards none of them could sleep.",
                    ),
                ],
            },
        ],
    },
    Being {
        id: BeingId::Hastur,
        name: "Hastur",
        epithet: "the Unspeakable",
        opening: "A whisper under wind. The line crackles every time you breathe.",
        drain: 5,
        questions: [
            Question {
                ask: "...you know who this is. Say who.",
                fact: "It is not to be named. Say \"Hastur\" on the line and it hears you say it.",
                replies: [
                    (W, "Is this the gas company?"),
                    (A, "Hastur. It's Hastur."),
                    (L, "I know. The one who is not to be named."),
                ],
            },
            Question {
                ask: "...have you seen my sign? Describe it.",
                fact: "Its sign is the Yellow Sign, and the King in Tatters wears a pallid mask.",
                replies: [
                    (L, "The Yellow Sign. I saw it and I will not draw it."),
                    (W, "A red triangle, on a hill."),
                    (A, "A sign? Hastur, you don't even have a sign."),
                ],
            },
            Question {
                ask: "...where do the black stars hang?",
                fact: "It dwells by the lake of Hali, under black stars, in lost Carcosa.",
                replies: [
                    (A, "Over your head, Hastur, wherever that is."),
                    (L, "Over the lake of Hali. Over Carcosa."),
                    (W, "Over Arkham, by the river."),
                ],
            },
        ],
    },
];
