// Tasks are abstractions over AI behavior.
// AI picks tasks based on priority values, which can come from the actor's
// state or direct assignment.
use bevy::prelude::*;

use crate::engine::actor::{pathfinding, Direction, Intelligent, Inventory,
                           Orientation, Status};
use crate::engine::{world,
                    world::{time, Position}};

// Goals
pub enum Goals {
    Eat,
    Drink,
    Wait,
}

// TODO: Generics!
pub struct DrinkGoal;
pub struct WaitGoal;

pub struct Moving;
pub struct PickingUp;

// Goal Parameters
pub struct Target(Entity);

// Tasks:
mod eating;

pub struct AIPlugin;
impl Plugin for AIPlugin {
    fn build(
        &self,
        app: &mut AppBuilder,
    ) {
        app.add_system(walk_system.system().label("acting"))
            .add_system(eating::eat_system.system().label("acting"))
            .add_system(pick_up_system.system().label("acting"))
            .add_system(eating::find_food_system.system().label("acting"))
            .add_system(choose_next_goal.system().label("preparation"))
            .add_system(
                eating::eating_ai
                    .system()
                    .label("planning")
                    .after("preparation")
                    .before("acting"),
            );
        //.add_system(eating::validate_food_target.system());
    }
}

pub fn choose_next_goal(
    mut commands: Commands,
    mut query: Query<
        (Entity, &Status),
        (
            With<Intelligent>,
            Without<(eating::EatGoal, DrinkGoal, WaitGoal)>,
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
            priority = status.thirst;
        }
        // TODO implement routines (time-based tasks)
        match goal {
            Goals::Eat => {
                debug!("EatGoal selected.");
                commands.entity(entity).insert(eating::EatGoal);
            }
            Goals::Drink => {
                debug!("DrinkGoal selected.")
            }
            Goals::Wait => {
                debug!("WaitGoal selected.")
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
        if inventory.is_full() {
            // Drop something?
            debug!(
                "Entity {:?} has full inventory. Picking up anyway. Because \
                 you didn't write the code. Thanks...",
                actor
            )
        }
        match object_query.get(target.0) {
            Ok(_) => {
                debug!("Entity: {:?} is picking up {:?}", actor, target.0);
                inventory.add(target.0); // Remove the item from the ground
                commands.entity(target.0).remove::<Position>();
            }
            Err(_) => {
                debug!("Entity: {:?} can't pick up {:?}", actor, target.0);
                // Someone else got there first?
                commands.entity(actor).remove::<Target>();
            }
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
        With<Moving>,
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
        } else if *timer <= *game_time
            && entity_map.get(path.0[0].x, path.0[0].y).is_none()
        {
            if !planned_moves.contains(&path.0[0]) {
                let next_step = path.0.remove(0);
                planned_moves.push(next_step);

                let next_direction = next_step - *position;
                match next_direction {
                    world::Position { x: 1, .. } => {
                        *orientation = Orientation(Direction::Up);
                    }
                    world::Position { x: -1, .. } => {
                        *orientation = Orientation(Direction::Down);
                    }
                    world::Position { y: 1, .. } => {
                        *orientation = Orientation(Direction::Right);
                    }
                    world::Position { y: -1, .. } => {
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
            } else {
                warn!(
                    "Entity {:?} is not allowed to walk because someone else \
                     got there first. NON-DETERMINISTIC BEHAVIOR",
                    entity
                );
            }
        } else {
            *timer = game_time.copy_and_tick(1);
        }
        if *destination == *position {
            debug!("Entity {:?} has arrived at destination.", entity);
            commands
                .entity(entity)
                .remove::<Moving>()
                .remove::<pathfinding::Path>();
        }
    }
}
