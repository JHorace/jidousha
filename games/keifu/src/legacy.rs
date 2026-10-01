//! Leaving a legacy (SPEC §14.1), receiving a new heirloom while holding one, and
//! the heir of the blood (§14.2).
//!
//! A fulfilled dream leaves its legacy here: an heirloom forged and given to the
//! fulfiller, a house tale, or a blessing on the line. The legacy is named for the
//! *dreamer* (the dream's owner, else the fulfiller) and given or placed by the
//! fulfiller. The rule returns the lines it writes, in order; the page is the
//! caller's (the telling, W6; the hearth, W7; the turning, W8).

use crate::blessing::{bless, blessing_effect, word_blessing};
use crate::constants::{BLADE_NAMES, TALE_YEARLY_RENOWN};
use crate::content::Content;
use crate::dream::Dream;
use crate::hero::{Deed, DeedKind, Heirloom, Hero, HeroId};
use crate::house::{House, Tale};
use crate::ids::{BondKind, DreamKind, LegacyKind};
use crate::text::{fmt, name_list};
use crate::words::W;

/// Leave `dream`'s legacy, fulfilled by `fulfiller` in `year` (SPEC §14.1).
/// Returns the lines it writes.
pub fn leave_legacy(
    content: &Content,
    house: &mut House,
    fulfiller: HeroId,
    dream: &Dream,
    year: i32,
) -> Vec<String> {
    let words = &content.words;
    let dreamer = dream.owner.unwrap_or(fulfiller);
    let kind = content.dreams[dream.kind.index()].legacy;
    let mut lines = Vec::new();
    let telling = match kind {
        LegacyKind::Heirloom => {
            let heirloom = forge(content, house, dream.kind, dreamer, fulfiller, year);
            let name = heirloom.name.clone();
            let effect = fmt(
                &content.legacies.heirloom_effect,
                &[
                    &heirloom.bonus.to_string(),
                    &content.lore.aptitudes[heirloom.aptitude.index()],
                ],
            );
            let provenance = heirloom.provenance.clone();
            // SPEC-GAPS KG-18: the giving's lines first, then the heirloom's, as §14.1 lists them.
            lines.extend(give_heirloom(
                content,
                &mut house.heroes,
                fulfiller,
                heirloom,
            ));
            lines.push(fmt(
                &words[W::LegacyHeirloom],
                &[&name, &effect, &provenance],
            ));
            name
        }
        LegacyKind::Tale => {
            let titles = &content.legacies.tale_titles;
            let title = match dream.kind {
                DreamKind::SeeTheSea => &titles[0],
                DreamKind::RoofOfTheWorld => &titles[1],
                DreamKind::KnownAtCourt => &titles[2],
                _ => &titles[3],
            };
            let title = fmt(title, &[&house.heroes[dreamer].name]);
            // SPEC-GAPS KG-24: about the dreamer, since the year it was left.
            house.tales.push(Tale {
                title: title.clone(),
                about: dreamer,
                since: year,
            });
            lines.push(fmt(
                &words[W::LegacyTale],
                &[&title, &TALE_YEARLY_RENOWN.to_string()],
            ));
            title
        }
        LegacyKind::Blessing => {
            let blessing = word_blessing(content, dream, &house.heroes[dreamer]);
            let blessed = bless(&mut house.heroes, &blessing, dream.kind, dreamer, fulfiller);
            let names: Vec<&str> = blessed
                .iter()
                .map(|&id| house.heroes[id].name.as_str())
                .collect();
            lines.push(fmt(
                &words[W::LegacyBlessing],
                &[
                    &blessing.title,
                    &blessing_effect(content, &blessing),
                    &name_list(content, &names),
                ],
            ));
            blessing.title
        }
        LegacyKind::None => panic!(
            "[keifu] {} leaves no legacy, and SPEC §14.1 gives every dream one\n  likely cause: \
             dreams.json changed a legacy and the loader's check was bypassed\n  fix: compare \
             dreams.json with SPEC §14.1",
            dream.kind.id()
        ),
    };
    let hero = &mut house.heroes[fulfiller];
    hero.legacy = (kind, telling.clone());
    // SPEC-GAPS KG-15: year and age now; no place, weight 0, no other hero.
    hero.deeds.push(Deed {
        kind: DeedKind::LeftLegacy,
        year,
        age: hero.age,
        place: None,
        weight: 0,
        other: None,
        telling: fmt(&words[W::DeedLeftLegacy], &[&telling]),
    });
    lines
}

