//! `epitaph.json` (SPEC §20, `content/README.md`), typed: the nine parts, the three
//! frames, the priority order, the sentence budget and every template by key — plus the
//! words the parts borrow from other files: `dreams.json`'s progress tellings,
//! `legacies.json`'s legacy nouns and `door.json`'s lock names.
//!
//! Every list is checked against the canonical part order on load, and the budget and
//! the frame count against CONSTANTS §13, so a reordered or renumbered file stops the
//! game at startup rather than shifting a coin or a frame.

use crate::constants::{EPITAPH_FRAME_COUNT, EPITAPH_SENTENCES};
use crate::content::{strings, text_at};
use crate::ids::{LegacyKind, Part};
use crate::json::{At, SchemaError};

/// Declares `E`, one variant per template key, and the table of keys.
macro_rules! templates {
    ($($variant:ident = $path:literal),+ $(,)?) => {
        /// One epitaph template, by its content key (`epitaph.<key>`).
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum E {
            $(
                #[doc = $path]
                $variant
            ),+
        }

        const KEYS: &[(E, &str)] = &[$((E::$variant, $path)),+];
    };
}

templates! {
    OriginArrived1 = "origin.arrived_1",
    OriginArrived0 = "origin.arrived_0",
    OriginBorn1 = "origin.born_1",
    OriginBorn0 = "origin.born_0",
    OriginChildOfOld = "origin.child_of_old",
    OriginFounder1 = "origin.founder_1",
    OriginFounder0 = "origin.founder_0",
    RoadsNeverOldEnough = "roads.never_old_enough",
    RoadsKeptHouse1 = "roads.kept_house_1",
    RoadsNeverWent0 = "roads.never_went_0",
    RoadsCountOnly = "roads.count_only",
    RoadsOnce = "roads.once",
    RoadsMany1 = "roads.many_1",
    RoadsMany0 = "roads.many_0",
    RoadsChildhoodTeachers = "roads.childhood_teachers",
    TriumphSpokenOf1 = "triumph.spoken_of_1",
    TriumphBestDay0 = "triumph.best_day_0",
    FearBornBrave = "fear.born_brave",
    FearConquered1 = "fear.conquered_1",
    FearConquered0 = "fear.conquered_0",
    FearBrokenByGrief = "fear.broken_by_grief",
    FearBroken1 = "fear.broken_1",
    FearBroken0 = "fear.broken_0",
    FearFaced1 = "fear.faced_1",
    FearFaced0 = "fear.faced_0",
    FearChild = "fear.child",
    FearKeptFrom = "fear.kept_from",
    DreamInheritedSuffix = "dream.inherited_suffix",
    DreamFulfilled1 = "dream.fulfilled_1",
    DreamFulfilled0 = "dream.fulfilled_0",
    DreamPassed1 = "dream.passed_1",
    DreamPassed0 = "dream.passed_0",
    DreamLeft = "dream.left",
    DreamLaid = "dream.laid",
    DreamLiving = "dream.living",
    DreamCrowned = "dream.crowned",
    DreamDied = "dream.died",
    ProphecyFireCame = "prophecy.fire_came",
    ProphecyFire = "prophecy.fire",
    ProphecyOutlive = "prophecy.outlive",
    ProphecyCrownCame = "prophecy.crown_came",
    ProphecyCrown = "prophecy.crown",
    ProphecyDoorBloodStood = "prophecy.door_blood_stood",
    ProphecyDoorBlood = "prophecy.door_blood",
    ProphecyDoorStood = "prophecy.door_stood",
    ProphecyDoor = "prophecy.door",
    ProphecySurpassCame = "prophecy.surpass_came",
    ProphecySurpass = "prophecy.surpass",
    ProphecyMendedCame = "prophecy.mended_came",
    ProphecyMended = "prophecy.mended",
    ProphecyBed = "prophecy.bed",
    ProphecyCarry = "prophecy.carry",
    ProphecyReflexiveHe = "prophecy.reflexive_HE",
    ProphecyReflexiveShe = "prophecy.reflexive_SHE",
    ProphecyGreaterCame = "prophecy.greater_came",
    ProphecyGreater = "prophecy.greater",
    LoveSpousesChildren = "love.spouses_children",
    LoveSpouses = "love.spouses",
    LoveChildren = "love.children",
    LoveChild = "love.child",
    LoveChildrenWord = "love.children_word",
    LoveSiblingFriend = "love.sibling_friend",
    LoveBrother = "love.brother",
    LoveSister = "love.sister",
    LoveFriend1 = "love.friend_1",
    LoveFriend0 = "love.friend_0",
    EndBeforeFirstYear = "end.before_first_year",
    EndDead1 = "end.dead_1",
    EndDead0 = "end.dead_0",
    EndDoorOpened = "end.door_opened",
    EndDoor = "end.door",
    EndChild = "end.child",
    EndLiving = "end.living",
    LeftDeparted = "left.departed",
    LeftLegacyAndHeirloom = "left.legacy_and_heirloom",
    LeftTale = "left.tale",
    LeftBlessing = "left.blessing",
    LeftHeirloomMade = "left.heirloom_made",
    LeftHeirloomWent = "left.heirloom_went",
    LeftBuriedWith = "left.buried_with",
    LivingMadeHeirloom = "living.made_heirloom",
    LivingMadeTale = "living.made_tale",
    LivingMadeBlessing = "living.made_blessing",
    LivingCarriedHeirloom = "living.carried_heirloom",
}

