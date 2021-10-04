// Pathfinding:
// Entities with a Position and Destinations component, but without a Path
// component use this module to generate a path. Paths are initialized in full
// using aStar. Paths are stored in the Path component (a vector of positions)
// and Ground Types are used to produce tile weights, which hopefully can
// encourage aStar to prefer sidewalks over roads.

// TODO: This probably isn't deterministic, and it needs to be. When two
// entities want the same position, the first one to try will get it. The order
// entities try in is not guaranteed to be deterministic (?)
// Explicit system ordering is probably also needed.

use std::cmp::min;

use bevy::prelude::*;
use pathfinding::prelude::{absdiff, astar};

use crate::engine::world::{Destination, Position, TileEntityMap, TileWeightMap};

#[derive(Clone)]
pub struct Path(pub Vec<Position>);

pub fn local_avoidance(
    // mut commands: Commands,
    entity_map: Res<TileEntityMap>,
    weight_map: Res<TileWeightMap>,
    mut query: Query<(Entity, &Position, &mut Path, &Destination)>,
) {
    // This system routes an entity's path around local entities. It first
    // checks if there are neighbors in the vicinity of an entity. If there
    // are, it checks to see if the next step in the path collides with any
    // entities. If it does, it finds a destination that can be pathed to, and
    // paths to that. Failing that, it resets the path- that behavior should
    // change
    for (_entity, position, mut path, destination) in query.iter_mut() {
        let nearby_entities = nearby_entities(position, 1, &entity_map);
        if nearby_entities.is_some() {
            if path.0.len() <= 1
                && entity_map.get(path.0[0].x, path.0[0].y).is_some()
            // Panics on path of length 0, which are not supposed to exist here
            {
                path.0 = Vec::<Position>::new();
            } else if entity_map.get(path.0[0].x, path.0[0].y).is_some() {
                let index = if path.0.len() == 2 { 1 } else { 2 };
                let local_destination = path.0[index];
                let valid_destination = best_nearest_valid_destination(
                    position,
                    &local_destination,
                    &destination.0,
                    &weight_map,
                    &entity_map,
                );

                if valid_destination.is_some() {
                    let local_path = get_path_around_entities(
                        position,
                        &local_destination,
                        &weight_map,
                        &entity_map,
                    );
                    path.0 = match local_path {
                        Some(mut p) => {
                            if p.last()
                                .unwrap()
                                .neighbors(1)
                                .contains(&path.0[index])
                            {
                                // If the old path can be affixed to the new
                                // one:
                                p.extend(path.0[index + 1..].iter().cloned());
                            }
                            p
                        }
                        None => vec![*position],
                    }
                } else {
                    path.0 = Vec::<Position>::new();
                }
            }
        }
    }
}

fn nearby_entities(
    position: &Position,
    range: i64,
    entity_map: &Res<TileEntityMap>,
) -> Option<Vec<Entity>> {
    let mut nearby_entities = Vec::new();
    for near_position in position.neighbors(range) {
        if let Some(entity) = entity_map.get(near_position.x, near_position.y) {
            nearby_entities.push(entity);
        }
    }
    if nearby_entities.is_empty() {
        None
    } else {
        Some(nearby_entities)
    }
}

fn best_nearest_valid_destination(
    // Make these arguments make sense without comments, but for now:
    position: &Position,    // Current position of entity
    target: &Position,      // Intended nearby destination
    destination: &Position, // Final destination
    weight_map: &Res<TileWeightMap>,
    entity_map: &Res<TileEntityMap>,
    // mut search_range: u32,
) -> Option<Position> {
    if entity_map.get(target.x, target.y).is_none()
        && weight_map.get(target.x, target.y) < i64::MAX
    {
        return Some(*target);
    }
    let min_weight = weight_map.get(position.x, position.y);
    let min_distance = diagonal_distance(position, destination);
    let mut valid_destination = None;
    for neighbor in neighbors_except_entities(target, weight_map, entity_map) {
        let weight = neighbor.1;
        let distance = diagonal_distance(target, &neighbor.0);
        if weight * distance < min_weight * min_distance {
            valid_destination = Some(neighbor.0);
        }
    }

    // search_range -= 1;
    if valid_destination.is_some() {
        println!("Found valid destination");
        valid_destination
    //} else if search_range > 0 {
    //    todo!() // This function could be rewritten to be recursive when it
    // inevitably comes up
    } else {
        None
    }
}

