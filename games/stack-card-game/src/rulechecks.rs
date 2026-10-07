//! The pure-rules half of `--verify`: card numbers, legality, each stack
//! effect, the turn machine, and sweeps over whole seeded matches. Every
//! expectation is a shipped literal, never arithmetic over the constant it
//! tests (make-game A.6).

use crate::cards::{Card, Side, deck, info};
use crate::checks::Checks;
use crate::npc;
use crate::players;
use crate::rules::*;

fn item(id: u32, card: Card, owner: Side, target: Option<u32>) -> Item {
    Item {
        id: ItemId(id),
        card,
        owner,
        target: target.map(ItemId),
        aim: default_aim(card, owner),
    }
}

/// A match at a fixed seed with the stack replaced by `items` (bottom first).
fn with_stack(items: &[Item]) -> Core {
    let mut core = Core::new(1);
    core.stack = items.to_vec();
    core.next_id = 100;
    core
}

/// Resolve everything and return the step texts.
fn resolve_all(core: &mut Core) -> Vec<String> {
    let mut texts = Vec::new();
    while !core.stack.is_empty() && core.outcome.is_none() {
        texts.push(resolve_top(core).text);
    }
    texts
}

fn card_table(checks: &mut Checks) {
    // (card, cost, sorcery, damage, heal): shipped literals.
    let table: [(Card, u8, bool, i32, i32); 11] = [
        (Card::Ember, 1, false, 2, 0),
        (Card::Cleaver, 3, true, 6, 0),
        (Card::Siege, 5, true, 10, 0),
        (Card::Mend, 2, false, 0, 4),
        (Card::Negate, 2, false, 0, 0),
        (Card::Hush, 1, false, 0, 0),
        (Card::Flip, 1, false, 0, 0),
        (Card::Bury, 2, false, 0, 0),
        (Card::Raise, 1, false, 0, 0),
        (Card::Redirect, 2, false, 0, 0),
        (Card::Echo, 2, false, 0, 0),
    ];
    for (card, cost, sorcery, damage, heal) in table {
        let data = info(card);
        checks.require(
            data.cost == cost && data.sorcery == sorcery && data.damage == damage && data.heal == heal,
            "a card's numbers are not the shipped ones",
            format!(
                "{}: cost {} sorcery {} damage {} heal {}; expected {cost} {sorcery} {damage} {heal}",
                data.name, data.cost, data.sorcery, data.damage, data.heal
            ),
        );
    }
    let count = |side: Side, card: Card| deck(side).iter().filter(|&&c| c == card).count();
    checks.require(
        deck(Side::You).len() == 20 && deck(Side::Npc).len() == 20,
        "a deck is not 20 cards",
        format!("{} and {}", deck(Side::You).len(), deck(Side::Npc).len()),
    );
    checks.require(
        count(Side::You, Card::Siege) == 0
            && count(Side::Npc, Card::Siege) == 2
            && count(Side::You, Card::Flip) == 2
            && count(Side::Npc, Card::Flip) == 1,
        "the decks are not asymmetric as designed",
        "the player's deck has no Siege and two Flip; the NPC's has two Siege and one Flip"
            .to_owned(),
    );
}

