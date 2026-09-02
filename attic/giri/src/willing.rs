//! The decision function v2: one function, a verdict, its margin, and its
//! reasons as words (DESIGN.md §6).
//!
//! willingness(c, party, quest) reads desperation and its source, the
//! trait-filtered reactions to each member's marks, the regard edges as the
//! traits weigh them, and the pot as the traits let it pull. It answers with a
//! **verdict** — joins / reluctant / refuses — its **margin** (kept on the
//! answer as strain groundwork for P2; nothing in this phase reads it after
//! the door), and its **reasons**: the top contributing causes, rendered from
//! the fixed vocabulary below and never from ad-hoc formatting.
//!
//! **One function, still** (DESIGN §6's discipline, ADR-0039's mechanism):
//! the party strip, the info panel, the door and the send gate all call this,
//! so the preview cannot say something the resolution disagrees with. The
//! door rule itself is unchanged from v1 — newcomer consent plus incumbent
//! veto, evaluated at the door only — and evaluates through this function.
//!
//! **The reason vocabulary is data.** One template per cause kind, ASCII,
//! filled with a mark's or a member's name and nothing else. A cause the
//! vocabulary has no words for is a compile error here, not a debug string on
//! a card.

use jidousha::prelude::*;

use crate::beats::Dungeon;
use crate::constants::Tuning;
use crate::model::{Social, share_each};
use crate::traits::{MarkId, MarkTone, reaction_delta};

/// The judgment the surface shows (DESIGN §6, §12): the sum stays behind
/// inspection, the verdict and its words go on the card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The margin clears `reluctant_below`.
    Joins,
    /// In, but barely: `0 <= margin < reluctant_below`. Strain, once P2 reads
    /// it.
    Reluctant,
    /// The margin is negative.
    Refuses,
}

/// One contributing cause, with the weight it contributed.
///
/// The weight is what sorts causes into "top reasons"; the words come from
/// [`Reason::text`], the one renderer the vocabulary has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    /// Desperation opened the sum.
    NeedsMoney,
    /// A mark on a partymate repels this character.
    MarkAgainst {
        /// The mark.
        mark: MarkId,
        /// Who wears it.
        of: &'static str,
    },
    /// A mark on a partymate attracts this character (DESIGN §5: reactions
    /// open doors too).
    MarkFor {
        /// The mark.
        mark: MarkId,
        /// Who wears it.
        of: &'static str,
    },
    /// Positive regard toward a partymate.
    Trusts {
        /// Who.
        of: &'static str,
    },
    /// Negative regard toward a partymate.
    Grudge {
        /// Who.
        of: &'static str,
    },
    /// The pot, through this character's traits.
    PotPull,
    /// Nothing contributed either way — the fallback that keeps "every verdict
    /// carries at least one reason" true by construction.
    Indifferent,
}

/// A cause and what it contributed to the sum.
#[derive(Clone, Copy, Debug)]
pub struct Reason {
    /// The cause.
    pub cause: Cause,
    /// Its signed contribution.
    pub weight: i32,
}

impl Reason {
    /// The cause as words — **the fixed vocabulary** (DESIGN §6, §14).
    ///
    /// One template per cause kind. ASCII (the font's whole range), filled
    /// with a mark's or a member's name and nothing else; the transcript
    /// asserts these exact strings, which is what makes "never free-form
    /// debug text" a property rather than a hope.
    pub fn text(&self) -> String {
        match self.cause {
            Cause::NeedsMoney => "needs the money".to_owned(),
            Cause::MarkAgainst { mark, .. } => format!("won't work with a {}", mark.name()),
            Cause::MarkFor { mark, .. } => format!("prefers a known {}", mark.name()),
            Cause::Trusts { of } => format!("trusts {of}"),
            Cause::Grudge { of } => format!("despises {of}"),
            Cause::PotPull => "the money is good".to_owned(),
            Cause::Indifferent => "nothing pulls either way".to_owned(),
        }
    }
}

/// One partymate's contribution to a willingness sum — the rung-2 breakdown,
/// kept as data (UI.md §5).
#[derive(Clone, Copy, Debug)]
pub struct MemberTerm {
    /// Who the term is about.
    pub member: Entity,
    /// The trait-filtered reaction to their marks.
    pub reaction: i32,
    /// The trait-weighted regard toward them.
    pub regard: i32,
}

/// A character's answer to "will you join this party for this job".
///
/// Carried whole rather than recomputed: the strip's verdict line, the info
/// panel's can't-join reason, the bounce toast and the log all read one of
/// these, so one refusal is one sentence everywhere.
#[derive(Clone, Debug)]
pub struct Willingness {
    /// Who was asked.
    pub who: Entity,
    /// Their name.
    pub name: &'static str,
    /// The desperation that opened the sum.
    pub desperation: i32,
    /// The sum of trait-filtered mark reactions.
    pub reaction_total: i32,
    /// The sum of trait-weighted regard.
    pub regard_total: i32,
    /// The pot's pull, through this character's traits.
    pub pot_total: i32,
    /// The margin: `desperation + reactions + regard + pot`.
    ///
    /// Stored on every answer as strain groundwork (DESIGN §6 — the margin
    /// persists into the quest as strain in P2). **Nothing in this phase
    /// reads it after the door**, and the deterministic betrayal rule does
    /// not see it.
    pub margin: i32,
    /// The judgment.
    pub verdict: Verdict,
    /// The contributing causes, strongest first. Never empty.
    pub reasons: Vec<Reason>,
    /// One term per other party member, in roster order.
    pub terms: Vec<MemberTerm>,
}

