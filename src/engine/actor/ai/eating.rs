use bevy::prelude::*;

use crate::engine::actor::{ai::{Moving, PickingUp, Target},
                           Inventory, Status};
use crate::engine::world::{item, Destination, Position};

pub struct EatGoal;

pub struct FindingFood;

pub struct Eating;

pub fn find_food_system(
    mut commands: Commands,
    mut actors: Query<
        (Entity, &Position, &Destination),
        (With<FindingFood>, Without<Target>),
    >,
    foods: Query<(Entity, &Position), With<item::NutritionValue>>,
) {
    // Finds a food object for every entity that wants food
    // Actors should not enter this state with food in their inventory; it will
    // be ignored
    // TODO: This is about the slowest way this can work OPTIMIZE ME!
    for (actor, position, mut destination) in actors.iter_mut() {
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
            let target = Target(selected_food.unwrap());
            destination = &Destination(food_location.unwrap());
            commands.entity(actor).remove::<FindingFood>();
            if position.neighbors(1).contains(&food_location.unwrap()) {
                commands.entity(actor).insert(target).insert(PickingUp);
            } else {
                commands.entity(actor).insert(Moving);
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
        // TODO: I have to figure out how to get access to the food's
        // nutrition_value here
        let food_entity = inventory.remove(&target.0).unwrap();
        let nutrition_value = foods.get(food_entity);
        match nutrition_value {
            Ok(item::NutritionValue(value)) => {
                if value >= &status.hunger {
                    status.hunger = 0;
                } else {
                    status.hunger = status.hunger - value
                }
                commands.entity(food_entity).despawn();
                commands
                    .entity(actor)
                    .remove::<EatGoal>()
                    .remove::<Eating>()
                    .remove::<Target>();
            }
            Err(error) => panic!(error),
        }
    }
}

pub fn eating_ai(
    mut commands: Commands,
    query: Query<
        (Entity, &Inventory, &Position),
        (With<EatGoal>, Without<(PickingUp, Eating, FindingFood)>),
    >,
    target_query: Query<(&Target, &Position)>,
    food_query: Query<(Entity, &item::NutritionValue)>,
) {
    // Eating AI Flow:
    // FindingFood -> Moving -> PickingUp -> Eating
    // This system identifies which step we're on and assigns it to the actor
    // Currently this also handles stopping movement, but TODO not anymore
    for (actor, inventory, position) in query.iter() {
        match target_query.get(actor) {
            Ok(result) => {
                let (target, location) = result;
                if inventory.contains(&target.0) {
                    commands.entity(actor).insert(Eating);
                } else if position.neighbors(1).contains(location) {
                    commands.entity(actor).remove::<Moving>().insert(PickingUp);
                } else {
                    commands
                        .entity(actor)
                        .insert(Moving)
                        .insert(Destination(*location));
                }
            }
            Err(_) => {
                let mut owned_food = None;
                let mut max_nutrition_value = 0;
                for object in &inventory.contents {
                    match food_query.get(*object) {
                        Ok(result) => {
                            let (food, nutrition_value) = result;
                            if nutrition_value.0 > max_nutrition_value {
                                max_nutrition_value = nutrition_value.0;
                                owned_food = Some(food);
                            }
                        }
                        Err(_) => todo!(),
                    }
                }
                if owned_food.is_some() {
                    commands
                        .entity(actor)
                        .insert(Target(owned_food.unwrap()))
                        .insert(Eating);
                } else {
                    commands.entity(actor).insert(FindingFood);
                }
            }
        }
    }
}
