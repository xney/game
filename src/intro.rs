use bevy::prelude::*;

use crate::states::{self, client::GameState};

pub struct IntroPlugin;

impl Plugin for IntroPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_system_set(
            SystemSet::on_enter(states::client::GameState::Intro).with_system(init_intro),
        )
        .add_system_set(
            SystemSet::on_update(states::client::GameState::Intro).with_system(mouse_exits),
        )
        .add_system_set(
            SystemSet::on_exit(states::client::GameState::Intro).with_system(destroy_intro),
        );
    }
}

#[derive(Component)]
struct IntroImage;


fn init_intro(mut commands: Commands, assets: Res<AssetServer>) {
    info!("init intro");
    commands
        .spawn().insert(IntroImage)
        .insert_bundle(SpriteBundle {
            texture: assets.load("intro_image.png"),
            sprite: Sprite {
                custom_size: Some(Vec2::from_array([1280., 720.])),
                ..default()
            },
            visibility: Visibility { 
                is_visible: true },
        ..default()
        });
        info!("spawned intro image");

}

fn mouse_exits(
    buttons: Res<Input<MouseButton>>,
    mut game_state: ResMut<State<GameState>>,
) {
    if buttons.any_just_pressed([MouseButton::Left, MouseButton::Right]) {
        game_state.set(GameState::Menu).unwrap();
    }
}


fn destroy_intro(mut commands: Commands, query: Query<Entity, With<IntroImage>>) {
    info!("destroying intro");
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<IntroImage>();
    //commands.remove_resource::<Timeout>();
}