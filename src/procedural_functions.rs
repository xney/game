use rand::{rngs::StdRng, SeedableRng, Rng};

use crate::world::CHUNK_WIDTH;


//Generates vector of random values, with seed, with amount
pub fn generate_random_values(seed: u64, amount: usize, low: usize, high: usize) -> Vec<i32>{
    let mut values: Vec<i32> = Vec::new();

    let mut rand = StdRng::seed_from_u64(seed);
    for _n in 0..amount{
        let value: i32 = rand.gen_range(low as i32, high as i32);
        values.push(value);
    }
    values
}

//Get the value (float) of a position X
pub fn slice_pos_x(x: usize, r: &Vec<i32>) -> f32{

    //Do this so we can generate spaces between points
    let x_float = (x as f32) / ((CHUNK_WIDTH/r.len()) + 1) as f32; 

    let x_int = x_float as u32; 
    let diff = x_float - (x_int as f32); 

    //Cubic curve
    let u = diff * diff * (3.0 - 2.0 * diff); 

    //Interpolate + return
    return (r[x_int as usize]) as f32 *(1.0f32-u) + ((r[(x_int+1) as usize]) as f32 * u); 
    
}