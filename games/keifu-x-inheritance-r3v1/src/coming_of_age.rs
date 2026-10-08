//! Coming of age (SPEC §17.5, `lineage/passage.jai:424-437`, `generation/hero-context/
//! hero-context.jai:155-225`) and the dream a child takes into it (§9.5).
//!
//! Every living hero twelve after this turning's ageing is given a calling — their
//! first teacher's, or one of their best aptitude's two — and that teacher's teaching
//! shows; then a dream, by §9.5's priority: the one they hold, vengeance for a parent
//! lost on a quest, an ancestor's ghost's (which is quiet after: the ghost is taken up,
//! §14.4), or a rolled one; and the Seer speaks.

use jidousha::prelude::Rng;

use crate::chance::index;
use crate::constants::{APTITUDE_LIMIT, TEACHING_SHOWS};
use crate::content::Content;
use crate::destiny::speak;
use crate::dream::{Dream, Setup, told_title};
use crate::heirs::deed;
use crate::hero::{DeedKind, DreamFate, HeroId, descends_from};
use crate::house::House;
use crate::ids::{BondKind, Phase, Place};
use crate::newcomers::{phase_effect, rolled_dream, vocations_of};
use crate::passage::{PageKind, TurnPage};
use crate::rivals::dream_rivals;
use crate::sheet::prophecy;
use crate::text::{capitalized, fmt, lowered};
use crate::words::W;

/// The comings of age of this turning, in creation order (SPEC §18 step 6).
pub fn comings_of_age(content: &Content, house: &mut House, rng: &mut Rng) -> Vec<TurnPage> {
    let coming: Vec<HeroId> = (0..house.heroes.len())
        .filter(|&id| {
            let hero = &house.heroes[id];
            hero.is_living() && hero.age == crate::constants::COMING_OF_AGE
        })
        .collect();
    coming
        .into_iter()
        .map(|id| come_of_age(content, house, id, rng))
        .collect()
}

/// One coming of age, and its page.
fn come_of_age(content: &Content, house: &mut House, id: HeroId, rng: &mut Rng) -> TurnPage {
    let words = &content.words;
    let year = house.calendar.current_year();
    let mut lines = Vec::new();
    // 1. The calling: rolled from the best aptitude's two, then the first teacher's.
    let best = house.heroes[id].best_aptitude();
    let callings = vocations_of(content, best);
    let rolled = callings[index(rng, callings.len())];
    let teacher = house.heroes[id]
        .bonds
        .iter()
        .find(|b| b.kind == BondKind::Mentor)
        .map(|b| b.other);
    let vocation = teacher.map_or(rolled, |t| house.heroes[t].vocation);
    house.heroes[id].vocation = vocation;
    let hero = &house.heroes[id];
    let forms = &content.lore.pronouns[hero.pronoun.index()];
    lines.push(fmt(
        &words[W::AgeCalling],
        &[
            &hero.name,
            &hero.age.to_string(),
            &lowered(&content.lore.vocations[vocation.index()]),
            &capitalized(&forms.subject),
            &phase_effect(content, Phase::Youth, hero.pronoun),
        ],
    ));
    // 2. The teacher's teaching shows, in the teacher's best aptitude.
    if let Some(t) = teacher {
        let aptitude = house.heroes[t].best_aptitude();
        if house.heroes[id].base(aptitude) < APTITUDE_LIMIT {
            house.heroes[id].aptitudes[aptitude.index()] += TEACHING_SHOWS;
            let hero = &house.heroes[id];
            lines.push(fmt(
                &words[W::AgeTeachingShows],
                &[
                    &house.heroes[t].name,
                    &content.lore.pronouns[hero.pronoun.index()].object,
                    &content.lore.aptitudes[aptitude.index()],
                    &hero.base(aptitude).to_string(),
                ],
            ));
        }
    }
    // 3. The dream (§9.5), then dream rivals — SPEC-GAPS KG-53: whichever priority gave
    // it, a dream held already included.
    lines.push(take_a_dream(content, house, id, rng));
    if let Some(dream) = house.heroes[id].dream.clone() {
        lines.extend(dream_rivals(content, &mut house.heroes, id, &dream, year));
    }
    // 4. The Seer speaks; 5. the page.
    speak(content, &mut house.heroes, id, rng);
    let hero = &house.heroes[id];
    let lore = &content.destinies[hero.destiny.kind.index()];
    lines.push(fmt(
        &words[W::AgeSeer],
        &[
            &content.lore.pronouns[hero.pronoun.index()].possessive,
            &prophecy(content, hero),
        ],
    ));
    lines.extend(seer_lines(lore));
    let title = fmt(&words[W::AgeTitle], &[&hero.full_name()]);
    let telling = words[W::DeedCameOfAge].to_owned();
    deed(
        &mut house.heroes[id],
        DeedKind::CameOfAge,
        year,
        None,
        telling,
    );
    TurnPage {
        kind: PageKind::ComingOfAge,
        title,
        lines,
        bequest: None,
        about: Some(id),
    }
}

