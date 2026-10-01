//! Loading `spec/content/*.json` into typed tables, validated on load.
//!
//! The files are consumed as they are: `include_str!` bakes each one into the
//! binary at build time (the same bytes on native and on the web, and no asset
//! load to wait for), and `load` parses and checks them. Nothing hand-authored
//! is retyped here — every string the player reads is copied out of a file by
//! key. A file that does not match its schema stops the game at startup with a
//! message naming the file, the path inside it, and what was expected.

use crate::constants::{BOND_RANK_POWER_GRIEF, DOOR_LOCKS, DOOR_YEARS, bond_mirror};
use crate::constants::{CROWN_RENOWN, DOOR_DESTINY_POWER, OUTLIVING_DREAD, PATRON_POWER};
use crate::household::{Founding, read_household};
use crate::ids::{Aptitude, BondKind, Destiny, DreamKind, LegacyKind, Place, Pool, Tag};
use crate::json::{At, Json, SchemaError, parse};
use crate::words::{Words, read_words};

/// Every content file, by name, as baked in.
pub const FILES: [(&str, &str); 15] = [
    ("lore.json", include_str!("../spec/content/lore.json")),
    (
        "destinies.json",
        include_str!("../spec/content/destinies.json"),
    ),
    ("bonds.json", include_str!("../spec/content/bonds.json")),
    ("dreams.json", include_str!("../spec/content/dreams.json")),
    (
        "legacies.json",
        include_str!("../spec/content/legacies.json"),
    ),
    ("writing.json", include_str!("../spec/content/writing.json")),
    ("names.json", include_str!("../spec/content/names.json")),
    (
        "household.json",
        include_str!("../spec/content/household.json"),
    ),
    ("door.json", include_str!("../spec/content/door.json")),
    ("ui-text.json", include_str!("../spec/content/ui-text.json")),
    ("lines.json", include_str!("../spec/content/lines.json")),
    ("quests.json", include_str!("../spec/content/quests.json")),
    ("ghost.json", include_str!("../spec/content/ghost.json")),
    ("epitaph.json", include_str!("../spec/content/epitaph.json")),
    (
        "wanderers.json",
        include_str!("../spec/content/wanderers.json"),
    ),
];

/// The top-level keys the files no W0/W1 rule reads must still carry.
///
/// Typed reading of these lands with the wave that uses them; until then their
/// shape is held at the top level so a renamed or missing table fails now.
const LATER_WAVES: [(&str, &[&str]); 4] = [
    ("quests.json", &["opening_quests"]),
    (
        "ghost.json",
        &["title", "aptitude", "seats", "danger", "premise", "endings"],
    ),
    (
        "epitaph.json",
        &[
            "parts",
            "frames",
            "priorities",
            "sentence_limit",
            "templates",
        ],
    ),
    (
        "wanderers.json",
        &["age_low", "age_high", "standings", "dreams"],
    ),
];

/// A destiny's three lines.
pub struct DestinyLore {
    /// "Your child will surpass you."
    pub prophecy: String,
    /// "Doom: ..."
    pub doom: String,
    /// "Gift: ..." (empty for UNSPOKEN).
    pub gift: String,
    /// Whether the Seer can speak it (SPEC §13): all but UNSPOKEN and the Door.
    pub speakable: bool,
}

/// A quest template, the parts W2's power sum reads. W4/W5 read the rest.
pub struct QuestTemplate {
    /// Where.
    pub place: Place,
    /// "The bell under the tide".
    pub title: String,
    /// What it needs.
    pub aptitude: Aptitude,
    /// What it carries (may be a subset of its place's tags).
    pub tags: Vec<Tag>,
}

/// `bonds.json`.
pub struct BondLore {
    /// Kind titles, by `BondKind`.
    pub titles: Vec<String>,
    /// Whether the sheet shows the kind, by `BondKind`.
    pub shown: Vec<bool>,
    /// Gendered titles (HE, SHE) for the kinds that have them, by `BondKind`.
    pub gendered: Vec<Option<[String; 2]>>,
    /// The kinship telling of a bond's other hero, by `BondKind`, then by the
    /// other's pronoun (HE, SHE); the same word twice for an ungendered kind.
    pub kinship: Vec<[String; 2]>,
}

/// One stage of a dream, as authored.
pub struct StageLore {
    /// "Win a triumph at the Barrow", or a `%` template.
    pub task: String,
    /// Whether `task` takes the setup argument.
    pub templated: bool,
    /// How many times the stage must be met.
    pub goal: i32,
}

/// One dream, as authored.
pub struct DreamLore {
    /// "To lay the Barrow's dead to rest", or a `%` template.
    pub title: String,
    /// For a templated title: the argument with no lost hero, for HE, for SHE.
    pub title_arguments: Option<[String; 3]>,
    /// What it leaves when fulfilled.
    pub legacy: LegacyKind,
    /// The three stages.
    pub stages: Vec<StageLore>,
}

