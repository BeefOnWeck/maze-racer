use libm::{ceilf, cosf, fabsf, floorf, sinf, tanf};
use core::f32::consts::{PI, FRAC_PI_2};

use rand::SeedableRng;
use rand::rngs::SmallRng;

use core::fmt::Write;
use heapless::{String,Vec};

use crate::constants::{
    WIDTH,
    HEIGHT,
    NUM_CELLS,
    MAX_PASSAGES,
    STEP_SIZE,
    BULLET_SPEED,
    RELOAD_TIME,
    NUM_BULLETS,
    HALF_FOV,
    ANGLE_STEP,
    WALL_HEIGHT
};

use crate::wasm4::{
    tone,
    TONE_NOISE,
    trace
};

use maze_gen::{find_passages, find_walls, there_is_no_passage_here};

mod arms;
use arms::{Ammo, Bullet};

mod util;
use util::{distance, get_index, point_in_wall};

pub struct State {
    pub player_x: f32,
    pub player_y: f32,
    pub player_angle: f32,
    pub player_ammo: [Ammo;NUM_BULLETS],
    pub bullets: Vec<Bullet,NUM_BULLETS>,
    visited: Vec<bool,NUM_CELLS>,
    passages: Vec<(usize,usize),MAX_PASSAGES>,
    pub horizontal_walls: Vec<u16,{HEIGHT+1}>,
    pub vertical_walls: Vec<u16,{WIDTH+1}>
}

impl State {

    pub const fn new() -> State {
        State {
            player_x: 0.5,
            player_y: 0.5,
            player_angle: 0.0,
            player_ammo: [Ammo::Loaded;NUM_BULLETS],
            bullets: Vec::<Bullet,NUM_BULLETS>::new(),
            visited: Vec::<bool,NUM_CELLS>::new(),
            passages: Vec::<(usize,usize),MAX_PASSAGES>::new(),
            horizontal_walls: Vec::<u16,{HEIGHT+1}>::new(),
            vertical_walls: Vec::<u16,{WIDTH+1}>::new()
        }
    }

    pub fn generate_maze(&mut self) {
        
        // TODO: Replace this fixed seed with a random external one
        let mut rng = SmallRng::seed_from_u64(11);

        // Initialize an empty maze
        self.visited.extend_from_slice(&[false;NUM_CELLS]).unwrap();
        self.horizontal_walls.extend_from_slice(&[0b0000000000000000;{HEIGHT+1}]).unwrap();
        self.vertical_walls.extend_from_slice(&[0b0000000000000000;{WIDTH+1}]).unwrap();

        // Randomly create passages to define the maze, starting from first index
        let index = 0;
        find_passages(index, WIDTH, HEIGHT, &mut self.visited, &mut self.passages, &mut rng);

        // Use the passages to define the walls of the maze
        find_walls(WIDTH, HEIGHT, &mut self.passages, &mut self.horizontal_walls, &mut self.vertical_walls);

    }

    /// Update the game state based on user input.
    pub fn update(&mut self, up: bool, down: bool, left: bool, right: bool, shoot: bool, spray: bool) {

        self.update_player(up, down, left, right);
        self.update_ammo(shoot, spray);
        self.update_bullets();

    }

    /// Moves a player around based on user input.
    fn update_player(&mut self, up: bool, down: bool, left: bool, right: bool) {
        // Store the current position and index in case we need to undo a move.
        let previous_position = (self.player_x, self.player_y);
        let previous_index = get_index(self.player_x, self.player_y, WIDTH, HEIGHT);

        // Tentative updates to player position and orientation.
        if up {
            self.player_x += cosf(self.player_angle) * STEP_SIZE;
            self.player_y += -sinf(self.player_angle) * STEP_SIZE;
        }
        if down {
            self.player_x -= cosf(self.player_angle) * STEP_SIZE;
            self.player_y -= -sinf(self.player_angle) * STEP_SIZE;
        }
        if right {
            self.player_angle -= STEP_SIZE;

        }
        if left {
            self.player_angle += STEP_SIZE;
        }

        // If the player has moved to a new cell, then new_index will differ from previous_index.
        let new_index = get_index(self.player_x, self.player_y, WIDTH, HEIGHT);

        // Conditionally undo the move.
        if ( // If move would cause player to leave the maze...
            (self.player_x <= 0.0) ||
            (self.player_y <= 0.0) ||
            (self.player_x as usize >= WIDTH) ||
            (self.player_y as usize >= HEIGHT)
        ) ||
        ( // ...or if they would go through a wall...
            (previous_index != new_index) && 
            there_is_no_passage_here(previous_index, new_index, &self.passages)
        )
        { // ... undo the move.
            (self.player_x, self.player_y) = previous_position;
        }
    }

