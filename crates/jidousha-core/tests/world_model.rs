//! The M1 exit criterion: the real world and a naive reference model agree
//! under thousands of random operation sequences (core.md §11, ADR-0006).
//!
//! A failure prints the seed and the shortest failing prefix of the sequence,
//! so any mismatch is reproducible by re-running that one seed.

mod support;

use jidousha_core::{Entity, World};
use support::{
    ComponentKind, ComponentValue, Frozen, Op, Position, ReferenceEntity, ReferenceWorld, Velocity,
    generate,
};

/// How many independent sequences to run, and how long each one is.
const SEQUENCES: u64 = 2000;
const SEQUENCE_LENGTH: usize = 150;

/// Both worlds, driven in lockstep, compared after every operation.
struct Pair {
    world: World,
    model: ReferenceWorld,
    handles: Vec<(Entity, ReferenceEntity)>,
    /// Operations that named a dead entity — the `try_*` error paths.
    dead_targets: usize,
}

impl Pair {
    fn new() -> Self {
        Self {
            world: World::new(),
            model: ReferenceWorld::new(),
            handles: Vec::new(),
            dead_targets: 0,
        }
    }

    /// Apply one operation to both worlds, returning a description of the
    /// first disagreement it produced.
    fn apply(&mut self, op: Op) -> Result<(), String> {
        match op {
            Op::Spawn => {
                let entity = self.world.spawn();
                let reference = self.model.spawn();
                if format!("{entity:?}") != reference.debug() {
                    return Err(format!(
                        "spawn handed out {entity:?}, model expected {}",
                        reference.debug()
                    ));
                }
                self.handles.push((entity, reference));
            }
            Op::Despawn(target) => {
                let (entity, reference) = self.handle(target);
                let got = self.world.try_despawn(entity).is_ok();
                let want = self.model.try_despawn(reference).is_ok();
                if got != want {
                    return Err(format!(
                        "try_despawn({entity:?}) returned ok={got}, model expected ok={want}"
                    ));
                }
                if !want {
                    self.dead_targets += 1;
                }
            }
            Op::Insert(target, value) => {
                let (entity, reference) = self.handle(target);
                let got = match value {
                    ComponentValue::Position(v) => self.world.try_insert(entity, Position(v)),
                    ComponentValue::Velocity(v) => self.world.try_insert(entity, Velocity(v)),
                    ComponentValue::Frozen => self.world.try_insert(entity, Frozen),
                }
                .is_ok();
                let want = self.model.try_insert(reference, value).is_ok();
                if got != want {
                    return Err(format!(
                        "try_insert({entity:?}, {value:?}) returned ok={got}, model expected ok={want}"
                    ));
                }
                if !want {
                    self.dead_targets += 1;
                }
            }
            Op::Remove(target, kind) => {
                let (entity, reference) = self.handle(target);
                let got = match kind {
                    ComponentKind::Position => self.world.try_remove::<Position>(entity),
                    ComponentKind::Velocity => self.world.try_remove::<Velocity>(entity),
                    ComponentKind::Frozen => self.world.try_remove::<Frozen>(entity),
                }
                .is_ok();
                let want = self.model.try_remove(reference, kind).is_ok();
                if got != want {
                    return Err(format!(
                        "try_remove({entity:?}, {kind:?}) returned ok={got}, model expected ok={want}"
                    ));
                }
                if !want {
                    self.dead_targets += 1;
                }
            }
        }
        self.compare_state()
    }

    /// Compare everything observable: the live count, and every handle ever
    /// created — alive or not — with each of its components.
    fn compare_state(&self) -> Result<(), String> {
        if self.world.entity_count() != self.model.entity_count() {
            return Err(format!(
                "entity_count is {}, model says {}",
                self.world.entity_count(),
                self.model.entity_count()
            ));
        }
        for (entity, reference) in &self.handles {
            let (entity, reference) = (*entity, *reference);
            if self.world.is_alive(entity) != self.model.is_alive(reference) {
                return Err(format!(
                    "is_alive({entity:?}) is {}, model says {}",
                    self.world.is_alive(entity),
                    self.model.is_alive(reference)
                ));
            }
            for kind in ComponentKind::ALL {
                let got = self.component(entity, kind);
                let want = self.model.find(reference, kind);
                if got != want {
                    return Err(format!(
                        "{entity:?} has {kind:?} = {got:?}, model says {want:?}"
                    ));
                }
            }
        }
        Ok(())
    }

