use bevy::{
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
    time::Stopwatch,
};
use crate::world::BlockType;
use iyes_loopless::prelude::*;
use std::{cmp, time::Duration};

use bincode::{Decode, Encode};

use crate::network::ClientAddress;
use crate::{
    states::client::GameState,
    world::{
        block_exists, derender_chunk, render_chunk, spawn_chunk, to_world_point_x,
        to_world_point_y, Terrain, CHUNK_HEIGHT, CHUNK_WIDTH,
    },
    CharacterCamera, WIN_H, WIN_W,
};

const PLAYER_ASSET: &str = "Ferris.png";
pub const PLAYER_AND_BLOCK_SIZE: f32 = 32.;
const PLAYER_START_POS: PlayerPosition = PlayerPosition { x: 0., y: 0. };
const PLAYER_SPEED: f32 = 20.;
const PLAYER_JUMP_DURATION: f32 = 0.3; //seconds
const PLAYER_MINE_DURATION: f32 = 2.; //seconds
const PLAYER_MINE_RADIUS: f32 = 3.; //number of blocks
const GRAVITY: f32 = -10.0;
pub const CAMERA_BOUNDS_SIZE: [f32; 2] = [1000., 500.];
const PLAYER_Z: f32 = 2.0;

#[derive(Component, Default, Debug, Encode, Decode, Clone)]
pub struct PlayerPosition {
    pub x: f32,
    pub y: f32,
}

/// Contains all inputs that the client needs to tell the server
#[derive(Component, Encode, Decode, Clone, Debug, Default)]
pub struct PlayerInput {
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub mine: bool, //true means the block at block_x, block_y was clicked on.
    pub block_x: usize,
    pub block_y: usize,
}

#[derive(Component)]
pub struct Inventory {
    blocktype: BlockType,
}

pub mod server {
    use crate::network::server::ConnectedClientInfo;

    use super::*;

    #[derive(Component)]
    pub struct JumpDuration {
        timer: Timer,
    }

    impl Default for JumpDuration {
        fn default() -> Self {
            Self {
                timer: Timer::new(Duration::from_secs_f32(PLAYER_JUMP_DURATION), false),
            }
        }
    }

    #[derive(Component)]
    struct MineDuration {
        timer: Stopwatch,
    }

    #[derive(Eq, PartialEq)]
    enum PlayerJumpState {
        Jumping,
        Falling,
        NonJumping,
    }

    impl Default for PlayerJumpState {
        fn default() -> Self {
            PlayerJumpState::NonJumping
        }
    }

    #[derive(Component, Default)]
    pub struct JumpState {
        state: PlayerJumpState,
    }

    #[derive(Component, Debug)]
    struct PlayerCollision {
        any: bool,
        top: Option<f32>,
        right: Option<f32>,
        bottom: Option<f32>,
        left: Option<f32>,
        inside: bool,
    }

    impl Default for PlayerCollision {
        fn default() -> PlayerCollision {
            PlayerCollision {
                any: false,
                top: None,
                right: None,
                bottom: None,
                left: None,
                inside: false,
            }
        }
    }

