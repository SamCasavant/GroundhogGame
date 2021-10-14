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
        .add_system(choose_next_task.system().label("planning"))
        .add_system(move_actor.system().label("action").after("planning"));
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
                let priority = time.how_soon(tasks[0].time) / 60;
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
    mut query: Query<(
        Entity,
        &mut world::time::GameTime,
        &mut world::Position,
        &mut Orientation,
        &world::Destination,
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
                .remove::<world::Destination>()
                .remove::<pathfinding::Path>();
        }
    }
}
