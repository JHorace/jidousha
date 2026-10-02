//! **Petitions**: the cast asking the player for things, to their face, with a
//! deadline (GDD §5's petitions module, wave 1.5) — the data half.
//!
//! # One format, three tables
//!
//! GDD §6's petition/event template is real here: [`Template`] is its id,
//! source class, trigger, body (text, deadline, reward, declared consequence)
//! and `next` links, and [`TEMPLATES`] is `CAST.md` §6's table as data — T1 to
//! T5 as written, their `next` links, T6 `thin-days`, the shortfall-sourced
//! template, and D1 to D3, the three the director carries in (wave 1.6). The consequence a template declares is **a reference into
//! [`DECLARED`]**, never a copy: the card prints that reference and the
//! deadline fires that reference (`pleas::fire`), and the battery asserts the
//! two are one pointer. A declared consequence names a row of [`KINDS`], the
//! vocabulary of four, and every word the card's chip explains is derived from
//! that row's fields ([`explain`]) — nothing is written per template.
//!
//! # Nothing branches on a template id
//!
//! A template is read through its fields: its [`Trigger`] says when it may be
//! raised, its [`Condition`] says when it is met and **what the card's "met
//! when" line says** ([`Condition::met_when`] beside [`Condition::met`], one
//! enum, so the line and the deadline's predicate are one derivation), and its
//! consequence says what fires. A seventh template is a row.
//!
//! # The record is elsewhere
//!
//! What has been raised, voiced, met and failed is `pleas.rs`; this file is the
//! content and the predicates over it. The surfaces are `card.rs`.

use crate::constants::Tuning;
use crate::lens::Lens;
use crate::pleas::Petition;
use crate::sim::{Activity, Sim};
use crate::traits::{TaskType, TraitId};

/// The module id, as GDD §5's registry spells it.
pub const MODULE: &str = "petitions";

/// **A world-day, in minutes** — what every day-count on a template is
/// measured in.
pub const DAY: u64 = crate::autonomy::DAY;

/// **The source classes** GDD §6 names, as data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// A want, speaking: raised for a carrier of this motivator row.
    Motivator(TraitId),
    /// A shortfall, pressing: raised by what the needs module did to somebody.
    Shortfall,
    /// **The director's** — carried in from outside the camp by the injector
    /// (`director.rs`, wave 1.6), and only ever raised through it: the
    /// petition check never raises one, and `pleas::raise` refuses one while
    /// the injector is off.
    Director,
}

impl Source {
    /// The class's name, as GDD §6 spells it.
    pub fn class(self) -> &'static str {
        match self {
            Source::Motivator(_) => "motivator",
            Source::Shortfall => "shortfall",
            Source::Director => "director",
        }
    }

    /// **This class's row of [`SOURCES`]** — its display word and what that
    /// word means. A class with no row is a fault the vocabulary check names;
    /// here it reads as the first row rather than panicking in a draw.
    pub fn row(self) -> &'static SourceRow {
        SOURCES
            .iter()
            .find(|row| row.class == self.class())
            .unwrap_or(&SOURCES[0])
    }

    /// **The word a card and a feed line show for the class** — off the
    /// table, never written at a surface. `director` shows as `event`: the
    /// player is never told a director exists, only that something came in
    /// from outside the camp.
    pub fn word(self) -> &'static str {
        self.row().word
    }

    /// What a card says about where the ask came from — the motivator row's
    /// own display name, read off the vocabulary; and for the other two
    /// classes, the table's own explanation of the class.
    pub fn phrase(self) -> String {
        match self {
            Source::Motivator(id) => id.def().name.to_owned(),
            Source::Shortfall => "short of upkeep".to_owned(),
            Source::Director => self.row().means.to_owned(),
        }
    }

    /// **The card's source chip** — for a want or a shortfall, whose it is,
    /// the class's display word and the template's id; for an event, the word
    /// and what it means, whole. An event leads with the word, because what
    /// came in from outside is not the petitioner's own and the chip must not
    /// read as if it were — and its id is in the feed line and the ledger row,
    /// where there is room for it, rather than clipping the meaning off the
    /// card (`eventshots` asserts the chip is drawn unclipped).
    pub fn chip(self, template: &str) -> String {
        match self {
            Source::Director => format!("{} - {}", self.word(), self.phrase()),
            _ => format!("{} - {} - {template}", self.phrase(), self.word()),
        }
    }

    /// **The tag a feed line puts before a template id**, where one is owed:
    /// only an event's — a want or a shortfall is the petitioner's own and the
    /// line already says whose, so their lines are 1.5's unchanged.
    pub fn tag(self, template: &str) -> String {
        match self {
            Source::Director => format!("{}: {template}", self.word()),
            _ => template.to_owned(),
        }
    }
}

