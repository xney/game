use bevy::prelude::*;
use bincode::{Decode, Encode};
use iyes_loopless::prelude::*;
use std::{
    fs::{create_dir_all, read, File},
    io::Write,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use crate::{
    args::ServerArgs,
    network::{ClientAddress, BINCODE_CONFIG},
    player::{Inventory, PlayerInput, PlayerPosition},
    states,
    world::Terrain,
};

pub const DEFAULT_SAVE_DIR: &str = "savedata";
pub const DEFAULT_SAVE_FILE_SERVER: &str = "server.sav";

pub fn default_save_path_server() -> PathBuf {
    Path::new(".")
        .join(DEFAULT_SAVE_DIR)
        .join(DEFAULT_SAVE_FILE_SERVER)
}

pub mod server {
    use super::*;

    pub struct SaveLoadPlugin;

    impl Plugin for SaveLoadPlugin {
        fn build(&self, app: &mut App) {
            // save
            app.add_fixed_timestep(std::time::Duration::from_secs(5), "SAVE_INTERVAL");
            app.add_fixed_timestep_system(
                "SAVE_INTERVAL",
                0,
                save_server
                    .run_in_state(states::server::GameState::Running)
                    .label("save_server"),
            );

            // load on start
            app.add_enter_system(
                states::server::GameState::Running,
                load_server.label("load_server").after("create_world"),
            );
        }
    }
}

/// Helper struct to save and load players
#[derive(Debug, Encode, Decode)]
struct PlayerInFile {
    addr: SocketAddr,
    position: PlayerPosition,
    inventory: Inventory,
}

/// Struct that get serialized to save the world
#[derive(Debug, Encode)]
pub struct SaveFile<'a> {
    players: Vec<PlayerInFile>,
    /// reference to the terrain resource
    terrain: &'a Terrain,
}

/// Struct that gets created whenever we deserialize the save file
#[derive(Debug, Decode)]
pub struct LoadFile {
    players: Vec<PlayerInFile>,
    /// owns a terrain that gets created from the file
    terrain: Terrain,
}

fn save_server(
    terrain: Res<Terrain>,
    query: Query<(&PlayerPosition, &ClientAddress, &Inventory)>,
    args: Res<ServerArgs>,
) {
    let mut players_in_file = Vec::<PlayerInFile>::new();
    for (position, addr, inv) in query.iter() {
        let player = PlayerInFile {
            addr: addr.addr,
            position: position.clone(),
            inventory: inv.clone(),
        };
        players_in_file.push(player);
    }

    let save_file = SaveFile {
        players: players_in_file,
        terrain: terrain.as_ref(),
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
            match File::create(&args.save_file) {
                Ok(mut file) => {
                    // write the bytes to file
                    match file.write_all(&encoded_vec) {
                        Ok(_) => {
                            // info!("saved to file!"),
                        }
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

/// Load the file
fn load_server(
    mut commands: Commands,
    players: Query<Entity, With<ClientAddress>>,
    args: Res<ServerArgs>,
) {
    match read(&args.save_file) {
        Ok(encoded_vec) => {
            // try to load the world and player
            let decoded: LoadFile = match bincode::decode_from_slice(&encoded_vec, BINCODE_CONFIG) {
                Ok((load, _size)) => load,
                Err(e) => {
                    error!("unable to decode save file: {}", e);
                    return;
                }
            };

            // delete old terrain
            commands.remove_resource::<Terrain>();

            // insert new terrain
            commands.insert_resource(decoded.terrain);

            // delete all player entities
            for entity in players.iter() {
                commands.entity(entity).despawn();
            }

            // spawn entities for each player that we loaded from file
            for player in decoded.players {
                spawn_player(&mut commands, &player)
            }

            warn!("loaded from file!");
        }
        Err(e) => {
            error!("could not read save file, {}", e);
        }
    }
}

/// Spawn in a previously-connected player (from a file)
fn spawn_player(commands: &mut Commands, player: &PlayerInFile) {
    commands
        .spawn()
        .insert(ClientAddress { addr: player.addr })
        .insert(player.position.clone())
        .insert(PlayerInput::default())
        .insert(player.inventory.clone());
}
