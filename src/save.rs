use bevy::prelude::*;
use bincode::{Decode, Encode};
use std::{
    fs::{create_dir_all, read, File},
    io::Write,
    path::{Path, PathBuf},
};

use crate::{
    network::BINCODE_CONFIG,
    player::Player,
    states,
    world::{RenderedBlock, Terrain},
    CharacterCamera,
};

pub const DEFAULT_SAVE_DIR: &'static str = "savedata";
pub const DEFAULT_SAVE_FILE: &'static str = "savegame.sav";

pub fn default_save_path() -> PathBuf {
    Path::new(".").join(DEFAULT_SAVE_DIR).join(DEFAULT_SAVE_FILE)
}

pub struct SaveLoadPlugin;

impl Plugin for SaveLoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_update(states::GameState::InGame)
                .with_system(f5_save_to_file)
                .with_system(f6_load_from_file),
        );
    }
}

/// Struct that get serialized to save the world
#[derive(Debug, Encode)]
pub struct SaveFile<'a> {
    player_coords: (u64, u64),
    /// reference to the terrain resource
    terrain: &'a Terrain,
}

/// Struct that gets created whenever we deserialize the save file
#[derive(Debug, Decode)]
pub struct LoadFile {
    player_coords: (u64, u64),
    /// owns a terrain that gets created from the file
    terrain: Terrain,
}

/// Saves the player and terrain in a file
pub fn f5_save_to_file(
    input: Res<Input<KeyCode>>,
    query: Query<&Transform, With<Player>>,
    terrain: Res<Terrain>,
) {
    // return early if f5 was not pressed
    if !input.just_pressed(KeyCode::F5) {
        return;
    }
    // make sure there's a player to encode
    let transform = match query.get_single() {
        Ok(t) => t,
        Err(_) => return,
    };

    let x_block_index = (transform.translation.x / 32.) as u64;
    let y_block_index = -(transform.translation.y / 32.) as u64;

    // the struct to serialize
    let save_file = SaveFile {
        player_coords: (x_block_index, y_block_index),
        terrain: &mut terrain.as_ref(),
    };

    // try to encode, allocating a vec
    // in a real packet, we should use a pre-allocated array and encode into its slice
    match bincode::encode_to_vec(save_file, BINCODE_CONFIG) {
        Ok(encoded_vec) => {
            // creates the savedata folder if it is missing
            if let Err(e) = create_dir_all(DEFAULT_SAVE_DIR) {
                error!("unable to create save dir, {}", e);
                return;
            }
            // else it was successful

            // open file in write-mode
            match File::create(default_save_path()) {
                Ok(mut file) => {
                    // write the bytes to file
                    match file.write_all(&encoded_vec) {
                        Ok(_) => warn!("saved to file!"),
                        Err(e) => error!("could not write to save file, {}", e),
                    }
                }
                Err(e) => {
                    error!("could not create save file, {}", e);
                }
            }
        }
        Err(e) => {
            error!("unable to encode terrain, {}", e);
        }
    }
}

/// Loads (despawns and respawns anew) the player and terrain from a file
pub fn f6_load_from_file(
    input: Res<Input<KeyCode>>,
    mut commands: Commands,
    assets: Res<AssetServer>,
    query: Query<Entity, Or<(With<RenderedBlock>, With<Player>)>>,
    mut query_camera: Query<(&mut Transform, With<CharacterCamera>, Without<Player>)>,
) {
    // return early if F6 was not just pressed
    if !input.just_pressed(KeyCode::F6) {
        return;
    }
    match read("./savedata/quicksave.sav") {
        Ok(encoded_vec) => {
            // clear rendered blocks and delete player
            for entity in query.iter() {
                commands.entity(entity).despawn();
            }
            commands.remove_resource::<Terrain>();
            // load the world and player
            let mut decoded: LoadFile = bincode::decode_from_slice(&encoded_vec, BINCODE_CONFIG)
                .unwrap()
                .0;
            crate::world::spawn_sprites_from_terrain(&mut commands, &assets, &mut decoded.terrain);
            crate::player::spawn_player_pos(
                decoded.player_coords,
                &mut commands,
                &assets,
                &mut query_camera.get_single_mut().unwrap().0,
            );
            commands.insert_resource(decoded.terrain);

            warn!("loaded from file!");
        }
        Err(e) => {
            error!("could not read save file, {}", e);
        }
    }
}
