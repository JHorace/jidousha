//! The character registry: who lives in the settlement (GDD §3).
//!
//! Ported from giri mainline's `model.rs` by copy-adapt — read beside it, not
//! depended on. What changed in the port is where the state lives: giri made
//! every character an entity and every field a component, because giri's whole
//! world was four people and a beat. ninjo's people live in the one `Sim`
//! resource beside the parties and the scheduler, for the reason the substrate
//! keeps everything else there — **replay is the contract** (GDD §1), and a
//! store outside sim state is a store a replay silently does not carry.
//!
//! What a character *has* is here; what a character *is* is `traits.rs`; what
//! everyone knows about them and what they think of each other is `stores.rs`.
//! Every screen reads all three through `lens.rs` and nothing else.
//!
//! **Nobody decides anything yet.** Autonomy is wave 1: in wave 0b a character
//! stands at their home tile, holds a wallet, carries a desperation with the
//! `source` line that says why it presses, and waits. The one thing that moves
//! them is being the member a party fields, which is the substrate's dispatch
//! loop wearing a name.

use crate::grid::Tile;
use crate::sprites::Art;
use crate::traits::TraitId;

/// One person in the settlement.
///
/// **The `source` line is the proven differentiator** (giri's DESIGN §3, borne
/// out in play): two characters at desperation 5 are two different problems,
/// and the sentence that says why is what makes them read as people rather
/// than as a number. It is bound at generation and never edited afterwards.
#[derive(Clone, Debug)]
pub struct Character {
    /// The id a log line, a link or a save names them by. ASCII, lowercase.
    pub id: &'static str,
    /// The name, as the map and the sheets draw it.
    pub name: &'static str,
    /// Where they live. Characters stand here when nothing has them out.
    pub home: Tile,
    /// Which portrait role draws them.
    pub icon: Art,
    /// Who they are, what they want, and what they can do — any number of
    /// rows from the vocabulary, of any kind. There is no cap (GDD §3).
    pub traits: Vec<TraitId>,
    /// What they have. Gold, the only v1 currency; conserved between holders
    /// and the treasury (GDD §4.1).
    pub wallet: i64,
    /// Need. The opener of every willingness sum, and the motive behind every
    /// bad decision the later waves will let them make.
    pub desperation: i64,
    /// Why the need presses — bound at generation, never empty.
    pub source: &'static str,
    /// The petition this character is currently carrying, if any.
    ///
    /// The slot GDD §3 asks foundation for; the ledger that fills it is the
    /// petitions module (wave 1.3). Wave 0b opens every character with it
    /// empty and asserts so — an occupied slot before petitions exist would
    /// mean something wrote through a door nobody has built yet.
    pub active_petition: Option<usize>,
}

/// The authored cast (GDD §7 wants 8-12 at the MVP scenario; wave 0b ships
/// four, one per committed portrait role, and the cast's *content* is an open
/// ledger item alongside the trait list — GDD §10).
///
/// Homes are spread around Ebisu on passable ground, far enough apart that
/// four names and the town's own label do not collide; `floors.rs` asserts
/// that rather than trusting this sentence.
pub fn roster() -> Vec<Character> {
    vec![
        Character {
            id: "alex",
            name: "Alex",
            home: Tile::new(3, 13),
            icon: Art::PortraitAlex,
            traits: vec![TraitId::Pragmatic, TraitId::Deft],
            wallet: 12,
            desperation: 1,
            source: "owes the crypt-keeper for a winter's grain",
            active_petition: None,
        },
        Character {
            id: "bob",
            name: "Bob",
            home: Tile::new(12, 13),
            icon: Art::PortraitBob,
            traits: vec![TraitId::Greedy, TraitId::Ambitious],
            wallet: 8,
            desperation: 3,
            source: "means to buy the mill, and is not close",
            active_petition: None,
        },
        Character {
            id: "steve",
            name: "Steve",
            home: Tile::new(17, 16),
            icon: Art::PortraitSteve,
            traits: vec![TraitId::Loyal, TraitId::Provider, TraitId::Strong],
            wallet: 4,
            desperation: 5,
            source: "feeds a sister whose hands no longer work",
            active_petition: None,
        },
        Character {
            id: "tim",
            name: "Tim",
            home: Tile::new(23, 16),
            icon: Art::PortraitTim,
            traits: vec![TraitId::Cold, TraitId::Upright, TraitId::Learned],
            wallet: 20,
            desperation: 2,
            source: "keeps the tally and is owed by half the town",
            active_petition: None,
        },
    ]
}

