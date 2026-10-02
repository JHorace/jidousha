//! **The scenario file** — GDD §6's fourth format, real since wave 1.6: the
//! seed, the map, the roster's opening balances and arrival minutes, the
//! treasury, pinned template firings, and whether the director speaks.
//!
//! # Files, not code
//!
//! GDD §5 says *scenarios are files*, and these are: ASCII text under
//! `games/ninjo/scenarios/`, one statement a line, compiled into the binary
//! with `include_str!` so the web build carries them with nothing to fetch.
//! [`parse`] reads one and refuses anything it cannot read whole — a line it
//! does not know, a number out of range, a person the cast does not have, a
//! pin naming a template the director does not speak — with the file, the
//! line, what was wrong and how to fix it. A scenario that half-loaded would
//! be a world nobody authored.
//!
//! # Two files, and what each is for
//!
//! - **`freeplay.txt`** is the authored start *moved*, not redesigned: the
//!   same ten people, the same purses and desperations, the same arrival
//!   minutes, the treasury empty and the seed zero. Until this wave those
//!   numbers were literals in `people.rs`; the proof that moving them changed
//!   nothing is a transcript identity (`directed::scenario_equality`), not a
//!   reading of this file.
//! - **`pinned-collector.txt`** is a *test* scenario: freeplay's camp, the
//!   director's own firings off, and one pin. It exists so the pin is
//!   exercised by something; the tutorial that will be made of pins is
//!   post-MVP.
//!
//! # What a scenario does not carry
//!
//! **The pressure params and the relationship preset are drawer rows**
//! (`calm_days`, `director_hours`, `director_max`; `bonds_preset`), and a
//! number with two homes is two ways to move it. A scenario says whether the
//! director speaks at all; the drawer says how hard. Both ride every stamp.
//!
//! **Who the cast are** — names, homes, portraits, traits and the lines they
//! were generated with — is `people::cast`, the content `CAST.md` owns. A
//! scenario opens them; it does not author them.

use std::sync::LazyLock;

use crate::people::{self, Character};
use crate::petitions::{self, Source, Template, Trigger};

/// **The files this build carries**, by the path a message names them by. The
/// first is the authored start.
const FILES: [(&str, &str); 2] = [
    (
        "scenarios/freeplay.txt",
        include_str!("../scenarios/freeplay.txt"),
    ),
    (
        "scenarios/pinned-collector.txt",
        include_str!("../scenarios/pinned-collector.txt"),
    ),
];

/// The id of the authored start.
pub const FREEPLAY: &str = "freeplay";

/// The one map this build has.
pub const MAP: &str = "kawaza";

/// **One person's opening**, as the file states it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opening {
    /// Whose — a `people::cast` id.
    pub id: &'static str,
    /// What their purse opens holding.
    pub wallet: i64,
    /// Their desperation at the opening.
    pub desperation: i64,
    /// The world-minute they are in the camp from: zero for a founder.
    pub arrives: u64,
}

/// **A pinned template firing** — the director speaking at a scripted
/// minute (GDD §6: "pinned template firings").
#[derive(Clone, Copy, Debug)]
pub struct Pin {
    /// The template it fires — a director-sourced row, by reference.
    pub template: &'static Template,
    /// The world-minute it fires at, exactly.
    pub at: u64,
    /// Whom it is for, by roster index; `None` is the director's own seeded
    /// choice over whoever the template can reach.
    pub who: Option<usize>,
}

/// **One scenario**, whole.
#[derive(Clone, Debug)]
pub struct Scenario {
    /// The id every stamp and transcript header carries.
    pub id: String,
    /// The authored seed — what a world opens on when no `?seed=` overrides
    /// it.
    pub seed: u64,
    /// The map, by id.
    pub map: String,
    /// What the settlement opens holding.
    pub treasury: i64,
    /// **Whether the director fires on its own.** Pins fire either way: they
    /// are the scenario's script, and the director is what happens between
    /// them.
    pub director: bool,
    /// Every person of the cast, in registry order.
    pub people: Vec<Opening>,
    /// Every pin, in file order.
    pub pins: Vec<Pin>,
    /// The file it was read from.
    pub path: &'static str,
}

