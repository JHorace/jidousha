//! Ghosts (SPEC §14.4), the quest part: a ghost on the house's list, and the quest
//! a board posts to lay it.
//!
//! W5 builds what a board reads — the list and the ghost quest. W6 reads `ghost.json`'s
//! endings for the ghost quest's story and lays a ghost on a won quest (`lay_ghost`);
//! W8 raises one on a death page (`heirs::choose`) and gives one to a descendant coming
//! of age (`coming_of_age`).

use crate::constants::{GHOST_APTITUDE, GHOST_DANGER, GHOST_SEATS};
use crate::content::{Content, id_at, text};
use crate::dream::{Dream, told_task};
use crate::hero::{Hero, HeroId};
use crate::ids::{Aptitude, Place};
use crate::json::{At, SchemaError};
use crate::quest::{Making, Quest, Source, stakes};
use crate::text::{capitalized, fmt};

/// `ghost.json`, the parts a board posts.
pub struct GhostLore {
    /// "Lay %'s ghost".
    pub title: String,
    /// "{name} died with a thing undone: {task}. {He} has not gone far."
    pub premise: String,
    /// The four endings, by `Outcome`: brace placeholders, then `%1` = the party's names.
    pub endings: [String; 4],
}

/// A ghost (SPEC §3.1 `ghosts`): the dead hero, a copy of the undone dream, and
/// the place it walks.
#[derive(Clone, Debug, PartialEq)]
pub struct Ghost {
    /// Whose ghost.
    pub hero: HeroId,
    /// The dream left undone.
    pub dream: Dream,
    /// Where its quest is posted.
    pub place: Place,
}

/// Read `ghost.json`'s title and premise, and hold its numbers to CONSTANTS §5.
pub fn read_ghost(at: &At<'_>) -> Result<GhostLore, SchemaError> {
    let aptitude = id_at(at, "aptitude", Aptitude::find)?;
    let (seats, danger) = (at.key("seats")?.int()?, at.key("danger")?.int()?);
    if (aptitude, seats, danger) != (GHOST_APTITUDE, GHOST_SEATS, GHOST_DANGER) {
        return Err(at.reject(format!(
            "aptitude {}, seats {seats}, danger {danger} disagree with CONSTANTS.md §5",
            aptitude.id()
        )));
    }
    Ok(GhostLore {
        title: text(at, "title")?,
        premise: text(at, "premise")?,
        endings: crate::content::read_endings(at)?,
    })
}

/// The ghost quest (SPEC §14.4, `lineage/ghost.jai:55-76`): "Lay <name>'s ghost",
/// the premise with the dead hero's words filled in, Spirit, the place's tags, calm
/// seats 2 and calm danger 2 under the place's trouble, no wobble. Draws nothing.
pub fn ghost_quest(
    content: &Content,
    heroes: &[Hero],
    ghost: &Ghost,
    trouble: i32,
    year: i32,
) -> Quest {
    let dead = &heroes[ghost.hero];
    let making = Making {
        source: Source::Ghost(ghost.hero),
        title: fmt(&content.ghost.title, &[&dead.name]),
        premise: ghost_text(content, &content.ghost.premise, dead, &ghost.dream),
        place: ghost.place,
        aptitude: GHOST_APTITUDE,
        tags: content.lore.places[ghost.place.index()].tags.clone(),
    };
    stakes(making, GHOST_SEATS, GHOST_DANGER, trouble, year)
}

/// A ghost text's brace placeholders (`content/README.md`): `{name}` the dead
/// hero's first name, `{task}` the told current stage task of the ghost's dream,
/// `{He}`, `{he}`, `{him}` the dead hero's pronouns.
pub fn ghost_text(content: &Content, text: &str, dead: &Hero, dream: &Dream) -> String {
    let Some(stage) = dream.stages.get(dream.current) else {
        panic!(
            "[keifu_x_inheritance_r2] {}'s ghost carries a fulfilled dream\n  likely cause: a ghost was raised \
             from a dream that was done\n  fix: SPEC §15.2 raises a ghost only for an undone dream",
            dead.name
        );
    };
    let pronoun = &content.lore.pronouns[dead.pronoun.index()];
    // SPEC-GAPS KG-31: the task is told about the dead hero, whose ghost it is.
    let task = told_task(content, &stage.task, dead.pronoun);
    text.replace("{name}", &dead.name)
        .replace("{task}", &task)
        .replace("{He}", &capitalized(&pronoun.subject))
        .replace("{he}", &pronoun.subject)
        .replace("{him}", &pronoun.object)
}

