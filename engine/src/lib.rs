/*
The engine is responsible for building and operating the game.
This file should be an interface between the engine and external world building scripts.
*/

use bevy::prelude::*;

pub mod movement;

pub use bevy_ecs_tilemap::prelude::*;

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

#[derive(Default)]
pub struct TileInit{
    pub tiles: Vec<(u16, TilePos)>
}

pub fn spawn_tiles(
    map_query: &mut MapQuery, 
    commands: &mut Commands,
    tile_init: TileInit,
) //Takes a vector of tiles; adds them to the map.
{
    for tile in tile_init.tiles {
        let _ = map_query.set_tile(
            commands,
            tile.1,
            Tile {
                texture_index: tile.0, 
                ..Default::default()
            },
            0u16,
            0u16,
        );
        map_query.notify_chunk_for_tile(tile.1, 0u16, 0u16);
    }
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
