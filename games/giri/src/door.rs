//! The door rule, asked directly (DESIGN §3.2).
//!
//! Amendment 1's rule is the one simulation change this presentation round
//! made, and it is the one a played beat exercises least: the tutorial's four
//! rosters produce a refusal but never a veto, because nobody in them is ever
//! standing inside a party an arrival would cost. So it is asked here, on
//! benches built for the case, and the mutation round runs this file with every
//! constant perturbed in turn — which is what makes "a veto flips when `K_inf`
//! moves" an assertion rather than a claim.

use jidousha::prelude::*;

use crate::beats::{QuestIcon, Requirement};
use crate::checks::Checks;
use crate::constants::Tuning;
use crate::contracts::bench;
use crate::flow::assess;
use crate::model::{Admission, Social, admit};

/// **The door rule** (DESIGN §3.2), asked directly.
///
/// Both directions and both failures, on one bench, because the rule's content
/// is that they are *the same numbers seen from two sides*: with the clean one
/// at the door they refuse, with the clean one already inside they block, and
/// the sum quoted is the same sum. A model that had only kept rule 1 would pass
/// every willingness assertion in this file and fail here.
///
/// Then the half no arrival can show: consent is evaluated **at the door
/// only**, so a departure that pushes a remaining member negative leaves them
/// in the party and the party sendable. That is the decided behaviour, and
/// asserting it is what stops a later reading of "willingness" quietly turning
/// membership into something re-checked every tick.
pub fn door(checks: &mut Checks, tuning: &Tuning) {
    // --- rule 2: an incumbent blocks an arrival --------------------------
    let (world, ids) = bench(&[("Clean", 1, 0), ("Known", 0, 3)], &[]);
    let social = Social::read(&world.view());
    let answer = admit(&social, tuning, ids[1], &ids[..1]);
    match &answer {
        Admission::Blocked {
            blocker,
            name,
            willingness,
        } => {
            checks.require(
                *blocker == ids[0] && *name == "Clean" && willingness.total < 0,
                "an incumbent's veto names the wrong person or the wrong arithmetic",
                format!(
                    "the door said {name} blocks, with {}",
                    willingness.arithmetic()
                ),
            );
        }
        other => checks.require(
            false,
            "an incumbent whose willingness would go negative did not block the arrival",
            format!(
                "Clean is standing in a party of one at a K_inf of {} and the door said \
                 {other:?} to Known joining",
                tuning.k_inf
            ),
        ),
    }
    // --- rule 1: the same two, the other way round -----------------------
    let mirror = admit(&social, tuning, ids[0], &ids[1..]);
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
            Admission::Blocked { willingness, .. } => willingness.total,
            _ => i32::MAX,
        },
        match &mirror {
            Admission::Refuses(entry) => entry.total,
            _ => i32::MIN,
        },
    );
    checks.require(
        blocked_sum == refused_sum,
        "the veto and the refusal are not the same sum",
        format!(
            "blocking quoted {blocked_sum} and refusing quoted {refused_sum}; DESIGN §3.2 says \
             they are one arithmetic seen from two sides"
        ),
    );
    // --- a bond outweighs the gap, so the same pair is admitted ----------
    let (world, bonded) = bench(&[("Clean", 1, 0), ("Known", 0, 3)], &[(0, 1, 5)]);
    let social = Social::read(&world.view());
    checks.require(
        admit(&social, tuning, bonded[1], &bonded[..1]).admitted(),
        "a bond no longer gets an arrival past the incumbent it is with",
        format!(
            "Clean holds 5 toward Known against a gap of 3 at a K_inf of {} and still blocked",
            tuning.k_inf
        ),
    );

    // --- consent is evaluated at the door only ---------------------------
    //
    // Anchor stands next to two known faces only because of what they hold
    // toward Friend. Take Friend away and Anchor is under water — and stays in
    // the party, and the party stays sendable.
    let (world, ids) = bench(
        &[("Anchor", 0, 0), ("Friend", 0, 3), ("Known", 0, 3)],
        &[(0, 1, 8)],
    );
    let social = Social::read(&world.view());
    let mut party: Vec<Entity> = Vec::new();
    for candidate in ids.iter().copied() {
        let door = admit(&social, tuning, candidate, &party);
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
    let job = crate::beats::Dungeon {
        name: "the bench",
        blurb: "a roster built for one question",
        icon: QuestIcon::Cave,
        headcount: 2,
        pot: 8,
        cut: 0,
        requires: Requirement::AnyParty,
    };
    let gate = assess(&social, tuning, &departed, Some(&job));
    let anchor = gate.entries.first().map(|entry| entry.total);
    checks.require(
        anchor.is_some_and(|total| total < 0),
        "the bench's departure case no longer pushes anybody negative",
        format!(
            "Anchor's willingness beside Known alone is {anchor:?} at a K_inf of {}; the case \
             is only about a departure if the departure costs something",
            tuning.k_inf
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
    // And the card says so rather than hiding it: the status line is still the
    // member's own sum, and the colour is the second channel.
    let line = crate::party::status_line(&social.members[0], &gate, true);
    checks.require(
        line.starts_with("in - ") && line.contains(&format!("= {}", anchor.unwrap_or(0))),
        "a member who went negative is not shown their own arithmetic",
        format!("the card says {line:?}"),
    );
}
