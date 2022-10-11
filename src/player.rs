use bevy::prelude::*;
use std::time::Duration;

const PLAYER_ASSET: &str = "Ferris.png";
const PLAYER_SIZE: f32 = 32.;
const PLAYER_SPEED: f32 = 500.;
const PLAYER_JUMP_DURATION: f32 = 0.25; //seconds
const GRAVITY: f32 = -650.0;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct JumpDuration{
    timer: Timer
}

#[derive(Eq, PartialEq)]
enum PlayerJumpState {
    Jumping,
    NonJumping,
}

impl Default for PlayerJumpState {
    fn default() -> Self {
        PlayerJumpState::NonJumping
    }
}

#[derive(Component)]
struct JumpState{
    state: PlayerJumpState
}


pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_startup_system(setup)
            .add_system(handle_movement);
    }
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
	commands
		.spawn_bundle(SpriteBundle {
            texture: assets.load(PLAYER_ASSET),
			sprite: Sprite {
				custom_size: Some(Vec2::splat(PLAYER_SIZE)),
				..default()
			},
			..default()
		})
		.insert(Player)
        .insert(JumpDuration{
            timer: Timer::new(Duration::from_secs_f32(PLAYER_JUMP_DURATION), false)
        })
        .insert(JumpState{
            state: PlayerJumpState::default()
        });
}

//Handles player movement, gravity, jumpstate
fn handle_movement(
    input: Res<Input<KeyCode>>,
    mut player: Query<&mut Transform, With<Player>>,
    mut jump_timer: Query<&mut JumpDuration, With<Player>>,
    mut jump_state: Query<&mut JumpState, With<Player>>,
    time: Res<Time>,
) {

	let mut player_transform = player.single_mut();
    let mut player_jump_timer = jump_timer.single_mut();
    let mut player_jump_state = jump_state.single_mut();

	let mut x_vel = 0.;
	let mut y_vel = 0.;

    //Player moves left
	if input.pressed(KeyCode::A) {
		x_vel -= PLAYER_SPEED * time.delta_seconds();
	}

    //Player moves right
	if input.pressed(KeyCode::D) {
		x_vel += PLAYER_SPEED * time.delta_seconds();
	}

    //When space pressed, set player to jumping and start timer
	if input.just_pressed(KeyCode::Space) && player_jump_state.state == PlayerJumpState::NonJumping {
        player_jump_timer.timer.reset();
        player_jump_state.state = PlayerJumpState::Jumping;
	}

    //Player jumps (increases in height) for PLAYER_JUMP_DURATION seconds
    if !player_jump_timer.timer.finished() && player_jump_state.state == PlayerJumpState::Jumping {
        y_vel += (PLAYER_SPEED - GRAVITY) * time.delta_seconds();
        player_jump_timer.timer.tick(time.delta());
    }

    //sets jump state as player touching ground
    //TODO: THIS SHOULD BE DONE WITH COLLISION, TECHNICALLY DOUBLE JUMPING IS POSSIBLE CURRENTLY
    if player_jump_timer.timer.just_finished(){
        player_jump_state.state = PlayerJumpState::NonJumping;
    }

    //Handles Gravity, Currently stops at arbitrary height
    if player_transform.translation.y > -200.0 {
        y_vel += GRAVITY * time.delta_seconds();
    }

	player_transform.translation.x += x_vel;
	player_transform.translation.y += y_vel;

}
