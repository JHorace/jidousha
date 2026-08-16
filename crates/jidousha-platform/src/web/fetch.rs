//! The browser half of the web asset source.
//!
//! Key types: `WebSource`.
//! Depends on: `web-sys`, `wasm-bindgen-futures`. Compiled for wasm only.
//! INVARIANT: every decision that could be made without a browser was made
//! somewhere else — the status mapping in `jidousha-assets`, the URL join in
//! the parent module. What is left here is the call into JavaScript, and the
//! browser check is what verifies it.

use std::sync::{Arc, Mutex};

use jidousha_assets::{
    AssetError, AssetKind, ByteSource, Completion, Payload, RequestId, decode_png,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture, spawn_local};

/// One request that has come back, before it has been decoded.
///
/// Decoding waits for the commit rather than happening in the callback because
/// assets.md §5 puts web decode on the main thread at the commit point — and
/// because a callback that decoded would do it at whatever moment the network
/// chose, which is the one thing this design keeps off the timeline.
struct Fetched {
    request: RequestId,
    kind: AssetKind,
    result: Result<Vec<u8>, AssetError>,
}

/// Reads assets over HTTP, from wherever the page was served.
///
/// ```no_run
/// # #[cfg(target_arch = "wasm32")] {
/// use jidousha_assets::Assets;
///
/// // Relative to the page, so a game deployed in a subdirectory works without
/// // knowing where it lives (assets.md §2).
/// let mut assets = Assets::new(jidousha_platform::WebSource::new("assets"));
/// let hero = assets.load_texture("sprites/hero.png");
/// # let _ = hero;
/// # }
/// ```
pub struct WebSource {
    root: String,
    /// What has come back and not yet been drained.
    ///
    /// An `Arc<Mutex<..>>` for the same reason `FileSource` has one: `Assets` is
    /// a world resource and resources are `Send + Sync`. Wasm is
    /// single-threaded, so the lock is never contended and never blocks — but
    /// the bound is the bound.
    arrived: Arc<Mutex<Vec<Fetched>>>,
    outstanding: usize,
    next_request: u64,
}

impl WebSource {
    /// A source fetching from `root`, relative to the page.
    #[must_use]
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            arrived: Arc::new(Mutex::new(Vec::new())),
            outstanding: 0,
            next_request: 0,
        }
    }
}

/// Ask the browser for one URL, and push the answer onto the queue.
///
/// DELIBERATE: `spawn_local`, where R1's GPU handshake polls its futures by
/// hand. R1's reason was that the same code had to work on native and web, and
/// a blocks-on-native/spawns-on-web design would be two implementations of one
/// thing. This file compiles for one target, so there is no second
/// implementation to keep honest — and the browser is going to call a callback
/// whatever shape the code around it takes (ADR-0011 governs the engine's
/// *API*, which is still poll-based: nothing here is visible until `commit`).
fn fetch(url: String, request: RequestId, kind: AssetKind, arrived: Arc<Mutex<Vec<Fetched>>>) {
    spawn_local(async move {
        let result = fetch_bytes(&url).await;
        if let Ok(mut queue) = arrived.lock() {
            queue.push(Fetched {
                request,
                kind,
                result,
            });
        }
    });
}

/// The bytes at `url`, or why they did not arrive.
async fn fetch_bytes(url: &str) -> Result<Vec<u8>, AssetError> {
    let Some(window) = web_sys::window() else {
        return Err(AssetError::Unreachable {
            detail: "there is no window to fetch from".to_owned(),
        });
    };
    let response = JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|error| AssetError::Unreachable {
            detail: describe(&error),
        })?;
    let Ok(response) = response.dyn_into::<web_sys::Response>() else {
        return Err(AssetError::Unreachable {
            detail: "fetch resolved to something that is not a response".to_owned(),
        });
    };
    // A 404 becomes `NotFound`, everything else keeps its number. The mapping
    // is in the assets crate, where a test on any machine can reach it.
    if let Some(error) = AssetError::from_http_status(response.status()) {
        return Err(error);
    }
    let buffer =
        JsFuture::from(
            response
                .array_buffer()
                .map_err(|error| AssetError::Unreachable {
                    detail: describe(&error),
                })?,
        )
        .await
        .map_err(|error| AssetError::Unreachable {
            detail: describe(&error),
        })?;
    Ok(js_sys::Uint8Array::new(&buffer).to_vec())
}

/// What the browser said, as a sentence.
fn describe(error: &wasm_bindgen::JsValue) -> String {
    error
        .as_string()
        .or_else(|| {
            error
                .dyn_ref::<js_sys::Error>()
                .map(|error| String::from(error.message()))
        })
        .unwrap_or_else(|| "the browser gave no reason".to_owned())
}

impl ByteSource for WebSource {
    fn request(&mut self, path: &str, kind: AssetKind) -> RequestId {
        let request = RequestId::from_bits(self.next_request);
        self.next_request += 1;
        self.outstanding += 1;
        let url = super::asset_url(&self.root, path);
        fetch(url, request, kind, Arc::clone(&self.arrived));
        request
    }

    fn drain_completed(&mut self, _tick: u64) -> Vec<Completion> {
        let Ok(mut arrived) = self.arrived.lock() else {
            // A poisoned lock means a fetch callback panicked mid-push. There
            // is nothing to drain and nothing to be done about it here.
            return Vec::new();
        };
        let mut completed: Vec<Completion> = core::mem::take(&mut *arrived)
            .into_iter()
            .map(|fetched| Completion {
                request: fetched.request,
                // Decoded here, at the commit point, on the main thread —
                // acceptable at prototype scale and the reason this is a
                // PERF-revisit rather than a worker (assets.md §5).
                result: fetched.result.and_then(|bytes| match fetched.kind {
                    AssetKind::Bytes => Ok(Payload::Bytes(bytes)),
                    AssetKind::Texture => decode_png(&bytes).map(Payload::Texture),
                }),
            })
            .collect();
        self.outstanding = self.outstanding.saturating_sub(completed.len());

        // CONTRACT (assets.md §5): one poll's completions come back in request
        // order. The network returns them in whatever order it likes — which is
        // exactly the environmental timing this sort keeps off the timeline.
        completed.sort_by_key(|completion| completion.request);
        completed
    }

    fn outstanding(&self) -> usize {
        self.outstanding
    }
}
