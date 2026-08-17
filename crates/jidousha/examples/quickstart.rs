//! A whole game, in one file. Copy it and start changing things.
//!
//! Move with WASD, touch the coin, watch the score go up. No art files: every
//! shape here is drawn by the engine, so this runs the moment you paste it.
//!
//! Run it: `cargo run -p jidousha --example quickstart`

use jidousha::prelude::*;

/// How far the player moves in one tick, in world units.
const SPEED: f32 = 0.35;
/// How close counts as touching.
const REACH: f32 = 1.2;
/// How far from the centre a coin may appear.
const FIELD: f32 = 8.0;

#[derive(Clone, Copy)]
struct Player;
impl Component for Player {}

#[derive(Clone, Copy)]
struct Coin;
impl Component for Coin {}

/// The score. A resource is a thing there is exactly one of.
#[derive(Default)]
struct Score(u32);
impl Resource for Score {}

fn main() -> Result<(), RunError> {
    run(GameConfig::default(), |app| {
        app.add_system(Startup, spawn_the_world);
        app.add_system(Update, walk);
        app.add_system(Update, collect);
        app.add_system(Draw, draw_everything);
    })
}

fn spawn_the_world(world: &mut World) {
    world.insert_resource(Score::default());
    let player = world.spawn();
    world.insert(player, Transform::at(Vec2::ZERO));
    world.insert(player, Player);
    let coin = world.spawn();
    world.insert(coin, Transform::at(Vec2::new(4.0, -2.0)));
    world.insert(coin, Coin);
}

/// WASD. Input is one value per tick, so a game only ever asks what is true now.
fn walk(world: &mut World) {
    let Some(input) = world.find_resource::<Input>() else {
        return;
    };
    let step = Vec2::new(
        f32::from(input.held(Key::D)) - f32::from(input.held(Key::A)),
        f32::from(input.held(Key::S)) - f32::from(input.held(Key::W)),
    );
    for (_, transform, _) in world.query_mut::<(&mut Transform, &Player)>() {
        transform.pos += step * SPEED;
    }
}

/// Touch the coin, score a point, and move the coin somewhere new.
fn collect(world: &mut World) {
    let Some((_, player, _)) = world.query::<(&Transform, &Player)>().next() else {
        return;
    };
    let player = player.pos;
    let hit = world
        .query::<(&Transform, &Coin)>()
        .find(|(_, coin, _)| (coin.pos - player).length() < REACH)
        .map(|(entity, _, _)| entity);
    let Some(coin) = hit else { return };

    // The engine's RNG is seeded from `GameConfig`, so the same run makes the
    // same game every time — which is what lets a test replay it.
    let rng = world.resource_mut::<Rng>();
    let (x, y) = (rng.next_f32(), rng.next_f32());
    world.component_mut::<Transform>(coin).pos =
        Vec2::new((x - 0.5) * 2.0 * FIELD, (y - 0.5) * 2.0 * FIELD);
    world.resource_mut::<Score>().0 += 1;
}

/// Draw systems take a `DrawCtx` and cannot change the world: the type says so.
fn draw_everything(ctx: &mut DrawCtx) {
    for (_, transform, _) in ctx.world.query::<(&Transform, &Player)>() {
        ctx.rect(
            Rect::from_center_size(transform.pos, Vec2::splat(1.0)),
            Color::rgb(0.4, 0.9, 1.0),
            Depth::default(),
        );
    }
    for (_, transform, _) in ctx.world.query::<(&Transform, &Coin)>() {
        ctx.circle(
            transform.pos,
            0.5,
            Color::rgb(1.0, 0.85, 0.2),
            Depth::default(),
        );
    }
    let score = ctx.world.resource::<Score>().0;
    ctx.text(
        Vec2::new(-15.0, -9.0),
        &format!("score {score}"),
        TextStyle::default(),
    );
}
