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

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(
        &self,
        app: &mut AppBuilder,
    ) {
        const WIDTH: u32 = 400;
        const HEIGHT: u32 = 400;
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
            .add_plugin(time::TimePlugin)
            .insert_resource(item::HamburgerTimer(Timer::from_seconds(0.2, true)))
            .add_system(item::spawn_hamburger_every_second.system());
    }
}

pub struct RelativePosition {
    pub x: i8,
    pub y: i8,
}

#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub struct Position {
    pub x: u32,
    pub y: u32,
}
impl Position {
    pub fn neighbors(
        self,
        radius: u32,
    ) -> Vec<Self> {
        let mut neighbors = Vec::new();
        let x_min = self.x.saturating_sub(radius);
        let x_max = self.x.saturating_add(radius);

        let y_min = self.y.saturating_sub(radius);
        let y_max = self.y.saturating_add(radius);
        for x in x_min..=x_max {
            for y in y_min..=y_max {
                let position = Self { x, y };
                if !(self == position) {
                    neighbors.push(position);
                }
            }
        }
        neighbors
    }
    pub fn side_neighbors(self) -> Vec<Self> {
        let mut neighbors = Vec::new();
        for &(step_x, step_y) in &[(1, 2), (1, 0), (2, 1), (0, 1)] {
            let try_x = (self.x + step_x).checked_sub(1);
            let try_y = (self.y + step_y).checked_sub(1);
            if let Some(x) = try_x {
                if let Some(y) = try_y {
                    neighbors.push(Self { x, y });
                }
            }
        }
        neighbors
    }
    pub fn corner_neighbors(self) -> Vec<Self> {
        let mut neighbors = Vec::new();
        for &(step_x, step_y) in &[(2, 2), (0, 0), (2, 0), (0, 2)] {
            let try_x = (self.x + step_x).checked_sub(1);
            let try_y = (self.y + step_y).checked_sub(1);
            if let Some(x) = try_x {
                if let Some(y) = try_y {
                    neighbors.push(Self { x, y });
                }
            }
        }
        neighbors
    }
    pub fn diagonal_distance(
        self,
        other: Self,
    ) -> u32 {
        let dx = absdiff(self.x, other.x);
        let dy = absdiff(self.y, other.y);
        let diag = ((dx + dy) as f64) - (min(dx, dy) as f64) * (2f64.sqrt());
        diag as u32
    }
}
impl Sub for Position {
    type Output = RelativePosition;
    // Sub is only used for local points, so casting to i8 should be okay
    // Maybe should be renamed
    #[allow(clippy::cast_possible_truncation)]
    fn sub(
        self,
        other: Self,
    ) -> RelativePosition {
        RelativePosition {
            x: (i64::from(self.x) - i64::from(other.x)) as i8,
            y: (i64::from(self.y) - i64::from(other.y)) as i8,
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
    width:   u32,
    height:  u32,
    /* Maps position to weight (i64)
     * i64::MAX is treated as an obstacle */
}
// Checks are built in
#[allow(clippy::indexing_slicing)]
impl TileWeightMap {
    // consider morton encoding if this is slow
    pub fn new(
        width: u32,
        height: u32,
    ) -> Self {
        let map = vec![100; (width * height) as usize];
        Self { map, width, height }
    }
    pub fn get(
        &self,
        x: u32,
        y: u32,
    ) -> i64 {
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            self.map[index]
        } else {
            i64::MAX
        }
    }
    pub fn set(
        &mut self,
        x: u32,
        y: u32,
        weight: i64,
    ) {
        if x < self.width && y < self.height {
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
    width:   u32,
    height:  u32,
}
// Checks are built in
#[allow(clippy::indexing_slicing)]
impl TileEntityMap {
    // Consider morton encoding if this is slow
    pub fn new(
        width: u32,
        height: u32,
    ) -> Self {
        let map = vec![None; (width * height) as usize];
        Self { map, width, height }
    }
    pub fn get(
        &self,
        x: u32,
        y: u32,
    ) -> Option<Entity> {
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            self.map[index]
        } else {
            None
        }
    }
    pub fn set(
        &mut self,
        x: u32,
        y: u32,
        entity: Option<Entity>,
    ) {
        if self.get(x, y).is_some() && entity.is_some() {
            panic!("TILE CONFLICT: Two entities are occupying a single tile.")
        }
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            self.map[index] = entity;
        } else {
            panic!("Writing entity to tile outside of map: {:?}, {:?}", x, y)
        }
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
        map: Map::new(0u16, map_entity),
        transform: Transform::from_xyz(0.0, 0.0, 0.0)
            .mul_transform(Transform::from_scale(Vec3::splat(4.0))),
        ..Default::default()
    });
}
