//! What a newcomer to the house is made of (SPEC §17.1, §17.4, §21): the name and
//! house bags, a new hero's blank record, the rolled dream, and the phase-effect
//! sentence the turning's lines read.
//!
//! The bags are W0's primitive (`chance::Bag`), three of them on the house: a name for
//! him, a name for her, a house — each drawn without replacement and refilled when
//! empty. Founders' names are not in them, so a newborn can share a founder's name
//! (OQ-26).

use jidousha::prelude::Rng;

use crate::chance::{Bag, unclaimed_index};
use crate::constants::{PHASE_ADJUSTMENT, PHASE_FROM_AGE, TEACHER_AGE_FOR_DREAM};
use crate::content::Content;
use crate::dream::Dream;
use crate::hero::{DestinyState, Fate, Fear, Hero};
use crate::ids::{Aptitude, Destiny, DreamKind, LegacyKind, Phase, Pronoun, Tag, Vocation};
use crate::text::{capitalized, fmt};

/// The house's three bags (SPEC §3.1 `generation`, §17.1).
#[derive(Clone, Debug, PartialEq)]
pub struct Bags {
    /// Thirty-two names for him.
    pub him: Bag<String>,
    /// Thirty-two names for her.
    pub her: Bag<String>,
    /// Sixteen houses.
    pub houses: Bag<String>,
}

impl Bags {
    /// Three full bags, from `names.json`.
    pub fn new(content: &Content) -> Self {
        Self {
            him: Bag::new(content.names.him.clone()),
            her: Bag::new(content.names.her.clone()),
            houses: Bag::new(content.names.houses.clone()),
        }
    }

    /// A name for a hero of `pronoun`.
    pub fn name(&mut self, pronoun: Pronoun, rng: &mut Rng) -> String {
        match pronoun {
            Pronoun::He => self.him.draw(rng),
            Pronoun::She => self.her.draw(rng),
        }
    }
}

/// He or she, 50/50 (SPEC §17.1): one draw.
pub fn roll_pronoun(rng: &mut Rng) -> Pronoun {
    Pronoun::ALL[crate::chance::index(rng, Pronoun::ALL.len())]
}

/// A fear of `tag`, untouched.
pub fn fear_of(tag: Tag) -> Fear {
    Fear {
        tag,
        dread: 0,
        courage: 0,
        conquered: false,
        broken: false,
        born_brave: false,
    }
}

/// A new hero's record: who they are, and every other field at its default — no
/// dream, an unspoken destiny, no bonds, the default calling (Knight, OQ-31).
pub fn newcomer(
    name: String,
    house: String,
    pronoun: Pronoun,
    age: i32,
    born_year: i32,
    fear: Fear,
) -> Hero {
    Hero {
        key: String::new(),
        name,
        house,
        family: true,
        pronoun,
        vocation: Vocation::Knight,
        age,
        born_year,
        aptitudes: [0; 3],
        dream: None,
        burden: None,
        traits: Vec::new(),
        fear,
        destiny: DestinyState {
            kind: Destiny::Unspoken,
            fulfilled: false,
            blood_of: None,
        },
        bonds: Vec::new(),
        heirloom: None,
        blessings: Vec::new(),
        scars: Vec::new(),
        deeds: Vec::new(),
        legacy: (LegacyKind::None, String::new()),
        renown: 0,
        wounded: false,
        settled: false,
        fate: Fate::Living,
        fate_year: 0,
        fate_age: 0,
        fate_telling: String::new(),
        death_place: None,
        death_tag: None,
        grieved: false,
        bequest_decided: false,
        bequest_heir: None,
        bequest_heirloom: None,
        dream_fate: crate::hero::DreamFate::Undecided,
        laid_year: None,
        wording: None,
        epitaph: None,
        parents: [None; 2],
        roads_walked: Vec::new(),
        quests_faced: 0,
        fears_faced: 0,
        winters_taught: 0,
    }
}

