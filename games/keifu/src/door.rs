//! The Sealed Door (SPEC §16): the last summer's one quest, three locks tried in turn by
//! one party, and the outlook that reads them before they are tried.
//!
//! **One source.** A lock is a `Quest` (`lock_quest`): the Door's place and tags, its
//! seats, danger and renown, the lock's aptitude and demand. The card's and the top bar's
//! numbers (`outlook`) and the resolution (`try_the_door`) read that one quest through the
//! same `power::party_power` and `forecast::forecast`, so a number the player reads before
//! "Try the Door" is the number the lock rolls against — until the locks change the party
//! (OQ-15: the outlook's "all three" is the product of three chances, as if nothing carried
//! over; the resolution carries everything over).
//!
//! **The resolution** (§16.2, `lineage/door.jai:127-162`): the prologue (`door_prologue`),
//! then for each lock in order Might, Wits, Spirit — the standing party (the seated still
//! living), the bearer (the standing member with the highest solo power for the lock,
//! patrons 0, the first on ties), the lock resolved as a quest (`resolve::resolve_party`)
//! with the lock's ending as its story — and a lock opened on a success or a triumph, the
//! living bearer's OPENED_A_LOCK deed with it. Wounds, deaths, dread and breaking carry
//! from one lock into the next [emergent]: the next lock's party is read off the house
//! after the last one changed it, and a hero who breaks at the Door keeps going (OQ-24).
//! Then every living member stands at the Door.

use jidousha::prelude::Rng;

use crate::chance::between;
use crate::constants::{BEST_FOUR_DIVISOR, DICE_SIDES, DOOR_DANGER, DOOR_RENOWN, DOOR_SEATS};
use crate::content::Content;
use crate::forecast::{DICE_OUTCOMES, forecast, percent};
use crate::hero::{Deed, DeedKind, Hero, HeroId};
use crate::house::House;
use crate::ids::{Outcome, Place};
use crate::power::party_power;
use crate::quest::{Quest, Source};
use crate::reading::adults;
use crate::resolve::resolve_party;
use crate::telling::QuestPage;
use crate::text::{fmt, fmt_numbered_story, name_list};
use crate::words::W;

/// Lock `lock` as the quest it is resolved as (SPEC §16.2: "demand 34, danger 3, renown 5,
/// tags Dark+Cold, 4 seats"; CONSTANTS §12: no wobble, no year creep, no trouble).
pub fn lock_quest(content: &Content, lock: usize) -> Quest {
    let words = &content.door.locks[lock];
    Quest {
        source: Source::Door(lock),
        title: words.title.clone(),
        premise: words.premise.clone(),
        place: Place::SealedDoor,
        aptitude: words.aptitude,
        tags: content.lore.places[Place::SealedDoor.index()].tags.clone(),
        calm_seats: DOOR_SEATS,
        seats: DOOR_SEATS,
        calm_danger: DOOR_DANGER,
        danger: DOOR_DANGER,
        renown: DOOR_RENOWN,
        demand: words.demand,
        trouble: 0,
    }
}

/// The last summer's board (SPEC §5.1): slot 0 is the Might lock, four seats.
pub fn door_board(content: &Content) -> Vec<Quest> {
    vec![lock_quest(content, 0)]
}

/// A party's outlook on the three locks (SPEC §16.4): what it brings to each, and how
/// many of the 36 dice pairs open each (success or better).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outlook {
    /// Who, in seat (or creation) order.
    pub party: Vec<HeroId>,
    /// Power at each lock, the house's patrons counted.
    pub powers: [i32; 3],
    /// Pairs of 36 that open each lock.
    pub ways: [i32; 3],
}

impl Outlook {
    /// "open N in 100" for one lock: CONSTANTS §3's `percent`.
    pub fn lock_percent(&self, lock: usize) -> i32 {
        percent(self.ways[lock])
    }

