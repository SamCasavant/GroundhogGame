// Tasks are abstractions over AI behavior.
// AI picks tasks based on priority values, which can come from the actor's
// state or direct assignment.
use bevy::prelude::*;

use crate::engine::actor::{Direction, Frozen, Intelligent, Inventory,
                           Orientation, Status};
use crate::engine::{world, world::Position};

pub mod pathfinding;

// Tasks:
mod eating;
mod waiting;

// Goals
pub enum Goals {
    Eat,
    Drink,
    Wait,
}

// TODO: Generics!
pub struct DrinkGoal;

pub struct Moving;
pub struct PickingUp;

// Goal Parameters
pub struct Target(Entity);

pub struct AIPlugin;
impl Plugin for AIPlugin {
    fn build(
        &self,
        app: &mut AppBuilder,
    ) {
        app.add_system_set(
            SystemSet::new()
                .label("ai")
                .before("action")
                .with_system(choose_next_goal.system().label("goal ai"))
                .with_system(
                    eating::eating_ai
                        .system()
                        .label("task ai")
                        .after("goal ai")
                        .before("plan ai"),
                )
                .with_system(
                    waiting::wait_ai
                        .system()
                        .label("task ai")
                        .after("goal ai")
                        .before("plan ai"),
                )
                .with_system(
                    pathfinding::plan_path
                        .system()
                        .label("plan ai")
                        .after("task ai")
                        .before("post ai"),
                )
                .with_system(
                    pathfinding::local_avoidance
                        .system()
                        .label("post ai")
                        .after("plan ai")
                        .before("action"),
                ),
        )
        .add_system_set(
            SystemSet::new()
                .label("action")
                .with_system(waiting::wander_system.system())
                .with_system(walk_system.system())
                .with_system(eating::eat_system.system())
                .with_system(pick_up_system.system())
                .with_system(eating::find_food_system.system()),
        );
    }
}

pub fn choose_next_goal(
    mut commands: Commands,
    mut query: Query<
        (Entity, &Status),
        (
            With<Intelligent>,
            Without<eating::EatGoal>,
            Without<DrinkGoal>,
            Without<waiting::WaitGoal>,
        ),
    >,
) {
    for (entity, status) in query.iter_mut() {
        debug!("Entity {:?} has no active goal, updating.", entity);
        let mut priority = status.laziness;
        let mut goal = Goals::Wait;
        // TODO: Refactor because this will get large and repetive
        if status.hunger > priority {
            goal = Goals::Eat;
            priority = status.hunger;
        }
        if status.thirst > priority {
            goal = Goals::Drink;
            // priority = status.thirst;
        }
        // TODO implement routines (time-based tasks)
        match goal {
            Goals::Eat => {
                debug!("EatGoal selected.");
                commands.entity(entity).insert(eating::EatGoal);
            }
            Goals::Drink => {
                debug!("DrinkGoal selected.");
            }
            Goals::Wait => {
                debug!("WaitGoal selected.");
                commands.entity(entity).insert(waiting::WaitGoal);
            } /* {
               * commands.entity(entity).insert(WaitGoal);
               * } */
        }
    }
}

fn pick_up_system(
    mut commands: Commands,
    mut actors: Query<(Entity, &mut Inventory, &Target), With<PickingUp>>,
    object_query: Query<&Position>,
) {
    // Takes items from ground and adds them to actor inventory
    // Do not enter PickingUp state when too far from target; no checks
    for (actor, mut inventory, target) in actors.iter_mut() {
        commands.entity(actor).remove::<Moving>();
        if inventory.is_full() {
            // Drop something?
            debug!(
                "Entity {:?} has full inventory. Picking up anyway. Because \
                 you didn't write the code. Thanks...",
                actor
            );
        }
        if object_query.get(target.0).is_ok() {
            debug!("Entity: {:?} is picking up {:?}", actor, target.0);
            inventory.add(target.0); // Remove the item from the ground
            commands.entity(target.0).remove::<Position>();
        } else {
            debug!("Entity: {:?} can't pick up {:?}", actor, target.0);
            // Someone else got there first?
            commands.entity(actor).remove::<Target>();
        }
        commands.entity(actor).remove::<PickingUp>();
    }
}

pub fn walk_system(
    mut entity_map: ResMut<world::TileEntityMap>,
    game_time: Res<world::time::GameTime>,
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &mut world::time::GameTime,
            &mut world::Position,
            &mut Orientation,
            &world::Destination,
            &mut pathfinding::Path,
        ),
        (With<Moving>, Without<Frozen>),
    >,
) {
    let mut planned_moves = Vec::new();
    for (
        entity,
        mut timer,
        mut position,
        mut orientation,
        destination,
        mut path,
    ) in &mut query.iter_mut()
    {
        if path.0.is_empty() {
            debug!("Entity {:?} has empty path, removing.", entity);
            commands.entity(entity).remove::<pathfinding::Path>();
        } else if *timer <= *game_time {
            if entity_map.get(path.0[0].x, path.0[0].y).is_none() {
                if planned_moves.contains(&path.0[0]) {
                    warn!(
                        "Entity {:?} is not allowed to walk because someone \
                         else got there first. NON-DETERMINISTIC BEHAVIOR",
                        entity
                    );
                } else {
                    let next_step = path.0.remove(0);
                    planned_moves.push(next_step);

                    let next_direction = next_step - *position;
                    match next_direction {
                        world::RelativePosition { x: 1, .. } => {
                            *orientation = Orientation(Direction::Up);
                        }
                        world::RelativePosition { x: -1, .. } => {
                            *orientation = Orientation(Direction::Down);
                        }
                        world::RelativePosition { y: 1, .. } => {
                            *orientation = Orientation(Direction::Right);
                        }
                        world::RelativePosition { y: -1, .. } => {
                            *orientation = Orientation(Direction::Left);
                        }
                        _ => (),
                    }
                    // Destructure for convenience
                    let old_x = position.x;
                    let old_y = position.y;

                    let new_x = next_step.x;
                    let new_y = next_step.y;
                    // Mark previous tile as unoccupied
                    entity_map.set(old_x, old_y, None);
                    // Move the actor
                    *position = next_step;
                    // Mark next tile as occupied
                    entity_map.set(new_x, new_y, Some(entity));
                    // Set time of next action
                    *timer = game_time.copy_and_tick(20);
                }
            } else {
                *timer = game_time.copy_and_tick(1);
            }
        }
        if *destination == *position {
            debug!("Entity {:?} has arrived at destination.", entity);
            commands.entity(entity).remove::<Moving>();
        }
    }
}