/// The two callings of `aptitude`, in table order (CONSTANTS §2).
pub fn vocations_of(content: &Content, aptitude: Aptitude) -> Vec<Vocation> {
    Vocation::ALL
        .iter()
        .copied()
        .filter(|v| content.lore.vocation_aptitudes[v.index()] == aptitude)
        .collect()
}

/// A rolled dream (SPEC §17.4, `generation/hero-context/hero-context.jai:46-59`): from
/// the eight wanderer dreams, claimed = the kinds any other living hero dreams, and
/// WORTHY_STUDENT for anyone younger than 35; uniform among the unclaimed, else among
/// all eight. `heroes` is everyone else; `age` is the dreamer's.
///
/// SPEC-GAPS KG-49: "dreamt by" reads a hero's own dream, done or not, not a burden.
pub fn rolled_dream(content: &Content, heroes: &[Hero], age: i32, rng: &mut Rng) -> Dream {
    let kinds = &content.wanderers.dreams;
    let claimed: Vec<bool> = kinds
        .iter()
        .map(|kind| {
            let dreamt = heroes
                .iter()
                .any(|h| h.is_living() && h.dream.as_ref().is_some_and(|d| d.kind == *kind));
            dreamt || (*kind == DreamKind::WorthyStudent && age < TEACHER_AGE_FOR_DREAM)
        })
        .collect();
    let kind = kinds[unclaimed_index(rng, &claimed)];
    match Dream::build(content, kind, None, None) {
        Ok(dream) => dream,
        Err(error) => panic!(
            "[keifu] a rolled dream did not build: {error}\n  likely cause: wanderers.json \
             names a dream with a setup\n  fix: SPEC §17.4 rolls the eight but AVENGE_THE_LOST"
        ),
    }
}

fn fragment<'c>(content: &'c Content, key: &str) -> &'c str {
    match content.lore.phase_effect.iter().find(|(k, _)| k == key) {
        Some((_, value)) => value,
        None => panic!(
            "[keifu] lore.json's phase_effect_fragments has no {key:?}\n  likely cause: the \
             file changed\n  fix: compare it with spec/content/README.md"
        ),
    }
}

