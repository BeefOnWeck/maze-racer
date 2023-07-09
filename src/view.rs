use core::f32::consts::{PI, FRAC_PI_2};
use libm::{ceilf, cosf, fabsf, floorf, tanf, atan2f, sinf, powf};
use heapless::Vec;

use crate::constants::{
    HEIGHT, WIDTH, HALF_FOV, ANGLE_STEP, WALL_HEIGHT, 
    NUM_BULLETS, BULLETS_PER_PLAYER, NUM_PLAYERS, PLAYER_WIDTH
};
use crate::util::{distance, point_in_wall};
use crate::arms::{Bullet, Ammo};

/// Filters other players by a player's field of view and returns drawing information. 
pub fn get_player_view(
    player_index: usize,
    player_angle: [f32; NUM_PLAYERS],
    player_x: [f32; NUM_PLAYERS],
    player_y: [f32; NUM_PLAYERS],
    player_life: [i32; NUM_PLAYERS]
) -> [(i32, i32, u32, u32, f32, bool, bool, bool); NUM_PLAYERS] {

    let fov_upper_limit = player_angle[player_index] + HALF_FOV;
    let fov_lower_limit = fov_upper_limit - (159.0 * ANGLE_STEP);

    // Each rect defined by: x position, y position, width, height, distance, and visibility flag
    let mut rects = [(0, 0, 0, 0, 0.0, false, false, false); NUM_PLAYERS];

    for index in 0..NUM_PLAYERS {
        if index != player_index {
            let alive = player_life[index] > 0;
            if alive {
                // TODO: Refactor into function and reuse here and get_bullet_view
                let rise = player_y[index] - player_y[player_index];
                let run = player_x[index] - player_x[player_index];
                let distance_to_player = distance(rise, run);

                let angle_to_player = -1.0 * atan2f(rise, run);
                let num_wraps = floorf((angle_to_player - player_angle[player_index])/(2.0 * PI));
                let unwrapped = angle_to_player - 2.0 * PI * num_wraps;
                let extra_unwrapped = unwrapped - 2.0 * PI;

                let unwrap_diff = fabsf(player_angle[player_index] - unwrapped);
                let extra_diff = fabsf(player_angle[player_index] - extra_unwrapped);
                let extra_is_closer = extra_diff < unwrap_diff;
                let unwrapped_angle = if extra_is_closer {
                    extra_unwrapped
                } else {
                    unwrapped
                };

                // Determine how large the player should appear
                let size = (PLAYER_WIDTH / distance_to_player / ANGLE_STEP) as u32;
                let correction = (size / 2) as i32;
                let fov_correction = ANGLE_STEP * ( size as f32 );

                // Adjust apparent width based upon the relative angle of the player
                let width = (( size as f32 ) * fabsf(cosf(player_angle[index] - angle_to_player))) as u32;

                // Check if the angle falls in the FOV
                if unwrapped_angle >= fov_lower_limit - fov_correction && 
                    unwrapped_angle <= fov_upper_limit + fov_correction 
                {
                    // Determine where the FOV the bullet falls
                    let h_position = ((fov_upper_limit - unwrapped_angle) / ANGLE_STEP) as i32 - correction;

                    // Vertical correction to account for size
                    let v_position = 80 - ( size as f32 / 2.0 ) as i32;

                    // Is this player facing me?
                    let sum_of_squares = 
                        powf(cosf(player_angle[index]) + cosf(angle_to_player), 2.0) +
                        powf(sinf(player_angle[index]) + sinf(angle_to_player), 2.0);
                    let facing_me = sum_of_squares < 2.0;

                    // Update the view for this player with this index
                    rects[index] = (h_position, v_position, width, size, distance_to_player, facing_me, alive, true);
                }
            }
        }
    }

    return rects;

}

/// Returns information for drawing a player's ammunition dashboard
pub fn get_ammo_view(player_ammo: [Ammo; BULLETS_PER_PLAYER]) -> [(i32, i32, u32, i32, u32); BULLETS_PER_PLAYER] {

    // Each player ammunition is represented by x and y positions, size, correction, and status.
    let mut ammo_dashboard: [(i32, i32, u32, i32, u32); BULLETS_PER_PLAYER] = [
        (120, 4, 8, 0, 0),
        (130, 4, 8, 0, 0),
        (140, 4, 8, 0, 0)
    ];

    // Determine status for each ammo
    for (index, ammo) in player_ammo.iter().enumerate() {
        let status = match ammo {
            Ammo::Loaded => 8,
            Ammo::Reloading(time_to_reload) => match time_to_reload {
                193..=255 => 0,
                127..=192 => 2,
                65..=128 => 4,
                _ => 6
            }
        };
        // We need a correction to get concentric circles as status changes
        let correction = (8 - status as i32) / 2;
        ammo_dashboard[index].3 = correction;
        ammo_dashboard[index].4 = status;
    }
    
    // Each player ammunition is represented by x and y positions, size, correction, and status.
    return ammo_dashboard;

}