pub fn plan_path(
    mut commands: Commands,
    query: Query<(Entity, &Position, &Destination), Without<Path>>,
    weight_map: Res<TileWeightMap>,
) {
    for (entity, position, destination) in query.iter() {
        let plan = get_path(position, &destination.0, &weight_map);
        if let Some(p) = plan {
            if !p.is_empty() {
                commands.entity(entity).insert(Path(p));
            }
        }
    }
}

pub fn get_path(
    position: &Position,
    destination: &Position,
    weight_map: &Res<TileWeightMap>,
) -> Option<Vec<Position>> {
    let mut path = Vec::new();
    if weight_map.get(destination.x, destination.y) == i64::MAX {
        panic!("Destination is inaccessible")
    }
    let plan = astar(
        position,
        |p| neighbors_with_weights(p, weight_map),
        |p| diagonal_distance(p, destination),
        |p| {
            *p == *destination
                || diagonal_distance(p, destination)
                    > 4 * diagonal_distance(position, destination)
        },
    )
    .unwrap_or((vec![*position], 0));
    if plan.0.last() == Some(destination) {
        for step in plan.0 {
            path.push(step);
        }
        path.remove(0);
        Some(path)
    } else {
        None
    }
}

pub fn neighbors_with_weights(
    position: &Position,
    weight_map: &Res<TileWeightMap>,
) -> Vec<(Position, i64)> {
    let x = position.x;
    let y = position.y;
    let mut neighbors = Vec::new();
    for (step_x, step_y) in &[(0, 1), (0, -1), (1, 0), (-1, 0)] {
        // TODO: Figure out a better way to write lines like these^, or accept
        // that this is the best I can do and remove this todo.
        let check_x = x + step_x;
        let check_y = y + step_y;
        let weight = weight_map.get(check_x, check_y);
        if weight < i64::MAX {
            neighbors.push((
                Position {
                    x: check_x,
                    y: check_y,
                },
                weight,
            ));
        }
    }
    for (step_x, step_y) in &[(1, 1), (-1, -1), (1, -1), (-1, 1)] {
        let check_x = x + step_x;
        let check_y = y + step_y;
        let weight = weight_map.get(check_x, check_y);
        if weight < i64::MAX {
            neighbors.push((
                Position {
                    x: check_x,
                    y: check_y,
                },
                weight,
            ));
        }
    }
    neighbors
}
fn get_path_around_entities(
    position: &Position,
    destination: &Position,
    weight_map: &Res<TileWeightMap>,
    entity_map: &Res<TileEntityMap>,
) -> Option<Vec<Position>> {
    let mut path = Vec::new();

    let plan = astar(
        position,
        |p| neighbors_except_entities(p, weight_map, entity_map),
        |p| diagonal_distance(p, destination),
        |p| {
            *p == *destination
                || diagonal_distance(p, destination)
                    > 100 * diagonal_distance(position, destination)
        },
    )
    .unwrap_or((vec![*position], 0));

    if plan.0.last() == Some(destination) {
        for step in plan.0 {
            path.push(step);
        }
        path.remove(0);
        Some(path)
    } else {
        None
    }
}

fn neighbors_except_entities(
    position: &Position,
    weight_map: &Res<TileWeightMap>,
    entity_map: &Res<TileEntityMap>,
) -> Vec<(Position, i64)> {
    let x = position.x;
    let y = position.y;
    let mut neighbors = Vec::new();
    for (step_x, step_y) in &[(0, 1), (0, -1), (1, 0), (-1, 0)] {
        let check_x = x + step_x;
        let check_y = y + step_y;
        let weight = weight_map.get(check_x, check_y);
        let entity = entity_map.get(check_x, check_y);
        if entity.is_none() && weight < i64::MAX {
            neighbors.push((
                Position {
                    x: check_x,
                    y: check_y,
                },
                weight,
            ));
        }
    }
    for (step_x, step_y) in &[(1, 1), (-1, -1), (1, -1), (-1, 1)] {
        let check_x = x + step_x;
        let check_y = y + step_y;
        let weight = weight_map.get(check_x, check_y);
        let entity = entity_map.get(check_x, check_y);
        if entity.is_none() && weight < i64::MAX {
            neighbors.push((
                Position {
                    x: check_x,
                    y: check_y,
                },
                (weight as f64 * 2_f64.sqrt()) as i64,
            ));
        }
    }
    neighbors
}

fn diagonal_distance(
    position: &Position,
    destination: &Position,
) -> i64 {
    let distance_mult = 1_i64;
    let distance_mult_two = 1_i64;
    let dx = absdiff(position.x, destination.x);
    let dy = absdiff(position.y, destination.y);
    distance_mult * (dx + dy)
        + (distance_mult_two - 2 * distance_mult) * min(dx, dy)
}