impl Willingness {
    /// Joins iff the margin is not negative — reluctant is still in.
    pub fn joins(&self) -> bool {
        self.margin >= 0
    }

    /// The leading cause, as words. Never empty (`Cause::Indifferent` backs
    /// it).
    pub fn top_reason(&self) -> String {
        self.reasons.first().map_or_else(String::new, Reason::text)
    }

    /// The verdict word the strip prints for a candidate.
    pub fn verdict_word(&self) -> &'static str {
        match self.verdict {
            Verdict::Joins => "would join",
            Verdict::Reluctant => "reluctant",
            Verdict::Refuses => "refuses",
        }
    }

    /// The sums, for a check's message — stderr prose, never drawn.
    pub fn breakdown(&self) -> String {
        format!(
            "d{:+} m{:+} r{:+} p{:+} = {}",
            self.desperation, self.reaction_total, self.regard_total, self.pot_total, self.margin
        )
    }
}

/// Regard as this character's traits weigh it: bonds through the bond
/// multipliers, grudges through the grudge multipliers, exactly.
fn weighted_regard(raw: i32, traits: &[crate::traits::TraitId]) -> i32 {
    if raw == 0 {
        return 0;
    }
    let (mut num, mut den) = (1i32, 1i32);
    for def in traits.iter().map(|id| id.def()) {
        if raw > 0 {
            num *= def.bond_num;
            den *= def.bond_den;
        } else {
            num *= def.grudge_num;
            den *= def.grudge_den;
        }
    }
    raw * num / den
}

/// The trait-filtered reaction to one mark: the tone's base, plus every table
/// cell the looker's traits hold for it.
fn reaction_to(tuning: &Tuning, traits: &[crate::traits::TraitId], mark: MarkId) -> i32 {
    let base = match mark.tone() {
        MarkTone::Dark => -tuning.mark_dark,
        MarkTone::Light => tuning.mark_light,
        MarkTone::Ambiguous => 0,
    };
    base + traits
        .iter()
        .map(|trait_id| reaction_delta(*trait_id, mark))
        .sum::<i32>()
}

/// **Willingness v2** (DESIGN §6): character `who` asked to join `party` for
/// `job`.
///
/// ```text
/// willingness(c, P, q) = desperation(c)
///                      + sum over m in P, mark on m: reaction(traits(c), mark)
///                      + sum over m in P: regard(c->m) as traits(c) weigh it
///                      + share(q) x pot_pull x pot_affinity(traits(c))
/// verdict: refuses below 0, reluctant below reluctant_below, joins above
/// ```
///
/// The share is the job's at its **stated headcount** — what the sheet
/// promises, not what the half-assembled party would split — so a character's
/// answer does not wobble while the party is being staged. `None` for `job`
/// is the board with nothing taken: no pot pulls yet.
pub fn willingness(
    social: &Social,
    tuning: &Tuning,
    who: Entity,
    party: &[Entity],
    job: Option<&Dungeon>,
) -> Willingness {
    let asker_traits = social.traits(who);
    let mut terms = Vec::new();
    let mut reasons: Vec<Reason> = Vec::new();
    let desperation = social.desperation(who);
    if desperation != 0 {
        reasons.push(Reason {
            cause: Cause::NeedsMoney,
            weight: desperation,
        });
    }
    for member in &social.members {
        if member.entity == who || !party.contains(&member.entity) {
            continue;
        }
        let mut reaction = 0;
        for mark in &member.marks {
            let one = reaction_to(tuning, &asker_traits, *mark);
            reaction += one;
            if one != 0 {
                reasons.push(Reason {
                    cause: if one < 0 {
                        Cause::MarkAgainst {
                            mark: *mark,
                            of: member.name,
                        }
                    } else {
                        Cause::MarkFor {
                            mark: *mark,
                            of: member.name,
                        }
                    },
                    weight: one,
                });
            }
        }
        let regard = weighted_regard(social.regard(who, member.entity), &asker_traits);
        if regard != 0 {
            reasons.push(Reason {
                cause: if regard > 0 {
                    Cause::Trusts { of: member.name }
                } else {
                    Cause::Grudge { of: member.name }
                },
                weight: regard,
            });
        }
        terms.push(MemberTerm {
            member: member.entity,
            reaction,
            regard,
        });
    }
    let reaction_total: i32 = terms.iter().map(|term| term.reaction).sum();
    let regard_total: i32 = terms.iter().map(|term| term.regard).sum();
    let affinity: i32 = asker_traits.iter().map(|id| id.def().pot_affinity).sum();
    let pot_total = job.map_or(0, |job| {
        share_each(
            job.pot,
            job.cut,
            i32::try_from(job.headcount).unwrap_or(i32::MAX),
        ) * tuning.pot_pull
            * affinity
    });
    if pot_total != 0 {
        reasons.push(Reason {
            cause: Cause::PotPull,
            weight: pot_total,
        });
    }
    // Strongest cause first; the sort is stable, so equal weights keep the
    // build order above (desperation, then each member's marks and edges in
    // roster order, then the pot) — deterministic, like everything else.
    reasons.sort_by_key(|reason| -reason.weight.abs());
    if reasons.is_empty() {
        reasons.push(Reason {
            cause: Cause::Indifferent,
            weight: 0,
        });
    }
    let margin = desperation + reaction_total + regard_total + pot_total;
    let verdict = if margin < 0 {
        Verdict::Refuses
    } else if margin < tuning.reluctant_below {
        Verdict::Reluctant
    } else {
        Verdict::Joins
    };
    Willingness {
        who,
        name: social.name(who),
        desperation,
        reaction_total,
        regard_total,
        pot_total,
        margin,
        verdict,
        reasons,
        terms,
    }
}

