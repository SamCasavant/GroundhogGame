use std::ops::RangeInclusive;

use bevy::prelude::*;
use pretty_trace::PrettyTrace;
use rand::Rng;
mod debug;
mod engine;

fn main() {
    env_logger::init();
    PrettyTrace::new().on();
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugins(engine::GamePlugins)
        .add_plugin(debug::DebugPlugin)
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
            z: 0,
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
