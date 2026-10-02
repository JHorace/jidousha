//! The preview and the winter, held against each other over many hearths.
//!
//! **The agreement battery**: houses founded on a fixed range of seeds, their heroes
//! stirred (wounds, dread, broken fears, ages from five to seventy, aptitudes, callings,
//! the two destinies that bar learning or teach greater, spouses and rivals), every
//! living hero dropped in a random seat of the hall or the hearth — then the plan the
//! seats preview read, the winter resolved and the year turned by the two halves of
//! `season::let_the_winter_pass` (what "Let the winter pass" calls), and the plan the
//! turning records required equal to the preview. Each plan's effect is asked of the house
//! as the winter left it, before the turning's ageing, deaths and comings of age: every
//! lesson's amount in the learner's base, every rest in the wound and the dread, every
//! verdict in the bonds, every teller's renown.
//! **The page**: on staged hearths, the notes the screen draws are the plan's.
//!
//! The battery must have met every excuse but one — "needs a teacher" in the general
//! plan, which no seat reaches (a child alone in the yard belongs on a bench; on a bench,
//! "needs a teacher" is the bench's own) — every verdict and every rest.

use jidousha::prelude::Rng;

use crate::chance::{between, chance, index};
use crate::checks::Checks;
use crate::content::Content;
use crate::hearth::{Group, Seat};
use crate::hearth_view::{notes, notes_now};
use crate::hero::HeroId;
use crate::house::House;
use crate::ids::{BondKind, Destiny, Vocation};
use crate::plans::{Courtship, Excuse, Lesson, Teller};
use crate::verify::{SEEDS, session};
use crate::w7::group_lines;
use crate::winter::plan;

/// The battery's seeds: none another check founds a house on.
const BATTERY: std::ops::Range<u64> = 0x7_7000..0x7_7000 + 3000;

/// Stir a founded house's living heroes and seat each at random (hall or hearth).
fn stir(house: &mut House, rng: &mut Rng) {
    house.calendar.begin_winter();
    house.open_hearth();
    let living: Vec<HeroId> = (0..house.heroes.len())
        .filter(|&h| house.heroes[h].is_living())
        .collect();
    for &h in &living {
        let hero = &mut house.heroes[h];
        hero.wounded = chance(rng, 0.3);
        hero.fear.broken = chance(rng, 0.1);
        hero.fear.dread = if hero.fear.broken {
            5
        } else {
            between(rng, 0, 4)
        };
        hero.age = if chance(rng, 0.4) {
            between(rng, 4, 11)
        } else {
            between(rng, 12, 70)
        };
        hero.aptitudes = [between(rng, 0, 9), between(rng, 0, 9), between(rng, 0, 9)];
        hero.vocation = Vocation::ALL[index(rng, Vocation::ALL.len())];
        if chance(rng, 0.15) {
            hero.destiny.kind = Destiny::TeachAGreater;
        }
    }
    for _ in 0..4 {
        let a = living[index(rng, living.len())];
        let b = living[index(rng, living.len())];
        let kind = if chance(rng, 0.5) {
            BondKind::Spouse
        } else {
            BondKind::Rival
        };
        if house.heroes[a].bond_to(b).is_none() {
            crate::bonds::form(&mut house.heroes, a, b, kind, 0);
        }
    }
    // Every living hero to a distinct random slot: the hall's twelve and the hearth's.
    house.roster = [None; crate::constants::ROSTER_SEATS];
    house.hearth.clear();
    // Half the hearths leave the hall out, so the winter seats fill (the garden's pair).
    let hall = if chance(rng, 0.5) {
        crate::constants::ROSTER_SEATS
    } else {
        0
    };
    let mut slots: Vec<Option<Seat>> = (0..hall).map(|_| None).collect();
    slots.extend(Seat::all().into_iter().map(Some));
    let mut roster_next = 0;
    for &h in &living {
        let pick = slots.remove(index(rng, slots.len()));
        match pick {
            Some(seat) => house.hearth.put(seat, Some(h)),
            None => {
                house.roster[roster_next] = Some(h);
                roster_next += 1;
            }
        }
    }
}

/// What the battery met, to require it met everything.
#[derive(Default)]
struct Met {
    excuses: Vec<Excuse>,
    learned: usize,
    verdicts: Vec<Courtship>,
    rests: [usize; 3],
    tellers: [usize; 3],
}

