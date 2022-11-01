use bevy::{prelude::*, render::render_resource::StencilFaceState};
use std::{io::Write, fs::{create_dir_all, File, read}};
use bincode::{Decode, Encode, BorrowDecode};

use crate::{player::Player, world::{Terrain, RenderedBlock}, network::BINCODE_CONFIG};

#[derive(Debug)]
pub struct SaveFile<'a> {
    player_coords:(u64, u64),
    terrain:&'a mut Terrain
}//i dont understand rust

impl Encode for SaveFile<'_> {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        bincode::Encode::encode(&self.player_coords, encoder)?;
        bincode::Encode::encode(self.terrain, encoder)?;
        Ok(())
    }
}


impl<'a> BorrowDecode<'a> for SaveFile<'a> {
    fn borrow_decode<D: bincode::de::Decoder+bincode::de::BorrowDecoder<'a>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        Ok(Self {
            player_coords: bincode::Decode::decode(decoder)?,
            terrain: &mut Box::new(bincode::BorrowDecode::borrow_decode(decoder)?),
        })
    }
}


fn save_to_file(input: Res<Input<KeyCode>>, query: Query<&Transform, With<Player>>, terrain: Res<Terrain>) {
    
    if !input.just_pressed(KeyCode::F5) {
        return;
    }
    let transform = query.get_single().unwrap();
    let x_block_index = (transform.translation.x / 32.) as u64;
    let y_block_index = -(transform.translation.y / 32.) as u64;
    let save_file = SaveFile{ player_coords: (x_block_index, y_block_index), terrain: &mut terrain.as_ref() };

    // try to encode, allocating a vec
    // in a real packet, we should use a pre-allocated array and encode into its slice
    match bincode::encode_to_vec(save_file, BINCODE_CONFIG) {
        Ok(encoded_vec) => {
            // write the bytes to file
            create_dir_all("./savedata/"); //creates the savedata folder if it is missing

            match File::create("./savedata/quicksave.sav") {
                Ok(mut file) => {
                    file.write_all(&encoded_vec)
                        .expect("could not write to save file");
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

fn f6_loads_terrain(
    input: Res<Input<KeyCode>>,
    mut commands: Commands,
    assets: Res<AssetServer>,
    query: Query<Entity, Or<(With<RenderedBlock>, With<Player>)>>
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
            let mut decoded: SaveFile = bincode::borrow_decode_from_slice(&encoded_vec, BINCODE_CONFIG)
                .unwrap()
                .0;
            crate::world::spawn_sprites_from_terrain(&mut commands, assets, &mut decoded.terrain);
            crate::player::load_player_pos(decoded.player_coords, &mut commands, &assets);
            commands.insert_resource(decoded);
        }
        Err(e) => {
            error!("could not read save file, {}", e);
        }
    }
}