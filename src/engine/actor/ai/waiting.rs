use bevy::prelude::*;
use rand::seq::SliceRandom;

use crate::engine::actor::ai::pathfinding::{neighbors_except_entities, Path};
use crate::engine::world::{Position, TileEntityMap, TileWeightMap};

pub struct WaitGoal;

pub struct Wandering(u32); // Value represents number of steps before checking for new goal

pub fn wait_ai(
    mut commands: Commands,
    query: Query<Entity, (With<WaitGoal>, Without<Wandering>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(Wandering);
    }
}

pub fn wander_system(
    mut commands: Commands,
    mut query: Query<(Entity, &Position, &mut Wandering)>,
    entity_map: Res<TileEntityMap>,
    weight_map: Res<TileWeightMap>,
) {
    let mut rng = rand::thread_rng();
    for (entity, position, mut wander_count) in query.iter_mut() {
        // TODO: Make this deterministic
        warn!("Wandering is NON-DETERMINISTIC BEHAVIOR");
        let neighbors =
            neighbors_except_entities(*position, &weight_map, &entity_map);
        let step = neighbors
            .choose(&mut rng)
            .map_or_else(|| *position, |step| step.0);

        commands.entity(entity).insert(Path(vec![step]));

        wander_count.0 -= 1;
    }
}
