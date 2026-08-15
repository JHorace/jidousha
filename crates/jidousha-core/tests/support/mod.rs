//! The naive reference world the real one is checked against, plus the
//! deterministic operation-sequence generator that drives the comparison.
//!
//! Key types: `ReferenceWorld`, `Op`, `Rng`.
//! Depends on: `jidousha_core`'s public API only.
//! INVARIANT: the reference implementation is written to be *obviously* right,
//! never efficient — a `Vec` of slots holding a `BTreeMap` of components. When
//! the two disagree, the reference is the one that is easy to read (ADR-0006).
//!
//! This model is load-bearing beyond M1: M2's archetype storage and M3's
//! commands are checked against the same reference semantics.

use std::collections::BTreeMap;

use jidousha_core::Component;

/// A component carrying a value, so replacement is observable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position(pub i32);
impl Component for Position {}

/// A second valued component, so component types are observably independent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Velocity(pub i32);
impl Component for Velocity {}

/// A zero-sized tag component — the idiomatic marker case (core.md §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frozen;
impl Component for Frozen {}

/// Which component type an operation names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComponentKind {
    Position,
    Velocity,
    Frozen,
}

impl ComponentKind {
    /// Every kind the model knows, in a fixed order — state comparison walks
    /// this, so it must never depend on run order.
    pub const ALL: [ComponentKind; 3] = [
        ComponentKind::Position,
        ComponentKind::Velocity,
        ComponentKind::Frozen,
    ];
}

/// A component value, type-erased the naive way: one variant per kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComponentValue {
    Position(i32),
    Velocity(i32),
    Frozen,
}

impl ComponentValue {
    pub fn kind(self) -> ComponentKind {
        match self {
            ComponentValue::Position(_) => ComponentKind::Position,
            ComponentValue::Velocity(_) => ComponentKind::Velocity,
            ComponentValue::Frozen => ComponentKind::Frozen,
        }
    }
}

/// The model's entity handle. Compared against the real one through the debug
/// format, which is the only thing `Entity` promises about its innards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferenceEntity {
    index: u32,
    generation: u32,
}

impl ReferenceEntity {
    /// The handle as `Entity`'s debug format spells it: `Entity(17 v3)`.
    pub fn debug(self) -> String {
        format!("Entity({} v{})", self.index, self.generation)
    }
}

#[derive(Clone, Debug)]
struct ReferenceSlot {
    generation: u32,
    alive: bool,
    components: BTreeMap<ComponentKind, ComponentValue>,
}

/// A world implemented the slowest, most obvious way there is.
pub struct ReferenceWorld {
    slots: Vec<ReferenceSlot>,
    free: Vec<u32>,
}