/// `epitaph.json` and the borrowed words, typed.
pub struct EpitaphLore {
    /// The three frames: the order the chosen parts are emitted in.
    pub frames: Vec<[Part; 9]>,
    /// The order parts are chosen in against the budget.
    pub priorities: [Part; 9],
    /// Every template's text, by `E`.
    templates: Vec<String>,
    /// `dreams.json` `progress_tellings`: "had not yet begun it", ... by min(current, 2).
    pub progress_tellings: [String; 3],
    /// `legacies.json` `legacy_nouns`, by `LegacyKind`.
    pub legacy_nouns: Vec<String>,
    /// `door.json` lock `name`s, Might, Wits, Spirit: "the lock of iron".
    pub lock_names: Vec<String>,
}

impl std::ops::Index<E> for EpitaphLore {
    type Output = str;

    fn index(&self, template: E) -> &str {
        &self.templates[template as usize]
    }
}

/// A list of the nine parts, each exactly once.
fn nine_parts(at: &At<'_>, key: &str) -> Result<[Part; 9], SchemaError> {
    let ids = at
        .items()?
        .iter()
        .map(|item| item.str())
        .collect::<Result<Vec<_>, _>>()?;
    let mut parts = [Part::Origin; 9];
    let mut seen = [false; 9];
    if ids.len() != Part::ALL.len() {
        return Err(at.reject(format!("{key} lists {} parts, not 9", ids.len())));
    }
    for (slot, id) in ids.iter().enumerate() {
        let Some(part) = Part::find(id) else {
            return Err(at.reject(format!("{key}: {id:?} is not a part of SPEC §20")));
        };
        if seen[part.index()] {
            return Err(at.reject(format!("{key} lists {id} twice")));
        }
        seen[part.index()] = true;
        parts[slot] = part;
    }
    Ok(parts)
}

/// Read `epitaph.json` with the three borrowed tables.
pub fn read_epitaph(
    epitaph: &At<'_>,
    dreams: &At<'_>,
    legacies: &At<'_>,
    door: &At<'_>,
) -> Result<EpitaphLore, SchemaError> {
    // The coins are indexed by `parts`: it must be the canonical order.
    let parts = strings(epitaph, "parts")?;
    let want: Vec<&str> = Part::ALL.iter().map(|p| p.id()).collect();
    if parts != want {
        return Err(epitaph.reject(format!("parts are {parts:?}; SPEC §20's order is {want:?}")));
    }
    let limit = epitaph.key("sentence_limit")?.int()?;
    if limit as usize != EPITAPH_SENTENCES {
        return Err(epitaph.reject(format!(
            "sentence_limit {limit} disagrees with CONSTANTS.md §13 ({EPITAPH_SENTENCES})"
        )));
    }
    let frames_at = epitaph.key("frames")?;
    let items = frames_at.items()?;
    if items.len() != EPITAPH_FRAME_COUNT {
        return Err(frames_at.reject(format!(
            "{} frames; CONSTANTS.md §13 has {EPITAPH_FRAME_COUNT}",
            items.len()
        )));
    }
    let mut frames = Vec::new();
    for (index, frame) in items.iter().enumerate() {
        frames.push(nine_parts(frame, &format!("frame {index}"))?);
    }
    let templates_at = epitaph.key("templates")?;
    let mut templates = Vec::with_capacity(KEYS.len());
    for (index, (template, key)) in KEYS.iter().enumerate() {
        debug_assert_eq!(*template as usize, index, "KEYS is in E's order");
        templates.push(text_at(&templates_at.key(key)?, "text")?);
    }
    let tellings = strings(dreams, "progress_tellings")?;
    let Ok(progress_tellings) = <[String; 3]>::try_from(tellings) else {
        return Err(dreams.reject("progress_tellings is not three tellings".into()));
    };
    let nouns = legacies.key("legacy_nouns")?;
    let legacy_nouns = LegacyKind::ALL
        .iter()
        .map(|kind| text_at(&nouns, kind.id()))
        .collect::<Result<_, _>>()?;
    let lock_names = door
        .key("locks")?
        .items()?
        .iter()
        .map(|lock| text_at(lock, "name"))
        .collect::<Result<_, _>>()?;
    Ok(EpitaphLore {
        frames,
        priorities: nine_parts(&epitaph.key("priorities")?, "priorities")?,
        templates,
        progress_tellings,
        legacy_nouns,
        lock_names,
    })
}
