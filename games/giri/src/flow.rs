//! Game flow: which beat, which screen, what is taken, who is in — and the
//! pointer that moves all of it (UI.md §3).
//!
//! An explicit state machine in a resource (DESIGN §13). **Three modes get
//! three screens** (UI.md §1.3): the quest board, the full-screen resolution
//! takeover, and the end of the chain. The log is a drawer over the board and
//! never the primary channel.
//!
//! **The gate, the preview and the door are one function each, called from
//! both phases.** `assess` answers "what does this party say and can it be
//! sent"; `model::admit` answers "may this character be added". The send verb
//! and the info panel call the first; the click handler and the party strip
//! call the second. Nothing recomputes a rule at a draw site, which is why the
//! preview cannot say something the resolution disagrees with (DESIGN
//! invariant; ADR-0039's `World::view` is the mechanism).

use jidousha::prelude::*;

use crate::beats::{CHAIN, Dungeon, EdgeSpec};
use crate::constants::Tuning;
use crate::model::{
    Character, CleanJobs, Desperation, Marks, RegardEdge, Social, Source, Traits, Wealth,
};
use crate::onset::{Card, Onset};
use crate::resolve::{DriftLine, EventCard, apply, resolve};
use crate::tuning::Tuner;
use crate::willing::{Admission, Willingness, admit, willingness};
use crate::{layout, onset, sprites};

/// Which screen the player is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// The quest board: quests, the info panel, the party strip.
    Board,
    /// The resolution takeover, replacing the board entirely.
    Resolution,
    /// The chain is finished.
    Complete,
}

/// How many ticks a bounced click's toast stays up. About two and a half
/// seconds at the engine's fixed sixty, which is the mockup's own 2800ms.
pub const TOAST_TICKS: u64 = 168;

/// A transient message about a click that did not do what it looked like it
/// would (UI.md §4: "a bounced click surfaces the arithmetic in a transient
/// toast and in the log").
#[derive(Clone, Debug)]
pub struct Toast {
    /// What it says.
    pub text: String,
    /// The tick it stops being drawn.
    pub until: u64,
}

/// Where the game is.
#[derive(Clone, Debug)]
pub struct Flow {
    /// Which beat of `CHAIN`.
    pub beat: usize,
    /// Which screen.
    pub stage: Stage,
    /// The quest the player has taken, if any. `None` means no send verb.
    pub taken: Option<usize>,
    /// The quest under the pointer this tick, which *peeks* in the panel.
    pub peek: Option<usize>,
    /// The party being assembled, in roster order.
    pub party: Vec<Entity>,
    /// The last resolution's narration - the log's copy and the assertions'.
    pub report: Vec<String>,
    /// The last resolution's event cards.
    pub events: Vec<EventCard>,
    /// The last resolution's drift ledger.
    pub drift: Vec<DriftLine>,
    /// Which quest the takeover is about.
    pub resolved: Option<usize>,
    /// What the player has taken in cuts, across the session.
    pub gold: i32,
    /// The log, most recent first. Secondary by design (UI.md §3).
    pub log: Vec<String>,
    /// Whether the drawer is open.
    pub log_open: bool,
    /// The transient message, if one is up.
    pub toast: Option<Toast>,
    /// The tuning drawer's state — pending constants and all (`tuning.rs`).
    /// **UI state**: the active constants are the `Tuning` resource.
    pub tuner: Tuner,
    /// This beat's playtest counters (`onset.rs`).
    pub onset: Onset,
}
impl Resource for Flow {}

impl Default for Flow {
    fn default() -> Self {
        Self {
            beat: 0,
            stage: Stage::Board,
            taken: None,
            peek: None,
            party: Vec::new(),
            report: Vec::new(),
            events: Vec::new(),
            drift: Vec::new(),
            resolved: None,
            gold: 0,
            log: Vec::new(),
            log_open: false,
            toast: None,
            tuner: Tuner::default(),
            onset: Onset::default(),
        }
    }
}

impl Flow {
    /// The beat being played, if the chain still has one.
    pub fn spec(&self) -> Option<&'static crate::beats::BeatSpec> {
        CHAIN.get(self.beat)
    }

    /// Which quest the info panel is showing: the peek if there is one,
    /// otherwise the taken one. Moving off a peeked card re-locks (UI.md §3).
    pub fn shown(&self) -> Option<usize> {
        self.peek.or(self.taken)
    }

    /// The quest at `index` of this beat's offers.
    pub fn quest(&self, index: usize) -> Option<&'static Dungeon> {
        self.spec().and_then(|beat| beat.dungeons.get(index))
    }

    /// The taken quest — the only one the send verb will run.
    pub fn taken_quest(&self) -> Option<&'static Dungeon> {
        self.taken.and_then(|index| self.quest(index))
    }

    /// The quest the info panel is drawing.
    pub fn shown_quest(&self) -> Option<&'static Dungeon> {
        self.shown().and_then(|index| self.quest(index))
    }

    /// Put a line at the top of the log.
    pub fn note(&mut self, line: String) {
        self.log.insert(0, format!("b{} - {line}", self.beat + 1));
    }

    /// Raise a toast, and log the same sentence — nothing appears only in the
    /// log, and nothing that matters appears only in a toast (UI.md §3).
    pub fn bounce(&mut self, tick: u64, text: String) {
        self.note(text.clone());
        self.toast = Some(Toast {
            text,
            until: tick + TOAST_TICKS,
        });
    }
}

