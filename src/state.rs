use libm::{ceilf, cosf, fabsf, floorf, sinf, sqrtf, tanf};
use core::f32::consts::{PI, FRAC_PI_2};

use heapless::Vec;
use rand::SeedableRng;
use rand::rngs::SmallRng;

use maze_gen::find_next_passage;

const MAP: [u16; 8] = [
    0b1111111111111111,
    0b1000001010000101,
    0b1011100000110101,
    0b1000111010010001,
    0b1010001011110111,
    0b1011101001100001,
    0b1000100000001101,
    0b1111111111111111,
];

const WIDTH: usize = 13; // number of horizontal cells in maze
const HEIGHT: usize = 13; // number of vertical cells in maze
const NUM_CELLS: usize = WIDTH * HEIGHT;
const MAX_PASSAGES: usize = NUM_CELLS; // memory to reserve for maze

const FOV: f32 = PI / 2.7; // The player's field of view.
const HALF_FOV: f32 = FOV * 0.5; // Half the player's field of view.
const ANGLE_STEP: f32 = FOV / 160.0; // The angle between each ray.
const WALL_HEIGHT: f32 = 100.0; // A magic number.

fn distance(a: f32, b: f32) -> f32 {
    sqrtf((a * a) + (b * b))
}

/// Check if the map contains a wall at a point.
fn point_in_wall(x: f32, y: f32) -> bool {
    match MAP.get(y as usize) {
        Some(line) => (line & (0b1 << x as usize)) != 0,
        None => true
    }
}

pub struct State {
    pub player_x: f32,
    pub player_y: f32,
    pub player_angle: f32,
    pub visited: Vec<bool,NUM_CELLS>,
    pub passages: Vec<(usize,usize),MAX_PASSAGES>
}

const STEP_SIZE: f32 = 0.045;

impl State {

    pub const fn new() -> State {
        State {
            player_x: 1.5,
            player_y: 1.5,
            player_angle: 0.0,
            visited: Vec::<bool,NUM_CELLS>::new(),
            passages: Vec::<(usize,usize),MAX_PASSAGES>::new()
        }
    }

    pub fn gen_maze(&mut self) {
        let index = 0;
        let mut rng = SmallRng::seed_from_u64(11);
        self.visited.extend_from_slice(&[false;NUM_CELLS]).unwrap();
        find_next_passage(index, WIDTH, HEIGHT, &mut self.visited, &mut self.passages, &mut rng);
    }

    /// move the character
    pub fn update(&mut self, up: bool, down: bool, left: bool, right: bool) {
        // store our current position in case we might need it later
        let previous_position = (self.player_x, self.player_y);

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

        // if moving us on this frame would put us into a wall, just revert it
        if point_in_wall(self.player_x, self.player_y) {
            (self.player_x, self.player_y) = previous_position;
        }
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
                next_y + self.player_y - 1.0
            };

            // Tell the loop to quit if we've just hit a wall.
            if point_in_wall(current_x, current_y) {
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
                next_x + self.player_x - 1.0
            };
            let current_y = next_y + self.player_y;

            // Tell the loop to quit if we've just hit a wall.
            if point_in_wall(current_x, current_y) {
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

    /// Returns 160 wall heights and their "color" from the player's perspective.
    pub fn get_view(&self) -> [(i32, bool); 160] {
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
}