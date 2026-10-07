//! The pure rules: no engine types, so a check can run a thousand matches.
//!
//! Three functions are the game's decisions, each the *one* place its question
//! is answered — the panel, the NPC, the checks and the real resolution all
//! call them: `options` (may this card be played, and at what), `legal_targets`
//! (which stack items an effect may touch) and `resolve_top` (what the top item
//! does). `preview` is `resolve_top` run on a copy.

use crate::cards::{Card, Side, deck, info};
use std::fmt;

/// Life each side starts with.
pub const START_LIFE: i32 = 20;
/// Cards in the opening hand.
pub const HAND: usize = 5;
/// A side does not draw while holding this many cards.
pub const HAND_LIMIT: usize = 7;
/// Mana stops growing here.
pub const MANA_CAP: u32 = 6;
/// The match is decided on life after this many turns in all.
pub const TURN_LIMIT: u32 = 24;

/// A stack item's identity, stable while it moves around the stack.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ItemId(pub u32);

/// One played card, on the stack.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Item {
    pub id: ItemId,
    pub card: Card,
    pub owner: Side,
    pub target: Option<ItemId>,
    /// Which player this item's damage or healing lands on, if it has either.
    pub aim: Option<Side>,
}

/// How a match ended.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Won(Side),
    Draw,
}

/// What a side may do while it holds priority.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Play {
        hand_index: usize,
        target: Option<ItemId>,
    },
    Pass,
}

/// A refused action, in the engine's what / cause / fix shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleError {
    pub what: String,
    pub cause: String,
    pub fix: String,
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}) - {}", self.what, self.cause, self.fix)
    }
}

fn refuse(what: &str, cause: &str, fix: &str) -> RuleError {
    RuleError {
        what: what.to_owned(),
        cause: cause.to_owned(),
        fix: fix.to_owned(),
    }
}

/// One resolution, as the preview and the real thing both report it.
#[derive(Clone, Debug)]
pub struct Step {
    pub item: Item,
    pub text: String,
    pub fizzled: bool,
    /// Items this step removed from the stack without resolving them.
    pub countered: Vec<ItemId>,
    pub life_after: [i32; 2],
}

/// What will become of a stack item if nobody responds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fate {
    /// Resolves as the n-th step (1-based).
    Resolves(usize),
    /// Removed by the n-th step.
    Countered(usize),
}

/// The whole match state. Cloneable so the preview can run on a copy.
#[derive(Clone, Debug)]
pub struct Core {
    pub life: [i32; 2],
    pub mana: [u32; 2],
    pub hands: [Vec<Card>; 2],
    /// Draw from the end.
    pub decks: [Vec<Card>; 2],
    /// Last element is the top.
    pub stack: Vec<Item>,
    pub active: Side,
    pub priority: Side,
    pub passes: u8,
    pub turn: u32,
    pub turns_taken: [u32; 2],
    pub next_id: u32,
    pub outcome: Option<Outcome>,
    pub log: Vec<String>,
}

/// Seeded and platform-independent: the same seed shuffles the same way
/// everywhere (splitmix64).
struct Shuffler(u64);

impl Shuffler {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn shuffle(&mut self, cards: &mut [Card]) {
        for i in (1..cards.len()).rev() {
            let j = (self.next() % (i as u64 + 1)) as usize;
            cards.swap(i, j);
        }
    }
}

/// The side a card's damage or healing lands on by default.
pub fn default_aim(card: Card, owner: Side) -> Option<Side> {
    let data = info(card);
    if data.damage > 0 {
        Some(owner.other())
    } else if data.heal > 0 {
        Some(owner)
    } else {
        None
    }
}

/// Whether a card is played at a stack item.
pub fn needs_target(card: Card) -> bool {
    matches!(
        card,
        Card::Negate | Card::Hush | Card::Bury | Card::Raise | Card::Redirect | Card::Echo
    )
}

