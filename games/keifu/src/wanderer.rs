//! Wanderers (SPEC §17.2, `lineage/passage.jai:439-472`, `generation/hero-context/
//! hero-context.jai:74-98`): whether one comes to the door this turning, who they are,
//! and the page that tells their arrival.
//!
//! At most one a year, after the births and the comings of age: none in a house of ten
//! or more; certainly one when fewer than five adults live; otherwise by a chance that
//! grows with the house's renown. Who comes is drawn in the order the spec lists, from
//! the standing the house's renown draws.

use jidousha::prelude::Rng;

use crate::chance::{between, chance, index};
use crate::coming_of_age::seer_lines;
use crate::constants::{
    FEWEST_ADULTS, WANDERER_AGES, WANDERER_BASE_CHANCE, WANDERER_CHANCE_LIMIT,
    WANDERER_RENOWN_DRAW, WANDERER_RENOWN_HIGH, WANDERER_ROOM,
};
use crate::content::Content;
use crate::destiny::speak;
use crate::dream::told_title;
use crate::heirs::deed;
use crate::hero::DeedKind;
use crate::house::House;
use crate::ids::{Aptitude, Pool, Tag};
use crate::newcomers::{fear_of, newcomer, roll_pronoun, rolled_dream, vocations_of};
use crate::passage::{PageKind, TurnPage};
use crate::rivals::dream_rivals;
use crate::sheet::prophecy;
use crate::text::{capitalized, fmt, lowered};
use crate::turning_lore::standing;
use crate::words::W;

/// Whether a wanderer may come this turning (SPEC §17.2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Odds {
    /// Ten or more live: none comes, and nothing is rolled.
    NoRoom,
    /// Fewer than five adults live: one comes, and nothing is rolled.
    Certain,
    /// One comes on `min(0.2 + 0.01 * renown, 0.6)`.
    Chance(f64),
}

/// The odds of a wanderer this turning (SPEC §17.2).
pub fn wanderer_odds(house: &House) -> Odds {
    let living = house.heroes.iter().filter(|h| h.is_living()).count();
    let adults = house
        .heroes
        .iter()
        .filter(|h| h.is_living() && h.is_adult())
        .count();
    if living >= WANDERER_ROOM {
        Odds::NoRoom
    } else if adults < FEWEST_ADULTS {
        Odds::Certain
    } else {
        Odds::Chance(
            (WANDERER_BASE_CHANCE + WANDERER_RENOWN_DRAW * f64::from(house.renown))
                .min(WANDERER_CHANCE_LIMIT),
        )
    }
}

/// Whether a wanderer comes, and if one does, their arrival page (SPEC §18 step 7).
pub fn wanderer(content: &Content, house: &mut House, rng: &mut Rng) -> Option<TurnPage> {
    let comes = match wanderer_odds(house) {
        Odds::NoRoom => false,
        Odds::Certain => true,
        Odds::Chance(p) => chance(rng, p as f32),
    };
    comes.then(|| arrive(content, house, rng))
}

/// A wanderer drawn and taken in (SPEC §17.2), in roll order: pronoun; name; house;
/// age; the standing by house renown; the gift aptitude; the three aptitudes in the
/// standing's other range, then the gift's overwritten in its own; a calling of the
/// gift's two; a fear; a dream (§17.4); personal renown; the Seer.
fn arrive(content: &Content, house: &mut House, rng: &mut Rng) -> TurnPage {
    let words = &content.words;
    let next = house.calendar.current_year() + 1;
    let pronoun = roll_pronoun(rng);
    let name = house.bags.name(pronoun, rng);
    let family = house.bags.houses.draw(rng);
    let age = between(rng, WANDERER_AGES.0, WANDERER_AGES.1);
    let standing = standing(&content.wanderers, house.renown);
    let gift = Aptitude::ALL[index(rng, Aptitude::ALL.len())];
    let mut aptitudes = [0; 3];
    for aptitude in aptitudes.iter_mut() {
        *aptitude = between(rng, standing.other.0, standing.other.1);
    }
    aptitudes[gift.index()] = between(rng, standing.gift.0, standing.gift.1);
    let callings = vocations_of(content, gift);
    let vocation = callings[index(rng, callings.len())];
    let fear = fear_of(Tag::ALL[index(rng, Tag::ALL.len())]);
    let dream = rolled_dream(content, &house.heroes, age, rng);
    let renown = between(rng, 0, WANDERER_RENOWN_HIGH);
    let mut hero = newcomer(name, family, pronoun, age, next - age, fear);
    hero.aptitudes = aptitudes;
    hero.vocation = vocation;
    hero.dream = Some(dream.clone());
    hero.renown = renown;
    let id = house.heroes.len();
    house.heroes.push(hero);
    speak(content, &mut house.heroes, id, rng);
    deed(
        &mut house.heroes[id],
        DeedKind::Arrived,
        next,
        None,
        words[W::DeedArrived].to_owned(),
    );
    let pool = house.writing.pick(content, Pool::Arrivals, rng);
    let hero = &house.heroes[id];
    let forms = &content.lore.pronouns[hero.pronoun.index()];
    let he = capitalized(&forms.subject);
    let mut lines = vec![
        fmt(pool, &[&hero.full_name()]),
        fmt(
            &words[W::ArrivalWho],
            &[
                &lowered(&content.lore.vocations[hero.vocation.index()]),
                &hero.age.to_string(),
                &lowered(&content.lore.aptitudes[hero.best_aptitude().index()]),
                &forms.possessive,
                &standing.telling,
            ],
        ),
        fmt(
            &words[W::ArrivalDream],
            &[
                &he,
                &told_title(content, &dream, hero.pronoun),
                &content.legacies.promises[content.dreams[dream.kind.index()].legacy.index()],
            ],
        ),
    ];
    lines.extend(dream_rivals(
        content,
        &mut house.heroes,
        id,
        &dream,
        next - 1,
    ));
    let hero = &house.heroes[id];
    let forms = &content.lore.pronouns[hero.pronoun.index()];
    lines.push(fmt(
        &words[W::ArrivalFear],
        &[&he, &content.lore.tags[hero.fear.tag.index()].noun],
    ));
    lines.push(fmt(
        &words[W::ArrivalSeer],
        &[&forms.object, &prophecy(content, hero)],
    ));
    lines.extend(seer_lines(&content.destinies[hero.destiny.kind.index()]));
    TurnPage {
        kind: PageKind::Arrival,
        title: fmt(&words[W::ArrivalTitle], &[&hero.full_name()]),
        lines,
        bequest: None,
        about: Some(id),
    }
}
