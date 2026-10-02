//! Every `ui-text.json` and `lines.json` string this build shows, resolved by key at load.
//!
//! Each `W` names one key. `read_words` looks every one of them up when the game
//! starts, so a renamed or missing key stops the game with the key's path rather
//! than leaving a blank on a screen nobody happened to open.

use crate::json::{At, SchemaError};

/// Which file a key is in.
#[derive(Clone, Copy)]
enum File {
    Ui,
    Lines,
}

/// Declares `W`, one variant per key, and the table of where each one lives.
macro_rules! words {
    ($($variant:ident = $file:ident $path:literal),+ $(,)?) => {
        /// One string the game shows, by its content key.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum W {
            $(
                #[doc = $path]
                $variant
            ),+
        }

        const KEYS: &[(W, File, &str)] = &[$((W::$variant, File::$file, $path)),+];
    };
}

words! {
    TopYear = Ui "top_bar.year",
    TopLastSummer = Ui "top_bar.last_summer",
    TopRenown = Ui "top_bar.house_renown",
    TopDoorCountdown = Ui "top_bar.door_countdown",
    TopDoorOpen = Ui "top_bar.door_open",
    TopDoorTags = Ui "top_bar.door_tags",
    SummerHousehold = Ui "summer.household",
    SummerYard = Ui "summer.yard",
    SummerNoChildren = Ui "summer.no_children",
    SummerHelp = Ui "summer.help",
    SheetDream = Ui "hero_sheet.dream",
    SheetBurden = Ui "hero_sheet.burden",
    SheetHeirloom = Ui "hero_sheet.heirloom",
    SheetBlessed = Ui "hero_sheet.blessed",
    SheetLeaves = Ui "hero_sheet.leaves",
    SheetScar = Ui "hero_sheet.scar",
    SheetChildStation = Ui "hero_sheet.child_station",
    SheetStation = Ui "hero_sheet.station",
    SheetDied = Ui "hero_sheet.died",
    SheetLeft = Ui "hero_sheet.left",
    SheetRenownWounded = Ui "hero_sheet.renown_wounded",
    SheetRenownSettled = Ui "hero_sheet.renown_settled",
    SheetRenown = Ui "hero_sheet.renown",
    SheetAptitudeNote = Ui "hero_sheet.aptitude_note",
    SheetAptitudeNoteAdjusted = Ui "hero_sheet.aptitude_note_adjusted",
    SheetWoundedWarning = Ui "hero_sheet.wounded_warning",
    SheetTooYoung = Ui "hero_sheet.too_young",
    SheetTakenUp = Ui "hero_sheet.taken_up",
    SheetFulfilled = Ui "hero_sheet.fulfilled",
    SheetSettled = Ui "hero_sheet.settled",
    SheetDread = Ui "hero_sheet.dread",
    SheetCourage = Ui "hero_sheet.courage",
    SheetPenalty = Ui "hero_sheet.penalty",
    SheetDestinyCome = Ui "hero_sheet.destiny_come",
    SheetDestiny = Ui "hero_sheet.destiny",
    SheetBondGone = Ui "hero_sheet.bond_gone",
    SheetBonds = Ui "hero_sheet.bonds",
    SheetNoBonds = Ui "hero_sheet.no_bonds",
    SheetMoreBonds = Ui "hero_sheet.more_bonds",
    FamilyHeading = Ui "family.heading",
    FamilySubline = Ui "family.subline",
    FamilyTallyNone = Ui "family.tally_none",
    FamilyTallyOne = Ui "family.tally_one",
    FamilyTallyMany = Ui "family.tally_many",
    FamilyTally = Ui "family.tally",
    FamilyHelp = Ui "family.remembrance_help",
    FamilyLivingStation = Ui "family.living_station",
    FamilyLivingTooYoung = Ui "family.living_too_young",
    FamilyLivingDone = Ui "family.living_done",
    FamilyLivingWants = Ui "family.living_wants",
    FamilyLivingBurden = Ui "family.living_burden",
    FamilyLivingNoFear = Ui "family.living_no_fear",
    FamilyLivingBroken = Ui "family.living_broken",
    FamilyLivingFears = Ui "family.living_fears",
    FamilyLivingSeer = Ui "family.living_seer",
    FamilyOpen = Ui "family.buttons.open",
    FamilyClose = Ui "family.buttons.close",
    FearStateBornBrave = Lines "fear.state_born_brave",
    FearStateConquered = Lines "fear.state_conquered",
    FearStateBroken = Lines "fear.state_broken",
    FearState = Lines "fear.state",
    FearEffect = Lines "fear.effect",
    FearEffectConquered = Lines "fear.effect_conquered",
    FearEffectBroken = Lines "fear.effect_broken",
    QuestCardYouBring = Ui "quest_card.you_bring",
    QuestCardFear = Ui "quest_card.fear",
    QuestCardFearfulItem = Ui "quest_card.fearful_item",
    QuestCardSteadied = Ui "quest_card.steadied",
    AtPlace = Lines "common.at_place",
    ScarBroken = Lines "scar.broken",
    ScarBrokenByGrief = Lines "scar.broken_by_grief",
    DeedBroken = Lines "deed.broken",
    DeedBrokenByGrief = Lines "deed.broken_by_grief",
    DeedConqueredFear = Lines "deed.conquered_fear",
    FearBroken = Lines "fear.broken",
    FearConquered = Lines "fear.conquered",
    GriefRival = Lines "grief.rival",
    GriefDread = Lines "grief.dread",
    GriefBorneOne = Lines "grief.borne_one",
    GriefBorneSharedKinship = Lines "grief.borne_shared_kinship",
    GriefBorneMany = Lines "grief.borne_many",
    QuestCardDreamers = Ui "quest_card.dreamers",
    QuestCardGhost = Ui "quest_card.ghost",
    QuestSheetDreamCall = Ui "quest_sheet.dream_call",
    DreamCounted = Lines "dream.counted",
    DreamStageDone = Lines "dream.stage_done",
    DreamBearingOwned = Lines "dream.bearing_owned",
    DreamBearingOwn = Lines "dream.bearing_own",
    DreamFulfilledOwned = Lines "dream.fulfilled_owned",
    DreamFulfilled = Lines "dream.fulfilled",
    DeedDreamStep = Lines "deed.dream_step",
    DeedDreamFulfilled = Lines "deed.dream_fulfilled",
    BondDreamRivals = Lines "bond.dream_rivals",
    LegacyHandsFull = Lines "legacy.hands_full",
    LegacyHungOverHearth = Lines "legacy.hung_over_hearth",
    LegacyHeirloom = Lines "legacy.heirloom",
    LegacyTale = Lines "legacy.tale",
    LegacyBlessing = Lines "legacy.blessing",
    DeedLeftLegacy = Lines "deed.left_legacy",
    QuestCardDanger = Ui "quest_card.danger",
    QuestCardNeeds = Ui "quest_card.needs",
    QuestCardSucceed = Ui "quest_card.succeed",
    QuestCardTriumph = Ui "quest_card.triumph",
    QuestCardSetback = Ui "quest_card.setback",
    QuestCardDisaster = Ui "quest_card.disaster",
    QuestCardNoOne = Ui "quest_card.no_one",
    QuestCardRoomFor = Ui "quest_card.room_for",
    QuestCardRenown = Ui "quest_card.renown",
    QuestCardUnanswered = Ui "quest_card.unanswered",
    QuestCardWillNotGo = Ui "quest_card.will_not_go",
    QuestSheetNeeds = Ui "quest_sheet.needs",
    QuestSheetNoOne = Ui "quest_sheet.no_one",
    QuestSheetYouBring = Ui "quest_sheet.you_bring",
    QuestSheetDice = Ui "quest_sheet.dice",
    QuestSheetTriumph = Ui "quest_sheet.triumph",
    QuestSheetSuccess = Ui "quest_sheet.success",
    QuestSheetSetback = Ui "quest_sheet.setback",
    QuestSheetDisaster = Ui "quest_sheet.disaster",
    QuestSheetUnanswered = Ui "quest_sheet.unanswered",
    QuestSheetNotQuested = Ui "quest_sheet.not_quested",
    QuestSheetHistory = Ui "quest_sheet.history",
    QuestSheetFallen = Ui "quest_sheet.fallen",
    QuestSheetFewerSeats = Ui "quest_sheet.fewer_seats",
    QuestSheetTroubleStakes = Ui "quest_sheet.trouble_stakes",
    PowerLineMember = Ui "quest_sheet.power_line_member",
    PowerLineCarries = Ui "quest_sheet.power_line_carries",
    PowerLineFears = Ui "quest_sheet.power_line_fears",
    PowerLineConquered = Ui "quest_sheet.power_line_conquered",
    PowerLineWounded = Ui "quest_sheet.power_line_wounded",
    PowerLineDoor = Ui "quest_sheet.power_line_door",
    PowerLineBlessed = Ui "quest_sheet.power_line_blessed",
    PowerLineFloor = Ui "quest_sheet.power_line_floor",
    PowerLineBond = Ui "quest_sheet.power_line_bond",
    PowerLinePatron = Ui "quest_sheet.power_line_patron",
    YearWord = Lines "turning.year_word",
    YearsWord = Lines "turning.years_word",
    SummerSetOut = Ui "summer.button_set_out",
    SummerStayHome = Ui "summer.button_stay_home",
    SummerUnanswered = Lines "summer.unanswered",
    SummerUnansweredTotal = Lines "summer.unanswered_total",
    SummerMendedAtHome = Lines "summer.mended_at_home",
    QuestDisasterRenown = Lines "quest.disaster_renown",
    QuestReward = Lines "quest.reward",
    QuestRewardEach = Lines "quest.reward_each",
    QuestCarrierBonus = Lines "quest.carrier_bonus",
    QuestLesson = Lines "quest.lesson",
    QuestWoundShielded = Lines "quest.wound_shielded",
    QuestSecondWound = Lines "quest.second_wound",
    FateDiedOfWounds = Lines "fate.died_of_wounds",
    DeedWounded = Lines "deed.wounded",
    QuestBurned = Lines "quest.burned",
    FateBurned = Lines "fate.burned",
    QuestDeath = Lines "quest.death",
    FateFell = Lines "fate.fell",
    DeedSurvivedDisaster = Lines "deed.survived_disaster",
    ScarMended = Lines "scar.mended",
    QuestMended = Lines "quest.mended",
    DeedMended = Lines "deed.mended",
    FateCrowned = Lines "fate.crowned",
    QuestCrowned = Lines "quest.crowned",
    DeedCrowned = Lines "deed.crowned",
    CrownHeirLaysAside = Lines "crown.heir_lays_aside",
    CrownHeirloomLeft = Lines "crown.heirloom_left",
    CrownHeirloomTaken = Lines "crown.heirloom_taken",
    DeathCarrierLoss = Lines "death.carrier_loss",
    DeedFirstQuest = Lines "deed.first_quest",
    DeedTriumph = Lines "deed.triumph",
    FearCourage = Lines "fear.courage",
    FearSteadied = Lines "fear.steadied",
    FearDread = Lines "fear.dread",
    BondRivalsToFriends = Lines "bond.rivals_to_friends",
    DeedBefriended = Lines "deed.befriended",
    BondCompanionsToRivals = Lines "bond.companions_to_rivals",
    BondParentSawChild = Lines "bond.parent_saw_child",
    BondChildSawParent = Lines "bond.child_saw_parent",
    GhostLaid = Lines "ghost.laid",
    DeedLaidGhost = Lines "deed.laid_ghost",
    DestinyShieldedBed = Lines "destiny.shielded_bed",
    DestinyShieldedFire = Lines "destiny.shielded_fire",
    TellingHelp = Ui "telling.help",
    TellingQuietTitle = Ui "telling.quiet_title",
    TellingQuietText = Ui "telling.quiet_text",
    TellingMeanwhile = Ui "telling.meanwhile",
    TellingMeanwhileMore = Ui "telling.meanwhile_more",
    TellingHouseClosed = Ui "telling.house_closed",
    TellingQuestMore = Ui "telling.quest_more",
    TellingRoll = Ui "telling.roll",
    TellingMarginExact = Ui "telling.margin_exact",
    TellingMarginBeat = Ui "telling.margin_beat",
    TellingMarginMissed = Ui "telling.margin_missed",
    TellingNext = Ui "telling.buttons.next",
    TellingToWinter = Ui "telling.buttons.to_winter",
    TellingClosed = Ui "telling.buttons.closed",
    TellingSkip = Ui "telling.buttons.skip",
    EndingClosedWhen = Ui "ending.closed_when",
    EndingAgain = Ui "ending.buttons.again",
}

/// The resolved strings, indexable by `W`.
pub struct Words(Vec<String>);

impl std::ops::Index<W> for Words {
    type Output = str;

    fn index(&self, word: W) -> &str {
        &self.0[word as usize]
    }
}

/// Look up every `W` in the two files.
pub fn read_words(ui: At<'_>, lines: At<'_>) -> Result<Words, SchemaError> {
    let lines = lines.key("lines")?;
    let mut out = Vec::with_capacity(KEYS.len());
    for (index, (word, file, path)) in KEYS.iter().enumerate() {
        debug_assert_eq!(
            *word as usize, index,
            "KEYS is in W's order by construction"
        );
        let found = match file {
            // `ui-text.json` nests by dotted path; `lines.json` keys contain the dots.
            File::Ui => path
                .split('.')
                .try_fold(ui.clone(), |at, part| at.key(part))?,
            File::Lines => lines.key(path)?,
        };
        // A leaf is either a plain string or an object carrying `text`.
        out.push(match found.str() {
            Ok(text) => text,
            Err(_) => found.key("text")?.str()?,
        });
    }
    Ok(Words(out))
}
