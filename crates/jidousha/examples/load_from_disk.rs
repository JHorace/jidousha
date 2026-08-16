//! Real files, read off a real disk, decoded on a thread that is not this one
//! (assets.md §8, A1).
//!
//! Loads two PNGs from the repository's `assets/` root, waits for them the way
//! a game does — by committing once per frame and asking again — and then
//! asserts the texels are what the files hold. It also asks for two things that
//! are not there, which is where the interesting messages are: a missing file,
//! and a file whose name differs only in case.
//!
//! No window and no GPU: this is the asset system end to end, and it runs
//! headless.
//!
//! Run it: `cargo run -p jidousha --example load_from_disk`

#[cfg(not(target_arch = "wasm32"))]
use jidousha::prelude::*;

/// Where the art lives, relative to the workspace root.
#[cfg(not(target_arch = "wasm32"))]
const ASSET_ROOT: &str = "assets";

/// The web has no filesystem, and its source is A2's.
///
/// The example still builds for wasm — CI checks every example on every target
/// — and says why it does nothing there rather than being quietly absent.
#[cfg(target_arch = "wasm32")]
fn main() {
    println!(
        "load_from_disk reads files, which the web does not have (assets.md §5); the fetch source lands with A2"
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let mut assets = Assets::new(asset_source(ASSET_ROOT));

    // Forward slashes on every platform (assets.md §2). All four return
    // immediately; none of them has been read yet.
    let hero = assets.load_texture("sprites/hero.png");
    let glow = assets.load_texture("sprites/glow.png");
    // check-assets: deliberately missing — this example is about what failure
    // looks like, so one of its four loads has to fail.
    let missing = assets.load_texture("sprites/nothing_here.png");
    // The file on disk is `hero.png`; this asks for `Hero.png`. On Linux that
    // is simply absent, and on Windows or macOS the filesystem would hand it
    // over — the engine refuses on all three, so the bug is found on the first
    // run rather than after deploying to a web server (assets.md §2).
    // check-assets: deliberately missing
    let wrong_case = assets.load_texture("sprites/Hero.png");

    // The loading loop a game writes: commit once per frame, and keep going
    // until nothing is in flight. Real games draw during this rather than
    // spinning — the placeholder is why they can (ADR-0011).
    //
    // The sleep stands in for a frame. Without it this loop asks six hundred
    // times in well under a millisecond and gives up before the loader thread
    // has been scheduled once — which is a fair description of what "the disk
    // is slower than your loop" means, and the first thing this example got
    // wrong. A real frame takes about sixteen milliseconds; two is enough here.
    // Bytes, not a picture: `load_bytes` hands back exactly what is in the file,
    // for the level formats and data tables a game invents. Same store, same
    // commit point, same waiting.
    let raw: BytesHandle = assets.load_bytes("sprites/hero.png");

    let mut failures: Vec<AssetFailure> = Vec::new();
    let mut tick = 0;
    while !assets.all_ready() && tick < 600 {
        tick += 1;
        std::thread::sleep(std::time::Duration::from_millis(2));
        failures.extend(assets.commit(tick));
    }
    println!("everything resolved by tick {tick}");

    // The two that arrived.
    for (name, handle) in [("hero", hero), ("glow", glow)] {
        assert_eq!(assets.status(handle), AssetStatus::Ready, "{name}");
        let Some(texture) = assets.texture_of(handle) else {
            panic!("{name} is Ready, so it has texels");
        };
        println!(
            "{name}: {}x{} texels, {} bytes of RGBA",
            texture.width,
            texture.height,
            texture.rgba.len()
        );
        assert_eq!(
            texture.rgba.len() as u32,
            texture.width * texture.height * 4,
            "one RGBA quad per texel"
        );
    }

    // hero.png is a checkerboard: the top-left is the warm square, and four
    // texels along is the dark one. A decode that transposed the image or got
    // the stride wrong would still produce the right *number* of bytes, so the
    // check is on the pixels rather than the length.
    let Some(texture) = assets.texture_of(hero) else {
        panic!("hero is Ready");
    };
    assert_eq!(&texture.rgba[0..4], &[240, 80, 60, 255], "top-left square");
    assert_eq!(&texture.rgba[16..20], &[20, 30, 60, 255], "the next square");

    // glow.png ramps its alpha across the row, which is the channel most easily
    // lost by a decoder that widens RGB to RGBA and stops thinking.
    let Some(glow_texture) = assets.texture_of(glow) else {
        panic!("glow is Ready");
    };
    assert_eq!(glow_texture.rgba[3], 0, "the left edge is transparent");
    assert_eq!(glow_texture.rgba[7], 32, "and it ramps");

    // The two that did not.
    assert_eq!(assets.status(missing), AssetStatus::Failed);
    assert_eq!(assets.status(wrong_case), AssetStatus::Failed);
    println!("\n{} failure(s), each reported once:\n", failures.len());
    for failure in &failures {
        println!("{}\n", failure.message());
    }
    assert_eq!(failures.len(), 2, "one per failed asset, and only once");

    // The failures are typed, so a game can branch on *why* rather than parse a
    // sentence. A case mismatch is the one worth telling apart: it works on a
    // case-insensitive filesystem and 404s on a web server, and the error names
    // the file that is actually there.
    let near_miss = failures.iter().find_map(|failure| match &failure.error {
        AssetError::CaseMismatch { on_disk } => Some(on_disk.clone()),
        _ => None,
    });
    assert_eq!(near_miss.as_deref(), Some("sprites/hero.png"));
    assert!(
        failures
            .iter()
            .any(|failure| matches!(failure.error, AssetError::NotFound)),
        "and one that is simply absent"
    );

    // The same file, undecoded. A PNG starts with a fixed eight-byte signature,
    // which is a cheap way for the example to prove these are the file's own
    // bytes rather than something the engine made up.
    let Some(bytes) = assets.bytes_of(raw) else {
        panic!("the file is there, so its bytes are too");
    };
    assert_eq!(&bytes[..4], b"\x89PNG", "the bytes are the file's own");
    println!("read {} raw byte(s) from sprites/hero.png", bytes.len());

    // Committing again reports nothing: a failure is news exactly once, and the
    // placeholder does the per-frame signalling from then on (assets.md §6).
    assert!(assets.commit(tick + 1).is_empty());
    println!("committed again: nothing new to report");
}
