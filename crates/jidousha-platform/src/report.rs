//! Where the engine's messages go when something goes wrong at run time.
//!
//! Key types: `problem`.
//! Depends on: `web-sys` (web only).
//! INVARIANT: one call site vocabulary, two destinations. Everything above this
//! says "report a problem"; which stream that reaches is the platform's
//! business, which is what this crate is for.
//!
//! This exists because `eprintln!` does nothing on `wasm32-unknown-unknown`.
//! Rust's standard streams have nowhere to go in a browser, so every §9 message
//! the driver printed — a missing asset, a lost surface — was written into a
//! void on the one target where a person is most likely to be debugging by
//! reading. A2 is what made that reachable: mistyping an asset path is the
//! ordinary way a web build fails, and the message explaining it was silent.

/// Report a run-time problem to whoever is watching.
///
/// Not a panic and not a `Result`: these are the §9 messages for things that
/// have already been handled — an asset that will not arrive, a frame that
/// could not be drawn — where the program carries on and a person needs to know
/// (core.md §9).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn problem(message: &str) {
    eprintln!("{message}");
}

/// Report a run-time problem to the browser console.
///
/// `console.error` rather than `console.log`, so it keeps the red styling and
/// the stack-trace affordance a developer expects, and so a page filtering its
/// console still shows it.
#[cfg(target_arch = "wasm32")]
pub(crate) fn problem(message: &str) {
    web_sys::console::error_1(&wasm_bindgen::JsValue::from_str(message));
}
