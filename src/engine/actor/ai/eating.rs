use bevy::prelude::*;

use crate::engine::world::{item, Destination, Position};
use crate::engine::{actor::{ai::{Moving, PickingUp,
                                 Target},
                            Inventory, Status},
                    world::item::NutritionValue};

pub struct EatGoal;

pub struct FindingFood;

pub struct Eating;

pub fn find_food_system(
    mut commands: Commands,
    mut actors: Query<
        (Entity, &Position),
        (With<FindingFood>, Without<Target>),
    >,
    foods: Query<(Entity, &Position), With<item::NutritionValue>>,
) {
    // Finds a food object for every entity that wants food
    // Actors should not enter this state with food in their inventory; it will
    // be ignored
    // TODO: This is about the slowest way this can work OPTIMIZE ME!
    for (actor, position) in actors.iter_mut() {
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
        if let Some(food) = selected_food {
            let target = Target(food);
            let target_location = food_location.unwrap();
            commands.entity(actor).remove::<FindingFood>();
            if position == &target_location {
                debug!(
                    "Entity {:?} found target {:?} at their location, \
                     switching active state to Picking Up.",
                    actor, food
                );
                commands.entity(actor).insert(target).insert(PickingUp);
            } else {
                debug!(
                    "Entity {:?} found target {:?} setting target, \
                     destination and switching active state to Moving.",
                    actor, food
                );
                commands
                    .entity(actor)
                    .insert(target)
                    .insert(Moving)
                    .remove::<Destination>()
                    .insert(Destination(target_location));
            }
        }
    }
}

pub fn eat_system(
    mut commands: Commands,
    mut actors: Query<
        (Entity, &mut Inventory, &Target, &mut Status),
        With<Eating>,
    >,
    foods: Query<&item::NutritionValue>,
) {
    // Eats an object out of the inventory
    // Do not enter Eating state without target object in inventory
    for (actor, mut inventory, target, mut status) in actors.iter_mut() {
        debug!("Entity: {:?} is eating {:?}", actor, target.0);
        let food_entity = inventory.remove(&target.0).unwrap();
        match foods.get(food_entity) {
            Ok(item::NutritionValue(value)) => {
                if value >= &status.hunger {
                    status.hunger = 0;
                } else {
                    status.hunger -= value
                }
                commands.entity(food_entity).despawn();
            }
            Err(error) => panic!("{}", error),
        }
        commands
            .entity(actor)
            .remove::<EatGoal>()
            .remove::<Eating>()
            .remove::<Target>();
    }
}

pub fn eating_ai(
    mut commands: Commands,
    query: Query<
        (Entity, &Inventory, &Position),
        (With<EatGoal>, Without<(PickingUp, Eating, FindingFood)>),
    >,
    target_query: Query<&Target>,
    food_query: Query<(Entity, &NutritionValue, &Position)>,
) {
    // Eating AI Flow:
    // FindingFood -> Moving -> PickingUp -> Eating
    // This system identifies which step we're on and assigns it to the actor
    // Currently this also handles stopping movement, but TODO not anymore

    for (actor, inventory, position) in query.iter() {
        match target_query.get(actor) {
            Ok(target) => {
                // Entity already has target
                match food_query.get(target.0) {
                    Ok((_food, _nutritionvalue, location)) => {
                        // Target is on the map
                        if position == location {
                            debug!(
                                "Entity {:?} is at target {:?}, setting \
                                 active task to PickingUp.",
                                actor, target.0
                            );
                            commands
                                .entity(actor)
                                .remove::<Moving>()
                                .insert(PickingUp);
                        } else {
                            debug!(
                                "Entity {:?} has target {:?}, setting active \
                                 state to Moving.",
                                actor, target.0
                            );
                            commands
                                .entity(actor)
                                .insert(Moving)
                                .remove::<Destination>()
                                .insert(Destination(*location));
                        }
                    }

                    Err(_) => {
                        if inventory.contains(&target.0) {
                            debug!(
                                "Entity {:?} has target {:?} in inventory, \
                                 switching active state to Eating.",
                                actor, target.0
                            );
                            commands.entity(actor).insert(Eating);
                        } else {
                            debug!(
                                "Entity {:?} has lost track of target {:?}, \
                                 removing target and cancelling movement.",
                                actor, target.0
                            );
                            commands
                                .entity(actor)
                                .remove::<Target>()
                                .remove::<Moving>();
                        }
                    }
                }
            }
            Err(_) => {
                // Entity needs target
                let mut owned_food = None;
                let mut max_nutrition_value = 0;
                for object in &inventory.contents {
                    if let Ok(result) = food_query.get(*object) {
                        let (food, nutrition_value, _position) = result;
                        // TODO: Max nutrition value is a strange thing to
                        // optimize for here.
                        if nutrition_value.0 > max_nutrition_value {
                            max_nutrition_value = nutrition_value.0;
                            owned_food = Some(food);
                        }
                    }
                }
                if let Some(food) = owned_food {
                    debug!(
                        "Entity {:?} has target {:?} in inventory, switching \
                         active state to Eating.",
                        actor, food
                    );
                    commands.entity(actor).insert(Target(food)).insert(Eating);
                } else {
                    debug!(
                        "Entity {:?} has no food readily available, switching \
                         active state to FindingFood.",
                        actor
                    );
                    commands.entity(actor).insert(FindingFood);
                }
            }
        }
    }
}
