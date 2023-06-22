// Initially based on [wasm4-raycaster](https://github.com/grantshandy/wasm4-raycaster)
// which carries an MIT License and is Copyright (c) 2023 Grant Handy.

#![no_std]

mod constants;
mod state;
mod wasm4;

use wasm4::{
    DRAW_COLORS,
    GAMEPAD1,
    BUTTON_UP, BUTTON_DOWN,
    BUTTON_LEFT, BUTTON_RIGHT,
    BUTTON_1, BUTTON_2,
    vline
};

use state::State;

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

    // Go through each column on screen and draw walls in the center.
    for (x, wall) in STATE.get_walls().iter().enumerate() {
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

    PREVIOUS_GAMEPAD = *GAMEPAD1;
}