/// The stack items `card`, played by `owner`, may touch when it acts on `stack`.
///
/// The one legality function: the panel marks these while you choose, `options`
/// refuses anything else, and `resolve_top` asks again at resolution (the stack
/// may have changed) — a target that is no longer here makes the card fizzle.
/// Moves that would change nothing are not offered (no silent no-ops).
pub fn legal_targets(stack: &[Item], card: Card, _owner: Side) -> Vec<ItemId> {
    let ids = |items: &[Item]| items.iter().map(|item| item.id).collect::<Vec<_>>();
    match card {
        Card::Negate => ids(stack),
        Card::Hush => ids(&stack
            .iter()
            .filter(|item| info(item.card).cost <= 2)
            .copied()
            .collect::<Vec<_>>()),
        // Bottom item is already last to resolve.
        Card::Bury if stack.len() >= 2 => ids(&stack[1..]),
        // The top item is already first.
        Card::Raise if stack.len() >= 2 => ids(&stack[..stack.len() - 1]),
        Card::Redirect => ids(&stack
            .iter()
            .filter(|item| item.aim.is_some())
            .copied()
            .collect::<Vec<_>>()),
        Card::Echo => ids(&stack
            .iter()
            .filter(|item| item.card != Card::Echo)
            .copied()
            .collect::<Vec<_>>()),
        _ => Vec::new(),
    }
}

impl Core {
    /// A fresh match: seeded shuffles, opening hands, `You` to move.
    pub fn new(seed: u64) -> Core {
        let mut shuffler = Shuffler(seed);
        let mut decks = [deck(Side::You), deck(Side::Npc)];
        for d in &mut decks {
            shuffler.shuffle(d);
        }
        let mut hands = [Vec::new(), Vec::new()];
        for side in 0..2 {
            for _ in 0..HAND {
                if let Some(card) = decks[side].pop() {
                    hands[side].push(card);
                }
            }
        }
        Core {
            life: [START_LIFE; 2],
            mana: [1, 0],
            hands,
            decks,
            stack: Vec::new(),
            active: Side::You,
            priority: Side::You,
            passes: 0,
            turn: 1,
            turns_taken: [1, 0],
            next_id: 1,
            outcome: None,
            log: vec!["turn 1: YOU".to_owned()],
        }
    }

    fn say(&mut self, line: String) {
        self.log.push(line);
    }

    /// May `side` play hand card `hand_index` right now? Ok carries the legal
    /// targets, or `None` when the card takes no target.
    pub fn options(&self, side: Side, hand_index: usize) -> Result<Option<Vec<ItemId>>, RuleError> {
        if self.outcome.is_some() {
            return Err(refuse(
                "the match is over",
                "a result has been reached",
                "start a new match",
            ));
        }
        if self.priority != side {
            return Err(refuse(
                "not your priority",
                "the other side holds priority",
                "wait for it to act or pass",
            ));
        }
        let Some(&card) = self.hands[side.index()].get(hand_index) else {
            return Err(refuse(
                "no such card",
                "hand index out of range",
                "pick a card that is in hand",
            ));
        };
        let data = info(card);
        if u32::from(data.cost) > self.mana[side.index()] {
            return Err(refuse(
                "not enough mana",
                "the card costs more than the mana you have left",
                "pick a cheaper card; mana refills on your next turn",
            ));
        }
        if data.sorcery && !(self.active == side && self.stack.is_empty()) {
            return Err(refuse(
                "sorcery-speed card",
                "it can only be played by the active player on an empty stack",
                "play it on your own turn once the stack has resolved",
            ));
        }
        if card == Card::Flip && self.stack.len() < 2 {
            return Err(refuse(
                "Flip would change nothing",
                "it reverses the items below it and fewer than two are there",
                "play it over a stack of two or more items",
            ));
        }
        if !needs_target(card) {
            return Ok(None);
        }
        let targets = legal_targets(&self.stack, card, side);
        if targets.is_empty() {
            return Err(refuse(
                "no legal target",
                "nothing on the stack is a legal target for this card",
                "wait until the stack holds a suitable item",
            ));
        }
        Ok(Some(targets))
    }

