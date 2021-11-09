use bevy::prelude::*;

use crate::engine::world;

mod AI;

pub struct Moving;

pub struct Inventory {
    pub contents: Vec<Entity>,
    pub capacity: usize,
}
impl Inventory {
    fn contains(
        &self,
        entity: &Entity,
    ) -> bool {
        self.contents.contains(entity)
    }
    fn add(
        &mut self,
        entity: Entity,
    ) -> bool {
        if self.contents.len() < self.capacity {
            self.contents.push(entity);
            true
        } else {
            false
        }
    }
    fn remove(
        &mut self,
        entity: &Entity,
    ) -> Option<Entity> {
        for index in 0..self.capacity {
            if self.contents[index] == *entity {
                self.contents.remove(index);
                return Some(*entity);
            }
        }
        return None;
    }
    fn is_full(&self) -> bool { self.contents.len() >= self.capacity }
}

pub struct Intelligent; // Intelligent actor component

pub struct Status {
    // Used for keeping track of actor state, values are primarily used for
    // priority of subsequent action
    pub hunger: u32,
    laziness:   u32, /* Actor will prefer inaction over actions with lower
                      * priority than laziness */
    pub thirst: u32,
}

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(
        &self,
        app: &mut AppBuilder,
    ) {
        app.insert_resource(AnimalTimer(world::time::GameTime::from_stamp(
            &world::time::Stamp {
                day:    0,
                hour:   6,
                minute: 0,
                second: 0,
                frame:  0,
            },
        )))
        .add_system(pathfinding::plan_path.system().label("preparation"))
        .add_system(
            pathfinding::local_avoidance
                .system()
                .label("planning")
                .after("preparation"),
        )
        .add_system(animal_processes.system().label("preparation"))
        .add_system(AI::choose_next_goal.system().label("planning"))
        .add_system(move_actor.system().label("action").after("planning"));
    }
}

struct Animal; // Component marker for animals (including humans)

struct AnimalTimer(world::time::GameTime);

fn animal_processes(
    // Updates animal-inherent statuses; hunger, thirst, etc.
    mut query: Query<&mut Status, With<Animal>>,
    game_time: Res<world::time::GameTime>,
    mut timer: ResMut<AnimalTimer>,
) {
    if timer.0 <= *game_time {
        for mut status in query.iter_mut() {
            status.hunger += 1;
        }
        *timer = AnimalTimer(game_time.copy_and_tick_seconds(60));
    }
}

pub struct Orientation(pub Direction);

#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
pub enum Direction {
    Up,
    UpLeft,
    UpRight,
    Down,
    DownLeft,
    DownRight, // This is downright.
    Left,
    Right,
}

mod pathfinding;

pub fn move_actor(
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
            commands.entity(entity).remove::<pathfinding::Path>();
        } else if *timer <= *game_time {
            let next_step = path.0.remove(0);
            if path.0.is_empty() {
                commands.entity(entity).remove::<pathfinding::Path>();
            }
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
            *timer = game_time.copy_and_tick_seconds(1);
        } else {
            *timer = game_time.copy_and_tick_seconds(0);
        }
        if *destination == *position {
            commands
                .entity(entity)
                .remove::<Moving>()
                .remove::<pathfinding::Path>();
        }
    }
}
