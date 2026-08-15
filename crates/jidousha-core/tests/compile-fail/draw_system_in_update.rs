//! And the other way round: a Draw-shaped function cannot run in Update.
use jidousha_core::{DrawCtx, GameConfig, Update, headless};

fn draw_sprites(_ctx: &mut DrawCtx) {}

fn main() {
    let _ = headless(GameConfig::default(), |app| {
        app.add_system(Update, draw_sprites);
    });
}
