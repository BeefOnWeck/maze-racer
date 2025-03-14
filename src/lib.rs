// Initially based on [wasm4-raycaster](https://github.com/grantshandy/wasm4-raycaster)
// which carries an MIT License and is Copyright (c) 2023 Grant Handy.

#![no_std]

mod constants;
mod state;
mod util;
mod view;
mod wasm4;
mod arms;

use heapless::{String};
use rand::{rngs::SmallRng, SeedableRng};
use util::point_in_wall;
use wasm4::{
    DRAW_COLORS, BLIT_1BPP, NETPLAY, PALETTE,
    GAMEPAD1, GAMEPAD2, GAMEPAD3, GAMEPAD4,
    BUTTON_UP, BUTTON_DOWN,
    BUTTON_LEFT, BUTTON_RIGHT,
    BUTTON_1, BUTTON_2,
    vline, oval, rect, blit, line, diskr, trace, text
};
use core::{f32::consts::PI, fmt::Write};
use libm::{atan2f, fabsf, floorf};

use state::{State, View};
use constants::{WIDTH, HEIGHT, NUM_PLAYERS};

use view::{get_wall_view, get_bullet_view, get_ammo_view, get_player_view};

static mut STATE: State = State::new();
static mut PREVIOUS_GAMEPAD1: u8 = 0;
static mut PREVIOUS_GAMEPAD2: u8 = 0;
static mut PREVIOUS_GAMEPAD3: u8 = 0;
static mut PREVIOUS_GAMEPAD4: u8 = 0;