/// **One source class's row** — what fires it, the word the player sees for
/// it, and what that word means.
#[derive(Clone, Copy, Debug)]
pub struct SourceRow {
    /// The class, as GDD §6 spells it.
    pub class: &'static str,
    /// What raises a petition of this class.
    pub fired_by: &'static str,
    /// The word the card and the feed show.
    pub word: &'static str,
    /// What the word means — the chip's explanation, derived from here.
    pub means: &'static str,
}

/// **What fires each source class, and what the player is shown for it** —
/// the table GDD §6's column is read against.
pub const SOURCES: &[SourceRow] = &[
    SourceRow {
        class: "motivator",
        fired_by: "a petition check, on a carrier of the row the trigger names",
        word: "motivator",
        means: "their own want, spoken",
    },
    SourceRow {
        class: "shortfall",
        fired_by: "a petition check, on somebody the needs module pressed",
        word: "shortfall",
        means: "what going short did to them",
    },
    SourceRow {
        class: "director",
        fired_by: "the injector (events-director, wave 1.6) - its own seeded firings after the \
                   calm window, and a scenario's pins; never the petition check",
        word: "event",
        means: "from outside the camp - not their own want",
    },
];

/// **Where a director template's `{site}` comes from** — the slot's fill,
/// drawn by the injector at the firing's address (`director.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteSlot {
    /// The template names no site.
    None,
    /// Any authored site — somewhere the news passed through.
    Any,
    /// An authored site with open work on its board; with none, the template
    /// reaches nobody.
    OpenWork,
}

/// **When a template may be raised** — the state predicates of GDD §6's
/// trigger. The world-time window is [`Template::opens_day`] and the seeded
/// roll is [`Template::rolled`]; this is the third part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    /// Their purse holds less than the template's `{n}`.
    PurseBelow,
    /// They have finished no paid work of this kind for this many days, and
    /// have been in the camp that long.
    NoWorkFor {
        /// What kind of work.
        task: TaskType,
        /// For how many world-days.
        days: u64,
    },
    /// They have not set out on any errand for this many days, and have been
    /// in the camp that long — and somewhere stands that they have not been.
    NotOutFor {
        /// For how many world-days.
        days: u64,
    },
    /// Somebody else in the camp is desperate (the `desperate` chip's own
    /// predicate, `needs::is_desperate`).
    SomeoneDesperate,
    /// They have gone short this many times inside the thin-days window (a
    /// drawer row).
    Shortfalls {
        /// How many.
        count: usize,
    },
    /// **Never by a check**: only a `next` link raises it.
    Chained,
    /// **They carry any of these traits** — the director's eligibility (wave
    /// 1.6): an event reaches whoever it is about, purse regardless. The
    /// `{site}` slot is filled by the injector, from `site`.
    Carries {
        /// Any of these.
        any: &'static [TraitId],
        /// Where the `{site}` comes from.
        site: SiteSlot,
    },
}

/// **What a template pays its satisfier.**
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reward {
    /// Nothing in gold: a poor petitioner pays in regard, which is GDD §4.2's
    /// operation and not a transfer.
    InRegard,
    /// Gold out of the petitioner's purse to whoever met it (GDD §4.1's
    /// petition-reward TRANSFER) — up to this much, never an overdraft.
    Gold(i64),
}

/// **When a petition is met** — and the one place the card's "met when" line
/// comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    /// Their purse holds `{n}` at any point before the deadline. **The one
    /// wallet-shaped condition**: it is what a gift can answer.
    PurseAtLeast,
    /// They are sent to fight work whose pot is at least `{n}`.
    SentToFight,
    /// `{other}` is no longer desperate **at the deadline** — checked then and
    /// only then.
    OtherSettled,
    /// They are sent to a site they have never reached.
    SentSomewhereNew,
    /// An industry is put up after they asked, or they work `{n}` shifts.
    Bench,
    /// They finish craft work of any kind after they asked.
    CraftDone,
    /// They finish paid work of any kind after they asked.
    PaidWork,
    /// They are paid `{n}` or more — wages and shares, summed since voicing
    /// (the rival offer, wave 1.6).
    PaidAtLeast,
    /// A job at `{site}` is finished — by anybody — after they asked (the word
    /// from the road, wave 1.6).
    SiteWorked,
}

impl Condition {
    /// Whether this condition is answerable with money — the only shape the
    /// card's GIVE appears on.
    pub fn wallet_shaped(self) -> bool {
        self == Condition::PurseAtLeast
    }

    /// Whether it is judged only at the deadline rather than whenever it
    /// becomes true.
    pub fn at_deadline(self) -> bool {
        self == Condition::OtherSettled
    }

    /// **Whose circumstances meet it** — the petitioner, or `{other}`.
    pub fn subject(self, petition: &Petition) -> usize {
        match self {
            Condition::OtherSettled => petition.other.unwrap_or(petition.who),
            _ => petition.who,
        }
    }