fn legality(checks: &mut Checks) {
    let stack = [
        item(1, Card::Cleaver, Side::Npc, None),
        item(2, Card::Mend, Side::Npc, None),
        item(3, Card::Echo, Side::Npc, Some(1)),
    ];
    let ids = |card: Card| -> Vec<u32> {
        legal_targets(&stack, card, Side::You)
            .iter()
            .map(|id| id.0)
            .collect()
    };
    let cases: [(Card, Vec<u32>); 6] = [
        (Card::Negate, vec![1, 2, 3]),
        (Card::Hush, vec![2, 3]),
        (Card::Bury, vec![2, 3]),
        (Card::Raise, vec![1, 2]),
        (Card::Redirect, vec![1, 2]),
        (Card::Echo, vec![1, 2]),
    ];
    for (card, want) in cases {
        let got = ids(card);
        checks.require(
            got == want,
            "legal targets are not the designed ones",
            format!("{}: {got:?}, expected {want:?}", info(card).name),
        );
    }
    let mut core = Core::new(1);
    core.hands[0] = vec![Card::Cleaver, Card::Ember, Card::Flip, Card::Negate];
    core.mana = [6, 6];
    let refusal = |core: &Core, index: usize| core.options(Side::You, index).err().map(|e| e.what);
    // Empty stack: Flip and Negate have nothing to do, Cleaver and Ember are fine.
    checks.require(
        refusal(&core, 0).is_none()
            && refusal(&core, 1).is_none()
            && refusal(&core, 2).as_deref() == Some("Flip would change nothing")
            && refusal(&core, 3).as_deref() == Some("no legal target"),
        "an empty stack offers the wrong plays",
        format!(
            "refusals: {:?} {:?} {:?} {:?}",
            refusal(&core, 0),
            refusal(&core, 1),
            refusal(&core, 2),
            refusal(&core, 3)
        ),
    );
    // A sorcery is refused over a non-empty stack, and when you are not active.
    core.stack = vec![item(1, Card::Ember, Side::Npc, None)];
    checks.require(
        refusal(&core, 0).as_deref() == Some("sorcery-speed card"),
        "a sorcery was allowed over a non-empty stack",
        format!("{:?}", refusal(&core, 0)),
    );
    core.stack.clear();
    core.active = Side::Npc;
    checks.require(
        refusal(&core, 0).as_deref() == Some("sorcery-speed card") && refusal(&core, 1).is_none(),
        "a sorcery was allowed on the opponent's turn, or an instant was refused",
        format!("{:?} {:?}", refusal(&core, 0), refusal(&core, 1)),
    );
    core.mana[0] = 2;
    core.active = Side::You;
    checks.require(
        refusal(&core, 0).as_deref() == Some("not enough mana") && refusal(&core, 1).is_none(),
        "mana was not enforced",
        format!("{:?} {:?}", refusal(&core, 0), refusal(&core, 1)),
    );
    let bad = core.try_play(Side::You, 1, Some(ItemId(9)));
    checks.require(
        bad.is_err_and(|e| e.what == "this card takes no target"),
        "a target on a targetless card was accepted",
        "Ember with a target".to_owned(),
    );
}

