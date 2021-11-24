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
// TODO: For performance, consider limiting the number of pathfinding calls an
// entity can make per time

use std::i32::MAX;

use bevy::prelude::*;
use pathfinding::prelude::astar;

use crate::engine::{actor::ai::{Frozen, Moving},
                    world::{Destination, Position, TileEntityMap,
                            TileWeightMap}};

#[derive(Clone)]
pub struct Path(pub Vec<Position>);

// Tunables:
const PATH_SEARCH_RANGE: u32 = 16; // Range multiplier for finding paths
const MAX_PATH_LA: usize = 5; // Number of path elements to drop during local avoidance

pub fn local_avoidance(
    // TODO: Call this from walk system instead
    // TODO: Reduce cognitive complexity
    mut commands: Commands,
    entity_map: Res<TileEntityMap>,
    weight_map: Res<TileWeightMap>,
    mut query: Query<
        (Entity, &Position, &mut Path, &Destination),
        (With<Moving>, Without<Frozen>),
    >,
) {
    // This system routes an entity's path around local entities. It first
    // checks if there are neighbors in the vicinity of an entity. If there
    // are, it checks to see if the next step in the path collides with any
    // entities. If it does, it finds a destination that can be pathed to, and
    // paths to that. Failing that, it resets the path- that behavior should
    // change
    for (entity, position, mut path, destination) in query.iter_mut() {
        trace!("Pre-Local Avoidance Path: {:?}", path.0);
        if !path.0.is_empty()
            && entity_map.get(path.0[0].x, path.0[0].y).is_some()
        // FIXME: Does this cause index error on empty path?
        {
            debug!("Performing local avoidance for {:?}.", entity);
            // let nearby_entities = nearby_entities(*position, 1, &entity_map);
            // Note: Local avoidance should probably take this^ into account. My
            // implementation doesn't yet.
            if path.0.len() == 1 {
                debug!(
                    "Destination is inaccessible neighbor, freezing entity."
                );
                commands.entity(entity).insert(Frozen(20));
            } else {
                let break_index = if path.0.len() <= MAX_PATH_LA {
                    path.0.len() - 1
                } else {
                    MAX_PATH_LA
                };

                let local_destination = path.0[break_index];
                let valid_destination = best_nearest_valid_destination(
                    *position,
                    local_destination,
                    destination.0,
                    &weight_map,
                    &entity_map,
                );

                if valid_destination.is_some() {
                    let local_path = get_path_around_entities(
                        *position,
                        local_destination,
                        &weight_map,
                        &entity_map,
                    );

                    if let Some(p) = local_path {
                        // TODO: Reverse p and break when step is found
                        // TODO: Test this code
                        debug!("Local avoidance path found.");
                        let mut reattach_indices = None;
                        for (local_index, step) in
                            p.as_slice().iter().enumerate()
                        {
                            let neighbors = step.neighbors(1);
                            for (global_index, step) in
                                path.0[0..break_index].iter().enumerate()
                            {
                                if neighbors.contains(step) {
                                    reattach_indices =
                                        Some((local_index, global_index));
                                }
                            }
                        }
                        if let Some((local_index, global_index)) =
                            reattach_indices
                        {
                            debug!(
                                "Reattaching to path because {:?} neighbors \
                                 {:?}",
                                p[local_index], path.0[global_index]
                            );
                            let mut reattach = Vec::new();
                            reattach.extend_from_slice(&p[0..=local_index]);
                            reattach.extend_from_slice(
                                &path.0[global_index..path.0.len()],
                            );
                            path.0 = reattach;
                        } else {
                            path.0 = p;
                        }
                    } else {
                        debug!(
                            "No local avoidance path found, freezing entity."
                        );
                        commands.entity(entity).insert(Frozen(20));
                    }
                }
            }
        } else {
            debug!("Next step clear; skipping local avoidance.");
        }
        trace!("Post-Local Avoidance Path: {:?}", path.0);
    }
}

