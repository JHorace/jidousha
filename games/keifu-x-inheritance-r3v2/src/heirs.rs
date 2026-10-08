//! Heirs and inheritance (SPEC §15.1-§15.3, §14.4's raising): who may take a dead
//! hero's legacy, in what order, how each is named on the death page — and what the
//! choice does.
//!
//! One owner for both halves of the decision: `heirs` orders the page and
//! `heir_buttons` labels it, and `choose` acts on the press, both asking the same
//! `undone_dream` and `can_take_dream`, so the "(not the dream)" a button shows is
//! exactly the choice that would raise a ghost instead of passing the dream on.
//! The crowned's nearest kin (§15.3) reads the same list.

use crate::bonds::{kinship_telling, steadies};
use crate::constants::HEIRS_OFFERED;
use crate::content::Content;
use crate::dream::{Dream, progress};
use crate::dream_lore::GhostPlace;
use crate::ghost::Ghost;
use crate::hero::{Deed, DeedKind, DreamFate, Hero, HeroId, descends_from, kin};
use crate::house::House;
use crate::ids::{BondKind, Place, Pronoun};
use crate::rivals::dream_rivals;
use crate::text::{capitalized, fmt};
use crate::witness::Held;
use crate::words::W;

/// Where `other` ranks as `dead`'s heir (SPEC §15.1's table): children 0, other
/// descendants 1, the spouse 2, siblings by a shared parent and the parents 3, those
/// the dead taught 4, anyone the dead holds a steadying bond with 5, everyone else 6.
/// SPEC-GAPS KG-39: a hero several rows fit takes the first (the lowest rank), and a
/// parent is known by the bond or by the dead's own parents.
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

/// The dream a death leaves undone (SPEC §15.1 step 2): the burden if dreamt and
/// unfulfilled, else the own dream if dreamt and unfulfilled, else none. Only one
/// dream can pass [emergent] (OQ-3).
pub fn undone_dream(hero: &Hero) -> Option<(Held, &Dream)> {
    fn undone(dream: &Option<Dream>) -> Option<&Dream> {
        dream.as_ref().filter(|d| !d.is_fulfilled())
    }
    match (undone(&hero.burden), undone(&hero.dream)) {
        (Some(burden), _) => Some((Held::Burden, burden)),
        (None, Some(own)) => Some((Held::Own, own)),
        (None, None) => None,
    }
}

/// Whether `hero` can take a dream (SPEC §15.2): no dream, or a burden empty or fulfilled.
pub fn can_take_dream(hero: &Hero) -> bool {
    hero.dream.is_none() || hero.burden.as_ref().is_none_or(Dream::is_fulfilled)
}

/// How `heir` is named beside the dead on an heir button (SPEC §21, "Kinship to the
/// dead"): the dead's child, son or daughter; another descendant, grandson or
/// granddaughter; any shown bond, its kinship from the dead's side; kin, brother or
/// sister; else "of the house".
pub fn kinship_to_dead(content: &Content, heroes: &[Hero], dead: HeroId, heir: HeroId) -> String {
    let words = &content.words;
    let other = &heroes[heir];
    let bond = heroes[dead].bond_to(heir).map(|b| b.kind);
    let he = other.pronoun == Pronoun::He;
    if bond == Some(BondKind::Child) {
        kinship_telling(content, BondKind::Child, other).to_owned()
    } else if descends_from(heroes, heir, dead) {
        words[if he {
            W::HeirKinshipGrandchildHe
        } else {
            W::HeirKinshipGrandchildShe
        }]
        .to_owned()
    } else if let Some(kind) = bond.filter(|k| content.bonds.shown[k.index()]) {
        kinship_telling(content, kind, other).to_owned()
    } else if kin(heroes, dead, heir) {
        words[if he {
            W::HeirKinshipSiblingHe
        } else {
            W::HeirKinshipSiblingShe
        }]
        .to_owned()
    } else {
        words[W::HeirKinshipHouse].to_owned()
    }
}

/// One heir button: who it names (or no one), and what it reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeirButton {
    /// The heir, or `None` for "No one. Let it lie."
    pub heir: Option<HeroId>,
    /// "Maren, daughter", "Ysolde, of the house (not the dream)", "No one. Let it lie."
    pub label: String,
}