fn effects(checks: &mut Checks) {
    // Flip: [Ember(NPC) @1, Mend(You) @2, Cleaver(NPC) @3, Flip(You) @4]
    // Flip reverses what is below it: top-down order Cleaver, Mend, Ember
    // becomes Ember, Mend, Cleaver.
    let mut core = with_stack(&[
        item(1, Card::Ember, Side::Npc, None),
        item(2, Card::Mend, Side::You, None),
        item(3, Card::Cleaver, Side::Npc, None),
        item(4, Card::Flip, Side::You, None),
    ]);
    let texts = resolve_all(&mut core);
    let want = [
        "YOU Flip: the stack below is reversed",
        "NPC Ember: 2 damage to YOU (now 18)",
        "YOU Mend: YOU gains 4 (now 22)",
        "NPC Cleaver: 6 damage to YOU (now 16)",
    ];
    checks.require(
        texts == want,
        "Flip did not reverse the order of the stack below it",
        format!("{texts:?}"),
    );

    // Redirect: their Siege lands on them.
    let mut core = with_stack(&[
        item(1, Card::Siege, Side::Npc, None),
        item(2, Card::Redirect, Side::You, Some(1)),
    ]);
    let texts = resolve_all(&mut core);
    checks.require(
        core.life == [20, 10] && texts[1] == "NPC Siege: 10 damage to NPC (now 10)",
        "Redirect did not turn a Siege back on its caster",
        format!("life {:?}, {texts:?}", core.life),
    );

    // Echo: their Cleaver, copied, aimed back at them, resolving first.
    let mut core = with_stack(&[
        item(1, Card::Cleaver, Side::Npc, None),
        item(2, Card::Echo, Side::You, Some(1)),
    ]);
    let texts = resolve_all(&mut core);
    checks.require(
        core.life == [14, 14]
            && texts[1] == "YOU Cleaver: 6 damage to NPC (now 14)"
            && texts[2] == "NPC Cleaver: 6 damage to YOU (now 14)",
        "Echo did not put a copy on top, owned and aimed by its caster",
        format!("life {:?}, {texts:?}", core.life),
    );

    // Negate, then a second Negate whose target is gone fizzles.
    let mut core = with_stack(&[
        item(1, Card::Cleaver, Side::Npc, None),
        item(2, Card::Negate, Side::You, Some(1)),
        item(3, Card::Negate, Side::You, Some(1)),
    ]);
    let mut steps = Vec::new();
    while !core.stack.is_empty() {
        steps.push(resolve_top(&mut core));
    }
    checks.require(
        core.life == [20, 20]
            && !steps[0].fizzled
            && steps[0].countered == [ItemId(1)]
            && steps[1].fizzled
            && steps[1].text == "YOU Negate: fizzles, its target is gone",
        "a Negate whose target was already countered did not fizzle loudly",
        format!(
            "life {:?}, fizzled {:?}",
            core.life,
            steps.iter().map(|s| s.fizzled).collect::<Vec<_>>()
        ),
    );

    // Bury and Raise.
    let mut core = with_stack(&[
        item(1, Card::Ember, Side::Npc, None),
        item(2, Card::Cleaver, Side::Npc, None),
        item(3, Card::Mend, Side::You, None),
        item(4, Card::Bury, Side::You, Some(3)),
    ]);
    let ids: Vec<u32> = {
        resolve_top(&mut core);
        core.stack.iter().map(|i| i.id.0).collect()
    };
    checks.require(
        ids == [3, 1, 2],
        "Bury did not move its target to the bottom",
        format!("{ids:?}"),
    );
    let mut core = with_stack(&[
        item(1, Card::Ember, Side::Npc, None),
        item(2, Card::Cleaver, Side::Npc, None),
        item(3, Card::Mend, Side::You, None),
        item(4, Card::Raise, Side::You, Some(1)),
    ]);
    resolve_top(&mut core);
    let ids: Vec<u32> = core.stack.iter().map(|i| i.id.0).collect();
    checks.require(
        ids == [2, 3, 1],
        "Raise did not move its target to the top",
        format!("{ids:?}"),
    );

    // Mend by the other side, redirected, heals the redirector.
    let mut core = with_stack(&[
        item(1, Card::Mend, Side::Npc, None),
        item(2, Card::Redirect, Side::You, Some(1)),
    ]);
    let _ = resolve_all(&mut core);
    checks.require(
        core.life == [24, 20],
        "a redirected Mend did not heal the other player",
        format!("{:?}", core.life),
    );

    // Lethal damage ends the match at once, with the rest of the stack left.
    let mut core = with_stack(&[
        item(1, Card::Cleaver, Side::Npc, None),
        item(2, Card::Ember, Side::You, None),
    ]);
    core.life = [20, 2];
    let steps = preview(&core);
    resolve_top(&mut core);
    checks.require(
        core.outcome == Some(Outcome::Won(Side::You)) && steps.len() == 1,
        "lethal damage did not end the match immediately",
        format!("{:?}, preview of {} steps", core.outcome, steps.len()),
    );
}

