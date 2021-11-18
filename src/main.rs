use std::ops::RangeInclusive;

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use pretty_trace::*;
use rand::Rng;
mod debug;
mod engine;

fn main() {
    PrettyTrace::new().on();
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(debug::DebugPlugin)
        .add_plugin(engine::render::GraphicsPlugin)
        .add_plugin(engine::actor::ActorPlugin)
        .add_plugin(engine::world::WorldPlugin)
        .add_plugin(TilemapPlugin)
        .add_plugin(TiledMapPlugin)
        .add_startup_system(add_people.system())
        .run();
}

fn add_people(
    mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
) {
    let mut x = 0;

    while x < 100 {
        let x_range = RangeInclusive::new(0, 50);
        let y_range = x_range.clone();
        let mut rng = rand::thread_rng();
        let position = engine::world::Position {
            x: rng.gen_range(x_range),
            y: rng.gen_range(y_range),
        };
        let destination = engine::world::Destination(position);
        let inventory = engine::actor::Inventory {
            contents: Vec::new(),
            capacity: 5,
        };

        let sprite_sheet = engine::render::init_sprite_sheet(
            &"sprites/NPC1 (2).png".to_owned(),
            &asset_server,
            &mut texture_atlases,
            position,
        );
        engine::spawn_actor(
            &mut commands,
            engine::Identity {
                specific: true,
                name:     "Grumph Torgi".to_owned(),
            },
            position,
            destination,
            inventory,
            sprite_sheet,
        );
        x += 1;
    }
}
