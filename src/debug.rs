use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::ecs::{archetype::Archetypes, component::Components, entity::Entities};
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy_console::{ConsoleCommandEntered, ConsoleConfiguration, ConsolePlugin,
                   PrintConsoleLine};
use bevy_mod_debug_console::{build_commands, match_commands, Pause};

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(
        &self,
        app: &mut AppBuilder,
    ) {
        app.add_plugin(ConsolePlugin)
            .insert_resource(ConsoleConfiguration {
                // override config here
                ..Default::default()
            })
            .add_plugin(FrameTimeDiagnosticsPlugin::default())
            .add_plugin(LogPlugin::default())
            .insert_resource(Pause(false))
            .add_system(debug_console.system());
    }
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