/// `dreams.json`'s format pieces.
pub struct DreamFormats {
    /// "% (%/%)".
    pub progress: String,
    /// " my ".
    pub pronoun_find: String,
    /// " % ".
    pub pronoun_replace: String,
    /// " you".
    pub object_suffix: String,
    /// "% %".
    pub object_format: String,
}

/// `legacies.json`, the parts a sheet reads.
pub struct LegacyLore {
    /// "If it is ever done, ..." by `LegacyKind`.
    pub promises: Vec<String>,
    /// "+% % on quests."
    pub heirloom_effect: String,
}

/// `names.json`.
pub struct Names {
    /// Thirty-two names for him.
    pub him: Vec<String>,
    /// Thirty-two names for her.
    pub her: Vec<String>,
    /// Sixteen houses.
    pub houses: Vec<String>,
}

/// Everything in `spec/content/`, typed.
pub struct Content {
    /// `lore.json`.
    pub lore: crate::lore::Lore,
    /// `destinies.json`, by `Destiny`.
    pub destinies: Vec<DestinyLore>,
    /// "Blood of %: %".
    pub blood_of_prophecy: String,
    /// `bonds.json`.
    pub bonds: BondLore,
    /// `quests.json` templates, in file order.
    pub quest_templates: Vec<QuestTemplate>,
    /// `dreams.json`, by `DreamKind`.
    pub dreams: Vec<DreamLore>,
    /// `dreams.json`'s format pieces.
    pub dream_formats: DreamFormats,
    /// `legacies.json`.
    pub legacies: LegacyLore,
    /// `writing.json` pools, by `Pool`.
    pub pools: Vec<Vec<String>>,
    /// `names.json`.
    pub names: Names,
    /// `household.json`.
    pub founding: Founding,
    /// `door.json`: the Door's tags come from lore; its lock demands from here.
    pub door_locks: Vec<i32>,
    /// `ui-text.json` and `lines.json`, the keys this build reads.
    pub words: Words,
}

/// Parse and validate every content file.
pub fn load() -> Result<Content, SchemaError> {
    let mut docs = Vec::new();
    for (name, text) in FILES {
        docs.push((name, parse(name, text)?));
    }
    let doc = |name: &str| -> Result<&Json, SchemaError> {
        docs.iter()
            .find(|(file, _)| *file == name)
            .map(|(_, json)| json)
            .ok_or_else(|| SchemaError {
                at: name.to_owned(),
                what: "the file is not in the baked-in list".to_owned(),
            })
    };
    for (file, keys) in LATER_WAVES {
        let root = At::root(doc(file)?, file);
        for key in keys {
            root.key(key)?;
        }
    }
    let at = |name: &'static str| -> Result<At<'_>, SchemaError> { Ok(At::root(doc(name)?, name)) };
    let lore = crate::lore::read_lore(&at("lore.json")?)?;
    let destinies = at("destinies.json")?;
    let dreams_at = at("dreams.json")?;
    Ok(Content {
        destinies: read_destinies(&destinies)?,
        quest_templates: read_quest_templates(&at("quests.json")?)?,
        blood_of_prophecy: text_at(&destinies, "blood_of_prophecy")?,
        bonds: read_bonds(&at("bonds.json")?)?,
        dreams: table(&dreams_at, "dreams", DreamKind::ALL, DreamKind::id)?
            .iter()
            .map(read_dream)
            .collect::<Result<_, SchemaError>>()?,
        dream_formats: read_dream_formats(&dreams_at)?,
        legacies: read_legacies(&at("legacies.json")?)?,
        pools: read_pools(&at("writing.json")?)?,
        names: Names {
            him: strings(&at("names.json")?, "names_for_him")?,
            her: strings(&at("names.json")?, "names_for_her")?,
            houses: strings(&at("names.json")?, "houses")?,
        },
        founding: read_household(&at("household.json")?)?,
        door_locks: read_door(&at("door.json")?)?,
        words: read_words(at("ui-text.json")?, at("lines.json")?)?,
        lore,
    })
}

/// A string under `key` of a located object.
pub fn text(owned: &At<'_>, key: &str) -> Result<String, SchemaError> {
    text_at(owned, key)
}

/// A string under `key` of an `At`.
pub fn text_at(at: &At<'_>, key: &str) -> Result<String, SchemaError> {
    at.key(key)?.str()
}

/// An array of strings under `key`.
pub fn strings(at: &At<'_>, key: &str) -> Result<Vec<String>, SchemaError> {
    at.key(key)?
        .items()?
        .iter()
        .map(|item| item.str())
        .collect()
}

