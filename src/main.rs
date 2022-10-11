use bevy::{prelude::*,};

mod credit_image;
mod world;

fn main() {
    /*
    //Credits
    App::new()
        .add_startup_system(|mut c: Commands| {
            c.spawn_bundle(Camera2dBundle::default());
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(credit_image::CreditImagePlugin)
        .run();
    */
    App::new()
        .add_startup_system(|mut c: Commands| {
            c.spawn_bundle(Camera2dBundle::default());
        })
        .insert_resource(WindowDescriptor {
            title: "Game".to_string(),
            width: 1280.,
            height: 720.,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(world::WorldPlugin)
        .run();
}
