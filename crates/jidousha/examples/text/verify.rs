//! The headless check: what the sheet drew, and the floors it drew inside.
//!
//! Three things this asserts that nothing else can:
//!
//! 1. **The family really loaded**, off the real asset root, through
//!    `load_bytes` and `Fonts::try_create_face`. A face that silently failed to
//!    parse would leave the sheet drawing in the built-in font, which looks
//!    fine and is a different picture.
//! 2. **The atlas path ran.** Every loaded-face row's quads sample that face's
//!    atlas at that size, and the three sizes land on three different atlases.
//! 3. **The readability floors hold under measured extents** (ADR-0042 §3).
//!    Every floor here is asserted against `TextStyle::measure`, not against a
//!    character count times a fixed advance — the assumption a proportional
//!    face breaks, and the reason the measurement API is part of the feature.
//!
//! The floors themselves are ordinary: nothing is set below the minimum size,
//! no row runs off the page, no two rows collide, the two columns of the
//! specimen do not run into each other, and the fitted line stays inside the
//! column rule it was cut to fit.

use std::process::ExitCode;

use jidousha::prelude::*;
use jidousha::testing::{
    FONT_TEXTURE, FrameRecord, FrameRecorder, RenderBackend, RenderError, WgpuBackend,
    create_builtin_textures, encode_png, upload_text_atlases,
};

use crate::{
    BITMAP_COLUMN_END, COLUMN, Family, MIN_TEXT, PAGE, PROSE, SIZES, SPECIMEN, WINDOW, build, sheet,
};

/// How many ticks to give the family before giving up on it.
///
/// The native loader reads on a thread of its own (assets.md §5), so how many
/// ticks a file takes is a fact about the disk and the scheduler rather than
/// about the game — a warm cache resolved it in four hundred here and a cold
/// one did not manage it in six hundred, which is exactly why this is a gate
/// with a cap rather than a fixed number of ticks. The cap is enormous on
/// purpose: a headless tick of this sheet is microseconds, so the whole budget
/// is under a second, and a run that hangs waiting for a file is worse than one
/// that says the file never came.
const MAX_TICKS: u64 = 200_000;

/// How far a glyph may lean outside the box the pen swept, as a fraction of the
/// line.
///
/// Not slack in the floor — the ink of a proportional face genuinely leans
/// outside its advance box, which is what a side bearing is, and a border of a
/// texel is added around every glyph so nearest sampling is safe. This is the
/// size of that, stated once and asserted against rather than discovered.
const OVERHANG: f32 = 0.25;

/// How big the captured artifact is — the window's own shape, halved.
///
/// A capture at another aspect is a picture of a different framing, and this
/// sheet is a layout: a crop would cut the loaded column off entirely.
const CAPTURE_SIZE: PhysicalSize = PhysicalSize::new(1280, 720);

/// How many polls to give the GPU handshake before calling it absent.
const HANDSHAKE_POLLS: usize = 10_000;

/// One assertion, and what to say when it fails.
struct Checks {
    /// Every failure, in the order they were found.
    failures: Vec<String>,
    /// How many were made, so a silent check is visible as a low number.
    made: usize,
}

impl Checks {
    fn new() -> Self {
        Self {
            failures: Vec::new(),
            made: 0,
        }
    }

    /// Require `held`, and say what it means if it did not.
    fn require(&mut self, held: bool, what: &str, specifics: String) {
        self.made += 1;
        if !held {
            self.failures.push(format!("{what} — {specifics}"));
        }
    }
}