impl Scenario {
    /// **The people of this scenario at its opening** — the cast, with this
    /// file's columns written over each row. The one place a balance reaches
    /// a `Character`.
    pub fn cast(&self) -> Vec<Character> {
        people::cast()
            .into_iter()
            .zip(self.people.iter())
            .map(|(person, opening)| Character {
                wallet: opening.wallet,
                desperation: opening.desperation,
                present_from: opening.arrives,
                present: opening.arrives == 0,
                ..person
            })
            .collect()
    }

    /// **The scenario's stamp** — its id and whether the director speaks;
    /// what the opening log line and every report carry.
    pub fn stamp(&self) -> String {
        format!(
            "scenario:{} map:{} director:{} pins:{}",
            self.id,
            self.map,
            if self.director { "on" } else { "off" },
            self.pins.len()
        )
    }
}

/// Every scenario this build carries, parsed once.
static LOADED: LazyLock<Vec<Scenario>> = LazyLock::new(|| {
    FILES
        .iter()
        .map(|(path, text)| match parse(path, text) {
            Ok(scenario) => scenario,
            Err(error) => crate::checks::fail(
                "a scenario file this build carries does not parse",
                &error.message(),
            ),
        })
        .collect()
});

/// Every scenario, freeplay first.
pub fn all() -> &'static [Scenario] {
    &LOADED
}

/// **The authored start** — `scenarios/freeplay.txt`.
pub fn freeplay() -> &'static Scenario {
    match find(FREEPLAY) {
        Some(scenario) => scenario,
        None => crate::checks::fail(
            "the authored start is missing",
            "no scenario file carries the id freeplay; scenario::FILES must list \
             scenarios/freeplay.txt",
        ),
    }
}

/// The scenario with this id, if this build carries one.
pub fn find(id: &str) -> Option<&'static Scenario> {
    all().iter().find(|scenario| scenario.id == id)
}

/// **Why a scenario file was refused** — where, what, and how to fix it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioError {
    /// The file.
    pub path: String,
    /// The line, from one; zero for a fault of the file as a whole.
    pub line: usize,
    /// What was wrong.
    pub what: String,
    /// How to fix it.
    pub fix: String,
}

impl ScenarioError {
    /// The sentence a refusal says: what happened, where, and the fix.
    pub fn message(&self) -> String {
        let at = if self.line == 0 {
            self.path.clone()
        } else {
            format!("{} line {}", self.path, self.line)
        };
        format!("{at}: {} - {}", self.what, self.fix)
    }
}

