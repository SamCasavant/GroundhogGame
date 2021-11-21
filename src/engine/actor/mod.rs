use bevy::prelude::*;

use crate::engine::world;

mod ai;

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
        .insert_resource(FrozenTimer(world::time::GameTime::from_stamp(
            &world::time::Stamp {
                day:    0,
                hour:   6,
                minute: 0,
                second: 0,
                frame:  0,
            },
        )))
        .add_system_set(
            SystemSet::new()
                .label("status update")
                .before("ai")
                .with_system(animal_processes.system())
                .with_system(unfreeze.system()),
        )
        .add_plugin(ai::AIPlugin);
    }
}

pub struct Inventory {
    // Every actor should have an inventory component; animals can have an
    // inventory of size 1.
    pub contents: Vec<Entity>,
    pub capacity: usize,
}
impl Inventory {
    fn contains(
        &self,
        entity: Entity,
    ) -> bool {
        self.contents.contains(&entity)
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
        entity: Entity,
    ) -> Option<Entity> {
        for index in 0..self.contents.len() {
            if self.contents[index] == entity {
                self.contents.remove(index);
                return Some(entity);
            }
        }
        None
    }
    fn is_full(&self) -> bool { self.contents.len() >= self.capacity }
}

pub struct Intelligent; // Intelligent actor component

pub struct Status {
    // Used for keeping track of actor state, values are primarily used for
    // priority of subsequent action
    pub hunger:   u32,
    pub laziness: u32, /* Actor will prefer inaction over actions with lower
                        * priority than laziness */
    pub thirst:   u32,
}

pub struct Animal; // Component marker for animals (including humans)

struct AnimalTimer(world::time::GameTime);

fn animal_processes(
    // Updates animal-inherent statuses; hunger, thirst, etc.
    mut query: Query<&mut Status, With<Animal>>,
    game_time: Res<world::time::GameTime>,
    mut timer: ResMut<AnimalTimer>,
) {
    if timer.0 <= *game_time {
        debug!("Timer: {:?}", timer.0);
        debug!("Game Time {:?}", *game_time);
        for mut status in query.iter_mut() {
            status.hunger += 1;
        }
        *timer = AnimalTimer(game_time.copy_and_tick(300));
    }
}

pub struct Frozen(u32); // Value represents duration in frames (1/60th second)

struct FrozenTimer(world::time::GameTime);

fn unfreeze(
    mut commands: Commands,
    mut frozen_entities: Query<(Entity, &mut Frozen)>,
    mut timer: ResMut<FrozenTimer>,
    game_time: Res<world::time::GameTime>,
) {
    if timer.0 <= *game_time {
        for (entity, mut freeze_duration) in frozen_entities.iter_mut() {
            freeze_duration.0 -= 1;
            if freeze_duration.0 == 0 {
                commands.entity(entity).remove::<Frozen>();
            }
        }
    }
    *timer = FrozenTimer(game_time.copy_and_tick(1));
}

pub struct Orientation(pub Direction); // TODO: Orientation hasn't really found a home yet.

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