/// An enum id under `key`.
pub fn id_at<T>(at: &At<'_>, key: &str, find: fn(&str) -> Option<T>) -> Result<T, SchemaError> {
    let owned = at.key(key)?;
    let id = owned.str()?;
    find(&id).ok_or_else(|| owned.reject(format!("{id:?} is not a known id")))
}

/// An array under `key` whose entries carry `id`s in exactly canonical order.
pub fn table<'a, T: Copy>(
    at: &At<'a>,
    key: &str,
    all: &[T],
    id: fn(T) -> &'static str,
) -> Result<Vec<At<'a>>, SchemaError> {
    let owned = at.key(key)?;
    let items = owned.items()?;
    let found: Vec<String> = items
        .iter()
        .map(|item| text(item, "id"))
        .collect::<Result<_, _>>()?;
    let want: Vec<&str> = all.iter().map(|value| id(*value)).collect();
    if found != want {
        return Err(owned.reject(format!(
            "ids are {found:?}; the canonical order (content/README.md) is {want:?}"
        )));
    }
    Ok(items)
}

/// The destinies, with every number a doom or gift renders checked against
/// CONSTANTS.md, and the speakable set checked against SPEC §13.
fn read_destinies(at: &At<'_>) -> Result<Vec<DestinyLore>, SchemaError> {
    // (destiny, template key, the constant it renders) for the numbers W2's rules read.
    let rendered: [(Destiny, &str, &str, i32); 4] = [
        (
            Destiny::OutliveThoseYouLove,
            "doom_template",
            "doom",
            OUTLIVING_DREAD,
        ),
        (Destiny::WearACrown, "doom_template", "doom", CROWN_RENOWN),
        (Destiny::WearACrown, "gift_template", "gift", PATRON_POWER),
        (
            Destiny::OpenTheSealedDoor,
            "gift_template",
            "gift",
            DOOR_DESTINY_POWER,
        ),
    ];
    let items = table(at, "destinies", Destiny::ALL, Destiny::id)?;
    let mut out = Vec::new();
    for (kind, d) in Destiny::ALL.iter().zip(&items) {
        for (destiny, template, field, value) in rendered {
            if destiny == *kind
                && crate::text::fmt(&text(d, template)?, &[&value.to_string()]) != text(d, field)?
            {
                return Err(d.reject(format!(
                    "{field} does not render {template} with {value} (CONSTANTS.md §8)"
                )));
            }
        }
        let speakable = d.key("speakable")?.bool()?;
        if speakable != !matches!(kind, Destiny::Unspoken | Destiny::OpenTheSealedDoor) {
            return Err(d.reject(
                "speakable disagrees with SPEC §13 (all but UNSPOKEN and the Door)".into(),
            ));
        }
        out.push(DestinyLore {
            prophecy: text(d, "prophecy")?,
            doom: text(d, "doom")?,
            gift: text(d, "gift")?,
            speakable,
        });
    }
    let order = strings(at, "speakable_order")?;
    let want: Vec<&str> = Destiny::ALL
        .iter()
        .filter(|kind| out[kind.index()].speakable)
        .map(|kind| kind.id())
        .collect();
    if order != want {
        return Err(at.reject(format!(
            "speakable_order is {order:?}; the speakable destinies in canonical order are {want:?}"
        )));
    }
    Ok(out)
}

fn read_quest_templates(at: &At<'_>) -> Result<Vec<QuestTemplate>, SchemaError> {
    at.key("templates")?
        .items()?
        .iter()
        .map(|item| {
            let tags = strings(item, "tags")?
                .iter()
                .map(|id| Tag::find(id).ok_or_else(|| item.reject(format!("tag {id:?}"))))
                .collect::<Result<_, _>>()?;
            Ok(QuestTemplate {
                place: id_at(item, "place", Place::find)?,
                title: text(item, "title")?,
                aptitude: id_at(item, "aptitude", Aptitude::find)?,
                tags,
            })
        })
        .collect()
}