    /// Play a card onto the stack; the other side gets priority.
    pub fn try_play(
        &mut self,
        side: Side,
        hand_index: usize,
        target: Option<ItemId>,
    ) -> Result<ItemId, RuleError> {
        let legal = self.options(side, hand_index)?;
        match (&legal, target) {
            (Some(list), Some(t)) if list.contains(&t) => {}
            (Some(_), _) => {
                return Err(refuse(
                    "illegal target",
                    "the chosen item is not a legal target for this card",
                    "choose one of the marked items",
                ));
            }
            (None, Some(_)) => {
                return Err(refuse(
                    "this card takes no target",
                    "a target was supplied",
                    "play it without one",
                ));
            }
            (None, None) => {}
        }
        let card = self.hands[side.index()].remove(hand_index);
        self.mana[side.index()] -= u32::from(info(card).cost);
        let id = ItemId(self.next_id);
        self.next_id += 1;
        self.stack.push(Item {
            id,
            card,
            owner: side,
            target,
            aim: default_aim(card, side),
        });
        self.say(format!("{} plays {}", side.name(), info(card).name));
        self.passes = 0;
        self.priority = side.other();
        Ok(id)
    }

    /// Pass priority. Two passes in a row resolve the top item, or end the
    /// turn if the stack is empty.
    pub fn try_pass(&mut self, side: Side) -> Result<(), RuleError> {
        if self.outcome.is_some() {
            return Err(refuse(
                "the match is over",
                "a result has been reached",
                "start a new match",
            ));
        }
        if self.priority != side {
            return Err(refuse(
                "not your priority",
                "the other side holds priority",
                "wait for it to act or pass",
            ));
        }
        self.passes += 1;
        if self.passes < 2 {
            self.priority = side.other();
            return Ok(());
        }
        self.passes = 0;
        if self.stack.is_empty() {
            self.end_turn();
        } else {
            let step = resolve_top(self);
            self.say(step.text);
            self.priority = self.active;
        }
        Ok(())
    }

    /// Apply either action.
    pub fn try_act(&mut self, side: Side, action: Action) -> Result<(), RuleError> {
        match action {
            Action::Pass => self.try_pass(side),
            Action::Play { hand_index, target } => {
                self.try_play(side, hand_index, target).map(|_| ())
            }
        }
    }

    fn end_turn(&mut self) {
        if self.turn >= TURN_LIMIT {
            let [a, b] = self.life;
            self.outcome = Some(match a.cmp(&b) {
                std::cmp::Ordering::Greater => Outcome::Won(Side::You),
                std::cmp::Ordering::Less => Outcome::Won(Side::Npc),
                std::cmp::Ordering::Equal => Outcome::Draw,
            });
            self.say("turn limit reached".to_owned());
            return;
        }
        self.active = self.active.other();
        self.priority = self.active;
        self.turn += 1;
        let who = self.active.index();
        self.turns_taken[who] += 1;
        self.mana[who] = self.turns_taken[who].min(MANA_CAP);
        if self.hands[who].len() < HAND_LIMIT {
            if let Some(card) = self.decks[who].pop() {
                self.hands[who].push(card);
            }
        }
        let line = format!("turn {}: {}", self.turn, self.active.name());
        self.say(line);
    }
}

fn check_over(core: &mut Core) {
    if core.outcome.is_some() {
        return;
    }
    let you_dead = core.life[0] <= 0;
    let npc_dead = core.life[1] <= 0;
    core.outcome = match (you_dead, npc_dead) {
        (true, true) => Some(Outcome::Draw),
        (true, false) => Some(Outcome::Won(Side::Npc)),
        (false, true) => Some(Outcome::Won(Side::You)),
        (false, false) => None,
    };
}

