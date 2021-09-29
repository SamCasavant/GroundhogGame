use bevy::prelude::*;
use crate::world;

pub struct Routine{
    tasks: Option<Vec<(Task, Time)>, None>
}

pub struct Task{
    action: Action,
    parameters: ActionParameters,
    priority: u32
}

pub enum Action{
    Wait,
    Eat,
}

#[derive(Default)]
pub struct ActionParameters{
    location: Option<world::Position, None>,
    target: Option<Entity, None>
}

pub struct Intelligent; //Intelligent actor component

pub struct NeedsTask; //Component to mark entities for task initialization

pub struct Priority(u32); //Wrapper for code clarity

pub struct Status{
    hunger: Priority(u32),
    laziness: Priority(u32), //Actor will prefer inaction over actions with lower priority than laziness
}

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
        .insert_resource(AnimalTimer(Timer::from_seconds(5.0, true)))
        .add_system(animal_processes.system())
        .add_system(choose_next_task.system());
    }
}

fn choose_next_task(
    mut commands: Commands, 
    query: Query<
        (Entity, 
        &Status, 
        &Routine), 
        With<Intelligent>, 
        Without<HasTask>>)
{
    for (entity, status, routine) in query.iter_mut(){
        let mut curtask = Task{action: Action::Wait, parameters: ActionParameters(Default::default()), priority: Priority(status.laziness)};
        match routine {
            Some(task) => { 
                let priority = Priority(min(task[0].1 - CURRENT_TIME, 0));
                if priority > curtask.priority {
                    curtask = task[0];
            }},
            None => todo!(),
        }
        if status.hunger > curtask.priority {
            curtask = Task{action: Action::Eat, parameters: ActionParameters(Default::default()), priority: Priority(status.hunger)}
        }

    }

}

struct Animal; //Component marker for animals (including humans)

struct AnimalTimer(Timer);

fn animal_processes( //Updates animal-inherent statuses; hunger, thirst, etc.
    query: Query<(&Status), With<Animal>>,
    time: Res<Time>,
    mut timer: ResMut<AnimalTimer>
){
    if timer.0.tick(time.delta()).just_finished(){
        for mut status in query.iter_mut(){
            status.hunger += 1;
        }
    }
}