/// The battery and the page. Returns the summary lines.
pub fn check_agreement(checks: &mut Checks, content: &Content) -> Vec<String> {
    let mut met = Met::default();
    let mut cases = 0;
    for seed in BATTERY {
        let mut rng = Rng::from_seed(seed);
        let Ok(mut house) = House::found(content, seed, &mut rng) else {
            checks.require(
                false,
                "a battery house did not found",
                format!("seed {seed:#x}"),
            );
            continue;
        };
        stir(&mut house, &mut rng);
        let before = house.clone();
        let planned = plan(content, &house);
        let shown = notes(content, &house, &planned);
        // The winter, then the turning: the plan's effects are read between the two.
        let (winter, done) = crate::winter::resolve_winter(content, &mut house, &mut rng);
        let after_winter = house.clone();
        crate::turning::turn_the_year(content, &mut house, winter, done, &mut rng);
        cases += 1;
        let Some(passage) = &house.passage else {
            checks.require(
                false,
                "the winter passed and wrote no turning",
                format!("seed {seed:#x}"),
            );
            continue;
        };
        checks.require(
            passage.done == planned,
            "the winter did not do what its seats previewed",
            format!(
                "seed {seed:#x}: previewed {planned:?}, did {:?}",
                passage.done
            ),
        );
        // The previews are the plan's, whoever sat where.
        checks.require(
            shown == notes(content, &before, &passage.done),
            "the seat notes are not the notes of the plan the winter carried out",
            format!("seed {seed:#x}: {shown:?}"),
        );
        effects(checks, seed, &before, &after_winter, &planned, &mut met);
    }
    let all_excuses = [
        Excuse::ChildTeacher,
        Excuse::LearnsNothing,
        Excuse::NeedsTeacherNow,
        Excuse::TeacherKnowsNoMore,
        Excuse::TooYoung,
        Excuse::ChildLimit,
        Excuse::BelongsOnBench,
        Excuse::IsNoChild,
        Excuse::BenchNeedsTeacher,
    ];
    let all_verdicts = [
        Courtship::Nobody,
        Courtship::Waiting,
        Courtship::Kin,
        Courtship::TooYoung,
        Courtship::TooFarApart,
        Courtship::Rivals,
        Courtship::WedAlready,
        Courtship::WillWed,
    ];
    let count = |list: &[Excuse], e: Excuse| list.iter().filter(|x| **x == e).count();
    let vcount = |list: &[Courtship], v: Courtship| list.iter().filter(|x| **x == v).count();
    checks.require(
        all_excuses.iter().all(|e| count(&met.excuses, *e) > 0)
            && count(&met.excuses, Excuse::NeedsTeacher) == 0
            && all_verdicts.iter().all(|v| vcount(&met.verdicts, *v) > 0)
            && met.rests.iter().all(|&n| n > 0)
            && met.tellers.iter().all(|&n| n > 0)
            && met.learned > 0,
        "the agreement battery did not meet every excuse, verdict, rest and teller (or met the unreachable one)",
        format!(
            "excuses {:?}, verdicts {:?}, rests {:?}, tellers {:?}, lessons {}",
            all_excuses.map(|e| count(&met.excuses, e)),
            all_verdicts.map(|v| vcount(&met.verdicts, v)),
            met.rests,
            met.tellers,
            met.learned
        ),
    );
    let page = check_page(checks);
    vec![
        format!(
            "W7 agreement battery: {cases} stirred hearths, every seat's preview the plan the winter carried out, and every plan's effect in the house; lessons learned {}, excuses {:?}, verdicts {:?}, rests heals/calms/rests {:?}, tellers house/own/child {:?}",
            met.learned,
            all_excuses.map(|e| (e, count(&met.excuses, e))),
            all_verdicts.map(|v| (v, vcount(&met.verdicts, v))),
            met.rests,
            met.tellers
        ),
        page,
    ]
}

