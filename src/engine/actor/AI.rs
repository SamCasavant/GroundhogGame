// Tasks are abstractions over AI behavior.
// AI picks tasks based on priority values, which can come from the actor's
// state or direct assignment.
use bevy::prelude::*;

use crate::engine::actor::{Intelligent, Inventory, Moving, Status};
use crate::engine::world::{item, time, Destination, Position, TileEntityMap};

pub struct Routine {
    tasks: Option<Vec<ScheduledGoal>>,
}

pub struct ScheduledGoal {
    task: Goal,
    time: time::GameTime,
}

#[derive(Copy, Clone)]
pub struct Goal {
    action:     Action,
    parameters: ActionParameters,
    priority:   u32,
}

#[derive(Copy, Clone)]
pub enum Action {
    Wait,
    Eat,
    Drink,
}

#[derive(Default, Copy, Clone)]
pub struct ActionParameters {
    location: Option<Position>,
    target:   Option<Entity>,
}

// Tasks:
pub struct FindingFood;
pub struct PickingUp;
pub struct Eating;

pub fn choose_next_goal(
    mut commands: Commands,
    mut query: Query<
        (Entity, &Status, &Routine),
        (With<Intelligent>, Without<Goal>),
    >,
    time: Res<time::GameTime>,
) {
    for (entity, status, routine) in query.iter_mut() {
        let mut curtask = Goal {
            action:     Action::Wait,
            parameters: ActionParameters {
                ..Default::default()
            },
            priority:   status.laziness,
        };
        // TODO: Refactor because this will get large and repetive
        if status.hunger > curtask.priority {
            curtask = Goal {
                action:     Action::Eat,
                parameters: ActionParameters::default(),
                priority:   status.hunger,
            }
        }
        if status.thirst > curtask.priority {
            curtask = Goal {
                action:     Action::Drink,
                parameters: ActionParameters::default(),
                priority:   status.thirst,
            }
        }
        // TODO: Change priority calculation for scheduled events to incorporate
        // eta
        match &routine.tasks {
            Some(tasks) => {
                let priority = time.how_soon(tasks[0].time) / 60;
                if priority > curtask.priority {
                    curtask = tasks[0].task;
                }
            }
            None => todo!(),
        }
        commands.entity(entity).insert(curtask);
    }
}

fn generate_tasks(
    // Converts a goal into a set of tasks
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut Goal,
        &Position,
        &mut Destination,
        &mut Status,
        &mut Inventory,
    )>,
) {
    for (
        entity,
        mut goal,
        position,
        mut destination,
        mut status,
        mut inventory,
    ) in query.iter_mut()
    {
        match goal.action {
            Action::Wait => todo!(),
            Action::Eat => eat_tasks(
                &mut commands,
                entity,
                &mut goal,
                position,
                &mut destination,
                &mut inventory,
                &mut status,
            ),
            Action::Drink => todo!(),
        }
    }
}

fn eat_tasks(
    mut commands: &mut Commands,
    entity: Entity,
    mut goal: &mut Goal,
    position: &Position,
    destination: &mut Destination,
    inventory: &mut Inventory,
    status: &mut Status,
) {
    // Converts goal of eating into series of tasks.
    // Find food to eat. (Add to goal.parameters)
    // 1. Go to food. (Set destination to food location, start moving)
    // 2. Add food to inventory. ()
    // 3. Eat food. ()
}
// Entities with pick up task will pick up target entity
// Entities with destination and 'Moving' will move towards destination
// Entities

fn find_food(
    mut commands: Commands,
    mut actors: Query<
        (Entity, &Position, &Inventory, &mut Goal),
        With<FindingFood>,
    >,
    foods: Query<(Entity, &Position), With<item::NutritionValue>>,
) {
    // Finds a food object for every entity that wants food
    // Actors should not enter this state with food in their inventory; it will
    // be ignored
    // TODO: This is about the slowest way this can work OPTIMIZE ME!
    for (actor, position, inventory, mut goal) in actors.iter_mut() {
        let mut min_distance = i64::MAX;
        let mut selected_food = None;
        let mut food_location = None;
        for (food, food_position) in foods.iter() {
            // Todo: Make this pathfinding distance (at cost)
            let food_distance = position.diagonal_distance(food_position);
            if food_distance < min_distance {
                min_distance = food_distance;
                selected_food = Some(food);
                food_location = Some(*food_position);
            }
        }
        if selected_food.is_some() {
            goal.parameters.target = selected_food;
            goal.parameters.location = food_location;
            commands.entity(actor).remove::<FindingFood>();
            if position.neighbors(1).contains(&food_location.unwrap()) {
                commands.entity(actor).insert(PickingUp);
            } else {
                commands.entity(actor).insert(Moving);
            }
        }
    }
}
fn pick_up(
    mut commands: Commands,
    mut actors: Query<(Entity, &mut Inventory, &Goal), With<PickingUp>>,
    mut tile_entity_map: ResMut<TileEntityMap>,
) {
    // Takes items from ground and adds them to actor inventory
    // Do not enter PickingUp state when too far from target; no checks
    // Do not enter this state without a target object, no checks on unwrap
    for (actor, mut inventory, goal) in actors.iter_mut() {
        if inventory.is_full() {
            // Drop something?
        }
        let object = goal.parameters.target.unwrap();
        inventory.add(object);

        // Remove the item from the ground
        // TODO: Remove entity from tileentitymap
        commands
            .entity(goal.parameters.target.unwrap())
            .remove::<Position>();
        match goal.action {
            Action::Eat => {
                commands.entity(actor).remove::<PickingUp>().insert(Eating);
            }
            _ => todo!(),
        }
    }
}
fn eat(
    commands: Commands,
    mut actors: Query<
        (Entity, &mut Inventory, &mut Goal, &mut Status),
        With<Eating>,
    >,
) {
    // Eats an object out of the inventory
    // Do not enter Eating state without target object in inventory
    for (actor, mut inventory, mut goal, mut status) in actors.iter_mut() {
        // TODO: I have to figure out how to get access to the food's
        // nutrition_value here
    }
}
