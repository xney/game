use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use bevy::prelude::info;
use rand::{rngs::StdRng, Rng, SeedableRng};
use rand_distr::{Binomial, Distribution, Normal};

use crate::world::{BlockType, Cave, Vein, CHUNK_HEIGHT, CHUNK_WIDTH};

const APPROX_CAVES_PER_CHUNK: f64 = 6.0;
const AVG_CAVE_WIDTH: f32 = 6.;
const CAVE_STANDARD_DEV: f32 = 2.;
const CAVE_BOARDER_NOISE: f32 = 1.5;

pub fn generate_seed(base_seed: u64, additional_data: Vec<u64>) -> u64 {
    let mut s = DefaultHasher::new();
    base_seed.hash(&mut s);
    for data in additional_data {
        data.hash(&mut s);
    }
    s.finish()
}

//Generates vector of random values, with seed, with amount
pub fn generate_random_values(seed: u64, amount: usize, low: usize, high: usize) -> Vec<i32> {
    let mut values: Vec<i32> = Vec::new();

    let mut rand = StdRng::seed_from_u64(seed);
    for _n in 0..amount {
        let value: i32 = rand.gen_range(low as i32..high as i32);
        values.push(value);
    }
    values
}

//Generates a random count of veins for a chunk using a normal distribution
pub fn generate_random_vein_count(seed: u64, chunk_number: u64) -> u64 {
    let approx_veins_per_chunk = 8.0;
    // Treat it as if every block of a chunk has a % chance of originating an ore vein
    let mut rand = StdRng::seed_from_u64(generate_seed(seed, vec![chunk_number]));
    let bindist = Binomial::new(
        (CHUNK_WIDTH * CHUNK_HEIGHT) as u64,
        approx_veins_per_chunk / (CHUNK_WIDTH * CHUNK_HEIGHT) as f64,
    )
    .unwrap();
    let value = bindist.sample(&mut rand);
    value
}

//Generates random vein with a random start coordinate, end coordinate, and thickness
pub fn generate_random_vein(seed: u64, chunk_number: u64, vein_number: u64) -> Vein {
    let mut rand = StdRng::seed_from_u64(generate_seed(seed, vec![chunk_number, vein_number]));

    // Generate random start coordinate
    let start_x = rand.gen_range(0..CHUNK_WIDTH);
    let start_y = rand.gen_range(0..CHUNK_HEIGHT);

    // End x can be left or right of start
    let end_x = (start_x as i16)
        + (rand.gen_range(10 as i16..32 as i16) * (if rand.gen_bool(0.5) { 1 } else { -1 }));
    // End y can only be below start (so you don't have a new vein that's supposed to go up to the previous chunk)
    let end_y = (start_y as i16) + rand.gen_range(5 as i16..16 as i16);

    let thickness_sq: f32 = rand.gen_range(1.0..3.0);

    info!(
        "Generated vein from {},{} to {},{} in chunk {} with thickness_sq {}",
        start_x,
        (start_y + (chunk_number as usize * CHUNK_HEIGHT)),
        end_x,
        (end_y + (chunk_number as usize * CHUNK_HEIGHT) as i16),
        chunk_number,
        thickness_sq
    );

    Vein {
        block_type: BlockType::Coal,
        chunk_number,
        start_x,
        start_y,
        end_x,
        end_y,
        thickness_sq,
    }
}

//Get the value (float) of a position X
pub fn slice_pos_x(x: usize, r: &Vec<i32>) -> f32 {
    //Do this so we can generate spaces between points
    let x_float = (x as f32) / ((CHUNK_WIDTH / r.len()) + 1) as f32;

    let x_int = x_float as u32;
    let diff = x_float - (x_int as f32);

    //Cubic curve
    let u = diff * diff * (3.0 - 2.0 * diff);

    //Interpolate + return
    return (r[x_int as usize]) as f32 * (1.0f32 - u) + ((r[(x_int + 1) as usize]) as f32 * u);
}

fn dist_sq(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    ((x1 - x2).powf(2.0) + (y1 - y2).powf(2.0)).into()
}

pub fn dist_to_vein(vein: &Vein, x: f32, y: f32) -> f32 {
    // Get distance from point to line segment
    // Adapted from https://stackoverflow.com/a/1501725/1474787
    // Do all necessary casting first for readability's sake
    let vx1 = vein.start_x as f32;
    let vx2 = vein.end_x as f32;
    let vy1 = vein.start_y as f32;
    let vy2 = vein.end_y as f32;

    let len_sq = dist_sq(vx1, vy1, vx2, vy2);
    if len_sq == 0.0 {
        return dist_sq(x, y, vx1, vy1);
    };

    // Project point onto line, clamping from 0 to 1 to handle points that are outside the line segment
    let mut proj: f32 = ((x - vx1) * (vx2 - vx1) + (y - vy1) * (vy2 - vy1)) / len_sq;
    proj = (proj.min(1.0)).max(0.0);

    dist_sq(x, y, vx1 + (proj * (vx2 - vx1)), vy1 + (proj * (vy2 - vy1)))
}

pub fn generate_random_cave_count(seed: u64, chunk_number: u64) -> u64 {
    let mut rand = StdRng::seed_from_u64(generate_seed(seed, vec![chunk_number]));
    let bindist = Binomial::new(
        (CHUNK_WIDTH * CHUNK_HEIGHT) as u64,
        APPROX_CAVES_PER_CHUNK / (CHUNK_WIDTH * CHUNK_HEIGHT) as f64,
    )
    .unwrap();
    let value = bindist.sample(&mut rand);

    return value;
}

pub fn generate_random_cave(seed: u64, chunk_number: u64, cave_number: u64) -> Cave {
    let mut rand = StdRng::seed_from_u64(generate_seed(seed, vec![chunk_number, cave_number]));

    let start_x = rand.gen_range(0..CHUNK_WIDTH);
    let start_y = rand.gen_range(0..CHUNK_HEIGHT);

    let a = Normal::new(AVG_CAVE_WIDTH, CAVE_STANDARD_DEV)
        .unwrap()
        .sample(&mut rand);
    let b = Normal::new(AVG_CAVE_WIDTH, CAVE_STANDARD_DEV / 2.)
        .unwrap()
        .sample(&mut rand);

    return Cave {
        block_type: BlockType::CaveVoid,
        chunk_number,
        cave_number,
        start_x,
        start_y,
        a,
        b,
    };
}

//Defines oval boarder of Cave, adds noise to circumference of oval to make caves more random
pub fn is_point_in_cave(cave: &Cave, x: usize, y: usize, seed: u64) -> bool {
    let mut rand = StdRng::seed_from_u64(generate_seed(
        seed,
        vec![cave.chunk_number, cave.cave_number],
    ));

    let x0 = cave.start_x as f32;
    let y0 = cave.start_y as f32;

    let dist = ((x0 - x as f32).powf(2.) / cave.a.powf(2.) as f32)
        + ((y0 - y as f32).powf(2.) as f32 / cave.b.powf(2.) as f32);

    let noise = Normal::new(1., CAVE_BOARDER_NOISE)
        .unwrap()
        .sample(&mut rand);

    return dist <= noise;
}