/// **Read one scenario file**, refusing anything it cannot read whole.
pub fn parse(path: &'static str, text: &str) -> Result<Scenario, ScenarioError> {
    let fault = |line: usize, what: String, fix: &str| ScenarioError {
        path: path.to_owned(),
        line,
        what,
        fix: fix.to_owned(),
    };
    let cast = people::cast();
    let mut id: Option<String> = None;
    let mut seed: Option<u64> = None;
    let mut map: Option<String> = None;
    let mut treasury: Option<i64> = None;
    let mut director: Option<bool> = None;
    let mut openings: Vec<Opening> = Vec::new();
    let mut pins: Vec<Pin> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        if !raw
            .chars()
            .all(|glyph| glyph == '\t' || (' '..='~').contains(&glyph))
        {
            return Err(fault(
                line,
                "the line is not printable ASCII".to_owned(),
                "a scenario file is ASCII, like every other data format in this game",
            ));
        }
        let body = raw.split('#').next().unwrap_or("").trim();
        if body.is_empty() {
            continue;
        }
        let words: Vec<&str> = body.split_whitespace().collect();
        let once = |seen: bool, key: &str| {
            if seen {
                Err(fault(
                    line,
                    format!("{key:?} is given twice"),
                    "state each header once",
                ))
            } else {
                Ok(())
            }
        };
        let one = |words: &[&str], key: &str| -> Result<String, ScenarioError> {
            match words {
                [_, value] => Ok((*value).to_owned()),
                _ => Err(fault(
                    line,
                    format!(
                        "{key:?} takes exactly one value and was given {}",
                        words.len() - 1
                    ),
                    &format!("write `{key} <value>`"),
                )),
            }
        };
        match words.first().copied().unwrap_or("") {
            "scenario" => {
                once(id.is_some(), "scenario")?;
                let value = one(&words, "scenario")?;
                if value.is_empty()
                    || !value
                        .chars()
                        .all(|glyph| glyph.is_ascii_lowercase() || glyph == '-')
                {
                    return Err(fault(
                        line,
                        format!("the id {value:?} is not stamp-shaped"),
                        "an id is lowercase letters and hyphens, as every stamp's ids are",
                    ));
                }
                id = Some(value);
            }
            "seed" => {
                once(seed.is_some(), "seed")?;
                let value = one(&words, "seed")?;
                seed = Some(value.parse::<u64>().map_err(|_| {
                    fault(
                        line,
                        format!("the seed {value:?} is not a whole number"),
                        "write `seed 0`",
                    )
                })?);
            }
            "map" => {
                once(map.is_some(), "map")?;
                let value = one(&words, "map")?;
                if value != MAP {
                    return Err(fault(
                        line,
                        format!("the map {value:?} is not one this build has"),
                        "this build has one map, kawaza; map generation is a post-GDD session",
                    ));
                }
                map = Some(value);
            }
            "treasury" => {
                once(treasury.is_some(), "treasury")?;
                let value = one(&words, "treasury")?;
                treasury = Some(gold(&value).ok_or_else(|| {
                    fault(
                        line,
                        format!("the treasury {value:?} is not a whole number of gold, 0 or more"),
                        "write `treasury 0`",
                    )
                })?);
            }
            "director" => {
                once(director.is_some(), "director")?;
                let value = one(&words, "director")?;
                director = Some(match value.as_str() {
                    "on" => true,
                    "off" => false,
                    _ => {
                        return Err(fault(
                            line,
                            format!("the director is {value:?}"),
                            "write `director on` or `director off`",
                        ));
                    }
                });
            }
            "person" => openings.push(
                person(&words, &cast, openings.len())
                    .map_err(|(what, fix)| fault(line, what, &fix))?,
            ),
            "pin" => pins.push(pin(&words, &cast).map_err(|(what, fix)| fault(line, what, &fix))?),
            other => {
                return Err(fault(
                    line,
                    format!("{other:?} is not a statement a scenario file has"),
                    "the statements are scenario, seed, map, treasury, director, person and pin \
                     - the comment at the top of scenarios/freeplay.txt lists them",
                ));
            }
        }
    }
    let missing = |key: &str| {
        fault(
            0,
            format!("the file never says {key:?}"),
            &format!("every scenario states its {key}"),
        )
    };
    let id = id.ok_or_else(|| missing("scenario"))?;
    let seed = seed.ok_or_else(|| missing("seed"))?;
    let map = map.ok_or_else(|| missing("map"))?;
    let treasury = treasury.ok_or_else(|| missing("treasury"))?;
    let director = director.ok_or_else(|| missing("director"))?;
    if openings.len() != cast.len() {
        return Err(fault(
            0,
            format!(
                "the file opens {} people and the cast is {}",
                openings.len(),
                cast.len()
            ),
            "name every person of the cast, in registry order - the party list and the roster \
             are one list, so a scenario with fewer people is a design question, not a file",
        ));
    }
    if !openings.iter().any(|opening| opening.arrives == 0) {
        return Err(fault(
            0,
            "nobody is in the camp at minute zero".to_owned(),
            "at least one person arrives at 0, or the world opens on an empty camp",
        ));
    }
    if openings
        .windows(2)
        .any(|pair| pair[1].arrives < pair[0].arrives)
    {
        return Err(fault(
            0,
            "the arrival column is out of order".to_owned(),
            "the roster is read top to bottom as the order the camp filled up (CAST.md s4)",
        ));
    }
    Ok(Scenario {
        id,
        seed,
        map,
        treasury,
        director,
        people: openings,
        pins,
        path,
    })
}

