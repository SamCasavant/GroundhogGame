use bevy::{prelude::*, render::camera::Camera};
use bevy::input::mouse::{MouseWheel};

pub fn camera_movement(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut scroll_events: EventReader<MouseWheel>,
    mut query: Query<&mut Transform, With<Camera>>,
) {
    for mut transform in &mut query.iter_mut() {

        let mut direction = Vec3::ZERO;
        let scale = transform.scale.x;

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
        for event in scroll_events.iter() {
            let scale = scale - event.y/5.0;
            transform.scale = Vec3::splat(scale);
        }
        if transform.scale.x < 1.0 {
            transform.scale = Vec3::splat(1.0)
        }

        transform.translation += time.delta_seconds() * direction * 500.0;
    }
}
