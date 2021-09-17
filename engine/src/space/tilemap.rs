use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

pub struct Orientation(pub Direction);

#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub struct Position {
    pub x: i64,
    pub y: i64,
}

pub struct Path(pub Vec<Position>);

#[derive(Default)]
struct PlannedSteps {
    steps: HashMap<Position, bevy::prelude::Entity>,
}

#[derive(Debug, Copy, Clone)]
pub struct Destination(pub Position);

impl PartialEq<Position> for Destination {
    fn eq(&self, other: &Position) -> bool {
        self.0.x == other.x && self.0.y == other.y
    }
}

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
pub enum Direction {
    Up,
    UpLeft,
    UpRight,
    Down,
    DownLeft,
    DownRight, //This is downright.
    Left,
    Right,
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

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    //Make the camera
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    //Make the map
    let handle: Handle<TiledMap> = asset_server.load("maps/test.tmx");

    let map_entity = commands.spawn().id();

    commands.entity(map_entity).insert_bundle(TiledMapBundle {
        tiled_map: handle,
        map: Map::new(0u16, map_entity),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..Default::default()
    });
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