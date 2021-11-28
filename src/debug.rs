#![allow(dead_code)]
#![allow(unused_imports)]
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::ecs::{archetype::Archetypes, component::Components, entity::Entities};
use bevy::{app::AppExit, prelude::*, render::render_graph::RenderGraph};
use bevy_mod_debug_console::ConsoleDebugPlugin;
use bevy_mod_debugdump::schedule_graph::schedule_graph_dot;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(
        &self,
        app: &mut AppBuilder,
    ) {
        app.add_plugin(FrameTimeDiagnosticsPlugin::default()) //Todo: this broke when I added env_logger
            .add_plugin(ConsoleDebugPlugin);
        // app.set_runner(bevy_mod_debugdump::print_schedule_runner);
        // Uncomment^ to output dotfile of system schedule
    }
}
