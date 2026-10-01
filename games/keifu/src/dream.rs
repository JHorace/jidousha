//! The dream as stored state (SPEC §9.1), and its told title, task and progress (§9.2).
//!
//! W1 holds dreams as data the household seeds and the sheet shows. Nothing here
//! advances a dream on a moment — witnessing and fulfilment are W3.

use crate::constants::DREAM_STAGE_COUNT;
use crate::content::Content;
use crate::hero::HeroId;
use crate::ids::{DreamKind, Place, Pronoun, Tag};
use crate::text::{fmt, lowered};

/// AVENGE_THE_LOST's setup: the place, the tag, and the lost hero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Setup {
    /// Where the lost hero was lost.
    pub place: Place,
    /// What took them.
    pub tag: Tag,
    /// Who was lost.
    pub lost: Option<HeroId>,
}

/// One stage, with its task already rendered.
#[derive(Clone, Debug, PartialEq)]
pub struct Stage {
    /// The task as authored, with the setup filled in.
    pub task: String,
    /// How many times it must be met.
    pub goal: i32,
    /// How many times it has been.
    pub count: i32,
}

/// A dream (SPEC §9.1). Dreamt iff the title is non-empty; `Option<Dream>` on the
/// hero carries "not dreamt" as `None`.
#[derive(Clone, Debug, PartialEq)]
pub struct Dream {
    /// Which of the nine.
    pub kind: DreamKind,
    /// AVENGE_THE_LOST's setup.
    pub setup: Option<Setup>,
    /// Who first dreamt it, when it is carried by someone else.
    pub owner: Option<HeroId>,
    /// The title, rendered.
    pub title: String,
    /// Three stages.
    pub stages: Vec<Stage>,
    /// The current stage index; 3 is fulfilled.
    pub current: usize,
}

/// Where a stage stands, for the sheet's done / current / upcoming marks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageMark {
    /// Before the current stage.
    Done,
    /// The current stage.
    Current,
    /// After it.
    Upcoming,
}

impl Dream {
    /// Build a dream from its authored form, at stage 0. `lost_pronoun` is the lost
    /// hero's, for AVENGE_THE_LOST's title.
    ///
    /// Fails if the dream is templated and no setup was given.
    pub fn build(
        content: &Content,
        kind: DreamKind,
        setup: Option<Setup>,
        lost_pronoun: Option<Pronoun>,
    ) -> Result<Self, String> {
        let lore = &content.dreams[kind.index()];
        let title = match &lore.title_arguments {
            None => lore.title.clone(),
            Some([nobody, he, she]) => {
                let argument = match lost_pronoun {
                    None => nobody,
                    Some(Pronoun::He) => he,
                    Some(Pronoun::She) => she,
                };
                fmt(&lore.title, &[argument])
            }
        };
        let mut stages = Vec::with_capacity(DREAM_STAGE_COUNT);
        for (index, stage) in lore.stages.iter().enumerate() {
            let task = if stage.templated {
                let Some(setup) = setup else {
                    return Err(format!(
                        "{}'s stage {} names its setup, and no setup was given",
                        kind.id(),
                        index + 1
                    ));
                };
                // content/README.md: the second task names the setup place, the
                // third the setup tag (place `name`, tag `title`).
                let argument = match index {
                    1 => &content.lore.places[setup.place.index()].name,
                    2 => &content.lore.tags[setup.tag.index()].title,
                    _ => return Err(format!("{}'s stage {} is templated", kind.id(), index + 1)),
                };
                fmt(&stage.task, &[argument])
            } else {
                stage.task.clone()
            };
            stages.push(Stage {
                task,
                goal: stage.goal,
                count: 0,
            });
        }
        Ok(Self {
            kind,
            setup,
            owner: None,
            title,
            stages,
            current: 0,
        })
    }

    /// `advance_dream_to_stage(d, s)`: current = s, every earlier stage counted complete.
    pub fn advance_to_stage(&mut self, stage: usize) {
        self.current = stage;
        for earlier in self.stages.iter_mut().take(stage) {
            earlier.count = earlier.goal;
        }
    }

    /// Fulfilled once the current stage is past the last.
    pub fn is_fulfilled(&self) -> bool {
        self.current >= DREAM_STAGE_COUNT
    }

    /// How `stage` stands against the current one.
    pub fn mark(&self, stage: usize) -> StageMark {
        match stage.cmp(&self.current) {
            std::cmp::Ordering::Less => StageMark::Done,
            std::cmp::Ordering::Equal => StageMark::Current,
            std::cmp::Ordering::Greater => StageMark::Upcoming,
        }
    }
}

/// The told title: first letter lowered, " my " → " his "/" her " of the owner, or the
/// bearer if there is no owner (SPEC §9.2).
pub fn told_title(content: &Content, dream: &Dream, owner_or_bearer: Pronoun) -> String {
    let formats = &content.dream_formats;
    let possessive = &content.lore.pronouns[owner_or_bearer.index()].possessive;
    lowered(&dream.title).replace(
        &formats.pronoun_find,
        &fmt(&formats.pronoun_replace, &[possessive]),
    )
}

/// The told task: first letter lowered, a trailing " you" → " him"/" her" of the hero
/// it is told about (SPEC §9.2).
pub fn told_task(content: &Content, task: &str, about: Pronoun) -> String {
    let formats = &content.dream_formats;
    let task = lowered(task);
    match task.strip_suffix(formats.object_suffix.as_str()) {
        Some(stem) => fmt(
            &formats.object_format,
            &[stem, &content.lore.pronouns[about.index()].object],
        ),
        None => task,
    }
}

/// Progress: the told current task, plus " (count/goal)" when goal > 1, count capped.
pub fn progress(content: &Content, dream: &Dream, about: Pronoun) -> String {
    let Some(stage) = dream.stages.get(dream.current) else {
        return String::new();
    };
    let task = told_task(content, &stage.task, about);
    if stage.goal > 1 {
        let count = stage.count.min(stage.goal).to_string();
        fmt(
            &content.dream_formats.progress,
            &[&task, &count, &stage.goal.to_string()],
        )
    } else {
        task
    }
}
