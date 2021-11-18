// Draws the world map and tracks positions
//
// Drawing:
// Uses bevy_ecs_tilemap to draw tiles on screen.
// Note: Support for bevy_ecs_tilemap/tiled_map to be deprecated in future
//

use std::cmp::min;
use std::ops::Sub;

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use pathfinding::prelude::absdiff;

pub mod item;
pub mod time;

#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub struct Position {
    pub x: i64,
    pub y: i64,
}
impl Position {
    pub fn neighbors(
        &self,
        radius: i64,
    ) -> Vec<Self> {
        let mut neighbors = Vec::new();
        for x in (self.x - radius)..=(self.x + radius) {
            for y in (self.y - radius)..=(self.y + radius) {
                let position = Position { x, y };
                if !(*self == position) {
                    neighbors.push(position)
                }
            }
        }
        neighbors
    }
    pub fn side_neighbors(&self) -> Vec<Self> {
        let mut neighbors = Vec::new();
        for (step_x, step_y) in &[(0, 1), (0, -1), (1, 0), (-1, 0)] {
            neighbors.push(Position {
                x: self.x + step_x,
                y: self.y + step_y,
            });
        }
        neighbors
    }
    pub fn corner_neighbors(&self) -> Vec<Self> {
        let mut neighbors = Vec::new();
        for (step_x, step_y) in &[(1, 1), (-1, -1), (1, -1), (-1, 1)] {
            neighbors.push(Position {
                x: self.x + step_x,
                y: self.y + step_y,
            });
        }
        neighbors
    }
    pub fn diagonal_distance(
        &self,
        other: &Position,
    ) -> i64 {
        let distance_mult = 1_i64;
        let distance_mult_two = 1_i64;
        let dx = absdiff(self.x, other.x);
        let dy = absdiff(self.y, other.y);
        distance_mult * (dx + dy)
            + (distance_mult_two - 2 * distance_mult) * min(dx, dy)
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
    pub map: Vec<i64>, /* Should this be transitioned to fixed size array,
                        * because we know the size at compile time? */
    width:   i64,
    height:  i64,
    /* Maps position to weight (i64)
     * i64::MAX is treated as an obstacle */
}
impl TileWeightMap {
    // consider morton encoding if this is slow
    pub fn new(
        width: i64,
        height: i64,
    ) -> Self {
        let map = vec![0; (width * height) as usize];
        Self { map, width, height }
    }
    pub fn get(
        &self,
        x: i64,
        y: i64,
    ) -> i64 {
        if 0 <= x && x < self.width && 0 <= y && y < self.height {
            let index = (y * self.width + x) as usize;
            self.map[index]
        } else {
            i64::MAX
        }
    }
    pub fn set(
        &mut self,
        x: i64,
        y: i64,
        weight: i64,
    ) {
        if 0 <= x && x < self.width && 0 <= y && y < self.height {
            let index = (y * self.width + x) as usize;
            self.map[index] = weight;
        } else {
            panic!("Writing weight to tile outside of map.")
        }
    }
}

pub struct TileEntityMap {
    pub map: Vec<Option<Entity>>, /* Should this be converted to fixed
                                   * size array
                                   * * because we know the size at compile
                                   *   time? */
    width:   i64,
    height:  i64,
}
impl TileEntityMap {
    // Consider morton encoding if this is slow
    pub fn new(
        width: i64,
        height: i64,
    ) -> Self {
        let map = vec![None; (width * height) as usize];
        Self { map, width, height }
    }
    pub fn get(
        &self,
        x: i64,
        y: i64,
    ) -> Option<Entity> {
        if 0 <= x && x < self.width && 0 <= y && y < self.height {
            let index = (y * self.width + x) as usize;
            self.map[index]
        } else {
            None
        }
    }
    pub fn set(
        &mut self,
        x: i64,
        y: i64,
        entity: Option<Entity>,
    ) {
        //===TODO===//
        // This check should be removed when pathfinding conflicts are resolved.
        if self.get(x, y).is_some() && entity.is_some() {
            panic!("TILE CONFLICT")
        }
        //==========//
        if 0 <= x && x < self.width && 0 <= y && y < self.height {
            let index = (y * self.width + x) as usize;
            self.map[index] = entity;
        } else {
            panic!("Writing entity to tile outside of map: {:?}, {:?}", x, y)
        }
    }
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(
        &self,
        app: &mut AppBuilder,
    ) {
        const WIDTH: i64 = 400;
        const HEIGHT: i64 = 400;
        app
            //Tilemap
            .insert_resource(TileWeightMap::new(WIDTH, HEIGHT))
            .insert_resource(TileEntityMap::new(WIDTH, HEIGHT))
            //Window
            .insert_resource(WindowDescriptor {
                width: 1270.0,
                height: 720.0,
                title: String::from("game"),
                ..Default::default()
            })
            .add_startup_system(init_tilemaps.system())
            //.add_system(plan_path.system().label("preparation"))
            .add_plugin(time::TimePlugin)
            .insert_resource(item::HamburgerTimer(Timer::from_seconds(0.4, true)))
            .add_system(item::spawn_hamburger_every_second.system());
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