/// Which beat a run starts on.
///
/// A simulation input like `Tuning`: the windowed game inserts none and starts
/// at the top of the chain, and `--verify` puts one in before the first tick so
/// a beat can be played on its own.
#[derive(Clone, Copy, Debug, Default)]
pub struct StartAt(pub usize);
impl Resource for StartAt {}

/// What the assembly currently says, arithmetic and all.
///
/// Derived state, recomputed every tick from the same functions the send verb
/// and the click handler gate on. Nothing here is a decision; it is the
/// decisions, shown.
#[derive(Clone, Debug, Default)]
pub struct Preview {
    /// One entry per party member, in roster order.
    pub entries: Vec<Willingness>,
    /// One answer per living character *not* in the party, in roster order -
    /// what the door would say if they were clicked right now.
    pub doors: Vec<(Entity, Admission)>,
    /// Whether the party is the size the quest takes.
    pub headcount_ok: bool,
    /// Whether the party satisfies the quest's composition predicate.
    pub requirement_ok: bool,
    /// Whether every member is still willing.
    pub all_willing: bool,
    /// Whether the send verb is available.
    pub can_send: bool,
    /// Why not, if not — the reason the disabled button states (UI.md §3).
    pub blocked: String,
}
impl Resource for Preview {}

impl Preview {
    /// The door's answer for a character, if they are one the door was asked
    /// about.
    pub fn door(&self, who: Entity) -> Option<&Admission> {
        self.doors
            .iter()
            .find(|(entity, _)| *entity == who)
            .map(|(_, admission)| admission)
    }
}

/// The one gate: what this party says, who could still join, and whether it
/// can be sent.
pub fn assess(
    social: &Social,
    tuning: &Tuning,
    party: &[Entity],
    dungeon: Option<&Dungeon>,
) -> Preview {
    let entries: Vec<Willingness> = party
        .iter()
        .map(|member| willingness(social, tuning, *member, party, dungeon))
        .collect();
    let doors: Vec<(Entity, Admission)> = social
        .members
        .iter()
        .filter(|member| member.alive && !party.contains(&member.entity))
        .map(|member| {
            (
                member.entity,
                admit(social, tuning, member.entity, party, dungeon),
            )
        })
        .collect();
    let all_willing = entries.iter().all(Willingness::joins);
    let Some(dungeon) = dungeon else {
        return Preview {
            entries,
            doors,
            all_willing,
            blocked: "take a quest to send them out".to_owned(),
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
    let blocked = if party.len() < dungeon.headcount {
        format!("need {} more", dungeon.headcount - party.len())
    } else if party.len() > dungeon.headcount {
        format!("{} too many", party.len() - dungeon.headcount)
    } else if !requirement_ok {
        dungeon.requires.shortfall().to_owned()
    } else {
        String::new()
    };
    let _ = &refusing;
    Preview {
        entries,
        doors,
        headcount_ok,
        requirement_ok,
        all_willing,
        // **Not gated on `all_willing`, and that is the door rule, not an
        // oversight.** Consent is evaluated at the door only (DESIGN §6):
        // once a member is in they stay until the player removes them or the
        // party is sent, and removing a bonded partner can push a remaining
        // member negative. Gating the send on it would leave that player with a
        // party they assembled legally, cannot send, and can only fix by
        // removing somebody - which is the re-evaluation the rule declines to
        // do, arriving by the back door. The member's own card still says what
        // they now think, in ember; the game does not hide it, it just does not
        // ask them again.
        can_send: headcount_ok && requirement_ok,
        blocked,
    }
}

/// Put a beat's authored state into the world, replacing whatever was there.
///
/// Read pass then write pass: what exists is collected before anything is
/// despawned, because the query borrows the world it is about to change. The
/// log and the player's gold survive a beat boundary; nothing else does.
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
            world.insert(entity, Source(spec.source));
            world.insert(entity, Wealth(spec.wealth));
            world.insert(entity, Traits(spec.traits.to_vec()));
            world.insert(entity, Marks(spec.marks.to_vec()));
            world.insert(entity, CleanJobs(spec.clean_jobs));
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
        Stage::Board
    } else {
        Stage::Complete
    };
    flow.taken = None;
    flow.peek = None;
    flow.party.clear();
    flow.report.clear();
    flow.events.clear();
    flow.drift.clear();
    flow.resolved = None;
    flow.log_open = false;
    flow.toast = None;
    // A restart is a fresh assembly to measure, whether it came from finishing
    // the last beat or from an APPLY. The drawer's own state - open, pending,
    // and a fault nobody has acknowledged - survives it: pending persists until
    // applied or overwritten by a preset (UI.md §12), and `tuning::apply` puts
    // the drawer back up after calling this.
    flow.onset = Onset::default();
    flow.tuner.open = false;
    flow.tuner.hover = None;
}

