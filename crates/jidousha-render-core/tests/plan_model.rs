//! Sorting and batching against a naive reference, under thousands of random
//! submission streams (renderer.md §2, ADR-0006's house style).
//!
//! The real planner sorts an index array and merges batches as it walks it. The
//! model below does the obvious slow thing instead: build tuples, sort them,
//! then group equal neighbours. When they disagree, the model is the one that
//! is easy to read.

use jidousha_core::math::Vec2;
use jidousha_core::{Color, Depth, Quad, TextureId};
use jidousha_render_core::{BackendTextureId, Camera, TextureTable, plan_frame};

const WHITE: BackendTextureId = BackendTextureId(0);
const PLACEHOLDER: BackendTextureId = BackendTextureId(1);

/// How many independent streams to run, and how long each one is.
const STREAMS: u64 = 2000;
const STREAM_LENGTH: usize = 40;

/// The generator's own RNG — SplitMix64, short enough to read, and not the
/// engine's, so a broken engine RNG cannot quieten this test.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 32) as u32
    }

    fn below(&mut self, limit: u32) -> u32 {
        self.next_u32() % limit
    }
}

/// Build a stream of quads.
///
/// Small ranges on purpose: three layers, four z values, and five textures make
/// ties and texture runs common, which is where sorting and batching can go
/// wrong. Each quad's first corner carries its submission index, so the order
/// can be read back out of the plan.
fn generate(seed: u64, length: usize) -> (Vec<Quad>, TextureTable) {
    let mut rng = Rng::new(seed);
    let mut textures = TextureTable::new(WHITE, PLACEHOLDER);
    // Some textures uploaded, some not: the not-ready ones collapse onto the
    // placeholder and change how things batch.
    for id in 0..5u64 {
        if rng.below(2) == 0 {
            textures.register(
                TextureId::from_bits(id + 1),
                BackendTextureId(u32::try_from(id).unwrap_or(0) + 2),
            );
        }
    }

    let quads = (0..length)
        .map(|index| Quad {
            corners: [
                Vec2::new(index as f32, 0.0),
                Vec2::new(index as f32 + 1.0, 0.0),
                Vec2::new(index as f32 + 1.0, 1.0),
                Vec2::new(index as f32, 1.0),
            ],
            uvs: [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y],
            tint: Color::WHITE,
            texture: TextureId::from_bits(u64::from(rng.below(5)) + 1),
            depth: Depth {
                layer: rng.below(3) as i16 - 1,
                z: rng.below(4) as f32,
            },
        })
        .collect();
    (quads, textures)
}

/// What the plan should be: a list of (texture, submission indices), in order.
fn model(quads: &[Quad], textures: &TextureTable) -> Vec<(BackendTextureId, Vec<usize>)> {
    let mut sorted: Vec<(i16, f32, usize)> = quads
        .iter()
        .enumerate()
        .map(|(index, quad)| (quad.depth.layer, quad.depth.z, index))
        .collect();
    // Sort by layer, then z, then submission order — stated as a whole-tuple
    // comparison rather than relying on the sort being stable.
    sorted.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then(left.1.total_cmp(&right.1))
            .then(left.2.cmp(&right.2))
    });

    let mut batches: Vec<(BackendTextureId, Vec<usize>)> = Vec::new();
    for (_, _, index) in sorted {
        let texture = textures.resolve(quads[index].texture);
        match batches.last_mut() {
            Some((last, members)) if *last == texture => members.push(index),
            _ => batches.push((texture, vec![index])),
        }
    }
    batches
}

/// Read the plan back as the same shape the model produces.
fn actual(quads: &[Quad], textures: &TextureTable) -> Vec<(BackendTextureId, Vec<usize>)> {
    let plan = plan_frame(&Camera::default(), quads, textures);
    plan.batches
        .iter()
        .map(|batch| {
            let members = batch
                .vertices
                .chunks_exact(6)
                // The first corner's x is the submission index it was built with.
                .map(|chunk| chunk[0].position.x as usize)
                .collect();
            (batch.texture, members)
        })
        .collect()
}

#[test]
fn the_planner_matches_the_reference_model_under_random_submission_streams() {
    for seed in 0..STREAMS {
        let (quads, textures) = generate(seed, STREAM_LENGTH);
        assert_eq!(
            actual(&quads, &textures),
            model(&quads, &textures),
            "seed {seed}"
        );
    }
}

#[test]
fn planning_the_same_stream_twice_gives_the_same_plan() {
    for seed in 0..200 {
        let (quads, textures) = generate(seed, STREAM_LENGTH);
        let first = plan_frame(&Camera::default(), &quads, &textures);
        let second = plan_frame(&Camera::default(), &quads, &textures);
        assert_eq!(first, second, "seed {seed}");
    }
}

#[test]
fn every_submitted_quad_reaches_the_plan_exactly_once() {
    // Batching merges draw calls, never quads: a sprite that vanished into a
    // batch boundary would be invisible and blameless.
    for seed in 0..STREAMS {
        let (quads, textures) = generate(seed, STREAM_LENGTH);
        let mut seen: Vec<usize> = actual(&quads, &textures)
            .into_iter()
            .flat_map(|(_, members)| members)
            .collect();
        seen.sort_unstable();
        assert_eq!(seen, (0..quads.len()).collect::<Vec<_>>(), "seed {seed}");
    }
}

#[test]
fn the_generated_streams_reach_the_cases_worth_testing() {
    // Guards the generator: a stream with no depth ties, no texture runs, and
    // nothing waiting on a texture would compare only the easy paths.
    let mut ties = 0;
    let mut merged_batches = 0;
    let mut placeholder_batches = 0;

    for seed in 0..STREAMS {
        let (quads, textures) = generate(seed, STREAM_LENGTH);
        for (index, quad) in quads.iter().enumerate() {
            if quads[..index].iter().any(|other| other.depth == quad.depth) {
                ties += 1;
            }
        }
        for (texture, members) in actual(&quads, &textures) {
            if members.len() > 1 {
                merged_batches += 1;
            }
            if texture == PLACEHOLDER {
                placeholder_batches += 1;
            }
        }
    }

    assert!(ties > 0, "no two quads ever shared a depth");
    assert!(merged_batches > 0, "no batch ever held more than one quad");
    assert!(placeholder_batches > 0, "nothing ever waited on a texture");
}
