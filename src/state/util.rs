use libm::{sqrtf};
use heapless::{Vec};

// TODO: Move to util.rs
pub fn distance(a: f32, b: f32) -> f32 {
    sqrtf((a * a) + (b * b))
}

pub fn get_index(x: f32, y: f32, width: usize, height: usize) -> usize {
    (x as usize) + (y as usize) * width
}

pub fn point_in_wall<const N: usize>(d1: f32, d2: f32, walls: &Vec<u16,N>) -> bool {
    match walls.get(d1 as usize) {
        Some(line) => (line & (0b1 << d2 as usize)) != 0,
        None => true
    }
}