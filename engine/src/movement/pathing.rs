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

use std::collections::HashMap;
use std::ops::RangeInclusive;

use rand::Rng;

extern crate pathfinding;
use pathfinding::prelude::absdiff;

use crate::world;
pub struct Orientation(pub Direction);

#[derive(Default)]
struct PlannedSteps {
    steps: HashMap<world::Position, bevy::prelude::Entity>,
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

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(PlannedSteps {
            steps: HashMap::<world::Position, bevy::prelude::Entity>::new(),
        })
        .add_system(move_actor.system());
    }
}

pub fn move_actor(
    mut tilemap: ResMut<world::TileMap>,
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut Timer,
        &mut world::Position,
        &mut Orientation,
        &mut world::Destination,
        &mut world::Path,
    )>,
) {
    for (entity, mut timer, mut position, mut orientation, mut destination, mut path) in
        &mut query.iter_mut()
    {
        timer.tick(time.delta());
        if path.0.len() < 1 {
            commands.entity(entity).remove::<world::Path>();
        }
        if timer.just_finished() {
            if path.0.len() > 0 {
                let mut next_step = path.0[0];
                // If an entity's path is blocked by another entity, first try to find an alternate move that gets closer to the destination.
                if tilemap.map.contains_key(&next_step) == true
                    && tilemap.map.get(&next_step).unwrap().occupied == true
                {
                    let mut cur_distance =
                        (destination.0.x - position.x) ^ 2 + (destination.0.y - position.y) ^ 2;
                    next_step = *position;
                    let mut temp_steps = world::move_weights(&*position, &tilemap);
                    temp_steps.sort_by_key(|k| k.1);
                    for step in temp_steps.iter() {
                        if tilemap.map.contains_key(&step.0) == false
                            || tilemap.map.get(&step.0).unwrap().occupied == false
                        {
                            if (destination.0.x - (position.x + step.0.x))
                                ^ 2 + (destination.0.y - (position.y + step.0.y))
                                ^ 2
                                < cur_distance
                            {
                                cur_distance = (destination.0.x - (position.x + step.0.x))
                                    ^ 2 + (destination.0.y - (position.y + step.0.y))
                                    ^ 2;
                                next_step = step.0;
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
                        commands.entity(entity).remove::<world::Path>();
                    }
                } else {
                    path.0.remove(0);
                }

                match next_step {
                    world::Position { x: 1, .. } => *orientation = Orientation(Direction::Up),
                    world::Position { x: -1, .. } => *orientation = Orientation(Direction::Down),
                    world::Position { y: 1, .. } => *orientation = Orientation(Direction::Right),
                    world::Position { y: -1, .. } => *orientation = Orientation(Direction::Left),
                    _ => (),
                }
                let mut tile = tilemap.map.entry(*position).or_insert(world::Tile {
                    ground_type: world::GroundType::Path,
                    occupied: false,
                });
                tile.occupied = false;
                *position = next_step;
                tile = tilemap.map.entry(*position).or_insert(world::Tile {
                    ground_type: world::GroundType::Path,
                    occupied: true,
                });
                tile.occupied = true;
            }
            if *destination == *position {
                //TODO: Move destination changes to higher level module, derandomize. This will likely involve an 'at_destination component'.
                let xrange = RangeInclusive::new(0, 100);
                let yrange = xrange.clone();
                let mut rng = rand::thread_rng();
                destination.0.x = rng.gen_range(xrange);
                destination.0.y = rng.gen_range(yrange);
            }
        }
    }
}
