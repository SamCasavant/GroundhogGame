use std::ops::RangeInclusive;

use bevy::{diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
           ecs::{archetype::Archetypes, component::Components,
                 entity::Entities},
           prelude::*};
use bevy_ecs_tilemap::prelude::*;
use pretty_trace::*;
use rand::Rng;

mod engine;

fn main() {
    PrettyTrace::new().on();
    App::build()
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.05)))
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(engine::render::GraphicsPlugin)
        .add_plugin(engine::actor::ActorPlugin)
        .add_plugin(engine::world::WorldPlugin)
        .add_plugin(TilemapPlugin)
        .add_plugin(TiledMapPlugin)
        .add_startup_system(add_people.system())
        .add_system(inspect.system())
        .add_system(new_destination.system())
        .run();
}

fn add_people(
    mut commands: Commands,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    asset_server: Res<AssetServer>,
) {
    let mut x = 0;

    while x < 50 {
        let xrange = RangeInclusive::new(0, 199);
        let yrange = xrange.clone();
        let mut rng = rand::thread_rng();
        let position = engine::world::Position {
            x: rng.gen_range(xrange),
            y: rng.gen_range(yrange),
        };
        let destination = engine::world::Destination(engine::world::Position {
            x: 199,
            y: 199,
        });

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
            sprite_sheet,
        );
        x += 1;
    }
    while x < 100 {
        let xrange = RangeInclusive::new(0, 199);
        let yrange = xrange.clone();
        let mut rng = rand::thread_rng();
        let position = engine::world::Position {
            x: rng.gen_range(xrange),
            y: rng.gen_range(yrange),
        };

        let destination =
            engine::world::Destination(engine::world::Position { x: 0, y: 0 });

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
            sprite_sheet,
        );
        x += 1;
    }
}

// This should be moved to engine::actor module
fn new_destination(
    mut commands: Commands,
    query: Query<
        Entity,
        (
            With<engine::world::Position>,
            Without<engine::world::Destination>,
        ),
    >,
) {
    for entity in query.iter() {
        let xrange = RangeInclusive::new(0, 199);
        let yrange = xrange.clone();
        let mut rng = rand::thread_rng();
        let x = rng.gen_range(xrange);
        let y = rng.gen_range(yrange);
        let destination =
            engine::world::Destination(engine::world::Position { x, y });
        commands.entity(entity).insert(destination);
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
                if let Some(archetype) =
                    archetypes.get(entity_location.archetype_id)
                {
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