/// Lay the ghost whose quest was won (SPEC §14.4, `lineage/ghost.jai:83-111`): the
/// tale "The laying of <name>'s ghost"; the dead hero's dream fate LAID_TO_REST with
/// the year; `lines.ghost.laid`; a LAID_GHOST deed for each living member; and the
/// ghost swap-removed from the list. The dream is not fulfilled and leaves nothing
/// else. The dead hero's epitaph is recomposed with the wording they have.
pub fn lay_ghost(
    f: &crate::resolve::Afield<'_>,
    house: &mut crate::house::House,
    dead: HeroId,
    members: &[HeroId],
    out: &mut Vec<String>,
) {
    use crate::constants::TALE_YEARLY_RENOWN;
    use crate::words::W;
    let content = f.content;
    let Some(at) = house.ghosts.iter().position(|ghost| ghost.hero == dead) else {
        panic!(
            "[keifu_x_inheritance_r2] a ghost's quest was won and {} has no ghost on the list\n  likely cause: \
             the ghost was removed while its quest was posted\n  fix: SPEC §14.4 removes a \
             ghost only when it is laid or taken up",
            house.heroes[dead].name
        );
    };
    let title = fmt(
        &content.legacies.ghost_tale_title,
        &[&house.heroes[dead].name],
    );
    house.tales.push(crate::house::Tale {
        title: title.clone(),
        about: dead,
        since: f.year,
    });
    let hero = &mut house.heroes[dead];
    hero.dream_fate = crate::hero::DreamFate::LaidToRest;
    hero.laid_year = Some(f.year);
    crate::epitaph::recompose(content, &mut house.heroes, dead);
    let hero = &house.heroes[dead];
    let subject = &content.lore.pronouns[hero.pronoun.index()].subject;
    out.push(fmt(
        &content.words[W::GhostLaid],
        &[
            &hero.full_name(),
            subject,
            &title,
            &TALE_YEARLY_RENOWN.to_string(),
        ],
    ));
    let telling = fmt(&content.words[W::DeedLaidGhost], &[&hero.full_name()]);
    for &member in members {
        if house.heroes[member].is_living() {
            crate::harm::deed(
                f,
                &mut house.heroes[member],
                crate::hero::DeedKind::LaidGhost,
                0,
                telling.clone(),
            );
        }
    }
    // [emergent] a swap remove: the last ghost takes the laid one's place (OQ-29).
    house.ghosts.swap_remove(at);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{founded, id};

    fn garricks_ghost(heroes: &[Hero]) -> Ghost {
        let garrick = id(heroes, "Garrick");
        let dream = heroes[garrick].dream.clone().expect("Garrick dreams");
        Ghost {
            hero: garrick,
            dream,
            place: Place::Barrow,
        }
    }

    #[test]
    fn a_ghost_quest_is_spirit_at_its_place_with_two_seats_danger_two_and_no_wobble() {
        let (content, heroes) = founded();
        let ghost = garricks_ghost(&heroes);
        let quest = ghost_quest(&content, &heroes, &ghost, 0, 1);
        assert_eq!(quest.title, "Lay Garrick's ghost");
        assert_eq!(
            quest.premise,
            "Garrick died with a thing undone: win a triumph at the Barrow. He has not gone far."
        );
        assert_eq!(quest.source, Source::Ghost(ghost.hero));
        assert_eq!(quest.template(), None);
        assert_eq!(quest.aptitude, Aptitude::Spirit);
        assert_eq!(quest.place, Place::Barrow);
        assert_eq!(quest.tags, content.lore.places[0].tags);
        // seats 2, danger 2, renown 2, demand 2 * (3 + 2 + 0 + 0) = 10 exactly.
        assert_eq!(
            (
                quest.calm_seats,
                quest.seats,
                quest.danger,
                quest.renown,
                quest.demand
            ),
            (2, 2, 2, 2, 10)
        );
    }

    #[test]
    fn trouble_and_the_year_raise_a_ghost_quest_by_the_stakes_formula() {
        let (content, heroes) = founded();
        let ghost = garricks_ghost(&heroes);
        // Trouble 2 in year 13: seats max(2 - 2, 1) = 1, danger 4, renown 4,
        // demand 1 * (3 + 2 + 2 + 2) = 9.
        let quest = ghost_quest(&content, &heroes, &ghost, 2, 13);
        assert_eq!(
            (
                quest.seats,
                quest.danger,
                quest.renown,
                quest.demand,
                quest.trouble
            ),
            (1, 4, 4, 9, 2)
        );
    }
}