/// The registry's own validation — the authoring claims prose cannot hold.
///
/// Everything here is a fact about the shipped roster rather than about the
/// type: a home off the map or on water, two people with one id, a trait the
/// vocabulary does not have, an empty `source`. Each of them draws or reads
/// wrong somewhere far from the row that caused it.
pub fn registry(checks: &mut crate::checks::Checks, tuning: &crate::constants::Tuning) {
    let grid = crate::grid::grid();
    let cast = roster();
    checks.require(
        !cast.is_empty(),
        "the settlement has nobody in it",
        "people::roster is empty; ninjo is a game about people".to_owned(),
    );
    for (index, person) in cast.iter().enumerate() {
        checks.require(
            !person.id.is_empty()
                && person
                    .id
                    .chars()
                    .all(|glyph| glyph.is_ascii_lowercase() || glyph == '-'),
            "a character's id is not stamp-shaped ASCII",
            format!("roster[{index}] is {:?}", person.id),
        );
        checks.require(
            cast.iter().filter(|other| other.id == person.id).count() == 1,
            "two characters share an id",
            format!("{:?} appears more than once in the roster", person.id),
        );
        checks.require(
            !person.name.is_empty()
                && person
                    .name
                    .chars()
                    .all(|glyph| (' '..='~').contains(&glyph)),
            "a character's name is not printable ASCII",
            format!("{:?} is named {:?}", person.id, person.name),
        );
        checks.require(
            !person.source.is_empty()
                && person
                    .source
                    .chars()
                    .all(|glyph| (' '..='~').contains(&glyph)),
            "a character's desperation has no source",
            format!(
                "{:?}'s source is {:?}; GDD §3 binds one at generation - two identical \
                 desperations are two different problems, and this line is the difference",
                person.id, person.source
            ),
        );
        checks.require(
            grid.find(person.home)
                .is_some_and(|kind| kind.cost(tuning).is_some()),
            "a character lives on a tile nothing can stand on",
            format!(
                "{:?} lives at ({}, {}), which is {:?}",
                person.id,
                person.home.x,
                person.home.y,
                grid.find(person.home)
            ),
        );
        checks.require(
            cast.iter()
                .filter(|other| other.home == person.home)
                .count()
                == 1,
            "two characters live on one tile",
            format!(
                "({}, {}) is home to more than one person, and they would be drawn on top \
                 of each other",
                person.home.x, person.home.y
            ),
        );
        checks.require(
            crate::grid::location_at(person.home).is_none(),
            "a character lives on a named location's tile",
            format!(
                "{:?} lives at ({}, {}), where a marker already stands",
                person.id, person.home.x, person.home.y
            ),
        );
        for (slot, id) in person.traits.iter().enumerate() {
            checks.require(
                !person.traits[..slot].contains(id),
                "a character carries the same trait twice",
                format!("{:?} repeats {id:?}", person.id),
            );
            checks.require(
                crate::traits::TRAITS.iter().any(|def| def.id == *id),
                "a character carries a trait the vocabulary does not have",
                format!("{:?} carries {id:?}", person.id),
            );
        }
        checks.require(
            person.active_petition.is_none(),
            "a character opens the scenario already carrying a petition",
            format!(
                "{:?} opens with petition {:?}, and the petitions module is wave 1.3 - \
                 something wrote through a door nobody has built",
                person.id, person.active_petition
            ),
        );
        checks.require(
            person.wallet >= 0 && person.desperation >= 0,
            "a character opens with a negative wallet or desperation",
            format!(
                "{:?} opens at {}g and desperation {}",
                person.id, person.wallet, person.desperation
            ),
        );
    }
    // Every party's face is its member's face: a token on the road and a
    // figure at a doorstep are the same person, or the map is telling two
    // stories about one name.
    let sim = crate::sim::Sim::opening(tuning);
    for party in &sim.parties {
        match sim.people.get(party.member) {
            Some(person) => checks.require(
                party.token == person.icon,
                "a party's token is not its member's own portrait",
                format!(
                    "{} draws {:?} and its member {:?} draws {:?}",
                    party.name, party.token, person.id, person.icon
                ),
            ),
            None => checks.require(
                false,
                "a party is fielded by somebody who is not in the registry",
                format!("{} names member index {}", party.name, party.member),
            ),
        }
    }
}