    /// **The predicate** — the one function the deadline evaluates and every
    /// check after an occurrence evaluates. Reads the world it is given and
    /// decides nothing else.
    pub fn met(self, sim: &Sim, petition: &Petition) -> bool {
        let Some(since) = petition.voiced_at else {
            return false;
        };
        let who = self.subject(petition);
        let Some(person) = sim.people.get(who) else {
            return false;
        };
        let errand_job = || {
            let party = sim.parties.get(who)?;
            if party.activity == Activity::Idle {
                return None;
            }
            let job = party.job()?;
            Some((job, *sim.sites.get(job.site)?.quest(job.slot)?))
        };
        match self {
            Condition::PurseAtLeast => person.wallet >= petition.n,
            Condition::SentToFight => errand_job()
                .is_some_and(|(_, quest)| quest.task == TaskType::Fight && quest.pot >= petition.n),
            Condition::OtherSettled => {
                person.present && person.desperation < crate::needs::DESPERATE_AT
            }
            Condition::SentSomewhereNew => errand_job().is_some_and(|(job, _)| {
                sim.sites
                    .get(job.site)
                    .is_some_and(|site| site.industry.is_none())
                    && !person.memory.visited.contains(&job.site)
            }),
            Condition::Bench => {
                let built = sim.events.iter().any(|event| {
                    event.class == crate::attention::EventClass::Built && event.minute >= since
                });
                let shifts = person
                    .memory
                    .worked_since(since)
                    .filter(|done| done.shift)
                    .count();
                built || i64::try_from(shifts).unwrap_or(0) >= petition.n
            }
            Condition::CraftDone => person
                .memory
                .worked_since(since)
                .any(|done| done.task == TaskType::Craft),
            Condition::PaidWork => person.memory.worked_since(since).next().is_some(),
            Condition::PaidAtLeast => person.memory.earned_since(since) >= petition.n,
            Condition::SiteWorked => petition
                .site
                .is_some_and(|site| worked_at(sim, site, since).next().is_some()),
        }
    }

    /// **The card's "met when" line, derived from the variant the predicate
    /// above matches** — so a mutated `{n}` moves the line and the predicate
    /// together, and a line describing a condition the deadline does not
    /// check is not a state this card can reach.
    pub fn met_when(self, lens: &Lens<'_>, petition: &Petition) -> String {
        let name = lens.name(petition.who);
        let other = petition.other.map_or("somebody", |who| lens.name(who));
        let by = petition.voiced_at.map_or_else(
            || "the deadline".to_owned(),
            |_| crate::clock::stamp(petition.deadline),
        );
        match self {
            Condition::PurseAtLeast => {
                format!("{name}'s purse holds {}g, any time before {by}", petition.n)
            }
            Condition::SentToFight => format!(
                "{name} is sent to fight work with a pot of {}g or more, before {by}",
                petition.n
            ),
            Condition::OtherSettled => format!(
                "{other} is under desperation {} when {by} comes",
                crate::needs::DESPERATE_AT
            ),
            Condition::SentSomewhereNew => {
                format!("{name} is sent to a site they have never reached, before {by}")
            }
            Condition::Bench => format!(
                "an industry goes up, or {name} works {} shifts, before {by}",
                petition.n
            ),
            Condition::CraftDone => format!("{name} finishes craft work, before {by}"),
            Condition::PaidWork => format!("{name} finishes a paid job or a shift, before {by}"),
            Condition::PaidAtLeast => format!(
                "{name} is paid {}g or more in wages and shares, before {by}",
                petition.n
            ),
            Condition::SiteWorked => format!(
                "a job at {} is finished by anybody, before {by}",
                site_name(petition.site)
            ),
        }
    }
}

/// **Every job finished at an authored site since a minute** — what the word
/// from the road is met by, and whose hand was in it (`posted`).
pub fn worked_at(
    sim: &Sim,
    site: usize,
    since: u64,
) -> impl Iterator<Item = &crate::resolution::Resolved> {
    sim.resolved.iter().filter(move |record| {
        record.job.site == site && record.minute >= since && record.tier.succeeded()
    })
}

/// A site's name, for a line — "somewhere" when the slot was never filled.
pub fn site_name(site: Option<usize>) -> String {
    site.map_or("somewhere".to_owned(), |site| {
        crate::grid::LOCATIONS[crate::sim::site_location(site)]
            .name
            .to_owned()
    })
}

/// **One row of the consequence vocabulary** (`CAST.md` §6): what firing it
/// does, as fields. [`explain`] reads every field and nothing else, so the
/// chip's explanation of a kind is a function of this row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kind {
    /// The word on the chip.
    pub id: &'static str,
    /// Whether their purse is emptied, and the gold burned (GDD §4.1's
    /// declared-consequence BURN).
    pub burns_purse: bool,
    /// How far it presses their desperation.
    pub press: i64,
    /// Whether their source line is rewritten to the event.
    pub rewrites: bool,
    /// Whether they leave the camp for the declared number of days.
    pub leaves: bool,
    /// Whether half their purse goes to `{other}` (conserved).
    pub gives_half: bool,
}

