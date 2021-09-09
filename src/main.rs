use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    ecs::{archetype::Archetypes, entity::Entities, component::Components}
};

extern crate engine;

fn main() {
    App::build()
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(engine::movement::GraphicsPlugin)
        .add_plugin(engine::movement::pathing::MovementPlugin)
        .add_startup_system(add_roads.system())
        .add_startup_system(add_people.system())
        .add_system(inspect.system())
        .run();
}

fn add_people(
    mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
) {
    let mut x = 0;
    while x < 500 {
        let position = engine::movement::pathing::Position { x: -30, y: 0 };
        let destination = engine::movement::pathing::Destination(engine::movement::pathing::Position{x: 30, y: 0});

        let sprite_sheet = engine::movement::init_sprite_sheet(
            &"sprites/NPC1 (2).png".to_string(),
            &asset_server,
            &mut texture_atlases,
            position,
        );
        engine::spawn_actor(
            &mut commands,
            engine::Identity {
                specific: true,
                name: "Grumph Torgi".to_string(),
            },
            position,
            destination,
            sprite_sheet,
        );
        x += 1;
        println!("Spawing entity {}", x);
    }
    while x < 1000 {
        let position = engine::movement::pathing::Position { x: 30, y: 0 };

        let destination = engine::movement::pathing::Destination(engine::movement::pathing::Position{x: -30, y: 0});

        let sprite_sheet = engine::movement::init_sprite_sheet(
            &"sprites/NPC1 (2).png".to_string(),
            &asset_server,
            &mut texture_atlases,
            position,
        );
        engine::spawn_actor(
            &mut commands,
            engine::Identity {
                specific: true,
                name: "Grumph Torgi".to_string(),
            },
            position,
            destination,
            sprite_sheet,
        );
        x += 1;
        println!("Spawing entity {}", x);
    }
}

fn add_roads(mut tilemap: ResMut<engine::movement::pathing::TileMap>) {
    for x in 0..4 {
        for y in 0..30 {
            tilemap.map.insert(
                engine::movement::pathing::Position { x: x, y: y },
                engine::movement::pathing::Tile {
                    occupied: false,
                    ground_type: engine::movement::pathing::GroundType::Street,
                },
            );
        }
    }
    for x in 0..4 {
        tilemap.map.insert(
            engine::movement::pathing::Position { x: x, y: 15 },
            engine::movement::pathing::Tile {
                occupied: false,
                ground_type: engine::movement::pathing::GroundType::Crosswalk,
            },
        );
    }
}

fn inspect(
    keyboard: Res<Input<KeyCode>>,
    all_entities: Query<Entity>,
    entities: &Entities,
    archetypes: &Archetypes,
    components: &Components,
) {
    if keyboard.just_pressed(KeyCode::F1) {
        for entity in all_entities.iter() {
            println!("Entity: {:?}", entity);
            if let Some(entity_location) = entities.get(entity) {
                if let Some(archetype) = archetypes.get(entity_location.archetype_id) {
                    for component in archetype.components() {
                        if let Some(info) = components.get_info(component) {
                            println!("\tComponent: {}", info.name());
                        }
                    }
                }
            }
        }
    }
}