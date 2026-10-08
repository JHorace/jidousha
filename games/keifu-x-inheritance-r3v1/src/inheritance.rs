//! What an heir takes on a death page — the variant's reading of the choice
//! (DESIGN.md, "Inheritance").
//!
//! Mainline Keifu's heir buttons say only how the heir is kin to the dead and
//! whether they can take the dream or must lay an heirloom aside (SPEC §15.1). The
//! variant's heir also takes on half the dead's black marks, and carries the blood
//! they were born with; the button shows both — what the heir will hold once
//! chosen — and the choice applies exactly that.
//!
//! `bequeathed` is the one function: `heirs::heir_buttons` shows it and
//! `heirs::choose` applies it.

use crate::genes::{Genes, traits_word};
use crate::hero::{Hero, HeroId};
use crate::marks::DECAY_DIVISOR;

/// What an heir holds after the choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Inherited {
    /// The heir's blood (succession does not change it).
    pub genes: Genes,
    /// The heir's marks after taking on half the dead's.
    pub marks: i32,
}

/// What `heir` holds once chosen as `dead`'s heir.
pub fn bequeathed(heroes: &[Hero], dead: HeroId, heir: HeroId) -> Inherited {
    Inherited {
        genes: heroes[heir].genes,
        marks: heroes[heir].marks + heroes[dead].marks / DECAY_DIVISOR,
    }
}

/// The button's second line: "holds Strong, Bold; marks 2".
pub fn reading(inherited: Inherited) -> String {
    format!(
        "holds {}; marks {}",
        traits_word(&inherited.genes),
        inherited.marks
    )
}
