//! The knowledge lens: the one read-path from the simulation to a screen
//! (GDD §1, §3).
//!
//! # The rule
//!
//! **No screen may read simulation state except through this module.** Not
//! `world.resource::<Sim>()` in a Draw system, not a `&Sim` threaded into a
//! panel builder, not a field borrowed "just for the label". If you are about
//! to write a screen that reaches past this file, that is the failure this
//! seam exists to prevent — stop and add the accessor here instead.
//!
//! # Why, since v1 is the identity
//!
//! Because the sim runs on truth and the player does not see truth (GDD §1).
//! Today every accessor below hands back exactly what the store holds; the
//! knowledge module (GDD §5, post-MVP) makes that conditional — what a screen
//! may show becomes a function of what the player has been in a position to
//! learn, with regard as the information network. When that lands, **this file
//! changes and no screen does**. A screen that read around the lens would keep
//! showing the truth, and the bug would be invisible: the number would be
//! right, and it would be a number the player had no way to know.
//!
//! That is why the seam is worth a whole module while it does nothing. It is
//! cheap now and it is a rewrite of every surface later.
//!
//! # Who may read around it
//!
//! `--verify` and the sweep. They are the simulation's own instruments and
//! they assert on truth by definition; a check that could only see what the
//! player sees could not catch the sim lying to the player. Everything that
//! *draws* goes through here — `screens.rs` and the panel it builds take a
//! [`Lens`] and no `Sim` at all, which is the structural half of the rule that
//! this comment is the readable half of.

use crate::attention::{Attention, Pause};
use crate::constants::Tuning;
use crate::grid::Tile;
use crate::people::Character;
use crate::sim::{Activity, Event, Party, Sim, Site};
use crate::stores::{FactSet, Regarded};
use crate::traits::{self, MarkId, TraitId};

/// One view of the world, for one observer.
///
/// v1 is the identity: the observer is omniscient and there is nothing to
/// parameterize. The knowledge module gives this a subject and turns every
/// accessor below into a question about what that subject knows; the borrow
/// and the call sites are already shaped for it.
#[derive(Clone, Copy, Debug)]
pub struct Lens<'a> {
    sim: &'a Sim,
}

impl<'a> Lens<'a> {
    /// The view onto this world. **v1 = identity** (GDD §3).
    pub fn on(sim: &'a Sim) -> Self {
        Self { sim }
    }

    // ── the world ────────────────────────────────────────────────────────

    /// What the treasury holds.
    pub fn treasury(&self) -> i64 {
        self.sim.treasury
    }

