//! The A1 exit criterion: real files load, and every way they can fail says
//! something specific (assets.md §6, §8).
//!
//! These build their own asset root in a temporary directory rather than
//! reading the repository's, so a test can create a file whose name differs
//! only in case — which is not something to check into a repository that people
//! clone onto case-insensitive filesystems.

use std::path::{Path, PathBuf};

use jidousha_assets::{AssetStatus, Assets, MAX_TEXTURE_SIZE};
use jidousha_platform::FileSource;

/// A directory that deletes itself, so a failing test leaves nothing behind.
struct Root {
    path: PathBuf,
}

impl Root {
    /// A fresh asset root, named for the test that asked for it.
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!("jidousha-a1-{name}"));
        let _ = std::fs::remove_dir_all(&path);
        let Ok(()) = std::fs::create_dir_all(&path) else {
            panic!("a temporary directory should be creatable");
        };
        Self { path }
    }

    /// Write a file, creating any directories along the way.
    fn write(&self, relative: &str, bytes: &[u8]) -> &Self {
        let full = self.path.join(relative);
        if let Some(parent) = full.parent() {
            let Ok(()) = std::fs::create_dir_all(parent) else {
                panic!("a temporary directory should be creatable");
            };
        }
        let Ok(()) = std::fs::write(&full, bytes) else {
            panic!("a temporary file should be writable");
        };
        self
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Whether this filesystem tells `a.txt` from `A.txt`.
///
/// Asked rather than assumed. Linux says yes, Windows says no, and macOS says
/// whichever the volume was formatted for — so a `cfg` on the operating system
/// would be wrong on macOS and would go on being wrong silently. Writing a file
/// and trying to open it by another name is the actual question.
fn case_sensitive(root: &Root) -> bool {
    root.write("case-probe.txt", b"probe");
    std::fs::read(root.path().join("CASE-PROBE.TXT")).is_err()
}

/// A PNG of one flat colour.
fn png(width: u32, height: u32, rgba: [u8; 4]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let Ok(mut writer) = encoder.write_header() else {
            panic!("the test's own encoder should accept this header");
        };
        let texels: Vec<u8> = rgba
            .iter()
            .copied()
            .cycle()
            .take((width * height * 4) as usize)
            .collect();
        let Ok(()) = writer.write_image_data(&texels) else {
            panic!("the test's own encoder should accept this data");
        };
    }
    out
}

/// Drive a store until nothing is in flight, returning what failed.
///
/// The loader is on another thread, so this waits the way a game does — by
/// committing repeatedly — with a real pause between commits, because a spin
/// loop can outrun a disk by three orders of magnitude and conclude that
/// nothing ever arrives.
fn settle(assets: &mut Assets) -> Vec<jidousha_assets::AssetFailure> {
    let mut failures = Vec::new();
    for tick in 1..=600 {
        std::thread::sleep(std::time::Duration::from_millis(2));
        failures.extend(assets.commit(tick));
        if assets.all_ready() {
            break;
        }
    }
    assert!(assets.all_ready(), "the loader never finished");
    failures
}

#[test]
fn a_png_on_disk_becomes_texels() {
    let root = Root::new("reads");
    root.write("art/red.png", &png(4, 2, [200, 30, 40, 255]));

    let mut assets = Assets::new(FileSource::new(root.path()));
    let red = assets.load_texture("art/red.png");
    assert!(settle(&mut assets).is_empty());

    assert_eq!(assets.status(red), AssetStatus::Ready);
    let Some(texture) = assets.texture_of(red) else {
        panic!("a Ready texture has texels");
    };
    assert_eq!((texture.width, texture.height), (4, 2));
    assert_eq!(&texture.rgba[0..4], &[200, 30, 40, 255]);
    assert_eq!(texture.rgba.len(), 4 * 2 * 4);
}

#[test]
fn raw_bytes_load_without_being_decoded() {
    // `load_bytes` is for anything the engine does not decode itself
    // (assets.md §3), so its payload is the file, byte for byte.
    let root = Root::new("bytes");
    root.write("levels/one.bin", b"not a picture at all");

    let mut assets = Assets::new(FileSource::new(root.path()));
    let level = assets.load_bytes("levels/one.bin");
    assert!(settle(&mut assets).is_empty());

    assert_eq!(assets.bytes_of(level), Some(&b"not a picture at all"[..]));
}

