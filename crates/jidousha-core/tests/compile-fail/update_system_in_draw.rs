//! Registering an Update-shaped function in Draw is a compile error: phases
//! name their signatures (ADR-0008).
use jidousha_core::{Draw, GameConfig, World, headless};

fn physics(_world: &mut World) {}

fn main() {
    let _ = headless(GameConfig::default(), |app| {
        app.add_system(Draw, physics);
    });
}
