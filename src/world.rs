use bevy::{prelude::*, render::camera::Camera};
use bevy_tiled::TiledMapCenter;
use rand::Rng;
use std::collections::HashMap;

#[derive(Bundle)]
struct ActorComponents {
    spritesheet: SpriteSheetComponents,
    position: Position,
    identity: Identity,
    destination: Destination,
}

#[derive(Default)]
struct PlannedSteps {
    steps: HashMap<Position, bevy::prelude::Entity>,
}

#[derive(Default)]
struct TileMap {
    map: HashMap<Position, Tile>,
}

#[derive(Debug)]
struct Tile {
    occupied: bool,
    ground_type: GroundType,
}

#[derive(Debug)]
enum GroundType {
    ShortGrass,
    TallGrass,
    Sidewalk,
    Path,
    Street,
    Crosswalk,
    Obstacle,
}

const TILE_WIDTH: f32 = 16.0;

#[derive(Clone)]
struct Identity {
    specific: bool,
    name: String,
}

extern crate pathfinding;
use pathfinding::prelude::{absdiff, astar};
#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
struct Position {
    x: i64,
    y: i64,
}

use std::convert::TryInto;

#[derive(Debug, Copy, Clone, PartialEq, Hash, Eq)]
enum Direction {
    Up,
    UpLeft,
    UpRight,
    Down,
    DownLeft,
    DownRight, //This is downright.
    Left,
    Right,
}

struct Orientation(Direction);

#[derive(Debug, Copy, Clone)]
struct Destination(Position);

impl PartialEq<Position> for Destination {
    fn eq(&self, other: &Position) -> bool {
        self.0.x == other.x && self.0.y == other.y
    }
}

struct Path(Vec<Position>);

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_resource(PlannedSteps {
            steps: HashMap::<Position, bevy::prelude::Entity>::new(),
        })
        .add_resource(TileMap {
            map: HashMap::<Position, Tile>::new(),
        })
        .add_plugin(bevy_tiled::TiledMapPlugin)
        .add_startup_system(setup.system())
        .add_system(camera_movement.system())
        .add_system(animate_sprite_system.system())
        .add_system(move_actor.system());
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    textures: ResMut<Assets<Texture>>,
    texture_atlases: ResMut<Assets<TextureAtlas>>,
    tilemap: ResMut<TileMap>,
) {
    commands
        //.spawn(bevy_tiled::TiledMapComponents {
        //    map_asset: asset_server.load("assets/maps/test.tmx").unwrap(),
        //    center: TiledMapCenter(true),
        //    ..Default::default()
        //})
        .spawn(Camera2dComponents::default())
        .spawn(UiCameraComponents::default());
    add_roads(tilemap);
    add_people(commands, asset_server, textures, texture_atlases);
}

fn animate_sprite_system(mut query: Query<(&mut TextureAtlasSprite, &mut Orientation)>) {
    for (mut sprite, orientation) in &mut query.iter() {
        match orientation.0 {
            Direction::Up => sprite.index = 5,
            Direction::Down => sprite.index = 1,
            Direction::Left => sprite.index = 10,
            Direction::Right => sprite.index = 13,
            _ => sprite.index = 15,
        }
    }
}

fn init_sprite_sheet(
    path: String,
    asset_server: &Res<AssetServer>,
    mut textures: &mut ResMut<Assets<Texture>>,
    texture_atlases: &mut ResMut<Assets<TextureAtlas>>,
    position: Position,
) -> SpriteSheetComponents {
    let texture_handle = asset_server.load_sync(&mut textures, path).unwrap();
    let texture = textures.get(&texture_handle).unwrap();
    let texture_atlas = TextureAtlas::from_grid(texture_handle, texture.size, 4, 4);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    let mut transform = Transform::from_scale(0.1);
    let translation = transform.translation_mut();
    *translation.x_mut() += position.x as f32 * TILE_WIDTH;
    *translation.y_mut() += position.y as f32 * TILE_WIDTH;
    SpriteSheetComponents {
        texture_atlas: texture_atlas_handle,
        transform: transform,
        ..Default::default()
    }
}

