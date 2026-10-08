//! The words on screen: the status bar, the hint line and the result card,
//! each a function of the `Match` alone so `--verify` judges the exact strings
//! the player reads.

use crate::draw::NAMES;
use crate::rules::{self, Contact, GRACE_TICKS};
use crate::sim::Match;
use crate::zone::{next_zone, shrinking_or_starts, zone_at};

/// How many seconds of grace are left after `out_ticks` ticks outside.
pub fn grace_left(out_ticks: u32) -> f32 {
    (GRACE_TICKS + 1).saturating_sub(out_ticks) as f32 / 60.0
}

/// The two always-visible status lines.
pub fn status_lines(game: &Match) -> [String; 2] {
    let me = &game.golfers[0];
    let seconds = game.tick / 60;
    let zone = zone_at(&game.schedule, game.tick);
    let zone_text = match (
        next_zone(&game.schedule, game.tick),
        shrinking_or_starts(game.tick),
    ) {
        (Some((next, _)), Ok(())) => {
            format!("ZONE r{:.0} shrinking to r{:.0}", zone.radius, next.radius)
        }
        (Some((next, _)), Err(starts)) => format!(
            "ZONE r{:.0} -> r{:.0} in {:.1}s",
            zone.radius,
            next.radius,
            (starts - game.tick) as f32 / 60.0
        ),
        (None, _) => "ZONE closed".to_owned(),
    };
    let out = if me.out_ticks > 0 {
        format!("  OUT {:.1}s", grace_left(me.out_ticks))
    } else {
        String::new()
    };
    let stash = rules::Kept {
        tokens: me.stash,
        item: me.item,
    }
    .line();
    let first = format!(
        "TIME {}:{:02}  {zone_text}  STASH {stash}{out}",
        seconds / 60,
        seconds % 60
    );
    let pads = if (0..2).any(|pad| game.pad_open(pad)) {
        "pads open"
    } else if game.tick < rules::PADS_OPEN {
        "pads open at 0:32"
    } else {
        "pads closed"
    };
    let rivals = game.golfers[1..]
        .iter()
        .filter(|golfer| golfer.ending.is_none())
        .count();
    let second = format!(
        "EXTRACT keeps {} ({pads})  CHAMPION {}/{}  LAST STANDING {rivals} left",
        rules::settle(game, 0, rules::Ending::Extracted).line(),
        me.holes,
        rules::CHAMPION_HOLES,
    );
    [first, second]
}

/// The one line of what the keys do right now.
pub fn hint_line(game: &Match) -> String {
    let me = &game.golfers[0];
    if me.ending.is_some() {
        return "ENTER: next match".to_owned();
    }
    if me.stun > 0 {
        return format!("STUNNED {:.1}s", me.stun as f32 / 60.0);
    }
    if let Some(contact) = rules::contact_for(game, 0) {
        return match contact {
            Contact::Club {
                target,
                stun_ticks,
                steal,
            } => format!(
                "F: CLUB {} - stun {:.1}s, take {steal} tokens",
                NAMES[target],
                stun_ticks as f32 / 60.0
            ),
            Contact::Strike { owner, to, .. } => format!(
                "F: STRIKE {}'s ball - it rolls {:.1}",
                NAMES[owner],
                (to - game.golfers[owner].ball.pos).length()
            ),
        };
    }
    if game.pad_underfoot(0).is_some() {
        return format!(
            "X: EXTRACT - keep {}",
            rules::settle(game, 0, rules::Ending::Extracted).line()
        );
    }
    if let Some(index) = game.pickup_underfoot(0) {
        let replaces = match me.item {
            Some(item) => format!(" (replaces {})", item.name()),
            None => String::new(),
        };
        return format!(
            "E: TAKE {}{replaces}",
            rules::describe(game.pickups[index].item)
        );
    }
    if game.addressing(0) {
        return format!("SPACE: shoot  arrows: aim, power {:.0}%", me.power * 100.0);
    }
    "WASD: walk to your ball".to_owned()
}

/// The result card, one string per line.
pub fn result_lines(game: &Match) -> Vec<String> {
    let mut lines = Vec::new();
    let Some((ending, kept, tick)) = game.golfers[0].ending else {
        return lines;
    };
    lines.push(format!(
        "{} at {}:{:02}",
        ending.name(),
        tick / 3600,
        tick / 60 % 60
    ));
    lines.push(format!("kept {}", kept.line()));
    lines.push(String::new());
    for (who, golfer) in game.golfers.iter().enumerate() {
        let state = match golfer.ending {
            Some((ending, kept, _)) => format!("{} - {}", ending.name(), kept.line()),
            None => format!("still in - {} tokens, {} cups", golfer.stash, golfer.holes),
        };
        lines.push(format!("{:<7}{state}", NAMES[who]));
    }
    lines.push(String::new());
    lines.push("ENTER: next match".to_owned());
    lines
}