/// The death page's buttons (SPEC §15.1): one per heir, in the list's order, each
/// `lines.heir.prospect` with its kinship and, read now by the rule the choice obeys,
/// "(not the dream)" when the dead leaves a dream the heir cannot take, else "(lays
/// one aside)" when both hold an heirloom; then "No one. Let it lie."
///
/// SPEC-GAPS KG-47: the list is fixed when the page is made, the marks are read when
/// they are shown, from the same `can_take_dream` the choice asks.
pub fn heir_buttons(
    content: &Content,
    heroes: &[Hero],
    dead: HeroId,
    list: &[HeroId],
) -> Vec<HeirButton> {
    let words = &content.words;
    let leaves_dream = undone_dream(&heroes[dead]).is_some();
    let leaves_heirloom = heroes[dead].heirloom.is_some();
    let mut buttons: Vec<HeirButton> = list
        .iter()
        .map(|&heir| {
            let mut label = fmt(
                &words[W::HeirProspect],
                &[
                    &heroes[heir].name,
                    &kinship_to_dead(content, heroes, dead, heir),
                ],
            );
            if leaves_dream && !can_take_dream(&heroes[heir]) {
                label.push_str(&words[W::HeirNotTheDream]);
            } else if leaves_heirloom && heroes[heir].heirloom.is_some() {
                label.push_str(&words[W::HeirLaysOneAside]);
            }
            HeirButton {
                heir: Some(heir),
                label,
            }
        })
        .collect();
    buttons.push(HeirButton {
        heir: None,
        label: words[W::TurningNoHeir].to_owned(),
    });
    buttons
}

/// Choose `heir` (or no one) on turning page `page` (SPEC §15.2, `lineage/passage.jai:
/// 250-311`): once, irrevocably. The heirloom goes to the heir — whose own is laid
/// aside and lost (OQ-5) — or into the ground; the undone dream passes to an heir who
/// can take it, or its ghost is raised. The choice's lines go right after the bequest
/// lines, and the dead's epitaph is recomposed with the wording their page rolled.
pub fn choose(content: &Content, house: &mut House, page: usize, heir: Option<HeroId>) {
    let year = house.calendar.current_year();
    let Some(bequest) = house
        .passage
        .as_ref()
        .and_then(|p| p.pages.get(page))
        .and_then(|p| p.bequest.clone())
        .filter(|b| b.undecided())
    else {
        panic!(
            "[keifu_x_inheritance_r3v2] an heir was chosen on turning page {page}, which waits for none\n  likely \
             cause: a heir button offered on a decided page, or off the turning\n  fix: offer \
             heir buttons only on an undecided death page"
        );
    };
    let dead = bequest.dead;
    assert!(
        heir.is_none_or(|h| bequest.heirs.contains(&h)),
        "[keifu_x_inheritance_r3v2] {:?} is not on {}'s heir list\n  likely cause: a button built from another \
         list\n  fix: build the buttons with heirs::heir_buttons",
        heir,
        house.heroes[dead].name
    );
    let words = &content.words;
    let mut lines = Vec::new();
    house.heroes[dead].bequest_decided = true;
    house.heroes[dead].bequest_heir = heir;
    if let Some(heirloom) = house.heroes[dead].heirloom.take() {
        match heir {
            Some(h) => {
                if let Some(old) = house.heroes[h].heirloom.take() {
                    lines.push(fmt(
                        &words[W::HeirLaysAside],
                        &[&house.heroes[h].name, &old.name],
                    ));
                }
                let telling = fmt(
                    &words[W::DeedInherited],
                    &[&heirloom.name, &house.heroes[dead].name],
                );
                deed(
                    &mut house.heroes[h],
                    DeedKind::Inherited,
                    year,
                    Some(dead),
                    telling,
                );
                house.heroes[h].heirloom = Some(heirloom);
            }
            None => {
                let object = &content.lore.pronouns[house.heroes[dead].pronoun.index()].object;
                lines.push(fmt(
                    &words[W::HeirBuriedWith],
                    &[&capitalized(&heirloom.name), object],
                ));
            }
        }
    }
    if let Some((_, dream)) = undone_dream(&house.heroes[dead]) {
        let dream = dream.clone();
        match heir.filter(|&h| can_take_dream(&house.heroes[h])) {
            Some(h) => {
                lines.extend(pass_dream(content, house, dead, h, &dream, year));
                house.heroes[dead].dream_fate = DreamFate::PassedOn;
            }
            None => {
                lines.push(raise_ghost(content, house, dead, &dream));
                house.heroes[dead].dream_fate = DreamFate::LeftToNoOne;
            }
        }
    }
    // SPEC §15.2: "the epitaph is recomposed with the same wording".
    crate::epitaph::recompose(content, &mut house.heroes, dead);
    let Some(page) = house.passage.as_mut().and_then(|p| p.pages.get_mut(page)) else {
        return;
    };
    let at = bequest.bequest_end;
    page.lines.splice(at..at, lines);
    if let Some(b) = page.bequest.as_mut() {
        b.chosen = Some(heir);
    }
}