fn move_actor(
    mut tilemap: ResMut<TileMap>,
    mut query: Query<(
        Entity,
        &mut Timer,
        &mut Position,
        &mut Orientation,
        &mut Destination,
        &mut Path,
        &mut Transform,
    )>,
) {
    for (entity, timer, mut position, mut orientation, mut destination, mut path, mut transform) in
        &mut query.iter()
    {
        if path.0.len() < 1 {
            *path = plan_path(*position, *destination, &tilemap);
        }
        if timer.finished {
            if path.0.len() > 0 {
                let mut next_step = path.0[0];
                // If an entity's path is blocked by another entity, first try to find an alternate move that gets closer to the destination.
                if tilemap.map.contains_key(&next_step) == true
                    && tilemap.map.get(&next_step).unwrap().occupied == true
                {
                    next_step = *position;
                    let mut temp_steps = move_weights(&*position, &tilemap);
                    temp_steps.sort_by_key(|k| k.1);
                    for step in temp_steps.iter() {
                        if tilemap.map.contains_key(&step.0) == false
                            || tilemap.map.get(&step.0).unwrap().occupied == false
                        {
                            if absdiff(destination.0.x, position.x)
                                - absdiff(destination.0.x, step.0.x)
                                + absdiff(destination.0.y, position.y)
                                - absdiff(destination.0.y, step.0.y)
                                > 0
                            {
                                next_step = step.0;
                                break;
                            }
                        }
                    }
                    //Failing that, try to find a move that moves closer in either X or Y.
                    if next_step == *position {
                        for step in temp_steps.iter() {
                            if tilemap.map.contains_key(&step.0) == false
                                || tilemap.map.get(&step.0).unwrap().occupied == false
                            {
                                if absdiff(destination.0.x, position.x)
                                    - absdiff(destination.0.x, step.0.x)
                                    > 0
                                    || absdiff(destination.0.y, position.y)
                                        - absdiff(destination.0.y, step.0.y)
                                        > 0
                                {
                                    next_step = step.0;
                                    break;
                                }
                            }
                        }
                    }
                    if next_step != *position{
                        //(If the corrections above found an alternate move)
                        //If we've moved off of our path, we'll need to get a new one on the next loop. 
                        //(This should be changed to only if we cannot immediately rejoin path.)
                        *path = Path(Vec::new());
                    }
                } else {
                    path.0.remove(0);
                }
                let translation = transform.translation_mut();
                *translation.x_mut() += (next_step.x - position.x) as f32 * TILE_WIDTH;
                *translation.y_mut() += (next_step.y - position.y) as f32 * TILE_WIDTH;
                match next_step {
                    Position { x: 1, .. } => *orientation = Orientation(Direction::Up),
                    Position { x: -1, .. } => *orientation = Orientation(Direction::Down),
                    Position { y: 1, .. } => *orientation = Orientation(Direction::Right),
                    Position { y: -1, .. } => *orientation = Orientation(Direction::Left),
                    _ => (),
                }
                let mut tile = tilemap.map.entry(*position).or_insert(Tile {
                    ground_type: GroundType::Path,
                    occupied: false,
                });
                tile.occupied = false;
                *position = next_step;
                tile = tilemap.map.entry(*position).or_insert(Tile {
                    ground_type: GroundType::Path,
                    occupied: true,
                });
                tile.occupied = true;
            }
            if *destination == *position {
                let mut rng = rand::thread_rng();
                destination.0.x = rng.gen_range(-100, 100);
                destination.0.y = rng.gen_range(-100, 100);
                *path = Path(Vec::new());
            }
        }
    }
}

fn plan_path(
    position: Position,
    destination: Destination,
    tilemap: &ResMut<TileMap>,
) -> Path {
    let mut plan = astar(
        &position,
        |p| move_weights(p, tilemap),
        |p| {
            (absdiff(p.x, destination.0.x) + absdiff(p.y, destination.0.y))
                .try_into()
                .unwrap()
        },
        |p| *p == destination.0,
    );
    if let Some(p) = plan {
        let mut path = p.0;
        path.remove(0);
        if path.len() > 0 {
            return Path(path);
        }
    }
    let mut path = Vec::new();
    path.push(position);
    return Path(path);
}

