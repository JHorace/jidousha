//! The beings who call, and everything there is to know about them.
//!
//! Content only: three beings, three facts each, two questions per fact, and
//! three answers per question — the lore answer the fact names, a harmless
//! wrong one, and an insult the fact warns against. The numbers that make a
//! being dangerous (drain, line, temper) live beside its words so a being is
//! one row. Drawn on Lovecraft's public-domain mythos; written for this game.
//!
//! Every string here is drawn by the built-in font, so every one is ASCII;
//! `verify` holds the whole table to that.

/// Who is on the line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Being {
    Cthulhu,
    Nyarlathotep,
    YogSothoth,
}

impl Being {
    /// Every being, in table order.
    pub(crate) const ALL: [Being; 3] = [Being::Cthulhu, Being::Nyarlathotep, Being::YogSothoth];

    /// This being's row of `BEINGS`.
    pub(crate) fn index(self) -> usize {
        match self {
            Being::Cthulhu => 0,
            Being::Nyarlathotep => 1,
            Being::YogSothoth => 2,
        }
    }

    /// This being's content and numbers.
    pub(crate) fn spec(self) -> &'static BeingSpec {
        &BEINGS[self.index()]
    }
}

/// What kind of answer an answer is — what the fact says about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// The answer the lore names: the being is satisfied sooner.
    Lore,
    /// Plausible and harmless: the call drags on.
    Wrong,
    /// What the lore warns against: the being is angered.
    Insult,
}

/// One answer the player may give.
pub(crate) struct Answer {
    pub(crate) text: &'static str,
    pub(crate) kind: Kind,
}

/// One question a being asks, with its three answers in the order the table
/// writes them (lore, wrong, insult) — `rules::shown_order` decides the order
/// they are shown in.
pub(crate) struct Question {
    pub(crate) ask: &'static str,
    pub(crate) answers: [Answer; 3],
}

/// One fact a morning's study teaches, and the two questions it answers.
pub(crate) struct Fact {
    /// What the fact is about, in two or three words — the morning screen's name for it.
    pub(crate) topic: &'static str,
    /// What the player learns, as the call screen prints it.
    pub(crate) lore: &'static str,
    pub(crate) questions: [Question; 2],
}

/// One being: who it is, how dangerous a call with it is, and its lore.
pub(crate) struct BeingSpec {
    pub(crate) name: &'static str,
    pub(crate) title: &'static str,
    /// Sanity one exchange costs before composure and anger.
    pub(crate) drain: i32,
    /// How many exchanges it needs to say its piece, at the start of a call.
    pub(crate) line: i32,
    /// The anger at which it is wrath.
    pub(crate) temper: i32,
    /// The cult a morning can bargain with to keep it silent.
    pub(crate) cult: &'static str,
    pub(crate) facts: [Fact; 3],
}

const fn answer(text: &'static str, kind: Kind) -> Answer {
    Answer { text, kind }
}

const fn question(
    ask: &'static str,
    lore: &'static str,
    wrong: &'static str,
    insult: &'static str,
) -> Question {
    Question {
        ask,
        answers: [
            answer(lore, Kind::Lore),
            answer(wrong, Kind::Wrong),
            answer(insult, Kind::Insult),
        ],
    }
}