/// The answer at the door (DESIGN §6's door rule, **unchanged** — v2 only
/// changed what the sum is made of).
///
/// The two failures are different things to a player — one is somebody saying
/// no, the other is somebody inside saying no on their behalf — so they are
/// different variants, and each carries the whole `Willingness` its reason
/// line is read from.
#[derive(Clone, Debug)]
pub enum Admission {
    /// Both directions consent; the candidate is in.
    Admitted(Willingness),
    /// Rule 1: the newcomer will not come. Their answer.
    Refuses(Willingness),
    /// Rule 2: an incumbent would go negative and blocks the arrival.
    Blocked {
        /// Who is blocking.
        blocker: Entity,
        /// Their name, for the line that says so.
        name: &'static str,
        /// **The blocker's** answer, not the newcomer's — it is their
        /// objection.
        willingness: Willingness,
    },
}

impl Admission {
    /// Whether the candidate may be added.
    pub fn admitted(&self) -> bool {
        matches!(self, Admission::Admitted(_))
    }

    /// The margin behind the answer — the blocker's for a veto, because the
    /// objection is theirs.
    pub fn margin(&self) -> i32 {
        match self {
            Admission::Admitted(entry) | Admission::Refuses(entry) => entry.margin,
            Admission::Blocked { willingness, .. } => willingness.margin,
        }
    }

    /// The status line UI.md §4 states for a character not in the party:
    /// the verdict, and the leading reason as words.
    pub fn status_line(&self) -> String {
        match self {
            Admission::Admitted(entry) => {
                format!("{} - {}", entry.verdict_word(), entry.top_reason())
            }
            Admission::Refuses(entry) => format!("refuses - {}", entry.top_reason()),
            Admission::Blocked {
                name, willingness, ..
            } => format!("{name} blocks - {}", willingness.top_reason()),
        }
    }

    /// The toast a bounced click raises, naming who, the words, and the
    /// margin — the number stays reachable (invariant 2), one step off the
    /// card.
    pub fn bounce(&self, candidate: &str) -> Option<String> {
        match self {
            Admission::Admitted(_) => None,
            Admission::Refuses(entry) => Some(format!(
                "{candidate} refuses this company - {} ({})",
                entry.top_reason(),
                entry.margin
            )),
            Admission::Blocked {
                name, willingness, ..
            } => Some(format!(
                "{name} will not work with {candidate} - {} ({})",
                willingness.top_reason(),
                willingness.margin
            )),
        }
    }
}

/// **The door** (DESIGN §6, unchanged in rule, v2 in what it sums):
///
/// ```text
/// admit(c, P, q) iff willingness(c, P + {c}, q) >= 0
///              and for every m in P: willingness(m, P + {c}, q) >= 0
/// ```
///
/// Order-symmetric about who is at the door, and consent is evaluated **at
/// the door only** — nothing re-runs this when a member leaves (owner,
/// 2026-08-23). Incumbents are walked in roster order so the named blocker is
/// stable.
pub fn admit(
    social: &Social,
    tuning: &Tuning,
    candidate: Entity,
    party: &[Entity],
    job: Option<&Dungeon>,
) -> Admission {
    let mut with = party.to_vec();
    if !with.contains(&candidate) {
        with.push(candidate);
    }
    let newcomer = willingness(social, tuning, candidate, &with, job);
    if !newcomer.joins() {
        return Admission::Refuses(newcomer);
    }
    for member in &social.members {
        if !party.contains(&member.entity) || member.entity == candidate {
            continue;
        }
        let incumbent = willingness(social, tuning, member.entity, &with, job);
        if !incumbent.joins() {
            return Admission::Blocked {
                blocker: member.entity,
                name: member.name,
                willingness: incumbent,
            };
        }
    }
    Admission::Admitted(newcomer)
}
