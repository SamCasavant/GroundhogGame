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
            .add_system(camera_movement::pan_orbit_camera.system());
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
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 1000.0 })),
        material: materials.add(Color::rgb(0.1, 0.7, 0.2).into()),
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ..Default::default()
    });
    // Ambient light
    ambient_light.color = Color::WHITE;
    ambient_light.brightness = 0.1;
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

fn load_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let building_assets = [
        "assets/models/buildings/barnhouse.vox",
        "assets/models/buildings/windybean.vox",
    ];
    let object_assets = ["assets/models/objects/pot.vox"];
    let character_assets = ["assets/models/characters/temp.vox"];
    let mut position = world::Position { x: 0, y: 0, z: 1 }; // TODO: Temporary; convert to world::Position when that is updated
    for asset in building_assets {
        // Load .vox file
        let building = dot_vox::load(asset).unwrap();
        let vox_palette = &building.palette;
        for voxel in &building.models[0].voxels {
            let color =
            palette::rgb::Rgb::<palette::encoding::srgb::Srgb, u8>::from_u32::<
                palette::rgb::channels::Abgr,
            >(vox_palette[voxel.i as usize]);

            commands.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
                material: materials.add(
                    Color::rgb(
                        (color.red as f32 / 255.0),
                        (color.green as f32 / 255.0),
                        (color.blue as f32 / 255.0),
                    )
                    .into(),
                ),
                transform: Transform::from_xyz(
                    (voxel.x as f32) + position.x as f32,
                    (voxel.z as f32),
                    (voxel.y as f32),
                ),
                ..Default::default()
            });
        }
        position.x += building.models[0].size.x;
    }
    for asset in object_assets {
        // Load .vox file
        let object = dot_vox::load(asset).unwrap();
        let vox_palette = &object.palette;
        for voxel in &object.models[0].voxels {
            let color =
            palette::rgb::Rgb::<palette::encoding::srgb::Srgb, u8>::from_u32::<
                palette::rgb::channels::Abgr,
            >(vox_palette[voxel.i as usize]);

            commands.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 0.1 })),
                material: materials.add(
                    Color::rgb(
                        (color.red as f32 / 255.0),
                        (color.green as f32 / 255.0),
                        (color.blue as f32 / 255.0),
                    )
                    .into(),
                ),
                transform: Transform::from_xyz(
                    (voxel.x as f32) / 10.0 + position.x as f32,
                    (voxel.z as f32) / 10.0,
                    (voxel.y as f32) / 10.0,
                ),
                ..Default::default()
            });
        }
        position.x += object.models[0].size.x.saturating_div(10);
    }
    for asset in character_assets {
        // Load .vox file
        let character = dot_vox::load(asset).unwrap();
        let vox_palette = &character.palette;
        for voxel in &character.models[0].voxels {
            let color =
            palette::rgb::Rgb::<palette::encoding::srgb::Srgb, u8>::from_u32::<
                palette::rgb::channels::Abgr,
            >(vox_palette[voxel.i as usize]);

            commands.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 0.1 })),
                material: materials.add(
                    Color::rgb(
                        (color.red as f32 / 255.0),
                        (color.green as f32 / 255.0),
                        (color.blue as f32 / 255.0),
                    )
                    .into(),
                ),
                transform: Transform::from_xyz(
                    (voxel.x as f32) / 10.0 + position.x as f32,
                    (voxel.z as f32) / 10.0,
                    (voxel.y as f32) / 10.0,
                ),
                ..Default::default()
            });
        }
        position.x += character.models[0].size.x.saturating_div(10);
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
