use bevy::prelude::*;

mod credit_image;

fn main() {
    App::new()
        .add_startup_system(|mut c: Commands| {
            c.spawn_bundle(Camera2dBundle::default());
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(credit_image::CreditImagePlugin)
        .run();
}