    /// Fires a bullet in response to player input; incrementally reloads spent ammo.
    fn update_ammo(&mut self, shoot: bool, spray: bool) {
        // When the player presses either button.
        // What's the difference between shoot and spray?
        // The shoot button is debounced, so when it is pressed, only one bullet will fire.
        // The spray button is "sticky." 
        // This means that a single press will activate it for several frames, shooting all ammo.
        if shoot || spray { 
            // Find the first loaded ammo
            match self.player_ammo.iter_mut().find(|&&mut a| a == Ammo::Loaded) {
                Some(mut ammo) => {
                    // Change it to reloading
                    *ammo = Ammo::Reloading(RELOAD_TIME);
                    trace("Shot bullet");
                    tone(1000 | (10 << 16), 10, 100, TONE_NOISE);
                    let attempt = self.bullets.push(
                        Bullet::new(
                            self.player_x,
                            self.player_y,
                            self.player_angle, // TODO: Add random jitter
                            true
                        )
                    );
                    match attempt {
                        Ok(()) => {},
                        Err(_) => trace("Reached max number of bullets in the air.")
                    }
                },
                None => trace("Empty")
            }
        }

        // Find the first ammo that is not loaded and incrementally reload it.
        // Spent ammo take RELOAD_TIME frames to reload and are reloaded one at a time.
        match self.player_ammo.iter_mut().find(|&&mut a| a != Ammo::Loaded) {
            Some(mut ammo) => match ammo {
                // Decrement time to reload until we reach 0 (means we are loaded)
                Ammo::Reloading(time_to_reload) => {
                    if *time_to_reload > 0 {
                        *ammo = Ammo::Reloading(*time_to_reload-1);
                    } else {
                        *ammo = Ammo::Loaded;
                        trace("Loaded!");
                        tone(50 | (150 << 16), 50, 100, TONE_NOISE);
                    }
                }
                _ => {}
            },
            None => {}
        }
    }

    /// Propagates bullets in flight, removing them when they hit a wall.
    fn update_bullets(&mut self) {
        // Update the position of each bullet in flight.
        self.bullets.iter_mut().for_each(|b| {
            let previous_index = get_index(b.x, b.y, WIDTH, HEIGHT);
            b.x += cosf(b.angle) * BULLET_SPEED;
            b.y += -sinf(b.angle) * BULLET_SPEED;
            let new_index = get_index(b.x, b.y, WIDTH, HEIGHT);
            if ( // If bullet leaves the maze...
                b.x <= 0.0 || b.y <= 0.0 || b.x as usize >= WIDTH || b.y as usize >= HEIGHT
            ) ||
            ( // ...or if it would go through a wall...
                (previous_index != new_index) && 
                there_is_no_passage_here(previous_index, new_index, &self.passages)
            )
            { // ... mark inflight as false.
                b.inflight = false;
                trace("Bullet done");
            }

        });

        // TODO: Player collision detection.

        // Remove bullets that are no longer inflight.
        self.bullets = self.bullets.iter().map(|b| *b).filter(|b| b.inflight == true).collect();
    }

    /// Returns 160 wall heights and their "color" from the player's perspective.
    pub fn get_walls(&self) -> [(i32, bool); 160] {
        // The player's FOV is split in half by their viewing angle.
        // In order to get the ray's starting angle we must
        // add half the FOV to the player's angle to get
        // the edge of the player's FOV.
        let starting_angle = self.player_angle + HALF_FOV;

        let mut walls = [(0, false); 160];

        for (idx, wall) in walls.iter_mut().enumerate() {
            // `idx` is what number ray we are, `wall` is
            // a mutable reference to a value in `walls`.
            let angle = starting_angle - idx as f32 * ANGLE_STEP;

            // Get both the closest horizontal and vertical wall
            // intersections for this angle.
            let h_dist = self.horizontal_intersection(angle);
            let v_dist = self.vertical_intersection(angle);

            let (min_dist, shadow) = if h_dist < v_dist {
                (h_dist, false)
            } else {
                (v_dist, true)
            };

            // Get the minimum of the two distances and
            // "convert" it into a wall height.
            *wall = (
                (WALL_HEIGHT / (min_dist * cosf(angle - self.player_angle))) as i32,
                shadow,
            );
        }

        walls
    }

    /// Returns the nearest wall the ray intersects with on a horizontal grid line.
    fn horizontal_intersection(&self, angle: f32) -> f32 {
        // This tells you if the angle is "facing up"
        // regardless of how big the angle is.
        let up = fabsf(floorf(angle / PI) % 2.0) != 0.0;

        // first_y and first_x are the first grid intersections
        // that the ray intersects with.
        let first_y = if up {
            ceilf(self.player_y) - self.player_y
        } else {
            floorf(self.player_y) - self.player_y
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
            let current_x = next_x + self.player_x;
            let current_y = if up {
                next_y + self.player_y
            } else {
                next_y + self.player_y
            };

            if point_in_wall(current_y, current_x, &self.horizontal_walls) {
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
    fn vertical_intersection(&self, angle: f32) -> f32 {
        // This tells you if the angle is "facing up"
        // regardless of how big the angle is.
        let right = fabsf(floorf((angle - FRAC_PI_2) / PI) % 2.0) != 0.0;
        // let mut data = String::<32>::new();
        // write!(data, "Right:{right}").unwrap();
        // trace(data);

        // first_y and first_x are the first grid intersections
        // that the ray intersects with. 
        let first_x = if right {
            ceilf(self.player_x) - self.player_x
        } else {
            floorf(self.player_x) - self.player_x
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
                next_x + self.player_x
            } else {
                next_x + self.player_x
            };
            let current_y = next_y + self.player_y;

            if point_in_wall(current_x, current_y, &self.vertical_walls) {
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
}