/*
This module builds and draws the world map and the sprites within it.
The role of this module and its name will likely change during restructuring process.
*/

pub(crate) use bevy::prelude::*;
//use bevy_ecs_tilemap::prelude::*;
use bevy::render::draw::OutsideFrustum;

use std::collections::HashMap;

mod camera_movement;
pub mod pathing;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(pathing::TileMap {
            map: HashMap::<pathing::Position, pathing::Tile>::new(),
        })
        .insert_resource(WindowDescriptor {
            width: 1270.0,
            height: 720.0,
            title: String::from("game"),
            ..Default::default()
        })
        //.add_plugin(TilemapPlugin)
        //.add_plugin(TiledMapPlugin)
        .add_system(animate_sprite_system.system())
        .add_startup_system(setup.system())
        .add_system(camera_movement::camera_movement.system());
    }
}

const TILE_WIDTH: f32 = 16.0;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    //Make the camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

// pub fn camera_view_check(
//     camera_query: Query<(&Transform, &OrthographicProjection), With<Camera>>,
//     mut visible_query: Query<(&Transform, &mut Visible)>,
// ) {
//     const MAX_TILE_SIZE: f32 = 50.;
//     for (camera_transform, projection) in camera_query.iter() {
//         let camera_pos = camera_transform.translation;
//         let scale = projection.scale;
//         let left = projection.left * scale + camera_pos.x - MAX_TILE_SIZE;
//         let right = projection.right * scale + camera_pos.x + MAX_TILE_SIZE;
//         let bottom = projection.bottom * scale + camera_pos.y - MAX_TILE_SIZE;
//         let top = projection.top * scale + camera_pos.y + MAX_TILE_SIZE;

//         for (transform, mut visible) in visible_query.iter_mut() {
//             let pos = transform.translation;
//             visible.is_visible =
//                 pos.x > left && pos.x < right &&
//                 pos.y > bottom && pos.y < top;
//         }
//     }
// }
fn animate_sprite_system(
    mut query: Query<(
        &mut TextureAtlasSprite,
        &mut Transform,
        &pathing::Orientation,
        &pathing::Position,
        Without<OutsideFrustum>,
    )>,
) {
    for (mut sprite, mut transform, orientation, position, _) in &mut query.iter_mut() {
        // Set sprite to match orientation
        match orientation.0 {
            pathing::Direction::Up => sprite.index = 5,
            pathing::Direction::Down => sprite.index = 1,
            pathing::Direction::Left => sprite.index = 10,
            pathing::Direction::Right => sprite.index = 13,
            pathing::Direction::UpLeft => todo!(),
            pathing::Direction::UpRight => todo!(),
            pathing::Direction::DownLeft => todo!(),
            pathing::Direction::DownRight => todo!(),
        }
        //Move sprite to match position
        let translation = Vec3::new(
            position.x as f32 * TILE_WIDTH,
            position.y as f32 * TILE_WIDTH,
            0.0,
        );
        transform.translation = translation;
    }
}

pub fn init_sprite_sheet(
    path: &str,
    asset_server: &Res<AssetServer>,
    texture_atlases: &mut ResMut<Assets<TextureAtlas>>,
    position: pathing::Position,
) -> SpriteSheetBundle {
    let texture_handle = asset_server.load(path);
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(4.0, 4.0), 4, 4);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    let translation = Vec3::new(
        position.x as f32 * TILE_WIDTH,
        position.y as f32 * TILE_WIDTH,
        0.0,
    );
    let mut transform = Transform::from_scale(Vec3::splat(6.0));
    transform.translation = translation;
    SpriteSheetBundle {
        texture_atlas: texture_atlas_handle,
        transform: transform,
        ..Default::default()
    }
}
