//! Every sentence the screens say, built from the three decision functions:
//! the service rows from `outcome_of`, the nemesis card from
//! `defeat_progress`, the ledger from `social_outlook`. The jokes live in
//! `sim.rs` beside the themes and tiers they belong to.
//!
//! Key functions: `order_lines`, `card_lines`, `ledger_lines`, `end_lines`.

use crate::rules::{
    Consequence, cut_followers, defeat_progress, outcome_of, social_outlook, tier_of, title,
};
use crate::sim::*;
use crate::turn::who;

/// How a line reads: plain, faint, a warning, the nemesis gold, or safe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    Plain,
    Faint,
    Warn,
    Gold,
    Safe,
}

/// The two lines of queue row `row`, and the tone of the second.
pub(crate) fn order_lines(game: &Game, row: usize) -> (String, String, Tone) {
    let order = game.queue[row];
    let n = row + 1;
    if let Kind::Nemesis(id) = order.kind {
        let first = match game.find_nemesis(id) {
            Some(nemesis) => format!(
                "[{n}] {} - {}, {} fol - wants {}, need {}",
                title(nemesis).to_uppercase(),
                tier_of(nemesis.followers).name(),
                nemesis.followers,
                order.customer.theme.name(),
                order.customer.need
            ),
            None => format!("[{n}] {} - defeated", who(game, row).to_uppercase()),
        };
        if order.served {
            return (first, "  SERVED".to_owned(), Tone::Safe);
        }
        let unmet = outcome_of(&order, game).if_unmet;
        let second = format!(
            "  unmet: +{} fol -${} -{} rep | press {n} for the card",
            unmet.leader_followers, -unmet.money, -unmet.rep
        );
        return (first, second, Tone::Gold);
    }
    let first = format!(
        "[{n}] {} wants {} ({}), need {}",
        who(game, row),
        order.customer.theme.dish(),
        order.customer.theme.name(),
        order.customer.need
    );
    if order.served {
        return (first, "  SERVED".to_owned(), Tone::Safe);
    }
    let outcome = outcome_of(&order, game);
    let (served, unmet) = (outcome.if_served, outcome.if_unmet);
    let (consequence, tone) = match unmet.consequence {
        Consequence::Spawns => ("SPAWN".to_owned(), Tone::Warn),
        Consequence::Feeds { id, followers } => {
            let name = game.find_nemesis(id).map_or("a nemesis", title);
            (format!("feeds {name} +{followers} fol"), Tone::Warn)
        }
        Consequence::Nothing => ("no spawn".to_owned(), Tone::Faint),
    };
    let second = format!(
        "  served: +${} +{} rep | unmet: {} sev {} -> {consequence}, -${} -{} rep",
        served.money,
        served.rep,
        unmet.theme.name(),
        unmet.severity,
        -unmet.money,
        -unmet.rep
    );
    let mut first = first;
    if unmet.leader_followers > 0 {
        first.push_str(&format!(" +{} fol if unmet", unmet.leader_followers));
    }
    (first, second, tone)
}

/// The nemesis card's rows, title row first.
pub(crate) fn card_lines(nemesis: &Nemesis, capacity_left: u32) -> Vec<String> {
    let progress = defeat_progress(nemesis, capacity_left);
    let theme = nemesis.theme.name();
    let serve_cost = NEMESIS_BASE_NEED + SENSITIVITY;
    let mut tally = format!(
        "won over {} of {} - S serves ({serve_cost} units)",
        progress.tally.0, progress.tally.1
    );
    if progress.defeated_if_served {
        tally.push_str(", DEFEATS them");
    }
    let overwhelm = match progress.overwhelm {
        Some(cost) => format!(
            "overwhelm - O ({cost} units), leaves {} of {capacity_left} for the rest, DEFEATS them",
            progress.leaves_for_rest
        ),
        None => format!(
            "overwhelm - O needs {} units, only {capacity_left} left",
            serve_cost + OVERWHELM_EXTRA
        ),
    };
    vec![
        "NEMESIS CARD".to_owned(),
        format!(
            "{} - {theme}, sensitivity +{SENSITIVITY} {theme}",
            title(nemesis).to_uppercase()
        ),
        format!(
            "{}, {} followers, {} spends to ratio",
            progress.tier.name(),
            progress.followers,
            progress.spends_to_ratio
        ),
        tally,
        overwhelm,
        nemesis.theme.complaint().to_owned(),
        "Esc back".to_owned(),
    ]
}

