//! Game flow: which beat, which stage, who is selected — and the pointer that
//! moves all three.
//!
//! An explicit state machine in a resource (DESIGN.md §9). Three stages and two
//! verbs: in `Assembly` the player selects a party and sends it; in `Report`
//! they read what it cost and continue; `Complete` is the end of the chain.
//! Pointer only.
//!
//! **The gate and the preview are one function.** `assess` answers "can this be
//! sent, and what does each member say" and is called by the send verb and by
//! the preview alike, so what the UI shows and what the simulation allows
//! cannot disagree. It calls `model::willingness`, which is also what
//! resolution's betrayal and drift are computed beside — one decision function,
//! three firing moments.

use jidousha::prelude::*;

use crate::beats::{CHAIN, Dungeon, EdgeSpec};
use crate::constants::Tuning;
use crate::model::{
    Character, Desperation, Infamy, RegardEdge, Social, Wealth, Willingness, willingness,
};
use crate::resolve::{apply, resolve};
use crate::ui;

/// Which stage of a beat the player is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Picking a party, with the willingness preview live.
    Assembly,
    /// Reading what the dungeon did.
    Report,
    /// The chain is finished.
    Complete,
}

/// Where the game is.
#[derive(Clone, Debug)]
pub struct Flow {
    /// Which beat of `CHAIN`.
    pub beat: usize,
    /// Which stage of it.
    pub stage: Stage,
    /// Which of the beat's dungeons is selected.
    pub dungeon: usize,
    /// The party being assembled, in roster order.
    pub party: Vec<Entity>,
    /// The last resolution's narration, line by line.
    pub report: Vec<String>,
}
impl Resource for Flow {}

impl Default for Flow {
    fn default() -> Self {
        Self {
            beat: 0,
            stage: Stage::Assembly,
            dungeon: 0,
            party: Vec::new(),
            report: Vec::new(),
        }
    }
}

impl Flow {
    /// The beat being played, if the chain still has one.
    pub fn spec(&self) -> Option<&'static crate::beats::BeatSpec> {
        CHAIN.get(self.beat)
    }

    /// The selected dungeon, if the beat offers one.
    pub fn dungeon(&self) -> Option<&'static Dungeon> {
        self.spec().and_then(|beat| beat.dungeons.get(self.dungeon))
    }
}

/// Which beat a run starts on.
///
/// A simulation input like `Tuning`: the windowed game inserts none and starts
/// at the top of the chain, and `--verify` puts one in before the first tick so
/// a beat can be played on its own. Stamped into the verify report for the same
/// reason the constants are — a run nobody can reproduce is not evidence.
#[derive(Clone, Copy, Debug, Default)]
pub struct StartAt(pub usize);
impl Resource for StartAt {}

/// What the party assembly currently says, arithmetic and all.
///
/// Derived state, recomputed every tick from the same function the send gate
/// uses. Nothing here is a decision; it is the decision, shown.
#[derive(Clone, Debug, Default)]
pub struct Preview {
    /// One entry per selected member, in roster order.
    pub entries: Vec<Willingness>,
    /// Whether the party is the size the dungeon takes.
    pub headcount_ok: bool,
    /// Whether the party satisfies the dungeon's composition predicate.
    pub requirement_ok: bool,
    /// Whether every selected member will actually come.
    pub all_willing: bool,
    /// Whether the send verb is available.
    pub can_send: bool,
    /// Why not, if not — stated in the same numbers the panels show.
    pub blocked: String,
}
impl Resource for Preview {}

/// The one gate: what this party says, and whether it can be sent.
///
/// Called by the preview and by the send verb. Two callers, one answer.
pub fn assess(
    social: &Social,
    tuning: &Tuning,
    party: &[Entity],
    dungeon: Option<&Dungeon>,
) -> Preview {
    let entries: Vec<Willingness> = party
        .iter()
        .map(|member| willingness(social, tuning, *member, party))
        .collect();
    let all_willing = entries.iter().all(Willingness::joins);
    let Some(dungeon) = dungeon else {
        return Preview {
            entries,
            all_willing,
            blocked: "no job selected".to_owned(),
            ..Preview::default()
        };
    };
    let headcount_ok = party.len() == dungeon.headcount;
    let requirement_ok = dungeon.requires.met(social, party);
    let refusing: Vec<&str> = entries
        .iter()
        .filter(|entry| !entry.joins())
        .map(|entry| entry.name)
        .collect();
    let blocked = if !headcount_ok {
        format!(
            "party of {}; {} takes {}",
            party.len(),
            dungeon.name,
            dungeon.headcount
        )
    } else if !requirement_ok {
        format!("{} wants {}", dungeon.name, dungeon.requires.describe())
    } else if !all_willing {
        format!("{} will not come", refusing.join(" and "))
    } else {
        String::new()
    };
    Preview {
        entries,
        headcount_ok,
        requirement_ok,
        all_willing,
        can_send: headcount_ok && requirement_ok && all_willing,
        blocked,
    }
}

