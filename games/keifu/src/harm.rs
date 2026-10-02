//! What a quest can do to the people on it (SPEC §7.4): wounds, a disaster's death
//! roll, mending, burning, crowning and death — and the heir list the crowned leave
//! their heirloom by (§15.1, §15.3).
//!
//! Every rule writes its lines to the quest page in the order the original does
//! (the `lines.json` source lines order them where SPEC lists effects), and changes
//! the house only through the shared rules: dread through `fear::add_dread`, grief
//! through `grief::grieve`, renown through `House::add_renown`.

use jidousha::prelude::Rng;

use crate::bonds::steadies;
use crate::chance::chance;
use crate::constants::{APTITUDE_LIMIT, CARRIER_LOSS, HEIRS_OFFERED, MENDED_BONUS, MENDED_DREAD};
use crate::destiny::{fire_claims, mends, shields_on_quests};
use crate::fear::{Occasion, add_dread};
use crate::grief::grieve;
use crate::hero::{Deed, DeedKind, Fate, Hero, HeroId, descends_from};
use crate::house::House;
use crate::ids::{BondKind, Destiny, Pool};
use crate::resolve::Afield;
use crate::text::{capitalized, fmt};
use crate::words::W;

/// Record a deed on `hero`, dated now, at the quest's place (SPEC-GAPS KG-34: the
/// W6 deeds no rule reads take weight 0 and no other hero, as KG-12 and KG-15 chose).
pub fn deed(f: &Afield<'_>, hero: &mut Hero, kind: DeedKind, weight: i32, telling: String) {
    hero.deeds.push(Deed {
        kind,
        year: f.year,
        age: hero.age,
        place: Some(f.quest.place),
        weight,
        other: None,
        telling,
    });
}

/// "He was 41." (`lore.age_at_death`).
pub fn age_at_death(f: &Afield<'_>, hero: &Hero) -> String {
    let subject = &f.content.lore.pronouns[hero.pronoun.index()].subject;
    fmt(
        &f.content.lore.age_at_death,
        &[&capitalized(subject), &hero.age.to_string()],
    )
}

/// The quest's place, mid-sentence: "the Barrow".
pub fn place_name<'c>(f: &Afield<'c>) -> &'c str {
    &f.content.lore.places[f.quest.place.index()].name
}

/// Wound (SPEC §7.4, `lineage/tale.jai:234-254`), skipped for the dead: a second wound
/// kills unless the destiny shields on quests; a first one wounds, with a WOUNDS line.
pub fn wound(f: &Afield<'_>, house: &mut House, rng: &mut Rng, id: HeroId, out: &mut Vec<String>) {
    let words = &f.content.words;
    if !house.heroes[id].is_living() {
        return;
    }
    let hero = &house.heroes[id];
    if hero.wounded {
        if shields_on_quests(hero) {
            out.push(fmt(&words[W::QuestWoundShielded], &[&hero.name]));
            return;
        }
        out.push(fmt(
            &words[W::QuestSecondWound],
            &[&hero.full_name(), &age_at_death(f, hero)],
        ));
        let fate = fmt(&words[W::FateDiedOfWounds], &[place_name(f)]);
        die(f, house, id, fate, out);
        return;
    }
    let line = house.writing.pick(f.content, Pool::Wounds, rng);
    let hero = &mut house.heroes[id];
    hero.wounded = true;
    out.push(fmt(line, &[&hero.name]));
    let telling = fmt(&words[W::DeedWounded], &[place_name(f)]);
    deed(f, hero, DeedKind::Wounded, 0, telling);
}

