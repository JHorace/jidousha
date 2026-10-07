//! The hero model (SPEC §3.2) and its derived quantities.
//!
//! A hero is plain data held in the house's `heroes` list in creation order, and
//! a hero's id is its index there (SPEC §0: ids 0, 1, 2... in creation order).
//! The derived quantities — phase, effective aptitude, adult, best aptitude, kin,
//! descent, firstborn — are free functions over that list, so the sheet, the
//! rules of later waves and the checks all ask the same function.

use crate::constants::{COMING_OF_AGE, PHASE_FROM_AGE, phase_adjustment};
use crate::dream::Dream;
use crate::ids::{Aptitude, BondKind, Destiny, LegacyKind, Phase, Place, Pronoun, Tag, Vocation};

/// A hero's id: its index in creation order.
pub type HeroId = usize;

/// A fear and where the hero stands with it.
#[derive(Clone, Debug, PartialEq)]
pub struct Fear {
    /// What is feared.
    pub tag: Tag,
    /// 0..=5.
    pub dread: i32,
    /// 0..=3.
    pub courage: i32,
    /// Courage reached 3.
    pub conquered: bool,
    /// Dread reached 5.
    pub broken: bool,
    /// Born to a parent who had conquered it.
    pub born_brave: bool,
}

/// A prophecy.
#[derive(Clone, Debug, PartialEq)]
pub struct DestinyState {
    /// Which.
    pub kind: Destiny,
    /// Whether it has come.
    pub fulfilled: bool,
    /// The original Door promisee's name, for an inherited Door destiny.
    pub blood_of: Option<String>,
}

/// One side of a bond. "A's bond to B is PARENT" means B is A's parent.
#[derive(Clone, Debug, PartialEq)]
pub struct Bond {
    /// What the other is to this hero.
    pub kind: BondKind,
    /// The other hero.
    pub other: HeroId,
    /// The year formed or last changed.
    pub since: i32,
    /// On the teacher's side: whether this hero taught the other.
    pub taught: bool,
    /// Successes shared on the road.
    pub shared_successes: i32,
}

/// An heirloom.
#[derive(Clone, Debug, PartialEq)]
pub struct Heirloom {
    /// "Thornfall".
    pub name: String,
    /// The sprite the original drew it with.
    pub sprite: String,
    /// What it adds to.
    pub aptitude: Aptitude,
    /// How much.
    pub bonus: i32,
    /// Where it came from.
    pub provenance: String,
}

/// Where a blessing applies (SPEC §6 line 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// On a quest carrying this tag.
    AgainstTag(Tag),
    /// On a quest at this place.
    AtPlace(Place),
    /// On every quest.
    Everywhere,
}

/// A blessing (SPEC §3.2, §14.1): its title, where it applies and how much it adds.
#[derive(Clone, Debug, PartialEq)]
pub struct Blessing {
    /// "Garrick's rest".
    pub title: String,
    /// Where it applies.
    pub scope: Scope,
    /// What it adds there.
    pub power: i32,
}

/// What a deed was. W2 writes the two fear deeds, W3 the three dream deeds, W6 the
/// summer's, W7 the winter's; later waves add their kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeedKind {
    /// Dread reached 5 (SPEC §10.4).
    Broken,
    /// Courage reached 3 (SPEC §10.3).
    ConqueredFear,
    /// A dream stage done, the dream not yet fulfilled (SPEC §9.3).
    DreamStep,
    /// A dream fulfilled (SPEC §9.4).
    DreamFulfilled,
    /// A legacy left (SPEC §14.1).
    LeftLegacy,
    /// The first quest ever (SPEC §7.1 step 4); weight is the quest's danger.
    FirstQuest,
    /// A triumph (SPEC §7.3); weight is the quest's danger.
    Triumph,
    /// Wounded on a quest (SPEC §7.4).
    Wounded,
    /// Lived through a disaster (SPEC §7.4).
    SurvivedDisaster,
    /// Broken and mended, as the Seer said (SPEC §7.4).
    Mended,
    /// Crowned (SPEC §7.4).
    Crowned,
    /// Companions became friends on the road (SPEC §12.2).
    Befriended,
    /// Laid a ghost (SPEC §14.4).
    LaidGhost,
    /// Taught for the first time (SPEC §11.5); `other` is the learner.
    Taught,
    /// Wed in the garden (SPEC §11.3); `other` is the spouse.
    Wed,
    /// Told the tale at the long table (SPEC §11.3).
    ToldTheTale,
    /// Given a dead hero's heirloom on their death page (SPEC §15.2); `other` is the dead.
    Inherited,
    /// Took up a dead hero's dream (SPEC §15.2); `other` is the dream's owner.
    TookUpDream,
    /// Had a child (SPEC §17.3); `other` is the child.
    ChildBorn,
    /// Came of age and heard the Seer (SPEC §17.5).
    CameOfAge,
    /// Failed a personal quest and marked the name (variant); weight is the quest's danger.
    Marked,
    /// Came to the house as a wanderer (SPEC §17.2), dated the year they arrive for.
    Arrived,
    /// Bore a lock of the Sealed Door open (SPEC §16.2); weight is the lock's index.
    /// W10 writes it; the epitaph reads it (§20 END).
    OpenedALock,
    /// Stood at the Sealed Door and lived (SPEC §16.2); weight is the locks opened.
    /// W10 writes it; the epitaph reads it (§20 PROPHECY, END).
    StoodAtTheDoor,
}

