//! The native asset source: a loader thread, a channel, and the filesystem.
//!
//! Key types: `FileSource`.
//! Depends on: `std::fs`, `std::thread`, `jidousha-assets`. Must never be
//! depended on by: `jidousha-assets` — I/O lives on this side of the seam
//! (assets.md §5).
//! INVARIANT: nothing here blocks the frame. Reading and decoding happen on the
//! loader thread; the frame's only contact with this is draining a channel at
//! the commit point, which is why a 2048×2048 PNG costs nothing on the tick it
//! finishes (assets.md §4, §5).
//! INVARIANT: paths are case-strict on every platform, Windows included — see
//! `resolve` below for why that is worth a directory listing.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

use jidousha_assets::{
    AssetError, AssetKind, ByteSource, Completion, Payload, RequestId, decode_png,
};

/// One job for the loader thread.
struct Job {
    request: RequestId,
    path: PathBuf,
    /// The path as the game wrote it, for the case-strict check and for errors.
    asked_for: String,
    kind: AssetKind,
}

/// Reads assets from disk, on a thread of its own.
///
/// ```no_run
/// use jidousha_assets::Assets;
/// use jidousha_platform::FileSource;
///
/// // Paths are relative to the root, with forward slashes everywhere
/// // (assets.md §2).
/// let mut assets = Assets::new(FileSource::new("assets"));
/// let hero = assets.load_texture("sprites/hero.png");
/// # let _ = hero;
/// ```
pub struct FileSource {
    root: PathBuf,
    jobs: Sender<Job>,
    /// DELIBERATE: a `Mutex` around a `Receiver`, which is `Send` but not
    /// `Sync`. `Assets` is a world resource and resources are `Send + Sync`
    /// (core.md §3), so the receiver needs to be too. The lock is never
    /// contended — the store is drained from one thread, at one point in the
    /// frame — and A0's `ByteSource` doc predicted exactly this shape.
    done: Mutex<Receiver<Completion>>,
    outstanding: usize,
    next_request: u64,
}

impl FileSource {
    /// A source reading from `root`.
    ///
    /// The thread starts here and lives as long as the source. It ends when the
    /// source is dropped and the job channel closes.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let (jobs, work): (Sender<Job>, Receiver<Job>) = channel();
        let (finished, done): (Sender<Completion>, Receiver<Completion>) = channel();

        // One thread, not a pool: prototype-scale games load a handful of
        // things at startup, and a pool is a scheduling decision with no
        // evidence behind it yet (PERF-revisit).
        thread::Builder::new()
            .name("jidousha-assets".to_owned())
            .spawn(move || {
                for job in work {
                    let result = load(&job);
                    // A send failure means the store is gone, which means
                    // nobody is waiting for this. Stopping is right.
                    if finished
                        .send(Completion {
                            request: job.request,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            // A machine that cannot start a thread cannot run a game; there is
            // no useful degraded mode to fall back to.
            .unwrap_or_else(|error| {
                panic!(
                    "{}",
                    jidousha_core::message(
                        "the asset loader thread could not be started",
                        &error.to_string(),
                        "the process has hit a thread or memory limit",
                        "raise the limit, or run fewer programs — the engine needs one loader \
                         thread for the life of the process",
                    )
                )
            });

        Self {
            root: root.into(),
            jobs,
            done: Mutex::new(done),
            outstanding: 0,
            next_request: 0,
        }
    }

    /// The root every path is relative to.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Do one job: resolve the path, read it, and decode it if it is a texture.
fn load(job: &Job) -> Result<Payload, AssetError> {
    resolve(&job.path, &job.asked_for)?;
    let bytes = std::fs::read(&job.path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => AssetError::NotFound,
        _ => AssetError::Unreadable {
            detail: error.to_string(),
        },
    })?;
    match job.kind {
        AssetKind::Bytes => Ok(Payload::Bytes(bytes)),
        // Decoded here, on the loader thread, which is the whole reason this
        // thread exists — PNG decoding is the slow part (assets.md §5).
        AssetKind::Texture => decode_png(&bytes).map(Payload::Texture),
    }
}

/// Check the file exists under exactly the name that was asked for.
///
/// CONTRACT (assets.md §2): loads are case-strict on **every** platform,
/// Windows and macOS included. Their filesystems are not, so a listing is the
/// only way to tell `player.png` from `Player.png` there — and the alternative
/// is art that loads locally and 404s on a web server, which is the single most
/// expensive class of asset bug because it only appears after deploying.
fn resolve(path: &Path, asked_for: &str) -> Result<(), AssetError> {
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err(AssetError::NotFound);
    };
    let Some(name) = name.to_str() else {
        return Err(AssetError::NotFound);
    };

    let Ok(listing) = std::fs::read_dir(parent) else {
        // No directory means no file. Reporting it as missing rather than
        // unreadable is right: from the game's side, the asset is not there.
        return Err(AssetError::NotFound);
    };

    let mut near_miss = None;
    for entry in listing.flatten() {
        let found = entry.file_name();
        let Some(found) = found.to_str() else {
            continue;
        };
        if found == name {
            return Ok(());
        }
        if found.eq_ignore_ascii_case(name) {
            // Rebuild the path the game *should* have written, so the message
            // can name it rather than just the leaf.
            let prefix = asked_for.rsplit_once('/').map_or("", |(head, _)| head);
            near_miss = Some(if prefix.is_empty() {
                found.to_owned()
            } else {
                format!("{prefix}/{found}")
            });
        }
    }
    match near_miss {
        Some(on_disk) => Err(AssetError::CaseMismatch { on_disk }),
        None => Err(AssetError::NotFound),
    }
}

impl ByteSource for FileSource {
    fn request(&mut self, path: &str, kind: AssetKind) -> RequestId {
        let request = RequestId::from_bits(self.next_request);
        self.next_request += 1;
        self.outstanding += 1;

        // Forward slashes in, native separators out: one path string works on
        // every platform, which is what makes the same game run everywhere
        // (assets.md §2 CONTRACT).
        let mut full = self.root.clone();
        for part in path.split('/') {
            full.push(part);
        }

        // A send failure means the loader thread is gone. Nothing here can
        // recover, and the request simply never completes — `all_ready` stays
        // false, which is visible, rather than a silent success.
        let _ = self.jobs.send(Job {
            request,
            path: full,
            asked_for: path.to_owned(),
            kind,
        });
        request
    }

    fn drain_completed(&mut self, _tick: u64) -> Vec<Completion> {
        let Ok(done) = self.done.lock() else {
            // A poisoned lock means the loader thread panicked mid-send. There
            // is nothing to drain and nothing to be done about it here.
            return Vec::new();
        };
        let mut completed = Vec::new();
        // Empty and Disconnected both mean "nothing more right now": a
        // disconnected channel is a loader thread that has stopped, and the
        // store finding out via `all_ready` staying false beats a panic here.
        while let Ok(completion) = done.try_recv() {
            completed.push(completion);
        }
        self.outstanding = self.outstanding.saturating_sub(completed.len());

        // CONTRACT (assets.md §5): one poll's completions come back in request
        // order. The channel preserves the order the loader finished them in,
        // which for one thread is the order they were asked for — sorting says
        // so rather than relying on it, and costs nothing at these sizes.
        completed.sort_by_key(|completion| completion.request);
        completed
    }

    fn outstanding(&self) -> usize {
        self.outstanding
    }
}
