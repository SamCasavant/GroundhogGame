use std::ops::RangeInclusive;

use bevy::prelude::*;
use rand::Rng;

use crate::engine::render::TILE_WIDTH;
use crate::engine::world;

pub struct NutritionValue(pub u32);

pub struct HamburgerTimer(pub Timer);

pub fn spawn_hamburger_every_second(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<HamburgerTimer>,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    // TODO Temporary
    if timer.0.tick(time.delta()).just_finished() {
        let x_range = RangeInclusive::new(0, 50);
        let y_range = x_range.clone();
        let mut rng = rand::thread_rng();
        let position = world::Position {
            x: rng.gen_range(x_range),
            y: rng.gen_range(y_range),
        };
        let translation = Vec3::new(
            (position.x as f32).mul_add(TILE_WIDTH, TILE_WIDTH / 2.0),
            (position.y as f32).mul_add(TILE_WIDTH, TILE_WIDTH / 2.0),
            -1.0, // Layer
        );
        let mut transform =
            Transform::from_scale(Vec3::splat(TILE_WIDTH / 256.0));

        transform.translation = translation;

        let texture_handle = asset_server.load("sprites/hamburger.png");
        let texture_atlas = TextureAtlas::from_grid(
            texture_handle,
            Vec2::new(256.0, 256.0),
            1,
            1,
        );
        let texture_atlas_handle = texture_atlases.add(texture_atlas);

        commands
            .spawn()
            .insert_bundle(SpriteSheetBundle {
                texture_atlas: texture_atlas_handle.clone(),
                transform: transform,
                sprite: TextureAtlasSprite::new(0),
                ..Default::default()
            })
            .insert(NutritionValue(10000))
            .insert(position);
    }
}