fn opening_and_npc(checks: &mut Checks) {
    let core = Core::new(3);
    checks.require(
        core.hands[0].len() == 5 && core.hands[1].len() == 5 && core.decks[0].len() == 15,
        "a match does not open with five cards each and fifteen in the deck",
        format!(
            "hands {} {}, deck {}",
            core.hands[0].len(),
            core.hands[1].len(),
            core.decks[0].len()
        ),
    );
    // Flip over a single item changes nothing, so it is refused.
    let mut one = Core::new(3);
    one.hands[0] = vec![Card::Flip];
    one.mana = [6, 6];
    one.stack = vec![item(1, Card::Ember, Side::Npc, None)];
    checks.require(
        one.options(Side::You, 0).err().map(|e| e.what).as_deref()
            == Some("Flip would change nothing"),
        "Flip was allowed over a one-item stack",
        "a stack of one item".to_owned(),
    );
    // The NPC answers a 6-damage Cleaver aimed at it with Redirect, then Negate; it
    // cannot Hush a cost-3 card, so with only Hush it lets it resolve.
    let answer = |hand: &[Card]| {
        let mut core = Core::new(3);
        core.active = Side::You;
        core.priority = Side::Npc;
        core.hands[1] = hand.to_vec();
        core.mana = [6, 6];
        let mut cleaver = item(1, Card::Cleaver, Side::You, None);
        cleaver.aim = Some(Side::Npc);
        core.stack = vec![cleaver];
        npc::choose(&core)
    };
    let play = |hand_index| Action::Play {
        hand_index,
        target: Some(ItemId(1)),
    };
    checks.require(
        answer(&[Card::Negate, Card::Redirect]) == play(1)
            && answer(&[Card::Hush, Card::Negate]) == play(1)
            && answer(&[Card::Hush]) == Action::Pass
            && answer(&[Card::Ember]) == Action::Pass,
        "the NPC does not answer big damage with Redirect, then Negate",
        format!(
            "{:?} {:?} {:?} {:?}",
            answer(&[Card::Negate, Card::Redirect]),
            answer(&[Card::Hush, Card::Negate]),
            answer(&[Card::Hush]),
            answer(&[Card::Ember])
        ),
    );
}

fn turn_machine(checks: &mut Checks) {
    let mut core = Core::new(3);
    let (a, b) = (core.hands[1].len(), core.decks[1].len());
    let _ = core.try_pass(Side::You);
    let early = core.turn;
    let _ = core.try_pass(Side::Npc);
    checks.require(
        early == 1 && core.turn == 2 && core.active == Side::Npc && core.priority == Side::Npc,
        "two passes on an empty stack did not end the turn",
        format!("turn {} active {:?}", core.turn, core.active),
    );
    checks.require(
        core.mana == [1, 1] && core.hands[1].len() == a + 1 && core.decks[1].len() == b - 1,
        "the new turn did not refill mana and draw one card",
        format!(
            "mana {:?}, hand {} deck {}",
            core.mana,
            core.hands[1].len(),
            core.decks[1].len()
        ),
    );
    core.turns_taken = [9, 9];
    core.hands[0] = vec![Card::Ember; 7];
    let deck_before = core.decks[0].len();
    core.active = Side::Npc;
    core.priority = Side::Npc;
    core.passes = 0;
    let _ = core.try_pass(Side::Npc);
    let _ = core.try_pass(Side::You);
    checks.require(
        core.mana[0] == 6 && core.hands[0].len() == 7 && core.decks[0].len() == deck_before,
        "mana did not stop growing at six, or a full hand drew",
        format!(
            "mana {:?}, hand {}, deck {} of {deck_before}",
            core.mana,
            core.hands[0].len(),
            core.decks[0].len()
        ),
    );
    let mut core = Core::new(3);
    core.turn = 24;
    core.life = [12, 9];
    let _ = core.try_pass(Side::You);
    let _ = core.try_pass(Side::Npc);
    checks.require(
        core.outcome == Some(Outcome::Won(Side::You)),
        "the turn limit did not give the match to the higher life",
        format!("{:?}", core.outcome),
    );
    // Casting passes priority; two passes resolve; the active player regains it.
    let mut core = Core::new(3);
    core.hands[0] = vec![Card::Ember];
    let _ = core.try_play(Side::You, 0, None);
    checks.require(
        core.priority == Side::Npc && core.passes == 0,
        "a cast did not hand priority over",
        format!("{:?}", core.priority),
    );
    let _ = core.try_pass(Side::Npc);
    checks.require(
        core.priority == Side::You && core.stack.len() == 1,
        "one pass resolved the stack",
        format!("{} items", core.stack.len()),
    );
    let _ = core.try_pass(Side::You);
    checks.require(
        core.stack.is_empty() && core.life == [20, 18] && core.priority == Side::You,
        "two passes did not resolve the top item",
        format!("life {:?}", core.life),
    );
}

