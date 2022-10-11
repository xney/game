use bevy::prelude::*;

use crate::states;

const CHUNK_HEIGHT: usize = 10;
const CHUNK_WIDTH: usize = 10;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_enter(states::GameState::InGame).with_system(create_world),
        )
        .add_system_set(SystemSet::on_exit(states::GameState::InGame).with_system(destroy_world));
    }
}

pub fn create_world(mut commands: Commands, assets: Res<AssetServer>) {
    info!("creating world");

    // create now, insert as resource later
    let mut terrain = Terrain::default();

    // Generate one chunk
    spawn_chunk(0, &mut commands, assets, &mut terrain);

    let locations_to_destroy = [(5, 5), (5, 5), (300, 300), (5, 500), (2, 8)];

    // destroy some blocks
    for (x, y) in locations_to_destroy {
        info!("attempting to destroy block at ({}, {})", x, y);
        let result = destroy_block(x, y, &mut commands, &mut terrain);
        match result {
            Ok(b) => info!(
                "successfully destroyed block at ({}, {}), was type {:?}",
                x, y, b.block_type
            ),
            Err(e) => warn!("unable to destroy block at ({}, {}): {:?}", x, y, e),
        }
    }

    // now add as resource
    commands.insert_resource(terrain);
}

fn destroy_world(mut commands: Commands, query: Query<Entity, With<RenderedBlock>>) {
    info!("destroying world");
    // remove all block sprites
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }

    commands.remove_resource::<Terrain>();
}

/// Represents all chunks in the game world
/// Should be a global resource
pub struct Terrain {
    /// Vector of chunks, each one contains its own chunk_number
    /// TODO: potentially convert into a symbol table for faster lookups?
    chunks: Vec<Chunk>,
}

impl Default for Terrain {
    fn default() -> Self {
        Self {
            chunks: Default::default(),
        }
    }
}

/// Represents a chunk of blocks; stored in the Terrain resource
pub struct Chunk {
    /// 2D array [x, y]
    pub blocks: [[Option<Block>; CHUNK_WIDTH]; CHUNK_HEIGHT],
    /// starting row for blocks is chunk_number * CHUNK_HEIGHT
    chunk_number: u64,
}

impl Chunk {
    pub fn new(depth: u64) -> Self {
        // For now: populate entire Chunk with Sandstone
        let c = Chunk {
            blocks: [[Some(Block {
                block_type: BlockType::Sandstone,
                entity: None,
            }); CHUNK_WIDTH]; CHUNK_HEIGHT],
            chunk_number: depth,
        };
        return c;
    }
}

/// _Not_ a component; stored in a Chunk
#[derive(Copy, Clone)]
pub struct Block {
    /// What kind of block is this
    block_type: BlockType,
    /// The sprite entity that renders this block
    entity: Option<Entity>,
}

/// Marker component for tagging sprite entity for block
#[derive(Component)]
pub struct RenderedBlock;

/// A distinct type of block, with its own texture
#[derive(Copy, Clone, Debug)]
pub enum BlockType {
    Sandstone,
}

impl BlockType {
    /// Return the file path for the image that should be displayed for this block
    const fn image_file_path(&self) -> &str {
        match self {
            BlockType::Sandstone => "Sandstone.png",
        }
    }
}

/// Create all blocks in chunk as actual entities (and store references to entity in chunk.blocks)
pub fn spawn_chunk(
    chunk_number: u64,
    commands: &mut Commands,
    assets: Res<AssetServer>,
    terrain: &mut Terrain,
) {
    let mut chunk = Chunk::new(chunk_number);

    // Loop through entire chunk (2D Array)
    for x in 0..CHUNK_WIDTH {
        for y in 0..CHUNK_HEIGHT {
            let block_opt = &mut chunk.blocks[x][y];

            // if there is a block at this location
            if let Some(block) = block_opt {
                // spawn in the sprite for the block
                let entity = commands
                    .spawn()
                    .insert_bundle(SpriteBundle {
                        texture: assets.load(block.block_type.image_file_path()),
                        transform: Transform {
                            translation: Vec3::from_array([
                                to_world_point_x(x),
                                to_world_point_y(y, chunk_number),
                                1.,
                            ]),
                            ..default()
                        },
                        ..default()
                    })
                    .insert(RenderedBlock)
                    .id();

                // link the entity to the block
                block.entity = Option::Some(entity);
            }
            // else there is no block and we don't have to spawn any sprite
        }
    }

    // add the chunk to our terrain resource
    terrain.chunks.push(chunk);
}

#[derive(Debug)]
pub enum DestroyBlockError {
    /// Tried to search past array index in X direction
    /// TODO: make this compile-time error
    InvalidX,
    /// Corresponding chunk location is not loaded (outside Y)
    ChunkNotLoaded,
    /// Block data at the location is empty (block doesn't exist!)
    BlockDoesntExist,
}

/// Destroy a block at a global position
pub fn destroy_block(
    x: usize,
    y: usize,
    commands: &mut Commands,
    terrain: &mut Terrain,
) -> Result<Block, DestroyBlockError> {
    let chunk_number = y / CHUNK_HEIGHT;
    let block_y_in_chunk = y % CHUNK_HEIGHT;

    // make sure our x is in range
    // TODO: do this in a const fashion?
    if x >= CHUNK_WIDTH {
        return Err(DestroyBlockError::InvalidX);
    }

    // find if we have the chunk in our terrain
    for chunk in &mut terrain.chunks {
        if chunk.chunk_number == (chunk_number as u64) {
            // we have found our chunk
            let block_opt = &mut chunk.blocks[x][block_y_in_chunk];

            match block_opt {
                Some(block) => {
                    match block.entity {
                        Some(entity) => {
                            info!("despawning sprite for block at ({}, {})", x, y);
                            commands.entity(entity).despawn();
                        }
                        None => {
                            warn!("block at ({}, {}) exists but had no entity attached!", x, y);
                        }
                    };

                    // unlink entity
                    block.entity = None;

                    // clone block data so we can give it to the caller
                    let clone = block.clone();

                    // remove the block from our data array
                    // original block is dropped here
                    *block_opt = None;

                    // give the clone back to the caller
                    // TODO: maybe give a different data type?
                    return Ok(clone);
                }
                None => {
                    warn!("no block exists at ({}, {})", x, y);
                    return Err(DestroyBlockError::BlockDoesntExist);
                }
            }
        }
    }

    Err(DestroyBlockError::ChunkNotLoaded)
}

fn to_world_point_x(x: usize) -> f32 {
    return (x as f32) * 32.;
}
fn to_world_point_y(y: usize, chunk_number: u64) -> f32 {
    return -(y as f32 + chunk_number as f32 * CHUNK_HEIGHT as f32) * 32.;
}
