use bevy::prelude::*;
use std::time::Duration;

use crate::CharacterCamera;

const PLAYER_ASSET: &str = "Ferris.png";
const PLAYER_SIZE: f32 = 32.;
const PLAYER_START_COORDS: [f32; 3] = [0., 0., 2.];
const PLAYER_SPEED: f32 = 500.;
const PLAYER_JUMP_DURATION: f32 = 0.1; //seconds
const GRAVITY: f32 = -650.0;
const CAMERA_BOUNDS_SIZE: [f32; 2] = [1000., 500.];

#[derive(Component)]
struct Player;

#[derive(Component)]
struct JumpDuration {
    timer: Timer,
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
struct JumpState {
    state: PlayerJumpState,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup)
            .add_system(handle_movement)
            .add_system(handle_camera_movement);
    }
}

#[derive(Component)]
struct CameraBoundsBox {
    center_coord: Vec3,
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    //Player Entity
    commands
        .spawn_bundle(SpriteBundle {
            transform: Transform {
                // render in front of blocks
                translation: Vec3::from_array(PLAYER_START_COORDS),
                ..default()
            },
            texture: assets.load(PLAYER_ASSET),
            sprite: Sprite {
                custom_size: Some(Vec2::splat(PLAYER_SIZE)),
                ..default()
            },
            ..default()
        })
        .insert(Player)
        .insert(JumpDuration {
            timer: Timer::new(Duration::from_secs_f32(PLAYER_JUMP_DURATION), false),
        })
        .insert(JumpState {
            state: PlayerJumpState::default(),
        })
        .insert(CameraBoundsBox {
            center_coord: Vec3::from_array(PLAYER_START_COORDS),
        });
}

//Handles player movement, gravity, jumpstate
fn handle_movement(
    input: Res<Input<KeyCode>>,
    mut query: Query<(
        &mut Transform,
        &mut JumpDuration,
        &mut JumpState,
        With<Player>,
    )>,
    time: Res<Time>,
) {
    for (mut player_transform, mut player_jump_timer, mut player_jump_state, _player) in
        query.iter_mut()
    {
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
        if input.just_pressed(KeyCode::Space)
            && player_jump_state.state == PlayerJumpState::NonJumping
        {
            player_jump_timer.timer.reset();
            player_jump_state.state = PlayerJumpState::Jumping;
        }

        //Player jumps (increases in height) for PLAYER_JUMP_DURATION seconds
        if !player_jump_timer.timer.finished()
            && player_jump_state.state == PlayerJumpState::Jumping
        {
            y_vel += (PLAYER_SPEED - GRAVITY) * time.delta_seconds();
            player_jump_timer.timer.tick(time.delta());
        }

        //sets jump state as player touching ground
        //TODO: THIS SHOULD BE DONE WITH COLLISION, TECHNICALLY DOUBLE JUMPING IS POSSIBLE CURRENTLY
        if player_jump_timer.timer.just_finished() {
            player_jump_state.state = PlayerJumpState::NonJumping;
        }

        //Handles Gravity, Currently stops at arbitrary height
        if player_transform.translation.y > -200.0 {
            y_vel += GRAVITY * time.delta_seconds();
        }

        player_transform.translation.x += x_vel;
        player_transform.translation.y += y_vel;
    }
}

fn handle_camera_movement(
    mut query: Query<(&Transform, &mut CameraBoundsBox, With<Player>)>,
    mut camera_query: Query<(&mut Transform, With<CharacterCamera>, Without<Player>)>,
) {
    for (player_transform, mut camera_box, _player) in query.iter_mut() {
        //Likely has to be changed when multiplayer is added
        let mut camera = camera_query.single_mut();

        //Calculate distance from center based on box size
        let horizontal_dist = CAMERA_BOUNDS_SIZE[0] / 2.;
        let vert_dist = CAMERA_BOUNDS_SIZE[1] / 2.;

        //Calculates coordinates of bounds based on distance from center of camera box
        let cam_x = camera_box.center_coord[0];
        let cam_y = camera_box.center_coord[1];

        let right_bound = cam_x + horizontal_dist;
        let left_bound = cam_x - horizontal_dist;
        let top_bound = cam_y + vert_dist;
        let bottom_bound = cam_y - vert_dist;

        //Checks if player is hitting boundaries of camera box
        if player_transform.translation.x >= right_bound {
            //moves center of camera box by how far player is past boundary
            camera_box.center_coord[0] += player_transform.translation.x - right_bound;
            //moves camera accordingly
            camera.0.translation.x += player_transform.translation.x - right_bound;
        }

        if player_transform.translation.x <= left_bound {
            camera_box.center_coord[0] += player_transform.translation.x - left_bound;
            camera.0.translation.x += player_transform.translation.x - left_bound;
        }

        if player_transform.translation.y >= top_bound {
            camera_box.center_coord[1] += player_transform.translation.y - top_bound;
            camera.0.translation.y += player_transform.translation.y - top_bound;
        }

        if player_transform.translation.y <= bottom_bound {
            camera_box.center_coord[1] += player_transform.translation.y - bottom_bound;
            camera.0.translation.y += player_transform.translation.y - bottom_bound;
        }
    }
}