/// **The vocabulary of four** (`CAST.md` §6). Every consequence also sours
/// unless it *is* `sours` — which is [`fire`'s](crate::pleas) rule and not a
/// field, because it holds of every row.
pub const KINDS: &[Kind] = &[SOURS, BROKE, WALKS_OUT, GIVES_AWAY];

/// `sours`: regard toward the player falls by the large step, and a grudge
/// is written on a repeat or an egregious failure.
pub const SOURS: Kind = Kind {
    id: "sours",
    burns_purse: false,
    press: 0,
    rewrites: false,
    leaves: false,
    gives_half: false,
};

/// `broke`: the purse burned to nothing, desperation +2, the source line
/// rewritten to the event.
pub const BROKE: Kind = Kind {
    id: "broke",
    burns_purse: true,
    press: 2,
    rewrites: true,
    leaves: false,
    gives_half: false,
};

/// `walks-out`: they leave the camp for `{n}` days — an away-state, not a
/// party — unpaid, and come back.
pub const WALKS_OUT: Kind = Kind {
    id: "walks-out",
    burns_purse: false,
    press: 0,
    rewrites: false,
    leaves: true,
    gives_half: false,
};

/// `gives-away`: half their purse to `{other}`, conserved, desperation +1.
pub const GIVES_AWAY: Kind = Kind {
    id: "gives-away",
    burns_purse: false,
    press: 1,
    rewrites: false,
    leaves: false,
    gives_half: true,
};

/// **A declared consequence** — a kind from the vocabulary, with the numbers
/// a template gives it. What a template's `consequence` field references.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Declared {
    /// Which row of the vocabulary.
    pub kind: &'static Kind,
    /// For a kind that leaves: how many world-days.
    pub days: u64,
    /// **A press the template adds** to the kind's own — T5's "sours,
    /// desperation +1", which is `sours` with one more step and not a fifth
    /// kind.
    pub press: i64,
}

impl Declared {
    /// Everything this consequence presses by, kind and template together.
    pub fn pressed(&self) -> i64 {
        self.kind.press + self.press
    }
}

/// **The declared consequences the table references**, one per distinct
/// shape a template asks for. A template points at one of these, and the card
/// and the deadline both read the template's pointer.
pub static DECLARED: [Declared; 7] = [
    Declared {
        kind: &SOURS,
        days: 0,
        press: 0,
    },
    Declared {
        kind: &SOURS,
        days: 0,
        press: 1,
    },
    Declared {
        kind: &BROKE,
        days: 0,
        press: 0,
    },
    Declared {
        kind: &WALKS_OUT,
        days: 3,
        press: 0,
    },
    Declared {
        kind: &WALKS_OUT,
        days: 4,
        press: 0,
    },
    Declared {
        kind: &WALKS_OUT,
        days: 5,
        press: 0,
    },
    Declared {
        kind: &GIVES_AWAY,
        days: 0,
        press: 0,
    },
];

/// **What a consequence chip says when it is tapped** — derived from the
/// vocabulary row's fields and the template's numbers, never written per
/// card. Moving `plea_regard` or a kind's press moves this sentence.
pub fn explain(declared: &Declared, tuning: &Tuning, other: Option<&str>) -> String {
    let kind = declared.kind;
    let mut clauses: Vec<String> = Vec::new();
    if kind.burns_purse {
        clauses.push("their purse is emptied and the gold is gone".to_owned());
    }
    if kind.gives_half {
        clauses.push(format!(
            "half their purse goes to {}",
            other.unwrap_or("the one they spoke for")
        ));
    }
    if kind.leaves {
        clauses.push(format!(
            "they leave the camp for {} days, unpaid, and come back",
            declared.days
        ));
    }
    if declared.pressed() != 0 {
        clauses.push(format!("desperation +{}", declared.pressed()));
    }
    if kind.rewrites {
        clauses.push("their source line is rewritten to it".to_owned());
    }
    let sour = format!(
        "regard toward you -{}, and a grudge if they were failed before",
        tuning.plea_regard
    );
    if clauses.is_empty() {
        format!("{}: {sour}.", kind.id)
    } else {
        format!(
            "{}: {}; and it sours - {sour}.",
            kind.id,
            clauses.join(", ")
        )
    }
}

