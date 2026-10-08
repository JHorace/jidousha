//! The state machine: mornings, calls, dawns and the end, over a plain struct
//! with no world in it.
//!
//! `step` is the only thing that changes a `Game`. It applies one choice,
//! moves the screen, and appends what the player now sees to the transcript —
//! the strings of the very panel the draw system draws.

use crate::beings::BeingId;
use crate::rules::{
    AnswerKind, DAYS, MAX_EXCHANGES, NightPlan, PATIENCE, START_SANITY, TEMPER_LIMIT,
    answer_outcome, screams, tonight,
};
use jidousha::prelude::Resource;

/// Which screen is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Morning,
    Exchange,
    Dawn,
    End { won: bool },
}

/// One thing to do with a morning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MorningAction {
    /// Learn this being's lowest unknown secret.
    Study(BeingId),
    /// Steady the nerves: every exchange tonight costs one less.
    Meditate,
    /// Attend this being's cult: its temper falls, and it calls tonight.
    Cult(BeingId),
}

/// What the player did on a screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// The numbered option at this index.
    Pick(usize),
    /// Enter, past a Dawn or End screen.
    Continue,
}

/// The night in progress.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Night {
    pub plan: NightPlan,
    /// Which call of the plan is on the line.
    pub call: usize,
    /// How many exchanges this call has had.
    pub exchange: u8,
    /// How much patience the caller has left.
    pub patience: u8,
    /// Every finished call: who, how many exchanges, what it cost.
    pub summary: Vec<(BeingId, u8, u32)>,
    /// Sanity when the current call began.
    pub call_start: i32,
}

/// The whole game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Game {
    pub day: u8,
    pub sanity: i32,
    pub tempers: [u8; 3],
    pub known: [[bool; 3]; 3],
    pub meditated: bool,
    pub cult: Option<BeingId>,
    pub rotation: [BeingId; 3],
    pub screen: Screen,
    pub night: Option<Night>,
    /// What the being did with the last reply, as the next screen says it.
    pub last_line: String,
    /// Every screen shown and every choice made, in order.
    pub transcript: Vec<String>,
}

impl Resource for Game {}

impl Game {
    /// A fresh run on the morning of day one.
    pub fn new(rotation: [BeingId; 3]) -> Self {
        let mut game = Self {
            day: 1,
            sanity: START_SANITY as i32,
            tempers: [0; 3],
            known: [[false; 3]; 3],
            meditated: false,
            cult: None,
            rotation,
            screen: Screen::Morning,
            night: None,
            last_line: String::new(),
            transcript: Vec::new(),
        };
        game.show();
        game
    }

    /// Tonight's plan as the day's state stands now.
    pub fn plan(&self) -> NightPlan {
        tonight(
            self.day,
            self.rotation,
            self.tempers,
            self.meditated,
            self.cult,
        )
    }

    /// The being on the line, during a call.
    pub fn caller(&self) -> Option<BeingId> {
        let night = self.night.as_ref()?;
        night.plan.calls.get(night.call).map(|call| call.being)
    }

    /// Which question the caller is asking, during a call.
    pub fn question_index(&self) -> usize {
        self.night
            .as_ref()
            .map_or(0, |night| usize::from(night.exchange) % 3)
    }

    /// How many secrets of `being` the player knows.
    pub fn secrets_known(&self, being: BeingId) -> usize {
        self.known[being.index()].iter().filter(|k| **k).count()
    }

    /// How many numbered options the current screen offers.
    pub fn option_count(&self) -> usize {
        match self.screen {
            Screen::Morning => morning_options(self).len(),
            Screen::Exchange => call_options(self).len(),
            Screen::Dawn | Screen::End { .. } => 0,
        }
    }

    /// Append the current screen to the transcript, under its marker.
    fn show(&mut self) {
        let marker = match self.screen {
            Screen::Morning => format!("== morning {} ==", self.day),
            Screen::Exchange => {
                let (call, exchange) = self
                    .night
                    .as_ref()
                    .map_or((0, 0), |night| (night.call, night.exchange));
                format!(
                    "== night {} call {} exchange {} ==",
                    self.day,
                    call + 1,
                    exchange + 1
                )
            }
            Screen::Dawn => format!("== dawn {} ==", self.day),
            Screen::End { won: true } => "== end won ==".to_owned(),
            Screen::End { won: false } => "== end lost ==".to_owned(),
        };
        let lines: Vec<String> = crate::screen::screen(self)
            .all_strings()
            .map(str::to_owned)
            .collect();
        self.transcript.push(marker);
        self.transcript.extend(lines);
    }
}

