use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};

mod engine;
fn main() {
    App::build()
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(engine::movement::GraphicsPlugin)
        .add_plugin(engine::movement::MovementPlugin)
        .add_startup_system(add_roads.system())
        .add_startup_system(add_people.system())
        .run();
}

fn add_people(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut textures: ResMut<Assets<Texture>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let position = engine::movement::Position { x: 0, y: 0 };

    let sprite_sheet = engine::movement::init_sprite_sheet(
        &"assets/sprites/NPC1 (2).png".to_string(),
        asset_server,
        texture_atlases,
        position,
    );
    engine::spawn_actor(
        &mut commands,
        engine::Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
}

fn add_roads(mut tilemap: ResMut<engine::movement::TileMap>) {
    for x in 0..4 {
        for y in 0..30 {
            tilemap.map.insert(
                engine::movement::Position { x: x, y: y },
                engine::movement::Tile {
                    occupied: false,
                    ground_type: engine::movement::GroundType::Street,
                },
            );
        }
    }
    for x in 0..4 {
        tilemap.map.insert(
            engine::movement::Position { x: x, y: 15 },
            engine::movement::Tile {
                occupied: false,
                ground_type: engine::movement::GroundType::Crosswalk,
            },
        );
    }
}