/// Suffer a disaster (SPEC §7.4, `lineage/tale.jai:265-296`), skipped for the dead:
/// the fire's claim, else the mending, else the death roll — ignored by a shielding
/// destiny — then a wound, and a SURVIVED_DISASTER deed for whoever still lives.
pub fn suffer_disaster(
    f: &Afield<'_>,
    house: &mut House,
    rng: &mut Rng,
    id: HeroId,
    out: &mut Vec<String>,
) {
    let words = &f.content.words;
    if !house.heroes[id].is_living() {
        return;
    }
    if fire_claims(&house.heroes[id], &f.quest.tags, f.outcome) {
        burn(f, house, id, out);
        return;
    }
    if mends(&house.heroes[id]) {
        mend(f, house, id, out);
        return;
    }
    let hit = chance(rng, f.quest.death_chance() as f32);
    let shielded = shields_on_quests(&house.heroes[id]);
    if hit && !shielded {
        let pool = house.writing.pick(f.content, Pool::QuestDeaths, rng);
        let hero = &house.heroes[id];
        let death = fmt(pool, &[&hero.full_name(), place_name(f)]);
        out.push(fmt(
            &words[W::QuestDeath],
            &[&death, &age_at_death(f, hero)],
        ));
        let fate = fmt(&words[W::FateFell], &[place_name(f)]);
        die(f, house, id, fate, out);
        return;
    }
    if hit {
        let hero = &house.heroes[id];
        let pronouns = &f.content.lore.pronouns[hero.pronoun.index()];
        out.push(match hero.destiny.kind {
            Destiny::DieInYourBed => fmt(
                &words[W::DestinyShieldedBed],
                &[&hero.name, &pronouns.object, &pronouns.subject],
            ),
            _ => fmt(
                &words[W::DestinyShieldedFire],
                &[&hero.name, &pronouns.subject],
            ),
        });
    }
    wound(f, house, rng, id, out);
    if house.heroes[id].is_living() {
        let telling = fmt(&words[W::DeedSurvivedDisaster], &[place_name(f)]);
        deed(
            f,
            &mut house.heroes[id],
            DeedKind::SurvivedDisaster,
            0,
            telling,
        );
    }
}

/// Mended (SPEC §7.4, `lineage/tale.jai:298-315`): the destiny comes; wounded (even if
/// already), never killed; dread +2 by the one rule (it may break them, and that line
/// comes first: `:301`); every base aptitude +1, capped; the scar, the line, the deed.
fn mend(f: &Afield<'_>, house: &mut House, id: HeroId, out: &mut Vec<String>) {
    let words = &f.content.words;
    let hero = &mut house.heroes[id];
    hero.destiny.fulfilled = true;
    hero.wounded = true;
    out.extend(add_dread(
        f.content,
        &mut house.heroes,
        id,
        MENDED_DREAD,
        Occasion::At(f.quest.place),
        f.year,
    ));
    let hero = &mut house.heroes[id];
    for aptitude in &mut hero.aptitudes {
        *aptitude = (*aptitude + MENDED_BONUS).min(APTITUDE_LIMIT);
    }
    hero.scars
        .push(fmt(&words[W::ScarMended], &[place_name(f)]));
    out.push(fmt(
        &words[W::QuestMended],
        &[&hero.name, place_name(f), &MENDED_BONUS.to_string()],
    ));
    let telling = fmt(&words[W::DeedMended], &[place_name(f)]);
    deed(f, hero, DeedKind::Mended, 0, telling);
}

/// Burned (SPEC §7.4, `lineage/tale.jai:256-263`): the line, the destiny comes, death.
pub fn burn(f: &Afield<'_>, house: &mut House, id: HeroId, out: &mut Vec<String>) {
    let words = &f.content.words;
    let hero = &mut house.heroes[id];
    out.push(fmt(
        &words[W::QuestBurned],
        &[&hero.full_name(), &age_at_death(f, hero)],
    ));
    hero.destiny.fulfilled = true;
    let fate = fmt(&words[W::FateBurned], &[place_name(f)]);
    die(f, house, id, fate, out);
}

/// A questing death (SPEC §7.4, `lineage/tale.jai:361-389`), after the line that tells
/// it: dead, with the year, the age and `fate`; the quest's place and first tag; among
/// the place's fallen; out of every seat; mourned; the house's carrier set down (-4,
/// with its line); and grieved at once, on this page (§12.5).
pub fn die(f: &Afield<'_>, house: &mut House, id: HeroId, fate: String, out: &mut Vec<String>) {
    let hero = &mut house.heroes[id];
    hero.fate = Fate::Dead;
    hero.fate_year = f.year;
    hero.fate_age = hero.age;
    hero.fate_telling = fate;
    hero.death_place = Some(f.quest.place);
    hero.death_tag = f.quest.tags.first().copied();
    let carrier = hero.destiny.kind == Destiny::CarryTheHouse;
    let name = hero.name.clone();
    house.fallen[f.quest.place.index()].push(id);
    house.unseat(id);
    house.mourned.push(id);
    if carrier {
        house.add_renown(-CARRIER_LOSS);
        out.push(fmt(
            &f.content.words[W::DeathCarrierLoss],
            &[&name, &CARRIER_LOSS.to_string()],
        ));
    }
    out.extend(grieve(f.content, &mut house.heroes, id, f.year));
}