fn move_weights(position: &Position, tilemap: &ResMut<TileMap>) -> Vec<(Position, u32)> {
    let &Position { x, y } = position;
    let mut weights = Vec::<(Position, u32)>::new();
    for next_x in [-1, 0, 1].iter() {
        for next_y in [-1, 0, 1].iter() {
            let next_position = Position {
                x: x + next_x,
                y: y + next_y,
            };
            let tile_weight = tile_weight(next_position, tilemap);
            if tile_weight != u32::MAX {
                weights.push((next_position, tile_weight));
            }
        }
    }
    return weights;
}

fn tile_weight(position: Position, tilemap: &ResMut<TileMap>) -> u32 {
    let mut weight = 1;
    if tilemap.map.contains_key(&position) {
        match tilemap.map.get(&position).unwrap() {
            Tile {
                ground_type: GroundType::Obstacle,
                ..
            } => weight = u32::MAX,
            Tile {
                ground_type: GroundType::Street,
                ..
            } => weight = 10,
            _ => weight = 1,
        }
    }
    weight
}

fn add_people(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut textures: ResMut<Assets<Texture>>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let position = Position { x: 0, y: 0 };
    
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
    let sprite_sheet = init_sprite_sheet(
        "assets/sprites/NPC1 (2).png".to_string(),
        &asset_server,
        &mut textures,
        &mut texture_atlases,
        position,
    );
    spawn_actor(
        &mut commands,
        Identity {
            specific: true,
            name: "Grumph Torgi".to_string(),
        },
        position,
        sprite_sheet,
    );
    commands.insert_one(
        commands.current_entity().unwrap(),
        Destination(Position { x: 0, y: 0 }),
    );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
//     let sprite_sheet = init_sprite_sheet(
//         "assets/sprites/NPC1 (2).png".to_string(),
//         &asset_server,
//         &mut textures,
//         &mut texture_atlases,
//         position,
//     );
//     spawn_actor(
//         &mut commands,
//         Identity {
//             specific: true,
//             name: "Grumph Torgi".to_string(),
//         },
//         position,
//         sprite_sheet,
//     );
//     commands.insert_one(
//         commands.current_entity().unwrap(),
//         Destination(Position { x: 0, y: 0 }),
//     );
}

fn spawn_actor(
    commands: &mut Commands,
    identity: Identity,
    position: Position,
    sprite_sheet: SpriteSheetComponents,
) -> Entity {
    commands
        .spawn((
            identity,
            position,
            Path(vec![]),
            Orientation(Direction::Down),
        ))
        .with_bundle(sprite_sheet)
        .with(Timer::from_seconds(0.05, true));
    commands.current_entity().unwrap()
}

fn add_roads(mut tilemap: ResMut<TileMap>) {
    for x in 0..4 {
        for y in 0..30 {
            tilemap.map.insert(
                Position { x: x, y: y },
                Tile {
                    occupied: false,
                    ground_type: GroundType::Street,
                },
            );
        }
    }
    for x in 0..4 {
        tilemap.map.insert(
            Position { x: x, y: 15 },
            Tile {
                occupied: false,
                ground_type: GroundType::Crosswalk,
            },
        );
    }
}

fn camera_movement(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&Camera, &mut Transform)>,
) {
    for (_, mut transform) in &mut query.iter() {
        let mut direction = Vec3::zero();
        if keyboard_input.pressed(KeyCode::A) {
            direction -= Vec3::new(1.0, 0.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::E) {
            direction += Vec3::new(1.0, 0.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::Comma) {
            direction += Vec3::new(0.0, 1.0, 0.0);
        }

        if keyboard_input.pressed(KeyCode::O) {
            direction -= Vec3::new(0.0, 1.0, 0.0);
        }

        let translation = transform.translation();
        transform.set_translation(translation + time.delta_seconds * direction * 1000.0);
    }
}
