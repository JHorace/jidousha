//! An epitaph's last four parts (SPEC §20 "Parts", `lineage/epitaph.jai:328-484`):
//! PROPHECY, LOVE, END and LEFT, each part's selection rule branch by branch; ORIGIN to
//! DREAM, and the helpers, are `epitaph_parts.rs`.

use crate::content::Content;
use crate::epitaph::Wording;
use crate::epitaph_lore::E;
use crate::epitaph_parts::{first_deed, forms};
use crate::hero::{Deed, DeedKind, Fate, Hero, HeroId, kin};
use crate::ids::{BondKind, Destiny, LegacyKind, Part, Pronoun};
use crate::text::{capitalized, fmt, name_list, year_telling};

/// PROPHECY (`:328-382`): nothing for the unspoken; each destiny's sentence, the `*_came`
/// one once fulfilled; the Door by blood and by standing there; the greater student, read
/// as the students stand when the epitaph is composed (SPEC-GAPS KG-62).
pub fn prophecy(content: &Content, heroes: &[Hero], id: HeroId) -> String {
    let lore = &content.epitaph;
    let hero = &heroes[id];
    let f = forms(content, hero.pronoun);
    let came = hero.destiny.fulfilled;
    let (s, o, p) = (&f.subject, &f.object, &f.possessive);
    match hero.destiny.kind {
        Destiny::Unspoken => String::new(),
        Destiny::FireWillEndYou if came => fmt(&lore[E::ProphecyFireCame], &[o]),
        Destiny::FireWillEndYou => fmt(&lore[E::ProphecyFire], &[o, o]),
        Destiny::OutliveThoseYouLove => fmt(&lore[E::ProphecyOutlive], &[s, s, o]),
        Destiny::WearACrown if came => fmt(&lore[E::ProphecyCrownCame], &[s, s]),
        Destiny::WearACrown => fmt(&lore[E::ProphecyCrown], &[s]),
        Destiny::OpenTheSealedDoor => {
            let stood = first_deed(hero, DeedKind::StoodAtTheDoor).is_some();
            match (&hero.destiny.blood_of, stood) {
                (Some(blood), true) => fmt(&lore[E::ProphecyDoorBloodStood], &[blood, s, blood]),
                (Some(blood), false) => fmt(&lore[E::ProphecyDoorBlood], &[blood, o, blood]),
                (None, true) => fmt(&lore[E::ProphecyDoorStood], &[o, s]),
                (None, false) => fmt(&lore[E::ProphecyDoor], &[o, s]),
            }
        }
        Destiny::ChildWillSurpassYou if came => fmt(&lore[E::ProphecySurpassCame], &[p, o, p, s]),
        Destiny::ChildWillSurpassYou => fmt(&lore[E::ProphecySurpass], &[p, o]),
        Destiny::BreakAndBeMended if came => fmt(&lore[E::ProphecyMendedCame], &[s, s]),
        Destiny::BreakAndBeMended => fmt(&lore[E::ProphecyMended], &[s, &f.he]),
        Destiny::DieInYourBed => fmt(&lore[E::ProphecyBed], &[o, p, o]),
        Destiny::CarryTheHouse => fmt(&lore[E::ProphecyCarry], &[s, s]),
        Destiny::TeachAGreater => {
            let reflexive = &lore[match hero.pronoun {
                Pronoun::He => E::ProphecyReflexiveHe,
                Pronoun::She => E::ProphecyReflexiveShe,
            }];
            let best = |h: &Hero| h.aptitudes.iter().copied().max().unwrap_or(0);
            let greater = hero
                .bonds
                .iter()
                .filter(|bond| bond.taught)
                .map(|bond| &heroes[bond.other])
                .find(|student| best(student) > best(hero));
            match greater {
                Some(student) => fmt(
                    &lore[E::ProphecyGreaterCame],
                    &[s, reflexive, s, &student.name, o, o],
                ),
                None => fmt(&lore[E::ProphecyGreater], &[s, reflexive, s]),
            }
        }
    }
}