    //Handles player movement, gravity, jumpstate
    pub fn handle_movement(
        mut query: Query<
            (
                &mut PlayerPosition,
                &mut JumpDuration,
                &mut JumpState,
                &PlayerInput,
            ),
            With<ConnectedClientInfo>,
        >,
        _time: Res<Time>,
        terrain: Res<Terrain>,
    ) {
        const DEBUG_COLLISIONS: bool = false;

        // timers don't work with iyes_loopless?
        // TODO: maybe make this system run _not_ on a fixed timestep and user a timer
        let time_delta = 1f32 / 60f32;

        for (mut player_position, mut player_jump_timer, mut player_jump_state, input) in
            query.iter_mut()
        {
            player_jump_timer
                .timer
                .tick(Duration::from_secs_f32(time_delta));

            let mut x_diff = 0.;
            let mut y_diff = 0.;

            let prev_x = player_position.x;
            let prev_y = player_position.y;

            // info!("movement calc, starting: ({}, {})", prev_x, prev_y);

            //Player moves left
            if input.left {
                x_diff -= PLAYER_SPEED * time_delta;
            }

            //Player moves right
            if input.right {
                x_diff += PLAYER_SPEED * time_delta;
            }

            //When space pressed, set player to jumping and start timer
            if input.jump && player_jump_state.state == PlayerJumpState::NonJumping {
                player_jump_timer.timer.reset();
                player_jump_state.state = PlayerJumpState::Jumping;
                // info!("player starting jump");
            }

            //Player jumps (increases in height) for PLAYER_JUMP_DURATION seconds
            if !player_jump_timer.timer.finished()
                && player_jump_state.state == PlayerJumpState::Jumping
            {
                y_diff += PLAYER_SPEED * time_delta;
                // info!("player is jumping");
            }

            //sets jump state as player falling
            if player_jump_state.state == PlayerJumpState::Jumping
                && player_jump_timer.timer.finished()
            {
                player_jump_state.state = PlayerJumpState::Falling;
                // info!("player is falling");
            }

            // gravity already negative
            y_diff += GRAVITY * time_delta;

            // info!(
            //     "moving player, time_delta:{:.5} x_diff:{:.2}, y_diff:{:.2}",
            //     time_delta, x_diff, y_diff
            // );

            player_position.x += x_diff as f32;
            player_position.y += y_diff as f32;

            // prevent going past horizontal world boundaries
            player_position.x =
                f32::min(f32::max(player_position.x, 0.0), (CHUNK_WIDTH - 1) as f32);

            loop {
                let player_collision = get_collisions(&player_position, &terrain, DEBUG_COLLISIONS);
                if !player_collision.any {
                    break;
                }

                // info!("There's a collision: {:?}", player_collision);
                // Check for "inside" conditions that can occur and just reset in those scenarios
                if (player_collision.left.is_some() && player_collision.right.is_some())
                    || (player_collision.top.is_some() && player_collision.bottom.is_some())
                    || player_collision.inside
                {
                    player_position.x = prev_x;
                    player_position.y = prev_y;
                    // info!("Inside collision");

                    continue;
                }

                if player_collision.left.is_some() {
                    player_position.x = player_collision.left.unwrap();
                    // info!("Left collision");
                    continue;
                } else if player_collision.right.is_some() {
                    player_position.x = player_collision.right.unwrap();
                    // info!("Right collision");]
                    continue;
                }

                if player_collision.top.is_some() {
                    player_position.y = player_collision.top.unwrap();
                    // info!("Top collision");
                    continue;
                } else if player_collision.bottom.is_some() {
                    player_position.y = player_collision.bottom.unwrap();
                    // info!("Bottom collision");
                    player_jump_state.state = PlayerJumpState::NonJumping;
                    // info!("player hit ground");

                    continue;
                }
            }
        }
    }

    fn get_collisions(
        player_position: &Mut<PlayerPosition>,
        terrain: &Terrain,
        debug: bool,
    ) -> PlayerCollision {
        // Get block indices we need to check

        // how many blocks to the right the player is
        let player_x_block = (player_position.x) as usize;
        // how many blocks down the player is
        let player_y_block = -(player_position.y) as usize;

        // info!("player: ({}, {})", player_x_block, player_y_block);

        let sizes = Vec2 { x: 1., y: 1. };

        let mut collisions = PlayerCollision::default();

        for x_index in
            (cmp::max(1, player_x_block) - 1)..=(cmp::min(player_x_block + 1, CHUNK_WIDTH - 1))
        {
            for y_index in (cmp::max(1, player_y_block) - 1)..=player_y_block + 1 {
                let chunk_number = y_index / CHUNK_HEIGHT;
                // index inside the chunk
                let chunk_y_index = y_index - (chunk_number * CHUNK_HEIGHT);

                let block = terrain.chunks[chunk_number].blocks[chunk_y_index][x_index];

                // info!("checking chunk: {}, x: {}, y: {}, block = {:?}", chunk_number, x_index, chunk_y_index, block);
                if block.is_some() {
                    let z = PLAYER_Z; // always collide on same z plane
                    let block_pos = Vec3 {
                        x: x_index as f32,
                        y: -(chunk_y_index as f32 + (chunk_number * CHUNK_HEIGHT) as f32) as f32,
                        z: z,
                    };
                    let collision = collide(
                        Vec3::new(player_position.x as f32, player_position.y as f32, z),
                        sizes,
                        block_pos,
                        sizes,
                    );
                    if collision.is_some() {
                        collisions.any = true;
                        match collision {
                            Some(Collision::Top) => collisions.bottom = Some(block_pos.y + sizes.y),
                            Some(Collision::Left) => collisions.right = Some(block_pos.x - sizes.x),
                            Some(Collision::Bottom) => collisions.top = Some(block_pos.y - sizes.y),
                            Some(Collision::Right) => collisions.left = Some(block_pos.x + sizes.x),
                            Some(Collision::Inside) => collisions.inside = true,
                            None => (),
                        }
                    }
                    if debug {
                        info!("Block x: {}, y: {}, chunk: {}, collision: {:?}, playerxy: {:?}, blockxy: {},{}", x_index, chunk_y_index, chunk_number, collision, player_position, block_pos.x, block_pos.y);
                    }
                }
            }
        }

        return collisions;
    }
}

pub mod client {
    use super::*;

    pub struct PlayerPlugin;

