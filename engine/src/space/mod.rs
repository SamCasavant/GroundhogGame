mod camera_movement;
pub mod pathing;
pub mod movement;
pub mod tilemap;

pub struct SpacePlugin;

impl Plugin for SpacePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(pathing::TileMap {
            map: HashMap::<pathing::Position, pathing::Tile>::new(),
        })
        .insert_resource(WindowDescriptor {
            width: 1270.0,
            height: 720.0,
            title: String::from("game"),
            ..Default::default()
        })
        .add_plugin(TilemapPlugin)
        .add_plugin(TiledMapPlugin)
        .add_system(animate_sprite_system.system())
        .add_startup_system(setup.system())
        .add_system(camera_movement::camera_movement.system());
    }
}