    /// "All three open: P in 100": `percent` of the product of the three chances, in
    /// integers — `int(x * 100 + 0.5)` for x = k / 36^3. No k / 46656 lies within 8 / 93312
    /// of a half, so the original's float rounding cannot land elsewhere.
    pub fn all_percent(&self) -> i32 {
        let all = DICE_OUTCOMES.pow(3);
        let product: i32 = self.ways.iter().product();
        (product * 100 + all / 2) / all
    }

    /// The best-four score `P(all three) + (P1 + P2 + P3) / 1000` (CONSTANTS §12), scaled
    /// by 36^3 * 1000 so two parties compare exactly (SPEC-GAPS KG-65).
    fn score(&self) -> i64 {
        let product: i64 = self.ways.iter().map(|&w| i64::from(w)).product();
        let sum: i64 = self.ways.iter().map(|&w| i64::from(w)).sum();
        product * BEST_FOUR_DIVISOR as i64 + sum * i64::from(DICE_OUTCOMES).pow(2)
    }
}

/// The outlook for `party` (SPEC §16.4): each lock's power with the house's patrons, and
/// its chance — read through the lock's own quest, as the resolution reads it.
pub fn outlook(content: &Content, heroes: &[Hero], party: &[HeroId], patrons: i32) -> Outlook {
    let mut powers = [0; 3];
    let mut ways = [0; 3];
    for lock in 0..3 {
        let quest = lock_quest(content, lock);
        powers[lock] = party_power(heroes, party, quest.facts(), patrons);
        let odds = forecast(powers[lock], quest.demand, !party.is_empty());
        ways[lock] = odds.ways(Outcome::Success) + odds.ways(Outcome::Triumph);
    }
    Outlook {
        party: party.to_vec(),
        powers,
        ways,
    }
}

/// The best four (SPEC §16.4): with four living adults or fewer, all of them; otherwise
/// every combination of four in creation order, the first strictly best kept. The wounded
/// and those who would refuse are not left out [emergent] (OQ-14).
pub fn best_four(content: &Content, house: &House) -> Outlook {
    let living = adults(&house.heroes);
    let seats = DOOR_SEATS as usize;
    if living.len() <= seats {
        return outlook(content, &house.heroes, &living, house.patrons);
    }
    let mut best: Option<Outlook> = None;
    let mut pick = [0usize, 1, 2, 3];
    loop {
        let party: Vec<HeroId> = pick.iter().map(|&i| living[i]).collect();
        let this = outlook(content, &house.heroes, &party, house.patrons);
        if best.as_ref().is_none_or(|b| this.score() > b.score()) {
            best = Some(this);
        }
        // The next combination in lexicographic (creation) order.
        let Some(at) = (0..seats)
            .rev()
            .find(|&i| pick[i] < living.len() - seats + i)
        else {
            break;
        };
        pick[at] += 1;
        for i in at + 1..seats {
            pick[i] = pick[i - 1] + 1;
        }
    }
    best.unwrap_or_else(|| outlook(content, &house.heroes, &[], house.patrons))
}

/// The bearer of lock `quest` among `standing` (SPEC §16.2): the highest solo power for
/// the lock, patrons 0; the first on ties.
pub fn bearer(heroes: &[Hero], standing: &[HeroId], quest: &Quest) -> Option<HeroId> {
    let mut best: Option<(i32, HeroId)> = None;
    for &member in standing {
        let solo = party_power(heroes, &[member], quest.facts(), 0);
        if best.is_none_or(|(power, _)| solo > power) {
            best = Some((solo, member));
        }
    }
    best.map(|(_, member)| member)
}

/// One lock as it was tried.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LockTried {
    /// Which lock.
    pub lock: usize,
    /// Who stood before it, in party order.
    pub standing: Vec<HeroId>,
    /// Who bore it.
    pub bearer: HeroId,
    /// It gave: a success or a triumph.
    pub opened: bool,
}

