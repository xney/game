use bevy::prelude::*;

mod credit_image;
mod states;
mod world;

fn main() {
    App::new()
        .add_startup_system(|mut c: Commands| {
            c.spawn_bundle(Camera2dBundle::default());
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(states::StatePlugin)
        .add_plugin(credit_image::CreditImagePlugin)
        .add_plugin(world::WorldPlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    //Generate one chunk
    let mut chunk = world::Chunk::new();
    world::spawn_chunk(&mut chunk, &mut commands, assets);

    //(Example): Destroy a single block at 3,3
    world::destroy_block(&mut chunk, &mut commands, 3, 3);
}
