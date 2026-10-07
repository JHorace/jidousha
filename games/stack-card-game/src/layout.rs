//! Layout stated as the game's requirements, checked at three window shapes
//! with the worst screen the rules can produce: a full hand of the longest
//! names, a stack deeper than its panel, a long log and a long refusal.

use crate::cards::{Card, Side};
use crate::checks::{Checks, greater};
use crate::game::Game;
use crate::rules::{Core, Item, ItemId, default_aim};
use crate::ui::{self, TEXT};
use jidousha::prelude::*;

fn worst_case() -> Game {
    let mut core = Core::new(5);
    core.hands[0] = vec![
        Card::Redirect,
        Card::Negate,
        Card::Cleaver,
        Card::Siege,
        Card::Echo,
        Card::Bury,
        Card::Hush,
    ];
    core.mana = [6, 6];
    for id in 1..=9u32 {
        let card = [Card::Cleaver, Card::Echo, Card::Redirect][id as usize % 3];
        let owner = if id % 2 == 0 { Side::You } else { Side::Npc };
        core.stack.push(Item {
            id: ItemId(id),
            card,
            owner,
            target: (card != Card::Cleaver).then_some(ItemId(1)),
            aim: default_aim(card, owner),
        });
    }
    for n in 0..30 {
        core.log
            .push(format!("NPC Redirect: an item's aim is flipped, line {n}"));
    }
    core.life = [100, 100];
    let mut game = Game::new(5);
    game.core = core;
    game.ui.selected = Some(0);
    game.ui.message =
        "sorcery-speed card: play it on your own turn once the stack has resolved, and not before"
            .to_owned();
    game
}

pub fn check(checks: &mut Checks) {
    let game = worst_case();
    // 16:9, 4:3 and 21:9: the camera is 20 tall and as wide as the aspect makes it.
    for (name, width) in [("16:9", 35.55), ("4:3", 26.66), ("21:9", 46.66)] {
        let view = Rect::from_center_size(Vec2::ZERO, Vec2::new(width, ui::VIEW_HEIGHT));
        let screen = ui::build(&game, view);
        for label in &screen.labels {
            let bounds = label.bounds();
            checks.require(
                label.clip.contains_rect(bounds) && view.contains_rect(bounds),
                "text spills out of its box",
                format!(
                    "{name}: {:?} spans {:.2}..{:.2} x {:.2}..{:.2} in a box {:.2}..{:.2} x {:.2}..{:.2}",
                    label.text, bounds.min.x, bounds.max.x, bounds.min.y, bounds.max.y,
                    label.clip.min.x, label.clip.max.x, label.clip.min.y, label.clip.max.y
                ),
            );
        }
        for block in &screen.blocks {
            checks.require(
                view.contains_rect(block.rect),
                "a panel or card leaves the screen",
                format!("{name}: {:?}", block.rect),
            );
        }
        for (i, a) in screen.hand_rects.iter().enumerate() {
            for b in &screen.hand_rects[i + 1..] {
                checks.require(
                    !a.overlaps(*b),
                    "two cards in hand overlap",
                    format!("{name}: {a:?} {b:?}"),
                );
            }
        }
        checks.require(
            screen.hand_rects.len() == 7,
            "a full hand is not seven cards on screen",
            format!("{name}: {}", screen.hand_rects.len()),
        );
        let panel = screen.stack_panel.unwrap_or(view);
        let style = TextStyle {
            size: TEXT,
            ..TextStyle::default()
        };
        for row in &screen.stack_rows {
            let left_end = panel.min.x + 1.1 + style.width_of(&row.left);
            let right_start = panel.max.x - 0.4 - style.width_of(&row.right);
            checks.require(
                greater(right_start - left_end, 0.2),
                "a stack row's two columns collide",
                format!(
                    "{name}: {:?} ends {left_end:.2}, {:?} starts {right_start:.2}",
                    row.left, row.right
                ),
            );
        }
        // A stack deeper than the panel says how much it left out.
        checks.require(
            screen.stack_rows.len() == 6,
            "a nine-item stack did not show six rows and a count of the rest",
            format!("{name}: {} rows", screen.stack_rows.len()),
        );
    }
}
