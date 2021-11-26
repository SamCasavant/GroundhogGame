use std::ops::RangeInclusive;

use bevy::prelude::*;
use bevy_asset_loader::{AssetCollection, AssetLoader};
use pretty_trace::PrettyTrace;
use rand::Rng;

use crate::engine::asset_collections::*;
mod debug;
mod engine;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum AppState {
    LoadingAssets,
    BuildingWorld,
    InGame,
}
fn main() {
    env_logger::init();
    PrettyTrace::new().on();
    let mut app = App::build();
    AssetLoader::new(AppState::LoadingAssets, AppState::BuildingWorld)
        .with_collection::<TextureAssets>()
        .build(&mut app);
    app.add_plugins(DefaultPlugins)
        .add_state(AppState::LoadingAssets)
        .add_system_set(
            SystemSet::on_enter(AppState::BuildingWorld)
                .with_system(print_loading_complete.system())
                .with_system(engine::render::voxel::build.system()),
        )
        .add_plugins(engine::GamePlugins)
        .add_plugin(debug::DebugPlugin)
        .add_startup_system(add_people.system())
        .run();
}

fn print_loading_complete() {
    println!("Loading complete...?");
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
