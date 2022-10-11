use bevy::prelude::*;

mod credit_image;
mod states;
mod player;

const TITLE: &str = "The Krusty Krabs";
const WIN_W: f32 = 1280.;
const WIN_H: f32 = 720.;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: String::from(TITLE),
            width: WIN_W,
            height: WIN_H,
            ..default()
        })
        .add_startup_system(|mut c: Commands| {
            c.spawn_bundle(Camera2dBundle::default());
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(states::StatePlugin)
        .add_plugin(credit_image::CreditImagePlugin)
        .add_plugin(player::PlayerPlugin)
        .run();
}