/// The morning's options: study each being with a secret left, in id order;
/// steady the nerves; attend each being's cult, in id order.
pub fn morning_options(game: &Game) -> Vec<MorningAction> {
    let mut options: Vec<MorningAction> = BeingId::ALL
        .into_iter()
        .filter(|being| game.secrets_known(*being) < 3)
        .map(MorningAction::Study)
        .collect();
    options.push(MorningAction::Meditate);
    options.extend(BeingId::ALL.into_iter().map(MorningAction::Cult));
    options
}

/// The option's label on the morning screen.
pub fn morning_label(action: MorningAction) -> String {
    match action {
        MorningAction::Study(being) => format!("Study the lore of {}", being.name()),
        MorningAction::Meditate => "Steady your nerves".to_owned(),
        MorningAction::Cult(being) => format!("Attend the cult of {}", being.name()),
    }
}

/// What a morning action does to tonight, stated before it is chosen. Every
/// number in it is read off `tonight` and `answer_outcome`.
pub fn effect_line(game: &Game, action: MorningAction) -> String {
    let plan = game.plan();
    let calling = |being: BeingId| plan.calls.iter().any(|call| call.being == being);
    match action {
        MorningAction::Study(being) => {
            let k = game.secrets_known(being) + 1;
            if calling(being) {
                let per =
                    answer_outcome(being, game.tempers[being.index()], false, AnswerKind::Wrong)
                        .sanity_cost;
                let lore =
                    answer_outcome(being, game.tempers[being.index()], false, AnswerKind::Lore)
                        .sanity_cost;
                format!(
                    "learn secret {k} of 3: tonight {}'s question {k} costs {lore}, not {per}",
                    being.name()
                )
            } else {
                format!(
                    "learn secret {k} of 3: {} does not call tonight",
                    being.name()
                )
            }
        }
        MorningAction::Meditate => {
            let steady = tonight(game.day, game.rotation, game.tempers, true, None);
            let parts: Vec<String> = plan
                .calls
                .iter()
                .zip(steady.calls.iter())
                .map(|(now, then)| {
                    format!(
                        "{} {}->{}",
                        now.being.name(),
                        now.per_exchange,
                        then.per_exchange
                    )
                })
                .collect();
            format!("tonight each exchange costs 1 less: {}", parts.join(", "))
        }
        MorningAction::Cult(being) => {
            let t = game.tempers[being.index()];
            let temper = if t == 0 {
                format!("{}'s temper stays 0", being.name())
            } else {
                format!("{}'s temper {}->{}", being.name(), t, t - 1)
            };
            let first = plan
                .calls
                .first()
                .map_or("nobody", |call| call.being.name());
            if calling(being) {
                format!("{temper}; {} calls tonight anyway", being.name())
            } else {
                format!(
                    "{temper}; {} calls tonight instead of {first}",
                    being.name()
                )
            }
        }
    }
}

/// The current question's options, in the order they are listed: the lore
/// reply first, only if its secret is known; the other two replies in the
/// question's authored order; hanging up last.
pub fn call_options(game: &Game) -> Vec<(AnswerKind, &'static str)> {
    let Some(being) = game.caller() else {
        return Vec::new();
    };
    let q = game.question_index();
    let question = &being.being().questions[q];
    let mut options = Vec::new();
    if game.known[being.index()][q]
        && let Some(lore) = question
            .replies
            .iter()
            .find(|(kind, _)| *kind == AnswerKind::Lore)
    {
        options.push(*lore);
    }
    options.extend(
        question
            .replies
            .iter()
            .filter(|(kind, _)| *kind != AnswerKind::Lore)
            .copied(),
    );
    options.push((AnswerKind::HangUp, "Hang up."));
    options
}

/// The option's text as listed: numbered, and starred if it is the lore reply.
pub fn option_text(index: usize, kind: AnswerKind, text: &str) -> String {
    let star = if kind == AnswerKind::Lore { "* " } else { "" };
    format!("{}. {star}{text}", index + 1)
}

