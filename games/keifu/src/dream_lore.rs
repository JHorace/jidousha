//! `dreams.json`, typed: the nine dreams with their requirement trees (SPEC §9.1)
//! and the format pieces of §9.2 and §9.6. What a dream leaves is `legacy_lore`.
//!
//! A requirement is read into a `Requirement` tree of `Predicate`s once, at load, so
//! witnessing and dream calls ask a typed value rather than a string. Every number
//! the file carries that CONSTANTS.md also states — the stage goals, the Court's
//! renown — is checked against it here, and a disagreement stops the game at startup.

use crate::constants::{COURT_DREAM_RENOWN, DREAM_STAGE_COUNT, stage_goal};
use crate::content::{id_at, table, text, text_at};
use crate::ids::{DreamKind, LegacyKind, Outcome, Place, Tag, WinterAction};
use crate::json::{At, SchemaError};

/// A place a predicate names: a fixed one, or the dream's setup place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaceParam {
    /// This place.
    Fixed(Place),
    /// AVENGE_THE_LOST's setup place.
    Setup,
}

/// A tag a predicate names: a fixed one, or the dream's setup tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagParam {
    /// This tag.
    Fixed(Tag),
    /// AVENGE_THE_LOST's setup tag.
    Setup,
}

/// One requirement predicate (SPEC §9.1's table), with its parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Predicate {
    /// Quest moment, present, at the place.
    WentToPlace(PlaceParam),
    /// Quest moment, present, the quest carries the tag.
    FacedTag(TagParam),
    /// Quest moment, present, the outcome at least this.
    FaredAtLeast(Outcome),
    /// Quest moment, present, not the Door, a place not quested at before.
    WalkedANewRoad,
    /// Quest moment, present, not the Door, every questing place now walked.
    WalkedEveryRoad,
    /// Winter moment with this action.
    SpentTheWinter(WinterAction),
    /// Winter moment TEACH or MIND_A_CHILD.
    TaughtTheYoung,
    /// Quest moment, present, a student the hero taught in the party.
    StoodBesideAStudent,
    /// Quest moment, absent, at least a success, a student the hero taught in the party.
    StudentFaredAlone,
    /// Personal renown at least this, any moment.
    RenownIsAtLeast(i32),
    /// A spouse bond, living or dead, any moment.
    IsWed,
    /// A child bond, living or dead, any moment.
    HasAChild,
    /// A living adult child, any moment.
    HasAGrownChild,
}

/// A stage's requirement: one predicate, or all of several.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Requirement {
    /// This predicate.
    One(Predicate),
    /// Every one of these.
    All(Vec<Requirement>),
}

/// One stage of a dream, as authored.
pub struct StageLore {
    /// "Win a triumph at the Barrow", or a `%` template.
    pub task: String,
    /// Whether `task` takes the setup argument.
    pub templated: bool,
    /// How many times the stage must be met.
    pub goal: i32,
    /// What meets it.
    pub requirement: Requirement,
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
    /// How a call is told: apart, triumph, succeed, any ("stay behind", ...).
    pub calls: [String; 4],
}

/// Read every dream, in canonical order, checked against CONSTANTS §11.
pub fn read_dreams(at: &At<'_>) -> Result<Vec<DreamLore>, SchemaError> {
    let items = table(at, "dreams", DreamKind::ALL, DreamKind::id)?;
    DreamKind::ALL
        .iter()
        .zip(&items)
        .map(|(kind, item)| read_dream(*kind, item))
        .collect()
}

/// What each dream leaves, as SPEC §14.1 groups them.
fn legacy_of(kind: DreamKind) -> LegacyKind {
    match kind {
        DreamKind::ForgeABlade | DreamKind::WalkEveryRoad | DreamKind::SeeAChildGrown => {
            LegacyKind::Heirloom
        }
        DreamKind::SeeTheSea | DreamKind::RoofOfTheWorld | DreamKind::KnownAtCourt => {
            LegacyKind::Tale
        }
        DreamKind::QuietTheBarrow | DreamKind::AvengeTheLost | DreamKind::WorthyStudent => {
            LegacyKind::Blessing
        }
    }
}

