//! Trait genetics — the variant's rudimentary blood (DESIGN.md, "Genes").
//!
//! Mainline Keifu passes a share of aptitudes, a fear and blessings to a child
//! (SPEC §17.3) and nothing else of the blood; the variant gives every hero two
//! trait slots. A child takes one slot from each parent, each picked by a coin,
//! and one slot may mutate; a wanderer's slots are rolled. A trait in both slots
//! is true-bred and counts twice.
//!
//! INVARIANT: every gene roll draws from a per-hero sub-generator seeded from the
//! house's seed and the hero's id, never from the run's generator — so mainline's
//! roll inventory (SPEC §22.2) is untouched and a mainline seed replays the same
//! births, deaths and arrivals in the variant. (Variant design decision, recorded
//! in `tools/yakin/runs/keifu-x-inheritance-r3v1/DESIGN.md`.)

use jidousha::prelude::Rng;

use crate::chance::{chance, index};
use crate::fear::fear_power;
use crate::hero::{Hero, HeroId};
use crate::ids::Aptitude;
use crate::power::QuestFacts;

/// One heritable trait.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trait {
    /// +1 power per copy on a Might quest.
    Strong,
    /// +1 power per copy on a Wits quest.
    Clever,
    /// +1 power per copy on a Spirit quest.
    Devout,
    /// An unconquered fear's penalty is 1 less per copy (never below 0).
    Bold,
}

impl Trait {
    /// Every trait, in table order.
    pub const ALL: &'static [Trait] = &[Trait::Strong, Trait::Clever, Trait::Devout, Trait::Bold];

    /// "Strong".
    pub fn word(self) -> &'static str {
        match self {
            Trait::Strong => "Strong",
            Trait::Clever => "Clever",
            Trait::Devout => "Devout",
            Trait::Bold => "Bold",
        }
    }

    /// The aptitude a trait adds to, for the three that add to one.
    fn aptitude(self) -> Option<Aptitude> {
        match self {
            Trait::Strong => Some(Aptitude::Might),
            Trait::Clever => Some(Aptitude::Wits),
            Trait::Devout => Some(Aptitude::Spirit),
            Trait::Bold => None,
        }
    }
}

/// Two slots; `None` carries nothing.
pub type Genes = [Option<Trait>; 2];

/// The chance a newborn's genes mutate one slot.
pub const MUTATION_CHANCE: f32 = 0.15;
/// The chance each of a wanderer's slots carries a trait.
pub const WANDERER_TRAIT_CHANCE: f32 = 0.5;

/// How many copies of `t` the hero carries (0, 1, or 2 when true-bred).
pub fn copies(hero: &Hero, t: Trait) -> i32 {
    hero.genes.iter().filter(|g| **g == Some(t)).count() as i32
}

/// The distinct traits a hero shows, in slot order.
pub fn traits(genes: &Genes) -> Vec<Trait> {
    let mut out = Vec::new();
    for t in genes.iter().flatten() {
        if !out.contains(t) {
            out.push(*t);
        }
    }
    out
}

/// "Strong, Bold" — or "Strong x2" for a true-bred trait; "none" when empty.
pub fn traits_word(genes: &Genes) -> String {
    let shown: Vec<String> = traits(genes)
        .into_iter()
        .map(|t| {
            if genes.iter().filter(|g| **g == Some(t)).count() == 2 {
                format!("{} x2", t.word())
            } else {
                t.word().to_owned()
            }
        })
        .collect();
    if shown.is_empty() {
        "none".to_owned()
    } else {
        shown.join(", ")
    }
}

/// What the blood adds to one member's power on `quest`: +1 per copy of the
/// quest's aptitude trait, and Bold taking up to one point per copy off an
/// unconquered fear's penalty. Read by `power::member_power` before its floor.
pub fn trait_power(hero: &Hero, quest: QuestFacts<'_>) -> i32 {
    let mut sum = 0;
    for t in Trait::ALL {
        if t.aptitude() == Some(quest.aptitude) {
            sum += copies(hero, *t);
        }
    }
    let fear = fear_power(hero, quest.tags);
    if fear < 0 {
        sum += copies(hero, Trait::Bold).min(-fear);
    }
    sum
}

/// The sub-generator for hero `id`'s gene rolls (see the module's DELIBERATE note).
pub fn gene_rng(house_seed: u64, id: HeroId) -> Rng {
    let mixed = house_seed ^ (id as u64 + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    Rng::from_seed(mixed)
}

/// A newborn's genes: one slot from each parent (each by a coin), then a
/// `MUTATION_CHANCE` that a coin-picked slot becomes a uniform trait.
pub fn child_genes(house_seed: u64, child: HeroId, first: &Genes, second: &Genes) -> Genes {
    let mut rng = gene_rng(house_seed, child);
    let mut genes = [first[index(&mut rng, 2)], second[index(&mut rng, 2)]];
    if chance(&mut rng, MUTATION_CHANCE) {
        let slot = index(&mut rng, 2);
        genes[slot] = Some(Trait::ALL[index(&mut rng, Trait::ALL.len())]);
    }
    genes
}

/// A wanderer's genes: each slot a uniform trait with `WANDERER_TRAIT_CHANCE`.
pub fn wanderer_genes(house_seed: u64, id: HeroId) -> Genes {
    let mut rng = gene_rng(house_seed, id);
    let slot = |rng: &mut Rng| {
        chance(rng, WANDERER_TRAIT_CHANCE).then(|| Trait::ALL[index(rng, Trait::ALL.len())])
    };
    let a = slot(&mut rng);
    let b = slot(&mut rng);
    [a, b]
}

/// The founders' authored genes: only the two founding children carry any, so no
/// founding adult's power — and none of mainline's founding oracles — moves.
pub fn founding_genes(name: &str) -> Genes {
    match name {
        "Pip" => [Some(Trait::Strong), None],
        "Wren" => [Some(Trait::Bold), Some(Trait::Clever)],
        _ => [None, None],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_child_takes_one_slot_from_each_parent_or_a_mutation() {
        let a: Genes = [Some(Trait::Strong), Some(Trait::Strong)];
        let b: Genes = [Some(Trait::Clever), Some(Trait::Clever)];
        for id in 0..200 {
            let g = child_genes(42, id, &a, &b);
            let from_a = g[0] == Some(Trait::Strong);
            let from_b = g[1] == Some(Trait::Clever);
            // At most one slot mutates.
            assert!(from_a || from_b, "{g:?}");
        }
    }

    #[test]
    fn gene_rolls_replay_on_the_same_seed_and_id() {
        let a: Genes = [Some(Trait::Strong), None];
        let b: Genes = [Some(Trait::Bold), Some(Trait::Devout)];
        assert_eq!(child_genes(7, 12, &a, &b), child_genes(7, 12, &a, &b));
        assert_eq!(wanderer_genes(7, 12), wanderer_genes(7, 12));
    }

    #[test]
    fn a_true_bred_trait_is_shown_twice_over() {
        assert_eq!(
            traits_word(&[Some(Trait::Strong), Some(Trait::Strong)]),
            "Strong x2"
        );
        assert_eq!(traits_word(&[None, None]), "none");
    }
}
