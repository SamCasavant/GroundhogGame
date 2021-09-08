use bevy::{prelude::*, sprite::SpriteSettings};

use std::collections::{btree_map::Range, HashMap};

pub mod movement;

#[derive(Bundle)]
struct ActorComponents {
    spritesheet: SpriteSheetBundle,
    position: movement::Position,
    identity: Identity,
    destination: movement::Destination,
}

pub fn spawn_actor(
    commands: &mut Commands,
    identity: Identity,
    position: movement::Position,
    sprite_sheet: SpriteSheetBundle,
) {
    commands
        .spawn()
        .insert(identity)
        .insert(position)
        .insert(movement::Path(vec![]))
        .insert(movement::Orientation(movement::Direction::Down))
        .insert(movement::Destination(movement::Position { x: 0, y: 0 }))
        .insert_bundle(sprite_sheet)
        .insert(Timer::from_seconds(0.1, true));
}

#[derive(Clone)]
pub struct Identity {
    pub specific: bool,
    pub name: String,
}


