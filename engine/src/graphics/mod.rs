fn animate_sprite_system(mut query: Query<(&mut TextureAtlasSprite, &mut Transform, &pathing::Orientation, &pathing::Position)>) {
    for (mut sprite, mut transform, orientation, position) in &mut query.iter_mut() {
        // Set sprite to match orientation
        match orientation.0 {
            pathing::Direction::Up => sprite.index = 5,
            pathing::Direction::Down => sprite.index = 1,
            pathing::Direction::Left => sprite.index = 10,
            pathing::Direction::Right => sprite.index = 13,
            pathing::Direction::UpLeft => todo!(),
            pathing::Direction::UpRight => todo!(),
            pathing::Direction::DownLeft => todo!(),
            pathing::Direction::DownRight => todo!(),
        }
        //Move sprite to match position
        let translation = Vec3::new(
            position.x as f32 * TILE_WIDTH,
            position.y as f32 * TILE_WIDTH,
            0.0,
        );
        transform.translation = translation;
    }
}

pub fn init_sprite_sheet(
    path: &str,
    asset_server: &Res<AssetServer>,
    texture_atlases: &mut ResMut<Assets<TextureAtlas>>,
    position: pathing::Position,
) -> SpriteSheetBundle {
    let texture_handle = asset_server.load(path);
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(4.0, 4.0), 4, 4);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    let translation = Vec3::new(
        position.x as f32 * TILE_WIDTH,
        position.y as f32 * TILE_WIDTH,
        0.0,
    );
    let mut transform = Transform::from_scale(Vec3::splat(6.0));
    transform.translation = translation;
    SpriteSheetBundle {
        texture_atlas: texture_atlas_handle,
        transform: transform,
        ..Default::default()
    }
}