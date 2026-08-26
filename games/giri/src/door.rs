//! The door rule, asked directly (DESIGN §6 — unchanged by v2, and re-asked
//! through the v2 function).
//!
//! The tutorial's rosters produce a refusal but rarely a veto, because nobody
//! in them is often standing inside a party an arrival would cost. So both
//! directions are asked here, on benches built for the case, and the mutation
//! round runs this file with every constant perturbed in turn — which is what
//! makes "a veto flips when `mark_dark` moves" an assertion rather than a
//! claim.

use jidousha::prelude::*;

use crate::checks::Checks;
use crate::constants::Tuning;
use crate::contracts::{bench, bench_job};
use crate::flow::assess;
use crate::model::Social;
use crate::traits::{MarkId, TraitId};
use crate::willing::{Admission, admit};

/// **The door rule** (DESIGN §6), asked directly.
///
/// Both directions and both failures, on one bench, because the rule's content
/// is that they are *the same numbers seen from two sides*: with the clean one
/// at the door they refuse, with the clean one already inside they block, and
/// the margin quoted is the same margin. A model that had only kept rule 1
/// would pass every willingness assertion and fail here.
///
/// Then the half no arrival can show: consent is evaluated **at the door
/// only**, so a departure that pushes a remaining member negative leaves them
/// in the party and the party sendable. That is the decided behaviour, and
/// asserting it is what stops a later reading of "willingness" quietly turning
/// membership into something re-checked every tick.
pub fn door(checks: &mut Checks, tuning: &Tuning) {
    // Clean is upright and hungry by exactly one; Known wears the mark. The
    // upright reaction (-mark_dark - 2) beats a desperation of 1 at every
    // authorable mark_dark, which is what makes the pair a door case.
    let rows: &[(&'static str, i32, &'static [TraitId], &'static [MarkId])] = &[
        ("Clean", 1, &[TraitId::Upright], &[]),
        ("Known", 1, &[], &[MarkId::ComradeKiller]),
    ];
    // --- rule 2: an incumbent blocks an arrival --------------------------
    let (world, ids) = bench(rows, &[]);
    let social = Social::read(&world.view());
    let answer = admit(&social, tuning, ids[1], &ids[..1], None);
    match &answer {
        Admission::Blocked {
            blocker,
            name,
            willingness,
        } => {
            checks.require(
                *blocker == ids[0] && *name == "Clean" && willingness.margin < 0,
                "an incumbent's veto names the wrong person or the wrong margin",
                format!(
                    "the door said {name} blocks, with {}",
                    willingness.breakdown()
                ),
            );
            checks.require(
                willingness
                    .top_reason()
                    .contains("won't work with a comrade-killer"),
                "a veto's reason is not the mark that caused it",
                format!("the blocker's top reason is {:?}", willingness.top_reason()),
            );
        }
        other => checks.require(
            false,
            "an incumbent whose willingness would go negative did not block the arrival",
            format!(
                "Clean is standing in a party of one, upright against a comrade-killer at a \
                 mark_dark of {}, and the door said {other:?} to Known joining",
                tuning.mark_dark
            ),
        ),
    }
    // --- rule 1: the same two, the other way round -----------------------
    let mirror = admit(&social, tuning, ids[0], &ids[1..], None);
    checks.require(
        matches!(mirror, Admission::Refuses(_)),
        "the door rule is not symmetric about who is standing at it",
        format!(
            "with Known inside, Clean answered {mirror:?}; the same numbers make Clean refuse \
             that make Clean block"
        ),
    );
    let (blocked_sum, refused_sum) = (
        match &answer {
            Admission::Blocked { willingness, .. } => willingness.margin,
            _ => i32::MAX,
        },
        match &mirror {
            Admission::Refuses(entry) => entry.margin,
            _ => i32::MIN,
        },
    );
    checks.require(
        blocked_sum == refused_sum,
        "the veto and the refusal are not the same margin",
        format!(
            "blocking quoted {blocked_sum} and refusing quoted {refused_sum}; DESIGN §6 says \
             they are one arithmetic seen from two sides"
        ),
    );
    // --- a bond outweighs the mark, so the same pair is admitted ----------
    let (world, bonded) = bench(rows, &[(0, 1, 8)]);
    let social = Social::read(&world.view());
    checks.require(
        admit(&social, tuning, bonded[1], &bonded[..1], None).admitted(),
        "a bond no longer gets an arrival past the incumbent it is with",
        format!(
            "Clean holds 8 toward Known against an upright reaction at a mark_dark of {} and \
             still blocked",
            tuning.mark_dark
        ),
    );

    // --- consent is evaluated at the door only ---------------------------
    //
    // Anchor stands next to two marked names only because of what they hold
    // toward Friend. Take Friend away and Anchor is under water — and stays in
    // the party, and the party stays sendable.
    let (world, ids) = bench(
        &[
            ("Anchor", 0, &[], &[]),
            ("Friend", 1, &[], &[MarkId::ComradeKiller]),
            ("Known", 1, &[], &[MarkId::ComradeKiller]),
        ],
        &[(0, 1, 8)],
    );
    let social = Social::read(&world.view());
    let mut party: Vec<Entity> = Vec::new();
    for candidate in ids.iter().copied() {
        let door = admit(&social, tuning, candidate, &party, None);
        checks.require(
            door.admitted(),
            "the door turned somebody away that the bench assembles in order",
            format!(
                "{} was refused or blocked: {}",
                social.name(candidate),
                door.status_line()
            ),
        );
        party.push(candidate);
    }
    let departed = vec![ids[0], ids[2]];
    let gate = assess(&social, tuning, &departed, Some(&bench_job(2, 8, 0)));
    let anchor = gate.entries.first().map(|entry| entry.margin);
    checks.require(
        anchor.is_some_and(|total| total < 0),
        "the bench's departure case no longer pushes anybody negative",
        format!(
            "Anchor's margin beside Known alone is {anchor:?} at a mark_dark of {}; the case \
             is only about a departure if the departure costs something",
            tuning.mark_dark
        ),
    );
    checks.require(
        !gate.all_willing && gate.can_send && gate.blocked.is_empty(),
        "a member who went negative after a departure was thrown out of the party",
        format!(
            "the gate said can_send {}, all_willing {}, blocked {:?}; consent is evaluated at \
             the door only, so a party assembled legally stays sendable",
            gate.can_send, gate.all_willing, gate.blocked
        ),
    );
    // And the card says so rather than hiding it: the status line is the
    // member's own verdict and reason, and the colour is the second channel.
    let line = crate::party::status_line(&social.members[0], &gate, true);
    checks.require(
        line.starts_with("in - ") && line.contains("won't work with a comrade-killer"),
        "a member who went negative is not shown why",
        format!("the card says {line:?}"),
    );
}
