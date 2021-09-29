use bevy::prelude::*;
use crate::world;

pub struct Routine{
    tasks: Option<Vec<ScheduledTask>>
}

pub struct ScheduledTask{
    task: Task,
    time: world::time::GameTime
}

#[derive(Copy, Clone)]
pub struct Task{
    action: Action,
    parameters: ActionParameters,
    priority: u32
}

#[derive(Copy, Clone)]
pub enum Action{
    Wait,
    Eat,
}

#[derive(Default, Copy, Clone)]
pub struct ActionParameters{
    location: Option<world::Position>,
    target: Option<Entity>
}

pub struct Intelligent; //Intelligent actor component

pub struct NeedsTask; //Component to mark entities for task initialization

pub struct Status{ //Used for keeping track of actor state, values are primarily used for priority of subsequent action
    hunger: u32,
    laziness: u32, //Actor will prefer inaction over actions with lower priority than laziness
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
    mut query: Query<
        (Entity, 
        &Status, 
        &Routine), 
        (With<Intelligent>, 
        Without<Task>)>,
    time: Res<world::time::GameTime>,)
{
    for (entity, status, routine) in query.iter_mut(){
        let mut curtask = Task{action: Action::Wait, parameters: ActionParameters{..Default::default()}, priority: status.laziness};
        //TODO: Change priority calculation for scheduled events to incorporate eta
        match &routine.tasks {
            Some(tasks) => { 
                let priority = time.how_soon(tasks[0].time); 
                if priority > curtask.priority {
                    curtask = tasks[0].task.clone();
            }},
            None => todo!(),
        }
        if status.hunger > curtask.priority {
            curtask = Task{action: Action::Eat, parameters: ActionParameters{..Default::default()}, priority: status.hunger}
        }
        commands.entity(entity).insert(curtask);

    }

}

struct Animal; //Component marker for animals (including humans)

struct AnimalTimer(Timer);

fn animal_processes( //Updates animal-inherent statuses; hunger, thirst, etc.
    mut query: Query<&mut Status, With<Animal>>,
    time: Res<Time>,
    mut timer: ResMut<AnimalTimer>
){
    if timer.0.tick(time.delta()).just_finished(){
        for mut status in query.iter_mut(){
            status.hunger += 1;
        }
    }
}