/// The pointer, which is the whole of the player's input (DESIGN §12).
///
/// Hover as well as click: the info panel's peek is a hover state, so the
/// pointer's *position* is read every tick and not only on the tick it is
/// pressed.
pub fn handle_pointer(world: &mut World) {
    let Some((clicked, screen)) = world.find_resource::<Input>().map(|input| {
        let pointer = input.pointer();
        (pointer.just_pressed(PointerButton::Primary), pointer.screen)
    }) else {
        return;
    };
    let at = world.resource::<Camera>().screen_to_world(screen);
    let tick = world.resource::<Time>().tick;
    let stage = world.resource::<Flow>().stage;

    // A toast is transient by the clock, not by the next click: a player who
    // clicks somewhere harmless should still be able to read why the last click
    // bounced.
    {
        let flow = world.resource_mut::<Flow>();
        if flow.toast.as_ref().is_some_and(|toast| tick >= toast.until) {
            flow.toast = None;
        }
    }

    match stage {
        Stage::Board => board_input(world, at, tick, clicked),
        // The takeover is dismissed by a click anywhere (UI.md §3).
        Stage::Resolution | Stage::Complete => {
            if clicked {
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

fn board_input(world: &mut World, at: Vec2, tick: u64, clicked: bool) {
    let social = Social::read(&world.view());
    let tuning = *world.resource::<Tuning>();
    let offered = world
        .resource::<Flow>()
        .spec()
        .map_or(0, |beat| beat.dungeons.len());

    // The tuning drawer first, and every tick: it covers the board while it is
    // open, so what it does not want is the only thing anything under it gets
    // (`tuning::handle_pointer` says which).
    let taken_by_tuner = crate::tuning::handle_pointer(world, at, tick, clicked);

    // Hover next, every tick, click or no click.
    let over_quest =
        (0..offered.min(layout::QUEST_SLOTS)).find(|index| layout::quest_card(*index).contains(at));
    let over_person = social
        .members
        .iter()
        .position(|member| layout::party_card(member.roster_index).contains(at));
    // What the pointer is on, for the playtest counters. Nothing is on a sheet
    // while the drawer covers the board - the sheet is not visible to look at.
    let on = if taken_by_tuner {
        None
    } else {
        over_quest
            .map(Card::Quest)
            .or(over_person.map(Card::Person))
    };
    {
        let taken = world.resource::<Flow>().taken;
        let flow = world.resource_mut::<Flow>();
        // A taken quest does not peek: the panel is locked to it (UI.md §3). It
        // is still a card the pointer can be on, which is why the look counter
        // above is not derived from the peek.
        flow.peek = over_quest.filter(|index| !taken_by_tuner && Some(*index) != taken);
        flow.onset.look(on);
    }
    if !clicked || taken_by_tuner {
        return;
    }

    // The drawer swallows clicks while it is open: it covers the board, and a
    // click that fell through it would act on a card the player cannot see.
    if world.resource::<Flow>().log_open {
        let flow = world.resource_mut::<Flow>();
        flow.log_open = false;
        return;
    }
    if layout::log_button().contains(at) {
        let flow = world.resource_mut::<Flow>();
        flow.log_open = true;
        // The other drawer gives way, for the reason `tuning::handle_pointer`
        // states: one board, one drawer.
        flow.tuner.open = false;
        return;
    }

    // Take a quest, or release the taken one. Two verbs, one each: while a
    // quest is taken the panel is locked to it and RELEASE is the only way out
    // (UI.md §3), so a click on another card peeks and does not re-take.
    let taken = world.resource::<Flow>().taken;
    if taken.is_some() {
        if layout::release_button().contains(at) {
            let flow = world.resource_mut::<Flow>();
            flow.taken = None;
            flow.note("released the quest".to_owned());
            return;
        }
    } else if let Some(index) =
        (0..offered.min(layout::QUEST_SLOTS)).find(|index| layout::quest_card(*index).contains(at))
    {
        let line = world.resource::<Flow>().quest(index).map(crate::job_line);
        let flow = world.resource_mut::<Flow>();
        flow.taken = Some(index);
        flow.peek = None;
        if let Some(line) = line {
            flow.note(format!("took {line}"));
        }
        return;
    }

    // The party strip: click to add or remove, under the door rule.
    for (index, member) in social.members.iter().enumerate() {
        if !layout::party_card(index).contains(at) {
            continue;
        }
        if !member.alive {
            // Dead characters stay on the roster, grayed and unclickable:
            // memory is a signifier (UI.md §2).
            return;
        }
        // Assembly starts at the first roster interaction, bounced or not: a
        // click that the door refused is still the player having started.
        world.resource_mut::<Flow>().onset.touch(tick);
        let party = world.resource::<Flow>().party.clone();
        // The taken quest is part of the question (DESIGN §6: willingness
        // takes the quest): with nothing taken, no pot pulls yet.
        let job = world.resource::<Flow>().taken_quest().copied();
        if let Some(position) = party.iter().position(|entity| *entity == member.entity) {
            let flow = world.resource_mut::<Flow>();
            flow.party.remove(position);
            flow.note(format!("{} stood down", member.name));
            return;
        }
        let answer = admit(&social, &tuning, member.entity, &party, job.as_ref());
        match answer.bounce(member.name) {
            Some(text) => world.resource_mut::<Flow>().bounce(tick, text),
            None => {
                let mut party = party;
                party.push(member.entity);
                // The party is kept in roster order, because that is the order
                // betrayal is evaluated in and a party whose order depended on
                // click order would make the outcome depend on it too.
                party.sort_by_key(|entity| {
                    social
                        .member(*entity)
                        .map_or(usize::MAX, |member| member.roster_index)
                });
                world.resource_mut::<Flow>().party = party;
            }
        }
        return;
    }

    // The send verb, which exists only while a quest is taken.
    if taken.is_some() && layout::send_button().contains(at) {
        send(world, &social, &tuning, tick);
    }
}

/// Run the taken quest, if the gate allows it, and go to the takeover.
fn send(world: &mut World, social: &Social, tuning: &Tuning, tick: u64) {
    let (party, quest, index) = {
        let flow = world.resource::<Flow>();
        let Some(index) = flow.taken else {
            return;
        };
        let Some(quest) = flow.taken_quest().copied() else {
            return;
        };
        (flow.party.clone(), quest, index)
    };
    let ready = assess(social, tuning, &party, Some(&quest));
    if !ready.can_send {
        return;
    }
    // The playtest counters, before the beat's state moves under them. Logged
    // and printed, and that is the whole of where they go (`onset.rs`).
    let fixed_dt = world.resource::<Time>().fixed_dt;
    let measured = world.resource::<Flow>().onset.line(tick, fixed_dt);
    if onset::playing() {
        println!(
            "[giri] beat {} - {measured}",
            world.resource::<Flow>().beat + 1
        );
    }
    world.resource_mut::<Flow>().note(measured);
    let resolution = resolve(social, tuning, &quest, &party);
    apply(world, &resolution);
    let survivors: Vec<&str> = resolution
        .survivors
        .iter()
        .map(|entity| social.name(*entity))
        .collect();
    let lost = party.len() - resolution.survivors.len();
    let flow = world.resource_mut::<Flow>();
    flow.gold += quest.cut;
    flow.report = resolution.lines;
    flow.events = resolution.events;
    flow.drift = resolution.drift;
    flow.resolved = Some(index);
    flow.stage = Stage::Resolution;
    flow.toast = None;
    flow.note(format!(
        "{} cleared - cut {}g - {} came back{}",
        quest.name,
        quest.cut,
        survivors.join(", "),
        if lost == 0 {
            String::new()
        } else {
            format!(" - {lost} did not")
        }
    ));
}

/// Recompute the preview, every tick, from the gate itself.
pub fn refresh_preview(world: &mut World) {
    let social = Social::read(&world.view());
    let tuning = *world.resource::<Tuning>();
    let flow = world.resource::<Flow>();
    let party = flow.party.clone();
    // The *taken* quest, not the peeked one: the gate is about what would
    // actually be sent, and a peek must not change whether the button is live.
    let quest = flow.taken_quest().copied();
    let preview = assess(&social, &tuning, &party, quest.as_ref());
    world.insert_resource(preview);
}

/// The art store and its handles, inserted before anything draws.
pub fn install_art(world: &mut World) {
    let mut assets = sprites::store();
    let gallery = sprites::Gallery::load(&mut assets);
    world.insert_resource(assets);
    world.insert_resource(gallery);
}