#[no_mangle]
unsafe fn start() {

    *PALETTE = [0xfff6d3, 0xeb6b6f, 0xf9a875, 0x7c3f58];

    let mut buffer = [0u8; core::mem::size_of::<i32>()];
    diskr(buffer.as_mut_ptr(), buffer.len() as u32);
    let seed = u32::from_le_bytes(buffer);

    let mut RNG = SmallRng::seed_from_u64(seed as u64);
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
        *GAMEPAD1 & (*GAMEPAD1 ^ PREVIOUS_GAMEPAD1) & BUTTON_2 != 0
    );

    // TODO: Remove netplay
    let pid = if *NETPLAY & 0b100 != 0 {
        (*NETPLAY & 0b011) as usize
    } else {
        0
    };

    // Draw either the first person view or the top-down view
    match STATE.player_view[pid] {
        View::FirstPerson => {
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
                STATE.player_y,
                STATE.player_life
            );

            // Draw walls first
            for (x, wall) in walls.iter().enumerate() {
                let (height, _, shadow) = wall;

                if *shadow {
                    // draw with color 2 for walls with "shadow"
                    *DRAW_COLORS = 0x2;
                } else {
                    // draw with color 3 for walls without "shadow"
                    *DRAW_COLORS = 0x3;
                }

                vline(x as i32, 80 - (height / 2), *height as u32);
            }

            // Then draw players
            for player in players.iter() {
                let (h_position, v_position, width, height, distance, facing_me, alive, not_me) = player;
                if *not_me && *alive {
                    let x = match *h_position {
                        0..=159 => *h_position as usize,
                        _ => 0
                    };
                    let (_, wall_distance, _) = walls[x];
                    // Only draw if not obstructed by a wall
                    if *distance < wall_distance {
                        // Body
                        *DRAW_COLORS = 0x41;
                        rect(*h_position + ((*height - *width) / 2) as i32, *v_position, *width, *height);
                        // Only draw the face if they are facing me
                        if *facing_me {
                            // Left eye
                            *DRAW_COLORS = 0x44;
                            let x = *h_position as f32 + *width as f32 * 1.0 / 8.0;
                            let y = *v_position as f32 + *height as f32 * 1.0 / 8.0;
                            let w = *width as f32 / 4.0;
                            let h = *height as f32 / 4.0;
                            rect(
                                x as i32  + ((*height - *width) / 2) as i32, 
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
                                x as i32  + ((*height - *width) / 2) as i32, 
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
                                x as i32  + ((*height - *width) / 2) as i32, 
                                y as i32,
                                w as u32,
                                h as u32
                            );
                        }
                    }
                }
            }

            // Next draw bullets that are in view
            *DRAW_COLORS = 0x04;
            for bullet in bullets.iter() {
                let (h_position, v_position, size, distance, inflight) = bullet;
                let x = match *h_position {
                    0..=159 => *h_position as usize,
                    _ => 0
                };
                let (_, wall_distance, _) = walls[x];
                if *inflight {
                    if *distance < wall_distance {
                        oval(*h_position, *v_position, *size, *size);
                    }
                }
            }

            // And draw the ammunition dashboard
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

            const HEART_ICON: [u8; 8] = [
                0b10011001,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b10000001,
                0b11000011,
                0b11100111,
            ];

            // Finally draw the life dashboard
            let num_hearts = STATE.player_life[pid];
            for heart in 1..=num_hearts {
                blit(&HEART_ICON, 10*heart, 4, 8, 8, BLIT_1BPP);
            }

            // Draw the score
            let mut message = String::<32>::new();
            let score = STATE.score;
            write!(message, "Score: {score}").unwrap();
            text(message, 10, 16);
        },
        View::TopDown => {
            // NOTE: Right now the top-down view is just a real-time display of the maze.
            //       But in the future it could be a place for selecting other weapons.
            *DRAW_COLORS = 0x04;
            // Horizontal walls
            for h in 0..=HEIGHT {
                let y = h as f32;
                for w in 0..=WIDTH {
                    let x = w as f32 + 0.5;
                    if point_in_wall(y, x, &STATE.horizontal_walls) {
                        line(((x-0.5)*10.0+15.0) as i32, (y*10.0 + 15.0) as i32, ((x+0.5)*10.0+15.0) as i32, (y*10.0 + 15.0) as i32);
                    }
                }
            }
            *DRAW_COLORS = 0x04;
            // Vertical walls
            for w in 0..=WIDTH {
                let x = w as f32;
                for h in 0..=HEIGHT {
                    let y = h as f32 + 0.5;
                    if point_in_wall(x, y, &STATE.vertical_walls) {
                        line((x*10.0 + 15.0) as i32, ((y-0.5)*10.0+15.0) as i32, (x*10.0 + 15.0) as i32, ((y+0.5)*10.0+15.0) as i32);
                    }
                }
            }
            *DRAW_COLORS = 0x44;
            *DRAW_COLORS = 0x04;
            // Players
            for player in 0..NUM_PLAYERS {
                // Only draw players that are alive
                if STATE.player_life[player] > 0 {
                    const X: [f32; 64] = [
                        -4.0, -3.0, -2.0, -1.0, 1.0, 2.0, 3.0, 4.0,
                        -4.0, -3.0, -2.0, -1.0, 1.0, 2.0, 3.0, 4.0,
                        -4.0, -3.0, -2.0, -1.0, 1.0, 2.0, 3.0, 4.0,
                        -4.0, -3.0, -2.0, -1.0, 1.0, 2.0, 3.0, 4.0,
                        -4.0, -3.0, -2.0, -1.0, 1.0, 2.0, 3.0, 4.0,
                        -4.0, -3.0, -2.0, -1.0, 1.0, 2.0, 3.0, 4.0,
                        -4.0, -3.0, -2.0, -1.0, 1.0, 2.0, 3.0, 4.0,
                        -4.0, -3.0, -2.0, -1.0, 1.0, 2.0, 3.0, 4.0,
                    ];
                    const Y: [f32; 64] = [
                        -4.0, -4.0, -4.0, -4.0, -4.0, -4.0, -4.0, -4.0,
                        -3.0, -3.0, -3.0, -3.0, -3.0, -3.0, -3.0, -3.0,
                        -2.0, -2.0, -2.0, -2.0, -2.0, -2.0, -2.0, -2.0,
                        -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0, -1.0,
                        1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0,
                        2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0,
                        3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0, 3.0,
                        4.0, 4.0, 4.0, 4.0, 4.0, 4.0, 4.0, 4.0,
                    ];
                    let mut blit_mask = [false; 64];
                    let mut n = 0;
                    for (y, x) in Y.into_iter().zip(X) {
                        let xp = -1.0 * x;
                        let yp = y;
                        let blit_angle = -1.0 * atan2f(yp, xp);
                        // let mut data = String::<64>::new();
                        // write!(data, "y: {yp}, x: {xp}, a: {blit_angle}").unwrap();
                        // trace(data);
                        let num_wraps = floorf((blit_angle - STATE.player_angle[player])/(2.0 * PI));
                        let unwrapped = blit_angle - 2.0 * PI * num_wraps;
                        let extra_unwrapped = unwrapped - 2.0 * PI;

                        let unwrap_diff = fabsf(STATE.player_angle[player] - unwrapped);
                        let extra_diff = fabsf(STATE.player_angle[player] - extra_unwrapped);
                        let extra_is_closer = extra_diff < unwrap_diff;
                        let angle_difference = if extra_is_closer {
                            extra_diff
                        } else {
                            unwrap_diff
                        };
                        blit_mask[n] = if
                            (angle_difference + 0.4 > PI / 2.0 && angle_difference - 0.4 < PI / 2.0) ||
                            (angle_difference + 0.4 > -1.0 * PI / 2.0 && angle_difference - 0.4 < -1.0 * PI / 2.0) ||
                            n == 27 || n == 28 || n == 35 || n == 36 { true } 
                        else { false };
                        n += 1;
                    }
                    let mut player_blit: [u8; 8] = [
                        0b11111111,
                        0b11111111,
                        0b11111111,
                        0b11111111,
                        0b11111111,
                        0b11111111,
                        0b11111111,
                        0b11111111,
                    ];
                    n = 0;
                    for bm in blit_mask {
                        let row = n / 8;
                        let col = n - row*8;
                        if bm == true {
                            if (row != 0 && row != 7 && col != 0 && col != 7) {
                                player_blit[row] -= 0b1 << col;
                            }
                        }
                        n += 1;
                    }
                    blit(&player_blit, (STATE.player_x[player]*10.0) as i32 + 15 - 3, (STATE.player_y[player]*10.0) as i32 + 15 - 3, 8, 8, BLIT_1BPP);
                }
            }
            *DRAW_COLORS = 0x44;
            // Bullets
            for bullet in STATE.bullets.iter() {
                oval((bullet.x*10.0) as i32 + 15, (bullet.y*10.0) as i32 + 15, 1, 1);
            }
        }
    }

    if STATE.player_life[pid] <= 0 {
        *DRAW_COLORS = 0x14;
        let mut message = String::<32>::new();
        let player_number = pid + 1;
        write!(message, "PLAYER {player_number} IS").unwrap();
        text(message, 40, 72);
        text("ELIMINATED!", 40, 80);
    }

    PREVIOUS_GAMEPAD1 = *GAMEPAD1;
    PREVIOUS_GAMEPAD2 = *GAMEPAD2;
    PREVIOUS_GAMEPAD3 = *GAMEPAD3;
    PREVIOUS_GAMEPAD4 = *GAMEPAD4;
}


