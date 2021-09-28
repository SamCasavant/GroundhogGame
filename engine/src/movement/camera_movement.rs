/* This system handles user input control of the camera.*/

use bevy::input::mouse::MouseWheel;
use bevy::{prelude::*, render::camera::Camera};

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
            let scale = scale - event.y / 5.0;
            transform.scale = Vec3::splat(scale);
        }
        if transform.scale.x < 1.0 {
            transform.scale = Vec3::splat(1.0)
        }
        direction = direction * transform.scale.x;
        //Add world bounds (Temporary until camera follows player)
        let mut planned_translation =
            transform.translation + time.delta_seconds() * direction * 200.0;
        let out_of_bounds_x = transform.scale.x * 250.0 - planned_translation.x;
        let out_of_bounds_y = transform.scale.y * 250.0 - planned_translation.y;

        if out_of_bounds_x > 0.0 {
            planned_translation += Vec3::new(out_of_bounds_x, 0.0, 0.0);
        }

        if out_of_bounds_y > 0.0 {
            planned_translation += Vec3::new(0.0, out_of_bounds_y, 0.0)
        }

        transform.translation = planned_translation;
    }
}