/// Forge the heirloom a dream leaves (SPEC §14.1): the next blade name for a blade,
/// "<dreamer>'s road-book", or "the <fulfiller's house> cradle-ring"; +2; its
/// provenance names the fulfiller and the year.
fn forge(
    content: &Content,
    house: &mut House,
    kind: DreamKind,
    dreamer: HeroId,
    fulfiller: HeroId,
    year: i32,
) -> Heirloom {
    let legacies = &content.legacies;
    let (lore, name) = match kind {
        DreamKind::ForgeABlade => {
            let name = legacies.blade_names[house.blades_named % BLADE_NAMES].clone();
            house.blades_named += 1;
            (&legacies.heirlooms[0], name)
        }
        DreamKind::WalkEveryRoad => (
            &legacies.heirlooms[1],
            fmt(&legacies.heirlooms[1].name, &[&house.heroes[dreamer].name]),
        ),
        _ => (
            &legacies.heirlooms[2],
            fmt(
                &legacies.heirlooms[2].name,
                &[&house.heroes[fulfiller].house],
            ),
        ),
    };
    Heirloom {
        name,
        sprite: lore.sprite.clone(),
        aptitude: lore.aptitude,
        bonus: lore.bonus,
        provenance: fmt(
            &lore.provenance,
            &[&house.heroes[fulfiller].full_name(), &year.to_string()],
        ),
    }
}

/// Give `heirloom` to `hero` (SPEC §14.2). An heirloom already held passes to the
/// hero's heir, overwriting whatever the heir held, or with no heir is hung over the
/// hearth and lost. Returns the lines.
pub fn give_heirloom(
    content: &Content,
    heroes: &mut [Hero],
    hero: HeroId,
    heirloom: Heirloom,
) -> Vec<String> {
    let words = &content.words;
    let mut lines = Vec::new();
    let heir = heir(heroes, hero);
    if let Some(old) = heroes[hero].heirloom.take() {
        match heir {
            Some(heir) => {
                lines.push(fmt(
                    &words[W::LegacyHandsFull],
                    &[&heroes[hero].name, &old.name, &heroes[heir].name],
                ));
                heroes[heir].heirloom = Some(old);
            }
            // SPEC-GAPS KG-21: the name as it is, not capitalised.
            None => lines.push(fmt(&words[W::LegacyHungOverHearth], &[&old.name])),
        }
    }
    heroes[hero].heirloom = Some(heirloom);
    lines
}

/// The heir of an heirloom passed on in life (SPEC §14.2): the heir of the blood —
/// the earliest-born living, empty-handed descendant found by walking the hero's
/// children and, past a child who is dead or holds one, that child's own line;
/// else the first bond, in bond order, to a living, empty-handed spouse or someone
/// the hero taught; else nobody.
pub fn heir(heroes: &[Hero], hero: HeroId) -> Option<HeroId> {
    let mut found = Vec::new();
    of_the_blood(heroes, hero, &mut found);
    let mut first: Option<HeroId> = None;
    // SPEC-GAPS KG-19: on equal born years the first found is kept.
    for candidate in found {
        if first.is_none_or(|f| heroes[candidate].born_year < heroes[f].born_year) {
            first = Some(candidate);
        }
    }
    first.or_else(|| {
        heroes[hero]
            .bonds
            .iter()
            .find(|bond| {
                let other = &heroes[bond.other];
                other.is_living()
                    && other.heirloom.is_none()
                    && (bond.kind == BondKind::Spouse || bond.taught)
            })
            .map(|bond| bond.other)
    })
}

