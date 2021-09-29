/*
Draws the world map and handles pathfinding

Drawing:
Uses bevy_ecs_tilemap to draw tiles on screen.
Note: Support for bevy_ecs_tilemap/tiled_map to be deprecated in future

Pathfinding:
Entities with a Position and Destinations component, but without a Path component use this module to generate a path.
Paths are initialized in full using aStar.
Paths are stored in the Path component (a vector of positions) and
Ground Types are used to produce tile weights, which hopefully can encourage aStar to prefer sidewalks over roads.
Note: This may not be deterministic, and needs to be. Consider invoking bevy stages.

*/

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use std::collections::HashMap;
use std::convert::TryInto;

extern crate pathfinding;
use pathfinding::prelude::{absdiff, astar};
pub mod time;

pub struct Path(pub Vec<Position>);

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

pub struct Destination(pub Position);

impl PartialEq<Position> for Destination {
    fn eq(&self, other: &Position) -> bool {
        self.0.x == other.x && self.0.y == other.y
    }
}

#[derive(Debug)]
pub struct Tile {
    pub occupied: bool,
    pub ground_type: GroundType,
}
#[derive(Default)]
pub struct TileMap {
    pub map: HashMap<Position, Tile>,
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
        //Tilemap
        .insert_resource(TileMap {
            map: HashMap::<Position, Tile>::new(),
        })
        //Window
        .insert_resource(WindowDescriptor {
            width: 1270.0,
            height: 720.0,
            title: String::from("game"),
            ..Default::default()
        })
        .add_startup_system(init_tilemaps.system())
        .add_system(plan_path.system())
        .add_plugin(time::TimePlugin);
    }
}

fn init_tilemaps(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    let handle: Handle<TiledMap> = asset_server.load("maps/test.tmx");

    let map_entity = commands.spawn().id();

    commands.entity(map_entity).insert_bundle(TiledMapBundle {
        tiled_map: handle,
        map: Map::new(0u16, map_entity),
        transform: Transform::from_xyz(0.0, 0.0, 0.0)
            .mul_transform(Transform::from_scale(Vec3::splat(4.0))),
        ..Default::default()
    });
}

fn plan_path(
    mut commands: Commands,
    query: Query<(Entity, &Position, &Destination), Without<Path>>,
    tilemap: ResMut<TileMap>,
) {
    for (entity, position, destination) in query.iter() {
        let plan = astar(
            position,
            |p| move_weights(p, &tilemap),
            |p| {
                (absdiff(p.x, destination.0.x) + absdiff(p.y, destination.0.y))
                    .try_into()
                    .unwrap()
            },
            |p| *p == destination.0,
        );
        if let Some(p) = plan {
            let mut path = p.0;
            path.remove(0);
            if path.len() > 0 {
                commands.entity(entity).insert(Path(path));
            }
        }
    }
}

pub fn move_weights(position: &Position, tilemap: &ResMut<TileMap>) -> Vec<(Position, u32)> {
    let &Position { x, y } = position;
    let mut weights = Vec::<(Position, u32)>::new();
    for next_x in [-1, 0, 1].iter() {
        for next_y in [-1, 0, 1].iter() {
            let next_position = Position {
                x: x + next_x,
                y: y + next_y,
            };
            let tile_weight = tile_weight(next_position, tilemap);
            if tile_weight != u32::MAX {
                weights.push((next_position, tile_weight));
            }
        }
    }
    return weights;
}

fn tile_weight(position: Position, tilemap: &ResMut<TileMap>) -> u32 {
    let mut weight = 1;
    if tilemap.map.contains_key(&position) {
        match tilemap.map.get(&position).unwrap() {
            Tile {
                ground_type: GroundType::Obstacle,
                ..
            } => weight = u32::MAX,
            Tile {
                ground_type: GroundType::Street,
                ..
            } => weight = 10,
            _ => weight = 1,
        }
    }
    weight
}