/// A deed of the turning, dated now (SPEC-GAPS KG-48: no place, weight 0, the other
/// hero its telling names).
pub fn deed(hero: &mut Hero, kind: DeedKind, year: i32, other: Option<HeroId>, telling: String) {
    hero.deeds.push(Deed {
        kind,
        year,
        age: hero.age,
        place: None,
        weight: 0,
        other,
        telling,
    });
}

/// Pass `dream` from `dead` to `heir` (SPEC §15.2, `:293-311`): a copy at its stage
/// and counts, owned by the dead unless it was someone's already; the heir's dream if
/// they have none, else their burden (replacing a fulfilled one). The line, the deed,
/// and dream rivals. Returns the lines.
fn pass_dream(
    content: &Content,
    house: &mut House,
    dead: HeroId,
    heir: HeroId,
    dream: &Dream,
    year: i32,
) -> Vec<String> {
    let mut passed = dream.clone();
    let owner = *passed.owner.get_or_insert(dead);
    let hero = &mut house.heroes[heir];
    if hero.dream.is_none() {
        hero.dream = Some(passed.clone());
    } else {
        hero.burden = Some(passed.clone());
    }
    let words = &content.words;
    let mut lines = vec![fmt(
        &words[W::HeirTakesDream],
        &[
            &house.heroes[heir].name,
            &house.heroes[dead].name,
            &progress(content, &passed, house.heroes[owner].pronoun),
        ],
    )];
    let telling = fmt(
        &words[W::DeedTookUpDream],
        &[&house.heroes[owner].full_name()],
    );
    deed(
        &mut house.heroes[heir],
        DeedKind::TookUpDream,
        year,
        Some(owner),
        telling,
    );
    lines.extend(dream_rivals(
        content,
        &mut house.heroes,
        heir,
        &passed,
        year,
    ));
    lines
}

/// Raise `dead`'s ghost over `dream` (SPEC §14.4): a copy owned by the dead unless it
/// was someone's already, at the dream's place — for the three dreams with none, where
/// the dead died questing (not at the Door), else the Barrow. Returns the line.
fn raise_ghost(content: &Content, house: &mut House, dead: HeroId, dream: &Dream) -> String {
    let mut copy = dream.clone();
    copy.owner.get_or_insert(dead);
    let hero = &house.heroes[dead];
    let place = match content.dreams[dream.kind.index()].ghost_place {
        GhostPlace::Fixed(place) => place,
        GhostPlace::Setup => match dream.setup {
            Some(setup) => setup.place,
            None => panic!(
                "[keifu_x_inheritance_r3v2] {}'s ghost walks at its setup place and the dream holds none\n  likely \
                 cause: a dream built without Dream::build\n  fix: build every dream with \
                 Dream::build",
                dream.kind.id()
            ),
        },
        GhostPlace::WhereTheDreamerDied => hero
            .death_place
            .filter(|p| *p != Place::SealedDoor)
            .unwrap_or(Place::Barrow),
    };
    let line = fmt(
        &content.words[W::GhostRaised],
        &[
            &content.lore.pronouns[hero.pronoun.index()].object,
            &content.lore.places[place.index()].name,
            &hero.name,
        ],
    );
    house.ghosts.push(Ghost {
        hero: dead,
        dream: copy,
        place,
    });
    line
}
