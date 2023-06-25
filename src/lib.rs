// Initially based on [wasm4-raycaster](https://github.com/grantshandy/wasm4-raycaster)
// which carries an MIT License and is Copyright (c) 2023 Grant Handy.

#![no_std]

mod constants;
mod state;
mod util;
mod view;
mod wasm4;
mod arms;

use core::arch::wasm32::i32x4_abs;

use rand::{rngs::SmallRng, SeedableRng};
use wasm4::{
    DRAW_COLORS,
    GAMEPAD1,
    BUTTON_UP, BUTTON_DOWN,
    BUTTON_LEFT, BUTTON_RIGHT,
    BUTTON_1, BUTTON_2,
    vline, oval
};

use state::State;

use view::{get_wall_view, get_bullet_view, get_ammo_view};

static mut STATE: State = State::new();
static mut PREVIOUS_GAMEPAD: u8 = 0;

#[no_mangle]
unsafe fn start() {
    let mut RNG = SmallRng::seed_from_u64(11);
    STATE.generate_maze(&mut RNG);
}

#[no_mangle]
unsafe fn update() {
    STATE.update(
        *GAMEPAD1 & BUTTON_UP != 0,
        *GAMEPAD1 & BUTTON_DOWN != 0,
        *GAMEPAD1 & BUTTON_LEFT != 0,
        *GAMEPAD1 & BUTTON_RIGHT != 0,
        *GAMEPAD1 & (*GAMEPAD1 ^ PREVIOUS_GAMEPAD) & BUTTON_1 != 0,
        *GAMEPAD1 & BUTTON_2 != 0
    );

    let walls = get_wall_view(
        STATE.player_angle, 
        STATE.player_x, 
        STATE.player_y, 
        &STATE.horizontal_walls, 
        &STATE.vertical_walls
    );

    let bullets = get_bullet_view(
        STATE.player_angle, 
        STATE.player_x, 
        STATE.player_y,
        &STATE.bullets
    );

    let ammo = get_ammo_view(
        STATE.player_ammo
    );

    // Go through each column on screen and draw walls in the center.
    for (x, wall) in walls.iter().enumerate() {
        let (height, shadow) = wall;

        if *shadow {
            // draw with color 2 for walls with "shadow"
            *DRAW_COLORS = 0x2;
        } else {
            // draw with color 3 for walls without "shadow"
            *DRAW_COLORS = 0x3;
        }

        vline(x as i32, 80 - (height / 2), *height as u32);
    }

    *DRAW_COLORS = 0x04;
    for bullet in bullets.iter() {
        let (h_position, v_position, size, inflight) = bullet;
        if *inflight {
            oval(*h_position, *v_position, *size, *size);
        }
    }

    *DRAW_COLORS = 0x40;
    oval(120, 4, 8, 8);
    oval(130, 4, 8, 8);
    oval(140, 4, 8, 8);

    *DRAW_COLORS = 0x04;
    if ammo[0] > 0 {
        let correction: i32 = (8 - ammo[0] as i32)/2;
        oval(120+correction, 4+correction, ammo[0], ammo[0]);
    }
    if ammo[1] > 0 {
        let correction: i32 = (8 - ammo[1] as i32)/2;
        oval(130+correction, 4+correction, ammo[1], ammo[1]);
    }
    if ammo[2] > 0 {
        let correction: i32 = (8 - ammo[2] as i32)/2;
        oval(140+correction, 4+correction, ammo[2], ammo[2]);
    }

    PREVIOUS_GAMEPAD = *GAMEPAD1;
}