    impl Plugin for PlayerPlugin {
        fn build(&self, app: &mut App) {
            app.add_system(
                move_players_sprites_to_position
                    .run_in_state(GameState::InGame)
                    .label("move_players_sprites_to_position"),
            )
            .add_system(
                handle_camera_movement
                    .run_in_state(GameState::InGame)
                    .after("move_players_sprites_to_position")
                    .label("handle_camera_movement"),
            )
            .add_enter_system(GameState::InGame, init_spawn_local_player)
            .add_exit_system(GameState::InGame, destroy_all_players);
        }
    }

    /// Marker struct for _our_ player
    #[derive(Component)]
    pub struct LocalPlayer;

    /// Marker struct for all players
    #[derive(Component)]
    pub struct Player;

    #[derive(Component)]
    pub struct CameraBoundsBox {
        pub center_coord: Vec3,
    }

    /// Moves the transform of player entities to their stored PlayerPosition
    fn move_players_sprites_to_position(
        mut query: Query<
            (&mut Transform, &PlayerPosition, Option<&LocalPlayer>),
            Without<CharacterCamera>,
        >,
        mut camera: Query<&mut Transform, With<CharacterCamera>>,
    ) {
        for (mut render_pos, game_pos, local) in query.iter_mut() {
            let bevy_x = game_pos.x as f32 * PLAYER_AND_BLOCK_SIZE as f32;
            let bevy_y = game_pos.y as f32 * PLAYER_AND_BLOCK_SIZE as f32;

            if bevy_x != render_pos.translation.x {
                render_pos.translation.x = bevy_x;
            }
            if bevy_y != render_pos.translation.y {
                render_pos.translation.y = bevy_y;
            }
        }
    }

