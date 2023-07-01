// Initially based on [wasm4-raycaster](https://github.com/grantshandy/wasm4-raycaster)
// which carries an MIT License and is Copyright (c) 2023 Grant Handy.

#![no_std]

mod constants;
mod state;
mod util;
mod view;
mod wasm4;
mod arms;

use rand::{rngs::SmallRng, SeedableRng};
use wasm4::{
    DRAW_COLORS, NETPLAY,
    GAMEPAD1, GAMEPAD2, GAMEPAD3, GAMEPAD4,
    BUTTON_UP, BUTTON_DOWN,
    BUTTON_LEFT, BUTTON_RIGHT,
    BUTTON_1, BUTTON_2,
    vline, oval, rect
};

use state::State;

use view::{get_wall_view, get_bullet_view, get_ammo_view, get_player_view};

static mut STATE: State = State::new();
static mut PREVIOUS_GAMEPAD1: u8 = 0;
static mut PREVIOUS_GAMEPAD2: u8 = 0;
static mut PREVIOUS_GAMEPAD3: u8 = 0;
static mut PREVIOUS_GAMEPAD4: u8 = 0;

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
        *GAMEPAD1 & (*GAMEPAD1 ^ PREVIOUS_GAMEPAD1) & BUTTON_1 != 0,
        *GAMEPAD1 & BUTTON_2 != 0,
        *GAMEPAD2 & BUTTON_UP != 0,
        *GAMEPAD2 & BUTTON_DOWN != 0,
        *GAMEPAD2 & BUTTON_LEFT != 0,
        *GAMEPAD2 & BUTTON_RIGHT != 0,
        *GAMEPAD2 & (*GAMEPAD2 ^ PREVIOUS_GAMEPAD2) & BUTTON_1 != 0,
        *GAMEPAD2 & BUTTON_2 != 0,
        *GAMEPAD3 & BUTTON_UP != 0,
        *GAMEPAD3 & BUTTON_DOWN != 0,
        *GAMEPAD3 & BUTTON_LEFT != 0,
        *GAMEPAD3 & BUTTON_RIGHT != 0,
        *GAMEPAD3 & (*GAMEPAD3 ^ PREVIOUS_GAMEPAD3) & BUTTON_1 != 0,
        *GAMEPAD3 & BUTTON_2 != 0,
        *GAMEPAD4 & BUTTON_UP != 0,
        *GAMEPAD4 & BUTTON_DOWN != 0,
        *GAMEPAD4 & BUTTON_LEFT != 0,
        *GAMEPAD4 & BUTTON_RIGHT != 0,
        *GAMEPAD4 & (*GAMEPAD4 ^ PREVIOUS_GAMEPAD4) & BUTTON_1 != 0,
        *GAMEPAD4 & BUTTON_2 != 0,
    );

    let pid = if *NETPLAY & 0b100 != 0 {
        (*NETPLAY & 0b011) as usize
    } else {
        0
    };

    let walls = get_wall_view(
        STATE.player_angle[pid], 
        STATE.player_x[pid], 
        STATE.player_y[pid], 
        &STATE.horizontal_walls, 
        &STATE.vertical_walls
    );

    let bullets = get_bullet_view(
        STATE.player_angle[pid], 
        STATE.player_x[pid], 
        STATE.player_y[pid],
        &STATE.bullets
    );

    let ammunition = get_ammo_view(
        STATE.player_ammo[pid]
    );

    let players = get_player_view(
        pid,
        STATE.player_angle, 
        STATE.player_x, 
        STATE.player_y
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

    for player in players.iter() {
        let (h_position, v_position, width, height, notme) = player;
        if *notme {
            // Body
            *DRAW_COLORS = 0x41;
            rect(*h_position, *v_position, *width, *height);
            // Left eye
            *DRAW_COLORS = 0x44;
            let x = *h_position as f32 + *width as f32 * 1.0 / 8.0;
            let y = *v_position as f32 + *height as f32 * 1.0 / 8.0;
            let w = *width as f32 / 4.0;
            let h = *height as f32 / 4.0;
            rect(
                x as i32, 
                y as i32,
                w as u32,
                h as u32
            );
            // Right eye
            *DRAW_COLORS = 0x44;
            let x = *h_position as f32 + *width as f32 * 5.0 / 8.0;
            let y = *v_position as f32 + *height as f32 * 1.0 / 8.0;
            let w = *width as f32 / 4.0;
            let h = *height as f32 / 4.0;
            rect(
                x as i32, 
                y as i32,
                w as u32,
                h as u32
            );
            // Mouth
            *DRAW_COLORS = 0x44;
            let x = *h_position as f32 + *width as f32 * 1.0 / 8.0;
            let y = *v_position as f32 + *height as f32 * 5.0 / 8.0;
            let w = *width as f32 * 3.0 / 4.0;
            let h = *height as f32 * 1.0 / 4.0;
            rect(
                x as i32, 
                y as i32,
                w as u32,
                h as u32
            );
        }
    }

    *DRAW_COLORS = 0x04;
    for bullet in bullets.iter() {
        let (h_position, v_position, size, inflight) = bullet;
        if *inflight {
            oval(*h_position, *v_position, *size, *size);
        }
    }

    *DRAW_COLORS = 0x40;
    for ammo in ammunition.iter() {
        let (x, y, size, _, _) = *ammo;
        oval(x, y, size, size);
    }

    *DRAW_COLORS = 0x04;
    for ammo in ammunition.iter() {
        let (x, y, _, fix, fill) = *ammo;
        if fill > 0 {
            oval(x+fix, y+fix, fill, fill);
        }
    }

    PREVIOUS_GAMEPAD1 = *GAMEPAD1;
    PREVIOUS_GAMEPAD2 = *GAMEPAD2;
    PREVIOUS_GAMEPAD3 = *GAMEPAD3;
    PREVIOUS_GAMEPAD4 = *GAMEPAD4;
}