/// What became of a dead hero's own dream (SPEC §3.2 `bequest`): the death page
/// decides it (§15.1-§15.2), a ghost laid (§14.4) or taken up (§9.5) changes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DreamFate {
    /// Not decided yet.
    Undecided,
    /// No own dream to leave.
    NeverDreamt,
    /// The own dream was done.
    Fulfilled,
    /// The undone dream went to the heir (`Hero::bequest_heir`).
    PassedOn,
    /// No one took it up; its ghost walks.
    LeftToNoOne,
    /// Its ghost was laid, in `Hero::laid_year`.
    LaidToRest,
}

/// One entry in a hero's record of deeds (SPEC §3.2). The telling is never shown;
/// the epitaph (`epitaph.rs`) reads the deeds themselves.
#[derive(Clone, Debug, PartialEq)]
pub struct Deed {
    /// What it was.
    pub kind: DeedKind,
    /// The year it happened.
    pub year: i32,
    /// The hero's age then.
    pub age: i32,
    /// Where, when it happened somewhere.
    pub place: Option<Place>,
    /// How much it weighs (a quest's danger, a lock's index); 0 for the fear deeds.
    pub weight: i32,
    /// The other hero it concerns: the mourned, for a break by grief.
    pub other: Option<HeroId>,
    /// "was broken by deep water at the Drowned Coast".
    pub telling: String,
}

/// Living, dead, or departed (crowned).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fate {
    /// Still in the house.
    Living,
    /// Dead.
    Dead,
    /// Crowned and gone to Court.
    Departed,
}

/// A hero (SPEC §3.2). Fields later waves first write are present and at their defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct Hero {
    /// The content key (`household.json`), for founding heroes.
    pub key: String,
    /// First name.
    pub name: String,
    /// House name.
    pub house: String,
    /// Of the family: a founder, a child of the family, or an outsider who married in.
    /// An outsider is a wanderer who has not (variant: `VARIANT.md`).
    pub family: bool,
    /// He or she.
    pub pronoun: Pronoun,
    /// Calling; children carry the default Knight, invisibly.
    pub vocation: Vocation,
    /// Years.
    pub age: i32,
    /// Absolute year (year 1 is the first summer).
    pub born_year: i32,
    /// Base Might, Wits, Spirit, 0..=9.
    pub aptitudes: [i32; 3],
    /// Own dream; `None` is "not dreamt".
    pub dream: Option<Dream>,
    /// A carried, inherited dream.
    pub burden: Option<Dream>,
    /// Black marks on the name this hero carries (variant), in the order taken.
    pub marks: Vec<crate::marks::Mark>,
    /// Traits, at most one per aptitude, Might then Wits then Spirit (variant).
    pub traits: Vec<crate::ids::Trait>,
    /// The fear.
    pub fear: Fear,
    /// The prophecy.
    pub destiny: DestinyState,
    /// Bonds, in formation order.
    pub bonds: Vec<Bond>,
    /// At most one heirloom.
    pub heirloom: Option<Heirloom>,
    /// Blessings.
    pub blessings: Vec<Blessing>,
    /// Scars.
    pub scars: Vec<String>,
    /// The record of deeds, in the order they happened.
    pub deeds: Vec<Deed>,
    /// What a fulfilled dream left, and its telling.
    pub legacy: (LegacyKind, String),
    /// Personal renown.
    pub renown: i32,
    /// Wounded.
    pub wounded: bool,
    /// Settled: dread has no hold.
    pub settled: bool,
    /// Living, dead or departed.
    pub fate: Fate,
    /// The year of death or departure.
    pub fate_year: i32,
    /// The age at death or departure.
    pub fate_age: i32,
    /// "was lost at the Drowned Coast".
    pub fate_telling: String,
    /// Where a questing death happened.
    pub death_place: Option<Place>,
    /// What a questing death was against: the quest's first tag.
    pub death_tag: Option<Tag>,
    /// Grief already applied.
    pub grieved: bool,
    /// The bequest has been decided.
    pub bequest_decided: bool,
    /// The bequest's heir, once one is named.
    pub bequest_heir: Option<HeroId>,
    /// The heirloom the bequest records.
    pub bequest_heirloom: Option<String>,
    /// What became of the own dream.
    pub dream_fate: DreamFate,
    /// The year the hero's ghost was laid (SPEC §14.4).
    pub laid_year: Option<i32>,
    /// The epitaph's wording — its frame and its nine coins — once rolled (SPEC §20).
    pub wording: Option<crate::epitaph::Wording>,
    /// The epitaph, once composed (SPEC §20); the remembrance shows it (§19.2).
    pub epitaph: Option<String>,
    /// Up to two parents.
    pub parents: [Option<HeroId>; 2],
    /// Places quested at.
    pub roads_walked: Vec<Place>,
    /// Quests faced.
    pub quests_faced: i32,
    /// Fears faced.
    pub fears_faced: i32,
    /// Winters taught.
    pub winters_taught: i32,
}

