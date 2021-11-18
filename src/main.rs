use std::ops::RangeInclusive;

use bevy::{diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
           ecs::{archetype::Archetypes, component::Components,
                 entity::Entities},
           prelude::*};
use bevy_console::*;
use bevy_ecs_tilemap::prelude::*;
use bevy_egui::EguiPlugin;
use pretty_trace::*;
use rand::Rng;

mod engine;

use bevy_mod_debug_console::{build_commands, match_commands, Pause};

fn main() {
    PrettyTrace::new().on();
    App::build()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.05)))
        .add_plugin(ConsolePlugin)
        .insert_resource(ConsoleConfiguration {
            // override config here
            ..Default::default()
        })
        .insert_resource(Pause(false))
        .add_system(debug_console.system())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(engine::render::GraphicsPlugin)
        .add_plugin(engine::actor::ActorPlugin)
        .add_plugin(engine::world::WorldPlugin)
        .add_plugin(TilemapPlugin)
        .add_plugin(TiledMapPlugin)
        .add_startup_system(add_people.system())
        .run();
}

fn debug_console(
    mut console_events: EventReader<ConsoleCommandEntered>,
    mut console_line: EventWriter<PrintConsoleLine>,
    a: &Archetypes,
    c: &Components,
    e: &Entities,
    mut pause: ResMut<Pause>,
    reflect: Res<bevy::reflect::TypeRegistry>,
) {
    let app_name = "";
    for event in console_events.iter() {
        let console_app = build_commands(app_name);
        let mut args = vec![app_name];
        args.push(&event.command);
        let split = event.args.split_whitespace();
        args.append(&mut split.collect());
        let matches_result = console_app.try_get_matches_from(args);

        if let Err(e) = matches_result {
            console_line.send(PrintConsoleLine::new(e.to_string()));
            return;
        }

        let output = match_commands(
            &matches_result.unwrap(),
            a,
            c,
            e,
            &mut pause,
            &*reflect,
        );

        console_line.send(PrintConsoleLine::new(output));
    }
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
