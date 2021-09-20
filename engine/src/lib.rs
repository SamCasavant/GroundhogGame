/*
The engine is responsible for building and operating the game.
This file should be an interface between the engine and external world building scripts.
*/

use bevy::prelude::*;

pub mod movement;

#[derive(Bundle)]
struct ActorComponents {
    spritesheet: SpriteSheetBundle,
    position: movement::pathing::Position,
    identity: Identity,
    destination: movement::pathing::Destination,
}

pub fn spawn_actor(
    commands: &mut Commands,
    identity: Identity,
    position: movement::pathing::Position,
    destination: movement::pathing::Destination,
    sprite_sheet: SpriteSheetBundle,
) {
    commands
        .spawn()
        .insert(identity)
        .insert(position)
        .insert(movement::pathing::Path(vec![]))
        .insert(movement::pathing::Orientation(
            movement::pathing::Direction::Down,
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
