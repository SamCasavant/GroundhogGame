use std::cmp::min;
use std::ops::Range;

use bevy::prelude::*;
use pathfinding::prelude::{absdiff, astar};

use crate::engine::actor::{Direction, Orientation};
use crate::engine::world::{Destination, Position, TileEntityMap, TileWeightMap};

#[derive(Clone)]
pub struct Path(pub Vec<Position>);
impl Path {}

pub fn local_avoidance(
    mut commands: Commands,
    entity_map: Res<TileEntityMap>,
    weight_map: Res<TileWeightMap>,
    mut query: Query<(Entity, &Position, &mut Path, &Orientation)>,
) {
    for (entity, position, mut path, orientation) in query.iter_mut() {
        let mut nearby_entities = Vec::new();
        for near_position in position.get_range(2, 2) {
            match entity_map.map.get(&near_position) {
                Some(entity) => nearby_entities.push(entity),
                None => (),
            }
        }
        if !nearby_entities.is_empty() {
            // TODO: Rewrite this trash
            let x = position.x;
            let y = position.y;
            let detour = match orientation.0 {
                Direction::Up => Position { x: x + 1, y: y + 1 },
                Direction::Down => Position { x: x - 1, y: y - 1 },
                Direction::UpLeft => Position { x, y: y + 1 },
                Direction::UpRight => Position { x: x + 1, y },
                Direction::DownLeft => Position { x: x - 1, y },
                Direction::DownRight => Position { x, y: y - 1 },
                Direction::Left => Position { x: x - 1, y: y + 1 },
                Direction::Right => Position { x: x + 1, y: y - 1 },
            };
            let (index, slice) =
                local_detour(&path.clone(), &detour, &weight_map);
            path.0.splice(..index, slice);
        }
    }
}

fn local_detour(
    path: &Path,
    position: &Position,
    weight_map: &Res<TileWeightMap>,
) -> (usize, [Position; 2]) {
    // Adds a step to the beginning of the path and then subsequent steps
    // to rejoin the path
    for neighbor in neighbors_with_weights(position, weight_map) {
        if neighbor.1 != i64::MAX {
            for index in 0..1 {
                if path.0[index] == neighbor.0 {
                    let slice = [*position, neighbor.0];
                    return (index, slice);
                }
            }
        }
    }
    let slice = get_path(position, &path.0[2], weight_map).unwrap();
    return (2, [slice.0[0], slice.0[1]]);
}

pub fn plan_path(
    mut commands: Commands,
    query: Query<(Entity, &Position, &Destination), Without<Path>>,
    weight_map: Res<TileWeightMap>,
) {
    for (entity, position, destination) in query.iter() {
        let plan = get_path(position, &destination.0, &weight_map);
        if let Some(p) = plan {
            let mut path = p.0;
            path.remove(0);
            if !path.is_empty() {
                commands.entity(entity).insert(Path(path));
            }
        }
    }
}

pub fn get_path(
    position: &Position,
    destination: &Position,
    weight_map: &Res<TileWeightMap>,
) -> Option<Path> {
    let mut path = Vec::new();
    let plan = astar(
        position,
        |p| neighbors_with_weights(p, &weight_map),
        |p| diagonal_distance(p, destination),
        |p| p == destination,
    );
    if let Some(p) = plan {
        for step in p.0 {
            path.push(step);
        }
        Some(Path(path))
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
        if let Some(weight) = weight_map.map.get(&position) {
            if weight < &i64::MAX {
                neighbors.push((
                    Position {
                        x: x + step_x,
                        y: y + step_y,
                    },
                    weight_map.map[&position],
                ));
            }
        }
    }
    for (step_x, step_y) in &[(1, 1), (-1, -1), (1, -1), (-1, 1)] {
        if let Some(weight) = weight_map.map.get(&position) {
            if weight < &i64::MAX {
                neighbors.push((
                    Position {
                        x: x + step_x,
                        y: y + step_y,
                    },
                    ((weight_map.map[&position] as f64) * 2_f64.sqrt()) as i64,
                ));
            }
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