/// Crowned (SPEC §7.4, `lineage/tale.jai:317-352`): the destiny comes; departed; a
/// patron for the house; out of every seat; the line and the deed. An heirloom goes
/// to the nearest kin (§15.3) — whose own is laid aside, OQ-5 — or to Court with them.
/// The epitaph the original rolls and composes here is W9's.
pub fn crown(f: &Afield<'_>, house: &mut House, id: HeroId, out: &mut Vec<String>) {
    let words = &f.content.words;
    let hero = &mut house.heroes[id];
    hero.destiny.fulfilled = true;
    hero.fate = Fate::Departed;
    hero.fate_year = f.year;
    hero.fate_age = hero.age;
    hero.fate_telling = words[W::FateCrowned].to_owned();
    let he = capitalized(&f.content.lore.pronouns[hero.pronoun.index()].subject);
    out.push(fmt(&words[W::QuestCrowned], &[&hero.full_name(), &he]));
    deed(
        f,
        hero,
        DeedKind::Crowned,
        0,
        words[W::DeedCrowned].to_owned(),
    );
    house.patrons += 1;
    house.unseat(id);
    let Some(heirloom) = house.heroes[id].heirloom.clone() else {
        return;
    };
    let heir = nearest_kin(&house.heroes, id);
    let hero = &mut house.heroes[id];
    hero.bequest_heirloom = Some(heirloom.name.clone());
    hero.bequest_heir = heir;
    hero.bequest_decided = true;
    let name = hero.name.clone();
    match heir {
        Some(heir) => {
            if let Some(old) = house.heroes[heir].heirloom.take() {
                out.push(fmt(
                    &words[W::CrownHeirLaysAside],
                    &[&house.heroes[heir].name, &old.name],
                ));
            }
            out.push(fmt(
                &words[W::CrownHeirloomLeft],
                &[&name, &heirloom.name, &house.heroes[heir].name],
            ));
            house.heroes[heir].heirloom = Some(heirloom);
            house.heroes[id].heirloom = None;
        }
        None => out.push(fmt(&words[W::CrownHeirloomTaken], &[&name, &heirloom.name])),
    }
}

/// Where `other` ranks as `dead`'s heir (SPEC §15.1's table): children 0, other
/// descendants 1, the spouse 2, siblings by a shared parent and the parents 3, those
/// the dead taught 4, anyone the dead holds a steadying bond with 5, everyone else 6.
fn heir_rank(heroes: &[Hero], dead: HeroId, other: HeroId) -> usize {
    let bond = heroes[dead].bond_to(other);
    let kind = bond.map(|b| b.kind);
    let shares_a_parent = heroes[dead]
        .parents
        .iter()
        .flatten()
        .any(|parent| heroes[other].parents.contains(&Some(*parent)));
    if kind == Some(BondKind::Child) {
        0
    } else if descends_from(heroes, other, dead) {
        1
    } else if kind == Some(BondKind::Spouse) {
        2
    } else if shares_a_parent
        || kind == Some(BondKind::Parent)
        || heroes[dead].parents.contains(&Some(other))
    {
        3
    } else if bond.is_some_and(|b| b.taught) {
        4
    } else if kind.is_some_and(steadies) {
        5
    } else {
        6
    }
}

/// The heir list (SPEC §15.1, `lineage/passage.jai:186-217`): the living other than
/// the dead, by rank, then creation order, at most eight.
pub fn heirs(heroes: &[Hero], dead: HeroId) -> Vec<HeroId> {
    let mut list: Vec<(usize, HeroId)> = (0..heroes.len())
        .filter(|&id| id != dead && heroes[id].is_living())
        .map(|id| (heir_rank(heroes, dead, id), id))
        .collect();
    list.sort();
    list.into_iter()
        .take(HEIRS_OFFERED)
        .map(|(_, id)| id)
        .collect()
}

/// Nearest kin (SPEC §15.3): on the heir list, the first without an heirloom, else the
/// first, else no one.
pub fn nearest_kin(heroes: &[Hero], dead: HeroId) -> Option<HeroId> {
    let list = heirs(heroes, dead);
    list.iter()
        .copied()
        .find(|&id| heroes[id].heirloom.is_none())
        .or_else(|| list.first().copied())
}
