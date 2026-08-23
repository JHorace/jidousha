//! `World::view` is a door into the read-only world, not around it: a `&mut T`
//! query through a view is the same compile error it is through a Draw
//! context, so the shared projection cannot smuggle a write (ADR-0008,
//! ADR-0039).
use jidousha_core::{Component, World};

struct Position(i32);
impl Component for Position {}

fn shove_everything(world: &mut World) {
    for (_entity, position) in world.view().query::<&mut Position>() {
        position.0 += 1;
    }
}

fn main() {
    let _ = shove_everything;
}
