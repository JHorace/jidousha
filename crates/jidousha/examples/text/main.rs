//! Type: the engine's built-in font and a loaded TTF family, side by side.
//!
//! The specimen sheet for ADR-0042. The left column is the compiled-in
//! five-by-seven bitmap that every game has had since R3; the right column is
//! Fira Sans, loaded from `assets/fonts/`, rasterized to a glyph atlas and drawn
//! through the same sprite path — at three sizes, in two weights. **Both paths
//! are on the screen at once**, which is the claim: adding a real typeface did
//! not replace the one that needs no assets.
//!
//! What each block is for, because each one is a thing that can be wrong:
//!
//! - **The three sizes.** A face is rasterized once per size, so the small line
//!   is not the large one scaled down. Look at the 14 and the 40 together: if
//!   one atlas were serving both, one of them would be mush.
//! - **The measured box.** A rectangle drawn from `TextStyle::measure`, behind
//!   the line it measured. Type that leans outside it is a measurement a game
//!   would centre a heading with and get wrong.
//! - **The fitted column.** A long sentence cut to a fixed width with
//!   `TextStyle::fits_in`, inside the rule that shows the width. One character
//!   more would cross the rule.
//! - **The Latin-1 row and the strays.** `£ § ° ± × ÷ Ä ä Ö ö ç ñ` are covered;
//!   the two boxes after them are a snowman and a CJK character, which the face
//!   does not have. A box, never a panic and never a silent gap.
//!
//! **The camera is one world unit per pixel** (`height` equals the window's), so
//! a size of 22 is 22 pixels tall on screen and the atlas is rasterized 1:1.
//! That is the idiom a loaded face is authored against, and it is the one
//! `games/giri-rt` already uses for its chrome (renderer.md §6).
//!
//! Run it: `cargo run -p jidousha --example text`
//! Check it: `tools/verify text`
//! On the web: `tools/serve-web text`

mod verify;

use std::process::ExitCode;

use jidousha::prelude::*;

/// Where this example's art and fonts live (assets.md §2).
const ASSET_ROOT: &str = "assets";

/// How big the window opens, and therefore how big the world is.
const WINDOW: PhysicalSize = PhysicalSize::new(1280, 720);

/// The three sizes the sheet is set at, in world units — which are pixels here.
///
/// Small, body, and display. The small one is the size a readability floor
/// actually argues about; the large one is where a scaled-up atlas would show.
pub const SIZES: [f32; 3] = [40.0, 22.0, 14.0];

/// What every block of the sheet says.
pub const SPECIMEN: &str = "Hamburgefons 123";

/// The Latin-1 row, and two characters no Latin-1 face has.
pub const COVERAGE: &str = "£ § ° ± × ÷ Ä ä Ö ö ç ñ \u{2603}\u{4e2d}";

/// The sentence the fitted column cuts down.
pub const PROSE: &str = "A proportional face has no column width, so a game measures the string it \
     actually has.";

/// How wide the fitted column is, in world units.
pub const COLUMN: f32 = 300.0;

/// Where the loaded face's column starts, in world units.
///
/// Far enough right that the widest bitmap specimen cannot reach it — which is
/// a floor `verify.rs` asserts rather than a number eyeballed here.
pub const BITMAP_COLUMN_END: f32 = 640.0;

/// The page, in world units. One unit per pixel, so this is the window.
pub const PAGE: Rect = Rect {
    min: Vec2::new(0.0, 0.0),
    max: Vec2::new(WINDOW.width as f32, WINDOW.height as f32),
};

/// The smallest type this sheet is allowed to set, in world units.
///
/// The readability floor, asserted in `verify.rs` against measured extents.
pub const MIN_TEXT: f32 = 11.0;

/// Which draw band each thing is on.
mod layers {
    /// The measured box and the column rule, behind the type they describe.
    pub const RULE: i16 = 0;
    /// The type.
    pub const TEXT: i16 = 1;
}

/// The palette, so no colour is written twice.
mod ink {
    use jidousha::prelude::Color;

    /// The page.
    pub const PAGE: Color = Color::rgb(0.07, 0.07, 0.09);
    /// Body type.
    pub const BODY: Color = Color::rgb(0.93, 0.93, 0.95);
    /// A heading, and the bold weight.
    pub const HEAD: Color = Color::rgb(1.0, 0.84, 0.42);
    /// A label naming what a block is.
    pub const LABEL: Color = Color::rgb(0.52, 0.60, 0.70);
    /// The measured box and the column rule.
    pub const RULE: Color = Color::rgba(0.35, 0.55, 0.85, 0.55);
}

/// The two faces this sheet is set in, once they have loaded.
///
/// `None` until the bytes arrive, which is a real state and lasts a frame or
/// two: the sheet draws in the built-in font meanwhile, and says so.
#[derive(Debug, Default)]
pub struct Family {
    /// The regular weight.
    pub regular: Option<Face>,
    /// The bold weight.
    pub bold: Option<Face>,
    /// The two handles, kept so the loading state can be read each tick.
    pub handles: Vec<BytesHandle>,
}

