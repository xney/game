use bevy::prelude::*;

const CHUNK_HEIGHT: usize = 16;
const CHUNK_WIDTH: usize = 16;

/// Adds terrain and chunks
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Terrain::default())
            .add_startup_system(setup);
    }
}

/// Represents the all terrain in the game world
/// Should be a global resource
pub struct Terrain {
    // 1D Vector of chunks
    chunks: Vec<Chunk>,
}

impl Default for Terrain {
    fn default() -> Self {
        Self {
            chunks: Default::default(),
        }
    }
}

/// Represents a chunk of blocks
pub struct Chunk {
    pub blocks: [[Block; CHUNK_HEIGHT]; CHUNK_WIDTH],
    /// starting row for blocks is chunk_number * CHUNK_HEIGHT
    chunk_number: u64,
}

impl Chunk {
    pub fn new(depth: u64) -> Self {
        // For now: populate entire Chunk with Sandstone
        let c = Chunk {
            blocks: [[Block {
                block_type: BlockType::Sandstone,
                entity: None,
            }; CHUNK_WIDTH]; CHUNK_HEIGHT],
            chunk_number: depth,
        };
        return c;
    }
}

#[derive(Copy, Clone)]
pub struct Block {
    block_type: BlockType,
    entity: Option<Entity>,
}

#[derive(Copy, Clone)]
pub enum BlockType {
    None,
    Sandstone,
}

impl BlockType {
    /// Return the file path for the image that should be displayed for this block
    fn image_file_path(&self) -> &str {
        match self {
            BlockType::Sandstone => "Sandstone.png",
            BlockType::None => "",
        }
    }
}

/// Create all blocks in chunk as actual entities (and store references to entity in chunk.blocks)
pub fn spawn_chunk(
    chunk_number: u64,
    commands: &mut Commands,
    assets: Res<AssetServer>,
    terrain: &mut ResMut<Terrain>,
) {
    let mut chunk = Chunk::new(chunk_number);

    // Loop through entire chunk (2D Array)
    for x in 0..CHUNK_WIDTH {
        for y in 0..CHUNK_HEIGHT {
            let mut block = &mut chunk.blocks[x][y];

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
                .id();

            block.entity = Some(entity);
        }
    }

    // add the chunk to our terrain resource
    terrain.chunks.push(chunk);
}

/// Destroy a block at a global position
/// Returns Ok(()) on success, or Err(()) if the block was not found
pub fn destroy_block(
    x: usize,
    y: usize,
    commands: &mut Commands,
    terrain: &mut ResMut<Terrain>,
) -> Result<(), ()> {
    let chunk_number = y / CHUNK_HEIGHT;
    let block_y_in_chunk = y % CHUNK_HEIGHT;

    let mut success = false;

    // make sure our x is in range
    if x >= CHUNK_WIDTH {
        return Err(());
    }

    // find if we have the chunk in our terrain
    for chunk in &terrain.chunks {
        if chunk.chunk_number == (chunk_number as u64) {
            // we have found our chunk
            let mut block = chunk.blocks[x][block_y_in_chunk];

            block.block_type = BlockType::None;

            match block.entity {
                Some(entity) => {
                    info!("despawning sprite for block at ({}, {})", x, y);
                    commands.entity(entity).despawn();
                }
                None => {
                    warn!("block at ({}, {}) had no entity attached!", x, y);
                }
            };

            success = true;
        }
    }

    return if success { Ok(()) } else { Err(()) };
}

fn to_world_point_x(x: usize) -> f32 {
    return (x as f32) * 32.;
}
fn to_world_point_y(y: usize, chunk_number: u64) -> f32 {
    return -(y as f32 + chunk_number as f32 * CHUNK_HEIGHT as f32) * 32.;
}

pub fn setup(mut commands: Commands, assets: Res<AssetServer>, mut terrain: ResMut<Terrain>) {
    // Generate one chunk
    spawn_chunk(0, &mut commands, assets, &mut terrain);

    // (Example): Destroy a single block
    let result = destroy_block(5, 5, &mut commands, &mut terrain);
    match result {
        Ok(_) => info!("successfully destroyed block"),
        Err(_) => info!("unable to destroy block"),
    }
}