impl Hero {
    /// "Name House".
    pub fn full_name(&self) -> String {
        format!("{} {}", self.name, self.house)
    }

    /// Living iff fate is LIVING.
    pub fn is_living(&self) -> bool {
        self.fate == Fate::Living
    }

    /// Adult iff age >= 12.
    pub fn is_adult(&self) -> bool {
        self.age >= COMING_OF_AGE
    }

    /// The phase this hero's age puts them in.
    pub fn phase(&self) -> Phase {
        phase_of(self.age)
    }

    /// Base aptitude.
    pub fn base(&self, aptitude: Aptitude) -> i32 {
        self.aptitudes[aptitude.index()]
    }

    /// Effective aptitude: base plus the phase adjustment, floored at 0.
    pub fn effective(&self, aptitude: Aptitude) -> i32 {
        (self.base(aptitude) + phase_adjustment(self.phase(), aptitude)).max(0)
    }

    /// Highest base value; ties go to the lower index.
    pub fn best_aptitude(&self) -> Aptitude {
        let mut best = Aptitude::Might;
        for aptitude in Aptitude::ALL {
            if self.base(*aptitude) > self.base(best) {
                best = *aptitude;
            }
        }
        best
    }

    /// This hero's bond to `other`, if any.
    pub fn bond_to(&self, other: HeroId) -> Option<&Bond> {
        self.bonds.iter().find(|bond| bond.other == other)
    }
}

/// The phase an age is in: Child <12, Youth 12-19, Prime 20-39, Veteran 40-54, Elder 55+.
pub fn phase_of(age: i32) -> Phase {
    let mut phase = Phase::Child;
    for candidate in Phase::ALL {
        if age >= PHASE_FROM_AGE[candidate.index()] {
            phase = *candidate;
        }
    }
    phase
}

/// Kin: a PARENT/CHILD bond between them, or a shared non-null parent. Nothing else.
pub fn kin(heroes: &[Hero], a: HeroId, b: HeroId) -> bool {
    let parent_bond = heroes[a]
        .bond_to(b)
        .is_some_and(|bond| matches!(bond.kind, BondKind::Parent | BondKind::Child));
    let shared_parent = heroes[a]
        .parents
        .iter()
        .flatten()
        .any(|parent| heroes[b].parents.contains(&Some(*parent)));
    parent_bond || shared_parent
}

/// Whether `hero` descends from `ancestor`, via parents, recursively.
pub fn descends_from(heroes: &[Hero], hero: HeroId, ancestor: HeroId) -> bool {
    heroes[hero]
        .parents
        .iter()
        .flatten()
        .any(|&parent| parent == ancestor || descends_from(heroes, parent, ancestor))
}

/// Among `hero`'s CHILD bonds (living or dead), the smallest born_year; first on ties.
pub fn firstborn(heroes: &[Hero], hero: HeroId) -> Option<HeroId> {
    let mut first: Option<HeroId> = None;
    for bond in heroes[hero]
        .bonds
        .iter()
        .filter(|b| b.kind == BondKind::Child)
    {
        if first.is_none_or(|f| heroes[bond.other].born_year < heroes[f].born_year) {
            first = Some(bond.other);
        }
    }
    first
}
