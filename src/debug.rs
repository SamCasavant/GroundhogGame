use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::ecs::{archetype::Archetypes, component::Components, entity::Entities};
use bevy::prelude::*;
use bevy_mod_debug_console::ConsoleDebugPlugin;

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(
        &self,
        app: &mut AppBuilder,
    ) {
        app.add_plugin(FrameTimeDiagnosticsPlugin::default())
            .add_plugin(ConsoleDebugPlugin);
    }
}