/// LOVE (`:384-417`): every spouse (full names) and every child (first names); else the
/// last FRIEND bond, kin or not; else nothing.
pub fn love(content: &Content, heroes: &[Hero], id: HeroId, wording: Wording) -> String {
    let lore = &content.epitaph;
    let hero = &heroes[id];
    let f = forms(content, hero.pronoun);
    let of_kind = |kind: BondKind| hero.bonds.iter().filter(move |b| b.kind == kind);
    let spouses: Vec<String> = of_kind(BondKind::Spouse)
        .map(|b| heroes[b.other].full_name())
        .collect();
    let spouses: Vec<&str> = spouses.iter().map(String::as_str).collect();
    let children: Vec<&str> = of_kind(BondKind::Child)
        .map(|b| heroes[b.other].name.as_str())
        .collect();
    match (spouses.is_empty(), children.is_empty()) {
        (false, false) => {
            return fmt(
                &lore[E::LoveSpousesChildren],
                &[
                    &f.he,
                    &name_list(content, &spouses),
                    &name_list(content, &children),
                ],
            );
        }
        (false, true) => {
            return fmt(
                &lore[E::LoveSpouses],
                &[&f.he, &name_list(content, &spouses)],
            );
        }
        (true, false) => {
            let word = if children.len() == 1 {
                E::LoveChild
            } else {
                E::LoveChildrenWord
            };
            return fmt(
                &lore[E::LoveChildren],
                &[
                    &f.he,
                    &f.possessive,
                    &lore[word],
                    &name_list(content, &children),
                ],
            );
        }
        (true, true) => {}
    }
    let Some(friend) = of_kind(BondKind::Friend).next_back() else {
        return String::new();
    };
    let other = &heroes[friend.other];
    if kin(heroes, id, friend.other) {
        let word = match other.pronoun {
            Pronoun::He => E::LoveBrother,
            Pronoun::She => E::LoveSister,
        };
        return fmt(
            &lore[E::LoveSiblingFriend],
            &[&f.he, &f.possessive, &lore[word], &other.name],
        );
    }
    if wording.coin(Part::Love) {
        fmt(&lore[E::LoveFriend1], &[&f.his, &other.full_name()])
    } else {
        fmt(&lore[E::LoveFriend0], &[&f.he, &other.full_name()])
    }
}

/// END (`:419-448`): the living — at the Door, a child, still living; the dead and the
/// departed — before the first year, or in their year.
pub fn end(content: &Content, hero: &Hero, wording: Wording) -> String {
    let lore = &content.epitaph;
    let f = forms(content, hero.pronoun);
    if hero.is_living() {
        if let Some(stood) = first_deed(hero, DeedKind::StoodAtTheDoor) {
            let age = stood.age.to_string();
            let locks: Vec<&str> = hero
                .deeds
                .iter()
                .filter(|d| d.kind == DeedKind::OpenedALock)
                .map(|d| lock_name(content, hero, d))
                .collect();
            return if locks.is_empty() {
                fmt(&lore[E::EndDoor], &[&f.he, &age])
            } else {
                fmt(
                    &lore[E::EndDoorOpened],
                    &[&f.he, &age, &name_list(content, &locks)],
                )
            };
        }
        let key = if hero.is_adult() {
            E::EndLiving
        } else {
            E::EndChild
        };
        return fmt(&lore[key], &[&f.he, &hero.age.to_string()]);
    }
    let age = hero.fate_age.to_string();
    if hero.fate_year < 1 {
        return fmt(
            &lore[E::EndBeforeFirstYear],
            &[&f.he, &hero.fate_telling, &f.he, &age],
        );
    }
    let year = year_telling(content, hero.fate_year);
    if wording.coin(Part::End) {
        fmt(
            &lore[E::EndDead1],
            &[
                &capitalized(&year),
                &f.subject,
                &hero.fate_telling,
                &f.he,
                &age,
            ],
        )
    } else {
        fmt(
            &lore[E::EndDead0],
            &[&f.he, &hero.fate_telling, &year, &age],
        )
    }
}

