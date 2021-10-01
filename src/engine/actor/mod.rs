use bevy::prelude::*;

use crate::engine::world;

pub struct Routine {
    tasks: Option<Vec<ScheduledTask>>,
}

pub struct ScheduledTask {
    task: Task,
    time: world::time::GameTime,
}

#[derive(Copy, Clone)]
pub struct Task {
    action:     Action,
    parameters: ActionParameters,
    priority:   u32,
}

#[derive(Copy, Clone)]
pub enum Action {
    Wait,
    Eat,
}

#[derive(Default, Copy, Clone)]
pub struct ActionParameters {
    location: Option<world::Position>,
    target:   Option<Entity>,
}

pub struct Intelligent; // Intelligent actor component

pub struct Status {
    // Used for keeping track of actor state, values are primarily used for
    // priority of subsequent action
    hunger:   u32,
    laziness: u32, /* Actor will prefer inaction over actions with lower
                    * priority than laziness */
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
            },
        )))
        .add_system(animal_processes.system().label("preparation"))
        .add_system(choose_next_task.system().label("planning"))
        .add_system(move_actor.system().label("action"));
    }
}

fn choose_next_task(
    mut commands: Commands,
    mut query: Query<
        (Entity, &Status, &Routine),
        (With<Intelligent>, Without<Task>),
    >,
    time: Res<world::time::GameTime>,
) {
    for (entity, status, routine) in query.iter_mut() {
        let mut curtask = Task {
            action:     Action::Wait,
            parameters: ActionParameters {
                ..Default::default()
            },
            priority:   status.laziness,
        };
        // TODO: Change priority calculation for scheduled events to incorporate
        // eta
        match &routine.tasks {
            Some(tasks) => {
                let priority = time.how_soon(tasks[0].time);
                if priority > curtask.priority {
                    curtask = tasks[0].task;
                }
            }
            None => todo!(),
        }
        if status.hunger > curtask.priority {
            curtask = Task {
                action:     Action::Eat,
                parameters: ActionParameters::default(),
                priority:   status.hunger,
            }
        }
        commands.entity(entity).insert(curtask);
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
        *timer = AnimalTimer(game_time.copy_and_tick(60));
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
    weightmap: Res<world::TileWeightMap>,
    game_time: Res<world::time::GameTime>,
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut world::time::GameTime,
        &mut world::Position,
        &mut Orientation,
        &mut world::Destination,
        &mut pathfinding::Path,
    )>,
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
            let mut next_step = path.0[0];
            // If an entity's path is blocked by another entity, first try
            // to find an alternate move that gets closer to
            // the destination.
            // if entity_map.map.contains_key(&next_step)
            //     && let Some(e) entity_map.map[&next_step]
            // {
            //     commands.entity(entity).remove::<pathfinding::Path>();
            //     let mut cur_distance = (destination.0.x - position.x).pow(2)
            //         + (destination.0.y - position.y).pow(2);
            //     next_step = *position;
            //     let mut temp_steps = pathfinding::neighbors_with_weights(
            //         &*position,
            //         &entity_map,
            //     );
            //     temp_steps.sort_by_key(|k| k.1);
            //     for step in &temp_steps {
            //         if !entity_map.map.contains_key(&step.0)
            //             || !entity_map.map[&step.0].occupied
            //         {
            //             let new_distance =
            //                 (destination.0.x - (position.x +
            // step.0.x)).pow(2)                     +
            // (destination.0.y - (position.y + step.0.y))
            //                         .pow(2);
            //             if new_distance < cur_distance {
            //                 cur_distance = new_distance;
            //                 next_step = step.0;
            //             }
            //         }
            //     }
            // } else {
            //     path.0.remove(0);
            // }

            match next_step {
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
            let mut tile = entity_map.map.entry(*position).or_insert(None);
            *position = next_step;
            tile = entity_map.map.entry(*position).or_insert(Some(entity));
            *timer = game_time.copy_and_tick(1);
        } else {
            *timer = game_time.copy_and_tick(0);
        }
        if *destination == *position {
            commands.entity(entity).remove::<world::Destination>();
        }
    }
}
