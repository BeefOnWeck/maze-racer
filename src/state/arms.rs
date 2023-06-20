#[derive(Clone, Copy, PartialEq)]
pub enum Ammo {
    Loaded,
    Reloading(u8)
}

pub struct Bullet {
    pub x: f32,
    pub y: f32,
    pub angle: f32
}

impl Bullet {

    pub fn new(x: f32, y: f32, angle: f32) -> Bullet {
        Bullet {
            x,
            y,
            angle
        }
    }
}