//! A Draw system may not take `&mut T` in a query: Draw runs per rendered
//! frame, so writing there would make the simulation depend on frame rate
//! (ADR-0008).
use jidousha_core::{Component, DrawCtx};

struct Position(i32);
impl Component for Position {}

fn shove_everything(ctx: &mut DrawCtx) {
    for (_entity, position) in ctx.world.query::<&mut Position>() {
        position.0 += 1;
    }
}

fn main() {
    let _ = shove_everything;
}
