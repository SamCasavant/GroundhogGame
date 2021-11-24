// This module builds and draws sprites and spawns a camera. Its scope will
// likely increase
// Structure and terrain scale: 1 cube = 10in^3
// Object scale: 1 cube = (1/2)in^3

use bevy::prelude::*;
use bevy::render::draw::OutsideFrustum;
use dot_vox;
use palette;

use crate::engine::actor;
use crate::engine::world;
mod camera_movement;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(
        &self,
        app: &mut AppBuilder,
    ) {
        debug!("Initializing GraphicsPlugin");
        app.insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.05)))
            .insert_resource(Msaa { samples: 1 })
            .add_startup_system(load_assets.system())
            .add_startup_system(setup.system())
            .add_system(animate_sprite_system.system().label("render"))
            .add_system(camera_movement::pan_orbit_camera.system())
            .add_system(draw_world_voxels.system());
    }
}

pub const TILE_WIDTH: f32 = 64.0;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut ambient_light: ResMut<bevy::pbr::AmbientLight>,
) {
    // set up the camera
    let translation = Vec3::new(100.0, 100.0, 100.0);
    let radius = translation.length();
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_translation(translation)
                .looking_at(Vec3::ZERO, Vec3::Z),
            ..Default::default()
        })
        .insert(camera_movement::PanOrbitCamera {
            radius,
            ..Default::default()
        });
    // Draw a ground plane TODO: Add a terrain
    for x in 0..10 {
        for y in 0..1000 {
            commands
                .spawn()
                .insert(Voxel {
                    x,
                    y,
                    z: 0,
                    material: Color::rgb(0.2, 0.2, 0.2),
                })
                .insert(Visible);
            commands
                .spawn()
                .insert(Voxel {
                    x,
                    y,
                    z: 1,
                    material: Color::rgb(0.2, 0.15, 0.1),
                })
                .insert(Visible);
            commands
                .spawn()
                .insert(Voxel {
                    x,
                    y,
                    z: 2,
                    material: Color::rgb(0.1, 0.9, 0.3),
                })
                .insert(Visible);
        }
    }
    // Ambient light
    ambient_light.color = Color::WHITE;
    ambient_light.brightness = 0.05;
    // Sunlight
    commands.spawn_bundle(LightBundle {
        light: Light {
            color: Color::rgb(0.95, 0.8, 0.05),
            fov: 360.0,
            intensity: 99999.0,
            range: 500.0,
            ..Default::default()
        },
        transform: Transform::from_xyz(0.0, 0.0, 200.0),
        ..Default::default()
    });
}

pub struct Visible;

pub struct Voxel {
    // Voxels belonging to the main world
    x:        u32,
    y:        u32,
    z:        u32,
    material: Color,
}

pub struct ObjectVoxel {
    // Voxels belonging to entities, coordinates; position relative to base
    // Voxel
    x:        f32,
    y:        f32,
    z:        f32,
    material: Color,
}