/// Every plan's effect, read off the house after the winter.
fn effects(
    checks: &mut Checks,
    seed: u64,
    before: &House,
    after: &House,
    planned: &crate::winter::WinterPlan,
    met: &mut Met,
) {
    let at = |seat| before.hearth.at(seat);
    let mut lessons = vec![(Seat::Learner, planned.yard)];
    lessons
        .extend((0..crate::constants::BENCHES).map(|b| (Seat::BenchChild(b), planned.benches[b])));
    for (seat, lesson) in lessons {
        let (Some(learner), Some(lesson)) = (at(seat), lesson) else {
            continue;
        };
        match lesson {
            Lesson::Learn { aptitude, amount } => {
                met.learned += 1;
                let (was, now) = (
                    before.heroes[learner].base(aptitude),
                    after.heroes[learner].base(aptitude),
                );
                checks.require(
                    now == was + amount && amount > 0 && now <= 9,
                    "a planned lesson is not what the learner learned",
                    format!("seed {seed:#x}, {seat:?}: {aptitude:?} {was} + {amount} -> {now}"),
                );
            }
            Lesson::Wasted(excuse) => {
                met.excuses.push(excuse);
                checks.require(
                    before.heroes[learner].aptitudes == after.heroes[learner].aptitudes,
                    "a wasted lesson changed the learner's aptitudes",
                    format!("seed {seed:#x}, {seat:?}: {excuse:?}"),
                );
            }
        }
    }
    for (i, rest) in planned.rests.iter().enumerate() {
        let (Some(hero), Some(rest)) = (at(Seat::Fire(i)), rest) else {
            continue;
        };
        met.rests[if rest.heals {
            0
        } else if rest.calms {
            1
        } else {
            2
        }] += 1;
        let (b, a) = (&before.heroes[hero], &after.heroes[hero]);
        // A dream fulfilled at the fire's own moment sets dread to 0 after the shedding.
        let dread_ok = if a.settled && !b.settled {
            a.fear.dread == 0
        } else {
            a.fear.dread == b.fear.dread - i32::from(rest.calms)
        };
        checks.require(
            !a.wounded && dread_ok && rest.heals == b.wounded,
            "a fire seat's rest is not what the hero had",
            format!(
                "seed {seed:#x}: {rest:?}, dread {} -> {}, wounded {}",
                b.fear.dread, a.fear.dread, b.wounded
            ),
        );
    }
    if let Some(verdict) = planned.garden {
        met.verdicts.push(verdict);
        if let (Some(a), Some(b)) = (at(Seat::Garden(0)), at(Seat::Garden(1))) {
            let kind = after.heroes[a].bond_to(b).map(|bond| bond.kind);
            let ok = match verdict {
                Courtship::WillWed => kind == Some(BondKind::Spouse),
                Courtship::Rivals => kind == Some(BondKind::Friend),
                _ => kind == before.heroes[a].bond_to(b).map(|bond| bond.kind),
            };
            checks.require(
                ok,
                "the garden's verdict is not what became of the pair",
                format!("seed {seed:#x}: {verdict:?}, bond {kind:?}"),
            );
        }
    }
    let mut house_renown = 0;
    for (i, part) in planned.tellers.iter().enumerate() {
        let (Some(hero), Some(part)) = (at(Seat::Table(i)), part) else {
            continue;
        };
        met.tellers[match part {
            Teller::House => 0,
            Teller::Own => 1,
            Teller::Child => 2,
        }] += 1;
        let gained = after.heroes[hero].renown - before.heroes[hero].renown;
        house_renown += i32::from(*part == Teller::House);
        checks.require(
            gained == i32::from(*part != Teller::Child),
            "a teller's renown is not their part's",
            format!("seed {seed:#x}: {part:?} gained {gained}"),
        );
    }
    checks.require(
        after.renown - before.renown == house_renown,
        "the house's renown did not rise by the table's one, once",
        format!("seed {seed:#x}: {} -> {}", before.renown, after.renown),
    );
}

/// On staged hearths, the notes the page draws in each group are the plan's notes.
fn check_page(checks: &mut Checks) -> String {
    let mut judged = 0;
    for case in 0..24u64 {
        let mut sim = session(SEEDS[0]);
        crate::w7::stay_home_into_winter(&mut sim);
        let mut rng = Rng::from_seed(0x7_8000 + case);
        stir(sim.world_mut().resource_mut::<House>(), &mut rng);
        let want = {
            let ui = *sim.world().resource::<crate::screen::UiState>();
            notes_now(
                crate::verify::content_of(&sim),
                sim.world().resource::<House>(),
                &ui,
            )
        };
        let wanted: Vec<(Group, Vec<String>)> = vec![
            (Group::Fire, want.fire.iter().flatten().cloned().collect()),
            (Group::Training, want.training.iter().cloned().collect()),
            (Group::Garden, want.garden.iter().cloned().collect()),
            (Group::Table, want.table.iter().flatten().cloned().collect()),
            (
                Group::Benches,
                want.benches.iter().flatten().cloned().collect(),
            ),
        ];
        for (group, notes) in wanted {
            let drawn = group_lines(&sim, group);
            judged += 1;
            checks.require(
                notes.iter().all(|n| drawn.contains(n)),
                "a seat's preview drawn on the hearth is not the plan's note",
                format!("case {case}, {group:?}: want {notes:?} among {drawn:?}"),
            );
        }
    }
    format!(
        "W7 page: on 24 stirred hearths every group's drawn notes are the plan's ({judged} groups read)"
    )
}
