/*
This module builds and draws the world map and the sprites within it.
The role of this module and its name will likely change during restructuring process.
*/

pub(crate) use bevy::prelude::*;
use bevy::render::draw::OutsideFrustum;
use bevy_ecs_tilemap::prelude::*;
use rand::{thread_rng, Rng};

use std::collections::HashMap;

use crate::world;
mod camera_movement;
pub mod pathing;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(animate_sprite_system.system())
            .add_startup_system(setup.system())
            .add_system(camera_movement::camera_movement.system());
        //.add_system(update_map.system());
    }
}

#[derive(Default)]
struct LastUpdate {
    value: f64,
}

const TILE_WIDTH: f32 = 64.0;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut map_query: MapQuery,
) {
    //Make the camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}
//     let texture_handle = asset_server.load("maps/roguelikeCity_magenta.png");
//     let material_handle = materials.add(ColorMaterial::texture(texture_handle));

//     let map_entity = commands.spawn().id();
//     let mut map = Map::new(0u16, map_entity);

//     let mut layer_settings = LayerSettings::new(
//         MapSize(4, 4),
//         ChunkSize(32, 32),
//         TileSize(16.0, 16.0),
//         TextureSize(628.0, 475.0),
//     );
//     layer_settings.tile_spacing = Vec2::new(1.0, 1.0);

//     let (mut layer_builder, layer_entity) =
//         LayerBuilder::<TileBundle>::new(&mut commands, layer_settings, 0u16, 0u16, None);

//     let mut tile_bundle = TileBundle::default();
//     tile_bundle.tile.texture_index = 963u16;

//     layer_builder.set_all(tile_bundle);
//     layer_builder = build_vertical_road(
//         layer_builder,
//         TilePos(0, 0),
//         4,
//         100,
//         true,
//         Tile {
//             texture_index: 788u16,
//             ..Default::default()
//         },
//         Tile {
//             texture_index: 752u16,
//             ..Default::default()
//         },
//         Tile {
//             texture_index: 753u16,
//             ..Default::default()
//         },
//         Tile {
//             texture_index: 749u16,
//             ..Default::default()
//         },
//         Tile {
//             texture_index: 714u16,
//             ..Default::default()
//         },
//         Tile {
//             texture_index: 22u16,
//             ..Default::default()
//         },
//     );

//     map_query.build_layer(&mut commands, layer_builder, material_handle);

//     commands.entity(layer_entity).insert(LastUpdate::default());

//     map.add_layer(&mut commands, 0u16, layer_entity);

//     commands
//         .entity(map_entity)
//         .insert(map)
//         .insert(
//             Transform::from_xyz(-128.0, -128.0, 0.0)
//                 .mul_transform(Transform::from_scale(Vec3::splat(TILE_WIDTH / 16.0))),
//         )
//         .insert(GlobalTransform::default());
// }

// fn build_vertical_road(
//     mut layer_builder: LayerBuilder<TileBundle>,
//     mut origin: TilePos,
//     lanes: u32, //must be even if greater than 1
//     length: u32,
//     sidewalks: bool,
//     left_edge_tile: Tile,
//     right_edge_tile: Tile,
//     center_tile: Tile,         //Separates two directions of traffic
//     lane_separator_tile: Tile, //Separates two lanes in the same direction
//     lane_tile: Tile,
//     sidewalk_tile: Tile,
// ) -> LayerBuilder<TileBundle> {
//     let mut remaining_width = 2 // edges
//         + sidewalks as u32 * 2 // sidewalks are always added on both sides for now
//         + lanes * 2 - 1
//         -1 ; // each lane adds a line of center_tile and a line of lane_tile except the last
//     println!("Remaining width: {}", remaining_width);
//     if sidewalks {
//         layer_builder.fill(
//             origin,
//             TilePos(origin.0 + 1, origin.1 + length),
//             sidewalk_tile.into(),
//         );
//         layer_builder.fill(
//             TilePos(origin.0 + remaining_width, origin.1),
//             TilePos(origin.0 + remaining_width + 1, origin.1 + length),
//             sidewalk_tile.into(),
//         );
//         origin = TilePos(origin.0 + 1, origin.1);
//         remaining_width -= 2;
//     }
//     if lanes < 1 {
//         return layer_builder;
//     }
//     println!("Remaining width: {}", remaining_width);
//     //First build the edges
//     layer_builder.fill(
//         origin,
//         TilePos(origin.0 + 1, origin.1 + length),
//         left_edge_tile.into(),
//     );
//     layer_builder.fill(
//         TilePos(origin.0 + remaining_width, origin.1),
//         TilePos(origin.0 + remaining_width + 1, origin.1 + length),
//         right_edge_tile.into(),
//     );
//     origin = TilePos(origin.0 + 1, origin.1);
//     remaining_width -= 2;

//     //Then build the lanes
//     if lanes == 1 {
//         layer_builder.fill(
//             origin,
//             TilePos(origin.0 + 1, origin.1 + length),
//             lane_tile.into(),
//         );
//         return layer_builder;
//     } else {
//         let center_position = origin.0 + remaining_width / 2;
//         for each in 0..lanes {
//             layer_builder.fill(
//                 origin,
//                 TilePos(origin.0 + 1, origin.1 + length),
//                 lane_tile.into(),
//             );
//             origin = TilePos(origin.0 + 1, origin.1);
//             if origin.0 == center_position { // middle lane
//                 layer_builder.fill(
//                     TilePos(center_position, origin.1),
//                     TilePos(origin.0 + 1, origin.1 + length),
//                     center_tile.into(),
//                 );
//                 origin = TilePos(origin.0 + 1, origin.1);
//             }
//             else if each == lanes - 1 { // last lane
//                 continue
//             }
//             else {
//                 layer_builder.fill(
//                     origin,
//                     TilePos(origin.0 + 1, origin.1 + length),
//                     lane_separator_tile.into(),
//                 );
//                 origin = TilePos(origin.0 + 1, origin.1);
//             }
//         }
//     }
//     origin = TilePos(origin.0 + 1, origin.1);

//     return layer_builder;
//}

// fn build_map(map_query: &mut MapQuery, commands: &mut Commands) {
//     let mut random = thread_rng();
//     for _ in 0..100 {
//         let position = TilePos(random.gen_range(0..16), random.gen_range(0..16));
//         let _ = map_query.set_tile(
//             commands,
//             position,
//             Tile {
//                 texture_index: 0,
//                 ..Default::default()
//             },
//             0u16,
//             0u16,
//         );
//         map_query.notify_chunk_for_tile(position, 0u16, 0u16);
//     }
// }

// fn update_map(
//     time: ResMut<Time>,
//     mut commands: Commands,
//     mut query: Query<&mut LastUpdate>,
//     mut map_query: MapQuery,
// ) {
//     let current_time = time.seconds_since_startup();
//     for mut last_update in query.iter_mut() {
//         if (current_time - last_update.value) > 1.0 {
//             map_query.despawn_layer_tiles(&mut commands, 0u16, 0u16);
//             build_map(&mut map_query, &mut commands);
//             last_update.value = current_time;
//         }
//     }
// }

fn animate_sprite_system(
    mut query: Query<(
        &mut TextureAtlasSprite,
        &mut Transform,
        &pathing::Orientation,
        &world::Position,
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
            position.x as f32 * TILE_WIDTH + TILE_WIDTH / 2.0,
            position.y as f32 * TILE_WIDTH + TILE_WIDTH / 2.0,
            1.0, //Layer
        );
        transform.translation = translation;
    }
}

pub fn init_sprite_sheet(
    path: &str,
    asset_server: &Res<AssetServer>,
    texture_atlases: &mut ResMut<Assets<TextureAtlas>>,
    position: world::Position,
) -> SpriteSheetBundle {
    let texture_handle = asset_server.load(path);
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(4.0, 4.0), 4, 4);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    let translation = Vec3::new(
        position.x as f32 * (TILE_WIDTH) + TILE_WIDTH / 2.0,
        position.y as f32 * (TILE_WIDTH) + TILE_WIDTH / 2.0,
        -1.0, //Layer
    );
    let mut transform = Transform::from_scale(Vec3::splat(TILE_WIDTH / 3.0));
    transform.translation = translation;
    SpriteSheetBundle {
        texture_atlas: texture_atlas_handle,
        transform: transform,
        ..Default::default()
    }
}