fn best_nearest_valid_destination(
    // Make these arguments make sense without comments, but for now:
    position: Position,    // Current position of entity
    target: Position,      // Intended nearby destination
    destination: Position, // Final destination
    weight_map: &Res<TileWeightMap>,
    entity_map: &Res<TileEntityMap>,
) -> Option<Position> {
    if entity_map.get(target.x, target.y).is_none()
        && weight_map.get(target.x, target.y) < i64::MAX
    {
        return Some(target);
    }
    let min_weight = weight_map.get(position.x, position.y);
    let min_distance = position.diagonal_distance(destination);
    let mut valid_destination = None;
    for neighbor in neighbors_except_entities(target, weight_map, entity_map) {
        let weight = neighbor.1;
        let distance = target.diagonal_distance(neighbor.0);
        if weight * (distance as i64) < min_weight * (min_distance as i64) {
            valid_destination = Some(neighbor.0);
        }
    }

    if valid_destination.is_some() {
        valid_destination
    } else {
        None
    }
}

pub struct NeedsPath;

pub fn plan_path(
    mut commands: Commands,
    query: Query<(Entity, &Position, &Destination), With<NeedsPath>>,
    weight_map: Res<TileWeightMap>,
) {
    for (entity, position, destination) in query.iter() {
        debug!("Performing pathfinding for entity: {:?}", entity);
        let plan = get_path(*position, destination.0, &weight_map);
        if let Some(p) = plan {
            if !p.is_empty() {
                commands
                    .entity(entity)
                    .insert(Path(p))
                    .remove::<NeedsPath>();
            }
        }
    }
}

pub fn get_path(
    position: Position,
    destination: Position,
    weight_map: &Res<TileWeightMap>,
) -> Option<Vec<Position>> {
    let mut path = Vec::new();
    if weight_map.get(destination.x, destination.y) == i64::MAX {
        panic!("Destination is inaccessible")
    }
    let plan = astar(
        &position,
        |p| neighbors_with_weights(*p, weight_map),
        |p| p.diagonal_distance(destination).into(),
        |p| {
            *p == destination
                || p.diagonal_distance(destination)
                    > PATH_SEARCH_RANGE
                        * position.diagonal_distance(destination)
        },
    )
    .unwrap_or((vec![position], 0));
    (plan.0.last() == Some(&destination)).then(|| {
        for step in plan.0 {
            path.push(step);
        }
        path.remove(0);
        path
    })
}

pub fn neighbors_with_weights(
    position: Position,
    weight_map: &Res<TileWeightMap>,
) -> Vec<(Position, i64)> {
    let mut neighbors = Vec::with_capacity(8);
    for Position { x, y, z } in position.side_neighbors() {
        let weight = weight_map.get(x, y);
        if weight < i64::MAX {
            neighbors.push((Position { x, y, z }, weight));
        }
    }
    for Position { x, y, z } in position.corner_neighbors() {
        let weight = ((weight_map.get(x, y) as f64) * 2f64.sqrt()) as i64;
        if weight < i64::MAX {
            neighbors.push((Position { x, y, z }, weight));
        }
    }
    neighbors
}
fn get_path_around_entities(
    position: Position,
    destination: Position,
    weight_map: &Res<TileWeightMap>,
    entity_map: &Res<TileEntityMap>,
) -> Option<Vec<Position>> {
    let mut path = Vec::new();

    let plan = astar(
        &position,
        |p| neighbors_except_entities(*p, weight_map, entity_map),
        |p| p.diagonal_distance(destination).into(),
        |p| {
            *p == destination
                || p.diagonal_distance(destination)
                    > PATH_SEARCH_RANGE
                        * position.diagonal_distance(destination)
        },
    )
    .unwrap_or((vec![position], 0));
    (plan.0.last() == Some(&destination)).then(|| {
        for step in plan.0 {
            path.push(step);
        }
        path.remove(0);
        path
    })
}

// TODO: Impl this for position
pub fn neighbors_except_entities(
    position: Position,
    weight_map: &Res<TileWeightMap>,
    entity_map: &Res<TileEntityMap>,
) -> Vec<(Position, i64)> {
    let mut neighbors = Vec::with_capacity(8);
    for Position { x, y, z } in position.side_neighbors() {
        let weight = weight_map.get(x, y);
        let entity = entity_map.get(x, y);
        if entity.is_none() && weight < i64::MAX {
            neighbors.push((Position { x, y, z }, weight));
        }
    }
    for Position { x, y, z } in position.corner_neighbors() {
        let weight = weight_map.get(x, y);
        let distance_adjusted_weight = ((weight as f64) * 2f64.sqrt()) as i64;
        let entity = entity_map.get(x, y);
        if entity.is_none() && distance_adjusted_weight < i64::MAX {
            neighbors.push((Position { x, y, z }, distance_adjusted_weight));
        }
    }
    neighbors
}
