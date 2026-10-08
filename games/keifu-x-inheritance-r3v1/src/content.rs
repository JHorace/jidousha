//! Loading `spec/content/*.json` into typed tables, validated on load.
//!
//! The files are consumed as they are: `include_str!` bakes each one into the
//! binary at build time (the same bytes on native and on the web, and no asset
//! load to wait for), and `load` parses and checks them. Nothing hand-authored
//! is retyped here — every string the player reads is copied out of a file by
//! key. A file that does not match its schema stops the game at startup with a
//! message naming the file, the path inside it, and what was expected.

use crate::constants::{BOND_RANK_POWER_GRIEF, bond_mirror};
use crate::constants::{
    CROWN_RENOWN, DOOR_DESTINY_POWER, MAXIMUM_SEATS, OUTLIVING_DREAD, PATRON_POWER,
};
use crate::dream_lore::{DreamFormats, DreamLore, read_dream_formats, read_dreams};
use crate::household::{Founding, read_household};
use crate::ids::{Aptitude, BondKind, Destiny, Outcome, Place, Pool, Tag};
use crate::json::{At, Json, SchemaError, parse};
use crate::legacy_lore::{LegacyLore, read_legacies};
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
    /// The fewest calm seats it is posted with.
    pub seats_low: i32,
    /// The most.
    pub seats_high: i32,
    /// Its calm danger.
    pub danger: i32,
    /// "Robbers went in at dusk. Bring them out, or what is left."
    pub premise: String,
    /// The four endings (`%1` = the party's names), by `Outcome`.
    pub endings: [String; 4],
}

/// `bonds.json`.
pub struct BondLore {
    /// Kind titles, by `BondKind`.
    pub titles: Vec<String>,
    /// The phrase a pair holding this kind is named by in a power line
    /// ("parent and child"), by `BondKind`.
    pub pairs: Vec<String>,
    /// Whether the sheet shows the kind, by `BondKind`.
    pub shown: Vec<bool>,
    /// Gendered titles (HE, SHE) for the kinds that have them, by `BondKind`.
    pub gendered: Vec<Option<[String; 2]>>,
    /// The kinship telling of a bond's other hero, by `BondKind`, then by the
    /// other's pronoun (HE, SHE); the same word twice for an ungendered kind.
    pub kinship: Vec<[String; 2]>,
    /// "brother", "sister": kin by a shared parent, between two of a party (SPEC §21).
    pub sibling: [String; 2],
    /// "companion": two of a party with no bond between them (SPEC §21).
    pub no_bond: String,
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
    /// `quests.json` `opening_quests`: the templates forced onto year 1's board, by index.
    pub opening_quests: Vec<usize>,
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
    /// `ghost.json`: a ghost quest's title and premise.
    pub ghost: crate::ghost::GhostLore,
    /// `door.json`: the three locks, the verdicts and the closed house's (SPEC §16, §23).
    /// The Door's tags come from lore.
    pub door: crate::door_lore::DoorLore,
    /// `wanderers.json`: who a wanderer is drawn as, and the dreams a newcomer rolls.
    pub wanderers: crate::turning_lore::WandererLore,
    /// `ui-text.json` and `lines.json`, the keys this build reads.
    pub words: Words,
    /// `epitaph.json`, with the words its parts borrow (SPEC §20).
    pub epitaph: crate::epitaph_lore::EpitaphLore,
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
    let at = |name: &'static str| -> Result<At<'_>, SchemaError> { Ok(At::root(doc(name)?, name)) };
    let lore = crate::lore::read_lore(&at("lore.json")?)?;
    let destinies = at("destinies.json")?;
    let dreams_at = at("dreams.json")?;
    Ok(Content {
        destinies: read_destinies(&destinies)?,
        quest_templates: read_quest_templates(&at("quests.json")?)?,
        opening_quests: read_opening_quests(&at("quests.json")?)?,
        blood_of_prophecy: text_at(&destinies, "blood_of_prophecy")?,
        bonds: read_bonds(&at("bonds.json")?)?,
        dreams: read_dreams(&dreams_at)?,
        dream_formats: read_dream_formats(&dreams_at)?,
        legacies: read_legacies(&at("legacies.json")?)?,
        pools: read_pools(&at("writing.json")?)?,
        names: Names {
            him: strings(&at("names.json")?, "names_for_him")?,
            her: strings(&at("names.json")?, "names_for_her")?,
            houses: strings(&at("names.json")?, "houses")?,
        },
        founding: read_household(&at("household.json")?)?,
        door: crate::door_lore::read_door(&at("door.json")?)?,
        ghost: crate::ghost::read_ghost(&at("ghost.json")?)?,
        wanderers: crate::turning_lore::read_wanderers(&at("wanderers.json")?)?,
        words: read_words(at("ui-text.json")?, at("lines.json")?)?,
        epitaph: crate::epitaph_lore::read_epitaph(
            &at("epitaph.json")?,
            &dreams_at,
            &at("legacies.json")?,
            &at("door.json")?,
        )?,
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
            let (seats_low, seats_high) = (
                item.key("seats_low")?.int()?,
                item.key("seats_high")?.int()?,
            );
            let danger = item.key("danger")?.int()?;
            if !(1..=seats_high).contains(&seats_low)
                || seats_high > MAXIMUM_SEATS as i32
                || danger < 0
            {
                return Err(item.reject(format!(
                    "seats {seats_low}..{seats_high} or danger {danger} cannot be posted \
                     (SPEC §5.2 needs 1 <= seats_low <= seats_high <= MAXIMUM_SEATS, danger >= 0)"
                )));
            }
            Ok(QuestTemplate {
                place: id_at(item, "place", Place::find)?,
                title: text(item, "title")?,
                aptitude: id_at(item, "aptitude", Aptitude::find)?,
                tags,
                seats_low,
                seats_high,
                danger,
                premise: text(item, "premise")?,
                endings: read_endings(item)?,
            })
        })
        .collect()
}

/// A quest's or a ghost's four endings, by `Outcome` (SPEC §7.1 step 2).
pub fn read_endings(at: &At<'_>) -> Result<[String; 4], SchemaError> {
    let endings = at.key("endings")?;
    Ok([
        text(&endings, Outcome::Disaster.id())?,
        text(&endings, Outcome::Setback.id())?,
        text(&endings, Outcome::Success.id())?,
        text(&endings, Outcome::Triumph.id())?,
    ])
}

/// The opening quests, each named by a template's title.
fn read_opening_quests(at: &At<'_>) -> Result<Vec<usize>, SchemaError> {
    let templates = read_quest_templates(at)?;
    strings(at, "opening_quests")?
        .iter()
        .map(|title| {
            templates
                .iter()
                .position(|t| &t.title == title)
                .ok_or_else(|| at.reject(format!("opening quest {title:?} names no template")))
        })
        .collect()
}

fn read_bonds(at: &At<'_>) -> Result<BondLore, SchemaError> {
    let kinds = table(at, "kinds", BondKind::ALL, BondKind::id)?;
    let gendered = at.key("gendered_titles")?;
    let kinship = at.key("kinship_tellings")?;
    let mut lore = BondLore {
        titles: Vec::new(),
        pairs: Vec::new(),
        shown: Vec::new(),
        gendered: Vec::new(),
        kinship: Vec::new(),
        sibling: {
            let forms = kinship.key("sibling")?;
            [text(&forms, "HE")?, text(&forms, "SHE")?]
        },
        no_bond: text(&kinship, "no_bond")?,
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
        lore.pairs.push(text(item, "pair")?);
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