    /// Every party, in authored order.
    pub fn parties(&self) -> &'a [Party] {
        &self.sim.parties
    }

    /// One party, if there is one at this index.
    pub fn party(&self, index: usize) -> Option<&'a Party> {
        self.sim.parties.get(index)
    }

    /// Every quest site, in `LOCATIONS` order (sans the town).
    pub fn sites(&self) -> &'a [Site] {
        &self.sim.sites
    }

    /// How many quests the site standing on `location` still has open.
    pub fn open_quests(&self, location: usize) -> usize {
        self.sim
            .sites
            .iter()
            .find(|site| site.location == location)
            .map_or(0, |site| site.quests.len() - site.claimed)
    }

    /// Everything that has happened, in firing order.
    pub fn events(&self) -> &'a [Event] {
        &self.sim.events
    }

    // ── the attention architecture (GDD §3) ──────────────────────────────

    /// What each class of event currently does to the world.
    ///
    /// Read through the lens like everything else, because it is sim state:
    /// the config panel shows what the *simulation* will do, never a copy of
    /// it kept beside the screen.
    pub fn attention(&self) -> &'a Attention {
        &self.sim.attention
    }

    /// Why the world stopped itself, if it did.
    pub fn pause(&self) -> Option<Pause> {
        self.sim.paused_by
    }

    /// How many times the world has stopped itself.
    pub fn pauses(&self) -> u64 {
        self.sim.pauses
    }

    // ── the people ───────────────────────────────────────────────────────

    /// Everyone in the settlement, in registry order.
    pub fn people(&self) -> &'a [Character] {
        &self.sim.people
    }

    /// One character, if there is one at this index.
    pub fn person(&self, index: usize) -> Option<&'a Character> {
        self.sim.people.get(index)
    }

    /// A character's name, for a line that has to say who.
    pub fn name(&self, index: usize) -> &'a str {
        self.person(index).map_or("someone", |person| person.name)
    }

    /// What this character has in their purse (GDD §4.1).
    pub fn wallet(&self, index: usize) -> i64 {
        self.person(index).map_or(0, |person| person.wallet)
    }

    /// How hard the need presses on them.
    pub fn desperation(&self, index: usize) -> i64 {
        self.person(index).map_or(0, |person| person.desperation)
    }

    /// Why it presses — the sentence that makes two identical desperations two
    /// different problems (GDD §3).
    pub fn source(&self, index: usize) -> &'a str {
        self.person(index).map_or("", |person| person.source)
    }

    /// What they carry, in the order the registry authored it.
    pub fn traits(&self, index: usize) -> &'a [TraitId] {
        self.person(index).map_or(&[], |person| &person.traits)
    }

    /// Where they live.
    pub fn home(&self, index: usize) -> Option<Tile> {
        self.person(index).map(|person| person.home)
    }

    /// What they are doing right now, as the character panel says it.
    ///
    /// Derived from the same facts the map draws them with: a character is at
    /// home unless a party they field is out, and then they are doing what
    /// that party is doing. Wave 1's autonomy gives them something of their
    /// own to be doing and this answer grows a term; the panel does not.
    pub fn activity_line(&self, index: usize) -> String {
        match self
            .sim
            .parties
            .iter()
            .find(|party| party.member == index && party.activity != Activity::Idle)
        {
            Some(party) => format!("out with {} - {}", party.name, party.status()),
            None => "at home, and nobody has asked them for anything".to_owned(),
        }
    }

    /// Whether this character is standing at their home tile.
    ///
    /// Derived, never stored: a character is at home unless a party they field
    /// is out. Wave 1's autonomy gives them somewhere else to be and this
    /// answer grows a second term; nothing that draws them has to change.
    pub fn at_home(&self, index: usize) -> bool {
        !self
            .sim
            .parties
            .iter()
            .any(|party| party.member == index && party.activity != Activity::Idle)
    }

    // ── the shared state (GDD §4) ────────────────────────────────────────

    /// `regard(from -> to)`, raw. Absent is zero.
    pub fn regard(&self, from: usize, to: Regarded) -> i64 {
        self.sim.shared.regard(from, to)
    }

    /// The same edge **as `from` feels it**: weighed by their personality's
    /// bond and grudge multipliers (`traits::weighted_regard`).
    ///
    /// The two are different numbers on purpose. A sheet shows what somebody
    /// feels; an economy report shows what is stored. Both come from here so
    /// neither can be computed a second way on a surface.
    pub fn regard_as_felt(&self, from: usize, to: Regarded) -> i64 {
        let raw = self.regard(from, to);
        match self.person(from) {
            Some(person) => traits::weighted_regard(raw, &person.traits),
            None => raw,
        }
    }

    /// What this pair holds — bond, grudge, both, or neither.
    pub fn facts(&self, from: usize, to: Regarded) -> FactSet {
        self.sim.shared.facts(from, to)
    }

    /// What everyone knows about `who`.
    pub fn marks(&self, who: usize) -> Vec<MarkId> {
        self.sim.shared.marks(who)
    }

    /// What `looker` makes of `subject`'s marks: the trait-filtered sum over
    /// the reaction table.
    pub fn reaction(&self, tuning: &Tuning, looker: usize, subject: usize) -> i64 {
        let Some(person) = self.person(looker) else {
            return 0;
        };
        self.marks(subject)
            .into_iter()
            .map(|mark| traits::reaction_to(tuning, &person.traits, mark))
            .sum()
    }

    /// How many times regard has drifted since the scenario opened.
    pub fn drifts(&self) -> u64 {
        self.sim.shared.drifts()
    }
}

