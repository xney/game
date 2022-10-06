//let data = vec!["sand"];
use bevy::prelude::*;
static BLOCK_MAPPING: &'static [&str] = &[
    "NONE",
    "Sandstone.png",
];

pub struct Chunk{
    tiles: [[Block; 5];8],// state = [[0u8; 4]; 6];
    //startX: i32
}

impl Chunk{
    pub fn new() -> Self{
        let mut c = Chunk{ tiles: [[Block {block_type:0 , id: 0}; 5];8] };
        c.tiles[0][0] = Block{block_type: 1, id: 0};
        c.tiles[0][2] = Block{block_type: 1, id: 0};
        c.tiles[1][1] = Block{block_type: 1, id: 0};

        return c;
    }
}

pub fn SpawnChunk(c: &Chunk, mut commands: Commands, assets: Res<AssetServer>){
            
        for (i, row) in c.tiles.iter().enumerate() {
            for (y, col) in row.iter().enumerate() {
                if(c.tiles[i][y].block_type == 0){
                    continue;
                }
                commands.spawn()
                .insert_bundle(
                    SpriteBundle {
                        texture: assets.load(BLOCK_MAPPING[c.tiles[i][y].block_type]),
                        transform: Transform {
                            translation: Vec3::from_array([ToWorldPoint(i), ToWorldPoint(y), 1.]),
                            ..default()
                        },
                        ..default()
                    }
                );
            }
        }
}

#[derive(Copy, Clone)]
struct Block{
    block_type: usize,
    id: u32
}

fn ToWorldPoint(i: usize) -> f32{
    return (i as f32) * 32.;
}

fn ToIndex(){
    
}