/// Apply one choice. Returns whether the choice meant anything on this screen;
/// one that did not changes nothing (the input system offers only valid ones).
pub fn step(game: &mut Game, choice: Choice) -> bool {
    match (game.screen, choice) {
        (Screen::Morning, Choice::Pick(index)) => {
            let options = morning_options(game);
            let Some(action) = options.get(index).copied() else {
                return false;
            };
            game.transcript.push(format!("> {}", morning_label(action)));
            apply_morning(game, action);
            let plan = game.plan();
            game.night = Some(Night {
                plan,
                call: 0,
                exchange: 0,
                patience: PATIENCE,
                summary: Vec::new(),
                call_start: game.sanity,
            });
            game.last_line.clear();
            game.screen = Screen::Exchange;
            game.show();
            true
        }
        (Screen::Exchange, Choice::Pick(index)) => {
            let options = call_options(game);
            let Some((kind, text)) = options.get(index).copied() else {
                return false;
            };
            game.transcript.push(format!("> {text}"));
            answer(game, kind);
            game.show();
            true
        }
        (Screen::Dawn, Choice::Continue) => {
            if game.day >= DAYS {
                game.screen = Screen::End { won: true };
            } else {
                game.day += 1;
                game.meditated = false;
                game.cult = None;
                game.night = None;
                game.screen = Screen::Morning;
            }
            game.show();
            true
        }
        _ => false,
    }
}

/// What a morning action does to the day's state.
fn apply_morning(game: &mut Game, action: MorningAction) {
    match action {
        MorningAction::Study(being) => {
            let row = &mut game.known[being.index()];
            if let Some(slot) = row.iter_mut().find(|known| !**known) {
                *slot = true;
            }
        }
        MorningAction::Meditate => game.meditated = true,
        MorningAction::Cult(being) => {
            let temper = &mut game.tempers[being.index()];
            *temper = temper.saturating_sub(1);
            game.cult = Some(being);
        }
    }
}

/// Resolve one reply on the line, and move to whatever follows it.
fn answer(game: &mut Game, kind: AnswerKind) {
    let Some(being) = game.caller() else {
        return;
    };
    let slot = being.index();
    let temper = game.tempers[slot];
    let outcome = answer_outcome(being, temper, game.meditated, kind);
    let scream = screams(temper, outcome);
    game.sanity -= outcome.sanity_cost as i32;
    game.tempers[slot] = temper + outcome.temper_delta;
    let new_temper = game.tempers[slot];
    let name = being.name();
    let cost = outcome.sanity_cost;

    let Some(night) = game.night.as_mut() else {
        return;
    };
    night.exchange += 1;
    let mut ended = true;
    game.last_line = if scream {
        game.sanity -= crate::rules::WRATH as i32;
        format!(
            "{name} SCREAMS. -{cost} sanity and -{} more, temper {new_temper}/{TEMPER_LIMIT}. The line goes dead.",
            crate::rules::WRATH
        )
    } else if outcome.hangs_up {
        format!("You hang up on {name}. -{cost} sanity, temper {new_temper}/{TEMPER_LIMIT}.")
    } else {
        night.patience = night.patience.saturating_sub(outcome.progress);
        let mood = match kind {
            AnswerKind::Lore => "is pleased",
            AnswerKind::Anger => "is offended",
            _ => "lingers on the line",
        };
        if night.patience == 0 {
            format!(
                "{name} {mood}, and satisfied. -{cost} sanity, temper {new_temper}/{TEMPER_LIMIT}. The line goes dead."
            )
        } else if night.exchange >= MAX_EXCHANGES {
            format!(
                "{name} {mood}. -{cost} sanity, temper {new_temper}/{TEMPER_LIMIT}. The line dies at dawn."
            )
        } else {
            ended = false;
            format!("{name} {mood}. -{cost} sanity, temper {new_temper}/{TEMPER_LIMIT}.")
        }
    };

    if game.sanity <= 0 {
        game.screen = Screen::End { won: false };
        return;
    }
    if ended {
        let spent = (night.call_start - game.sanity).max(0) as u32;
        night.summary.push((being, night.exchange, spent));
        night.call += 1;
        night.exchange = 0;
        night.patience = PATIENCE;
        night.call_start = game.sanity;
        if night.call >= night.plan.calls.len() {
            game.screen = Screen::Dawn;
        } else {
            game.screen = Screen::Exchange;
        }
    }
}
