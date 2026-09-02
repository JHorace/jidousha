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
//! **The cast is `CAST.md`** — the founding band of ten, their homes, their
//! sheets and their source lines, landed by wave 1.1 with the vocabulary they
//! wear. What a character *decides* is `autonomy.rs`; every one of them fields
//! a one-person party, and that party is the only way anybody moves.

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

/// The founding band (`CAST.md` §4): ten, the number GDD §7's MVP scenario
/// asks for, drawn from everywhere on purpose.
///
/// Homes are the tents on the near bank of the Kawaza crossing — two rows just
/// south of the road and east of the ford, far enough apart that ten names,
/// ten figures and the town's own marker do not collide. `floors.rs` asserts that rather than trusting this sentence,
/// and [`registry`] asserts the ground is passable, unshared, and off every
/// named location.
pub fn roster() -> Vec<Character> {
    vec![
        Character {
            id: "bob",
            name: "Bob",
            home: Tile::new(12, 15),
            icon: Art::PortraitBob,
            traits: vec![TraitId::Fight, TraitId::Greedy, TraitId::Indebted],
            wallet: 6,
            desperation: 4,
            source: "owes the collector at the toll-house, who counts days",
            active_petition: None,
        },
        Character {
            id: "steve",
            name: "Steve",
            home: Tile::new(16, 15),
            icon: Art::PortraitSteve,
            traits: vec![TraitId::Labor, TraitId::Loyal, TraitId::Caring],
            wallet: 3,
            desperation: 5,
            source: "sends half of everything to a sister whose hands gave out",
            active_petition: None,
        },
        Character {
            id: "alex",
            name: "Alex",
            home: Tile::new(20, 15),
            icon: Art::PortraitAlex,
            traits: vec![TraitId::Scout, TraitId::Cold, TraitId::Restless],
            wallet: 12,
            desperation: 1,
            source: "has not slept a full month in one place since childhood",
            active_petition: None,
        },
        Character {
            id: "tim",
            name: "Tim",
            home: Tile::new(24, 15),
            icon: Art::PortraitTim,
            traits: vec![TraitId::Labor, TraitId::Proud, TraitId::Vengeful],
            wallet: 20,
            desperation: 2,
            source: "keeps the tally, and is owed by half the camp",
            active_petition: None,
        },
        Character {
            id: "rin",
            name: "Rin",
            home: Tile::new(28, 15),
            icon: Art::PortraitRin,
            traits: vec![TraitId::Craft, TraitId::Maker, TraitId::Loyal],
            wallet: 5,
            desperation: 2,
            source: "cooks for ten on a fire built for three",
            active_petition: None,
        },
        Character {
            id: "goro",
            name: "Goro",
            home: Tile::new(6, 17),
            icon: Art::PortraitGoro,
            traits: vec![TraitId::Fight, TraitId::Renown, TraitId::Proud],
            wallet: 9,
            desperation: 3,
            source: "left home to be talked about, and nobody is talking yet",
            active_petition: None,
        },
        Character {
            id: "hana",
            name: "Hana",
            home: Tile::new(10, 17),
            icon: Art::PortraitHana,
            traits: vec![TraitId::Scout, TraitId::Caring, TraitId::Vengeful],
            wallet: 7,
            desperation: 2,
            source: "came for her brother; stays exactly as long as he does",
            active_petition: None,
        },
        Character {
            id: "ludo",
            name: "Ludo",
            home: Tile::new(14, 17),
            icon: Art::PortraitLudo,
            traits: vec![TraitId::Labor, TraitId::Fight, TraitId::Indebted],
            wallet: 2,
            desperation: 4,
            source: "works off a debt that was his father's before it was his",
            active_petition: None,
        },
        Character {
            id: "ines",
            name: "Ines",
            home: Tile::new(18, 17),
            icon: Art::PortraitInes,
            traits: vec![TraitId::Craft, TraitId::Maker, TraitId::Craven],
            wallet: 10,
            desperation: 2,
            source: "mends what breaks, and would rather be far from what breaks it",
            active_petition: None,
        },
        Character {
            id: "odd",
            name: "Odd",
            home: Tile::new(22, 17),
            icon: Art::PortraitOdd,
            traits: vec![TraitId::Fight, TraitId::Renown, TraitId::Restless],
            wallet: 8,
            desperation: 3,
            source: "took the same job as Goro twice, and only one of them got paid",
            active_petition: None,
        },
    ]
}

/// The personalities `CAST.md` §3.3 **parks**: kept as rows of the vocabulary
/// because the trait x mark reaction table references them and stays whole,
/// and worn by nobody until marks are common — the betrayal ladder's era
/// (asks, wave 2, and after).
///
/// Declared here rather than as a field on the trait row, because being on a
/// sheet is a casting decision and not a property of the word.
pub const PARKED: &[TraitId] = &[TraitId::Pious, TraitId::Pragmatic, TraitId::Upright];

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
    // **The coverage matrix** (`CAST.md` §7), asserted as data so a future
    // edit that breaks it is caught at the row that caused it rather than in
    // a playtest: every aptitude and every motivator on at least two sheets,
    // every kept personality on at least one, every parked personality on
    // none.
    let carries = |id: crate::traits::TraitId| {
        cast.iter()
            .filter(|person| person.traits.contains(&id))
            .count()
    };
    for def in crate::traits::TRAITS {
        let sheets = carries(def.id);
        let (floor, why) = match def.kind {
            crate::traits::TraitKind::Aptitude | crate::traits::TraitKind::Motivator => (
                2usize,
                "an aptitude or a want on one sheet is a word the cast cannot compare",
            ),
            crate::traits::TraitKind::Personality => (
                usize::from(!PARKED.contains(&def.id)),
                "a kept personality nobody carries is a chip the player never meets",
            ),
        };
        checks.require(
            sheets >= floor,
            "a trait is on fewer sheets than the coverage matrix allows",
            format!(
                "{:?} is a {} carried by {sheets} of the ten and CAST.md §7 wants at least \
                 {floor}: {why}",
                def.id,
                def.kind.name()
            ),
        );
    }
    for id in PARKED.iter().copied() {
        checks.require(
            carries(id) == 0,
            "a parked personality is on somebody's sheet",
            format!(
                "{id:?} is carried by {} of the ten; CAST.md §3.3 parks it until marks are \
                 common, and a parked row is in the vocabulary and on no sheet",
                carries(id)
            ),
        );
        checks.require(
            crate::traits::TRAITS.iter().any(|def| def.id == id),
            "a parked personality was deleted from the vocabulary instead of parked",
            format!(
                "{id:?} has no row; the trait x mark reaction table references it and stays \
                 whole"
            ),
        );
    }
    // Every party's face is its member's face: a token on the road and a
    // figure at a doorstep are the same person, or the map is telling two
    // stories about one name. One party per character, in registry order —
    // the one-person party the dispatch loop moves (`sim.rs`).
    let sim = crate::sim::Sim::opening(tuning, crate::modules::ModuleSet::ALL);
    checks.require(
        sim.parties.len() == cast.len(),
        "the settlement does not field one party per person",
        format!(
            "{} parties over {} people; every character fields their own one-person party, \
             which is how anybody moves at all",
            sim.parties.len(),
            cast.len()
        ),
    );
    for (index, party) in sim.parties.iter().enumerate() {
        match sim.people.get(party.member) {
            Some(person) => checks.require(
                party.token == person.icon && party.member == index && party.name == person.name,
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
