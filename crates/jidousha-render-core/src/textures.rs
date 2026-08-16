//! Getting texels onto a backend: the two built-in textures, and everything the
//! asset store has finished loading.
//!
//! Key types: `create_builtin_textures`, `upload_ready_textures`.
//! Depends on: `jidousha-assets`, `backend`, `plan`.
//! INVARIANT (renderer.md §5, CONTRACT): the placeholder is an embedded texture,
//! not something a backend draws for itself. Its texels are the constant below,
//! so a missing texture looks identical under wgpu, under ash, and in a golden
//! image — which is what makes "did the art load?" a question a screenshot can
//! answer the same way everywhere.

use jidousha_assets::Assets;

use crate::backend::{PhysicalSize, RenderBackend, TextureDesc};
use crate::plan::TextureTable;

/// The placeholder's side, in texels.
const PLACEHOLDER_SIZE: u32 = 16;

/// How big one check of the placeholder's checkerboard is.
///
/// Four texels into a sixteen-texel square gives a 4×4 checkerboard, which
/// reads as "this is wrong" at any size a sprite is drawn at. Two large checks
/// can pass for art at a glance; sixteen small ones turn to mush when scaled
/// down.
const PLACEHOLDER_CHECK: u32 = 4;

/// Upload the two textures the renderer always has, and name them.
///
/// `TextureId::WHITE` gets a single opaque texel, so a shape carrying only a
/// color goes through the sprite pipeline like everything else — one pipeline,
/// one vertex format (renderer.md §7). Everything not yet uploaded resolves to
/// the checkered magenta placeholder.
///
/// Called once, when a backend is ready. The ids come back inside the table
/// rather than being assumed: a caller that hard-coded 0 and 1 would be right
/// only for as long as nothing else was created first.
pub fn create_builtin_textures(backend: &mut dyn RenderBackend) -> TextureTable {
    let white = backend.create_texture(
        &TextureDesc {
            size: PhysicalSize::new(1, 1),
        },
        &[255, 255, 255, 255],
    );
    let placeholder = backend.create_texture(
        &TextureDesc {
            size: PhysicalSize::new(PLACEHOLDER_SIZE, PLACEHOLDER_SIZE),
        },
        &placeholder_texels(),
    );
    TextureTable::new(white, placeholder)
}

/// The checkered magenta, RGBA8, row-major.
///
/// Magenta against black because no artist picks it: a frame full of this is
/// unambiguously the engine saying something did not load, rather than a
/// colour that might have been intended (renderer.md §5).
#[must_use]
fn placeholder_texels() -> Vec<u8> {
    let mut texels = Vec::with_capacity((PLACEHOLDER_SIZE * PLACEHOLDER_SIZE * 4) as usize);
    for y in 0..PLACEHOLDER_SIZE {
        for x in 0..PLACEHOLDER_SIZE {
            let check = (x / PLACEHOLDER_CHECK + y / PLACEHOLDER_CHECK).is_multiple_of(2);
            texels.extend_from_slice(if check {
                &[255, 0, 255, 255]
            } else {
                &[0, 0, 0, 255]
            });
        }
    }
    texels
}

