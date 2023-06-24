use libm::{ceilf, cosf, fabsf, floorf, sinf, tanf};
use core::f32::consts::{PI, FRAC_PI_2};

use rand::{SeedableRng, Rng};
use rand::rngs::SmallRng;

use core::fmt::Write;
use heapless::{String,Vec};

use crate::constants::{
    WIDTH, HEIGHT, NUM_CELLS, MAX_PASSAGES, STEP_SIZE, BULLET_SPEED, 
    RELOAD_TIME, NUM_BULLETS, HALF_FOV, ANGLE_STEP, WALL_HEIGHT
};

use crate::wasm4::{
    tone,
    TONE_NOISE,
    trace
};

use maze_gen::{find_passages, find_walls, there_is_no_passage_here};
use crate::arms::{Ammo, Bullet};

use crate::util::{distance, get_index, point_in_wall};

pub struct State {
    pub player_x: f32,
    pub player_y: f32,
    pub player_angle: f32,
    pub player_ammo: [Ammo;NUM_BULLETS],
    pub bullets: Vec<Bullet,NUM_BULLETS>,
    visited: Vec<bool,NUM_CELLS>,
    passages: Vec<(usize,usize),MAX_PASSAGES>,
    pub horizontal_walls: Vec<u16,{HEIGHT+1}>,
    pub vertical_walls: Vec<u16,{WIDTH+1}>,
    seed: u64
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
            vertical_walls: Vec::<u16,{WIDTH+1}>::new(),
            seed: 0
        }
    }

    /// Called from the game start(), this creates a random maze.
    pub fn generate_maze(&mut self, rng: &mut SmallRng) {
        // Initialize an empty maze
        self.visited.extend_from_slice(&[false;NUM_CELLS]).unwrap();
        self.horizontal_walls.extend_from_slice(&[0b0000000000000000;{HEIGHT+1}]).unwrap();
        self.vertical_walls.extend_from_slice(&[0b0000000000000000;{WIDTH+1}]).unwrap();

        // Randomly create passages to define the maze, starting from first index
        let index = 0;
        find_passages(index, WIDTH, HEIGHT, &mut self.visited, &mut self.passages, rng);

        // Use the passages to define the walls of the maze
        find_walls(WIDTH, HEIGHT, &mut self.passages, &mut self.horizontal_walls, &mut self.vertical_walls);

        self.seed = rng.gen::<u64>();
    }

    /// Update the game state based on user input.
    pub fn update(&mut self, up: bool, down: bool, left: bool, right: bool, 
        shoot: bool, spray: bool) 
    {
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
                    let mut rng = SmallRng::seed_from_u64(self.seed);
                    self.seed = rng.gen::<u64>();
                    let attempt = self.bullets.push(
                        Bullet::new(
                            self.player_x,
                            self.player_y,
                            self.player_angle + (rng.gen::<f32>() - 0.5)/10.0, // TODO: Add random jitter
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
}