/// How many seeds a sweep plays, and the claims it makes.
pub const SWEEP_SEEDS: u64 = 40;

pub struct Sweep {
    pub blind_wins: u32,
    pub power_wins: u32,
    pub skilled_wins: u32,
    pub manipulation_plays: u32,
    pub windows: u32,
    pub windows_with_stack: u32,
}

fn is_manipulation(card: Card) -> bool {
    !matches!(card, Card::Ember | Card::Cleaver | Card::Siege | Card::Mend)
}

/// Whole seeded matches for all three players, and two properties over every
/// priority window the skilled player met.
pub fn sweep(checks: &mut Checks) -> Sweep {
    let mut out = Sweep {
        blind_wins: 0,
        power_wins: 0,
        skilled_wins: 0,
        manipulation_plays: 0,
        windows: 0,
        windows_with_stack: 0,
    };
    for seed in 0..SWEEP_SEEDS {
        let wins = |outcome: Option<Outcome>| u32::from(outcome == Some(Outcome::Won(Side::You)));
        out.blind_wins += wins(players::play_match(seed, players::blind).outcome);
        out.power_wins += wins(players::play_match(seed, players::power).outcome);
        // The skilled match is replayed here by hand so each window can be inspected.
        let mut core = Core::new(seed);
        for _ in 0..5000 {
            if core.outcome.is_some() {
                break;
            }
            let side = core.priority;
            if side == Side::You {
                out.windows += 1;
                // Every action `legal_actions` offers must succeed on a copy.
                for action in legal_actions(&core, side) {
                    let mut trial = core.clone();
                    checks.require(
                        trial.try_act(side, action).is_ok(),
                        "legal_actions offered an action the rules then refused",
                        format!("seed {seed}: {action:?}"),
                    );
                }
                // The preview of the top item is what two passes then do.
                if !core.stack.is_empty() {
                    out.windows_with_stack += 1;
                    let predicted = preview(&core);
                    let mut trial = core.clone();
                    let _ = trial.try_pass(Side::You);
                    let _ = trial.try_pass(Side::Npc);
                    checks.require(
                        trial.log.last() == predicted.first().map(|s| &s.text),
                        "the preview and the real resolution disagree",
                        format!(
                            "seed {seed}: {:?} vs {:?}",
                            trial.log.last(),
                            predicted.first().map(|s| &s.text)
                        ),
                    );
                }
            }
            let action = if side == Side::You {
                players::skilled(&core)
            } else {
                npc::choose(&core)
            };
            if side == Side::You
                && let Action::Play { hand_index, .. } = action
                && is_manipulation(core.hands[0][hand_index])
            {
                out.manipulation_plays += 1;
            }
            if let Err(error) = core.try_act(side, action) {
                checks.require(
                    false,
                    "a player chose an illegal action",
                    format!("seed {seed}: {error}"),
                );
                break;
            }
        }
        out.skilled_wins += wins(core.outcome);
    }
    out
}

pub fn run_all(checks: &mut Checks) {
    card_table(checks);
    legality(checks);
    opening_and_npc(checks);
    effects(checks);
    turn_machine(checks);
}
