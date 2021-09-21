/*
This module builds and draws the world map and the sprites within it.
The role of this module and its name will likely change during restructuring process.
*/

pub(crate) use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use rand::{thread_rng, Rng};
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
        .add_plugin(TilemapPlugin)
        .add_plugin(TiledMapPlugin)
        .add_system(animate_sprite_system.system())
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
    mut map_query: MapQuery
) {
    //Make the camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    let texture_handle = asset_server.load("maps/roguelikeCity_magenta.png");
    let material_handle = materials.add(ColorMaterial::texture(texture_handle));

    let map_entity = commands.spawn().id();
    let mut map = Map::new(0u16, map_entity);

    let mut layer_settings = LayerSettings::new(
        MapSize(2, 2),
        ChunkSize(1, 1),
        TileSize(16.0, 16.0),
        TextureSize(592.0, 448.0),
    );
    let (mut layer_builder, layer_entity) = LayerBuilder::<TileBundle>::new(
        &mut commands,
        layer_settings,
        0u16,
        0u16,
        None,
    );

    let mut tile_bundle = TileBundle::default();
    tile_bundle.tile.texture_index = 963u16;

    layer_builder.set_all(tile_bundle);

    map_query.build_layer(&mut commands, layer_builder, material_handle);

    commands.entity(layer_entity).insert(LastUpdate::default());

    map.add_layer(&mut commands, 0u16, layer_entity);

    commands
        .entity(map_entity)
        .insert(map)
        .insert(Transform::from_xyz(-128.0, -128.0, 0.0).mul_transform(Transform::from_scale(Vec3::splat(TILE_WIDTH/16.0))))
        .insert(GlobalTransform::default());
}

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
            position.x as f32 * TILE_WIDTH + TILE_WIDTH/2.0,
            position.y as f32 * TILE_WIDTH + TILE_WIDTH/2.0,
            1.0, //Layer
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
    let texture_atlas = TextureAtlas::from_grid(
        texture_handle, 
        Vec2::new(4.0, 4.0), 
        4, 
        4,
    );
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    
    let translation = Vec3::new(
        position.x as f32 * (TILE_WIDTH) + TILE_WIDTH/2.0,
        position.y as f32 * (TILE_WIDTH) + TILE_WIDTH/2.0,
        1.0, //Layer
    );
    let mut transform = Transform::from_scale(Vec3::splat(TILE_WIDTH / 3.0));
    transform.translation = translation;
    SpriteSheetBundle {
        texture_atlas: texture_atlas_handle,
        transform: transform,
        ..Default::default()
    }
}
