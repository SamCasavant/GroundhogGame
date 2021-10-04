// Pathfinding:
// Entities with a Position and Destinations component, but without a Path
// component use this module to generate a path. Paths are initialized in full
// using aStar. Paths are stored in the Path component (a vector of positions)
// and Ground Types are used to produce tile weights, which hopefully can
// encourage aStar to prefer sidewalks over roads. Note: This may not be
// deterministic, and needs to be. Consider invoking bevy stages.
//

use std::cmp::min;

use bevy::prelude::*;
use pathfinding::prelude::{absdiff, astar};

use crate::engine::world::{Destination, Position, TileEntityMap, TileWeightMap};

#[derive(Clone)]
pub struct Path(pub Vec<Position>);
impl Path {}

pub fn local_avoidance(
    // mut commands: Commands,
    entity_map: Res<TileEntityMap>,
    weight_map: Res<TileWeightMap>,
    mut query: Query<(/* Entity, */ &Position, &mut Path)>,
) {
    // Path Wars: Episode IV
    // It is a period of civil war.
    // Bounding get_path has fixed the stalling issue...
    // But entities still can't find a path to an occupied destination.
    // The empire needs to be pathing to the position that is closest to the
    // destination. But this doesn't mesh well with current logic.
    // The empire will begin a rewrite of this file.
    // Powerful enough to destroy an entire planet, its completion spells
    // certain doom for the champions of freedom.

    for (/* entity, */ position, mut path) in query.iter_mut() {
        let mut nearby_entities = Vec::new();
        for near_position in position.get_range(1, 1) {
            match entity_map.get(near_position.x, near_position.y) {
                Some(entity) => nearby_entities.push(entity),
                None => (),
            }
        }
        if !nearby_entities.is_empty() {
            let mut index = 2;
            if path.0.len() < 1 {
                continue;
            } else if path.0.len() < 2 {
                index = 0;
            } else if path.0.len() < 3 {
                index = 1;
            }
            let local_path = get_path_around_entities(
                position,
                &mut path.0[index],
                &weight_map,
                &entity_map,
            );
            path.0 = match local_path {
                Some(mut p) => {
                    p.extend(path.0[index + 1..].iter().cloned());
                    p
                }
                None => vec![*position],
            }
        }
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
        |p| neighbors_with_entities(p, weight_map, entity_map),
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
        return Some(path);
    } else {
        println!("Could not find path");
        None
    }
}

fn neighbors_with_entities(
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
        if entity.is_none() {
            if weight < i64::MAX {
                neighbors.push((
                    Position {
                        x: check_x,
                        y: check_y,
                    },
                    weight,
                ));
            }
        } else {
            continue;
        }
    }
    for (step_x, step_y) in &[(1, 1), (-1, -1), (1, -1), (-1, 1)] {
        let check_x = x + step_x;
        let check_y = y + step_y;
        let weight = weight_map.get(check_x, check_y);
        let entity = entity_map.get(check_x, check_y);
        if entity.is_none() {
            if weight < i64::MAX {
                neighbors.push((
                    Position {
                        x: check_x,
                        y: check_y,
                    },
                    (weight as f64 * 2_f64.sqrt()) as i64,
                ));
            }
        } else {
            continue;
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

// pub fn plan_path(
//     position: Position,
//     // Who, day and night, must scramble for a living, feed a wife and
//     // children, say his daily prayers? And who has the right, as master of
// the     // house, to have the final word at home?
//     target_pos: Position, /* TODO: Add a target_pos wrapper to make this
//                            * joke work. Find a
//                            * good reason. */
//     weight_map: Res<TileWeightMap>,
//     entity_map: Res<TileEntityMap>,
// ) -> Option<Path> {
//     let mut to_see = BinaryHeap::new();
//     to_see.push(SmallestCostHolder {
//         estimated_cost: 0,
//         cost:           0,
//         index:          0,
//     });
//     let mut parents = FxIndexMap::default();
//     let mut steps =
//         neighbors_with_weights_avoid_entities(position, weight_map,
// entity_map);

//     let mut paths = Vec::new();
//     for step in steps {
//         paths.push((step.0, Path(vec![position, step.1]), step.1))
//     }
//     for path in paths {
//         path.0 += proximity_heuristic(position, target_pos);
//     }
//     &paths.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
//     loop {
//         let path = paths[0];
//         let position = path.2;
//         steps = neighbors_with_weights(position, weight_map)
//     }
//     return Some(Path(Vec::new()));
// }

// fn neighbors_with_weights_avoid_entities(
//     position: Position,
//     weight_map: Res<TileWeightMap>,
//     entity_map: Res<TileEntityMap>,
// ) -> Vec<(i64, Position)> {
//     let x = position.x;
//     let y = position.y;
//     let mut neighbors = Vec::new();
//     for (step_x, step_y) in &[(0, 1), (0, -1), (1, 0), (-1, 0)] {
//         if let Some(weight) = weight_map.map.get(&position) {
//             if weight < &i64::MAX {
//                 if let Some(entity) = entity_map.map.get(&position) {
//                     neighbors.push((weight_map.map[&position], Position {
//                         x: x + step_x,
//                         y: y + step_y,
//                     }));
//                 }
//             }
//         }
//     }
//     for (step_x, step_y) in &[(1, 1), (-1, -1), (1, -1), (-1, 1)] {
//         if let Some(weight) = weight_map.map.get(&position) {
//             if weight < &i64::MAX {
//                 if let Some(entity) = entity_map.map.get(&position) {
//                     neighbors.push((weight_map.map[&position], Position {
//                         x: x + step_x,
//                         y: y + step_y,
//                     }));
//                 }
//                 if let Some(entity) = entity_map.map.get(&position) {
//                     neighbors.push((weight_map.map[&position], Position {
//                         x: x + step_x,
//                         y: y + step_y,
//                     }));
//                 }
//             }
//         }
//     }
//     neighbors
// }

// fn proximity_heuristic(
//     position: Position,
//     destination: Position,
// ) -> i64 {
//     // Pythagorean distance
//     (((destination.x - position.x).pow(2) + (destination.y -
// position.y).pow(2))         as f64)
//         .sqrt() as i64
// }