/// **One template** — GDD §6's petition/event format, whole.
#[derive(Clone, Copy, Debug)]
pub struct Template {
    /// The id a stamp, a card and a `next` link name it by. ASCII, lowercase.
    pub id: &'static str,
    /// Which source class raises it.
    pub source: Source,
    /// The state predicate a check evaluates.
    pub trigger: Trigger,
    /// **The world-time window**: the world-day (from zero) before which no
    /// check raises it.
    pub opens_day: u64,
    /// Whether a check also has to pass the seeded roll (`plea_odds`).
    pub rolled: bool,
    /// The words, with `{name}`, `{other}`, `{site}`, `{n}`, `{deadline}`,
    /// `{building}` and `{industry}` slots.
    pub text: &'static str,
    /// How long from voicing to the cliff, in world-days.
    pub deadline_days: u64,
    /// What it pays whoever meets it.
    pub reward: Reward,
    /// When it is met.
    pub condition: Condition,
    /// **The declared consequence — a reference, never a copy.**
    pub consequence: &'static Declared,
    /// The `{n}` slot's number: a debt, a pot, a count of shifts.
    pub n: i64,
    /// **How far a director firing may draw `{n}` above it** (wave 1.6): the
    /// rival's offer is drawn between `n` and `n + spread` at the firing's
    /// address. Zero everywhere a check raises the row — a want's number is
    /// the row's.
    pub spread: i64,
    /// What they did, said when it is met — the feed line and the rewritten
    /// source line both (`{other}` may appear).
    pub relieved: &'static str,
    /// What became of them, said when it fails.
    pub broken: &'static str,
    /// The template a failure raises next, by id.
    pub next_on_fail: Option<&'static str>,
    /// The template satisfaction raises next, by id.
    pub next_on_met: Option<&'static str>,
}

impl Template {
    /// Whether it is ever raised by a check (as against only by a `next`).
    pub fn checked(&self) -> bool {
        self.trigger != Trigger::Chained && self.source != Source::Director
    }

    /// The minute its window opens.
    pub fn opens_at(&self) -> u64 {
        self.opens_day * DAY
    }

    /// The motivator a check needs its carrier to hold, if the source is one.
    pub fn motivator(&self) -> Option<TraitId> {
        match self.source {
            Source::Motivator(id) => Some(id),
            _ => None,
        }
    }
}

