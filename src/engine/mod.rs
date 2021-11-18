// The engine is responsible for building and operating the game.
// This file should be an interface between the engine and external world
// building scripts.

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;
pub mod actor;
pub mod render;
pub mod ui;
// When pub people run in pub circles it's a very, very
pub mod world;

pub use bevy_ecs_tilemap::prelude::*;

pub struct GamePlugins;

impl PluginGroup for GamePlugins {
    fn build(
        &mut self,
        group: &mut PluginGroupBuilder,
    ) {
        group
            .add(ui::UIPlugin)
            .add(render::GraphicsPlugin)
            .add(actor::ActorPlugin)
            .add(world::WorldPlugin);
    }
}

#[derive(Bundle)]
struct ActorComponents {
    spritesheet: SpriteSheetBundle,
    position:    world::Position,
    identity:    Identity,
    destination: world::Destination,
}

pub fn spawn_actor(
    commands: &mut Commands,
    identity: Identity,
    position: world::Position,
    destination: world::Destination,
    inventory: actor::Inventory,
    sprite_sheet: SpriteSheetBundle,
) {
    commands
        .spawn()
        .insert(identity)
        .insert(position)
        .insert(actor::Orientation(actor::Direction::Down))
        .insert(destination)
        .insert(inventory)
        .insert(actor::Status {
            hunger:   100,
            thirst:   0,
            laziness: 10,
        })
        .insert(actor::Animal)
        .insert(actor::Intelligent)
        .insert_bundle(sprite_sheet)
        .insert(world::time::GameTime::from_stamp(&world::time::Stamp {
            day:    0,
            hour:   6,
            minute: 0,
            second: 0,
            frame:  0,
        }));
}

#[derive(Clone)]
pub struct Identity {
    pub specific: bool,
    pub name:     String,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