/// What the store says about each file the sheet asked for.
///
/// In the failure message rather than only in a debugger, because the three
/// answers mean three different things: still `Loading` is a slow disk and a
/// cap set too low, `Failed` is a file that is not there or not readable, and
/// `Ready` with no face is bytes that did not parse.
fn asset_states(sim: &HeadlessSim) -> String {
    let handles = sim.world().resource::<Family>().handles.clone();
    let assets = sim.world().resource::<Assets>();
    handles
        .iter()
        .map(|handle| format!("{}: {:?}", assets.path_of(*handle), assets.status(*handle)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Whether `bounds` sits inside `area`, to within a hundredth of a unit.
fn inside(area: Rect, bounds: Rect) -> bool {
    const SLACK: f32 = 0.01;
    bounds.min.x >= area.min.x - SLACK
        && bounds.min.y >= area.min.y - SLACK
        && bounds.max.x <= area.max.x + SLACK
        && bounds.max.y <= area.max.y + SLACK
}

/// Run the sheet headless and judge it.
pub(super) fn run() -> ExitCode {
    let mut sim = headless(
        GameConfig {
            title: "jidousha — text",
            window_size: WINDOW,
            ..GameConfig::default()
        },
        build,
    );
    let mut recorder = FrameRecorder::new(WINDOW);
    let mut ticks = 0;
    while ticks < MAX_TICKS {
        ticks += 1;
        recorder.settle_assets(&mut sim, ticks);
        sim.tick();
        if sim.world().resource::<Family>().regular.is_some() {
            // One more, so the frame is drawn by a tick that had the faces
            // from its start rather than by the one that created them.
            recorder.settle_assets(&mut sim, ticks + 1);
            sim.tick();
            break;
        }
    }
    let frame = recorder.draw(&mut sim);

    let mut checks = Checks::new();
    let family = sim.world().resource::<Family>();
    let loaded = family.regular.is_some() && family.bold.is_some();
    checks.require(
        loaded,
        "the committed family did not load",
        format!(
            "regular is {:?} and bold is {:?} after {ticks} ticks; the files are \
             assets/fonts/FiraSans-Regular.ttf and FiraSans-Bold.ttf and the store says {}",
            family.regular.map(|face| face.name().to_owned()),
            family.bold.map(|face| face.name().to_owned()),
            asset_states(&sim),
        ),
    );
    let page = sheet(family);
    let (body, display) = (family.body(), family.display());
    checks.require(
        body != Face::BUILT_IN && display != Face::BUILT_IN && body != display,
        "the two weights are not two faces",
        format!(
            "body is {:?} and display is {:?}",
            body.name(),
            display.name()
        ),
    );

    let floors = judge_the_floors(&mut checks, &page);
    let atlases = judge_the_atlases(&mut checks, &recorder, &frame, &page, body);
    // The same quads the recorder planned, drawn again: the Draw phase cannot
    // write the world (ADR-0008), so running it twice produces the same frame,
    // and the replay needs the engine ids the `FrameRecord` has already
    // resolved away.
    let font = recorder.font_texture();
    let quads = sim.draw().quads().to_vec();
    let fonts = sim.world().resource::<Fonts>();
    let capture = capture_a_frame(&mut checks, &frame, fonts, &quads, font);

    let verdict = if checks.failures.is_empty() {
        format!("verified text: {} checks, all held", checks.made)
    } else {
        format!(
            "verified text: {} of {} checks failed",
            checks.failures.len(),
            checks.made
        )
    };
    println!("{verdict}");
    println!(
        "  the family: {}, {} — ready after {ticks} ticks",
        body.name(),
        display.name()
    );
    println!("  {floors}");
    println!("  {atlases}");
    println!("  capture: {capture}");
    for failure in &checks.failures {
        println!("  FAILED: {failure}");
    }
    if checks.failures.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// The readability floors, every one of them measured.
fn judge_the_floors(checks: &mut Checks, page: &crate::Sheet) -> String {
    let mut smallest = f32::MAX;
    for row in &page.rows {
        smallest = smallest.min(row.style.size);
        checks.require(
            row.style.size >= MIN_TEXT,
            "a row is set below the readability floor",
            format!(
                "{:?} is set at {:.1} and the floor is {MIN_TEXT:.0}",
                row.text, row.style.size
            ),
        );
        checks.require(
            inside(PAGE, row.bounds()),
            "a row runs off the page",
            format!(
                "{:?} measures {:?} and the page is {:?}",
                row.text,
                row.bounds(),
                PAGE
            ),
        );
        checks.require(
            !row.text.is_empty(),
            "a row says nothing",
            format!("at {:?}", row.at),
        );
        // A row set in the built-in face may only say what that face can
        // draw. It covers printable ASCII and nothing else, so a stray
        // typographic dash in a caption is a box on the screen — legible
        // enough to survive a screenshot, and wrong. The coverage row is the
        // one place a box is on purpose, and it is set in the loaded face.
        checks.require(
            row.style.face != Face::BUILT_IN || row.text.is_ascii(),
            "a row set in the built-in face says something it cannot draw",
            format!(
                "{:?} has a character outside printable ASCII, which draws the fallback box",
                row.text
            ),
        );
    }
    // No two rows collide. The measurement is what makes this assertable at
    // all: with a proportional face there is no character count that says how
    // wide a row is (ADR-0042 §3).
    for (index, row) in page.rows.iter().enumerate() {
        for other in page.rows.iter().skip(index + 1) {
            checks.require(
                !row.bounds().overlaps(other.bounds()),
                "two rows of type lie across each other",
                format!(
                    "{:?} at {:?} overlaps {:?} at {:?}",
                    row.text,
                    row.bounds(),
                    other.text,
                    other.bounds()
                ),
            );
        }
    }
    // The two columns of the specimen stay in their columns — the floor that a
    // fixed-advance assumption used to be able to state as arithmetic, now
    // stated as a measurement.
    for row in page.rows.iter().filter(|row| row.text == SPECIMEN) {
        let in_left = row.at.x < BITMAP_COLUMN_END;
        checks.require(
            !in_left || row.bounds().max.x <= BITMAP_COLUMN_END,
            "the built-in column runs into the loaded one",
            format!(
                "a {:.0}-unit specimen measures {:.1} wide and the column ends at \
                 {BITMAP_COLUMN_END:.0}",
                row.style.size,
                row.bounds().size().x
            ),
        );
    }
    // Every size on the sheet is one of the three it says it sets.
    for size in SIZES {
        checks.require(
            page.rows
                .iter()
                .any(|row| row.style.size == size && row.text == SPECIMEN),
            "a size the sheet advertises is not on it",
            format!("nothing is set at {size:.0}"),
        );
    }
    // The fitted line is inside the rule it was cut to fit, and one character
    // more would not be.
    let fitted = page
        .rows
        .iter()
        .find(|row| PROSE.starts_with(&row.text) && row.text != PROSE);
    match fitted {
        Some(row) => {
            checks.require(
                row.bounds().size().x <= COLUMN,
                "the fitted line runs past the column it was fitted to",
                format!(
                    "{} characters measure {:.2} in a column of {COLUMN:.0}",
                    row.text.chars().count(),
                    row.bounds().size().x
                ),
            );
            let more: String = PROSE.chars().take(row.text.chars().count() + 1).collect();
            checks.require(
                row.style.width_of(&more) > COLUMN,
                "the fitted line is shorter than it had to be",
                format!(
                    "one more character measures {:.2}, which still fits {COLUMN:.0}",
                    row.style.width_of(&more)
                ),
            );
        }
        None => checks.require(
            false,
            "the fitted column is not on the sheet",
            "no row is a prefix of the prose".to_owned(),
        ),
    }
    format!(
        "floors: {} rows, smallest {smallest:.0} units, floor {MIN_TEXT:.0}",
        page.rows.len()
    )
}

/// The atlas path: one texture per face and size, and quads on it.
fn judge_the_atlases(
    checks: &mut Checks,
    recorder: &FrameRecorder,
    frame: &FrameRecord,
    page: &crate::Sheet,
    body: Face,
) -> String {
    let placeholder = recorder.texture(TextureId::from_bits(u64::MAX));
    let mut seen = Vec::new();
    for size in SIZES {
        let atlas = recorder.texture(body.atlas_texture(size));
        checks.require(
            atlas != placeholder,
            "a size of the loaded face never reached the GPU",
            format!("the atlas for {size:.0} resolves to the placeholder"),
        );
        checks.require(
            !seen.contains(&atlas),
            "two sizes share one atlas",
            format!("{size:.0} landed on an atlas another size is already on"),
        );
        seen.push(atlas);
        let drawn = frame
            .quads()
            .iter()
            .filter(|quad| quad.texture == atlas)
            .count();
        checks.require(
            drawn == SPECIMEN.chars().count(),
            "the specimen did not draw one quad per character",
            format!(
                "{drawn} quads on the {size:.0} atlas, and the specimen is {} characters",
                SPECIMEN.chars().count()
            ),
        );
    }
    // The built-in font is still on the frame: both paths coexist, which is
    // the no-flag-day half of ADR-0042 §5 shown rather than claimed.
    let bitmap = frame
        .quads()
        .iter()
        .filter(|quad| quad.texture == recorder.font_texture())
        .count();
    checks.require(
        bitmap > 0,
        "the built-in font drew nothing",
        "the sheet sets its captions and its left column in it".to_owned(),
    );
    // Every glyph the loaded face drew is inside the row that measured it,
    // allowing for the side bearing the ink genuinely leans out by.
    for row in page.rows.iter().filter(|row| row.style.face == body) {
        let atlas = recorder.texture(row.style.face.atlas_texture(row.style.size));
        let slack = row.style.size * OVERHANG;
        let allowed = Rect::from_min_size(
            row.bounds().min - Vec2::splat(slack),
            row.bounds().size() + Vec2::splat(2.0 * slack),
        );
        for quad in frame.quads().iter().filter(|quad| quad.texture == atlas) {
            checks.require(
                inside(allowed, quad.bounds()),
                "a glyph drew outside the row that measured it",
                format!(
                    "{:?} measures {:?} and a glyph reached {:?}, further than the {OVERHANG} \
                     of a line a side bearing accounts for",
                    row.text,
                    row.bounds(),
                    quad.bounds()
                ),
            );
        }
    }
    format!(
        "atlases: {} sizes, {} distinct textures, {bitmap} built-in glyph quads",
        SIZES.len(),
        seen.len()
    )
}

/// Render the recorded frame on a GPU and write it out as a PNG.
///
/// The half a person looks at, and the half that would catch an atlas that
/// uploaded as garbage — every assertion above is about quads, and a quad
/// sampling a blank texture asserts exactly as well as one sampling a letter.
///
/// **A loaded face makes the texture-id check load-bearing twice over.** A plan
/// names engine texture ids, and an id only means anything to a backend that
/// created its textures in the same order — which for text means the built-ins
/// first and then the same atlases, from the same faces, in the order the same
/// quads name them. That is what `upload_text_atlases` is doing here.
///
/// A machine with no GPU is not a failure (renderer.md §9).
fn capture_a_frame(
    checks: &mut Checks,
    frame: &FrameRecord,
    fonts: &Fonts,
    quads: &[Quad],
    font: jidousha::testing::BackendTextureId,
) -> String {
    checks.require(
        frame.quad_count() > 0,
        "the sheet drew nothing at all",
        "no quads were submitted".to_owned(),
    );
    let mut gpu = WgpuBackend::offscreen(CAPTURE_SIZE);
    for _ in 0..HANDSHAKE_POLLS {
        match gpu.poll() {
            Ok(()) if gpu.is_ready() => break,
            Ok(()) => {}
            // No GPU on this machine is a fact about the runner, and the run
            // stays green. Anything else is a fault worth failing for.
            Err(error @ RenderError::NoAdapter { .. }) => {
                return format!("skipped, no GPU on this machine ({})", one_line(&error));
            }
            Err(error) => {
                checks.require(
                    false,
                    "the GPU handshake failed, and not because the machine has no GPU",
                    one_line(&error),
                );
                return format!("skipped, the GPU handshake failed ({})", one_line(&error));
            }
        }
    }
    if !gpu.is_ready() {
        return "skipped, the GPU handshake never finished".to_owned();
    }

    let mut textures = create_builtin_textures(&mut gpu);
    upload_text_atlases(fonts.faces(), quads, &mut gpu, &mut textures);
    checks.require(
        textures.resolve(FONT_TEXTURE) == font,
        "the replay's texture ids do not mean what the recorded plan means",
        format!(
            "the recorder put the built-in font on {font:?} and this backend put it on {:?}",
            textures.resolve(FONT_TEXTURE)
        ),
    );

    if let Err(error) = gpu.render(&frame.plan) {
        checks.require(
            false,
            "the GPU refused a plan the recorder had already accepted",
            one_line(&error),
        );
        return "skipped, the GPU refused the plan".to_owned();
    }
    let Ok(image) = gpu.capture() else {
        checks.require(
            false,
            "the GPU rendered the frame and then would not hand it back",
            "an offscreen backend can always read its own target".to_owned(),
        );
        return "skipped, the frame could not be read back".to_owned();
    };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("verify")
        .join("text.png");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(&path, encode_png(&image)).is_err() {
        checks.require(
            false,
            "the captured frame could not be written",
            format!("tried to write {}", path.display()),
        );
        return "skipped, the frame could not be written".to_owned();
    }
    let shown = std::fs::canonicalize(&path).unwrap_or(path);
    format!(
        "{}x{} written to {}",
        image.size.width,
        image.size.height,
        shown.display()
    )
}

/// An engine message flattened onto one line, for a `--verify` summary.
fn one_line(error: &RenderError) -> String {
    error
        .to_string()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