/// **The lens is the identity in v1** (GDD §3), asserted rather than assumed.
///
/// Every accessor is checked against the store it reads, over a world that has
/// been written into — so the day the knowledge module makes a read
/// conditional, this battery is what says which reads changed and gives the
/// change somewhere to be recorded. It is also the check that a lens built
/// over a `Sim` is not quietly a copy of one: it is asserted against the same
/// `Sim` it was built from, after that `Sim` has been mutated.
pub fn identity(checks: &mut crate::checks::Checks, tuning: &Tuning) {
    let mut sim = Sim::opening(tuning);
    // Write something into every store, through the write API, so the
    // accessors have more than zeroes to agree about.
    let (a, b) = (0usize, 1usize);
    sim.shared.adjust_regard(tuning, a, Regarded::Person(b), 4);
    sim.shared.adjust_regard(tuning, b, Regarded::Person(a), -3);
    sim.shared.adjust_regard(tuning, a, Regarded::Player, 2);
    sim.shared.record_grudge(
        tuning,
        b,
        Regarded::Person(a),
        crate::stores::GrudgeCause::Betrayal,
    );
    sim.shared.write_mark(b, MarkId::Skimmer);
    sim.shared.drift(tuning);
    sim.treasury = 137;
    // And into the attention state, which is sim state for the same reason
    // the stores are: a replay carries it.
    sim.attention.set(
        crate::attention::EventClass::Departed,
        crate::attention::Mode::PauseAndFocus,
    );
    sim.paused_by = Some(crate::attention::Pause {
        event: 0,
        class: crate::attention::EventClass::Departed,
        minute: 41,
    });
    sim.pauses = 3;

    let lens = Lens::on(&sim);
    checks.require(
        lens.treasury() == sim.treasury
            && lens.parties().len() == sim.parties.len()
            && lens.sites().len() == sim.sites.len()
            && lens.events().len() == sim.events.len()
            && lens.people().len() == sim.people.len()
            && lens.drifts() == sim.shared.drifts(),
        "the knowledge lens does not hand back what the world holds",
        format!(
            "the lens reads {}g over {} parties, {} sites, {} events, {} people and {} \
             drifts; the world holds {}g over {}, {}, {}, {} and {}",
            lens.treasury(),
            lens.parties().len(),
            lens.sites().len(),
            lens.events().len(),
            lens.people().len(),
            lens.drifts(),
            sim.treasury,
            sim.parties.len(),
            sim.sites.len(),
            sim.events.len(),
            sim.people.len(),
            sim.shared.drifts()
        ),
    );
    // The attention architecture reads through the lens like everything else:
    // the config panel shows what the *simulation* will do.
    checks.require(
        lens.attention() == &sim.attention
            && lens.pause() == sim.paused_by
            && lens.pauses() == sim.pauses,
        "the knowledge lens does not hand back the attention state the world holds",
        format!(
            "the lens reads {} / {:?} / {} pauses and the world holds {} / {:?} / {}",
            lens.attention().stamp(),
            lens.pause(),
            lens.pauses(),
            sim.attention.stamp(),
            sim.paused_by,
            sim.pauses
        ),
    );
    for who in 0..sim.people.len() {
        let person = &sim.people[who];
        checks.require(
            lens.wallet(who) == person.wallet
                && lens.desperation(who) == person.desperation
                && lens.source(who) == person.source
                && lens.traits(who) == person.traits.as_slice()
                && lens.home(who) == Some(person.home),
            "the knowledge lens does not hand back what a character has",
            format!(
                "{:?} reads {}g / desperation {} / {:?} through the lens and {}g / {} / {:?} \
                 in the registry",
                person.id,
                lens.wallet(who),
                lens.desperation(who),
                lens.traits(who),
                person.wallet,
                person.desperation,
                person.traits
            ),
        );
        checks.require(
            !lens.activity_line(who).is_empty(),
            "the knowledge lens has nothing to say about what somebody is doing",
            format!("{:?}'s activity line is empty", person.id),
        );
    }
    for from in 0..sim.people.len() {
        for to in
            std::iter::once(Regarded::Player).chain((0..sim.people.len()).map(Regarded::Person))
        {
            checks.require(
                lens.regard(from, to) == sim.shared.regard(from, to)
                    && lens.facts(from, to) == sim.shared.facts(from, to),
                "the knowledge lens does not hand back the shared state it reads",
                format!(
                    "{from} -> {to:?} reads {} / {:?} through the lens and {} / {:?} in the \
                     store; v1 is the identity",
                    lens.regard(from, to),
                    lens.facts(from, to),
                    sim.shared.regard(from, to),
                    sim.shared.facts(from, to)
                ),
            );
        }
        checks.require(
            lens.marks(from) == sim.shared.marks(from) && lens.name(from) == sim.people[from].name,
            "the knowledge lens does not hand back a character's own facts",
            format!(
                "{from} wears {:?} through the lens and {:?} in the store",
                lens.marks(from),
                sim.shared.marks(from)
            ),
        );
    }
    // The felt value is the weighed one, and it is a different number: a lens
    // that returned the raw edge here would make every sheet lie about the
    // person reading it.
    let felt = lens.regard_as_felt(b, Regarded::Person(a));
    let raw = lens.regard(b, Regarded::Person(a));
    checks.require(
        felt == crate::traits::weighted_regard(raw, &sim.people[b].traits),
        "the lens's felt regard is not the raw edge as the holder's traits weigh it",
        format!("the edge reads {raw} raw and {felt} felt"),
    );
    // The reaction a character has to another's marks, through the lens.
    let looked = lens.reaction(tuning, a, b);
    let by_hand: i64 = lens
        .marks(b)
        .into_iter()
        .map(|mark| crate::traits::reaction_to(tuning, &sim.people[a].traits, mark))
        .sum();
    checks.require(
        looked == by_hand,
        "the lens's mark reaction is not the table's own sum",
        format!("the lens reads {looked} and the table sums to {by_hand}"),
    );

    // `at_home` is derived from the parties, never stored: everyone is home
    // while every party is idle, and a party's member leaves when it does.
    checks.require(
        (0..sim.people.len()).all(|index| lens.at_home(index)),
        "somebody is not at home in a world where nothing has moved",
        "the opening scenario has every party idle, so every character stands at their \
         home tile"
            .to_owned(),
    );
    let mut moved = Sim::opening(tuning);
    let out = moved.parties[0].member;
    moved.parties[0].activity = Activity::Working { until: 99 };
    let lens = Lens::on(&moved);
    checks.require(
        !lens.at_home(out)
            && (0..moved.people.len())
                .filter(|index| !lens.at_home(*index))
                .count()
                == 1,
        "a party going out did not take exactly its own member with it",
        format!(
            "with one party working, {} of {} characters are away from home",
            (0..moved.people.len())
                .filter(|index| !lens.at_home(*index))
                .count(),
            moved.people.len()
        ),
    );
}