fn flag(owned: &At<'_>, key: &str) -> Result<bool, SchemaError> {
    Ok(match owned.find(key)? {
        Some(flag) => flag.bool()?,
        None => false,
    })
}

fn read_dream(kind: DreamKind, item: &At<'_>) -> Result<DreamLore, SchemaError> {
    let title_arguments = if flag(item, "title_is_template")? {
        let args = item.key("title_argument")?;
        Some([
            text(&args, "no_lost_hero")?,
            text(&args, "lost_hero_HE")?,
            text(&args, "lost_hero_SHE")?,
        ])
    } else {
        None
    };
    let has_setup = item.find("setup")?.is_some();
    let mut stages = Vec::new();
    for (index, stage) in item.key("stages")?.items()?.iter().enumerate() {
        let goal = stage.key("goal")?.int()?;
        if goal != stage_goal(kind, index) {
            return Err(stage.reject(format!(
                "goal {goal}; CONSTANTS.md §11 says {} for {} stage {}",
                stage_goal(kind, index),
                kind.id(),
                index + 1
            )));
        }
        let requirement = read_requirement(&stage.key("requirement")?, has_setup)?;
        let task = text(stage, "task")?;
        if let Some(template) = stage.find("task_template")? {
            // KNOWN_AT_COURT's "Earn 4 renown" is rendered from COURT_DREAM_RENOWN.
            let rendered = crate::text::fmt(&template.str()?, &[&COURT_DREAM_RENOWN.to_string()]);
            if rendered != task {
                return Err(template.reject(format!(
                    "{rendered:?} is not the task {task:?} (CONSTANTS.md §11 COURT_DREAM_RENOWN)"
                )));
            }
        }
        stages.push(StageLore {
            task,
            templated: flag(stage, "task_is_template")?,
            goal,
            requirement,
        });
    }
    if stages.len() != DREAM_STAGE_COUNT {
        return Err(item.reject(format!("{} stages; a dream has 3", stages.len())));
    }
    let legacy = id_at(item, "legacy", LegacyKind::find)?;
    if legacy != legacy_of(kind) {
        return Err(item.reject(format!(
            "legacy {}; SPEC §14.1 says {}",
            legacy.id(),
            legacy_of(kind).id()
        )));
    }
    Ok(DreamLore {
        title: text(item, "title")?,
        title_arguments,
        legacy,
        stages,
    })
}

fn read_requirement(at: &At<'_>, has_setup: bool) -> Result<Requirement, SchemaError> {
    if let Some(all) = at.find("all")? {
        return Ok(Requirement::All(
            all.items()?
                .iter()
                .map(|part| read_requirement(part, has_setup))
                .collect::<Result<_, _>>()?,
        ));
    }
    let name = text(at, "predicate")?;
    let parameter = at.find("parameter")?;
    let id = || -> Result<String, SchemaError> {
        match &parameter {
            Some(p) => p.str(),
            None => Err(at.reject(format!("{name} needs a parameter"))),
        }
    };
    let setup = |what: &str| -> Result<(), SchemaError> {
        if has_setup {
            Ok(())
        } else {
            Err(at.reject(format!("{what} names a setup the dream does not have")))
        }
    };
    let unknown = |what: &str, id: &str| at.reject(format!("{name}: {id:?} is not a {what}"));
    let predicate = match name.as_str() {
        "went_to_place" => Predicate::WentToPlace(match id()?.as_str() {
            "SETUP_PLACE" => {
                setup("SETUP_PLACE")?;
                PlaceParam::Setup
            }
            other => PlaceParam::Fixed(Place::find(other).ok_or_else(|| unknown("place", other))?),
        }),
        "faced_tag" => Predicate::FacedTag(match id()?.as_str() {
            "SETUP_TAG" => {
                setup("SETUP_TAG")?;
                TagParam::Setup
            }
            other => TagParam::Fixed(Tag::find(other).ok_or_else(|| unknown("tag", other))?),
        }),
        "fared_at_least" => {
            let id = id()?;
            Predicate::FaredAtLeast(Outcome::find(&id).ok_or_else(|| unknown("outcome", &id))?)
        }
        "spent_the_winter" => {
            let id = id()?;
            Predicate::SpentTheWinter(
                WinterAction::find(&id).ok_or_else(|| unknown("winter action", &id))?,
            )
        }
        "renown_is_at_least" => {
            let Some(p) = &parameter else {
                return Err(at.reject("renown_is_at_least needs a parameter".into()));
            };
            let n = p.int()?;
            if n != COURT_DREAM_RENOWN {
                return Err(p.reject(format!(
                    "{n}; CONSTANTS.md §11 COURT_DREAM_RENOWN is {COURT_DREAM_RENOWN}"
                )));
            }
            Predicate::RenownIsAtLeast(n)
        }
        bare => {
            let predicate = match bare {
                "walked_a_new_road" => Predicate::WalkedANewRoad,
                "walked_every_road" => Predicate::WalkedEveryRoad,
                "taught_the_young" => Predicate::TaughtTheYoung,
                "stood_beside_a_student" => Predicate::StoodBesideAStudent,
                "student_fared_alone" => Predicate::StudentFaredAlone,
                "is_wed" => Predicate::IsWed,
                "has_a_child" => Predicate::HasAChild,
                "has_a_grown_child" => Predicate::HasAGrownChild,
                other => {
                    return Err(at.reject(format!("{other:?} is not a predicate of SPEC §9.1")));
                }
            };
            if parameter.is_some() {
                return Err(at.reject(format!("{bare} takes no parameter")));
            }
            predicate
        }
    };
    Ok(Requirement::One(predicate))
}