/// Filters bullets by player's field of view and returns bullet size and position on screen.
pub fn get_bullet_view(
    player_angle: f32,
    player_x: f32,
    player_y: f32,
    bullets: &Vec<Bullet,NUM_BULLETS>
) -> [(i32, i32, u32, f32, bool); NUM_BULLETS] {

    let fov_upper_limit = player_angle + HALF_FOV;
    let fov_lower_limit = fov_upper_limit - (159.0 * ANGLE_STEP);

    // Each oval defined by: x position, y position, size, distance, and visibility flag
    let mut ovals = [(0, 0, 0, 0.0, false); NUM_BULLETS];

    // Enumerate the bullets and update their oval if they are in the field of view
    for (index, bullet) in bullets.iter().enumerate() {
        // Only consider bullets that are still inflight
        if bullet.inflight {
            // Calculate angle and distance of bullet
            let rise = bullet.y - player_y;
            let run = bullet.x - player_x;
            let bullet_distance = distance(rise, run);

            // Calculate the angle and unwrap
            let bullet_angle = -1.0 * atan2f(rise, run);
            let num_wraps = floorf((bullet_angle - player_angle)/(2.0 * PI));
            let unwrapped = bullet_angle - 2.0 * PI * num_wraps;
            let extra_unwrapped = unwrapped - 2.0 * PI;

            // Sometimes unwrapping is off by one (end condition)
            let extra_is_closer = fabsf(player_angle - unwrapped) > fabsf(player_angle - extra_unwrapped);
            let unwrapped_angle = if extra_is_closer {
                extra_unwrapped
            } else {
                unwrapped
            };

            // Check if the angle falls in the FOV
            if unwrapped_angle >= fov_lower_limit && unwrapped_angle <= fov_upper_limit {
                // Determine where the FOV the bullet falls
                let h_position = ((fov_upper_limit - unwrapped_angle) / ANGLE_STEP) as i32;

                // Determine how large the bullet should appear
                let size = (0.1 / bullet_distance / ANGLE_STEP) as u32;

                // Vertical correction for far away bullets
                let v_position = 75 + bullet_distance as i32;
                
                ovals[index] = (h_position, v_position, size, bullet_distance, true);
            }
        }
    }

    // Each oval defined by: x position, y position, size, distance, and visibility flag
    return ovals;
}

/// Returns 160 wall heights and their "color" from the player's perspective.
/// Source: https://github.com/grantshandy/wasm4-raycaster/blob/main/src/lib.rs
/// Copyright (c) 2023 Grant Handy
/// MIT License
pub fn get_wall_view(
    player_angle: f32,
    player_x: f32,
    player_y: f32,
    horizontal_walls: &Vec<u16, { HEIGHT + 1 }>,
    vertical_walls: &Vec<u16, { WIDTH + 1 }>,
) -> [(i32, f32, bool); 160] {
    // The player's FOV is split in half by their viewing angle.
    // In order to get the ray's starting angle we must
    // add half the FOV to the player's angle to get
    // the edge of the player's FOV.
    let starting_angle = player_angle + HALF_FOV;

    let mut walls = [(0, 0.0, false); 160];

    for (idx, wall) in walls.iter_mut().enumerate() {
        // `idx` is what number ray we are, `wall` is
        // a mutable reference to a value in `walls`.
        let angle = starting_angle - idx as f32 * ANGLE_STEP;

        // Get both the closest horizontal and vertical wall
        // intersections for this angle.
        let h_dist = horizontal_intersection(player_x, player_y, &horizontal_walls, angle);
        let v_dist = vertical_intersection(player_x, player_y, &vertical_walls, angle);

        let (min_dist, shadow) = if h_dist < v_dist {
            (h_dist, false)
        } else {
            (v_dist, true)
        };

        // Get the minimum of the two distances and
        // "convert" it into a wall height.
        *wall = (
            (WALL_HEIGHT / (min_dist * cosf(angle - player_angle))) as i32,
            min_dist,
            shadow,
        );
    }

    walls
}

