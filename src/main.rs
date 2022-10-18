use bevy::prelude::*;

mod credit_image;
mod menu;
mod states;

fn main() {
    App::new()
        .add_startup_system(setup)
        .add_plugins(DefaultPlugins)
        .add_plugin(states::StatePlugin)
        .add_plugin(credit_image::CreditImagePlugin)
        .add_plugin(menu::MenuPlugin)
        .run();
}

fn setup(mut c: Commands) {
    c.spawn_bundle(Camera2dBundle::default());
}
