use bevy::prelude::*;

pub struct NutritionValue(pub u32);

struct HamburgerTimer(Timer);

fn spawn_hamburger_every_second(
    mut commands: Commands,
    timer: ResMut<HamburgerTimer>,
) { // TODO Temporary
}
