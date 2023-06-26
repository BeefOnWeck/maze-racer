use core::f32::consts::PI;

pub const WIDTH: usize = 13; // number of horizontal cells in maze
pub const HEIGHT: usize = 13; // number of vertical cells in maze
pub const NUM_CELLS: usize = WIDTH * HEIGHT;
pub const MAX_PASSAGES: usize = NUM_CELLS; // memory to reserve for maze

pub const FOV: f32 = PI / 2.7; // The player's field of view.
pub const HALF_FOV: f32 = FOV * 0.5; // Half the player's field of view.
pub const ANGLE_STEP: f32 = FOV / 160.0; // The angle between each ray.
pub const WALL_HEIGHT: f32 = 80.0; // A magic number.
pub const STEP_SIZE: f32 = 0.045;

pub const NUM_PLAYERS: usize = 4;
pub const BULLETS_PER_PLAYER: usize = 3;
pub const NUM_BULLETS: usize = NUM_PLAYERS * BULLETS_PER_PLAYER;
pub const RELOAD_TIME: u8 = 255;
pub const BULLET_SPEED: f32 = 0.01;