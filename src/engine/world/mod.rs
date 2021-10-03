// Draws the world map and handles pathfinding
//
// Drawing:
// Uses bevy_ecs_tilemap to draw tiles on screen.
// Note: Support for bevy_ecs_tilemap/tiled_map to be deprecated in future
//

use std::collections::HashMap;
use std::ops::Sub;

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

pub mod time;

#[derive(Debug)]
pub enum GroundType {
    ShortGrass,
    TallGrass,
    Sidewalk,
    Path,
    Street,
    Crosswalk,
    Obstacle,
}

#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub struct Position {
    pub x: i64,
    pub y: i64,
}
impl Position {
    pub fn get_range(
        &self,
        x_radius: i64,
        y_radius: i64,
    ) -> Vec<Position> {
        let mut range = Vec::new();
        for x in (self.x - x_radius)..=(self.x + x_radius) {
            for y in (self.y - y_radius)..=(self.y + y_radius) {
                let position = Position { x, y };
                if !(*self == position) {
                    range.push(position)
                }
            }
        }
        range
    }
}
impl Sub for Position {
    type Output = Self;
    fn sub(
        self: Position,
        other: Self,
    ) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

pub struct Destination(pub Position);

impl PartialEq<Position> for Destination {
    fn eq(
        &self,
        other: &Position,
    ) -> bool {
        self.0.x == other.x && self.0.y == other.y
    }
}

#[derive(Default)]
pub struct TileWeightMap {
    // Maps position to weight (i64)
    // i64::MAX is treated as an obstacle
    pub map: HashMap<Position, i64>,
}

pub struct TileEntityMap {
    pub map: HashMap<Position, Option<Entity>>,
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(
        &self,
        app: &mut AppBuilder,
    ) {
        app
            //Tilemap
            .insert_resource(TileWeightMap {
                map: HashMap::<Position, i64>::new(),
            })
            .insert_resource(TileEntityMap {
                map: HashMap::<Position, Option<Entity>>::new()
            })
            //Window
            .insert_resource(WindowDescriptor {
                width: 1270.0,
                height: 720.0,
                title: String::from("game"),
                ..Default::default()
            })
            .add_startup_system(init_tilemaps.system())
            //.add_system(plan_path.system().label("preparation"))
            .add_plugin(time::TimePlugin);
    }
}

fn init_tilemaps(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    let handle: Handle<TiledMap> = asset_server.load("maps/test.tmx");

    let map_entity = commands.spawn().id();

    commands.entity(map_entity).insert_bundle(TiledMapBundle {
        tiled_map: handle,
        map: Map::new(0_u16, map_entity),
        transform: Transform::from_xyz(0.0, 0.0, 0.0)
            .mul_transform(Transform::from_scale(Vec3::splat(4.0))),
        ..Default::default()
    });
}

// pub fn move_weights(
//     position: &Position,
//     tilemap: &ResMut<TileMap>,
// ) -> Vec<(Position, u32)> {
//     let &Position { x, y } = position;
//     let mut weights = Vec::<(Position, u32)>::new();
//     for next_x in &[-1, 0, 1] {
//         for next_y in &[-1, 0, 1] {
//             let next_position = Position {
//                 x: x + next_x,
//                 y: y + next_y,
//             };
//             let tile_weight = tile_weight(next_position, tilemap);
//             if tile_weight != u32::MAX && next_position != *position {
//                 weights.push((next_position, tile_weight));
//             }
//         }
//     }
//     println!("Weights: {:?}", weights);
//     weights
// }

// fn tile_weight(
//     position: Position,
//     tilemap: &ResMut<TileMap>,
// ) -> u32 {
//     let mut weight = 1;
//     if tilemap.map.contains_key(&position) {
//         match tilemap.map[&position] {
//             Tile {
//                 ground_type: GroundType::Obstacle,
//                 ..
//             } => weight = u32::MAX,
//             Tile {
//                 ground_type: GroundType::Street,
//                 ..
//             } => weight = 10,
//             _ => weight = 1,
//         }
//         if tilemap.map[&position].occupied {
//             weight = u32::MAX;
//         }
//     }

//     weight
// }