impl Resource for Family {}

impl Family {
    /// The regular weight if it loaded, and the built-in font until it does.
    #[must_use]
    pub fn body(&self) -> Face {
        self.regular.unwrap_or(Face::BUILT_IN)
    }

    /// The bold weight if it loaded, and the built-in font until it does.
    #[must_use]
    pub fn display(&self) -> Face {
        self.bold.unwrap_or(Face::BUILT_IN)
    }
}

fn main() -> ExitCode {
    if std::env::args().any(|argument| argument == "--verify") {
        return verify::run();
    }
    match run(
        GameConfig {
            title: "jidousha: text",
            window_size: WINDOW,
            ..GameConfig::default()
        },
        build,
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Everything this example is, in one place, so `--verify` builds the same
/// game the window does.
pub fn build(app: &mut App) {
    app.add_system(Startup, set_the_page);
    app.add_system(Update, load_the_family);
    app.add_system(Draw, draw_the_sheet);
}

/// One world unit per pixel, origin at the top-left corner.
///
/// The camera is centred, so putting the origin in the corner is a matter of
/// centring it on half the page — and then every number in the layout below is
/// a pixel from the top-left, which is what a specimen sheet wants to be
/// written in.
fn set_the_page(world: &mut World) {
    world.insert_resource(Camera {
        center: Vec2::new(WINDOW.width as f32 / 2.0, WINDOW.height as f32 / 2.0),
        height: WINDOW.height as f32,
        clear_color: ink::PAGE,
        ..Camera::default()
    });
    world.insert_resource(Assets::new(asset_source(ASSET_ROOT)));
    world.insert_resource(Fonts::new());
    let mut family = Family::default();
    {
        let assets = world.resource_mut::<Assets>();
        // Written out at the load site, both of them, because that is what
        // makes `tools/check-assets` able to say the files are really there
        // (assets.md §2). The regular weight is committed with its licence;
        // the bold is a face of its own, not a flag on one (`CREDITS.md`).
        family.handles = vec![
            assets.load_bytes("fonts/FiraSans-Regular.ttf"),
            assets.load_bytes("fonts/FiraSans-Bold.ttf"),
        ];
    }
    world.insert_resource(family);
}

/// Turn the loaded bytes into faces, the tick they arrive.
///
/// The store never reports ready for bytes it does not have (assets.md §3), so
/// this is the moment a face can be built: either the file is there and the
/// face is real, or it is not and nothing was created.
fn load_the_family(world: &mut World) {
    if world.resource::<Family>().regular.is_some() {
        return;
    }
    let handles = world.resource::<Family>().handles.clone();
    let mut bytes = Vec::new();
    {
        let assets = world.resource::<Assets>();
        for handle in &handles {
            match assets.bytes_of(*handle) {
                Some(loaded) => bytes.push(loaded.to_vec()),
                // Still in flight, or gone. Either way there is nothing to
                // parse yet and the sheet keeps drawing in the built-in font.
                None => return,
            }
        }
    }
    let mut faces = Vec::new();
    {
        let fonts = world.resource_mut::<Fonts>();
        for (name, loaded) in ["Fira Sans", "Fira Sans Bold"].iter().zip(&bytes) {
            match fonts.try_create_face(name, loaded) {
                Ok(face) => faces.push(face),
                // A `FontError` is a fact about the file — a truncated
                // download, or something that is not a font — so it is reported
                // once and the sheet carries on in the face that needs no file.
                Err(error) => {
                    let error: FontError = error;
                    eprintln!("{error}");
                    return;
                }
            }
        }
    }
    let family = world.resource_mut::<Family>();
    family.regular = faces.first().copied();
    family.bold = faces.get(1).copied();
}

/// One row of type the sheet sets, as data.
///
/// **Every row is data before it is a picture.** `draw_the_sheet` submits these
/// and `verify.rs` measures them, so the readability floors are asserted
/// against the same rows the frame was built from — and against
/// `TextStyle::measure` rather than against a character count times a fixed
/// advance, which is a number a proportional face does not have (ADR-0042 §3).
#[derive(Clone, Debug)]
pub struct Row {
    /// The top-left of the first line.
    pub at: Vec2,
    /// What it says.
    pub text: String,
    /// How it is set.
    pub style: TextStyle,
}

impl Row {
    /// A row.
    fn new(at: Vec2, text: impl Into<String>, style: TextStyle) -> Self {
        Self {
            at,
            text: text.into(),
            style,
        }
    }

    /// The rectangle this row's pen sweeps, measured rather than assumed.
    #[must_use]
    pub fn bounds(&self) -> Rect {
        let extents: TextExtents = self.style.measure(&self.text);
        Rect::from_min_size(self.at, extents.size)
    }
}

/// Everything the sheet puts on the frame.
#[derive(Clone, Debug, Default)]
pub struct Sheet {
    /// Every row of type, in draw order.
    pub rows: Vec<Row>,
    /// The rectangles drawn behind rows to show a measurement.
    pub rules: Vec<Rect>,
}

/// A style on the text band.
fn style(face: Face, size: f32, color: Color) -> TextStyle {
    TextStyle {
        face,
        size,
        color,
        depth: Depth::layer(layers::TEXT),
    }
}

/// The style every caption under a block is set in.
///
/// The built-in face on purpose: a caption saying what a loaded face did should
/// still be readable on the frame where the loaded face is what went wrong.
fn caption() -> TextStyle {
    style(Face::BUILT_IN, 11.0, ink::LABEL)
}

/// The whole sheet, as data.
///
/// One function, so the window and the check cannot disagree about what is on
/// the page.
#[must_use]
pub fn sheet(family: &Family) -> Sheet {
    let (body, display) = (family.body(), family.display());
    let loaded = family.regular.is_some();
    let mut sheet = Sheet::default();

    sheet.rows.push(Row::new(
        Vec2::new(40.0, 30.0),
        "jidousha: text",
        style(display, 34.0, ink::HEAD),
    ));
    sheet.rows.push(Row::new(
        Vec2::new(40.0, 76.0),
        if loaded {
            "left: the built-in 5x7 bitmap  /  right: Fira Sans, loaded and rasterized"
        } else {
            "the family has not loaded yet: everything below is the built-in font"
        },
        style(body, 16.0, if loaded { ink::LABEL } else { ink::HEAD }),
    ));

    // The same specimen at three sizes, in both faces. A face is rasterized
    // once per size, so the small line is not the large one scaled down.
    let mut y = 130.0;
    for size in SIZES {
        sheet.rows.push(Row::new(
            Vec2::new(40.0, y),
            format!("{size:.0}"),
            caption(),
        ));
        sheet.rows.push(Row::new(
            Vec2::new(80.0, y),
            SPECIMEN,
            style(Face::BUILT_IN, size, ink::BODY),
        ));
        sheet.rows.push(Row::new(
            Vec2::new(BITMAP_COLUMN_END, y),
            SPECIMEN,
            style(body, size, ink::BODY),
        ));
        y += size + 30.0;
    }

    // One line with the rectangle `measure` reported drawn behind it. Type
    // leaning outside it is a measurement a game would centre a heading with.
    let measured = Row::new(
        Vec2::new(80.0, 330.0),
        SPECIMEN,
        style(body, 28.0, ink::BODY),
    );
    sheet.rules.push(measured.bounds());
    let extents = measured.style.measure(&measured.text);
    sheet.rows.push(Row::new(
        Vec2::new(80.0, 330.0 + extents.size.y + 10.0),
        format!(
            "TextStyle::measure: {:.1} x {:.1} world units, {} line",
            extents.size.x, extents.size.y, extents.lines
        ),
        caption(),
    ));
    sheet.rows.push(measured);

    // A sentence cut to a fixed column with `fits_in`, inside the rule that
    // shows the width. One character more would cross it.
    let column_at = Vec2::new(80.0, 430.0);
    let prose = style(body, 18.0, ink::BODY);
    sheet.rules.push(Rect::from_min_size(
        column_at,
        Vec2::new(COLUMN, prose.size),
    ));
    let fitted = prose.fits_in(PROSE, COLUMN);
    sheet.rows.push(Row::new(
        column_at,
        PROSE.chars().take(fitted).collect::<String>(),
        prose,
    ));
    sheet.rows.push(Row::new(
        Vec2::new(column_at.x, column_at.y + prose.size + 10.0),
        format!(
            "TextStyle::fits_in: {fitted} of {} characters in {COLUMN:.0} units; \
             columns_in says {} of any string",
            PROSE.chars().count(),
            prose.columns_in(COLUMN)
        ),
        caption(),
    ));

    // Latin-1, and the box a stray codepoint draws.
    let coverage = Vec2::new(80.0, 520.0);
    sheet
        .rows
        .push(Row::new(coverage, COVERAGE, style(body, 26.0, ink::BODY)));
    sheet.rows.push(Row::new(
        Vec2::new(coverage.x, coverage.y + 44.0),
        COVERAGE,
        style(display, 26.0, ink::HEAD),
    ));
    sheet.rows.push(Row::new(
        Vec2::new(coverage.x, coverage.y + 90.0),
        "ASCII and Latin-1 are covered; the last two are a snowman and a CJK character, \
         which draw the box",
        caption(),
    ));
    sheet.rows.push(Row::new(
        Vec2::new(coverage.x, coverage.y + 112.0),
        format!(
            "faces: {} / {} / {}",
            Face::BUILT_IN.name(),
            body.name(),
            display.name()
        ),
        caption(),
    ));
    sheet
}

/// Submit the sheet.
fn draw_the_sheet(ctx: &mut DrawCtx) {
    let sheet = sheet(ctx.world.resource::<Family>());
    for rule in &sheet.rules {
        ctx.rect(*rule, ink::RULE, Depth::layer(layers::RULE));
    }
    for row in &sheet.rows {
        ctx.text(row.at, &row.text, row.style);
    }
}