    fn component(&self, entity: Entity, kind: ComponentKind) -> Option<ComponentValue> {
        match kind {
            ComponentKind::Position => self
                .world
                .find_component::<Position>(entity)
                .map(|position| ComponentValue::Position(position.0)),
            ComponentKind::Velocity => self
                .world
                .find_component::<Velocity>(entity)
                .map(|velocity| ComponentValue::Velocity(velocity.0)),
            ComponentKind::Frozen => self
                .world
                .find_component::<Frozen>(entity)
                .map(|_| ComponentValue::Frozen),
        }
    }

    /// Operations name handles by position; the generator only ever produces
    /// positions that exist by the time the operation runs.
    fn handle(&self, target: usize) -> (Entity, ReferenceEntity) {
        self.handles[target % self.handles.len()]
    }
}

/// Run a sequence, returning how many operations named a dead entity.
fn run(ops: &[Op]) -> Result<usize, String> {
    let mut pair = Pair::new();
    for (step, op) in ops.iter().enumerate() {
        pair.apply(*op)
            .map_err(|mismatch| format!("step {step} ({op:?}): {mismatch}"))?;
    }
    Ok(pair.dead_targets)
}

/// The shortest prefix that still fails — a poor agent's shrinker, and enough
/// to turn a 100-operation sequence into something readable.
fn shortest_failing_prefix(ops: &[Op]) -> &[Op] {
    for length in 1..=ops.len() {
        if run(&ops[..length]).is_err() {
            return &ops[..length];
        }
    }
    ops
}

#[test]
fn the_world_matches_the_reference_model_under_random_operation_sequences() {
    for seed in 0..SEQUENCES {
        let ops = generate(seed, SEQUENCE_LENGTH);
        if let Err(mismatch) = run(&ops) {
            let prefix = shortest_failing_prefix(&ops);
            panic!(
                "seed {seed}: {mismatch}\nshortest failing prefix ({} ops):\n{prefix:#?}",
                prefix.len()
            );
        }
    }
}

/// Guards the generator, not the world: a sequence that never names a dead
/// entity would compare only the happy paths and still pass.
#[test]
fn the_generated_sequences_exercise_operations_on_dead_entities() {
    let dead_targets: usize = (0..SEQUENCES)
        .map(|seed| match run(&generate(seed, SEQUENCE_LENGTH)) {
            Ok(count) => count,
            Err(mismatch) => panic!("seed {seed}: {mismatch}"),
        })
        .sum();
    assert!(
        dead_targets > SEQUENCES as usize,
        "only {dead_targets} operations named a dead entity across {SEQUENCES} sequences"
    );
}

#[test]
fn the_same_operation_sequence_produces_the_same_handles_every_run() {
    let ops = generate(7, SEQUENCE_LENGTH);
    let transcript = |()| -> Vec<String> {
        let mut world = World::new();
        let mut handles: Vec<Entity> = Vec::new();
        let mut spawned = Vec::new();
        for op in &ops {
            match *op {
                Op::Spawn => {
                    let entity = world.spawn();
                    spawned.push(format!("{entity:?}"));
                    handles.push(entity);
                }
                Op::Despawn(target) => {
                    let entity = handles[target % handles.len()];
                    let _ = world.try_despawn(entity);
                }
                Op::Insert(target, value) => {
                    let entity = handles[target % handles.len()];
                    let _ = match value {
                        ComponentValue::Position(v) => world.try_insert(entity, Position(v)),
                        ComponentValue::Velocity(v) => world.try_insert(entity, Velocity(v)),
                        ComponentValue::Frozen => world.try_insert(entity, Frozen),
                    };
                }
                Op::Remove(target, kind) => {
                    let entity = handles[target % handles.len()];
                    let _ = match kind {
                        ComponentKind::Position => world.try_remove::<Position>(entity),
                        ComponentKind::Velocity => world.try_remove::<Velocity>(entity),
                        ComponentKind::Frozen => world.try_remove::<Frozen>(entity),
                    };
                }
            }
        }
        spawned
    };
    assert_eq!(transcript(()), transcript(()));
}

#[test]
fn slot_reuse_is_last_freed_first_across_a_whole_sequence() {
    let mut world = World::new();
    let entities: Vec<Entity> = (0..4).map(|_| world.spawn()).collect();
    for entity in &entities {
        world.despawn(*entity);
    }
    let reused: Vec<String> = (0..4).map(|_| format!("{:?}", world.spawn())).collect();
    assert_eq!(
        reused,
        [
            "Entity(3 v2)",
            "Entity(2 v2)",
            "Entity(1 v2)",
            "Entity(0 v2)"
        ]
    );
}