/// `dreams.json`'s format pieces.
pub fn read_dream_formats(at: &At<'_>) -> Result<DreamFormats, SchemaError> {
    let swap = at.key("title_pronoun_swap")?;
    let object = at.key("task_object_swap")?;
    let calls = at.key("call_tellings")?;
    Ok(DreamFormats {
        progress: text_at(at, "progress_format")?,
        pronoun_find: text(&swap, "find")?,
        pronoun_replace: text(&swap, "replace_with")?,
        object_suffix: text(&object, "suffix")?,
        object_format: text(&object, "format")?,
        calls: [
            text(&calls, "apart")?,
            text(&calls, "TRIUMPH")?,
            text(&calls, "SUCCESS")?,
            text(&calls, "any")?,
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_barrows_last_stage_is_a_triumph_at_the_barrow_and_the_avengers_name_its_setup() {
        let content = crate::content::load().expect("the content loads");
        let barrow = &content.dreams[DreamKind::QuietTheBarrow.index()];
        assert_eq!(
            barrow.stages[2].requirement,
            Requirement::All(vec![
                Requirement::One(Predicate::WentToPlace(PlaceParam::Fixed(Place::Barrow))),
                Requirement::One(Predicate::FaredAtLeast(Outcome::Triumph)),
            ])
        );
        assert_eq!(
            barrow.stages.iter().map(|s| s.goal).collect::<Vec<_>>(),
            [1, 2, 1]
        );
        let avenge = &content.dreams[DreamKind::AvengeTheLost.index()];
        assert_eq!(
            avenge.stages[2].requirement,
            Requirement::All(vec![
                Requirement::One(Predicate::WentToPlace(PlaceParam::Setup)),
                Requirement::One(Predicate::FacedTag(TagParam::Setup)),
                Requirement::One(Predicate::FaredAtLeast(Outcome::Success)),
            ])
        );
        let court = &content.dreams[DreamKind::KnownAtCourt.index()];
        assert_eq!(
            court.stages[0].requirement,
            Requirement::One(Predicate::RenownIsAtLeast(4))
        );
        let student = &content.dreams[DreamKind::WorthyStudent.index()];
        assert_eq!(student.stages[0].goal, 2);
        assert_eq!(
            content.dreams[DreamKind::WalkEveryRoad.index()].stages[0].goal,
            3
        );
        assert_eq!(
            content.dream_formats.calls,
            ["stay behind", "triumph", "succeed", "go"]
        );
    }
}