    /// creates local player at starting position,
    /// sprite will be moved to correct location in other system
    fn init_spawn_local_player(mut commands: Commands, assets: Res<AssetServer>) {
        let game_position = PLAYER_START_POS;
        info!(
            "spawning player at game position=({}, {})",
            game_position.x, game_position.y,
        );
        // dummy position,
        let bevy_position = Vec3::new(0., 0., PLAYER_Z);
        //Player Entity
        commands
            .spawn_bundle(SpriteBundle {
                transform: Transform {
                    // render in front of blocks
                    translation: bevy_position.clone(),
                    ..default()
                },
                texture: assets.load(PLAYER_ASSET),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(PLAYER_AND_BLOCK_SIZE)),
                    ..default()
                },
                ..default()
            })
            .insert(LocalPlayer)
            .insert(Player)
            .insert(game_position)
            .insert(CameraBoundsBox {
                center_coord: bevy_position.clone(),
            });
        // TODO: reset camera


     let inventory_text_style = TextStyle {
        font: assets.load("fonts/milky_coffee.ttf"),
        font_size: 15.0,
        color: Color::RED,
    };

    let mut binding = (&mut commands).spawn();
    let inventory = binding
        .insert_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                position_type: PositionType::Absolute,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexStart,
                ..default()
            },
            color: Color::NONE.into(),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn().insert_bundle(ImageBundle {
                style: Style {
                    size: Size::new(Val::Px(600.0), Val::Auto),
                    ..default()
                },
                image: assets.load("Hotbar.png").into(),
                ..default()
            });
        });
    let blocktypes = vec![
        BlockType::Peridot,
        BlockType::Quartz,
        BlockType::Sand,
        BlockType::Trunk,
        BlockType::Basalt,
        BlockType::Clay,
        BlockType::Coal,
        BlockType::Diabase,
        BlockType::Gabbro,
        BlockType::Granite,
        BlockType::Iron,
        BlockType::Labradorite,
        BlockType::Leaves,
        BlockType::Limestone,
    ];
    let mut n = 0;
    for blocks in blocktypes {
        inventory.with_children(|parent| {
            parent
                .spawn()
                .insert_bundle(
                    TextBundle::from_section("", inventory_text_style.clone()).with_style(Style {
                        position_type: PositionType::Absolute,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        position: UiRect {
                            left: Val::Px(356.0 + ((n * 42) as f32)),
                            right: Val::Px(388.0 + ((n * 42) as f32)),
                            top: Val::Px(30.0),
                            bottom: Val::Px(30.0),
                        },
                        ..default()
                    }),
                )
                .insert(Inventory { blocktype: blocks });
        });
        n = n + 1;
    }
    }

    fn destroy_all_players(
        players: Query<Entity, With<Player>>,
        mut camera_query: Query<&mut Transform, (With<CharacterCamera>, Without<Player>)>,
        mut commands: Commands,
    ) {
        // despawn all players
        for entity in players.iter() {
            commands.entity(entity).despawn();
        }

        // move camera to start position
        for mut camera in camera_query.iter_mut() {
            camera.translation.x = PLAYER_START_POS.x as f32;
            camera.translation.y = PLAYER_START_POS.y as f32;
        }
    }

    pub fn spawn_other_player_at(
        commands: &mut Commands,
        assets: &AssetServer,
        addr: &ClientAddress,
        position: &PlayerPosition
    ) {
        // color based on address
        let color = addr.color();

        // game coords -> bevy rendering coords
        let real_x = position.x * 32.;
        let real_y = position.y * 32.;

        commands
            .spawn()
            .insert_bundle(SpriteBundle {
                transform: Transform {
                    // render in front of blocks
                    translation: Vec3::new(real_x as f32, real_y as f32, PLAYER_Z),
                    ..default()
                },
                texture: assets.load(PLAYER_ASSET),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(PLAYER_AND_BLOCK_SIZE)),
                    color: color, // tint
                    ..default()
                },
                ..default()
            })

            .insert(Player)
            .insert(position.clone())
            .insert(addr.clone());
    }

    fn handle_camera_movement(
        mut query: Query<(&Transform, &mut CameraBoundsBox, With<LocalPlayer>)>,
        mut camera_query: Query<(&mut Transform, With<CharacterCamera>, Without<LocalPlayer>)>,
        input: Res<Input<KeyCode>>,
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

            //DEBUGGING: Free Roam Camera with Arrow Keys
            if input.pressed(KeyCode::Right) {
                camera.0.translation.x += 25.;
            }
            if input.pressed(KeyCode::Left) {
                camera.0.translation.x -= 25.;
            }
            if input.pressed(KeyCode::Up) {
                camera.0.translation.y += 25.;
            }
            if input.pressed(KeyCode::Down) {
                camera.0.translation.y -= 25.;
            }

            //Pressing R returns camera to player after free roam
            if input.pressed(KeyCode::R) {
                camera.0.translation.x = camera_box.center_coord[0];
                camera.0.translation.y = camera_box.center_coord[1];
            }
        }
    }

    // fn handle_mining(
    //     mut windows: ResMut<Windows>,
    //     mouse: Res<Input<MouseButton>>,
    //     mut query: Query<(
    //         &mut Transform,
    //         &mut CameraBoundsBox,
    //         &mut MineDuration,
    //         With<LocalPlayer>,
    //     )>,
    //     mut commands: Commands,
    //     mut terrain: ResMut<Terrain>,
    //     time: Res<Time>,
    // ) {
    //     let window = windows.get_primary_mut();

    //     if !window.is_none() {
    //         let win = window.unwrap();

    //         for (transform, camera_box, mut mine_timer, _player) in query.iter_mut() {
    //             let ms = win.cursor_position();

    //             if !ms.is_none() {
    //                 let mouse_pos = ms.unwrap();

    //                 //calculate distance of click from camera center
    //                 let dist_x = mouse_pos.x - (WIN_W / 2.);
    //                 let dist_y = mouse_pos.y - (WIN_H / 2.);

    //                 //calculate bevy choords of click
    //                 let game_x = camera_box.center_coord.x + dist_x;
    //                 let game_y = camera_box.center_coord.y + dist_y;

    //                 //calculate block coords from bevy coords
    //                 let block_x = (game_x / 32.).round() as usize;
    //                 let block_y = (game_y / -32.).round() as usize;

    //                 //calculate player distance from mined blocks
    //                 let player_x_coord = transform.translation.x;
    //                 let player_y_coord = transform.translation.y;

    //                 let player_x = (player_x_coord / 32.).round();
    //                 let player_y = (player_y_coord / -32.).round();

    //                 let mine_dist = ((block_x as f32 - player_x).powi(2)
    //                     + (block_y as f32 - player_y).powi(2) as f32)
    //                     .sqrt();

    //                 if mouse.pressed(MouseButton::Left)
    //                     && mine_dist <= PLAYER_MINE_RADIUS
    //                     && block_exists(block_x, block_y, &mut terrain)
    //                 {
    //                     if mine_timer.timer.elapsed_secs() >= PLAYER_MINE_DURATION {
    //                         // let _res = destroy_block(block_x, block_y, &mut commands, &mut terrain);
    //                         mine_timer.timer.reset();
    //                     }

    //                     mine_timer.timer.tick(time.delta());
    //                 } else if mouse.just_released(MouseButton::Left) {
    //                     mine_timer.timer.reset();
    //                 }

    //                 //DEBUGGING: Right click to instantly mine
    //                 if mouse.pressed(MouseButton::Right) {
    //                     // let _res = destroy_block(block_x, block_y, &mut commands, &mut terrain);
    //                 }
    //             }
    //         }
    //     }
    // }

    

    /// Helper function, centers the camera in the camera bounds
    
    fn reset_camera(camera_bounds: &CameraBoundsBox, mut camera_transform: &mut Transform) {
        camera_transform.translation.x = camera_bounds.center_coord[0];
        camera_transform.translation.y = camera_bounds.center_coord[1];
    }
}
