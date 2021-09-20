/*
Entities with a Path component use this module to pathfind and proceed through steps in their path.
Paths are initialized in full using aStar.
Paths are stored in the Path component (a vector of positions) and elements are shifted off when a step is taken.
The move_actor function handles final adjustments to the path in the event of a potential collision with another entity.
Ground Types are used to produce tile weights, which hopefully can encourage aStar to prefer sidewalks over roads.

TODO:
Allow actors to rejoin path rather than starting over in the event of a correction.
Create a system for near objectives to save aStar effort.
Find a better way to store currently occupied positions? (Using hashmap for now)
Improve GroundType integration with tile map.
Integrate tile system with bevy_ECS_tiles

*/

pub(crate) use bevy::prelude::*;

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

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(PlannedSteps {
            steps: HashMap::<Position, bevy::prelude::Entity>::new(),
        })
        .add_system(move_actor.system());
    }
}

pub fn move_actor(
    mut tilemap: ResMut<TileMap>,
    time: Res<Time>,
    mut query: Query<(
        &mut Timer,
        &mut Position,
        &mut Orientation,
        &mut Destination,
        &mut Path,
    )>,
) {
    for (mut timer, mut position, mut orientation, mut destination, mut path) in
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
                //TODO: Move destination changes to higher level module, derandomize. This will likely involve an 'at_destination component'.
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