/// The spend preview for the nemesis in `slot`: `social_outlook` on a copy
/// with one cut applied, against the real list.
pub(crate) fn spend_line(game: &Game, slot: usize) -> String {
    let now = social_outlook(&game.nemeses, &game.tomorrow_base);
    let mut after = game.nemeses.clone();
    cut_followers(&mut after[slot]);
    let then = social_outlook(&after, &game.tomorrow_base);
    let (a, b) = (now.per[slot], then.per[slot]);
    let mut line = format!(
        "spend ${SOCIAL_COST}: {} -> {} fol, {} -> {}, {} -> {} followers",
        a.followers,
        b.followers,
        a.tier.name(),
        if b.followers == 0 {
            "gone"
        } else {
            b.tier.name()
        },
        a.share,
        b.share
    );
    if b.followers == 0 {
        line.push_str(", DEFEATS them");
    }
    line
}

/// Whether a log line is one the ledger shows (events, not per-order lines).
fn is_event(line: &str) -> bool {
    ["SPAWNED:", "FED:", "WON OVER:", "OVERWHELMED:", "RATIOED:"]
        .iter()
        .any(|head| line.starts_with(head))
        || line.contains(" is trending: ")
}

/// Events the ledger lists at most.
const LEDGER_EVENTS: usize = 6;

/// The ledger's rows, after its title, with their tones.
pub(crate) fn ledger_lines(game: &Game) -> Vec<(String, Tone)> {
    let mut rows = Vec::new();
    let t = game.tonight;
    rows.push((
        format!(
            "tonight: served {}, unmet {} (-${}, -{} rep)",
            t.served, t.unmet, t.money_lost, t.rep_lost
        ),
        Tone::Plain,
    ));
    for line in game.log.iter().filter(|l| is_event(l)).take(LEDGER_EVENTS) {
        rows.push((line.clone(), Tone::Warn));
    }
    rows.push((
        format!(
            "money ${} (closes below $0)  rep {} (closes at 0)",
            game.money, game.rep
        ),
        Tone::Plain,
    ));
    let outlook = social_outlook(&game.nemeses, &game.tomorrow_base);
    for (slot, (nemesis, reach)) in game.nemeses.iter().zip(&outlook.per).enumerate() {
        let progress = defeat_progress(nemesis, game.capacity_left);
        rows.push((
            format!(
                "[{}] {} - {}, {} fol, {} spends to ratio",
                slot + 1,
                title(nemesis).to_uppercase(),
                reach.tier.name(),
                reach.followers,
                progress.spends_to_ratio,
            ),
            Tone::Gold,
        ));
        rows.push((format!("  {}", reach.tier.post()), Tone::Faint));
        rows.push((
            format!(
                "  tomorrow {} of 6 customers are followers ({}, +{FOLLOWER_EXTRA_NEED} need)",
                reach.share,
                nemesis.theme.name()
            ),
            Tone::Plain,
        ));
        rows.push((format!("  {}", spend_line(game, slot)), Tone::Faint));
    }
    let capacity = BASE_CAPACITY + PREP_BONUS * game.preps_tonight;
    rows.push((
        format!(
            "P prep ${PREP_COST}: capacity tomorrow {capacity} -> {} (max {})",
            capacity + PREP_BONUS,
            BASE_CAPACITY + PREP_BONUS * MAX_PREPS
        ),
        Tone::Plain,
    ));
    rows.push(("tomorrow:".to_owned(), Tone::Plain));
    for (index, (kind, customer)) in outlook.customers.iter().enumerate() {
        let mark = match kind {
            Kind::Follower { leader } => game
                .find_nemesis(*leader)
                .map(|n| format!(" F:{}", title(n).to_uppercase()))
                .unwrap_or_default(),
            _ => String::new(),
        };
        rows.push((
            format!(
                "{}. {} need {} temper {}{mark}",
                index + 1,
                customer.theme.name(),
                customer.need,
                customer.temper
            ),
            if mark.is_empty() {
                Tone::Faint
            } else {
                Tone::Gold
            },
        ));
    }
    rows
}

/// The end screen's rows: the verdict, the reason, and the run's tally.
pub(crate) fn end_lines(game: &Game, won: bool, reason: &str) -> Vec<String> {
    let (first, second) = if won {
        (
            "SURVIVED 7 DAYS - THE REVIEWS ARE MIXED".to_owned(),
            "the doors stay open".to_owned(),
        )
    } else {
        let why = if reason == "money" {
            "out of money"
        } else {
            "reputation hit zero"
        };
        (
            "THE DOORS ARE SHUT".to_owned(),
            format!("CLOSED DOWN - {why}"),
        )
    };
    let mut rows = vec![
        first,
        second,
        format!("day {}, money ${}, rep {}", game.day, game.money, game.rep),
        format!(
            "nemeses spawned {}, defeated {}",
            game.nemeses.len() + game.defeated.len(),
            game.defeated.len()
        ),
    ];
    for nemesis in game.defeated.iter().chain(&game.nemeses) {
        let state = if game.defeated.contains(nemesis) {
            "defeated"
        } else {
            "still out there"
        };
        rows.push(format!(
            "  {} ({}) - {state}",
            title(nemesis),
            nemesis.theme.name()
        ));
    }
    rows.push("ENTER to open again".to_owned());
    rows
}
