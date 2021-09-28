/*
The engine is responsible for building and operating the game.
This file should be an interface between the engine and external world building scripts.
*/

use bevy::prelude::*;
pub mod movement;
pub mod world; // When people run in circles it's a very, very

pub use bevy_ecs_tilemap::prelude::*;

#[derive(Bundle)]
struct ActorComponents {
    spritesheet: SpriteSheetBundle,
    position: world::Position,
    identity: Identity,
    destination: world::Destination,
}

pub fn spawn_actor(
    commands: &mut Commands,
    identity: Identity,
    position: world::Position,
    destination: world::Destination,
    sprite_sheet: SpriteSheetBundle,
) {
    commands
        .spawn()
        .insert(identity)
        .insert(position)
        .insert(movement::movement::Orientation(
            movement::movement::Direction::Down,
        ))
        .insert(destination)
        .insert_bundle(sprite_sheet)
        .insert(Timer::from_seconds(0.1, true));
}

#[derive(Clone)]
pub struct Identity {
    pub specific: bool,
    pub name: String,
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