/// A whole number of gold, zero or more.
fn gold(text: &str) -> Option<i64> {
    text.parse::<i64>().ok().filter(|value| *value >= 0)
}

/// `person <id> wallet <gold> desperation <0-10> arrives <minute>`, the
/// `place`-th of the file.
fn person(words: &[&str], cast: &[Character], place: usize) -> Result<Opening, (String, String)> {
    let shape = "write `person <id> wallet <gold> desperation <0-10> arrives <world-minute>`";
    let [
        _,
        who,
        "wallet",
        wallet,
        "desperation",
        desperation,
        "arrives",
        arrives,
    ] = words
    else {
        return Err((
            format!(
                "the person line {:?} is not in the person shape",
                words.join(" ")
            ),
            shape.to_owned(),
        ));
    };
    let Some(row) = cast.get(place) else {
        return Err((
            format!("{who:?} is one person more than the cast has"),
            "the cast is people::cast; a scenario opens exactly them".to_owned(),
        ));
    };
    if row.id != *who {
        return Err((
            format!(
                "person {} is {who:?} and the cast's {} is {:?}",
                place + 1,
                place + 1,
                row.id
            ),
            "name the cast in registry order (people::cast)".to_owned(),
        ));
    }
    let wallet = gold(wallet).ok_or_else(|| {
        (
            format!("{who}'s wallet {wallet:?} is not a whole number of gold, 0 or more"),
            shape.to_owned(),
        )
    })?;
    let desperation = desperation
        .parse::<i64>()
        .ok()
        .filter(|value| (0..=people::DESPERATION_MAX).contains(value))
        .ok_or_else(|| {
            (
                format!(
                    "{who}'s desperation {desperation:?} is not a whole number from 0 to {}",
                    people::DESPERATION_MAX
                ),
                shape.to_owned(),
            )
        })?;
    let arrives = arrives.parse::<u64>().map_err(|_| {
        (
            format!("{who}'s arrival {arrives:?} is not a world-minute"),
            shape.to_owned(),
        )
    })?;
    Ok(Opening {
        id: row.id,
        wallet,
        desperation,
        arrives,
    })
}

