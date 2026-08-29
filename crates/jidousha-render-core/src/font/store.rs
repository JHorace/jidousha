//! Where loaded faces live, and how their atlases reach the GPU.
//!
//! Key types: `Fonts`, `upload_text_atlases`, `atlas_texture`.
//! Depends on: `jidousha-core`, `backend`, `plan`, `ttf`.
//! INVARIANT: an atlas is uploaded through `RenderBackend::create_texture` and
//! registered in the same `TextureTable` a loaded sprite goes into, so text in a
//! real face is, from the draw path's side, a sprite sampling a texture nobody
//! had to load (renderer.md §5, §6). Nothing here knows what a quad is for.

use jidousha_core::{Quad, TextureId};

use crate::backend::{RenderBackend, TextureDesc};
use crate::plan::TextureTable;
use jidousha_core::PhysicalSize;

use super::ttf::{self, FontError, MAX_PX, TtfFace};
use super::{Face, Kind};

/// The first texture id a loaded face's atlas can take.
///
/// Zero is `TextureId::WHITE` and one is the built-in font, so the reserved
/// range below `1 << 32` starts here for everything else.
const ATLAS_BASE: u64 = 2;

/// How many ids each face reserves — one per raster size, with room above
/// [`MAX_PX`] so the encoding does not have to move if the cap does.
const IDS_PER_FACE: u64 = 128;

/// Which texture a face's atlas at `px` texels to the line is registered under.
///
/// Arithmetic rather than allocation, and that is the point: the draw path
/// works out a glyph's texture id from the face and the size alone, with no
/// store to consult and nothing to mutate, so a quad can name an atlas that has
/// not been rasterized yet. The atlas catches up in [`upload_text_atlases`],
/// and until it does the id resolves to the checkered placeholder like any
/// texture that is not there — which is the policy renderer.md §5 already sets
/// for art in flight.
pub(super) fn atlas_texture(face_id: u32, px: u32) -> TextureId {
    debug_assert!(px as u64 <= IDS_PER_FACE, "a raster size outside the range");
    TextureId::from_bits(ATLAS_BASE + face_id as u64 * IDS_PER_FACE + px as u64)
}

/// Which face and raster size an atlas id names, if it names one at all.
fn atlas_of(id: TextureId) -> Option<(u32, u32)> {
    let bits = id.bits();
    if bits < ATLAS_BASE || bits >= 1 << 32 {
        return None;
    }
    let offset = bits - ATLAS_BASE;
    Some((
        (offset / IDS_PER_FACE) as u32,
        (offset % IDS_PER_FACE) as u32,
    ))
}

/// The parsed face behind a [`Face`], if it has one.
fn ttf_of(face: &Face) -> Option<&'static TtfFace> {
    match face.0 {
        Kind::BuiltIn => None,
        Kind::Ttf(face) => Some(face),
    }
}

/// Every typeface a game has loaded.
///
/// A world resource, like [`Assets`](jidousha_assets::Assets), and used the same
/// way: a game creates a face once from bytes it loaded, keeps the [`Face`], and
/// puts it in the styles it draws with.
///
/// ```no_run
/// # use jidousha_render_core::{Face, Fonts, TextStyle};
/// # fn example(bytes: &[u8]) -> Result<(), Box<dyn core::error::Error>> {
/// let mut fonts = Fonts::new();
/// let body = fonts.try_create_face("Fira Sans", bytes)?;
/// let style = TextStyle { face: body, size: 18.0, ..TextStyle::default() };
/// # let _ = style;
/// # Ok(())
/// # }
/// ```
///
/// DELIBERATE: there is no `destroy_face` (ADR-0042). A face's outlines are kept
/// for the life of the program, which is exactly the lifetime policy the asset
/// store already has in v1 — assets live until `unload` or exit, with no
/// refcounting and no automatic drop (assets.md §1) — and it is what lets a
/// [`Face`] be a plain `Copy` value that a style can hold and a layout can
/// measure with, anywhere, without reaching for this store. Freeing one would
/// mean either handing out something that can dangle or making every style
/// borrow, and prototypes load two faces, not two thousand.
#[derive(Debug)]
pub struct Fonts {
    /// Every face created here, in creation order — the index is the id.
    faces: Vec<Face>,
}

