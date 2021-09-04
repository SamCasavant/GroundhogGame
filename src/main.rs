use bevy::prelude::*;

mod fps_print;
mod world;

fn main() {
    App::build()
        .add_default_plugins()
        .add_plugin(world::WorldPlugin)
        .add_plugin(fps_print::FPSPlugin)
        .run();
}