#[test]
fn a_texture_has_no_raw_bytes_and_a_blob_has_no_texels() {
    // The two payload shapes do not impersonate each other: asking a texture
    // for its file gets nothing, rather than the decoded texels pretending.
    let root = Root::new("shapes");
    root.write("art/one.png", &png(1, 1, [1, 2, 3, 4]));
    root.write("data.bin", b"bytes");

    let mut assets = Assets::new(FileSource::new(root.path()));
    let texture = assets.load_texture("art/one.png");
    let blob = assets.load_bytes("data.bin");
    assert!(settle(&mut assets).is_empty());

    assert_eq!(assets.bytes_of(texture), None);
    assert!(assets.texture_of(texture).is_some());
    assert_eq!(assets.bytes_of(blob), Some(&b"bytes"[..]));
}

#[test]
fn a_missing_file_says_so_and_says_where_to_look() {
    let root = Root::new("missing");
    let mut assets = Assets::new(FileSource::new(root.path()));
    let _ = assets.load_texture("art/nowhere.png");
    let failures = settle(&mut assets);

    assert_eq!(failures.len(), 1);
    let text = failures[0].message();
    assert!(text.contains("art/nowhere.png"), "{text}");
    assert!(text.contains("no file at that path"), "{text}");
    assert!(text.contains("asset root"), "{text}");
    assert!(text.contains("file_source.rs"), "the game's line: {text}");
}

#[test]
fn a_name_that_differs_only_in_case_is_refused_and_the_real_name_is_named() {
    // The bug this check exists for: it works on Windows and macOS, whose
    // filesystems do not care, and 404s on a web server that does. Refusing on
    // every platform means it is found on the first run (assets.md §2).
    let root = Root::new("case");
    root.write("art/Hero.png", &png(1, 1, [9, 9, 9, 255]));

    let mut assets = Assets::new(FileSource::new(root.path()));
    let handle = assets.load_texture("art/hero.png");
    let failures = settle(&mut assets);

    assert_eq!(assets.status(handle), AssetStatus::Failed);
    assert_eq!(failures.len(), 1);
    let text = failures[0].message();
    assert!(
        text.contains("\"art/Hero.png\""),
        "names the real file: {text}"
    );
    assert!(text.contains("case differs"), "{text}");
    assert!(text.contains("rename"), "the fix is a rename: {text}");
}

#[test]
fn the_exact_name_still_loads_when_a_near_miss_exists() {
    // The case check must not become a case *ban*: a directory holding both
    // `Hero.png` and `hero.png` is legal, and each should load itself.
    //
    // Only where the filesystem can hold both. On Windows — and on a
    // case-insensitive macOS volume — writing the second name overwrites the
    // first, so the situation under test cannot be built, and a failure there
    // would say something about `std::fs` rather than about the loader. The
    // *interesting* case test, the one that refuses a near miss, runs
    // everywhere and does pass on Windows.
    let root = Root::new("both");
    if !case_sensitive(&root) {
        println!("skipped: this filesystem cannot hold Hero.png beside hero.png");
        return;
    }
    root.write("art/Hero.png", &png(1, 1, [1, 1, 1, 255]));
    root.write("art/hero.png", &png(2, 2, [2, 2, 2, 255]));

    let mut assets = Assets::new(FileSource::new(root.path()));
    let upper = assets.load_texture("art/Hero.png");
    let lower = assets.load_texture("art/hero.png");
    assert!(
        settle(&mut assets).is_empty(),
        "both exist exactly as asked"
    );

    assert_eq!(
        assets.texture_of(upper).map(|texture| texture.width),
        Some(1)
    );
    assert_eq!(
        assets.texture_of(lower).map(|texture| texture.width),
        Some(2)
    );
}

#[test]
fn a_file_that_is_not_a_png_fails_at_decode_and_says_so() {
    let root = Root::new("garbage");
    root.write("art/broken.png", b"PNG? no, this is just some text");

    let mut assets = Assets::new(FileSource::new(root.path()));
    let _ = assets.load_texture("art/broken.png");
    let failures = settle(&mut assets);

    let text = failures[0].message();
    assert!(text.contains("not a PNG"), "{text}");
    assert!(text.contains("re-export"), "{text}");
}

#[test]
fn a_file_that_is_not_a_png_still_loads_as_bytes() {
    // Only textures are decoded. The same file is fine as a blob, which is
    // what makes `load_bytes` the escape hatch for anything a game invents.
    let root = Root::new("garbage-bytes");
    root.write("art/broken.png", b"PNG? no, this is just some text");

    let mut assets = Assets::new(FileSource::new(root.path()));
    let blob = assets.load_bytes("art/broken.png");
    assert!(settle(&mut assets).is_empty());
    assert_eq!(assets.status(blob), AssetStatus::Ready);
}