/// Hand every newly loaded texture to the backend, and record where it landed.
///
/// Called once a frame by the driver, after `Assets::commit` and before the
/// frame's ticks — so a sprite drawn this frame samples art that became ready
/// this frame, rather than showing the placeholder for one frame more than it
/// has to (assets.md §4, §5).
///
/// The texels are **moved** out of the store: the GPU is where they live from
/// here on, and a second copy in memory would have no reader (renderer.md §3
/// forbids simulation from looking at them).
pub fn upload_ready_textures(
    assets: &mut Assets,
    backend: &mut dyn RenderBackend,
    textures: &mut TextureTable,
) {
    for upload in assets.take_uploads() {
        let desc = TextureDesc {
            size: PhysicalSize::new(upload.data.width, upload.data.height),
        };
        let uploaded = backend.create_texture(&desc, &upload.data.rgba);
        textures.register(upload.id, uploaded);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::null::NullBackend;
    use jidousha_assets::{MemorySource, TextureData};
    use jidousha_core::TextureId;

    fn texture(width: u32, height: u32, fill: u8) -> TextureData {
        TextureData {
            width,
            height,
            rgba: vec![fill; (width * height * 4) as usize],
        }
    }

    #[test]
    fn the_placeholder_is_a_checkerboard_of_magenta_and_black() {
        let texels = placeholder_texels();
        assert_eq!(
            texels.len(),
            (PLACEHOLDER_SIZE * PLACEHOLDER_SIZE * 4) as usize
        );
        assert_eq!(&texels[0..4], &[255, 0, 255, 255], "the first check");
        // One check across, in the same row: the other colour.
        let next = (PLACEHOLDER_CHECK * 4) as usize;
        assert_eq!(&texels[next..next + 4], &[0, 0, 0, 255]);
        // And one check down, which must alternate too — a checkerboard that
        // only alternates along X is a set of stripes.
        let below = (PLACEHOLDER_CHECK * PLACEHOLDER_SIZE * 4) as usize;
        assert_eq!(&texels[below..below + 4], &[0, 0, 0, 255]);
    }

    #[test]
    fn the_built_in_textures_are_the_ids_the_table_reports() {
        // The bug this rules out: a caller assuming the built-ins are 0 and 1.
        let mut backend = NullBackend::new();
        let stray = backend.create_texture(
            &TextureDesc {
                size: PhysicalSize::new(1, 1),
            },
            &[0; 4],
        );
        let table = create_builtin_textures(&mut backend);
        assert_ne!(table.resolve(TextureId::WHITE), stray);
        assert_ne!(table.placeholder(), stray);
        assert_eq!(backend.texture_count(), 3);
    }

    #[test]
    fn the_white_texel_and_the_placeholder_go_to_the_backend_they_belong_to() {
        // Swapping the two would be invisible to every other test here — both
        // are `BackendTextureId`s and both are registered — and the result would
        // be untextured shapes drawing a checkerboard and missing art drawing
        // white. Only the texels tell them apart.
        let mut backend = NullBackend::new();
        let table = create_builtin_textures(&mut backend);

        let Some((desc, texels)) = backend.uploaded(table.resolve(TextureId::WHITE)) else {
            panic!("the white texture was created");
        };
        assert_eq!(desc.size, PhysicalSize::new(1, 1));
        assert_eq!(texels, [255, 255, 255, 255], "one opaque white texel");

        let Some((desc, texels)) = backend.uploaded(table.placeholder()) else {
            panic!("the placeholder was created");
        };
        assert_eq!(
            desc.size,
            PhysicalSize::new(PLACEHOLDER_SIZE, PLACEHOLDER_SIZE)
        );
        assert_eq!(texels, placeholder_texels(), "bit-identical, per §5");
    }

    #[test]
    fn a_texture_reaches_the_backend_with_the_size_and_texels_it_was_decoded_at() {
        // A width and height read from the wrong place, or texels handed over
        // by the wrong slot, both produce a texture that uploads without
        // complaint and samples as garbage.
        let mut source = MemorySource::new();
        source.insert_texture("hero.png", texture(4, 2, 9));
        let mut assets = Assets::new(source);
        let hero = assets.load_texture("hero.png");

        let mut backend = NullBackend::new();
        let mut table = create_builtin_textures(&mut backend);
        assets.commit(1);
        upload_ready_textures(&mut assets, &mut backend, &mut table);

        let Some((desc, texels)) = backend.uploaded(table.resolve(hero.texture_id())) else {
            panic!("hero was uploaded");
        };
        assert_eq!(desc.size, PhysicalSize::new(4, 2));
        assert_eq!(texels, vec![9u8; 4 * 2 * 4]);
    }

    #[test]
    fn a_texture_that_finished_loading_reaches_the_backend() {
        let mut source = MemorySource::new();
        source.insert_texture("hero.png", texture(2, 2, 7));
        let mut assets = Assets::new(source);
        let hero = assets.load_texture("hero.png");

        let mut backend = NullBackend::new();
        let mut table = create_builtin_textures(&mut backend);
        assert_eq!(
            table.resolve(hero.texture_id()),
            table.placeholder(),
            "nothing has loaded yet"
        );

        assets.commit(1);
        upload_ready_textures(&mut assets, &mut backend, &mut table);
        assert_ne!(
            table.resolve(hero.texture_id()),
            table.placeholder(),
            "it has now"
        );
        assert!(table.is_ready(hero.texture_id()));
    }

    #[test]
    fn a_texture_is_uploaded_once_however_often_the_frame_asks() {
        // The loop runs every frame; a store that kept handing the same texels
        // back would upload the same art sixty times a second.
        let mut source = MemorySource::new();
        source.insert_texture("hero.png", texture(2, 2, 7));
        let mut assets = Assets::new(source);
        assets.load_texture("hero.png");

        let mut backend = NullBackend::new();
        let mut table = create_builtin_textures(&mut backend);
        let built_in = backend.texture_count();

        assets.commit(1);
        for tick in 2..6 {
            upload_ready_textures(&mut assets, &mut backend, &mut table);
            assets.commit(tick);
        }
        assert_eq!(backend.texture_count(), built_in + 1);
    }

    #[test]
    fn textures_upload_in_the_order_they_committed() {
        // Upload order decides backend ids, and a golden image taken on one run
        // has to match the next. Commit order is on the timeline; a walk of the
        // asset table is not.
        let mut source = MemorySource::new();
        for path in ["a.png", "b.png", "c.png"] {
            source.insert_texture(path, texture(1, 1, 1));
        }
        source.complete_at("c.png", 1);
        source.complete_at("a.png", 2);
        source.complete_at("b.png", 3);
        let mut assets = Assets::new(source);
        let (a, b, c) = (
            assets.load_texture("a.png"),
            assets.load_texture("b.png"),
            assets.load_texture("c.png"),
        );

        let mut backend = NullBackend::new();
        let mut table = create_builtin_textures(&mut backend);
        for tick in 1..=3 {
            assets.commit(tick);
            upload_ready_textures(&mut assets, &mut backend, &mut table);
        }
        let ids = [c, a, b].map(|handle| table.resolve(handle.texture_id()).0);
        let mut sorted = ids;
        sorted.sort_unstable();
        assert_eq!(ids, sorted, "c committed first, so c uploaded first");
    }

    #[test]
    fn a_texture_unloaded_before_it_was_uploaded_never_reaches_the_gpu() {
        // The game said it was finished with it. Uploading it anyway would put
        // art on the GPU that nothing can draw and nothing will free.
        let mut source = MemorySource::new();
        source.insert_texture("hero.png", texture(2, 2, 7));
        let mut assets = Assets::new(source);
        let hero = assets.load_texture("hero.png");

        let mut backend = NullBackend::new();
        let mut table = create_builtin_textures(&mut backend);
        let built_in = backend.texture_count();

        assets.commit(1);
        assets.unload(hero);
        upload_ready_textures(&mut assets, &mut backend, &mut table);
        assert_eq!(backend.texture_count(), built_in, "nothing was uploaded");
    }

    #[test]
    fn a_texture_queued_before_its_slot_was_recycled_does_not_claim_the_new_one() {
        // What the generation on a queued id is actually for, which is *not*
        // plain unload — an unloaded slot has no entry, so it is skipped anyway.
        // This is the case where the slot came back: without the generation
        // check the stale id finds the *new* entry, takes its texels, and
        // registers them under the old texture's name — leaving the new handle
        // resolving to the placeholder for the rest of the run.
        let mut source = MemorySource::new();
        source.insert_texture("first.png", texture(1, 1, 1));
        source.insert_texture("second.png", texture(1, 1, 2));
        source.complete_at("second.png", 2);
        let mut assets = Assets::new(source);

        let first = assets.load_texture("first.png");
        assets.commit(1);
        // Ready and queued, and then thrown away before anything uploaded it.
        assets.unload(first);
        // Slots come back most-recently-freed first, so this takes the same
        // index with a new generation (assets.md §1).
        let second = assets.load_texture("second.png");
        assets.commit(2);

        let mut backend = NullBackend::new();
        let mut table = create_builtin_textures(&mut backend);
        upload_ready_textures(&mut assets, &mut backend, &mut table);

        assert!(
            table.is_ready(second.texture_id()),
            "the new texture is registered under its own id"
        );
        let Some((_, texels)) = backend.uploaded(table.resolve(second.texture_id())) else {
            panic!("the second texture was uploaded");
        };
        assert_eq!(texels[0], 2, "its own texels, not the dead handle's");
    }

    #[test]
    fn a_texture_that_failed_to_load_keeps_drawing_the_placeholder() {
        // renderer.md §5's whole policy, from the outside: no upload happens, so
        // the id stays unregistered, so it resolves to the placeholder. There is
        // no code path that could get this wrong.
        // An empty source is what a missing file looks like.
        let mut assets = Assets::new(MemorySource::new());
        let missing = assets.load_texture("nowhere.png");

        let mut backend = NullBackend::new();
        let mut table = create_builtin_textures(&mut backend);
        assets.commit(1);
        upload_ready_textures(&mut assets, &mut backend, &mut table);
        assert_eq!(table.resolve(missing.texture_id()), table.placeholder());
    }

    #[test]
    fn bytes_are_not_mistaken_for_textures() {
        // `load_bytes` fills a different table, and its payload must never be
        // queued for upload — the backend would be handed a font or a level file
        // and told it was RGBA.
        let mut source = MemorySource::new();
        source.insert("level.dat", vec![1, 2, 3, 4]);
        let mut assets = Assets::new(source);
        assets.load_bytes("level.dat");

        let mut backend = NullBackend::new();
        let mut table = create_builtin_textures(&mut backend);
        let built_in = backend.texture_count();
        assets.commit(1);
        upload_ready_textures(&mut assets, &mut backend, &mut table);
        assert_eq!(backend.texture_count(), built_in);
    }
}