/// Returns the nearest wall the ray intersects with on a horizontal grid line.
/// Source: https://github.com/grantshandy/wasm4-raycaster/blob/main/src/lib.rs
/// Copyright (c) 2023 Grant Handy
/// MIT License
fn horizontal_intersection(
    player_x: f32,
    player_y: f32,
    horizontal_walls: &Vec<u16, { HEIGHT + 1 }>,
    angle: f32,
) -> f32 {
    // This tells you if the angle is "facing up"
    // regardless of how big the angle is.
    let up = fabsf(floorf(angle / PI) % 2.0) != 0.0;

    // first_y and first_x are the first grid intersections
    // that the ray intersects with.
    let first_y = if up {
        ceilf(player_y) - player_y
    } else {
        floorf(player_y) - player_y
    };
    let first_x = -first_y / tanf(angle);

    // dy and dx are the "ray extension" values mentioned earlier.
    let dy = if up { 1.0 } else { -1.0 };
    let dx = -dy / tanf(angle);

    // next_x and next_y are mutable values which will keep track
    // of how far away the ray is from the player.
    let mut next_x = first_x;
    let mut next_y = first_y;

    // This is the loop where the ray is extended until it hits
    // the wall. It's not an infinite loop as implied in the
    // explanation, instead it only goes from 0 to 256.
    //
    // This was chosen because if something goes wrong and the
    // ray never hits a wall (which should never happen) the
    // loop will eventually break and the game will keep on running.
    for _ in 0..256 {
        // current_x and current_y are where the ray is currently
        // on the map, while next_x and next_y are relative
        // coordinates, current_x and current_y are absolute
        // points.
        let current_x = next_x + player_x;
        let current_y = if up {
            next_y + player_y
        } else {
            next_y + player_y
        };

        if point_in_wall(current_y, current_x, &horizontal_walls) {
            break;
        }

        // if we didn't hit a wall on this extension add
        // dx and dy to our current position and keep going.
        next_x += dx;
        next_y += dy;
    }

    // return the distance from next_x and next_y to the player.
    distance(next_x, next_y)
}

/// Returns the nearest wall the ray intersects with on a vertical grid line.
/// Source: https://github.com/grantshandy/wasm4-raycaster/blob/main/src/lib.rs
/// Copyright (c) 2023 Grant Handy
/// MIT License
fn vertical_intersection(
    player_x: f32,
    player_y: f32,
    vertical_walls: &Vec<u16, { WIDTH + 1 }>,
    angle: f32,
) -> f32 {
    // This tells you if the angle is "facing up"
    // regardless of how big the angle is.
    let right = fabsf(floorf((angle - FRAC_PI_2) / PI) % 2.0) != 0.0;

    // first_y and first_x are the first grid intersections
    // that the ray intersects with.
    let first_x = if right {
        ceilf(player_x) - player_x
    } else {
        floorf(player_x) - player_x
    };
    let first_y = -tanf(angle) * first_x;

    // dy and dx are the "ray extension" values mentioned earlier.
    let dx = if right { 1.0 } else { -1.0 };
    let dy = dx * -tanf(angle);

    // next_x and next_y are mutable values which will keep track
    // of how far away the ray is from the player.
    let mut next_x = first_x;
    let mut next_y = first_y;

    // This is the loop where the ray is extended until it hits
    // the wall. It's not an infinite loop as implied in the
    // explanation, instead it only goes from 0 to 256.
    //
    // This was chosen because if something goes wrong and the
    // ray never hits a wall (which should never happen) the
    // loop will eventually quit and the game will keep on running.
    for _ in 0..256 {
        // current_x and current_y are where the ray is currently
        // on the map, while next_x and next_y are relative
        // coordinates, current_x and current_y are absolute
        // points.
        let current_x = if right {
            next_x + player_x
        } else {
            next_x + player_x
        };
        let current_y = next_y + player_y;

        if point_in_wall(current_x, current_y, &vertical_walls) {
            break;
        }

        // if we didn't hit a wall on this extension add
        // dx and dy to our current position and keep going.
        next_x += dx;
        next_y += dy;
    }

    // return the distance from next_x and next_y to the player.
    distance(next_x, next_y)
}