/// The phase-effect sentence (SPEC §21, `lineage/lore.jai:125-157`): "He brings 1 less
/// in every aptitude until he is 20. " or "His Might fades by 1 and his Wits grows by 1. ",
/// then the phase's note — "He learns fast: ...", "He teaches well now."
pub fn phase_effect(content: &Content, phase: Phase, pronoun: Pronoun) -> String {
    let forms = &content.lore.pronouns[pronoun.index()];
    let adjustment = PHASE_ADJUSTMENT[phase.index()];
    let mut out = String::new();
    let word = |n: i32, less: &str, more: &str| -> String {
        fragment(content, if n < 0 { less } else { more }).to_owned()
    };
    if adjustment.iter().all(|&n| n == adjustment[0]) && adjustment[0] != 0 {
        let n = adjustment[0];
        out += &fmt(
            fragment(content, "uniform"),
            &[
                &capitalized(&forms.subject),
                &n.abs().to_string(),
                &word(n, "less", "more"),
            ],
        );
        if phase == Phase::Youth {
            out += &fmt(
                fragment(content, "youth_until"),
                &[
                    &forms.subject,
                    &PHASE_FROM_AGE[Phase::Prime.index()].to_string(),
                ],
            );
        }
        out += fragment(content, "sentence_end");
    } else {
        let mut any = false;
        for aptitude in Aptitude::ALL {
            let n = adjustment[aptitude.index()];
            if n == 0 {
                continue;
            }
            let his = if any {
                forms.possessive.clone()
            } else {
                capitalized(&forms.possessive)
            };
            let clause = fmt(
                fragment(content, "clause"),
                &[
                    &his,
                    &content.lore.aptitudes[aptitude.index()],
                    &word(n, "fades", "grows"),
                    &n.abs().to_string(),
                ],
            );
            out += &if any {
                fmt(fragment(content, "and_clause"), &[&clause])
            } else {
                clause
            };
            any = true;
        }
        if any {
            out += fragment(content, "sentence_end");
        }
    }
    let note = &content.lore.phases[phase.index()].note;
    out + &note
        .replace("{He}", &capitalized(&forms.subject))
        .replace("{his}", &forms.possessive)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_phase_effect_reads_as_section_21_quotes_it() {
        let content = crate::content::load().expect("the content loads");
        assert_eq!(
            phase_effect(&content, Phase::Youth, Pronoun::He),
            "He brings 1 less in every aptitude until he is 20. He learns fast: more from a \
             winter's training, and something from any quest that succeeds."
        );
        assert_eq!(
            phase_effect(&content, Phase::Prime, Pronoun::She),
            "She is at full strength."
        );
        assert_eq!(
            phase_effect(&content, Phase::Veteran, Pronoun::He),
            "His Might fades by 1 and his Wits grows by 1. He teaches well now."
        );
        assert_eq!(
            phase_effect(&content, Phase::Elder, Pronoun::She),
            "Her Might fades by 2 and her Wits grows by 1. She teaches well, and each winter \
             may be her last."
        );
    }

    #[test]
    fn a_bag_gives_every_name_once_before_any_twice() {
        let content = crate::content::load().expect("the content loads");
        let mut bags = Bags::new(&content);
        let mut rng = Rng::from_seed(5);
        let mut first: Vec<String> = (0..32).map(|_| bags.name(Pronoun::She, &mut rng)).collect();
        first.sort();
        let mut all = content.names.her.clone();
        all.sort();
        assert_eq!(first, all, "thirty-two draws are the thirty-two names");
        assert_eq!(bags.her.left(), 0);
        let again = bags.name(Pronoun::She, &mut rng);
        assert!(content.names.her.contains(&again));
        assert_eq!(bags.her.left(), 31, "an empty bag refills before the draw");
        let houses: std::collections::BTreeSet<String> =
            (0..16).map(|_| bags.houses.draw(&mut rng)).collect();
        assert_eq!(houses.len(), 16);
    }

    #[test]
    fn a_rolled_dream_is_never_one_another_living_hero_dreams_while_one_is_free() {
        let (content, heroes) = crate::testkit::founded();
        // The living founders dream QUIET_THE_BARROW, AVENGE_THE_LOST, SEE_THE_SEA,
        // WALK_EVERY_ROAD, FORGE_A_BLADE, WORTHY_STUDENT; free for a 20-year-old:
        // ROOF_OF_THE_WORLD, KNOWN_AT_COURT, SEE_A_CHILD_GROWN.
        let mut seen = std::collections::BTreeSet::new();
        for seed in 0..200 {
            let dream = rolled_dream(&content, &heroes, 20, &mut Rng::from_seed(seed));
            seen.insert(dream.kind);
        }
        assert_eq!(
            seen.into_iter().collect::<Vec<_>>(),
            [
                DreamKind::RoofOfTheWorld,
                DreamKind::KnownAtCourt,
                DreamKind::SeeAChildGrown
            ]
        );
    }

    #[test]
    fn the_worthy_student_is_rolled_from_thirty_five_and_everything_once_all_are_claimed() {
        let (content, mut heroes) = crate::testkit::founded();
        let odo = crate::testkit::id(&heroes, "Odo");
        heroes[odo].dream = None;
        let kinds = |age: i32, heroes: &[Hero]| -> std::collections::BTreeSet<DreamKind> {
            (0..300)
                .map(|seed| rolled_dream(&content, heroes, age, &mut Rng::from_seed(seed)).kind)
                .collect()
        };
        assert!(!kinds(34, &heroes).contains(&DreamKind::WorthyStudent));
        assert!(kinds(35, &heroes).contains(&DreamKind::WorthyStudent));
        // Every kind claimed: uniform over all eight.
        let n = heroes.len();
        for (i, kind) in content.wanderers.dreams.iter().enumerate() {
            heroes[i % n].dream = Some(Dream::build(&content, *kind, None, None).expect("builds"));
            heroes[i % n].fate = Fate::Living;
        }
        assert_eq!(kinds(40, &heroes).len(), 8);
    }
}