/// **The template table** — `CAST.md` §6, as data: T1 to T5 as written with
/// their `next` links, T6 `thin-days`, the shortfall exemplar, and D1 to D3,
/// the director's (wave 1.6).
pub static TEMPLATES: [Template; 12] = [
    // T1 — indebted.
    Template {
        id: "collectors-visit",
        source: Source::Motivator(TraitId::Indebted),
        trigger: Trigger::PurseBelow,
        opens_day: 1,
        rolled: true,
        text: "{name}: I owe {n} gold to a man who counts days. Find me work that pays \
               before {deadline}, or he takes it out of me.",
        deadline_days: 6,
        reward: Reward::InRegard,
        condition: Condition::PurseAtLeast,
        consequence: &DECLARED[2],
        n: 30,
        spread: 0,
        relieved: "paid the collector off, for now",
        broken: "was cleaned out by the collector",
        next_on_fail: Some("collectors-visit-again"),
        next_on_met: None,
    },
    // T1's chain on failure: the same ask, sooner, and worse.
    Template {
        id: "collectors-visit-again",
        source: Source::Motivator(TraitId::Indebted),
        trigger: Trigger::Chained,
        opens_day: 1,
        rolled: false,
        text: "{name}: He came once already. I owe {n} gold to a man who counts days. Find \
               me work that pays before {deadline}, or he takes it out of me.",
        deadline_days: 3,
        reward: Reward::InRegard,
        condition: Condition::PurseAtLeast,
        consequence: &DECLARED[4],
        n: 30,
        spread: 0,
        relieved: "paid the collector off at the second time of asking",
        broken: "was dragged off by the collector's men",
        next_on_fail: None,
        next_on_met: None,
    },
    // T2 — renown.
    Template {
        id: "proving-job",
        source: Source::Motivator(TraitId::Renown),
        trigger: Trigger::NoWorkFor {
            task: TaskType::Fight,
            days: 2,
        },
        opens_day: 1,
        rolled: true,
        text: "{name}: Send me somewhere that matters. {site} - not the safe one. People \
               should hear about it.",
        deadline_days: 5,
        reward: Reward::InRegard,
        condition: Condition::SentToFight,
        consequence: &DECLARED[0],
        n: 55,
        spread: 0,
        relieved: "was sent somewhere that matters",
        broken: "was sent nowhere that mattered",
        next_on_fail: Some("proving-job-again"),
        next_on_met: None,
    },
    // T2's chain on a repeat failure: the walk-out CAST names.
    Template {
        id: "proving-job-again",
        source: Source::Motivator(TraitId::Renown),
        trigger: Trigger::Chained,
        opens_day: 1,
        rolled: false,
        text: "{name}: I asked once. {site} - not the safe one - before {deadline}, or I go \
               and find a name somewhere else.",
        deadline_days: 5,
        reward: Reward::InRegard,
        condition: Condition::SentToFight,
        consequence: &DECLARED[5],
        n: 55,
        spread: 0,
        relieved: "was sent somewhere that matters at last",
        broken: "went looking for a name somewhere else",
        next_on_fail: None,
        next_on_met: None,
    },
    // T3 — caring.
    Template {
        id: "look-after-them",
        source: Source::Motivator(TraitId::Caring),
        trigger: Trigger::SomeoneDesperate,
        opens_day: 1,
        rolled: true,
        text: "{name}: {other} has not eaten properly in days. Get {other} paying work \
               before {deadline}, or I will feed {other} out of my own pocket.",
        deadline_days: 4,
        reward: Reward::InRegard,
        condition: Condition::OtherSettled,
        consequence: &DECLARED[6],
        n: 0,
        spread: 0,
        relieved: "saw {other} fed",
        broken: "fed {other} out of their own pocket",
        next_on_fail: None,
        next_on_met: None,
    },
    // T4 — restless.
    Template {
        id: "the-far-road",
        source: Source::Motivator(TraitId::Restless),
        trigger: Trigger::NotOutFor { days: 2 },
        opens_day: 1,
        rolled: true,
        text: "{name}: I have been looking at the same tents for too long. Send me to \
               {site} before {deadline}. Anywhere I have not been.",
        deadline_days: 6,
        reward: Reward::InRegard,
        condition: Condition::SentSomewhereNew,
        consequence: &DECLARED[3],
        n: 0,
        spread: 0,
        relieved: "went somewhere new, finally",
        broken: "wandered off",
        next_on_fail: None,
        next_on_met: None,
    },
    // T5 — maker.
    Template {
        id: "a-proper-bench",
        source: Source::Motivator(TraitId::Maker),
        trigger: Trigger::NoWorkFor {
            task: TaskType::Craft,
            days: 2,
        },
        opens_day: 1,
        rolled: true,
        text: "{name}: I can make things this camp needs, if I have somewhere to make them. \
               Put up {building} or give me {n} days at the {industry} before {deadline}.",
        deadline_days: 8,
        reward: Reward::InRegard,
        condition: Condition::Bench,
        consequence: &DECLARED[1],
        n: 2,
        spread: 0,
        relieved: "got a bench to make things on",
        broken: "has nowhere to make anything",
        next_on_fail: None,
        next_on_met: Some("first-order"),
    },
    // T5's chain on satisfaction: the seed of the industry arc.
    Template {
        id: "first-order",
        source: Source::Motivator(TraitId::Maker),
        trigger: Trigger::Chained,
        opens_day: 1,
        rolled: false,
        text: "{name}: It is up. Give me something to make.",
        deadline_days: 3,
        reward: Reward::InRegard,
        condition: Condition::CraftDone,
        consequence: &DECLARED[0],
        n: 0,
        spread: 0,
        relieved: "made the first thing on the new bench",
        broken: "has a bench and nothing to make on it",
        next_on_fail: None,
        next_on_met: None,
    },
    // T6 — the shortfall exemplar (new with wave 1.5).
    Template {
        id: "thin-days",
        source: Source::Shortfall,
        trigger: Trigger::Shortfalls { count: 3 },
        opens_day: 1,
        rolled: false,
        text: "{name}: Three intervals short now. I need paying work by {deadline} or I am \
               done waiting for it.",
        deadline_days: 4,
        reward: Reward::InRegard,
        condition: Condition::PaidWork,
        consequence: &DECLARED[3],
        n: 3,
        spread: 0,
        relieved: "found paying work",
        broken: "walked out, done waiting for paying work",
        next_on_fail: None,
        next_on_met: None,
    },
    // **The director's three** (wave 1.6, `CAST.md` §6 D1-D3) — the capsule's
    // own examples, petition-flavoured, raised only by the injector. All
    // three pay in regard and declare a kind from the vocabulary of four.
    //
    // D1 — the canned loan shark T1's note promised: T1's words and its
    // consequence, `broke`, with T1's chain; but an event reaches anyone
    // indebted, purse regardless — the collector does not wait for poverty.
    Template {
        id: "the-collector-comes",
        source: Source::Director,
        trigger: Trigger::Carries {
            any: &[TraitId::Indebted],
            site: SiteSlot::None,
        },
        opens_day: 0,
        rolled: false,
        text: "{name}: I owe {n} gold to a man who counts days. Find me work that pays \
               before {deadline}, or he takes it out of me.",
        deadline_days: 6,
        reward: Reward::InRegard,
        condition: Condition::PurseAtLeast,
        consequence: &DECLARED[2],
        n: 30,
        spread: 0,
        relieved: "paid the collector off, for now",
        broken: "was cleaned out by the collector",
        next_on_fail: Some("collectors-visit-again"),
        next_on_met: None,
    },
    // D2 — a rival company's offer, to the proud of their name and the greedy.
    Template {
        id: "rival-offer",
        source: Source::Director,
        trigger: Trigger::Carries {
            any: &[TraitId::Renown, TraitId::Greedy],
            site: SiteSlot::Any,
        },
        opens_day: 0,
        rolled: false,
        text: "{name}: The Grey Banners came through {site} offering {n} a week. Show me \
               what I am worth here before {deadline}, or I will go find out.",
        deadline_days: 5,
        reward: Reward::InRegard,
        condition: Condition::PaidAtLeast,
        consequence: &DECLARED[5],
        n: 30,
        spread: 20,
        relieved: "was shown what they are worth here",
        broken: "went to find out what the Grey Banners pay",
        next_on_fail: None,
        next_on_met: None,
    },
    // D3 — word from the road, to the restless and the ambitious.
    Template {
        id: "word-from-the-road",
        source: Source::Director,
        trigger: Trigger::Carries {
            any: &[TraitId::Restless, TraitId::Renown],
            site: SiteSlot::OpenWork,
        },
        opens_day: 0,
        rolled: false,
        text: "{name}: Travelers say {site} is worth somebody's time again - good pots for \
               whoever moves first. We should be first.",
        deadline_days: 4,
        reward: Reward::InRegard,
        condition: Condition::SiteWorked,
        consequence: &DECLARED[0],
        n: 0,
        spread: 0,
        relieved: "saw the camp move first on the word from the road",
        broken: "watched the word from the road go cold",
        next_on_fail: None,
        next_on_met: None,
    },
];