fn load_assets(mut commands: Commands) {
    let mut voxel_count = 0;
    let building_assets = [
        "assets/models/buildings/barnhouse.vox",
        "assets/models/buildings/windybean.vox",
    ];
    let object_assets = ["assets/models/objects/pot.vox"];
    let character_assets = ["assets/models/characters/temp.vox"];
    let mut position = world::Position { x: 0, y: 0, z: 1 }; // TODO: Temporary; convert to world::Position when that is updated
    for _ in 0..1000 {
        // Fixme: This^ is just for benchmarking
        for asset in building_assets {
            // Load .vox file
            let building = dot_vox::load(asset).unwrap();
            let vox_palette = &building.palette;
            for voxel in &building.models[0].voxels {
                let color_u32 = palette::rgb::Rgb::<
                    palette::encoding::srgb::Srgb,
                    u8,
                >::from_u32::<palette::rgb::channels::Abgr>(
                    vox_palette[voxel.i as usize],
                );
                let color = Color::rgb(
                    color_u32.red as f32 / 255.0,
                    color_u32.green as f32 / 255.0,
                    color_u32.blue as f32 / 255.0,
                );
                commands.spawn().insert(Voxel {
                    x:        (voxel.x as u32) + position.x as u32,
                    y:        voxel.z as u32,
                    z:        voxel.y as u32,
                    material: color,
                });
            }
            position.x += building.models[0].size.x;
        }
        for asset in object_assets {
            // Load .vox file
            let object = dot_vox::load(asset).unwrap();
            let vox_palette = &object.palette;
            for voxel in &object.models[0].voxels {
                let color_u32 = palette::rgb::Rgb::<
                    palette::encoding::srgb::Srgb,
                    u8,
                >::from_u32::<palette::rgb::channels::Abgr>(
                    vox_palette[voxel.i as usize],
                );
                let color = Color::rgb(
                    color_u32.red as f32 / 255.0,
                    color_u32.green as f32 / 255.0,
                    color_u32.blue as f32 / 255.0,
                );
                commands.spawn().insert(ObjectVoxel {
                    x:        0.0,
                    y:        0.0,
                    z:        0.0,
                    material: color,
                });
                voxel_count += 1;
            }
            position.x += object.models[0].size.x.saturating_div(10);
        }
        for asset in character_assets {
            // Load .vox file
            let character = dot_vox::load(asset).unwrap();
            let vox_palette = &character.palette;
            for voxel in &character.models[0].voxels {
                let color_u32 = palette::rgb::Rgb::<
                    palette::encoding::srgb::Srgb,
                    u8,
                >::from_u32::<palette::rgb::channels::Abgr>(
                    vox_palette[voxel.i as usize],
                );
                let color = Color::rgb(
                    color_u32.red as f32 / 255.0,
                    color_u32.green as f32 / 255.0,
                    color_u32.blue as f32 / 255.0,
                );
                commands.spawn().insert(ObjectVoxel {
                    x:        0.0,
                    y:        0.0,
                    z:        0.0,
                    material: color,
                });
                voxel_count += 1;
            }
            position.x += character.models[0].size.x.saturating_div(10);
        }
    }
    println!("Voxels: {:?}", voxel_count);
}

fn draw_world_voxels(
    mut commands: Commands,
    query: Query<(Entity, &Voxel), (With<Visible>, Without<Mesh>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, voxel) in query.iter() {
        commands.entity(entity).insert_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(voxel.material.into()),
            transform: Transform::from_xyz(
                voxel.x as f32,
                voxel.y as f32,
                voxel.z as f32,
            ),
            ..Default::default()
        });
    }
}

fn animate_sprite_system(
    mut query: Query<(
        &mut TextureAtlasSprite,
        &mut Transform,
        &actor::Orientation,
        &world::Position,
        Without<OutsideFrustum>,
    )>
) {
    debug!("Running animate_sprite_system.");
    for (mut sprite, mut transform, orientation, position, _) in
        &mut query.iter_mut()
    {
        // Set sprite to match orientation
        match orientation.0 {
            actor::Direction::Up => sprite.index = 5,
            actor::Direction::Down => sprite.index = 1,
            actor::Direction::Left => sprite.index = 10,
            actor::Direction::Right => sprite.index = 13,
            actor::Direction::UpLeft => todo!(),
            actor::Direction::UpRight => todo!(),
            actor::Direction::DownLeft => todo!(),
            actor::Direction::DownRight => todo!(),
        }
        // Move sprite to match position
        let translation = Vec3::new(
            (position.x as f32).mul_add(TILE_WIDTH, TILE_WIDTH / 2.0),
            (position.y as f32).mul_add(TILE_WIDTH, TILE_WIDTH / 2.0),
            1.0, // Layer
        );
        transform.translation = translation;
    }
}

pub fn init_sprite_sheet(
    path: &str,
    asset_server: &Res<AssetServer>,
    texture_atlases: &mut ResMut<Assets<TextureAtlas>>,
    position: world::Position,
) -> SpriteSheetBundle {
    let texture_handle = asset_server.load(path);
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, Vec2::new(4.0, 4.0), 4, 4);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    let translation = Vec3::new(
        (position.x as f32).mul_add(TILE_WIDTH, TILE_WIDTH / 2.0),
        (position.y as f32).mul_add(TILE_WIDTH, TILE_WIDTH / 2.0),
        -1.0, // Layer
    );
    let mut transform = Transform::from_scale(Vec3::splat(TILE_WIDTH / 3.0));
    transform.translation = translation;
    SpriteSheetBundle {
        texture_atlas: texture_atlas_handle,
        transform,
        ..Default::default()
    }
}
