// This module builds and draws sprites and spawns a camera. Its scope will
// likely increase
// Structure and terrain scale: 1 cube = 10in^3
// Object scale: 1 cube = (1/2)in^3

use bevy::prelude::*;
use bevy::render::draw::OutsideFrustum;
use log::trace;

use crate::engine;
use crate::engine::actor;
use crate::engine::world;
mod camera_movement;
pub mod voxel;

pub struct GraphicsPlugin;

impl Plugin for GraphicsPlugin {
    fn build(
        &self,
        app: &mut AppBuilder,
    ) {
        debug!("Initializing GraphicsPlugin");
        app.insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.05)))
            .insert_resource(Msaa { samples: 1 })
            .add_startup_system(setup.system())
            .add_system(animate_sprite_system.system().label("render"))
            .add_system(camera_movement::pan_orbit_camera.system())
            .add_system_set(
                SystemSet::on_enter(engine::AppState::BuildingWorld)
                    .with_system(engine::render::voxel::build.system()),
            );
    }
}

pub const TILE_WIDTH: f32 = 64.0;

fn setup(
    mut commands: Commands,
    mut ambient_light: ResMut<bevy::pbr::AmbientLight>,
) {
    // set up the camera
    let translation = Vec3::new(0.0, 100.0, 100.0);
    let radius = translation.length();
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_translation(translation)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        })
        .insert(camera_movement::PanOrbitCamera {
            radius,
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
        transform: Transform::from_xyz(100.0, 100.0, 100.0),
        ..Default::default()
    });
    commands.spawn_bundle(LightBundle {
        light: Light {
            color: Color::rgb(0.95, 0.8, 0.05),
            fov: 360.0,
            intensity: 99999.0,
            range: 500.0,
            ..Default::default()
        },
        transform: Transform::from_xyz(-100.0, -100.0, -100.0),
        ..Default::default()
    });
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
