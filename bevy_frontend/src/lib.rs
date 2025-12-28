use bevy::prelude::*;

pub fn run() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.08, 0.75, 0.72)))
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}
