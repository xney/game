use bevy::prelude::*;

use crate::states;

static BLOCK_MAPPING: &'static [&str] = &["NONE", "Sandstone.png"];
const CHUNK_HEIGHT: usize = 5;
const CHUNK_WIDTH: usize = 40;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_enter(states::GameState::InGame).with_system(setup_world),
        )
        .add_system_set(
            SystemSet::on_exit(states::GameState::InGame).with_system(destroy_world),
        );
    }
}

fn setup_world(mut commands: Commands, assets: Res<AssetServer>) {
    //Generate one chunk
    let mut chunk = Chunk::new();
    spawn_chunk(&mut chunk, &mut commands, assets);

    //(Example): Destroy a single block at 3,3
    destroy_block(&mut chunk, &mut commands, 3, 3);
}

fn destroy_world(mut commands: Commands, query: Query<Entity, With<Block>>) {
    info!("destroying world");
    // remove all block entities
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub struct Chunk {
    pub blocks: [[Block; CHUNK_HEIGHT]; CHUNK_WIDTH],
    //depth_number: i32 //TODO make sure chunk generates at the correct depth (right now, all starting from 0)
}

impl Chunk {
    pub fn new() -> Self {
        //For now: populate entire Chunk with block_type 1
        let c = Chunk {
            blocks: [[Block {
                block_type: 1,
                id: Option::None,
            }; CHUNK_HEIGHT]; CHUNK_WIDTH],
        };
        return c;
    }
}

#[derive(Copy, Clone, Component)]
pub struct Block {
    pub block_type: usize,
    pub id: Option<Entity>,
}

/// Create all blocks in chunk as actual entities (and store references to entity in chunk.blocks)
pub fn spawn_chunk(chunk: &mut Chunk, commands: &mut Commands, assets: Res<AssetServer>) {
    //Loop through entire chunk (2D Array)
    for i in 0..chunk.blocks.len() {
        for j in 0..chunk.blocks[i].len() {
            if chunk.blocks[i][j].block_type == 0 {
                continue;
            }
            let entity = commands
                .spawn()
                .insert_bundle(SpriteBundle {
                    texture: assets.load(BLOCK_MAPPING[chunk.blocks[i][j].block_type]),
                    transform: Transform {
                        translation: Vec3::from_array([
                            to_world_point_x(i),
                            to_world_point_y(j),
                            1.,
                        ]),
                        ..default()
                    },
                    ..default()
                })
                .insert(chunk.blocks[i][j])
                .id();
            chunk.blocks[i][j].id = Option::Some(entity);
        }
    }
}

//Destroy an individual block of a chunk
pub fn destroy_block(c: &mut Chunk, commands: &mut Commands, x: usize, y: usize) {
    let opt = c.blocks[x][y].id;

    match opt {
        Some(opt) => {
            c.blocks[x][y] = Block {
                block_type: 0,
                id: Option::None,
            };
            commands.entity(opt).despawn();
        }
        None => {}
    }
}

fn to_world_point_x(i: usize) -> f32 {
    return (i as f32) * 32.;
}
fn to_world_point_y(i: usize) -> f32 {
    return -(i as f32) * 32.;
}