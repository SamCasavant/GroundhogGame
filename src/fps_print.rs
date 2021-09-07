use bevy::diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;

pub struct FPSPlugin;

impl Plugin for FPSPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(FrameTimeDiagnosticsPlugin::default())
            .insert_resource(FPSTimer(Timer::from_seconds(1.0, true)))
            .add_system(fps_system.system());
    }
}

struct FPSTimer(Timer);

fn fps_system(diagnostics: Res<Diagnostics>, time: Res<Time>, mut timer: ResMut<FPSTimer>) {
    timer.0.tick(time);
    if timer.0.finished() {
        let mut fps_value = 0.;

        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(average) = fps.average() {
                fps_value = average;
            }
        };

        println!("{}", fps_value)
    }
}