fn of_the_blood(heroes: &[Hero], hero: HeroId, found: &mut Vec<HeroId>) {
    for bond in heroes[hero]
        .bonds
        .iter()
        .filter(|b| b.kind == BondKind::Child)
    {
        let child = &heroes[bond.other];
        if child.is_living() && child.heirloom.is_none() {
            found.push(bond.other);
        } else {
            of_the_blood(heroes, bond.other, found);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bonds::form;
    use crate::testkit::{house, id};

    fn blade_dream(content: &Content) -> Dream {
        Dream::build(content, DreamKind::ForgeABlade, None, None).expect("builds")
    }

    fn held(house: &House, name: &str) -> Option<String> {
        house.heroes[id(&house.heroes, name)]
            .heirloom
            .as_ref()
            .map(|h| h.name.clone())
    }

    #[test]
    fn blades_take_their_names_in_order_and_the_eleventh_is_the_first_again() {
        let (content, mut house) = house();
        let brannoc = id(&house.heroes, "Brannoc");
        let dream = blade_dream(&content);
        let mut names = Vec::new();
        for _ in 0..11 {
            house.heroes[brannoc].heirloom = None;
            leave_legacy(&content, &mut house, brannoc, &dream, 4);
            names.extend(held(&house, "Brannoc"));
        }
        assert_eq!(
            names,
            [
                "Emberwake",
                "Last Word",
                "Coldharbor",
                "The Long Answer",
                "Widow's Patience",
                "Morning",
                "Kept Promise",
                "Second Thought",
                "Hearthward",
                "Undertow",
                "Emberwake",
            ]
        );
        assert_eq!(house.blades_named, 11);
    }

    #[test]
    fn a_new_heirloom_passes_the_old_one_to_the_heir_of_the_blood() {
        let (content, mut house) = house();
        let garrick = id(&house.heroes, "Garrick");
        let lines = leave_legacy(&content, &mut house, garrick, &blade_dream(&content), 7);
        assert_eq!(
            lines,
            [
                "Garrick has two hands and one of them is full. Thornfall passes to Maren.",
                "It leaves an heirloom: Emberwake. +2 Might on quests. Forged by Garrick Thorne at Emberfall, and named in year 7 after a triumph.",
            ]
        );
        assert_eq!(held(&house, "Garrick").as_deref(), Some("Emberwake"));
        assert_eq!(held(&house, "Maren").as_deref(), Some("Thornfall"));
        let maren = id(&house.heroes, "Maren");
        assert_eq!(
            house.heroes[maren].heirloom.as_ref().map(|h| h.bonus),
            Some(1)
        );
        let garrick_hero = &house.heroes[garrick];
        assert_eq!(
            garrick_hero.legacy,
            (LegacyKind::Heirloom, "Emberwake".to_owned())
        );
        assert_eq!(
            garrick_hero
                .deeds
                .last()
                .map(|d| (d.kind, d.telling.as_str())),
            Some((DeedKind::LeftLegacy, "left Emberwake"))
        );
    }

    #[test]
    fn the_blood_runs_past_a_child_who_holds_one_to_the_earliest_born_grandchild() {
        let (_, mut house) = house();
        let (garrick, maren, pip) = (
            id(&house.heroes, "Garrick"),
            id(&house.heroes, "Maren"),
            id(&house.heroes, "Pip"),
        );
        house.heroes[maren].heirloom = house.heroes[garrick].heirloom.clone();
        assert_eq!(heir(&house.heroes, garrick), Some(pip));
        // A second grandchild, born before Pip: the earlier one is the heir.
        let mut elder = house.heroes[pip].clone();
        elder.name = "Elder".into();
        elder.born_year = -12;
        elder.bonds.clear();
        house.heroes.push(elder);
        let born_first = house.heroes.len() - 1;
        form(&mut house.heroes, maren, born_first, BondKind::Child, -12);
        assert_eq!(heir(&house.heroes, garrick), Some(born_first));
        // A dead child's line is walked too.
        house.heroes[maren].heirloom = None;
        house.heroes[maren].fate = crate::hero::Fate::Dead;
        assert_eq!(heir(&house.heroes, garrick), Some(born_first));
    }

    #[test]
    fn with_no_blood_the_heir_is_a_spouse_or_someone_taught_and_with_no_one_it_hangs_over_the_hearth()
     {
        let (content, mut house) = house();
        let (odo, wren, brannoc) = (
            id(&house.heroes, "Odo"),
            id(&house.heroes, "Wren"),
            id(&house.heroes, "Brannoc"),
        );
        assert_eq!(heir(&house.heroes, odo), None, "a friend is no heir");
        let thornfall = house.heroes[id(&house.heroes, "Garrick")]
            .heirloom
            .clone()
            .expect("Thornfall");
        house.heroes[odo].heirloom = Some(thornfall.clone());
        let lines = give_heirloom(&content, &mut house.heroes, odo, thornfall.clone());
        assert_eq!(
            lines,
            ["Thornfall is hung over the hearth. There is no one to take it."]
        );
        form(&mut house.heroes, wren, odo, BondKind::Mentor, 2);
        assert_eq!(
            heir(&house.heroes, odo),
            None,
            "a student untaught is no heir"
        );
        if let Some(bond) = house.heroes[odo].bonds.iter_mut().find(|b| b.other == wren) {
            bond.taught = true;
        }
        assert_eq!(heir(&house.heroes, odo), Some(wren));
        // Brannoc's child Wren is empty-handed: the blood comes before anyone else.
        assert_eq!(heir(&house.heroes, brannoc), Some(wren));
        house.heroes[wren].heirloom = Some(thornfall);
        assert_eq!(
            heir(&house.heroes, brannoc),
            None,
            "his wife is dead, his child's hands are full"
        );
    }

    #[test]
    fn a_road_book_is_named_for_its_dreamer_and_a_cradle_ring_for_its_makers_house() {
        let (content, mut house) = house();
        let (ysolde, maren) = (id(&house.heroes, "Ysolde"), id(&house.heroes, "Maren"));
        let mut road =
            Dream::build(&content, DreamKind::WalkEveryRoad, None, None).expect("builds");
        road.owner = Some(ysolde);
        let lines = leave_legacy(&content, &mut house, maren, &road, 9);
        assert_eq!(
            lines,
            [
                "It leaves an heirloom: Ysolde's road-book. +2 Wits on quests. Every road there is, written down by Maren Thorne and finished in year 9."
            ]
        );
        let mut ring =
            Dream::build(&content, DreamKind::SeeAChildGrown, None, None).expect("builds");
        // Aud Hale dreamt it; Ysolde Vane made it: the ring is the maker's house's.
        ring.owner = Some(id(&house.heroes, "Aud"));
        let lines = leave_legacy(&content, &mut house, ysolde, &ring, 9);
        assert_eq!(
            lines[0],
            "It leaves an heirloom: the Vane cradle-ring. +2 Spirit on quests. Made by Ysolde Vane in year 9, the year a child of the house came of age."
        );
        assert_eq!(
            house.heroes[ysolde]
                .heirloom
                .as_ref()
                .map(|h| (h.aptitude, h.sprite.as_str())),
            Some((crate::ids::Aptitude::Spirit, "reward-ring"))
        );
        assert_eq!(
            house.heroes[maren]
                .heirloom
                .as_ref()
                .map(|h| (h.aptitude, h.sprite.as_str())),
            Some((crate::ids::Aptitude::Wits, "reward-guidebook"))
        );
    }

    #[test]
    fn the_tales_are_named_for_the_dreamer_by_the_dreams_kind() {
        let (content, mut house) = house();
        let odo = id(&house.heroes, "Odo");
        for (kind, want) in [
            (
                DreamKind::RoofOfTheWorld,
                "It leaves a tale: The tale of Odo on the roof of the world. It will be told as long as there is a house: +1 renown every year.",
            ),
            (
                DreamKind::KnownAtCourt,
                "It leaves a tale: The tale of Odo at Court. It will be told as long as there is a house: +1 renown every year.",
            ),
        ] {
            let dream = Dream::build(&content, kind, None, None).expect("builds");
            assert_eq!(leave_legacy(&content, &mut house, odo, &dream, 3), [want]);
        }
        assert_eq!(
            house
                .tales
                .iter()
                .map(|t| t.title.as_str())
                .collect::<Vec<_>>(),
            [
                "The tale of Odo on the roof of the world",
                "The tale of Odo at Court"
            ]
        );
    }

    #[test]
    fn a_living_spouse_with_empty_hands_comes_before_the_taught_and_full_hands_take_nothing() {
        let (_, mut house) = house();
        let (odo, ysolde, wren) = (
            id(&house.heroes, "Odo"),
            id(&house.heroes, "Ysolde"),
            id(&house.heroes, "Wren"),
        );
        form(&mut house.heroes, odo, ysolde, BondKind::Spouse, 2);
        form(&mut house.heroes, wren, odo, BondKind::Mentor, 2);
        if let Some(bond) = house.heroes[odo].bonds.iter_mut().find(|b| b.other == wren) {
            bond.taught = true;
        }
        assert_eq!(heir(&house.heroes, odo), Some(ysolde));
        let thornfall = house.heroes[id(&house.heroes, "Garrick")].heirloom.clone();
        house.heroes[ysolde].heirloom = thornfall.clone();
        assert_eq!(
            heir(&house.heroes, odo),
            Some(wren),
            "the wife's hands are full"
        );
        house.heroes[wren].heirloom = thornfall;
        assert_eq!(heir(&house.heroes, odo), None, "and so are the student's");
    }
}
