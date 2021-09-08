use bevy::{prelude::*, render::camera::Camera, sprite::SpriteSettings};

extern crate pathfinding;
use pathfinding::prelude::{absdiff, astar};

use std::collections::HashMap;
use std::convert::TryInto;
use std::ops::RangeInclusive;

use rand::Rng;

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

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(PlannedSteps {
            steps: HashMap::<Position, bevy::prelude::Entity>::new(),
        })
        .add_system(move_actor.system());
    }
}
#[derive(Debug, Copy, Clone)]
pub struct Destination(pub Position);

impl PartialEq<Position> for Destination {
    fn eq(&self, other: &Position) -> bool {
        self.0.x == other.x && self.0.y == other.y
    }
}

fn move_actor(
    mut tilemap: ResMut<TileMap>,
    time: Res<Time>,
    mut query: Query<(
        Entity,
        &mut Timer,
        &mut Position,
        &mut Orientation,
        &mut Destination,
        &mut Path,
        &mut Transform,
    )>,
) {
    for (entity, mut timer, mut position, mut orientation, mut destination, mut path, mut transform) in
        &mut query.iter_mut()
    {
        timer.tick(time.delta());
        if path.0.len() < 1 {
            *path = plan_path(*position, *destination, &tilemap);
        }
        if timer.just_finished() {
            if path.0.len() > 0 {
                let mut next_step = path.0[0];
                // If an entity's path is blocked by another entity, first try to find an alternate move that gets closer to the destination.
                if tilemap.map.contains_key(&next_step) == true
                    && tilemap.map.get(&next_step).unwrap().occupied == true
                {
                    next_step = *position;
                    let mut temp_steps = move_weights(&*position, &tilemap);
                    temp_steps.sort_by_key(|k| k.1);
                    for step in temp_steps.iter() {
                        if tilemap.map.contains_key(&step.0) == false
                            || tilemap.map.get(&step.0).unwrap().occupied == false
                        {
                            if absdiff(destination.0.x, position.x)
                                - absdiff(destination.0.x, step.0.x)
                                + absdiff(destination.0.y, position.y)
                                - absdiff(destination.0.y, step.0.y)
                                > 0
                            {
                                next_step = step.0;
                                break;
                            }
                        }
                    }
                    //Failing that, try to find a move that moves closer in either X or Y.
                    if next_step == *position {
                        for step in temp_steps.iter() {
                            if tilemap.map.contains_key(&step.0) == false
                                || tilemap.map.get(&step.0).unwrap().occupied == false
                            {
                                if absdiff(destination.0.x, position.x)
                                    - absdiff(destination.0.x, step.0.x)
                                    > 0
                                    || absdiff(destination.0.y, position.y)
                                        - absdiff(destination.0.y, step.0.y)
                                        > 0
                                {
                                    next_step = step.0;
                                    break;
                                }
                            }
                        }
                    }
                    if next_step != *position {
                        //(If the corrections above found an alternate move)
                        //If we've moved off of our path, we'll need to get a new one on the next loop.
                        //(This should be changed to only if we cannot immediately rejoin path.)
                        *path = Path(Vec::new());
                    }
                } else {
                    path.0.remove(0);
                }
                //Prepare animation
                let translation = Vec3::new(
                    (next_step.x - position.x) as f32 * TILE_WIDTH,
                    (next_step.y - position.y) as f32 * TILE_WIDTH,
                    0.0,
                );
                transform.translation += translation;
                match next_step {
                    Position { x: 1, .. } => *orientation = Orientation(Direction::Up),
                    Position { x: -1, .. } => *orientation = Orientation(Direction::Down),
                    Position { y: 1, .. } => *orientation = Orientation(Direction::Right),
                    Position { y: -1, .. } => *orientation = Orientation(Direction::Left),
                    _ => (),
                }
                let mut tile = tilemap.map.entry(*position).or_insert(Tile {
                    ground_type: GroundType::Path,
                    occupied: false,
                });
                tile.occupied = false;
                *position = next_step;
                tile = tilemap.map.entry(*position).or_insert(Tile {
                    ground_type: GroundType::Path,
                    occupied: true,
                });
                tile.occupied = true;
            }
            if *destination == *position {
                let xrange = RangeInclusive::new(-15, 15);
                let yrange = xrange.clone();
                let mut rng = rand::thread_rng();
                destination.0.x = rng.gen_range(xrange);
                destination.0.y = rng.gen_range(yrange);

            }
        }
    }
}

fn plan_path(position: Position, destination: Destination, tilemap: &ResMut<TileMap>) -> Path {
    let plan = astar(
        &position,
        |p| move_weights(p, tilemap),
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
            return Path(path);
        }
    }
    let mut path = Vec::new();
    path.push(position);
    return Path(path);
}

fn move_weights(position: &Position, tilemap: &ResMut<TileMap>) -> Vec<(Position, u32)> {
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
use bevy_ecs_tilemap::prelude::*;

#[derive(Debug)]
pub struct Tile {
    pub occupied: bool,
    pub ground_type: GroundType,
}
#[derive(Default)]
pub struct TileMap {
    pub map: HashMap<Position, Tile>,
}
pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(TileMap {
            map: HashMap::<Position, Tile>::new(),
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
        .add_system(camera_movement.system());
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
const TILE_WIDTH: f32 = 16.0;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    textures: ResMut<Assets<Texture>>,
    texture_atlases: ResMut<Assets<TextureAtlas>>,
    tilemap: ResMut<TileMap>,
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

fn animate_sprite_system(mut query: Query<(&mut TextureAtlasSprite, &mut Transform, &Orientation, &Position)>) {
    for (mut sprite, mut transform, orientation, position) in &mut query.iter_mut() {
        match orientation.0 {
            Direction::Up => sprite.index = 5,
            Direction::Down => sprite.index = 1,
            Direction::Left => sprite.index = 10,
            Direction::Right => sprite.index = 13,
            Direction::UpLeft => todo!(),
            Direction::UpRight => todo!(),
            Direction::DownLeft => todo!(),
            Direction::DownRight => todo!(),
        }
    }
}

pub fn init_sprite_sheet(
    path: &str,
    asset_server: &Res<AssetServer>,
    mut texture_atlases: &mut ResMut<Assets<TextureAtlas>>,
    position: Position,
) -> SpriteSheetBundle {
    let texture_handle = asset_server.load(path);
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(4.0, 4.0), 4, 4);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    SpriteSheetBundle {
        texture_atlas: texture_atlas_handle,
        transform: Transform::from_scale(Vec3::splat(6.0)),
        ..Default::default()
    }
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

pub fn camera_movement(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    for mut transform in &mut query.iter_mut() {
        let mut direction = Vec3::ZERO;
        if keyboard_input.pressed(KeyCode::A) {
            direction -= Vec3::new(1.0, 0.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::E) {
            direction += Vec3::new(1.0, 0.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::Comma) {
            direction += Vec3::new(0.0, 1.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::O) {
            direction -= Vec3::new(0.0, 1.0, 0.0);
        }
        transform.translation += time.delta_seconds() * direction * 500.0;
    }
}
