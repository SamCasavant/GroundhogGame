// This module builds and draws sprites and spawns a camera. Its scope will
// likely increase

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

    // Load .vox file
    let barnhouse = dot_vox::load("assets/models/barnhouse.vox").unwrap();
    let vox_palette = &barnhouse.palette;
    for voxel in &barnhouse.models[0].voxels {
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
                -(voxel.x as f32),
                -(voxel.y as f32),
                (voxel.z as f32),
            ),
            ..Default::default()
        });
    }
    // Spawn Light
    commands.spawn_bundle(LightBundle {
        light: Light {
            color: Color::rgb(1.0, 1.0, 1.0),
            fov: 360.0,
            intensity: 9999.0,
            range: 100.0,
            ..Default::default()
        },
        transform: Transform::from_xyz(10.0, 10.0, 70.0),
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