/// The template with this id, if the table has one.
pub fn find(id: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|template| template.id == id)
}

/// **The motivators the table writes a template for** — what
/// `traits::vocabulary`'s no-dead-motivator rule walks (`CAST.md` §3.2).
/// It was a declared list until this wave; it is a walk over the real table
/// now, and the assertion that reads it did not change.
pub fn templated_motivators() -> Vec<TraitId> {
    TEMPLATES
        .iter()
        .filter(|template| template.checked())
        .filter_map(Template::motivator)
        .collect()
}

/// **The earliest world-minute any template's window opens** — when the first
/// petition check is worth scheduling.
pub fn first_window() -> u64 {
    TEMPLATES
        .iter()
        .filter(|template| template.checked())
        .map(Template::opens_at)
        .min()
        .unwrap_or(DAY)
}

/// **The roll a check makes**, addressed by the occurrence — the seed, the
/// world-minute, who and which template — and by nothing else (G-016's rule,
/// G-045's mix). Salted, so a petition roll never shadows a resolution roll
/// at the same minute.
pub fn roll(seed: u64, minute: u64, who: usize, template: usize) -> i64 {
    fn mix(mut z: u64) -> u64 {
        z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    const SALT: u64 = 0x7065_7469_7469_6f6e;
    let place = ((who as u64) << 32) | template as u64;
    let address = mix(mix(mix(seed ^ SALT) ^ minute) ^ place);
    i64::from(jidousha::prelude::Rng::from_seed(address).below(100))
}

/// **Fill a template's slots** — the card's words and nothing else.
///
/// Every slot is resolved from the record: who, the other, the site and the
/// number it was raised with, the deadline it was voiced against, and the
/// settlement's own content for the two the bench speaks of.
pub fn resolve(text: &str, lens: &Lens<'_>, petition: &Petition) -> String {
    let site = site_name(petition.site);
    let industry = crate::settlement::INDUSTRIES
        .first()
        .map_or("works", |spec| spec.name);
    let out = text
        .replace("{name}", lens.name(petition.who))
        .replace(
            "{other}",
            petition.other.map_or("somebody", |who| lens.name(who)),
        )
        .replace("{n}", &petition.n.to_string())
        .replace("{deadline}", &crate::clock::stamp(petition.deadline))
        .replace("{building}", industry)
        .replace("{industry}", crate::sim::plain(industry));
    // A site's name opens a sentence in the proving-job's text, after a stop.
    let site = capitalised_after_stop(&out, &site);
    out.replace("{site}", &site)
}

/// A site's name, capitalised where it opens a sentence in `text` — the
/// proving-job's "{site} - not the safe one" stands after a full stop.
fn capitalised_after_stop(text: &str, site: &str) -> String {
    let opens = text
        .find("{site}")
        .is_some_and(|at| text[..at].trim_end().ends_with('.'));
    if !opens {
        return site.to_owned();
    }
    let mut glyphs = site.chars();
    match glyphs.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + glyphs.as_str(),
        None => String::new(),
    }
}