#[test]
fn an_image_past_the_envelope_names_its_size_and_the_limit() {
    let root = Root::new("oversized");
    let over = MAX_TEXTURE_SIZE + 1;
    root.write("art/huge.png", &png(over, 1, [0, 0, 0, 255]));

    let mut assets = Assets::new(FileSource::new(root.path()));
    let _ = assets.load_texture("art/huge.png");
    let failures = settle(&mut assets);

    let text = failures[0].message();
    assert!(text.contains(&format!("{over}x1")), "{text}");
    assert!(text.contains("2048x2048"), "{text}");
}

#[test]
fn a_directory_asked_for_as_a_file_fails_rather_than_hanging() {
    let root = Root::new("directory");
    root.write("art/inside.png", &png(1, 1, [0, 0, 0, 255]));

    let mut assets = Assets::new(FileSource::new(root.path()));
    let handle = assets.load_bytes("art");
    let failures = settle(&mut assets);

    assert_eq!(assets.status(handle), AssetStatus::Failed);
    assert_eq!(failures.len(), 1);
}

#[test]
fn many_assets_load_together_and_each_reports_once() {
    let root = Root::new("many");
    for index in 0..12 {
        root.write(
            &format!("art/{index}.png"),
            &png(2, 2, [index as u8, 0, 0, 255]),
        );
    }
    root.write("art/bad.png", b"not a png");

    let mut assets = Assets::new(FileSource::new(root.path()));
    let handles: Vec<_> = (0..12)
        .map(|index| assets.load_texture(&format!("art/{index}.png")))
        .collect();
    let _ = assets.load_texture("art/bad.png");
    let failures = settle(&mut assets);

    for (index, handle) in handles.iter().enumerate() {
        assert_eq!(
            assets.status(*handle),
            AssetStatus::Ready,
            "art/{index}.png"
        );
        assert_eq!(
            assets.texture_of(*handle).map(|texture| texture.rgba[0]),
            Some(index as u8)
        );
    }
    assert_eq!(failures.len(), 1, "only the broken one, and only once");
}

#[test]
fn failures_are_reported_in_the_order_they_were_asked_for() {
    // CONTRACT (assets.md §5): one poll's completions come back ordered by
    // request id. It matters because replay compares what each tick reported,
    // and a loader thread finishing two files in whichever order the disk
    // happened to answer would make that list vary between runs.
    //
    // Found by mutation testing: reversing the order broke nothing until this
    // existed, because every other test here has at most one failure.
    let root = Root::new("order");
    root.write("art/a-bad.png", b"not a png");
    root.write("art/b-bad.png", b"also not a png");
    root.write("art/c-bad.png", b"nor this");

    let mut assets = Assets::new(FileSource::new(root.path()));
    // Asked for in an order that is neither alphabetical nor reversed, so
    // neither sorting by name nor reversing would pass by accident.
    for path in ["art/b-bad.png", "art/c-bad.png", "art/a-bad.png"] {
        let _ = assets.load_texture(path);
    }
    let failures = settle(&mut assets);

    let reported: Vec<&str> = failures
        .iter()
        .map(|failure| failure.path.as_str())
        .collect();
    assert_eq!(
        reported,
        vec!["art/b-bad.png", "art/c-bad.png", "art/a-bad.png"],
        "request order, not disk order"
    );
}

#[test]
fn forward_slashes_work_whatever_the_platform_separator_is() {
    // CONTRACT (assets.md §2): identical path strings work identically on
    // every platform, so a game says `art/hero.png` on Windows too.
    let root = Root::new("slashes");
    root.write("deep/nested/here.png", &png(1, 1, [5, 5, 5, 255]));

    let mut assets = Assets::new(FileSource::new(root.path()));
    let handle = assets.load_texture("deep/nested/here.png");
    assert!(settle(&mut assets).is_empty());
    assert_eq!(assets.status(handle), AssetStatus::Ready);
}

#[test]
fn nothing_arrives_before_a_commit() {
    // The commit point holds for the real loader exactly as it does for the
    // scripted one: the file may well be read before the game asks, and the
    // status still does not move until `commit` (assets.md §4 CONTRACT).
    let root = Root::new("commit-point");
    root.write("art/one.png", &png(1, 1, [1, 1, 1, 255]));

    let mut assets = Assets::new(FileSource::new(root.path()));
    let handle = assets.load_texture("art/one.png");
    std::thread::sleep(std::time::Duration::from_millis(50));

    assert_eq!(
        assets.status(handle),
        AssetStatus::Loading,
        "read by now, and still not visible"
    );
    assets.commit(1);
    assert_eq!(assets.status(handle), AssetStatus::Ready);
}