/// Resolve the top item of the stack. The one place an item does anything.
///
/// Panics if the stack is empty: callers resolve only when there is an item.
pub fn resolve_top(core: &mut Core) -> Step {
    let Some(item) = core.stack.pop() else {
        panic!("resolve_top called on an empty stack; callers must check first");
    };
    let data = info(item.card);
    let name = data.name;
    let who = item.owner.name();
    let mut countered = Vec::new();
    let mut fizzled = false;
    let text;
    let target_ok = |core: &Core| {
        item.target
            .filter(|t| legal_targets(&core.stack, item.card, item.owner).contains(t))
    };
    match item.card {
        Card::Ember | Card::Cleaver | Card::Siege => {
            let aim = item.aim.unwrap_or(item.owner.other());
            core.life[aim.index()] -= data.damage;
            text = format!(
                "{who} {name}: {} damage to {} (now {})",
                data.damage,
                aim.name(),
                core.life[aim.index()]
            );
        }
        Card::Mend => {
            let aim = item.aim.unwrap_or(item.owner);
            core.life[aim.index()] += data.heal;
            text = format!(
                "{who} {name}: {} gains {} (now {})",
                aim.name(),
                data.heal,
                core.life[aim.index()]
            );
        }
        Card::Negate | Card::Hush => match target_ok(core) {
            Some(t) => {
                core.stack.retain(|other| other.id != t);
                countered.push(t);
                text = format!("{who} {name}: countered an item");
            }
            None => {
                fizzled = true;
                text = format!("{who} {name}: fizzles, its target is gone");
            }
        },
        Card::Flip => {
            if core.stack.len() >= 2 {
                core.stack.reverse();
                text = format!("{who} {name}: the stack below is reversed");
            } else {
                fizzled = true;
                text = format!("{who} {name}: fizzles, nothing to reverse");
            }
        }
        Card::Bury => match target_ok(core) {
            Some(t) => {
                if let Some(pos) = core.stack.iter().position(|other| other.id == t) {
                    let moved = core.stack.remove(pos);
                    core.stack.insert(0, moved);
                }
                text = format!("{who} {name}: an item goes to the bottom");
            }
            None => {
                fizzled = true;
                text = format!("{who} {name}: fizzles, its target is gone");
            }
        },
        Card::Raise => match target_ok(core) {
            Some(t) => {
                if let Some(pos) = core.stack.iter().position(|other| other.id == t) {
                    let moved = core.stack.remove(pos);
                    core.stack.push(moved);
                }
                text = format!("{who} {name}: an item rises to the top");
            }
            None => {
                fizzled = true;
                text = format!("{who} {name}: fizzles, its target is gone");
            }
        },
        Card::Redirect => match target_ok(core) {
            Some(t) => {
                if let Some(other) = core.stack.iter_mut().find(|other| other.id == t) {
                    other.aim = other.aim.map(Side::other);
                }
                text = format!("{who} {name}: an item's aim is flipped");
            }
            None => {
                fizzled = true;
                text = format!("{who} {name}: fizzles, its target is gone");
            }
        },
        Card::Echo => match target_ok(core) {
            Some(t) => {
                let original = core.stack.iter().find(|other| other.id == t).copied();
                if let Some(original) = original {
                    let id = ItemId(core.next_id);
                    core.next_id += 1;
                    core.stack.push(Item {
                        id,
                        card: original.card,
                        owner: item.owner,
                        target: original.target,
                        aim: default_aim(original.card, item.owner),
                    });
                }
                text = format!("{who} {name}: an item is copied onto the top");
            }
            None => {
                fizzled = true;
                text = format!("{who} {name}: fizzles, its target is gone");
            }
        },
    }
    check_over(core);
    Step {
        item,
        text,
        fizzled,
        countered,
        life_after: core.life,
    }
}

/// What the stack will do, in order, if nobody responds: `resolve_top` run on
/// a copy until the stack is empty or the match is decided.
pub fn preview(core: &Core) -> Vec<Step> {
    let mut copy = core.clone();
    let mut steps = Vec::new();
    while !copy.stack.is_empty() && copy.outcome.is_none() {
        steps.push(resolve_top(&mut copy));
    }
    steps
}

/// What the preview says becomes of `id`.
pub fn fate(steps: &[Step], id: ItemId) -> Option<Fate> {
    for (index, step) in steps.iter().enumerate() {
        if step.item.id == id {
            return Some(Fate::Resolves(index + 1));
        }
        if step.countered.contains(&id) {
            return Some(Fate::Countered(index + 1));
        }
    }
    None
}

/// Every action `side` may take right now, `Pass` last.
pub fn legal_actions(core: &Core, side: Side) -> Vec<Action> {
    let mut actions = Vec::new();
    if core.priority == side && core.outcome.is_none() {
        for hand_index in 0..core.hands[side.index()].len() {
            match core.options(side, hand_index) {
                Ok(None) => actions.push(Action::Play {
                    hand_index,
                    target: None,
                }),
                Ok(Some(targets)) => {
                    for target in targets {
                        actions.push(Action::Play {
                            hand_index,
                            target: Some(target),
                        });
                    }
                }
                Err(_) => {}
            }
        }
        actions.push(Action::Pass);
    }
    actions
}