impl ReferenceWorld {
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    pub fn spawn(&mut self) -> ReferenceEntity {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.alive = true;
            slot.components.clear();
            return ReferenceEntity {
                index,
                generation: slot.generation,
            };
        }
        self.slots.push(ReferenceSlot {
            generation: 1,
            alive: true,
            components: BTreeMap::new(),
        });
        ReferenceEntity {
            index: (self.slots.len() - 1) as u32,
            generation: 1,
        }
    }

    /// `Ok(())` when the entity was alive, mirroring `World::try_despawn`.
    pub fn try_despawn(&mut self, entity: ReferenceEntity) -> Result<(), ()> {
        if !self.is_alive(entity) {
            return Err(());
        }
        let slot = &mut self.slots[entity.index as usize];
        slot.alive = false;
        slot.generation += 1;
        slot.components.clear();
        self.free.push(entity.index);
        Ok(())
    }

    pub fn try_insert(&mut self, entity: ReferenceEntity, value: ComponentValue) -> Result<(), ()> {
        if !self.is_alive(entity) {
            return Err(());
        }
        self.slots[entity.index as usize]
            .components
            .insert(value.kind(), value);
        Ok(())
    }

    pub fn try_remove(&mut self, entity: ReferenceEntity, kind: ComponentKind) -> Result<(), ()> {
        if !self.is_alive(entity) {
            return Err(());
        }
        self.slots[entity.index as usize].components.remove(&kind);
        Ok(())
    }

    pub fn is_alive(&self, entity: ReferenceEntity) -> bool {
        match self.slots.get(entity.index as usize) {
            Some(slot) => slot.alive && slot.generation == entity.generation,
            None => false,
        }
    }

    pub fn find(&self, entity: ReferenceEntity, kind: ComponentKind) -> Option<ComponentValue> {
        if !self.is_alive(entity) {
            return None;
        }
        self.slots[entity.index as usize]
            .components
            .get(&kind)
            .copied()
    }

    pub fn entity_count(&self) -> usize {
        self.slots.iter().filter(|slot| slot.alive).count()
    }

    /// Every live entity carrying all of `yielded` and `required` and none of
    /// `excluded`, with the values of the `yielded` kinds in the order asked for.
    ///
    /// `required` is what a `With<T>` filter contributes: the component must be
    /// present, but a filter yields nothing, so its value is not part of the
    /// answer.
    ///
    /// The naive answer to what a query should have found.
    pub fn matching(
        &self,
        yielded: &[ComponentKind],
        required: &[ComponentKind],
        excluded: &[ComponentKind],
    ) -> Vec<(String, Vec<ComponentValue>)> {
        let mut found = Vec::new();
        for (index, slot) in self.slots.iter().enumerate() {
            if !slot.alive {
                continue;
            }
            if excluded
                .iter()
                .any(|kind| slot.components.contains_key(kind))
            {
                continue;
            }
            if !required
                .iter()
                .all(|kind| slot.components.contains_key(kind))
            {
                continue;
            }
            let mut values = Vec::new();
            for kind in yielded {
                match slot.components.get(kind) {
                    Some(value) => values.push(*value),
                    None => break,
                }
            }
            if values.len() == yielded.len() {
                let entity = ReferenceEntity {
                    index: index as u32,
                    generation: slot.generation,
                };
                found.push((entity.debug(), values));
            }
        }
        found.sort();
        found
    }

    /// Add one to every live `Position`, mirroring a `query_mut` pass.
    pub fn bump_positions(&mut self) {
        for slot in &mut self.slots {
            if !slot.alive {
                continue;
            }
            if let Some(ComponentValue::Position(value)) =
                slot.components.get_mut(&ComponentKind::Position)
            {
                *value += 1;
            }
        }
    }
}

/// One operation in a generated sequence. Entity references are indices into
/// the handles created so far, so sequences replay identically in both worlds.
#[derive(Clone, Copy, Debug)]
pub enum Op {
    Spawn,
    Despawn(usize),
    Insert(usize, ComponentValue),
    Remove(usize, ComponentKind),
    /// Add one to every `Position` in the world, through a mutable query.
    BumpPositions,
}

/// splitmix64 — enough randomness for op sequences, and reproducible from a
/// seed on every platform.
///
/// The engine's own seeded `Rng` resource arrives in M3 (core.md §6); this one
/// exists so the model tests need no dependency and no engine state.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 32) as u32
    }

    pub fn below(&mut self, limit: u32) -> u32 {
        self.next_u32() % limit
    }
}

/// Build one operation sequence. Spawns are weighted highest so the world
/// grows, but despawns are frequent enough that slot reuse and swap-removes
/// dominate the middle of a long sequence.
pub fn generate(seed: u64, length: usize) -> Vec<Op> {
    let mut rng = Rng::new(seed);
    let mut ops = Vec::with_capacity(length);
    let mut handles = 0usize;
    for _ in 0..length {
        if handles == 0 {
            ops.push(Op::Spawn);
            handles += 1;
            continue;
        }
        let target = rng.below(handles as u32) as usize;
        let op = match rng.below(100) {
            0..=32 => {
                handles += 1;
                Op::Spawn
            }
            33..=57 => Op::Insert(target, random_value(&mut rng)),
            58..=77 => Op::Despawn(target),
            78..=91 => Op::Remove(target, random_kind(&mut rng)),
            _ => Op::BumpPositions,
        };
        ops.push(op);
    }
    ops
}

fn random_value(rng: &mut Rng) -> ComponentValue {
    match random_kind(rng) {
        ComponentKind::Position => ComponentValue::Position(rng.below(1000) as i32),
        ComponentKind::Velocity => ComponentValue::Velocity(rng.below(1000) as i32),
        ComponentKind::Frozen => ComponentValue::Frozen,
    }
}

fn random_kind(rng: &mut Rng) -> ComponentKind {
    ComponentKind::ALL[rng.below(ComponentKind::ALL.len() as u32) as usize]
}