/// The name of the lock an OPENED_A_LOCK deed bore open: its weight is the lock's index.
fn lock_name<'c>(content: &'c Content, hero: &Hero, deed: &Deed) -> &'c str {
    match content.epitaph.lock_names.get(deed.weight as usize) {
        Some(name) => name,
        None => panic!(
            "[keifu_x_inheritance_r3v2] {} opened lock {} of the Door, which has {}\n  likely cause: the deed's \
             weight is not the lock's index\n  fix: SPEC §16.2 records weight = lock index",
            hero.name,
            deed.weight,
            content.epitaph.lock_names.len()
        ),
    }
}

/// "Own making" (SPEC §20 helpers): the legacy telling with "<name>'s " turned into
/// "<his/her> own ".
fn own_making(hero: &Hero, possessive: &str) -> String {
    hero.legacy
        .1
        .replace(&format!("{}'s ", hero.name), &format!("{possessive} own "))
}

/// LEFT (`:450-484`): what the living made or carry; the departed's patron; what the dead
/// left — a legacy beside an heirloom, a tale, a blessing, a made heirloom, an heirloom
/// that went or was buried. SPEC-GAPS KG-64: the heir is the bequest's, which a ghost
/// taken up at a coming of age also names.
pub fn left(content: &Content, heroes: &[Hero], id: HeroId) -> String {
    let lore = &content.epitaph;
    let hero = &heroes[id];
    let f = forms(content, hero.pronoun);
    let (kind, telling) = (&hero.legacy.0, &hero.legacy.1);
    if hero.is_living() {
        return match kind {
            LegacyKind::Heirloom => fmt(
                &lore[E::LivingMadeHeirloom],
                &[&f.he, &own_making(hero, &f.possessive)],
            ),
            LegacyKind::Tale => fmt(&lore[E::LivingMadeTale], &[&f.he, &f.object, telling]),
            LegacyKind::Blessing => fmt(&lore[E::LivingMadeBlessing], &[&f.he, telling]),
            LegacyKind::None => match &hero.heirloom {
                Some(heirloom) => fmt(&lore[E::LivingCarriedHeirloom], &[&f.he, &heirloom.name]),
                None => String::new(),
            },
        };
    }
    if hero.fate == Fate::Departed {
        return lore[E::LeftDeparted].to_owned();
    }
    let heir = hero.bequest_heir.map(|h| heroes[h].name.as_str());
    match (kind, &hero.bequest_heirloom, heir) {
        (LegacyKind::None, _, _) => {}
        (_, Some(heirloom), Some(heir)) => {
            let noun = &lore.legacy_nouns[kind.index()];
            return fmt(
                &lore[E::LeftLegacyAndHeirloom],
                &[&f.he, noun, telling, heirloom, heir],
            );
        }
        (LegacyKind::Tale, _, _) => return fmt(&lore[E::LeftTale], &[&f.he, telling]),
        (LegacyKind::Blessing, _, _) => return fmt(&lore[E::LeftBlessing], &[&f.he, telling]),
        (LegacyKind::Heirloom, _, Some(heir)) => {
            return fmt(
                &lore[E::LeftHeirloomMade],
                &[&f.he, &own_making(hero, &f.possessive), heir],
            );
        }
        (LegacyKind::Heirloom, _, None) => {}
    }
    match (&hero.bequest_heirloom, heir) {
        (Some(heirloom), Some(heir)) => {
            fmt(&lore[E::LeftHeirloomWent], &[&capitalized(heirloom), heir])
        }
        (Some(heirloom), None) if hero.bequest_decided => fmt(
            &lore[E::LeftBuriedWith],
            &[&capitalized(heirloom), &f.object],
        ),
        _ => String::new(),
    }
}
