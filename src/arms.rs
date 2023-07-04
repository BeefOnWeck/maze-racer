
#[derive(Clone, Copy, PartialEq)]
pub enum Ammo {
    Loaded,
    Reloading(u8)
}

#[derive(Clone, Copy, PartialEq)]
pub struct Bullet {
    pub x: f32,
    pub y: f32,
    pub owner: usize,
    pub angle: f32,
    pub inflight: bool
}

impl Bullet {

    pub fn new(x: f32, y: f32, owner: usize, angle: f32, inflight: bool) -> Bullet {
        Bullet {
            x,
            y,
            owner,
            angle,
            inflight
        }
    }
}