// Initially based on [wasm4-raycaster](https://github.com/grantshandy/wasm4-raycaster)
// which carries an MIT License and is Copyright (c) 2023 Grant Handy.

#![no_std]

mod constants;
mod state;
mod util;
mod view;
mod wasm4;
mod arms;

use wasm4::{
    DRAW_COLORS,
    GAMEPAD1,
    BUTTON_UP, BUTTON_DOWN,
    BUTTON_LEFT, BUTTON_RIGHT,
    BUTTON_1, BUTTON_2,
    vline, oval
};

use state::State;

use view::{get_wall_view, get_bullet_view};

static mut STATE: State = State::new();
static mut PREVIOUS_GAMEPAD: u8 = 0;

#[no_mangle]
unsafe fn start() {
    STATE.generate_maze();
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
        let (x_position, size, inflight) = bullet;
        if *inflight {
            oval(*x_position, 75, 10, 10);
        }
    }

    PREVIOUS_GAMEPAD = *GAMEPAD1;
}


