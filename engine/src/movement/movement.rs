// The move_actor function handles final adjustments to the path in the event of a potential collision with another entity.
// It acts on entities with a position, destination, and path (vector of positions).
// Positions are shifted off of the path when a step is taken. When the path is completed or needs to be regenerated, the component is removed.
// crate::world::plan_path() produces a new path for entities that have none.
//
// TODO:
// Allow actors to rejoin path rather than starting over in the event of a correction.
// Create a system for near objectives to save aStar effort.
// Find a better way to store currently occupied positions? (Using hashmap for now)
// Improve GroundType integration with tile map.
// Integrate tile system with bevy_ECS_tiles
//

pub(crate) use bevy::prelude::*;

extern crate pathfinding;

use crate::world;
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

pub struct MovementPlugin;

impl Plugin for MovementPlugin {
   fn build(
      &self,
      app: &mut AppBuilder,
   ) {
      app.add_system(move_actor.system().label("action"));
   }
}

pub fn move_actor(
   mut tilemap: ResMut<world::TileMap>,
   game_time: Res<world::time::GameTime>,
   mut commands: Commands,
   mut query: Query<(
      Entity,
      &mut world::time::GameTime,
      &mut world::Position,
      &mut Orientation,
      &mut world::Destination,
      &mut world::Path,
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
      if path.0.len() < 1 {
         commands.entity(entity).remove::<world::Path>();
      }
      if *timer <= *game_time {
         let mut next_step = path.0[0];
         // If an entity's path is blocked by another entity, first try to find an alternate move that gets closer to the destination.
         if tilemap.map.contains_key(&next_step) == true
            && tilemap.map.get(&next_step).unwrap().occupied == true
         {
            let mut cur_distance = (destination.0.x - position.x)
               ^ 2 + (destination.0.y - position.y)
               ^ 2;
            next_step = *position;
            let mut temp_steps = world::move_weights(&*position, &tilemap);
            temp_steps.sort_by_key(|k| k.1);
            for step in temp_steps.iter() {
               if tilemap.map.contains_key(&step.0) == false
                  || tilemap.map.get(&step.0).unwrap().occupied == false
               {
                  let new_distance =
                     (destination.0.x - (position.x + step.0.x)).pow(2)
                        + (destination.0.y - (position.y + step.0.y)).pow(2);
                  if new_distance < cur_distance {
                     cur_distance = new_distance;
                     next_step = step.0;
                  }
               }
            }
            if next_step != *position {
               //(If the corrections above found an alternate move)
               // If we've moved off of our path, we'll need to get a new one on the next loop.
               //(This should be changed to only if we cannot immediately rejoin path.)
               commands.entity(entity).remove::<world::Path>();
            }
         } else {
            path.0.remove(0);
         }

         match next_step {
            world::Position { x: 1, .. } => {
               *orientation = Orientation(Direction::Up)
            }
            world::Position { x: -1, .. } => {
               *orientation = Orientation(Direction::Down)
            }
            world::Position { y: 1, .. } => {
               *orientation = Orientation(Direction::Right)
            }
            world::Position { y: -1, .. } => {
               *orientation = Orientation(Direction::Left)
            }
            _ => (),
         }
         let mut tile = tilemap.map.entry(*position).or_insert(world::Tile {
            ground_type: world::GroundType::Path,
            occupied:    false,
         });
         tile.occupied = false;
         *position = next_step;
         tile = tilemap.map.entry(*position).or_insert(world::Tile {
            ground_type: world::GroundType::Path,
            occupied:    true,
         });
         tile.occupied = true;
         *timer = game_time.copy_and_tick(1);
      } else {
         *timer = game_time.copy_and_tick(0);
      }
      if *destination == *position {
         commands.entity(entity).remove::<world::Destination>();
      }
   }
}