/// **The table's own validation** — the claims a comment cannot hold.
///
/// Ids are stamp-shaped and unique; every `next` link names a row; every
/// declared consequence is a kind of the vocabulary of four, and the four are
/// `CAST.md` §6's; every template's words are printable ASCII and use only
/// the slots `resolve` fills; a chained row is reachable from a `next`; the
/// source-class table names every class with its display word; and every
/// director-sourced row has the injector to fire it.
pub fn vocabulary(checks: &mut crate::checks::Checks) {
    const SLOTS: [&str; 7] = [
        "{name}",
        "{other}",
        "{site}",
        "{n}",
        "{deadline}",
        "{building}",
        "{industry}",
    ];
    for (index, template) in TEMPLATES.iter().enumerate() {
        checks.require(
            !template.id.is_empty()
                && template
                    .id
                    .chars()
                    .all(|glyph| glyph.is_ascii_lowercase() || glyph == '-')
                && TEMPLATES
                    .iter()
                    .filter(|other| other.id == template.id)
                    .count()
                    == 1,
            "a petition template's id is not a unique stamp-shaped name",
            format!("TEMPLATES[{index}] is {:?}", template.id),
        );
        for link in [template.next_on_fail, template.next_on_met]
            .into_iter()
            .flatten()
        {
            checks.require(
                find(link).is_some(),
                "a petition template links to a template the table does not have",
                format!("{:?} names {link:?} as its next", template.id),
            );
        }
        checks.require(
            KINDS.iter().any(|kind| kind == template.consequence.kind),
            "a petition declares a consequence outside the vocabulary of four",
            format!(
                "{:?} declares {:?}; CAST.md s6's vocabulary is the whole of what fires",
                template.id, template.consequence.kind.id
            ),
        );
        let mut rest = template.text.to_owned();
        for slot in SLOTS {
            rest = rest.replace(slot, "");
        }
        checks.require(
            rest.chars().all(|glyph| (' '..='~').contains(&glyph))
                && !rest.contains('{')
                && !rest.contains('}'),
            "a petition's words are not ASCII, or name a slot nothing fills",
            format!("{:?} reads {:?}", template.id, template.text),
        );
        checks.require(
            template.deadline_days > 0,
            "a petition has no time to be answered in",
            format!("{:?} gives {} days", template.id, template.deadline_days),
        );
        if template.trigger == Trigger::Chained {
            checks.require(
                TEMPLATES.iter().any(|other| {
                    other.next_on_fail == Some(template.id)
                        || other.next_on_met == Some(template.id)
                }),
                "a chained petition is reached by no next link",
                format!(
                    "{:?} is only ever raised by a next, and nothing names it",
                    template.id
                ),
            );
        }
        // **A director-sourced row requires the injector** (wave 1.6, flipped
        // from 1.5's refusal): the registry must carry the module that fires
        // it, the petition check must never raise it, and its trigger must be
        // the director's eligibility — otherwise the row is a template that
        // silently never fires, and never-lies applies to data too.
        if template.source == Source::Director {
            checks.require(
                crate::modules::MODULES
                    .iter()
                    .any(|spec| spec.id == crate::director::MODULE)
                    && !template.checked()
                    && matches!(template.trigger, Trigger::Carries { .. }),
                "a director-sourced template has nothing to fire it",
                format!(
                    "{:?} is director-sourced; it needs the {} row in the registry, a \
                     Carries trigger, and no petition check raising it",
                    template.id,
                    crate::director::MODULE
                ),
            );
            // **No dead director template** (`CAST.md` §8's 1.6 note: the
            // no-dead-motivator rule runs over the director's rows too):
            // somebody in the cast carries a trait it reaches.
            if let Trigger::Carries { any, .. } = template.trigger {
                checks.require(
                    crate::people::cast()
                        .iter()
                        .any(|person| any.iter().any(|id| person.traits.contains(id))),
                    "a director template reaches nobody in the cast",
                    format!(
                        "{:?} reaches carriers of {any:?} and nobody carries one",
                        template.id
                    ),
                );
            }
        } else {
            checks.require(
                !matches!(template.trigger, Trigger::Carries { .. }) && template.spread == 0,
                "a want's or a shortfall's template is shaped like the director's",
                format!(
                    "{:?} carries a Carries trigger or an {{n}} spread; those are the \
                     injector's to fill, and a petition check would never fill them",
                    template.id
                ),
            );
        }
    }
    // **The director's three are in the table** (`CAST.md` §6 D1-D3).
    let directed: Vec<&str> = TEMPLATES
        .iter()
        .filter(|template| template.source == Source::Director)
        .map(|template| template.id)
        .collect();
    checks.require(
        directed == ["the-collector-comes", "rival-offer", "word-from-the-road"],
        "the director's templates are not CAST.md s6's D1-D3",
        format!("the director-sourced rows are {directed:?}"),
    );
    let ids: Vec<&str> = KINDS.iter().map(|kind| kind.id).collect();
    checks.require(
        ids == ["sours", "broke", "walks-out", "gives-away"],
        "the consequence vocabulary is not CAST.md s6's four",
        format!("the kinds are {ids:?}"),
    );
    for class in [
        Source::Motivator(TraitId::Indebted).class(),
        Source::Shortfall.class(),
        Source::Director.class(),
    ] {
        checks.require(
            SOURCES
                .iter()
                .filter(|row| {
                    row.class == class
                        && !row.word.is_empty()
                        && !row.means.is_empty()
                        && !row.fired_by.is_empty()
                })
                .count()
                == 1,
            "a source class has no row saying what fires it and what the player sees",
            format!("{class:?} is not in SOURCES once, with a word and a meaning"),
        );
    }
    // **The shortfall exemplar is a shortfall's**, and T6 is in the table.
    checks.require(
        find("thin-days").is_some_and(|template| template.source == Source::Shortfall),
        "the shortfall-sourced template is missing",
        "thin-days is wave 1.5's T6, the escalation pipe's second rung".to_owned(),
    );
}