/// What the Door was (SPEC §16.2), for the telling and the verdict.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DoorRecord {
    /// The prologue's lines, in order.
    pub prologue: Vec<String>,
    /// The party, in seat order: the first lock's.
    pub party: Vec<HeroId>,
    /// Each lock tried, in order; the pages are `Telling::pages`, one to a lock.
    pub tried: Vec<LockTried>,
}

impl DoorRecord {
    /// How many locks gave.
    pub fn locks_opened(&self) -> usize {
        self.tried.iter().filter(|t| t.opened).count()
    }
}

/// A deed at the Door, dated now (SPEC-GAPS KG-70: at the Sealed Door, no other hero).
fn deed(hero: &mut Hero, kind: DeedKind, year: i32, weight: i32, telling: String) {
    hero.deeds.push(Deed {
        kind,
        year,
        age: hero.age,
        place: Some(Place::SealedDoor),
        weight,
        other: None,
        telling,
    });
}

/// "Try the Door" (SPEC §16.2): the party is the Door's seats. Each lock's page goes on
/// `pages`; the record is returned. `staged` throws each lock's dice for the checks; play
/// rolls them.
pub fn try_the_door(
    content: &Content,
    house: &mut House,
    rng: &mut Rng,
    staged: Option<[[i32; 2]; 3]>,
    pages: &mut Vec<QuestPage>,
) -> DoorRecord {
    let party = house.party(0);
    assert!(
        house.calendar.door_stands_open() && !party.is_empty(),
        "[keifu] the Door was tried with nobody before it, or before the last summer\n  \
         likely cause: \"Try the Door\" offered with no seated hero\n  fix: SPEC §16.1 — it \
         needs at least one"
    );
    let year = house.calendar.current_year();
    let mut record = DoorRecord {
        prologue: crate::door_prologue::prologue(content, house, &party),
        party: party.clone(),
        tried: Vec::new(),
    };
    for (lock, words) in content.door.locks.iter().enumerate() {
        let standing: Vec<HeroId> = party
            .iter()
            .copied()
            .filter(|&m| house.heroes[m].is_living())
            .collect();
        let quest = lock_quest(content, lock);
        let Some(bearer) = bearer(&house.heroes, &standing, &quest) else {
            break;
        };
        let dice = match staged {
            Some(dice) => dice[lock],
            None => [between(rng, 1, DICE_SIDES), between(rng, 1, DICE_SIDES)],
        };
        let names: Vec<String> = standing
            .iter()
            .map(|&m| house.heroes[m].name.clone())
            .collect();
        // SPEC-GAPS KG-69: some endings name only the bearer (`%2`); the party's `%1` is
        // then left out, as the original's print leaves an unused argument.
        let told = |house: &House, outcome: Outcome| {
            let names: Vec<&str> = names.iter().map(String::as_str).collect();
            fmt_numbered_story(
                &words.endings[outcome.index()],
                &[&name_list(content, &names), &house.heroes[bearer].name],
            )
        };
        let page = resolve_party(content, house, rng, quest, standing.clone(), dice, &told);
        let opened = page.outcome >= Outcome::Success;
        if opened && house.heroes[bearer].is_living() {
            let telling = fmt(&content.words[W::DeedOpenedALock], &[&words.name]);
            deed(
                &mut house.heroes[bearer],
                DeedKind::OpenedALock,
                year,
                lock as i32,
                telling,
            );
        }
        pages.push(page);
        record.tried.push(LockTried {
            lock,
            standing,
            bearer,
            opened,
        });
    }
    let opened = record.locks_opened() as i32;
    for &member in &party {
        if house.heroes[member].is_living() {
            let telling = content.words[W::DeedStoodAtTheDoor].to_owned();
            deed(
                &mut house.heroes[member],
                DeedKind::StoodAtTheDoor,
                year,
                opened,
                telling,
            );
        }
    }
    record
}
