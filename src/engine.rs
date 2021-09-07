use bevy::{prelude::*, sprite::SpriteSettings};

use std::collections::{btree_map::Range, HashMap};

pub mod movement;

#[derive(Bundle)]
struct ActorComponents {
    spritesheet: SpriteSheetBundle,
    position: movement::Position,
    identity: Identity,
    destination: movement::Destination,
}

pub fn spawn_actor(
    commands: &mut Commands,
    identity: Identity,
    position: movement::Position,
    sprite_sheet: SpriteSheetBundle,
) {
    commands
        .spawn()
        .insert(identity)
        .insert(position)
        .insert(movement::Path(vec![]))
        .insert(movement::Orientation(movement::Direction::Down))
        .insert(movement::Destination(movement::Position { x: 0, y: 0 }))
        .insert_bundle(sprite_sheet)
        .insert(Timer::from_seconds(0.05, true));
}

#[derive(Clone)]
pub struct Identity {
    pub specific: bool,
    pub name: String,
}

// commands
// .spawn_bundle(SpriteSheetBundle {
//     texture_atlas: texture_atlas_handle,
//     transform: Transform::from_scale(Vec3::splat(6.0)),
//     ..Default::default()
// }).get;
// fn init_sprite_sheet(
//     path: String,
//     asset_server: &Res<AssetServer>,
//     mut textures: &mut ResMut<Assets<Texture>>,
//     texture_atlases: &mut ResMut<Assets<TextureAtlas>>,
//     position: Position,
// ) -> SpriteSheetComponents {

//     let texture_handle = asset_server.load_sync(&mut textures, path).unwrap();
//     let texture = textures.get(&texture_handle).unwrap();
//     let texture_atlas = TextureAtlas::from_grid(texture_handle, texture.size, 4, 4);
//     let texture_atlas_handle = texture_atlases.add(texture_atlas);
//     let mut transform = Transform::from_scale(0.1);
//     let translation = Transform::from_xyz(
//         position.x as f32 * TILE_WIDTH,
//         position.y as f32 * TILE_WIDTH,
//         0.0,
//     );
//     SpriteSheetComponents {
//         texture_atlas: texture_atlas_handle,
//         transform: transform,
//         ..Default::default()
//     }
// }