/// The three beings.
pub(crate) const BEINGS: [BeingSpec; 3] = [
    BeingSpec {
        name: "Cthulhu",
        title: "the Dreamer in R'lyeh",
        drain: 2,
        line: 5,
        temper: 3,
        cult: "the Esoteric Order of Dagon",
        facts: [
            Fact {
                topic: "his sleep",
                lore: "He is not dead. In his house at R'lyeh he lies dreaming. Call it sleep or dreaming - never death.",
                questions: [
                    question(
                        "Do you know what I am, little caller?",
                        "The one who lies dreaming in R'lyeh.",
                        "A voice on a very bad line.",
                        "A dead god in a drowned tomb.",
                    ),
                    question(
                        "Why have I not come for you yet?",
                        "Because you are still dreaming.",
                        "Because the night is young.",
                        "Because you are dead and cannot.",
                    ),
                ],
            },
            Fact {
                topic: "the stars",
                lore: "He wakes only when the stars are right. His faithful watch the sky for that night and never doubt it.",
                questions: [
                    question(
                        "When shall I rise from the deep?",
                        "When the stars are right.",
                        "When the tide turns at dawn.",
                        "Never. The stars forgot you.",
                    ),
                    question(
                        "Do you watch the sky for me?",
                        "I watch for the stars to come right.",
                        "I watch the weather, mostly.",
                        "There is nothing up there for you.",
                    ),
                ],
            },
            Fact {
                topic: "sunken R'lyeh",
                lore: "R'lyeh sank beneath the Pacific. Its angles are wrong: speak of them with dread, never with a ruler.",
                questions: [
                    question(
                        "Have you seen the angles of my house?",
                        "They are wrong, and they frighten me.",
                        "Only in a sailor's drawings.",
                        "I could measure them with a ruler.",
                    ),
                    question(
                        "Where does my city wait?",
                        "Beneath the Pacific, sunk and waiting.",
                        "High in a valley of the Andes.",
                        "Nowhere. It was a sailor's lie.",
                    ),
                ],
            },
        ],
    },
    BeingSpec {
        name: "Nyarlathotep",
        title: "the Crawling Chaos",
        drain: 2,
        line: 4,
        temper: 2,
        cult: "the Church of Starry Wisdom",
        facts: [
            Fact {
                topic: "his masks",
                lore: "He wears a thousand masks. Greet whichever mask he wears tonight; never ask to see his true face.",
                questions: [
                    question(
                        "Which of my faces do you hear tonight?",
                        "The mask you chose to wear for me.",
                        "The face of a stranger.",
                        "Show me your true face.",
                    ),
                    question(
                        "Shall I come to you as a man?",
                        "As whatever mask pleases you.",
                        "Come by daylight, please.",
                        "Take that mask off first.",
                    ),
                ],
            },
            Fact {
                topic: "his errand",
                lore: "He is the soul and messenger of the Outer Gods. Honour the message; never call him their servant.",
                questions: [
                    question(
                        "Why do I bring you this call?",
                        "You carry the word of the Outer Gods.",
                        "You must be lonely out there.",
                        "Your masters sent their servant.",
                    ),
                    question(
                        "What am I to the gods beyond?",
                        "Their soul and their messenger.",
                        "Their oldest friend.",
                        "Their errand boy.",
                    ),
                ],
            },
            Fact {
                topic: "the Black Pharaoh",
                lore: "In Egypt he walked as the Black Pharaoh and men bowed. He loves a show: admire his wonders, never his tricks.",
                questions: [
                    question(
                        "Did you enjoy my little show in Egypt?",
                        "Your wonders made the people bow.",
                        "I have never been to Egypt.",
                        "It was a conjuror's trick.",
                    ),
                    question(
                        "What shall I show you next?",
                        "Any wonder you wish to bring.",
                        "Nothing, it is very late.",
                        "Spare me your tricks.",
                    ),
                ],
            },
        ],
    },
    BeingSpec {
        name: "Yog-Sothoth",
        title: "the Gate and the Key",
        drain: 3,
        line: 4,
        temper: 3,
        cult: "the Silver Twilight lodge",
        facts: [
            Fact {
                topic: "the gate",
                lore: "He is the gate, the key and the guardian of the gate. Name all three; never call him a lock.",
                questions: [
                    question(
                        "What am I, who speaks through the wire?",
                        "The gate, the key and the guardian.",
                        "A voice from very far away.",
                        "A lock someone forgot to bolt.",
                    ),
                    question(
                        "Through what do you hope to pass?",
                        "Through you, for you are the gate.",
                        "The front door, in the morning.",
                        "Nothing of yours. You are a lock.",
                    ),
                ],
            },
            Fact {
                topic: "all times",
                lore: "Every time is one to him. Never speak of his past or his future; speak of all times at once.",
                questions: [
                    question(
                        "When did this call begin?",
                        "It always was, and always will be.",
                        "A few minutes ago.",
                        "Long ago, in your past.",
                    ),
                    question(
                        "Will we speak again?",
                        "We speak in every time at once.",
                        "I hope not, sir.",
                        "Not in your future.",
                    ),
                ],
            },
            Fact {
                topic: "the spheres",
                lore: "He shows himself as a mass of iridescent globes. Name the spheres with wonder; never call them bubbles.",
                questions: [
                    question(
                        "What do you see when you close your eyes?",
                        "Iridescent spheres, endlessly shining.",
                        "Only the dark.",
                        "Soap bubbles.",
                    ),
                    question(
                        "How shall I appear to you?",
                        "As the shining globes you are.",
                        "As a man in grey.",
                        "As a froth of bubbles.",
                    ),
                ],
            },
        ],
    },
];

/// One question, named by who asks it, which fact it is about, and which of
/// that fact's two it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct QuestionId {
    pub(crate) being: Being,
    pub(crate) fact: usize,
    pub(crate) which: usize,
}

impl QuestionId {
    /// The question's content.
    pub(crate) fn question(self) -> &'static Question {
        &self.being.spec().facts[self.fact].questions[self.which]
    }

    /// The fact it is about.
    pub(crate) fn fact(self) -> &'static Fact {
        &self.being.spec().facts[self.fact]
    }
}

/// Every question a being can ask, in table order.
pub(crate) fn questions_of(being: Being) -> Vec<QuestionId> {
    (0..3)
        .flat_map(|fact| (0..2).map(move |which| QuestionId { being, fact, which }))
        .collect()
}

/// Every string in the table, for the check that the font can draw them all.
pub(crate) fn all_strings() -> Vec<&'static str> {
    let mut all = Vec::new();
    for spec in &BEINGS {
        all.extend([spec.name, spec.title, spec.cult]);
        for fact in &spec.facts {
            all.extend([fact.topic, fact.lore]);
            for question in &fact.questions {
                all.push(question.ask);
                all.extend(question.answers.iter().map(|answer| answer.text));
            }
        }
    }
    all
}