fn read_bonds(at: &At<'_>) -> Result<BondLore, SchemaError> {
    let kinds = table(at, "kinds", BondKind::ALL, BondKind::id)?;
    let gendered = at.key("gendered_titles")?;
    let kinship = at.key("kinship_tellings")?;
    let mut lore = BondLore {
        titles: Vec::new(),
        shown: Vec::new(),
        gendered: Vec::new(),
        kinship: Vec::new(),
    };
    for (kind, item) in BondKind::ALL.iter().zip(&kinds) {
        let numbers = (
            item.key("rank")?.int()?,
            item.key("power")?.int()?,
            item.key("grief")?.int()?,
        );
        let mirror = id_at(item, "mirror", BondKind::find)?;
        if numbers != BOND_RANK_POWER_GRIEF[kind.index()] || mirror != bond_mirror(*kind) {
            return Err(item.reject(format!(
                "rank/power/grief {numbers:?} mirror {} disagree with CONSTANTS.md §7",
                mirror.id()
            )));
        }
        lore.titles.push(text(item, "title")?);
        lore.shown.push(item.key("shown")?.bool()?);
        lore.gendered.push(match gendered.find(kind.id())? {
            Some(forms) => Some([text(&forms, "HE")?, text(&forms, "SHE")?]),
            None => None,
        });
        // A kind the table does not name takes `otherwise` ("friend").
        let telling = match kinship.find(kind.id())? {
            Some(forms) => match forms.str() {
                Ok(word) => [word.clone(), word],
                Err(_) => [text(&forms, "HE")?, text(&forms, "SHE")?],
            },
            None => {
                let word = text(&kinship, "otherwise")?;
                [word.clone(), word]
            }
        };
        lore.kinship.push(telling);
    }
    Ok(lore)
}

fn read_dream(item: &At<'_>) -> Result<DreamLore, SchemaError> {
    let at = item;
    let templated = |owned: &At<'_>, key: &str| -> Result<bool, SchemaError> {
        Ok(match owned.find(key)? {
            Some(flag) => flag.bool()?,
            None => false,
        })
    };
    let title_arguments = if templated(item, "title_is_template")? {
        let args = at.key("title_argument")?;
        Some([
            text(&args, "no_lost_hero")?,
            text(&args, "lost_hero_HE")?,
            text(&args, "lost_hero_SHE")?,
        ])
    } else {
        None
    };
    let stages = at
        .key("stages")?
        .items()?
        .iter()
        .map(|stage| {
            Ok(StageLore {
                task: text(stage, "task")?,
                templated: templated(stage, "task_is_template")?,
                goal: stage.key("goal")?.int()?,
            })
        })
        .collect::<Result<Vec<_>, SchemaError>>()?;
    if stages.len() != crate::constants::DREAM_STAGE_COUNT {
        return Err(at.reject(format!("{} stages; a dream has 3", stages.len())));
    }
    Ok(DreamLore {
        title: text(item, "title")?,
        title_arguments,
        legacy: id_at(at, "legacy", LegacyKind::find)?,
        stages,
    })
}

fn read_dream_formats(at: &At<'_>) -> Result<DreamFormats, SchemaError> {
    let swap = at.key("title_pronoun_swap")?;
    let object = at.key("task_object_swap")?;
    Ok(DreamFormats {
        progress: text_at(at, "progress_format")?,
        pronoun_find: text(&swap, "find")?,
        pronoun_replace: text(&swap, "replace_with")?,
        object_suffix: text(&object, "suffix")?,
        object_format: text(&object, "format")?,
    })
}

fn read_legacies(at: &At<'_>) -> Result<LegacyLore, SchemaError> {
    let promises = at.key("promises")?;
    Ok(LegacyLore {
        promises: LegacyKind::ALL
            .iter()
            .map(|kind| text(&promises, kind.id()))
            .collect::<Result<_, _>>()?,
        heirloom_effect: text_at(at, "heirloom_effect")?,
    })
}

fn read_pools(at: &At<'_>) -> Result<Vec<Vec<String>>, SchemaError> {
    let pools = at.key("pools")?;
    let entries = pools.entries()?;
    let names: Vec<&str> = entries.iter().map(|(name, _)| name.as_str()).collect();
    let want: Vec<&str> = Pool::ALL.iter().map(|pool| pool.id()).collect();
    if names != want {
        return Err(pools.reject(format!("pools are {names:?}, expected {want:?}")));
    }
    entries
        .iter()
        .map(|(_, pool)| {
            let lines = strings(pool, "lines")?;
            if lines.len() < 2 {
                return Err(pool.reject("a pool needs two lines to never repeat".into()));
            }
            Ok(lines)
        })
        .collect()
}

fn read_door(at: &At<'_>) -> Result<Vec<i32>, SchemaError> {
    let years = at.key("years")?.int()?;
    let locks = at.key("locks")?;
    let mut demands = Vec::new();
    for (aptitude, lock) in Aptitude::ALL.iter().zip(locks.items()?) {
        let named = id_at(&lock, "aptitude", Aptitude::find)?;
        demands.push(lock.key("demand")?.int()?);
        if named != *aptitude {
            return Err(lock.reject("locks are not in Might, Wits, Spirit order".into()));
        }
    }
    if years != DOOR_YEARS || demands != DOOR_LOCKS {
        return Err(at.reject(format!(
            "years {years} and lock demands {demands:?} disagree with CONSTANTS.md §1/§12"
        )));
    }
    Ok(demands)
}