/// The doom and the gift after the Seer's words, each when it says anything.
pub fn seer_lines(lore: &crate::content::DestinyLore) -> Vec<String> {
    [&lore.doom, &lore.gift]
        .into_iter()
        .filter(|line| !line.is_empty())
        .cloned()
        .collect()
}

/// The dream a hero takes into their coming of age (SPEC §9.5), and its line: the
/// dream they hold; vengeance for a parent (parent 0 first) dead on a quest not at the
/// Door; the first ghost on the list that is an ancestor's, taken up — its dead's
/// dream fate becomes PASSED_ON to them and their epitaph is recomposed; or a rolled one.
fn take_a_dream(content: &Content, house: &mut House, id: HeroId, rng: &mut Rng) -> String {
    let words = &content.words;
    let hero = &house.heroes[id];
    let forms = &content.lore.pronouns[hero.pronoun.index()];
    let he = capitalized(&forms.subject);
    if let Some(dream) = &hero.dream {
        let told = told_title(
            content,
            dream,
            dream
                .owner
                .map_or(hero.pronoun, |o| house.heroes[o].pronoun),
        );
        return match dream.owner {
            Some(owner) => fmt(
                &words[W::AgeCarriedDream],
                &[&he, &house.heroes[owner].name, &forms.subject, &told],
            ),
            None => fmt(&words[W::AgeHasDream], &[&he, &told]),
        };
    }
    let lost = hero.parents.iter().flatten().find_map(|&p| {
        let parent = &house.heroes[p];
        let questing = parent
            .death_place
            .filter(|place| *place != Place::SealedDoor);
        match (parent.fate, questing, parent.death_tag) {
            (crate::hero::Fate::Dead, Some(place), Some(tag)) => Some((p, place, tag)),
            _ => None,
        }
    });
    if let Some((parent, place, tag)) = lost {
        let lost = &house.heroes[parent];
        let setup = Setup {
            place,
            tag,
            lost: Some(parent),
        };
        let dream = match Dream::build(
            content,
            crate::ids::DreamKind::AvengeTheLost,
            Some(setup),
            Some(lost.pronoun),
        ) {
            Ok(dream) => dream,
            Err(error) => panic!("[keifu_x_inheritance_r3v1] a vengeance did not build: {error}"),
        };
        // SPEC-GAPS KG-52: the line's arguments as listed, though every questing fate
        // telling already names the place ("fell at Emberfall at Emberfall").
        let line = fmt(
            &words[W::AgeAvengeDream],
            &[
                &he,
                &told_title(content, &dream, house.heroes[id].pronoun),
                &lost.full_name(),
                &lost.fate_telling,
                &content.lore.places[place.index()].name,
                &content.lore.tags[tag.index()].noun,
            ],
        );
        house.heroes[id].dream = Some(dream);
        return line;
    }
    if let Some(at) = house
        .ghosts
        .iter()
        .position(|ghost| descends_from(&house.heroes, id, ghost.hero))
    {
        // [emergent] a swap remove, as laying a ghost is (OQ-29).
        let ghost = house.ghosts.swap_remove(at);
        let dead = &mut house.heroes[ghost.hero];
        dead.dream_fate = DreamFate::PassedOn;
        dead.bequest_heir = Some(id);
        crate::epitaph::recompose(content, &mut house.heroes, ghost.hero);
        let owner = ghost.dream.owner.unwrap_or(ghost.hero);
        let line = fmt(
            &words[W::AgeGhostDream],
            &[
                &forms.object,
                &he,
                &house.heroes[ghost.hero].full_name(),
                &told_title(content, &ghost.dream, house.heroes[owner].pronoun),
            ],
        );
        house.heroes[id].dream = Some(ghost.dream);
        return line;
    }
    let age = hero.age;
    let dream = rolled_dream(content, &house.heroes, age, rng);
    let line = fmt(
        &words[W::AgeRolledDream],
        &[
            &he,
            &told_title(content, &dream, house.heroes[id].pronoun),
            &content.legacies.promises[content.dreams[dream.kind.index()].legacy.index()],
        ],
    );
    house.heroes[id].dream = Some(dream);
    line
}