/// `pin <template> at <minute> [for <person>]`.
fn pin(words: &[&str], cast: &[Character]) -> Result<Pin, (String, String)> {
    let shape = "write `pin <template> at <world-minute>` or `... for <person>`";
    let (template, at, who) = match words {
        [_, template, "at", at] => (*template, *at, None),
        [_, template, "at", at, "for", who] => (*template, *at, Some(*who)),
        _ => {
            return Err((
                format!("the pin {:?} is not in the pin shape", words.join(" ")),
                format!(
                    "{shape}; a predicate pin (GDD s6) is recorded in the format and not built \
                     - the tutorial that needs one is post-MVP"
                ),
            ));
        }
    };
    let Some(row) = petitions::find(template) else {
        return Err((
            format!("the pin names {template:?}, which is not a template"),
            "pin a row of petitions::TEMPLATES by its id".to_owned(),
        ));
    };
    if row.source != Source::Director {
        return Err((
            format!(
                "the pin names {template:?}, which is {}-sourced",
                row.source.class()
            ),
            "a pin is the director speaking at a scripted minute, so it fires a \
             director-sourced template (D1-D3); the cast's own wants are theirs to voice"
                .to_owned(),
        ));
    }
    let at = at.parse::<u64>().map_err(|_| {
        (
            format!("the pin's minute {at:?} is not a world-minute"),
            shape.to_owned(),
        )
    })?;
    let who = match who {
        None => None,
        Some(id) => {
            let Some(index) = cast.iter().position(|person| person.id == id) else {
                return Err((
                    format!("the pin is for {id:?}, who is not in the cast"),
                    "name a person by their people::cast id".to_owned(),
                ));
            };
            let reachable = match row.trigger {
                Trigger::Carries { any, .. } => cast
                    .get(index)
                    .is_some_and(|person| any.iter().any(|id| person.traits.contains(id))),
                _ => false,
            };
            if !reachable {
                return Err((
                    format!("{template:?} cannot reach {id}: they carry none of its traits"),
                    "pin it for somebody its trigger names, or leave `for` off and let the \
                     director choose"
                        .to_owned(),
                ));
            }
            Some(index)
        }
    };
    Ok(Pin {
        template: row,
        at,
        who,
    })
}

/// **The format's own validation**: every file this build carries parses,
/// freeplay is first and is the authored start, and the parser refuses what
/// it should — by name, never by silence.
pub fn vocabulary(checks: &mut crate::checks::Checks) {
    checks.require(
        all().len() == FILES.len()
            && all()
                .first()
                .is_some_and(|scenario| scenario.id == FREEPLAY),
        "the scenario files are not the ones this build carries, freeplay first",
        format!(
            "{} parsed of {} files; the first is {:?}",
            all().len(),
            FILES.len(),
            all().first().map(|scenario| scenario.id.as_str())
        ),
    );
    for scenario in all() {
        checks.require(
            all().iter().filter(|other| other.id == scenario.id).count() == 1,
            "two scenario files share an id",
            format!(
                "{:?} ({}) is the id of more than one file",
                scenario.id, scenario.path
            ),
        );
    }
    let free = freeplay();
    checks.require(
        free.director && free.pins.is_empty() && free.seed == 0 && free.treasury == 0,
        "freeplay is not the least-pinned scenario with the director speaking",
        format!(
            "freeplay reads director {} with {} pins at seed {} and {}g; GDD s6: freeplay is \
             the least pinned, and the injector is what makes its world move without you",
            free.director,
            free.pins.len(),
            free.seed,
            free.treasury
        ),
    );
    let base = FILES[0].1;
    for (broken, why) in [
        (
            base.replace("director on", "director maybe"),
            "director maybe",
        ),
        (base.replace("map kawaza", "map elsewhere"), "map elsewhere"),
        (
            base.replace("person odd ", "person ood "),
            "a misnamed person",
        ),
        (
            base.replace("desperation 4", "desperation 11"),
            "desperation 11",
        ),
        (base.replace("wallet 6 ", "wallet -6 "), "a negative purse"),
        (base.replace("seed 0", "seed 0\nseed 1"), "two seeds"),
        (base.replace("treasury 0", ""), "no treasury"),
        (format!("{base}\nweather rain\n"), "an unknown statement"),
        (
            format!("{base}\npin collectors-visit at 90\n"),
            "a pin of a want's template",
        ),
        (
            format!("{base}\npin the-collector-comes at 90 for steve\n"),
            "a pin for somebody it cannot reach",
        ),
        (
            format!("{base}\npin the-collector-comes when broke\n"),
            "a predicate pin",
        ),
    ] {
        let refused = parse("scenarios/broken.txt", &broken);
        checks.require(
            refused
                .as_ref()
                .is_err_and(|error| !error.what.is_empty() && !error.fix.is_empty()),
            "a broken scenario file was read rather than refused",
            format!("{why} parsed as {:?}", refused.map(|scenario| scenario.id)),
        );
    }
}