impl Fonts {
    /// An empty store.
    ///
    /// DELIBERATE: no `Default` impl, despite `clippy::new_without_default`
    /// (see ADR-0012) — one way to do everything, and `new` is that way.
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self { faces: Vec::new() }
    }

    /// Parse `ttf` into a face this store keeps, or say why it could not.
    ///
    /// `name` is what the face is called in error messages and in `Debug`; it
    /// has no other meaning and nothing looks a face up by it.
    ///
    /// The bytes are a `.ttf` or `.otf` file — normally one an
    /// [`Assets`](jidousha_assets::Assets) store loaded with `load_bytes` and
    /// reported `Ready`, which is what makes this the moment a face can be
    /// built: the store never says ready for bytes it does not have
    /// (assets.md §3), so a face is either made of a real file or not made at
    /// all.
    ///
    /// # Errors
    ///
    /// [`FontError`] if the bytes are not a face this engine can read, or if
    /// the face has no glyph in the covered range. Both are facts about the
    /// file rather than mistakes at the call, which is why this reports rather
    /// than panics (core §9).
    pub fn try_create_face(&mut self, name: &str, ttf: &[u8]) -> Result<Face, FontError> {
        let id = self.faces.len() as u32;
        let face = TtfFace::parse(id, name, ttf)?;
        // Leaked on purpose: see the `destroy_face` note on this type. A face
        // outlives everything that could hold one, which is what makes `Face`
        // a `Copy` value rather than a borrow.
        let face: &'static TtfFace = Box::leak(Box::new(face));
        let face = Face(Kind::Ttf(face));
        self.faces.push(face);
        Ok(face)
    }

    /// Every face created here, in creation order.
    ///
    /// The index is the face's id, which is what [`upload_text_atlases`] reads
    /// it as: a driver holds this list and an atlas id names a position in it.
    #[must_use]
    pub fn faces(&self) -> &[Face] {
        &self.faces
    }
}

impl jidousha_core::Resource for Fonts {}

/// Upload the atlas behind every glyph `quads` draws that is not on the GPU yet.
///
/// Called once a frame by the driver, **after** the Draw phase and before the
/// frame is planned, because until a game has drawn there is nothing to say
/// which faces at which sizes this frame wants. That ordering is what keeps a
/// new size from flashing the placeholder for a frame: the quads naming the
/// atlas and the atlas itself reach the plan together.
///
/// `faces` is [`Fonts::faces`] — the list an atlas id indexes into. A game with
/// no loaded face draws no such quad and this does nothing.
///
/// PERF: the scan is over the frame's quads and does nothing but a table lookup
/// for all but the first frame a size appears on. Rasterizing one atlas is a
/// few milliseconds and happens once per (face, size) for the life of the
/// program.
pub fn upload_text_atlases(
    faces: &[Face],
    quads: &[Quad],
    backend: &mut dyn RenderBackend,
    textures: &mut TextureTable,
) {
    for quad in quads {
        if textures.is_ready(quad.texture) {
            continue;
        }
        let Some((face_id, px)) = atlas_of(quad.texture) else {
            continue;
        };
        let Some(face) = faces.get(face_id as usize).and_then(ttf_of) else {
            // Not an atlas of any face this store made — an asset id below the
            // reserved line cannot happen, so this is a quad from a `Fonts`
            // other than the one the driver holds. It draws the placeholder,
            // which is the right answer and the one §5 already gives.
            continue;
        };
        if px < ttf::MIN_PX || px > MAX_PX {
            continue;
        }
        let (width, height) = ttf::atlas_px(face, px);
        let uploaded = backend.create_texture(
            &TextureDesc {
                size: PhysicalSize::new(width, height),
            },
            &ttf::atlas_texels(face, px),
        );
        textures.register(quad.texture, uploaded);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_atlas_id_round_trips_through_its_face_and_size() {
        for face_id in [0_u32, 1, 7, 4095] {
            for px in [ttf::MIN_PX, 12, 33, MAX_PX] {
                let id = atlas_texture(face_id, px);
                assert_eq!(atlas_of(id), Some((face_id, px)));
            }
        }
    }

    #[test]
    fn an_atlas_id_is_never_a_built_in_or_an_asset() {
        // The reservation the whole scheme rests on, from both ends: below the
        // asset line, and above the two ids the renderer already spent.
        let id = atlas_texture(0, ttf::MIN_PX);
        assert!(id.bits() > super::super::FONT_TEXTURE.bits());
        assert_ne!(id, TextureId::WHITE);
        assert!(atlas_texture(4095, MAX_PX).bits() < 1 << 32);
        assert_eq!(atlas_of(TextureId::WHITE), None);
    }

    #[test]
    fn bytes_that_are_not_a_font_are_reported_rather_than_parsed() {
        let mut fonts = Fonts::new();
        let Err(error) = fonts.try_create_face("nonsense", b"this is not a font") else {
            panic!("nonsense parsed as a face");
        };
        let message = error.to_string();
        assert!(message.contains("[jidousha]"), "{message}");
        assert!(message.contains("nonsense"), "{message}");
        assert!(message.contains("fix:"), "{message}");
        assert!(fonts.faces().is_empty(), "and no face was created");
    }
}