/// Put a beat's authored state into the world, replacing whatever was there.
///
/// Read pass then write pass: what exists is collected before anything is
/// despawned, because the query borrows the world it is about to change.
pub fn load_beat(world: &mut World, index: usize) {
    let characters: Vec<Entity> = world
        .query::<&Character>()
        .map(|(entity, _)| entity)
        .collect();
    let edges: Vec<Entity> = world
        .query::<&RegardEdge>()
        .map(|(entity, _)| entity)
        .collect();
    for entity in characters.into_iter().chain(edges) {
        world.despawn(entity);
    }

    if let Some(beat) = CHAIN.get(index) {
        let mut spawned: Vec<(&'static str, Entity)> = Vec::new();
        for (roster_index, spec) in beat.roster.iter().enumerate() {
            let entity = world.spawn();
            world.insert(
                entity,
                Character {
                    name: spec.name,
                    roster_index,
                },
            );
            world.insert(entity, Desperation(spec.desperation));
            world.insert(entity, Infamy(spec.infamy));
            world.insert(entity, Wealth(spec.wealth));
            spawned.push((spec.name, entity));
        }
        let find = |name: &str| {
            spawned
                .iter()
                .find(|(spawned_name, _)| *spawned_name == name)
                .map(|(_, entity)| *entity)
        };
        for EdgeSpec { from, to, value } in beat.edges.iter().copied() {
            if let (Some(from), Some(to)) = (find(from), find(to)) {
                let entity = world.spawn();
                world.insert(entity, RegardEdge { from, to, value });
            }
        }
    }

    let flow = world.resource_mut::<Flow>();
    flow.beat = index;
    flow.stage = if index < CHAIN.len() {
        Stage::Assembly
    } else {
        Stage::Complete
    };
    flow.dungeon = 0;
    flow.party.clear();
    flow.report.clear();
}

/// The pointer, which is the whole of the player's input (DESIGN §7).
pub fn handle_pointer(world: &mut World) {
    let Some((clicked, screen)) = world.find_resource::<Input>().map(|input| {
        let pointer = input.pointer();
        (pointer.just_pressed(PointerButton::Primary), pointer.screen)
    }) else {
        return;
    };
    if !clicked {
        return;
    }
    let at = world.resource::<Camera>().screen_to_world(screen);
    let stage = world.resource::<Flow>().stage;
    match stage {
        Stage::Assembly => click_in_assembly(world, at),
        Stage::Report | Stage::Complete => {
            if ui::continue_button().contains(at) {
                let next = match stage {
                    // The chain loops rather than dead-ending, so a playtest of
                    // the last beat is one click from the first.
                    Stage::Complete => 0,
                    _ => world.resource::<Flow>().beat + 1,
                };
                load_beat(world, next);
            }
        }
    }
}

fn click_in_assembly(world: &mut World, at: Vec2) {
    let social = Social::read(&world.view());
    let flow = world.resource::<Flow>();
    let Some(beat) = flow.spec() else {
        return;
    };
    let mut party = flow.party.clone();
    let mut dungeon = flow.dungeon;
    let mut send = false;

    for (index, member) in social.members.iter().enumerate() {
        if ui::card_rect(index).contains(at) {
            match party.iter().position(|entity| *entity == member.entity) {
                Some(position) => {
                    party.remove(position);
                }
                None => party.push(member.entity),
            }
        }
    }
    // The party is kept in roster order, because that is the order betrayal is
    // evaluated in and a party whose order depended on click order would make
    // the outcome depend on it too.
    party.sort_by_key(|entity| {
        social
            .member(*entity)
            .map_or(usize::MAX, |member| member.roster_index)
    });
    for index in 0..beat.dungeons.len() {
        if ui::dungeon_row_rect(index).contains(at) {
            dungeon = index;
        }
    }
    if ui::send_button().contains(at) {
        send = true;
    }

    let tuning = *world.resource::<Tuning>();
    let selected = beat.dungeons.get(dungeon).copied();
    let ready = assess(&social, &tuning, &party, selected.as_ref());
    {
        let flow = world.resource_mut::<Flow>();
        flow.party = party.clone();
        flow.dungeon = dungeon;
    }
    if send
        && ready.can_send
        && let Some(dungeon) = selected
    {
        let resolution = resolve(&social, &tuning, &dungeon, &party);
        apply(world, &resolution);
        let flow = world.resource_mut::<Flow>();
        flow.report = resolution.lines;
        flow.stage = Stage::Report;
    }
}

/// Recompute the willingness preview, every tick, from the gate itself.
pub fn refresh_preview(world: &mut World) {
    let social = Social::read(&world.view());
    let tuning = *world.resource::<Tuning>();
    let flow = world.resource::<Flow>();
    let party = flow.party.clone();
    let dungeon = flow.dungeon().copied();
    let preview = assess(&social, &tuning, &party, dungeon.as_ref());
    world.insert_resource(preview);
}
